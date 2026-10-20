//! The baseline scan decoder and output assembly (card 2.B4 core):
//! MCU walk, Huffman entropy decode, DC prediction, restart markers,
//! concealment, and the I420 / RGBA output paths.
//!
//! # Concelment policy (documented)
//!
//! Once the marker walk (parser) succeeds, the frame has enough
//! structure to be emitted honestly. Entropy-level damage — data
//! exhaustion, a Huffman code absent from the table, a coefficient
//! run past 63, a missing/misnumbered restart marker — does not fail
//! the frame: the undecodable blocks **from the failure point on**
//! are filled with mid-gray (the IDCT of zero coefficients, matching
//! libjpeg's missing-MCU behavior), the frame is emitted, and the
//! caller is told via `concealed = true` (the contract surfaces this
//! as `DecodeOutcome::Recovered { ConcealedFrame }` — bounded,
//! observable, never silent).
//!
//! Header-level damage (no SOI, no SOF, bad segments) emits nothing
//! and returns a typed error: fabricating a frame from nothing would
//! not be concealment.

use super::bitreader::{BitReader, ReaderStop};
use super::color::{ComponentGeom, PlaneView, RgbComposer};
use super::error::MjpegError;
use super::huffman::{HuffmanDecoder, Stop};
use super::idct::idct_block;
use super::parser::{self, ComponentInfo, JpegInfo, ZIGZAG};

/// Pixel-count cap — the fuzz-safety memory bound.
///
/// 8 MiP (8_388_608 pixels, e.g. 3840×2176 — full UHD) keeps the
/// worst-case decode footprint (padded component planes + a 4-byte
/// RGBA output) under the corpus' 64 MiB rss budget
/// `[PROPOSAL: the number; the invariant is the bound itself]`.
pub(crate) const MAX_PIXELS: u64 = 8_388_608;

/// Output format of a decoded frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    /// Planar 4:2:0: `Y (w×h) || Cb (⌈w/2⌉×⌈h/2⌉) || Cr` — codec-native
    /// (doc 01 §3.3: I420 is "the codec-native default"), zero
    /// conversion for 4:2:0 streams and the neutral-chroma
    /// representation of grayscale.
    I420,
    /// Interleaved RGBA (straight alpha, 0xff), for samplings with no
    /// planar vocabulary yet (4:4:4 / 4:2:2 / 4:4:0).
    Rgba8,
}

/// Why entropy decoding stopped early (concealment observability —
/// doc 03 §4.5(d): recovery is bounded *and observable*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConcealCause {
    /// The entropy-coded data ended before the scan completed
    /// (truncation).
    EndOfData,
    /// A marker terminated the scan before all MCUs were decoded.
    PrematureMarker(u8),
    /// A Huffman code absent from the table / a coefficient run past
    /// 63.
    InvalidCode,
    /// A restart marker was missing or misnumbered.
    RestartMismatch,
}

/// A decoded frame (internal form; `mod.rs` lifts it into `Frame`).
pub(crate) struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub format: OutputFormat,
    pub data: Vec<u8>,
    /// Entropy-level damage was concealed (see module docs).
    pub concealed: bool,
    /// The damage class when `concealed`.
    pub cause: Option<ConcealCause>,
}

/// One padded per-component sample plane.
struct Plane {
    data: Vec<u8>,
    /// Padded row pitch (bytes).
    stride: usize,
    /// Blocks per padded row (MCU grid width × h_i).
    block_cols: usize,
    /// Padded block rows (MCU grid height × v_i).
    block_rows: usize,
    /// Valid sample dimensions (component width/height).
    valid_w: usize,
    valid_h: usize,
}

impl Plane {
    /// Write one decoded block's samples at block position
    /// `(col, row)`.
    fn write_block(&mut self, samples: &[u8; 64], col: usize, row: usize) {
        let x0 = col * 8;
        let y0 = row * 8;
        for y in 0..8 {
            let Some(dst_row) = self.data.get_mut((y0 + y) * self.stride..) else {
                return;
            };
            let Some(dst) = dst_row.get_mut(x0..x0 + 8) else {
                return;
            };
            if let Some(src) = samples.get(y * 8..y * 8 + 8) {
                dst.copy_from_slice(src);
            }
        }
    }

    /// Concealment: fill `from_block..total` with mid-gray (128 = the
    /// IDCT of zero coefficients).
    fn gray_blocks(&mut self, from_block: usize) {
        let total = self.block_cols * self.block_rows;
        for index in from_block..total {
            let col = index % self.block_cols;
            let row = index / self.block_cols;
            let x0 = col * 8;
            let y0 = row * 8;
            for y in 0..8 {
                let Some(dst_row) = self.data.get_mut((y0 + y) * self.stride..) else {
                    return;
                };
                if let Some(dst) = dst_row.get_mut(x0..x0 + 8) {
                    dst.fill(128);
                }
            }
        }
    }
}

/// Receive-and-extend an amplitude value of `s` bits (§F.2.2.1 EXTEND).
fn receive_extend(reader: &mut BitReader<'_>, s: u32) -> Result<i32, Stop> {
    let Some(bits) = reader.read_bits(s) else {
        return Err(Stop::Exhausted);
    };
    let v = bits as i32;
    if s == 0 {
        return Ok(0);
    }
    if v < (1 << (s - 1)) {
        Ok(v - (1 << s) + 1)
    } else {
        Ok(v)
    }
}

/// Decode one structure unit: one 8×8 block of one component
/// (§F.2.2.2 / F.2.2.3). `block` arrives zeroed.
fn decode_block(
    reader: &mut BitReader<'_>,
    dc_table: &HuffmanDecoder,
    ac_table: &HuffmanDecoder,
    dc_pred: &mut i64,
    block: &mut [i16; 64],
) -> Result<(), Stop> {
    let size = dc_table.decode(reader)?;
    let diff = if size == 0 {
        0
    } else {
        receive_extend(reader, u32::from(size))?
    };
    *dc_pred = dc_pred.saturating_add(i64::from(diff));
    // The DC predictor is clamped into i16 for storage; hostile
    // cumulative diffs cannot overflow (values far above the legit
    // ±2047 band saturate in the IDCT clip anyway).
    block[0] = (*dc_pred).clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16;
    let mut k = 1usize;
    while k < 64 {
        let rs = ac_table.decode(reader)?;
        let run = (rs >> 4) as usize;
        let size = (rs & 0x0f) as u32;
        if size == 0 {
            if run == 15 {
                // ZRL: sixteen zero coefficients.
                k += 16;
                continue;
            }
            break; // EOB
        }
        k += run;
        if k > 63 {
            return Err(Stop::InvalidCode);
        }
        let value = receive_extend(reader, size)?;
        if let Some(slot) = block.get_mut(ZIGZAG[k]) {
            *slot = value.clamp(-32768, 32767) as i16;
        }
        k += 1;
    }
    Ok(())
}

/// Decode one complete JPEG packet into a [`DecodedImage`].
pub(crate) fn decode(data: &[u8]) -> Result<DecodedImage, MjpegError> {
    let parsed = parser::parse(data)?;
    let info = &parsed.info;

    // Component-count semantics: JFIF grayscale (1) or YCbCr (3).
    // 2- and 4-component streams (e.g. Adobe CMYK/YCCK) are outside
    // the honest baseline subset.
    match info.components.len() {
        1 | 3 => {}
        other => {
            return Err(super::error::unsupported(format!(
                "{other} frame components (JFIF grayscale=1 / YCbCr=3 only)"
            )));
        }
    }

    // Limits (fuzz-safety memory bound).
    let w = u64::from(info.width);
    let h = u64::from(info.height);
    if w.saturating_mul(h) > MAX_PIXELS {
        return Err(MjpegError::LimitExceeded {
            limit: media_types::error::LimitKind::Memory,
            budget: MAX_PIXELS,
            attempted: w.saturating_mul(h),
        });
    }

    // Geometry (§A.1.1): MCU grid from the maximum sampling factors.
    let h_max = info.components.iter().map(|c| c.h).max().unwrap_or(1) as usize;
    let v_max = info.components.iter().map(|c| c.v).max().unwrap_or(1) as usize;
    let w = info.width as usize;
    let h = info.height as usize;
    let mcu_w = w.div_ceil(8 * h_max);
    let mcu_h = h.div_ceil(8 * v_max);

    // Per-component padded planes.
    let mut planes: Vec<Plane> = Vec::with_capacity(info.components.len());
    for comp in &info.components {
        let block_cols = mcu_w * comp.h as usize;
        let block_rows = mcu_h * comp.v as usize;
        let stride = block_cols
            .checked_mul(8)
            .ok_or_else(|| MjpegError::LimitExceeded {
                limit: media_types::error::LimitKind::Memory,
                budget: MAX_PIXELS,
                attempted: u64::try_from(block_cols.saturating_mul(8)).unwrap_or(u64::MAX),
            })?;
        let rows = block_rows * 8;
        let valid_w = (w * comp.h as usize).div_ceil(h_max);
        let valid_h = (h * comp.v as usize).div_ceil(v_max);
        let size = stride
            .checked_mul(rows)
            .ok_or_else(|| MjpegError::LimitExceeded {
                limit: media_types::error::LimitKind::Memory,
                budget: MAX_PIXELS,
                attempted: u64::try_from(rows).unwrap_or(u64::MAX).saturating_mul(1),
            })?;
        if u64::try_from(size).unwrap_or(u64::MAX) > MAX_PIXELS * 4 {
            return Err(MjpegError::LimitExceeded {
                limit: media_types::error::LimitKind::Memory,
                budget: MAX_PIXELS * 4,
                attempted: u64::try_from(size).unwrap_or(u64::MAX),
            });
        }
        planes.push(Plane {
            data: vec![0u8; size],
            stride,
            block_cols,
            block_rows,
            valid_w,
            valid_h,
        });
    }

    // Build the Huffman decoders (validated tables only — parse_sos
    // proved presence for the referenced selectors).
    let mut dc_tables: Vec<Option<HuffmanDecoder>> = Vec::with_capacity(4);
    let mut ac_tables: Vec<Option<HuffmanDecoder>> = Vec::with_capacity(4);
    for td in 0..4usize {
        dc_tables.push(if info.huff_present[0][td] {
            Some(
                match HuffmanDecoder::new(&info.huff_bits[0][td], &info.huff_vals[0][td]) {
                    Ok(table) => table,
                    Err(_) => return Err(super::error::corrupt_entropy("mjpeg.huffman.bad_table")),
                },
            )
        } else {
            None
        });
        ac_tables.push(if info.huff_present[1][td] {
            Some(
                match HuffmanDecoder::new(&info.huff_bits[1][td], &info.huff_vals[1][td]) {
                    Ok(table) => table,
                    Err(_) => return Err(super::error::corrupt_entropy("mjpeg.huffman.bad_table")),
                },
            )
        } else {
            None
        });
    }

    // Scan walk.
    let entropy = data.get(info.entropy_offset..).unwrap_or(&[]);
    let mut reader = BitReader::new(entropy);
    let mut dc_pred = [0i64; 4];
    let mut cursors: Vec<usize> = vec![0; info.components.len()];
    let mut concealed = false;
    let mut cause: Option<ConcealCause> = None;
    let restart_interval = info.restart_interval as usize;
    let mut restart_count: u32 = 0;

    'scan: for my in 0..mcu_h {
        for mx in 0..mcu_w {
            let mcu_index = my * mcu_w + mx;
            if restart_interval > 0 && mcu_index > 0 && mcu_index.is_multiple_of(restart_interval) {
                let expected = (restart_count & 7) as u8;
                if reader.take_restart(expected).is_err() {
                    concealed = true;
                    cause = Some(ConcealCause::RestartMismatch);
                    break 'scan;
                }
                restart_count += 1;
                dc_pred = [0; 4];
            }
            for scan_comp in &info.scan {
                let comp: ComponentInfo = info.components[scan_comp.sof_index];
                let dc_table = match &dc_tables[scan_comp.td as usize] {
                    Some(table) => table,
                    None => return Err(super::error::corrupt_entropy("mjpeg.huffman.no_dc_table")),
                };
                let ac_table = match &ac_tables[scan_comp.ta as usize] {
                    Some(table) => table,
                    None => return Err(super::error::corrupt_entropy("mjpeg.huffman.no_ac_table")),
                };
                let qt = &info.quant_tables[comp.quant_table as usize];
                for by in 0..comp.v as usize {
                    for bx in 0..comp.h as usize {
                        let mut block = [0i16; 64];
                        let pred = &mut dc_pred[scan_comp.sof_index.min(3)];
                        match decode_block(&mut reader, dc_table, ac_table, pred, &mut block) {
                            Ok(()) => {
                                let plane = &mut planes[scan_comp.sof_index];
                                let col = mx * comp.h as usize + bx;
                                let row = my * comp.v as usize + by;
                                let mut samples = [0u8; 64];
                                idct_block(&block, qt, &mut samples);
                                plane.write_block(&samples, col, row);
                                cursors[scan_comp.sof_index] += 1;
                            }
                            Err(stop) => {
                                // Entropy damage: conceal from here on
                                // (module docs: mid-gray, observable).
                                concealed = true;
                                cause = Some(match stop {
                                    Stop::Exhausted => match reader.stop() {
                                        Some(ReaderStop::Marker(code)) => {
                                            ConcealCause::PrematureMarker(code)
                                        }
                                        _ => ConcealCause::EndOfData,
                                    },
                                    Stop::InvalidCode => ConcealCause::InvalidCode,
                                });
                                break 'scan;
                            }
                        }
                    }
                }
            }
        }
    }

    if concealed {
        for scan_comp in &info.scan {
            let index = scan_comp.sof_index;
            planes[index].gray_blocks(cursors[index]);
        }
    }

    // Output assembly.
    let format = choose_format(info, h_max, v_max);
    let out = match format {
        OutputFormat::I420 => assemble_i420(&planes, w, h)?,
        OutputFormat::Rgba8 => assemble_rgba(&planes, info, h_max, v_max, w, h)?,
    };
    Ok(DecodedImage {
        width: info.width,
        height: info.height,
        format,
        data: out,
        concealed,
        cause,
    })
}

/// The output format for this sampling: I420 for 4:2:0 and grayscale
/// (neutral chroma), RGBA for the rest.
fn choose_format(info: &JpegInfo, h_max: usize, v_max: usize) -> OutputFormat {
    if info.components.len() == 1 {
        return OutputFormat::I420;
    }
    let luma = &info.components[0];
    let chroma_420 = info.components[1..].iter().all(|c| c.h == 1 && c.v == 1);
    if h_max == 2 && v_max == 2 && luma.h == 2 && luma.v == 2 && chroma_420 {
        OutputFormat::I420
    } else {
        OutputFormat::Rgba8
    }
}

/// Crop one plane's valid region into a compact plane.
fn crop_plane(plane: &Plane, out_w: usize, out_h: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(out_w * out_h);
    for y in 0..out_h.min(plane.valid_h) {
        let start = y * plane.stride;
        let end = start + out_w.min(plane.valid_w);
        if let Some(row) = plane.data.get(start..end) {
            out.extend_from_slice(row);
        } else {
            out.extend(std::iter::repeat_n(128u8, out_w));
        }
    }
    out
}

/// I420 assembly: `Y || Cb || Cr`, chroma at ⌈w/2⌉×⌈h/2⌉.
fn assemble_i420(planes: &[Plane], w: usize, h: usize) -> Result<Vec<u8>, MjpegError> {
    let cw = w.div_ceil(2);
    let ch = h.div_ceil(2);
    let luma_len = w.checked_mul(h).ok_or_else(|| limit_error(w * h))?;
    let chroma_len = cw.checked_mul(ch).ok_or_else(|| limit_error(cw * ch))?;
    let total = luma_len
        .checked_add(
            chroma_len
                .checked_mul(2)
                .ok_or_else(|| limit_error(chroma_len))?,
        )
        .ok_or_else(|| limit_error(luma_len))?;
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&crop_plane(&planes[0], w, h));
    if planes.len() == 1 {
        // Grayscale: neutral chroma, documented representation.
        out.extend(std::iter::repeat_n(128u8, chroma_len * 2));
    } else {
        out.extend_from_slice(&crop_plane(&planes[1], cw, ch));
        out.extend_from_slice(&crop_plane(&planes[2], cw, ch));
    }
    Ok(out)
}

/// RGBA assembly via the upsample + convert row composer.
fn assemble_rgba(
    planes: &[Plane],
    info: &JpegInfo,
    h_max: usize,
    v_max: usize,
    w: usize,
    h: usize,
) -> Result<Vec<u8>, MjpegError> {
    let total = w
        .checked_mul(h)
        .and_then(|px| px.checked_mul(4))
        .ok_or_else(|| limit_error(w * h * 4))?;
    let mut out = vec![0u8; total];
    let geom = |index: usize| ComponentGeom {
        h: info.components.get(index).map_or(1, |c| c.h as usize),
        v: info.components.get(index).map_or(1, |c| c.v as usize),
        valid_w: planes.get(index).map_or(1, |p| p.valid_w),
        valid_h: planes.get(index).map_or(1, |p| p.valid_h),
    };
    let chroma = if planes.len() == 1 {
        None
    } else {
        Some((
            PlaneView {
                data: &planes[1].data,
                stride: planes[1].stride,
            },
            PlaneView {
                data: &planes[2].data,
                stride: planes[2].stride,
            },
        ))
    };
    let composer = RgbComposer::new(
        PlaneView {
            data: &planes[0].data,
            stride: planes[0].stride,
        },
        chroma,
        [geom(0), geom(1), geom(2)],
        h_max,
        v_max,
    );
    for y in 0..h {
        let row_start = y.checked_mul(w * 4).ok_or_else(|| limit_error(y * w * 4))?;
        let Some(row) = out.get_mut(row_start..row_start + w * 4) else {
            break;
        };
        composer.compose_row(y, w, row);
    }
    Ok(out)
}

fn limit_error(attempted: usize) -> MjpegError {
    MjpegError::LimitExceeded {
        limit: media_types::error::LimitKind::Memory,
        budget: MAX_PIXELS,
        attempted: u64::try_from(attempted).unwrap_or(u64::MAX),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration-level tests of `decode` live in tests/ (corpus +
    // synthetic JPEGs); the unit tests here target the geometry and
    // assembly helpers with hand-built planes.

    fn plane_of(bytes: &[u8], w: usize) -> Plane {
        let h = bytes.len() / w;
        Plane {
            data: bytes.to_vec(),
            stride: w,
            block_cols: w.div_ceil(8),
            block_rows: h.div_ceil(8),
            valid_w: w,
            valid_h: h,
        }
    }

    #[test]
    fn crop_plane_trims_padding() {
        // A 10x2 valid region inside a 16-wide padded plane.
        let mut data = vec![9u8; 16 * 2];
        data[10..16].fill(255); // padding marker bytes
        data[16 + 10..16 + 16].fill(255);
        let plane = Plane {
            data,
            stride: 16,
            block_cols: 2,
            block_rows: 1,
            valid_w: 10,
            valid_h: 2,
        };
        let cropped = crop_plane(&plane, 10, 2);
        assert_eq!(cropped.len(), 20);
        assert!(cropped.iter().all(|&b| b == 9));
    }

    #[test]
    fn gray_blocks_fills_only_the_tail() {
        // 2x1 blocks: conceal from block 1 on.
        let mut plane = plane_of(&[7u8; 16], 16);
        plane.gray_blocks(1);
        assert_eq!(&plane.data[0..8], &[7u8; 8], "block 0 untouched");
        assert_eq!(&plane.data[8..16], &[128u8; 8], "block 1 gray");
    }

    #[test]
    fn receive_extend_decodes_negative_and_positive() {
        // s=2: values 0..1 → -3..-2; 2..3 → 2..3.
        let mut r = BitReader::new(&[0b00_10_11_00]);
        assert_eq!(receive_extend(&mut r, 2).ok(), Some(-3));
        assert_eq!(receive_extend(&mut r, 2).ok(), Some(2));
        assert_eq!(receive_extend(&mut r, 2).ok(), Some(3));
    }

    #[test]
    fn pixel_cap_rejects_oversize_frames() {
        // A complete header set declaring 8193×8193 (8 MiP + 1): the
        // parse succeeds, the pixel cap fires.
        let mut jpeg = vec![0xff, 0xd8];
        let seg = |jpeg: &mut Vec<u8>, marker: u8, payload: &[u8]| {
            jpeg.push(0xff);
            jpeg.push(marker);
            jpeg.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
            jpeg.extend_from_slice(payload);
        };
        // DQT: Tq=0, all 16s.
        let mut dqt = vec![0u8];
        dqt.extend(std::iter::repeat_n(16u8, 64));
        seg(&mut jpeg, 0xdb, &dqt);
        // DHT: DC table 0 with one symbol; AC table 0 with one symbol.
        for class in [0x00u8, 0x10u8] {
            let mut dht = vec![class];
            dht.extend_from_slice(&[1u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            dht.push(0x00);
            seg(&mut jpeg, 0xc4, &dht);
        }
        // SOF0: 8193×8193, one component, h1v1, Tq 0.
        let mut sof = vec![8u8];
        sof.extend_from_slice(&[0x20, 0x01, 0x20, 0x01, 1, 1, 0x11, 0]);
        seg(&mut jpeg, 0xc0, &sof);
        // SOS: one component, Td=0 Ta=0.
        seg(&mut jpeg, 0xda, &[1, 1, 0x00, 0, 63, 0]);
        jpeg.extend_from_slice(&[0x00, 0xff, 0xd9]);
        let e = match decode(&jpeg) {
            Err(e) => e,
            Ok(_) => unreachable!("oversize frame must be rejected"),
        };
        match e {
            MjpegError::LimitExceeded { attempted, .. } => {
                assert_eq!(attempted, 8193 * 8193);
            }
            other => unreachable!("expected LimitExceeded, got {other:?}"),
        }
    }
}
