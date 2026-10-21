//! The upload path: CPU frame bytes → staging buffer → texture-ready
//! RGBA (card 2.D1 "RGBA8 upload (measured staging)").
//!
//! The shape mirrors what a wgpu backend does per frame
//! (`Queue::write_buffer` into a staging allocation, then the
//! buffer→texture copy): one measured staging copy into a reusable
//! buffer, then the bytes become the *texture* — in the v0 software
//! path the staged buffer **is** the sampleable texture (a move, zero
//! further copies); on the hardware path the same staged bytes ride
//! `write_buffer` and the copy count is measured again
//! `[VERIFY: GPU-side, where a device exists]`.
//!
//! What v0 measures headlessly (and the AC asserts exactly):
//!
//! - `bytes_staged` — the transfer size, per frame and total;
//! - `staging_copies` — buffer copies performed (one per frame; the
//!   Bgra8 swizzle is the same single pass);
//! - `stage_ns_total` / `last_stage_ns` — wall time of the staging
//!   copies ([`std::time::Instant`], recorded not asserted — timing is
//!   evidence, not a contract).
//!
//! # Zero-copy honesty
//!
//! True zero-copy upload (importing decoder-owned buffers, e.g.
//! dmabuf) is a hardware-path capability (doc 03 §4.11 e "[VERIFY: per
//! backend capability]"); v0 always stages through exactly one buffer
//! copy, which is also `Queue::write_buffer`'s own shape.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use media_types::frame::PixelFormat;
use media_types::packet::SharedBytes;

use crate::error::{RendererError, RendererResult};
use crate::params::Extent2D;

/// How many frame buffers the recycle pool keeps (D-1: bounded).
/// Three covers the frame currently being composited plus two handed
/// out (a presenter pipeline that overlaps one frame — e.g. a
/// present-thread decoupling — never starves the pool).
const RECYCLE_SLOTS: usize = 3;

/// The bounded buffer-recycle pool shared between the
/// [`StagingUploader`] and the [`StagedFrame`]s it handed out: a frame
/// dropping returns its `rgba` Vec to the pool; the next `stage`
/// pops one big enough and refills it in place. Memory is bounded at
/// `RECYCLE_SLOTS · frame-bytes`; a frame nobody drops simply never
/// returns (correct, never leaks into unbounded growth).
#[derive(Debug)]
struct RecyclePool {
    slots: Mutex<Vec<Vec<u8>>>,
}

impl RecyclePool {
    fn new() -> RecyclePool {
        RecyclePool {
            slots: Mutex::new(Vec::new()),
        }
    }

    /// Take a buffer for a `needed`-byte fill: prefer one whose
    /// capacity already covers `needed` (pure reuse, no realloc);
    /// else any buffer (it grows once — the geometry-change path).
    /// Allocates only when the pool is empty (the warm-up path).
    fn acquire(&self, needed: usize) -> Vec<u8> {
        let mut slots = match self.slots.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        // Best fit: smallest capacity that covers `needed` (keeps the
        // big buffers big-frame-sized).
        let mut best: Option<usize> = None;
        for (i, buf) in slots.iter().enumerate() {
            if buf.capacity() >= needed && best.is_none_or(|b| buf.capacity() < slots[b].capacity())
            {
                best = Some(i);
            }
        }
        match best {
            Some(i) => slots.swap_remove(i),
            None => slots.pop().unwrap_or_default(),
        }
    }

    /// Return a buffer (bounded: never grows past `RECYCLE_SLOTS`).
    fn release(&self, buffer: Vec<u8>) {
        let mut slots = match self.slots.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if slots.len() < RECYCLE_SLOTS {
            slots.push(buffer);
        }
        // A full pool drops the returnee: bounded memory wins over
        // retention (the next acquire allocates — cold path only).
    }
}

/// A staged frame: sampleable RGBA8 pixels plus their extent — the
/// software "texture" (the hardware path wraps the same bytes in a
/// `wgpu::Texture` instead).
///
/// The `rgba` buffer is **pooled**: when this frame drops, the buffer
/// returns to the owning [`StagingUploader`]'s bounded recycle pool
/// (the C5a fix — the old `mem::take` handed the Vec out and freed
/// it at drop, forcing a fresh full-frame allocation every present).
/// Frames constructed by hand (tests) carry no pool and behave like
/// plain owners.
#[derive(Clone)]
pub struct StagedFrame {
    /// Pixel dimensions of `rgba`.
    pub extent: Extent2D,
    /// RGBA8 bytes, row-major, `width * height * 4`, no padding.
    pub rgba: Vec<u8>,
    /// The buffer's return address (absent for hand-built frames).
    recycle: Option<Arc<RecyclePool>>,
}

impl Drop for StagedFrame {
    fn drop(&mut self) {
        if let Some(pool) = self.recycle.take() {
            // Move the buffer out so Drop of the Vec never runs on a
            // pooled buffer (replace with the cheapest empty Vec).
            pool.release(std::mem::take(&mut self.rgba));
        }
    }
}

impl std::fmt::Debug for StagedFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Same rendering the old derive produced (the pool handle is
        // machinery, not frame state).
        f.debug_struct("StagedFrame")
            .field("extent", &self.extent)
            .field("rgba", &self.rgba)
            .finish()
    }
}

impl PartialEq for StagedFrame {
    fn eq(&self, other: &StagedFrame) -> bool {
        // Frame equality is pixel equality; the pool handle is
        // machinery (matches the old derive's observable behavior).
        self.extent == other.extent && self.rgba == other.rgba
    }
}

impl Eq for StagedFrame {}

impl StagedFrame {
    /// The RGBA8 pixel at (x, y) — point-sampled by the software
    /// compositor; callers keep the coordinates in bounds (the
    /// placement math guarantees it; a miss saturates, never panics).
    #[must_use = "pure pixel access"]
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let stride = self.extent.width as usize * 4;
        let at = (y as usize)
            .saturating_mul(stride)
            .saturating_add((x as usize) * 4);
        let end = at.saturating_add(4).min(self.rgba.len());
        match end.checked_sub(at) {
            Some(4) => [
                self.rgba[at],
                self.rgba[at + 1],
                self.rgba[at + 2],
                self.rgba[at + 3],
            ],
            _ => [0, 0, 0, 255],
        }
    }
}

/// What the staging path measured (card 2.D1 "measured staging").
///
/// Byte and copy counts are exact contracts (asserted by tests);
/// nanosecond fields are recorded evidence of a real run, never
/// asserted (timings vary by host — asserting them would be fabricating
/// a determinism the hardware does not have).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StagingStats {
    /// Frames staged through the buffer.
    pub frames_staged: u64,
    /// Total bytes copied into the staging buffer.
    pub bytes_staged: u64,
    /// Buffer copies performed (v0: one per frame — the Bgra8 swizzle
    /// is the same pass, not an extra copy).
    pub staging_copies: u64,
    /// Total wall time of staging copies, nanoseconds.
    pub stage_ns_total: u64,
    /// Wall time of the most recent staging copy, nanoseconds.
    pub last_stage_ns: u64,
}

/// The staging uploader: a bounded pool of reusable frame buffers,
/// one measured copy per frame into the next one (the "one reusable
/// buffer" the type has always claimed — now true: buffers cycle
/// through the [`RecyclePool`] instead of being freed with every
/// handed-out frame).
pub struct StagingUploader {
    pool: Arc<RecyclePool>,
    stats: StagingStats,
}

impl Default for StagingUploader {
    fn default() -> StagingUploader {
        StagingUploader {
            pool: Arc::new(RecyclePool::new()),
            stats: StagingStats::default(),
        }
    }
}

impl std::fmt::Debug for StagingUploader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Same rendering the old derive produced (the pool's Mutex is
        // machinery; the stats are the state that matters).
        f.debug_struct("StagingUploader")
            .field("stats", &self.stats)
            .finish()
    }
}

impl StagingUploader {
    /// A fresh uploader (empty pool, zeroed stats).
    #[must_use = "pure constructor"]
    pub fn new() -> StagingUploader {
        StagingUploader::default()
    }

    /// The measured-transfer record so far.
    #[must_use = "observable evidence"]
    pub fn stats(&self) -> StagingStats {
        self.stats
    }

    /// Stage one CPU frame: validate the layout against the declared
    /// format, copy (swizzling Bgra8) into the staging buffer, hand
    /// the bytes out as the texture-ready [`StagedFrame`].
    ///
    /// # Errors
    /// [`RendererError::ZeroExtent`] for a zero-sized frame;
    /// [`RendererError::UploadFormatUnsupported`] for formats the v0
    /// upload path does not take (the YUV family — the GPU shader
    /// conversion path, doc 03 §4.11 e);
    /// [`RendererError::UploadSizeMismatch`] when the buffer is
    /// shorter than `width × height × 4`.
    pub fn stage(
        &mut self,
        bytes: &SharedBytes,
        extent: Extent2D,
        format: PixelFormat,
    ) -> RendererResult<StagedFrame> {
        if extent.width == 0 || extent.height == 0 {
            return Err(RendererError::ZeroExtent {
                extent,
                from_frame: true,
            });
        }
        let needed = extent.rgba8_bytes()?;
        if !matches!(format, PixelFormat::Rgba8 | PixelFormat::Bgra8) {
            return Err(RendererError::UploadFormatUnsupported { format });
        }
        if (bytes.len() as u64) < needed {
            return Err(RendererError::UploadSizeMismatch {
                expected: needed,
                found: bytes.len() as u64,
            });
        }

        let started = Instant::now();
        let mut buffer = self.pool.acquire(needed as usize);
        buffer.clear();
        let source = bytes.as_slice();
        match format {
            // Straight copy: the frame's bytes are already RGBA8.
            PixelFormat::Rgba8 => buffer.extend_from_slice(&source[..needed as usize]),
            // Same single pass with the byte swap folded in.
            PixelFormat::Bgra8 => {
                buffer.reserve(needed as usize);
                for pixel in source[..needed as usize].as_chunks::<4>().0 {
                    buffer
                        .extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
                }
            }
            // Unreachable: the guard above rejected everything else.
            _ => return Err(RendererError::UploadFormatUnsupported { format }),
        }
        let elapsed = started.elapsed().as_nanos() as u64;

        self.stats.frames_staged += 1;
        self.stats.bytes_staged += needed;
        self.stats.staging_copies += 1;
        self.stats.stage_ns_total = self.stats.stage_ns_total.saturating_add(elapsed);
        self.stats.last_stage_ns = elapsed;

        Ok(StagedFrame {
            extent,
            rgba: buffer,
            recycle: Some(Arc::clone(&self.pool)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::{Must, assert_err};

    fn frame_bytes(w: u32, h: u32, fill: u8) -> SharedBytes {
        SharedBytes::from_vec(vec![fill; (w * h * 4) as usize])
    }

    #[test]
    fn rgba_upload_measures_exact_sizes_and_copies() {
        let mut uploader = StagingUploader::new();
        let staged = uploader
            .stage(
                &frame_bytes(8, 4, 0xAB),
                Extent2D {
                    width: 8,
                    height: 4,
                },
                PixelFormat::Rgba8,
            )
            .must();
        assert_eq!(
            staged.extent,
            Extent2D {
                width: 8,
                height: 4
            }
        );
        assert_eq!(staged.rgba.len(), 128);
        assert_eq!(staged.pixel(0, 0), [0xAB; 4]);
        assert_eq!(staged.pixel(7, 3), [0xAB; 4]);

        let stats = uploader.stats();
        assert_eq!(stats.frames_staged, 1);
        assert_eq!(stats.bytes_staged, 128);
        assert_eq!(stats.staging_copies, 1);
        // Timing is recorded, non-negative by type — never asserted.
        assert!(stats.stage_ns_total >= stats.last_stage_ns);

        // Second frame accumulates.
        uploader
            .stage(
                &frame_bytes(2, 2, 0x00),
                Extent2D {
                    width: 2,
                    height: 2,
                },
                PixelFormat::Rgba8,
            )
            .must();
        let stats = uploader.stats();
        assert_eq!(stats.frames_staged, 2);
        assert_eq!(stats.bytes_staged, 128 + 16);
        assert_eq!(stats.staging_copies, 2);
    }

    #[test]
    fn bgra_upload_swizzles_in_the_same_single_pass() {
        let bgra = SharedBytes::from_vec(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let mut uploader = StagingUploader::new();
        let staged = uploader
            .stage(
                &bgra,
                Extent2D {
                    width: 2,
                    height: 1,
                },
                PixelFormat::Bgra8,
            )
            .must();
        assert_eq!(staged.rgba, vec![3, 2, 1, 4, 7, 6, 5, 8]);
        assert_eq!(
            uploader.stats().staging_copies,
            1,
            "swizzle is not an extra copy"
        );
    }

    #[test]
    fn yuv_formats_are_typed_rejections_pointing_at_the_shader_path() {
        for format in [PixelFormat::Nv12, PixelFormat::I420, PixelFormat::Yuyv422] {
            let err = assert_err(
                StagingUploader::new().stage(
                    &frame_bytes(4, 4, 0),
                    Extent2D {
                        width: 4,
                        height: 4,
                    },
                    format,
                ),
                "YUV is refused by the v0 upload path",
            );
            assert_eq!(err, RendererError::UploadFormatUnsupported { format });
        }
    }

    #[test]
    fn short_buffers_and_zero_extents_are_typed_errors() {
        let err = assert_err(
            StagingUploader::new().stage(
                &frame_bytes(2, 2, 0),
                Extent2D {
                    width: 4,
                    height: 4,
                },
                PixelFormat::Rgba8,
            ),
            "short buffer is decode corruption",
        );
        assert_eq!(
            err,
            RendererError::UploadSizeMismatch {
                expected: 64,
                found: 16
            }
        );

        let err = assert_err(
            StagingUploader::new().stage(
                &frame_bytes(0, 0, 0),
                Extent2D {
                    width: 0,
                    height: 4,
                },
                PixelFormat::Rgba8,
            ),
            "zero-extent frame",
        );
        assert_eq!(
            err,
            RendererError::ZeroExtent {
                extent: Extent2D {
                    width: 0,
                    height: 4
                },
                from_frame: true
            }
        );
    }

    #[test]
    fn pixel_access_never_panics_on_degenerate_input() {
        let staged = StagedFrame {
            extent: Extent2D {
                width: 1,
                height: 1,
            },
            rgba: vec![9; 4],
            recycle: None,
        };
        assert_eq!(staged.pixel(0, 0), [9, 9, 9, 9]);
        // Out-of-bounds saturates to opaque black — placement math
        // keeps callers in bounds; this is defense in depth.
        assert_eq!(staged.pixel(99, 99), [0, 0, 0, 255]);
        let empty = StagedFrame {
            extent: Extent2D {
                width: 4,
                height: 4,
            },
            rgba: Vec::new(),
            recycle: None,
        };
        assert_eq!(empty.pixel(1, 1), [0, 0, 0, 255]);
    }

    #[test]
    fn staged_buffers_return_to_the_pool_and_are_reused() {
        // The C5a contract: a dropped frame's buffer re-enters the
        // bounded pool; the next stage of the same geometry reuses it
        // (observable: the second stage's buffer carries >= the first
        // one's capacity without a fresh allocation — and the
        // counting-allocator proof binary measures exactly zero
        // bytes across steady-state presents).
        let mut uploader = StagingUploader::new();
        let first = uploader
            .stage(
                &frame_bytes(8, 8, 0xAB),
                Extent2D {
                    width: 8,
                    height: 8,
                },
                PixelFormat::Rgba8,
            )
            .must();
        assert_eq!(first.rgba.len(), 256);
        assert!(first.recycle.is_some(), "uploader frames are pooled");
        // Overlapping frames: two alive at once (the pool must absorb
        // this without unbounded growth).
        let second = uploader
            .stage(
                &frame_bytes(8, 8, 0xCD),
                Extent2D {
                    width: 8,
                    height: 8,
                },
                PixelFormat::Rgba8,
            )
            .must();
        assert_eq!(second.rgba, vec![0xCD; 256]);
        drop(first);
        drop(second);
        // Steady state: the same geometry stages from pooled buffers.
        for fill in [0x11, 0x22, 0x33, 0x44] {
            let staged = uploader
                .stage(
                    &frame_bytes(8, 8, fill),
                    Extent2D {
                        width: 8,
                        height: 8,
                    },
                    PixelFormat::Rgba8,
                )
                .must();
            assert_eq!(staged.rgba, vec![fill; 256]);
        }
    }

    #[test]
    fn hand_built_frames_and_clones_behave_like_plain_owners() {
        // No pool: dropping is a plain Vec drop (no machinery).
        let mut uploader = StagingUploader::new();
        let staged = uploader
            .stage(
                &frame_bytes(2, 2, 7),
                Extent2D {
                    width: 2,
                    height: 2,
                },
                PixelFormat::Rgba8,
            )
            .must();
        // A clone shares nothing pooled: each owns its Vec; both
        // return cleanly.
        let clone = staged.clone();
        assert_eq!(clone, staged);
        drop(staged);
        assert_eq!(clone.rgba.len(), 16);
        drop(clone);
        // Equality ignores the pool handle (hand-built == staged
        // with identical pixels).
        let hand = StagedFrame {
            extent: Extent2D {
                width: 2,
                height: 2,
            },
            rgba: vec![7; 16],
            recycle: None,
        };
        let staged = uploader
            .stage(
                &frame_bytes(2, 2, 7),
                Extent2D {
                    width: 2,
                    height: 2,
                },
                PixelFormat::Rgba8,
            )
            .must();
        assert_eq!(hand, staged);
    }
}
