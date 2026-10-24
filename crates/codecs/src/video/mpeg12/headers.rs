//! MPEG-1/2 header syntax (ISO/IEC 11172-2 §6.2.2–6.2.4 / 13818-2
//! §6.2.2–6.2.4): sequence header, sequence extension, GOP, picture
//! header, picture coding extension, and slice headers with
//! macroblock addressing.
//!
//! Every parser takes a **complete start-code unit payload** (the
//! bytes after the code byte, as [`StartCodeScan`](super::startcode::
//! StartCodeScan) yields them) and returns a typed
//! `Result` — clean-room from the two specs' syntax sections, no
//! foreign code consulted.

use super::bitreader::BitReader;
use super::error::Mpeg12Error;

/// Frame-rate code table (11172-2 Table 6-5 / 13818-2 Table 6-4):
/// code → (numerator, denominator) in frames per second.
#[must_use]
pub fn frame_rate_table(code: u32) -> Option<(u32, u32)> {
    match code {
        1 => Some((24_000, 1001)),
        2 => Some((24, 1)),
        3 => Some((25, 1)),
        4 => Some((30_000, 1001)),
        5 => Some((30, 1)),
        6 => Some((50, 1)),
        7 => Some((60_000, 1001)),
        8 => Some((60, 1)),
        _ => None,
    }
}

/// Chroma format (13818-2 §6.2.2.3 `chroma_format`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaFormat {
    /// 01 — the DVD/DVB default.
    Chroma420,
    /// 10.
    Chroma422,
    /// 11.
    Chroma444,
}

impl ChromaFormat {
    /// The ffprobe spelling (`yuv420p` and friends).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            ChromaFormat::Chroma420 => "yuv420p",
            ChromaFormat::Chroma422 => "yuv422p",
            ChromaFormat::Chroma444 => "yuv444p",
        }
    }
}

/// MPEG-2 profile half of `profile_and_level_indication` (13818-2
/// §6.3.5, Table 8-2). Un-modeled/reserved ids return `None`.
///
/// The escape bit: ffmpeg's `mpeg2video` encoder authors 4:2:2
/// streams as `0x85` (escape + profile bits `000` + level), the
/// de-facto broadcast spelling of the 4:2:2 profile's escape
/// encoding (H.262 amendment); ffprobe decodes it to `"4:2:2"` —
/// measured on this tree's fixtures and the corpus's 4:2:2 lanes
/// (worklog 5.H1). Modeled accordingly; other escaped ids stay
/// honestly `None`.
#[must_use]
pub fn profile_name(profile_and_level: u8) -> Option<&'static str> {
    if profile_and_level & 0x80 != 0 {
        return match (profile_and_level >> 4) & 0x7 {
            0 => Some("4:2:2"),
            _ => None,
        };
    }
    match (profile_and_level >> 4) & 0x7 {
        1 => Some("High"),
        2 => Some("Spatial"),
        3 => Some("SNR"),
        4 => Some("Main"),
        5 => Some("Simple"),
        6 => Some("Multiview"),
        7 => Some("4:2:2"),
        _ => None,
    }
}

/// MPEG-2 level half of `profile_and_level_indication` (13818-2
/// Table 8-2; the ffprobe `level` value, e.g. 8 = Main).
#[must_use]
pub fn level_value(profile_and_level: u8) -> u8 {
    profile_and_level & 0x0F
}

/// Picture coding type (11172-2 / 13818-2 §6.3.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureCodingType {
    /// 1 — intra-coded (keyframe).
    Intra,
    /// 2 — predictive (references one past picture; motion vectors).
    Predictive,
    /// 3 — bidirectional (references past + future; motion vectors).
    Bidirectional,
    /// 4 — MPEG-1 DC-intra only (11172-2 §2.4.1.1; never in MPEG-2).
    DcIntra,
}

impl PictureCodingType {
    /// The one-letter ffprobe/log spelling.
    #[must_use]
    pub fn letter(self) -> char {
        match self {
            PictureCodingType::Intra => 'I',
            PictureCodingType::Predictive => 'P',
            PictureCodingType::Bidirectional => 'B',
            PictureCodingType::DcIntra => 'D',
        }
    }

    /// True for the types whose macroblock syntax carries motion
    /// vectors (P and B; the "motion-vector syntax presence" probe
    /// signal — card 5.H1).
    #[must_use]
    pub fn has_motion_vectors(self) -> bool {
        matches!(
            self,
            PictureCodingType::Predictive | PictureCodingType::Bidirectional
        )
    }
}

/// sequence_header (11172-2 §6.2.2.1 / 13818-2 §6.2.2.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceHeader {
    /// `horizontal_size_value` (12 bits).
    pub horizontal_size_value: u32,
    /// `vertical_size_value` (12 bits).
    pub vertical_size_value: u32,
    /// `aspect_ratio_information` (4 bits, 1..=4; 5..=15 forbidden).
    pub aspect_ratio_information: u32,
    /// `frame_rate_code` (4 bits, 1..=8; 0 and 9..=15 forbidden).
    pub frame_rate_code: u32,
    /// `bit_rate_value` (18 bits, 0x3FFFF = variable).
    pub bit_rate_value: u32,
    /// `vbv_buffer_size_value` (10 bits).
    pub vbv_buffer_size_value: u32,
    /// MPEG-1 `constrained_parameters_flag` (shall be 0 in MPEG-2).
    pub constrained_parameters_flag: bool,
    /// The 64-entry intra quantizer matrix when loaded.
    pub intra_quantizer_matrix: Option<[u8; 64]>,
    /// The 64-entry non-intra quantizer matrix when loaded.
    pub non_intra_quantizer_matrix: Option<[u8; 64]>,
}

impl SequenceHeader {
    /// Parse one sequence-header unit payload (after `00 00 01 B3`).
    ///
    /// # Errors
    /// `Truncated`/`Marker` on damage; `OutOfRange` on a forbidden
    /// aspect-ratio or frame-rate code.
    pub fn parse(payload: &[u8]) -> Result<SequenceHeader, Mpeg12Error> {
        let mut r = BitReader::new(payload);
        let horizontal_size_value = r.read_bits(12)?;
        let vertical_size_value = r.read_bits(12)?;
        let aspect_ratio_information = r.read_bits(4)?;
        if !(1..=4).contains(&aspect_ratio_information) {
            return Err(Mpeg12Error::OutOfRange {
                field: "aspect_ratio_information",
                value: u64::from(aspect_ratio_information),
                code: "mpeg12.seq.aspect",
            });
        }
        let frame_rate_code = r.read_bits(4)?;
        if frame_rate_table(frame_rate_code).is_none() {
            return Err(Mpeg12Error::OutOfRange {
                field: "frame_rate_code",
                value: u64::from(frame_rate_code),
                code: "mpeg12.seq.frame_rate",
            });
        }
        let bit_rate_value = r.read_bits(18)?;
        r.read_marker("mpeg12.seq.marker")?;
        let vbv_buffer_size_value = r.read_bits(10)?;
        let constrained_parameters_flag = r.read_bit()? == 1;
        let load_intra = r.read_bit()? == 1;
        let intra_quantizer_matrix = if load_intra {
            let mut matrix = [0u8; 64];
            for slot in &mut matrix {
                *slot = r.read_bits(8)? as u8;
            }
            Some(matrix)
        } else {
            None
        };
        let load_non_intra = r.read_bit()? == 1;
        let non_intra_quantizer_matrix = if load_non_intra {
            let mut matrix = [0u8; 64];
            for slot in &mut matrix {
                *slot = r.read_bits(8)? as u8;
            }
            Some(matrix)
        } else {
            None
        };
        Ok(SequenceHeader {
            horizontal_size_value,
            vertical_size_value,
            aspect_ratio_information,
            frame_rate_code,
            bit_rate_value,
            vbv_buffer_size_value,
            constrained_parameters_flag,
            intra_quantizer_matrix,
            non_intra_quantizer_matrix,
        })
    }

    /// Display aspect ratio as a (w, h) ratio pair (Table 6-2: 1
    /// square, 2 = 4:3, 3 = 16:9, 4 = 2.21:1).
    #[must_use]
    pub fn aspect_ratio(&self) -> (u32, u32) {
        match self.aspect_ratio_information {
            1 => (1, 1),
            2 => (4, 3),
            3 => (16, 9),
            _ => (221, 100),
        }
    }

    /// Nominal frame rate (num, den) from the code table (before any
    /// sequence-extension refinement).
    #[must_use]
    pub fn frame_rate(&self) -> (u32, u32) {
        frame_rate_table(self.frame_rate_code).unwrap_or((0, 0))
    }

    /// True when `bit_rate_value == 0x3FFFF` (the variable-bitrate
    /// marker).
    #[must_use]
    pub fn variable_bitrate(&self) -> bool {
        self.bit_rate_value == 0x3_FFFF
    }

    /// VBV buffer size in 16·1024-byte units (the spec's scale).
    #[must_use]
    pub fn vbv_buffer_bytes(&self) -> u32 {
        (self.vbv_buffer_size_value + 1) * 16 * 1024
    }
}

/// sequence_extension (13818-2 §6.2.2.3) — the MPEG-2 marker inside
/// an MPEG-1-shaped stream: its presence is what makes a stream
/// MPEG-2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceExtension {
    /// `profile_and_level_indication` (8 bits).
    pub profile_and_level_indication: u8,
    /// `progressive_sequence` (0 = interlaced-only content).
    pub progressive_sequence: bool,
    /// `chroma_format` (2 bits; 00 forbidden).
    pub chroma_format: Option<ChromaFormat>,
    /// `horizontal_size_extension` (2 bits).
    pub horizontal_size_extension: u32,
    /// `vertical_size_extension` (2 bits).
    pub vertical_size_extension: u32,
    /// `bit_rate_extension` (12 bits).
    pub bit_rate_extension: u32,
    /// `vbv_buffer_size_extension` (8 bits).
    pub vbv_buffer_size_extension: u32,
    /// `low_delay` (1 bit; 1 = no B-picture reordering buffering).
    pub low_delay: bool,
    /// `frame_rate_extension_n` (2 bits).
    pub frame_rate_extension_n: u32,
    /// `frame_rate_extension_d` (5 bits).
    pub frame_rate_extension_d: u32,
}

impl SequenceExtension {
    /// Parse one sequence-extension unit payload (the bytes after
    /// `00 00 01 B5`; extension id nibble `0001`, §6.2.2.3).
    ///
    /// # Errors
    /// `Invalid` when the id nibble is not `0001`; `Truncated`/
    /// `Marker` on damage; `OutOfRange` on forbidden `chroma_format`.
    pub fn parse(payload: &[u8]) -> Result<SequenceExtension, Mpeg12Error> {
        let mut r = BitReader::new(payload);
        let id = r.read_bits(4)?;
        if id != 0b0001 {
            return Err(Mpeg12Error::Invalid {
                code: "mpeg12.ext.not_sequence",
            });
        }
        let profile_and_level_indication = r.read_bits(8)? as u8;
        let progressive_sequence = r.read_bit()? == 1;
        let chroma_code = r.read_bits(2)?;
        let chroma_format = match chroma_code {
            0b00 => {
                return Err(Mpeg12Error::OutOfRange {
                    field: "chroma_format",
                    value: 0,
                    code: "mpeg12.ext.chroma",
                });
            }
            0b01 => Some(ChromaFormat::Chroma420),
            0b10 => Some(ChromaFormat::Chroma422),
            _ => Some(ChromaFormat::Chroma444),
        };
        let horizontal_size_extension = r.read_bits(2)?;
        let vertical_size_extension = r.read_bits(2)?;
        let bit_rate_extension = r.read_bits(12)?;
        r.read_marker("mpeg12.ext.marker")?;
        let vbv_buffer_size_extension = r.read_bits(8)?;
        let low_delay = r.read_bit()? == 1;
        let frame_rate_extension_n = r.read_bits(2)?;
        let frame_rate_extension_d = r.read_bits(5)?;
        Ok(SequenceExtension {
            profile_and_level_indication,
            progressive_sequence,
            chroma_format,
            horizontal_size_extension,
            vertical_size_extension,
            bit_rate_extension,
            vbv_buffer_size_extension,
            low_delay,
            frame_rate_extension_n,
            frame_rate_extension_d,
        })
    }

    /// Refined frame rate (13818-2 §6.3.3):
    /// `frame_rate_value · (n+1) / (d+1)`, **reduced to lowest
    /// terms** — the differential's 0130 corpus lane caught the
    /// unreduced `24/12` vs ffprobe's `2/1` (worklog 5.H1: the
    /// extension factors share common divisors with the table's
    /// 1001-based denominators).
    #[must_use]
    pub fn frame_rate_with_ext(&self, code: u32) -> (u64, u64) {
        let Some((num, den)) = frame_rate_table(code) else {
            return (0, 0);
        };
        let num = u64::from(num) * u64::from(self.frame_rate_extension_n + 1);
        let den = u64::from(den) * u64::from(self.frame_rate_extension_d + 1);
        let mut a = num;
        let mut b = den;
        while b != 0 {
            let t = a % b;
            a = b;
            b = t;
        }
        match num.checked_div(a).zip(den.checked_div(a)) {
            Some((rn, rd)) if a > 0 => (rn, rd),
            _ => (num, den),
        }
    }
}

/// group_start (GOP) header (11172-2 §6.2.2.6 / 13818-2 §6.2.2.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GopHeader {
    /// `time_code` drop-frame flag.
    pub drop_frame_flag: bool,
    /// Hours (5 bits).
    pub time_code_hours: u32,
    /// Minutes (6 bits).
    pub time_code_minutes: u32,
    /// Seconds (6 bits).
    pub time_code_seconds: u32,
    /// Pictures within the second (6 bits).
    pub time_code_pictures: u32,
    /// `closed_gop` (1 bit).
    pub closed_gop: bool,
    /// `broken_link` (1 bit).
    pub broken_link: bool,
}

impl GopHeader {
    /// Parse one GOP unit payload (after `00 00 01 B8`).
    ///
    /// # Errors
    /// `Truncated`/`Marker` on damage.
    pub fn parse(payload: &[u8]) -> Result<GopHeader, Mpeg12Error> {
        let mut r = BitReader::new(payload);
        let drop_frame_flag = r.read_bit()? == 1;
        let time_code_hours = r.read_bits(5)?;
        let time_code_minutes = r.read_bits(6)?;
        r.read_marker("mpeg12.gop.marker")?;
        let time_code_seconds = r.read_bits(6)?;
        let time_code_pictures = r.read_bits(6)?;
        let closed_gop = r.read_bit()? == 1;
        let broken_link = r.read_bit()? == 1;
        Ok(GopHeader {
            drop_frame_flag,
            time_code_hours,
            time_code_minutes,
            time_code_seconds,
            time_code_pictures,
            closed_gop,
            broken_link,
        })
    }

    /// The SMPTE timecode string `HH:MM:SS:PP`.
    #[must_use]
    pub fn time_code(&self) -> String {
        format!(
            "{:02}:{:02}:{:02}:{:02}",
            self.time_code_hours,
            self.time_code_minutes,
            self.time_code_seconds,
            self.time_code_pictures
        )
    }
}

/// picture_header (11172-2 §6.2.3 / 13818-2 §6.2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureHeader {
    /// `temporal_reference` (10 bits; picture order within the GOP).
    pub temporal_reference: u32,
    /// `picture_coding_type` (3 bits; 0 and 5..=7 forbidden).
    pub picture_coding_type: Option<PictureCodingType>,
    /// `vbv_delay` (16 bits; 0xFFFF = variable).
    pub vbv_delay: u32,
    /// MPEG-1 forward predictor geometry (P/B pictures only).
    pub full_pel_forward_vector: bool,
    /// MPEG-1 `forward_f_code` (3 bits).
    pub forward_f_code: u32,
    /// MPEG-1 backward predictor geometry (B pictures only).
    pub full_pel_backward_vector: bool,
    /// MPEG-1 `backward_f_code` (3 bits).
    pub backward_f_code: u32,
}

impl PictureHeader {
    /// Motion-vector syntax presence for this picture (P/B carry
    /// vector syntax; I/D do not) — the probe fact of card 5.H1.
    #[must_use]
    pub fn has_motion_vectors(&self) -> bool {
        self.picture_coding_type
            .is_some_and(PictureCodingType::has_motion_vectors)
    }

    /// Parse one picture unit payload (after `00 00 01 00`).
    ///
    /// # Errors
    /// `Truncated` on damage; `OutOfRange` on a forbidden coding
    /// type.
    pub fn parse(payload: &[u8]) -> Result<PictureHeader, Mpeg12Error> {
        let mut r = BitReader::new(payload);
        let temporal_reference = r.read_bits(10)?;
        let code = r.read_bits(3)?;
        let picture_coding_type = match code {
            1 => Some(PictureCodingType::Intra),
            2 => Some(PictureCodingType::Predictive),
            3 => Some(PictureCodingType::Bidirectional),
            4 => Some(PictureCodingType::DcIntra),
            other => {
                return Err(Mpeg12Error::OutOfRange {
                    field: "picture_coding_type",
                    value: u64::from(other),
                    code: "mpeg12.pic.coding_type",
                });
            }
        };
        let vbv_delay = r.read_bits(16)?;
        let mut full_pel_forward_vector = false;
        let mut forward_f_code = 0;
        let mut full_pel_backward_vector = false;
        let mut backward_f_code = 0;
        if matches!(
            picture_coding_type,
            Some(PictureCodingType::Predictive | PictureCodingType::Bidirectional)
        ) {
            full_pel_forward_vector = r.read_bit()? == 1;
            forward_f_code = r.read_bits(3)?;
        }
        if picture_coding_type == Some(PictureCodingType::Bidirectional) {
            full_pel_backward_vector = r.read_bit()? == 1;
            backward_f_code = r.read_bits(3)?;
        }
        Ok(PictureHeader {
            temporal_reference,
            picture_coding_type,
            vbv_delay,
            full_pel_forward_vector,
            forward_f_code,
            full_pel_backward_vector,
            backward_f_code,
        })
    }
}

/// picture_coding_extension (13818-2 §6.2.3.1) — the MPEG-2 picture
/// geometry: field/frame structure, DCT precision, motion-vector
/// ranges (`f_code`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureCodingExtension {
    /// `f_code[0][0]` / `[0][1]` / `[1][0]` / `[1][1]` (4 bits each;
    /// the forward/backward motion-vector range exponents).
    pub f_code: [[u32; 2]; 2],
    /// `intra_dc_precision` (2 bits; 0 = 8-bit, 1 = 9, 2 = 10,
    /// 3 = 11).
    pub intra_dc_precision: u32,
    /// `picture_structure` (1 top field, 2 bottom field, 3 frame).
    pub picture_structure: u32,
    /// `top_field_first` (1 bit).
    pub top_field_first: bool,
    /// `frame_pred_frame_dct` (1 bit; 1 = no field predictions —
    /// progressive-style coding even in interlaced sequences).
    pub frame_pred_frame_dct: bool,
    /// `concealment_motion_vectors` (1 bit).
    pub concealment_motion_vectors: bool,
    /// `q_scale_type` (1 bit; the quantiser scale mapping).
    pub q_scale_type: bool,
    /// `intra_vlc_format` (1 bit; which DCT-coefficient table).
    pub intra_vlc_format: bool,
    /// `alternate_scan` (1 bit).
    pub alternate_scan: bool,
    /// `repeat_first_field` (1 bit; 3:2 pulldown signal).
    pub repeat_first_field: bool,
    /// `chroma_420_type` (1 bit).
    pub chroma_420_type: bool,
    /// `progressive_frame` (1 bit).
    pub progressive_frame: bool,
    /// `composite_display_flag` (1 bit).
    pub composite_display_flag: bool,
}

impl PictureCodingExtension {
    /// Parse one extension payload whose id nibble is `1000`.
    ///
    /// # Errors
    /// `Invalid` on a foreign id; `Truncated` on damage.
    pub fn parse(payload: &[u8]) -> Result<PictureCodingExtension, Mpeg12Error> {
        let mut r = BitReader::new(payload);
        let id = r.read_bits(4)?;
        if id != 0b1000 {
            return Err(Mpeg12Error::Invalid {
                code: "mpeg12.ext.not_picture_coding",
            });
        }
        let mut f_code = [[0u32; 2]; 2];
        for row in &mut f_code {
            for slot in row {
                *slot = r.read_bits(4)?;
            }
        }
        let intra_dc_precision = r.read_bits(2)?;
        let picture_structure = r.read_bits(2)?;
        let top_field_first = r.read_bit()? == 1;
        let frame_pred_frame_dct = r.read_bit()? == 1;
        let concealment_motion_vectors = r.read_bit()? == 1;
        let q_scale_type = r.read_bit()? == 1;
        let intra_vlc_format = r.read_bit()? == 1;
        let alternate_scan = r.read_bit()? == 1;
        let repeat_first_field = r.read_bit()? == 1;
        let chroma_420_type = r.read_bit()? == 1;
        let progressive_frame = r.read_bit()? == 1;
        let composite_display_flag = r.read_bit()? == 1;
        Ok(PictureCodingExtension {
            f_code,
            intra_dc_precision,
            picture_structure,
            top_field_first,
            frame_pred_frame_dct,
            concealment_motion_vectors,
            q_scale_type,
            intra_vlc_format,
            alternate_scan,
            repeat_first_field,
            chroma_420_type,
            progressive_frame,
            composite_display_flag,
        })
    }
}

/// slice header (11172-2 §6.2.4 / 13818-2 §6.2.4) plus the first
/// macroblock-addressing VLC — the decode-adjacent seam this card
/// stops at (macroblock *data* stays unparsed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceHeader {
    /// The slice's start-code row number (1..=175; vertical position
    /// in macroblock rows, top to bottom).
    pub slice_number: u8,
    /// `quantiser_scale_code` (5 bits; 1..=31, 0 forbidden).
    pub quantiser_scale_code: u32,
    /// `intra_slice_flag` (present when `extra_bit_slice` was 1).
    pub intra_slice_flag: bool,
    /// The first `macroblock_address_increment` when its VLC matched
    /// a table entry (Table B-1; the gap from the slice's first MB
    /// row position to the first coded MB — 1 for a normal start).
    /// `None` = the VLC did not match the modeled table (escape or
    /// damage), recorded honestly rather than guessed.
    pub first_macroblock_address_increment: Option<u32>,
}

/// The macroblock_address_increment VLC (13818-2 Table B-1, identical
/// in 11172-2): (code, length) → increment. 1..=33 modeled; the
/// escape and unused suffixes return `None`.
#[must_use]
pub fn decode_macroblock_address_increment(code: u32, length: u32) -> Option<u32> {
    match (code, length) {
        (0b1, 1) => Some(1),
        (0b011, 3) => Some(2),
        (0b010, 3) => Some(3),
        (0b0011, 4) => Some(4),
        (0b0010, 4) => Some(5),
        (0b00011, 5) => Some(6),
        (0b00010, 5) => Some(7),
        (0b0000111, 7) => Some(8),
        (0b0000110, 7) => Some(9),
        (0b00001011, 8) => Some(10),
        (0b00001010, 8) => Some(11),
        (0b00001001, 8) => Some(12),
        (0b00001000, 8) => Some(13),
        (0b00000111, 8) => Some(14),
        (0b00000110, 8) => Some(15),
        (0b0000010111, 10) => Some(16),
        (0b0000010110, 10) => Some(17),
        (0b0000010101, 10) => Some(18),
        (0b0000010100, 10) => Some(19),
        (0b0000010011, 10) => Some(20),
        (0b0000010010, 10) => Some(21),
        (0b00000100011, 11) => Some(22),
        (0b00000100010, 11) => Some(23),
        (0b00000100001, 11) => Some(24),
        (0b00000100000, 11) => Some(25),
        (0b00000011111, 11) => Some(26),
        (0b00000011110, 11) => Some(27),
        (0b00000011101, 11) => Some(28),
        (0b00000011100, 11) => Some(29),
        (0b00000011011, 11) => Some(30),
        (0b00000011010, 11) => Some(31),
        (0b00000011001, 11) => Some(32),
        (0b00000011000, 11) => Some(33),
        _ => None,
    }
}

impl SliceHeader {
    /// Parse one slice unit payload (after `00 00 01 nn`,
    /// `nn` = slice_number).
    ///
    /// # Errors
    /// `Invalid` for slice number 0 (>0xAF is already not a slice
    /// start code); `Truncated` on damage.
    pub fn parse(slice_number: u8, payload: &[u8]) -> Result<SliceHeader, Mpeg12Error> {
        if slice_number == 0 {
            return Err(Mpeg12Error::Invalid {
                code: "mpeg12.slice.zero",
            });
        }
        let mut r = BitReader::new(payload);
        let quantiser_scale_code = r.read_bits(5)?;
        let mut intra_slice_flag = false;
        if r.read_bit()? == 1 {
            intra_slice_flag = r.read_bit()? == 1;
            if intra_slice_flag {
                // intra_slice + reserved bits (§6.2.4): consumed,
                // not modeled further.
                r.read_bit()?;
                r.read_bits(7)?;
            }
        }
        // First macroblock_address_increment VLC: greedy-match one of
        // the Table B-1 codes, longest sensible probe first is not
        // needed — the table is prefix-free, so a bit-by-bit walk
        // with a set membership check per length works: accumulate
        // bits until a known code appears (or 11 bits give up).
        let mut code = 0u32;
        let mut increment = None;
        for length in 1..=11u32 {
            let bit = r.read_bit().ok();
            let Some(bit) = bit else { break };
            code = (code << 1) | bit;
            if let Some(value) = decode_macroblock_address_increment(code, length) {
                increment = Some(value);
                break;
            }
        }
        Ok(SliceHeader {
            slice_number,
            quantiser_scale_code,
            intra_slice_flag,
            first_macroblock_address_increment: increment,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// MSB-first bit writer (test-side mirror of `BitReader`).
    struct Writer {
        bytes: Vec<u8>,
        bit: u8,
        used: u32,
    }

    impl Writer {
        fn new() -> Writer {
            Writer {
                bytes: Vec::new(),
                bit: 0,
                used: 0,
            }
        }
        fn put(&mut self, value: u32, n: u32) {
            for i in (0..n).rev() {
                self.bit = (self.bit << 1) | ((value >> i) & 1) as u8;
                self.used += 1;
                if self.used == 8 {
                    self.bytes.push(self.bit);
                    self.bit = 0;
                    self.used = 0;
                }
            }
        }
        fn finish(mut self) -> Vec<u8> {
            if self.used > 0 {
                self.bit <<= 8 - self.used;
                self.bytes.push(self.bit);
            }
            self.bytes
        }
    }

    #[test]
    fn sequence_header_roundtrip_ffmpeg_shaped() {
        // 640x480, 4:3, 25fps, 400_000*50 bitrate units, vbv 20.
        let mut w = Writer::new();
        w.put(640, 12);
        w.put(480, 12);
        w.put(2, 4);
        w.put(3, 4);
        w.put(20_000_000, 18);
        w.put(1, 1); // marker
        w.put(19, 10);
        w.put(0, 1); // constrained
        w.put(0, 1); // no intra matrix
        w.put(0, 1); // no non-intra matrix
        let seq = SequenceHeader::parse(&w.finish()).must();
        assert_eq!(seq.horizontal_size_value, 640);
        assert_eq!(seq.vertical_size_value, 480);
        assert_eq!(seq.aspect_ratio(), (4, 3));
        assert_eq!(seq.frame_rate(), (25, 1));
        assert!(!seq.variable_bitrate());
        assert_eq!(seq.vbv_buffer_bytes(), 20 * 16 * 1024);
        assert!(seq.intra_quantizer_matrix.is_none());
    }

    #[test]
    fn sequence_header_with_quant_matrices() {
        let mut w = Writer::new();
        w.put(320, 12);
        w.put(240, 12);
        w.put(1, 4);
        w.put(4, 4);
        w.put(0x3_FFFF, 18);
        w.put(1, 1);
        w.put(0, 10);
        w.put(1, 1); // constrained (MPEG-1)
        w.put(1, 1); // load intra
        for i in 0..64u32 {
            w.put(8 + i, 8);
        }
        w.put(1, 1); // load non-intra
        for _ in 0..64u32 {
            w.put(16, 8);
        }
        let seq = SequenceHeader::parse(&w.finish()).must();
        assert!(seq.variable_bitrate());
        assert!(seq.constrained_parameters_flag);
        let intra = seq.intra_quantizer_matrix.must();
        assert_eq!(intra[0], 8);
        assert_eq!(intra[63], 8 + 63);
        assert_eq!(seq.non_intra_quantizer_matrix, Some([16u8; 64]));
    }

    #[test]
    fn sequence_header_rejects_forbidden_codes() {
        let mut w = Writer::new();
        w.put(320, 12);
        w.put(240, 12);
        w.put(7, 4); // forbidden aspect
        w.put(3, 4);
        w.put(1000, 18);
        w.put(1, 1);
        w.put(1, 10);
        w.put(0, 3);
        assert_eq!(
            SequenceHeader::parse(&w.finish()),
            Err(Mpeg12Error::OutOfRange {
                field: "aspect_ratio_information",
                value: 7,
                code: "mpeg12.seq.aspect",
            })
        );
        let mut w = Writer::new();
        w.put(320, 12);
        w.put(240, 12);
        w.put(1, 4);
        w.put(9, 4); // forbidden frame rate
        w.put(1000, 18);
        w.put(1, 1);
        w.put(1, 10);
        w.put(0, 3);
        assert!(matches!(
            SequenceHeader::parse(&w.finish()),
            Err(Mpeg12Error::OutOfRange { field, .. }) if field == "frame_rate_code"
        ));
    }

    #[test]
    fn sequence_header_marker_violation_is_typed() {
        let mut w = Writer::new();
        w.put(320, 12);
        w.put(240, 12);
        w.put(1, 4);
        w.put(3, 4);
        w.put(1000, 18);
        w.put(0, 1); // marker violated
        assert_eq!(
            SequenceHeader::parse(&w.finish()),
            Err(Mpeg12Error::Marker {
                code: "mpeg12.seq.marker"
            })
        );
    }

    #[test]
    fn sequence_extension_roundtrip_and_chroma_gate() {
        // Main@Main = 0x48, interlaced, 4:2:0, size exts 0, rate exts 0.
        let mut w = Writer::new();
        w.put(0b0001, 4); // extension id
        w.put(0x48, 8);
        w.put(0, 1); // interlaced
        w.put(0b01, 2); // 4:2:0
        w.put(0, 2);
        w.put(0, 2);
        w.put(0, 12);
        w.put(1, 1); // marker
        w.put(0, 8);
        w.put(0, 1); // low_delay
        w.put(0, 2);
        w.put(0, 5);
        let ext = SequenceExtension::parse(&w.finish()).must();
        assert_eq!(profile_name(ext.profile_and_level_indication), Some("Main"));
        assert_eq!(level_value(0x48), 8);
        assert!(!ext.progressive_sequence);
        assert_eq!(ext.chroma_format, Some(ChromaFormat::Chroma420));
        assert_eq!(ext.frame_rate_with_ext(4), (30_000, 1001));

        // Forbidden chroma 00.
        let mut w = Writer::new();
        w.put(0b0001, 4);
        w.put(0x48, 8);
        w.put(1, 1);
        w.put(0b00, 2);
        assert_eq!(
            SequenceExtension::parse(&w.finish()),
            Err(Mpeg12Error::OutOfRange {
                field: "chroma_format",
                value: 0,
                code: "mpeg12.ext.chroma",
            })
        );
        // Foreign id.
        let mut w = Writer::new();
        w.put(0b0010, 4);
        assert_eq!(
            SequenceExtension::parse(&w.finish()),
            Err(Mpeg12Error::Invalid {
                code: "mpeg12.ext.not_sequence"
            })
        );
    }

    #[test]
    fn frame_rate_extension_refines() {
        let mut w = Writer::new();
        w.put(0b0001, 4);
        w.put(0x48, 8);
        w.put(1, 1);
        w.put(0b01, 2);
        w.put(0, 2);
        w.put(0, 2);
        w.put(0, 12);
        w.put(1, 1);
        w.put(0, 8);
        w.put(0, 1);
        w.put(1, 2); // n = 1
        w.put(0, 5); // d = 0
        let ext = SequenceExtension::parse(&w.finish()).must();
        // 25 * (1+1) / (0+1) = 50 fps.
        assert_eq!(ext.frame_rate_with_ext(3), (50, 1));
    }

    #[test]
    fn gop_header_roundtrip() {
        let mut w = Writer::new();
        w.put(0, 1); // drop frame
        w.put(1, 5); // hours
        w.put(2, 6); // minutes
        w.put(1, 1); // marker
        w.put(3, 6); // seconds
        w.put(0, 6); // pictures
        w.put(1, 1); // closed
        w.put(0, 1); // broken
        let gop = GopHeader::parse(&w.finish()).must();
        assert_eq!(gop.time_code(), "01:02:03:00");
        assert!(gop.closed_gop);
        assert!(!gop.broken_link);
    }

    #[test]
    fn picture_header_i_p_b_shapes() {
        let build = |coding: u32, temporal: u32| {
            let mut w = Writer::new();
            w.put(temporal, 10);
            w.put(coding, 3);
            w.put(0x1234, 16);
            if coding == 2 || coding == 3 {
                w.put(1, 1); // full_pel_forward
                w.put(3, 3); // forward_f_code
            }
            if coding == 3 {
                w.put(0, 1); // full_pel_backward
                w.put(2, 3); // backward_f_code
            }
            w.finish()
        };
        let i = PictureHeader::parse(&build(1, 0)).must();
        assert_eq!(i.picture_coding_type, Some(PictureCodingType::Intra));
        assert!(
            !i.picture_coding_type
                .is_some_and(PictureCodingType::has_motion_vectors)
        );
        assert_eq!(i.vbv_delay, 0x1234);
        let p = PictureHeader::parse(&build(2, 1)).must();
        assert_eq!(p.picture_coding_type, Some(PictureCodingType::Predictive));
        assert!(p.full_pel_forward_vector);
        assert_eq!(p.forward_f_code, 3);
        let b = PictureHeader::parse(&build(3, 2)).must();
        assert_eq!(
            b.picture_coding_type,
            Some(PictureCodingType::Bidirectional)
        );
        assert_eq!(b.backward_f_code, 2);
        // Forbidden coding types.
        for bad in [0u32, 5, 7] {
            let mut w = Writer::new();
            w.put(0, 10);
            w.put(bad, 3);
            w.put(0, 16);
            assert!(matches!(
                PictureHeader::parse(&w.finish()),
                Err(Mpeg12Error::OutOfRange { field, .. }) if field == "picture_coding_type"
            ));
        }
    }

    #[test]
    fn picture_coding_extension_roundtrip() {
        let mut w = Writer::new();
        w.put(0b1000, 4);
        for v in [1u32, 2, 3, 4] {
            w.put(v, 4);
        }
        w.put(0, 2); // 8-bit dc
        w.put(3, 2); // frame picture
        w.put(1, 1); // top first
        w.put(0, 1); // field predictions allowed
        w.put(0, 1); // no concealment MVs
        w.put(0, 1); // linear q scale
        w.put(0, 1); // table 0
        w.put(0, 1); // zigzag
        w.put(0, 1); // no repeat
        w.put(0, 1);
        w.put(1, 1); // progressive frame
        w.put(0, 1);
        let pce = PictureCodingExtension::parse(&w.finish()).must();
        assert_eq!(pce.f_code, [[1, 2], [3, 4]]);
        assert_eq!(pce.picture_structure, 3);
        assert!(pce.progressive_frame);
        assert!(!pce.concealment_motion_vectors);
        // Foreign id.
        let mut w = Writer::new();
        w.put(0b0001, 4);
        assert_eq!(
            PictureCodingExtension::parse(&w.finish()),
            Err(Mpeg12Error::Invalid {
                code: "mpeg12.ext.not_picture_coding"
            })
        );
    }

    #[test]
    fn slice_header_with_first_mb_increment() {
        // quantiser 10, extra_bit_slice 0, then VLC '1' (=increment 1).
        let mut w = Writer::new();
        w.put(10, 5);
        w.put(0, 1);
        w.put(1, 1); // VLC for increment 1
        let sh = SliceHeader::parse(1, &w.finish()).must();
        assert_eq!(sh.slice_number, 1);
        assert_eq!(sh.quantiser_scale_code, 10);
        assert!(!sh.intra_slice_flag);
        assert_eq!(sh.first_macroblock_address_increment, Some(1));
        // VLC '011' = increment 2.
        let mut w = Writer::new();
        w.put(10, 5);
        w.put(0, 1);
        w.put(0b011, 3);
        let sh = SliceHeader::parse(5, &w.finish()).must();
        assert_eq!(sh.first_macroblock_address_increment, Some(2));
        // Escape-shaped bits (00000001 000...) — 11 bits, no match:
        // honestly None.
        let mut w = Writer::new();
        w.put(10, 5);
        w.put(0, 1);
        w.put(0, 8);
        w.put(1, 1);
        w.put(0, 2);
        let sh = SliceHeader::parse(1, &w.finish()).must();
        assert_eq!(sh.first_macroblock_address_increment, None);
    }

    #[test]
    fn mb_increment_table_covers_one_through_thirty_three() {
        let mut codes = Vec::new();
        for (code, len) in [
            (0b1u32, 1),
            (0b011, 3),
            (0b010, 3),
            (0b0011, 4),
            (0b0010, 4),
            (0b00011, 5),
            (0b00010, 5),
            (0b0000111, 7),
            (0b0000110, 7),
            (0b00001011, 8),
            (0b00001010, 8),
            (0b00001001, 8),
            (0b00001000, 8),
            (0b00000111, 8),
            (0b00000110, 8),
            (0b0000010111, 10),
            (0b0000010110, 10),
            (0b0000010101, 10),
            (0b0000010100, 10),
            (0b0000010011, 10),
            (0b0000010010, 10),
            (0b00000100011, 11),
            (0b00000100010, 11),
            (0b00000100001, 11),
            (0b00000100000, 11),
            (0b00000011111, 11),
            (0b00000011110, 11),
            (0b00000011101, 11),
            (0b00000011100, 11),
            (0b00000011011, 11),
            (0b00000011010, 11),
            (0b00000011001, 11),
            (0b00000011000, 11),
        ] {
            let value = decode_macroblock_address_increment(code, len).must();
            codes.push((code, len, value));
        }
        // 33 distinct codes, values exactly 1..=33 (prefix-free by
        // construction check: distinct codes of equal length, and no
        // code equals a shorter one's prefix — spot-checked by the
        // value bijection).
        for (idx, (_, _, value)) in codes.iter().enumerate() {
            assert_eq!(*value, idx as u32 + 1);
        }
        // Unknown suffixes are honestly absent.
        assert_eq!(decode_macroblock_address_increment(0b00000001000, 11), None);
        assert_eq!(decode_macroblock_address_increment(0, 11), None);
    }

    #[test]
    fn frame_rate_table_matches_spec() {
        assert_eq!(frame_rate_table(1), Some((24_000, 1001)));
        assert_eq!(frame_rate_table(2), Some((24, 1)));
        assert_eq!(frame_rate_table(3), Some((25, 1)));
        assert_eq!(frame_rate_table(4), Some((30_000, 1001)));
        assert_eq!(frame_rate_table(5), Some((30, 1)));
        assert_eq!(frame_rate_table(6), Some((50, 1)));
        assert_eq!(frame_rate_table(7), Some((60_000, 1001)));
        assert_eq!(frame_rate_table(8), Some((60, 1)));
        for bad in [0u32, 9, 15] {
            assert_eq!(frame_rate_table(bad), None);
        }
    }

    #[test]
    fn coding_type_letters_and_motion_vectors() {
        assert_eq!(PictureCodingType::Intra.letter(), 'I');
        assert!(!PictureCodingType::Intra.has_motion_vectors());
        assert!(PictureCodingType::Predictive.has_motion_vectors());
        assert!(PictureCodingType::Bidirectional.has_motion_vectors());
        assert!(!PictureCodingType::DcIntra.has_motion_vectors());
    }
}
