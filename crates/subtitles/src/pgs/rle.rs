//! PGS RLE — the object bitmap decode (card 5.N1).
//!
//! The code table, verified against the reference decoder
//! (ffmpeg `pgssubdec.c` `decode_rle`, fetched 2026-10-25):
//!
//! ```text
//! nn                one pixel of palette index nn          (nn != 0)
//! 00 00             end of line
//! 00 0L             run of L pixels of index 0             (L = 1..63)
//! 00 4L LL          run of (L<<8 | LL) pixels of index 0   (long run)
//! 00 8L cc          run of L pixels of index cc            (L = 1..63)
//! 00 CL LL cc       run of (L<<8 | LL) pixels of index cc  (long run)
//! ```
//!
//! (`L` = the low 6 bits of the flags byte; bit 6 selects the
//! 16-bit length form, bit 7 the explicit color byte.)
//!
//! # Divergences from the reference (deliberate, typed)
//!
//! ffmpeg writes into a FLAT `w*h` buffer and lets a short line
//! shift every following line sideways (it only logs); a run past
//! `w*h` is silently dropped. Both are hostile-input paths here:
//! a short line SNAPS to the next row boundary and a crossing run
//! is CLIPPED at the canvas edge — each recorded once per object in
//! [`RleDamage`] (the caller's `RleOverrun` ledger entry), because
//! a subtitle renderer must never smear pixels to the wrong row.
//! The canvas cap bounds allocation (D-2).

/// Object pixel-buffer cap: width × height indices, 4 MiB — twice a
/// 1080p canvas (Blu-ray objects are at most the video extent),
/// tight against hostile dimensions (D-2, the DVB precedent).
pub const MAX_OBJECT_PIXELS: u32 = 4 * 1024 * 1024;

/// One RLE decode's typed damage summary (aggregated into the
/// decoder's `RleOverrun` ledger entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RleDamage {
    /// Runs clipped at the canvas edge.
    pub clipped_runs: u32,
    /// End-of-lines that arrived before the row was full.
    pub short_lines: u32,
    /// The RLE data ended before the canvas was full (the tail is
    /// transparent black).
    pub truncated: bool,
}

impl RleDamage {
    /// Anything to report?
    pub fn any(&self) -> bool {
        self.clipped_runs > 0 || self.short_lines > 0 || self.truncated
    }
}

/// Decode one object's RLE into `width × height` palette indices
/// (0 = transparent black entry). Never panics; every hostile byte
/// sequence lands in the returned damage summary.
///
/// `data` is the FULL reassembled object data (all ODS fragments
/// concatenated, without the 7-byte first-fragment header).
pub fn decode_object(data: &[u8], width: u32, height: u32) -> (Vec<u8>, RleDamage) {
    let n = width as usize * height as usize;
    let mut out = vec![0u8; n];
    let mut damage = RleDamage::default();
    let w = width as usize;

    let mut pos = 0usize; // write cursor into `out`
    let mut i = 0usize; // read cursor into `data`
    while i < data.len() && pos < n {
        let b = data[i];
        i += 1;
        let (run, color) = if b != 0x00 {
            // One literal pixel.
            (1usize, b)
        } else {
            // 0x00 escape: the flags byte carries the run.
            let Some(&flags) = data.get(i) else {
                break;
            };
            i += 1;
            let mut run = (flags & 0x3F) as usize;
            if flags & 0x40 != 0 {
                // Long form: the low length byte follows.
                let Some(&lo) = data.get(i) else {
                    break;
                };
                i += 1;
                run = (run << 8) | lo as usize;
            }
            let color = if flags & 0x80 != 0 {
                let Some(&c) = data.get(i) else {
                    break;
                };
                i += 1;
                c
            } else {
                0
            };
            (run, color)
        };

        if run == 0 {
            // End of line.
            let col = pos % w;
            if col > 0 {
                damage.short_lines += 1;
                // Snap to the next row boundary (see module docs).
                pos += w - col;
            } else if pos == n {
                break;
            }
            continue;
        }

        let take = run.min(n - pos);
        if take < run {
            damage.clipped_runs += 1;
        }
        out[pos..pos + take].fill(color);
        pos += take;
    }

    if pos < n {
        damage.truncated = true;
    }
    // Leftover bytes past a full canvas (typically the legal final
    // EOL after the last row) are ignored — the reference behavior.
    (out, damage)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Row-major index grid helper.
    fn px(w: usize, x: usize, y: usize) -> usize {
        y * w + x
    }

    #[test]
    fn literal_pixels_and_color_runs() {
        // 4x2: row 1 = literals + a 1-run; row 2 = a 2-run + literal.
        let data = [
            0x01, 0x02, 0x03, 0x00, 0x81, 0x07, 0x00, 0x00, //
            0x00, 0x82, 0x07, 0x01, 0x09,
        ];
        let (img, dmg) = decode_object(&data, 4, 2);
        assert!(!dmg.any(), "clean decode, got {dmg:?}");
        assert_eq!(&img[0..4], &[1, 2, 3, 7]);
        assert_eq!(&img[4..8], &[7, 7, 1, 9]);
    }

    #[test]
    fn eol_and_short_color_runs() {
        // 4x2, line 1: color-0 run of 3 (`00 03` escape) then EOL
        // (short line, snaps); line 2: 4-run of 5.
        let data = [0x00, 0x03, 0x00, 0x00, 0x00, 0x84, 0x05];
        let (img, dmg) = decode_object(&data, 4, 2);
        assert_eq!(dmg.short_lines, 1);
        assert!(!dmg.truncated);
        assert_eq!(&img[0..4], &[0, 0, 0, 0], "row 0 zero-filled");
        assert_eq!(&img[4..8], &[5, 5, 5, 5], "row 1 the run");
    }

    #[test]
    fn long_runs_both_colors() {
        // 16-bit length forms: 00 4L LL (index 0) fills 256 px
        // exactly; 00 CL LL cc = a (L<<8|LL) run of cc.
        let (img, dmg) = decode_object(&[0x00, 0x41, 0x00], 16, 16);
        assert!(!dmg.any(), "exact 256-px long run, got {dmg:?}");
        assert!(img.iter().all(|&v| v == 0));
        // 00 C0 05 09: run of 5 nines into a 5-px canvas.
        let (img, dmg) = decode_object(&[0x00, 0xC0, 0x05, 0x09], 5, 1);
        assert!(!dmg.any());
        assert_eq!(img, [9, 9, 9, 9, 9]);
        // The 8-bit length form with 0x80: 00 83 cc = 3 of cc.
        let data = [0x00, 0x83, 0xAB];
        let (img, dmg) = decode_object(&data, 3, 1);
        assert!(!dmg.any());
        assert_eq!(img, [0xAB, 0xAB, 0xAB]);
    }

    #[test]
    fn overrun_run_is_clipped_and_typed() {
        // A 512-pixel run into a 4-pixel canvas: clipped, but the
        // canvas IS full (no truncation flag).
        let data = [0x00, 0x42, 0x00];
        let (img, dmg) = decode_object(&data, 2, 2);
        assert_eq!(dmg.clipped_runs, 1);
        assert!(!dmg.truncated, "the clipped run filled the canvas");
        assert_eq!(img, [0, 0, 0, 0]);
        assert!(dmg.any());
    }

    #[test]
    fn short_line_snaps_to_next_row() {
        // 4x2: one literal then EOL then one literal — the second
        // literal lands at row 1 col 0, NOT row 0 col 1.
        let data = [0x07, 0x00, 0x00, 0x08];
        let (img, dmg) = decode_object(&data, 4, 2);
        assert_eq!(dmg.short_lines, 1);
        assert_eq!(img[px(4, 0, 0)], 7);
        assert_eq!(img[px(4, 1, 0)], 0);
        assert_eq!(img[px(4, 0, 1)], 8);
        assert_eq!(img[px(4, 1, 1)], 0);
    }

    #[test]
    fn truncated_data_pads_transparent() {
        let (img, dmg) = decode_object(&[0x00], 4, 2);
        assert!(dmg.truncated);
        assert_eq!(img, vec![0u8; 8]);
        assert_eq!(decode_object(&[], 2, 2).0, vec![0u8; 4]);
    }

    #[test]
    fn trailing_bytes_past_full_canvas_are_ignored() {
        // Fill 2x1 then a trailing EOL (the legal final-EOL shape):
        // ignored, the reference behavior — no damage.
        let data = [0x01, 0x02, 0x00, 0x00, 0x00, 0x00];
        let (img, dmg) = decode_object(&data, 2, 1);
        assert_eq!(img, [1, 2]);
        assert!(!dmg.any(), "trailing EOL is legal, got {dmg:?}");
    }

    #[test]
    fn zero_dimensions_decode_to_empty() {
        let (img, dmg) = decode_object(&[0x01, 0x02], 0, 0);
        assert!(img.is_empty());
        assert!(dmg.truncated || !dmg.any());
    }
}
