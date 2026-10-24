//! The compositor driver: timing filter + z-order + placement + the
//! reference blend, with a persistent-scratch steady state (zero
//! allocation per frame after warm-up — proven by the counting-
//! allocator AC, the 3.B3/W2-07 discipline).
//!
//! The compositor owns NO cue data: callers pass the decoded cue
//! slice per frame (the engine glue's job once subtitle frames flow);
//! what persists here is exactly two scratch buffers, capacity-reused
//! — `order` (the active (z, index) pairs) and `xmap` (one cue's
//! nearest-neighbor source-column map, rebuilt per cue because it is
//! geometry-shaped, never grown in the steady state).

use crate::compositing::bitmap::CompositedCue;
use crate::compositing::layout::place_cue;
use crate::compositing::target::{ComposeError, ComposeTarget, blend_row, clip};
use media_types::time::Timestamp;

/// Drives the overlay pipeline: for a clock position, draw every
/// active cue onto the target in z-order.
#[derive(Default)]
pub struct SubtitleCompositor {
    /// Active `(z, cue index)` pairs, ascending by the tuple —
    /// z first, then insertion order (later cue wins ties).
    order: Vec<(u16, u32)>,
    /// Nearest-neighbor source-column map for the cue being blended:
    /// `xmap[dx] = floor((2·dx + 1)·src_w / (2·dst_w))` — pixel-center
    /// mapping, exact identity at scale 1:1, one division per cue
    /// column instead of per pixel.
    xmap: Vec<u32>,
}

impl std::fmt::Debug for SubtitleCompositor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubtitleCompositor")
            .field("order.len", &self.order.len())
            .field("xmap.cap", &self.xmap.capacity())
            .finish()
    }
}

impl SubtitleCompositor {
    /// A fresh compositor (empty scratch — the cues carry all
    /// per-frame state).
    #[must_use = "an unused compositor composes nothing"]
    pub fn new() -> SubtitleCompositor {
        SubtitleCompositor::default()
    }

    /// Draw every cue in `cues` active at `now` onto `target`,
    /// z-ascending (higher z on top; same z: later slice entry on
    /// top). Returns how many cues were drawn.
    ///
    /// `now` is the shared clock's position — the same
    /// `PlaybackClock::now()` timestamp the video scheduler holds its
    /// frames against (doc 03 §4.7/§4.10): subtitles and video swap on
    /// one clock, never two.
    ///
    /// # Errors
    /// [`ComposeError`] — an empty target extent, an unreadable clock
    /// position (over/underflow), or a target row-contract violation.
    /// A cue whose placement falls entirely outside the target is
    /// skipped, not an error (data, not a failure).
    pub fn compose(
        &mut self,
        cues: &[CompositedCue],
        now: Timestamp,
        target: &mut dyn ComposeTarget,
    ) -> Result<usize, ComposeError> {
        let extent = target.extent();
        if extent.0 == 0 || extent.1 == 0 {
            return Err(ComposeError::ZeroExtent { extent });
        }
        let now_ns = now.as_nanos()?.0 as i64;

        // 1. Timing filter into persistent scratch (clear keeps
        //    capacity — zero alloc after warm-up).
        self.order.clear();
        for (i, cue) in cues.iter().enumerate() {
            if cue.active_at(now_ns) {
                self.order.push((cue.z, i as u32));
            }
        }
        // 2. Z-order: sort_unstable is in-place and allocation-free;
        //    the index in the tuple is the explicit stable tie-break.
        self.order.sort_unstable();

        // 3. Blend each active cue, bottom z first (index loop: the
        //    tuple is `Copy`, so no borrow of `self.order` outlives
        //    the `&mut self` the blend needs).
        for slot in 0..self.order.len() {
            let (_, i) = self.order[slot];
            let cue = &cues[i as usize];
            let rect = place_cue(&cue.bitmap, &cue.layout, extent)?;
            let Some(dst) = clip(rect, extent) else {
                continue;
            };
            self.blend_one(cue, rect, dst, target)?;
        }
        Ok(self.order.len())
    }

    /// Blend one cue: nearest-neighbor rows onto the target rows.
    fn blend_one(
        &mut self,
        cue: &CompositedCue,
        placed: crate::compositing::layout::Rect,
        dst: crate::compositing::layout::Rect,
        target: &mut dyn ComposeTarget,
    ) -> Result<(), ComposeError> {
        let bmp = &cue.bitmap;
        let sw = bmp.width;
        let sh = bmp.height;
        let dw = placed.width as u64;
        let dh = placed.height as u64;

        // The source-column map for the FULL placed width (the clip
        // then slices it — clipped pixels need no map entries).
        self.xmap.clear();
        for dx in 0..dw {
            let sx = ((2 * dx + 1) * sw as u64) / (2 * dw);
            self.xmap.push(sx.min(sw as u64 - 1) as u32);
        }
        let xs = (dst.x - placed.x) as usize;
        let xe = (dst.x + dst.width - placed.x) as usize;

        let stride = sw as usize * 4;
        for y in dst.y..dst.y + dst.height {
            let dy = (y - placed.y) as u64;
            let sy = (((2 * dy + 1) * sh as u64) / (2 * dh)).min(sh as u64 - 1) as usize;
            let src_row = &bmp.rgba[sy * stride..sy * stride + stride];
            let dst_row = target.row_rgba(y)?;
            blend_row(dst_row, dst.x as usize, &self.xmap[xs..xe], src_row)?;
        }
        Ok(())
    }
}
