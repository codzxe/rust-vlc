//! The stream-level fact scan (card 5.H1's deliverable): walk an
//! MPEG-1/2 elementary-stream buffer start code by start code and
//! collect the sequence-layer facts probe/track-info report —
//! resolution, frame rate, profile/level, chroma, progressive/
//! interlaced, GOP timecode, picture-type census, slice/motion-
//! vector presence — **without decoding a single macroblock**.
//!
//! # Damage model
//!
//! [`Mpeg12Facts::scan`] is total: any byte string yields a fact
//! set. A damaged header (typed parse failure) does not abort the
//! walk — it is counted in `bad_headers` and the scan continues at
//! the next start code (broadcast-capture tolerance, the same
//! posture the TS demuxer landed with). No start codes at all (a
//! non-ES buffer, a one-packet slice of mid-stream PES payload)
//! yields an empty fact set with `start_codes == 0` — an honest
//! "not enough signal yet", never a guess.

use super::error::{accepts, decode_refusal};
use super::headers::{
    ChromaFormat, GopHeader, PictureCodingExtension, PictureHeader, SequenceExtension,
    SequenceHeader, SliceHeader, profile_name,
};
use super::startcode::{StartCode, StartCodeScan};

/// Sequence-layer facts for one MPEG-1/2 video stream (the first
/// occurrence of each header wins — the same "first parameter set"
/// identity ffprobe's stream facts are built from; later
/// `sequence_header`s, e.g. mid-stream resolution changes, are
/// counted and flagged via `sequence_changes`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mpeg12Facts {
    /// First `sequence_header`.
    pub sequence: Option<SequenceHeader>,
    /// First sequence extension (its presence makes the stream
    /// MPEG-2).
    pub sequence_ext: Option<SequenceExtension>,
    /// First GOP header.
    pub gop: Option<GopHeader>,
    /// First picture header.
    pub picture: Option<PictureHeader>,
    /// First picture coding extension.
    pub picture_ext: Option<PictureCodingExtension>,
    /// First slice header (quantiser scale + macroblock addressing).
    pub slice: Option<SliceHeader>,
    /// `sequence_header` count (a second, differing one is the
    /// mid-stream parameter change of the corpus's
    /// param-midchange lanes).
    pub sequence_count: u32,
    /// True when a later `sequence_header` differed from the first.
    pub sequence_changes: bool,
    /// `picture_start_code` count.
    pub picture_count: u32,
    /// Slice start-code count.
    pub slice_count: u32,
    /// Picture-type census as a bitmask over `PictureCodingType`
    /// discriminants (bit 0 = I seen, 1 = P, 2 = B, 3 = D).
    pub picture_type_mask: u8,
    /// Slice headers whose first macroblock VLC matched a modeled
    /// code (macroblock-addressing evidence).
    pub slices_with_mb_address: u32,
    /// Headers that failed a typed parse (damage census; the walk
    /// continued).
    pub bad_headers: u32,
    /// Total start codes walked.
    pub start_codes: u32,
    /// `sequence_end_code` seen.
    pub sequence_end: bool,
}

impl Mpeg12Facts {
    /// Scan a whole elementary-stream buffer (a demuxed track's
    /// concatenated packets or a raw `.m2v`/`.m1v` file).
    ///
    /// Bounded and total: one pass, no allocation beyond the fact
    /// struct, no panic on any input.
    #[must_use]
    pub fn scan(data: &[u8]) -> Mpeg12Facts {
        let mut facts = Mpeg12Facts::default();
        for unit in StartCodeScan::new(data) {
            facts.start_codes += 1;
            match unit.code {
                StartCode::SequenceHeader => {
                    facts.sequence_count += 1;
                    match SequenceHeader::parse(unit.payload) {
                        Ok(seq) => {
                            if let Some(first) = facts.sequence.as_ref() {
                                if first != &seq {
                                    facts.sequence_changes = true;
                                }
                            } else {
                                facts.sequence = Some(seq);
                            }
                        }
                        Err(_) => facts.bad_headers += 1,
                    }
                }
                StartCode::ExtensionStart => {
                    // Dispatch on the 4-bit extension id (MSB-first).
                    let id = unit.payload.first().map_or(0, |b| b >> 4);
                    match id {
                        0b0001 if facts.sequence_ext.is_none() => {
                            match SequenceExtension::parse(unit.payload) {
                                Ok(ext) => facts.sequence_ext = Some(ext),
                                Err(_) => facts.bad_headers += 1,
                            }
                        }
                        0b1000 if facts.picture_ext.is_none() => {
                            match PictureCodingExtension::parse(unit.payload) {
                                Ok(ext) => facts.picture_ext = Some(ext),
                                Err(_) => facts.bad_headers += 1,
                            }
                        }
                        _ => {}
                    }
                }
                StartCode::GroupStart => {
                    if facts.gop.is_none() {
                        match GopHeader::parse(unit.payload) {
                            Ok(gop) => facts.gop = Some(gop),
                            Err(_) => facts.bad_headers += 1,
                        }
                    }
                }
                StartCode::PictureStart => {
                    facts.picture_count += 1;
                    match PictureHeader::parse(unit.payload) {
                        Ok(pic) => {
                            if let Some(kind) = pic.picture_coding_type {
                                facts.picture_type_mask |= 1 << (kind_as_index(kind));
                            }
                            if facts.picture.is_none() {
                                facts.picture = Some(pic);
                            }
                        }
                        Err(_) => facts.bad_headers += 1,
                    }
                }
                StartCode::Slice(n) => {
                    facts.slice_count += 1;
                    if facts.slice.is_none() || facts.slice_count <= 64 {
                        match SliceHeader::parse(n, unit.payload) {
                            Ok(sh) => {
                                if sh.first_macroblock_address_increment.is_some() {
                                    facts.slices_with_mb_address += 1;
                                }
                                if facts.slice.is_none() {
                                    facts.slice = Some(sh);
                                }
                            }
                            Err(_) => facts.bad_headers += 1,
                        }
                    }
                }
                StartCode::SequenceEnd => facts.sequence_end = true,
                StartCode::UserData | StartCode::SequenceError | StartCode::Other(_) => {}
            }
        }
        facts
    }

    /// Coded pixel dimensions (12-bit values extended by the
    /// MPEG-2 2-bit extensions when present).
    #[must_use]
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        let seq = self.sequence.as_ref()?;
        let w = seq.horizontal_size_value
            + self
                .sequence_ext
                .as_ref()
                .map_or(0, |e| e.horizontal_size_extension << 12);
        let h = seq.vertical_size_value
            + self
                .sequence_ext
                .as_ref()
                .map_or(0, |e| e.vertical_size_extension << 12);
        Some((w, h))
    }

    /// Frame rate as (num, den), MPEG-2 extension-refined.
    #[must_use]
    pub fn frame_rate(&self) -> Option<(u64, u64)> {
        let seq = self.sequence.as_ref()?;
        Some(match self.sequence_ext.as_ref() {
            Some(ext) => ext.frame_rate_with_ext(seq.frame_rate_code),
            None => {
                let (num, den) = seq.frame_rate();
                (u64::from(num), u64::from(den))
            }
        })
    }

    /// The ffprobe `r_frame_rate`-style string (`"30000/1001"`).
    #[must_use]
    pub fn frame_rate_string(&self) -> Option<String> {
        self.frame_rate().map(|(num, den)| format!("{num}/{den}"))
    }

    /// True when a sequence extension marks the stream MPEG-2
    /// (MPEG-1 otherwise).
    #[must_use]
    pub fn is_mpeg2(&self) -> bool {
        self.sequence_ext.is_some()
    }

    /// MPEG-2 profile name (None for MPEG-1 streams / escape bit).
    #[must_use]
    pub fn profile(&self) -> Option<&'static str> {
        self.sequence_ext
            .as_ref()
            .and_then(|e| profile_name(e.profile_and_level_indication))
    }

    /// MPEG-2 level value (the ffprobe `level` integer).
    #[must_use]
    pub fn level(&self) -> Option<u8> {
        self.sequence_ext
            .as_ref()
            .map(|e| super::headers::level_value(e.profile_and_level_indication))
    }

    /// Chroma format (MPEG-1 is always 4:2:0 by definition).
    #[must_use]
    pub fn chroma(&self) -> ChromaFormat {
        self.sequence_ext
            .as_ref()
            .and_then(|e| e.chroma_format)
            .unwrap_or(ChromaFormat::Chroma420)
    }

    /// Interlaced-content signal: a sequence extension with
    /// `progressive_sequence == 0` (MPEG-1 has no interlace).
    #[must_use]
    pub fn interlaced(&self) -> bool {
        self.sequence_ext
            .as_ref()
            .is_some_and(|e| !e.progressive_sequence)
    }

    /// Motion-vector syntax presence: P or B pictures observed.
    #[must_use]
    pub fn has_motion_vectors(&self) -> bool {
        self.picture_type_mask & 0b0110 != 0
    }

    /// The codec descriptor this scan supports facts for
    /// (`mpeg2video`/`mpeg1video`; the demuxers' vocabulary).
    #[must_use]
    pub fn descriptor(&self) -> &'static str {
        if self.is_mpeg2() {
            "mpeg2video"
        } else {
            "mpeg1video"
        }
    }

    /// The typed decode refusal for this stream's descriptor (the
    /// codec-api gate; see [`decode_refusal`]).
    #[must_use]
    pub fn decode_refusal(&self) -> media_types::error::EngineError {
        decode_refusal(self.descriptor())
    }

    /// Enough structure for a demuxer's track to classify as
    /// MPEG-1/2 video (a sequence header, at minimum).
    #[must_use]
    pub fn is_video_stream(&self) -> bool {
        self.sequence.is_some()
    }
}

/// The `picture_type_mask` bit for a coding type.
const fn kind_as_index(kind: super::headers::PictureCodingType) -> u8 {
    use super::headers::PictureCodingType;
    match kind {
        PictureCodingType::Intra => 0,
        PictureCodingType::Predictive => 1,
        PictureCodingType::Bidirectional => 2,
        PictureCodingType::DcIntra => 3,
    }
}

/// The codec-api descriptor gate for demuxer wiring: true when the
/// descriptor is one of ours (re-exported from [`super::error`] as
/// the single spelling the registry consumes).
#[must_use]
pub fn descriptor_gate(descriptor: &str) -> bool {
    accepts(descriptor)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal-but-honest MPEG-2 elementary stream: sequence
    /// header + sequence extension + GOP + I-picture + one slice,
    /// then sequence end.
    fn tiny_mpeg2_stream() -> Vec<u8> {
        let mut w = BitWriter::default();
        let mut out = Vec::new();
        // sequence_header: 352x288, 4:3, 25fps, vbv size 20.
        out.extend_from_slice(&[0, 0, 1, 0xB3]);
        w.put(352, 12);
        w.put(288, 12);
        w.put(2, 4);
        w.put(3, 4);
        w.put(0x3_FFFF, 18); // VBR marker
        w.put(1, 1);
        w.put(19, 10);
        w.put(0, 1);
        w.put(0, 1);
        w.put(0, 1);
        out.extend(w.take());
        // sequence extension: Main@Main, interlaced, 4:2:0.
        out.extend_from_slice(&[0, 0, 1, 0xB5]);
        w.put(0b0001, 4);
        w.put(0x48, 8);
        w.put(0, 1);
        w.put(0b01, 2);
        w.put(0, 2);
        w.put(0, 2);
        w.put(0, 12);
        w.put(1, 1);
        w.put(0, 8);
        w.put(0, 1);
        w.put(0, 2);
        w.put(0, 5);
        out.extend(w.take());
        // GOP: 00:00:00:00, closed.
        out.extend_from_slice(&[0, 0, 1, 0xB8]);
        w.put(0, 1);
        w.put(0, 5);
        w.put(0, 6);
        w.put(1, 1);
        w.put(0, 6);
        w.put(0, 6);
        w.put(1, 1);
        w.put(0, 1);
        out.extend(w.take());
        // I picture: temporal 0, type 1, vbv_delay 0xFFFF.
        out.extend_from_slice(&[0, 0, 1, 0x00]);
        w.put(0, 10);
        w.put(1, 3);
        w.put(0xFFFF, 16);
        out.extend(w.take());
        // slice 1: quantiser 8, extra 0, MB increment VLC '1'.
        out.extend_from_slice(&[0, 0, 1, 0x01]);
        w.put(8, 5);
        w.put(0, 1);
        w.put(1, 1);
        out.extend(w.take());
        out.extend_from_slice(&[0, 0, 1, 0xB7]);
        out
    }

    /// MSB-first bit writer with byte flushing.
    #[derive(Default)]
    struct BitWriter {
        bytes: Vec<u8>,
        bit: u8,
        used: u32,
    }

    impl BitWriter {
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
        fn take(&mut self) -> Vec<u8> {
            let mut out = core::mem::take(&mut self.bytes);
            if self.used > 0 {
                self.bit <<= 8 - self.used;
                out.push(self.bit);
                self.bit = 0;
                self.used = 0;
            }
            out
        }
    }

    #[test]
    fn tiny_stream_facts() {
        let stream = tiny_mpeg2_stream();
        let facts = Mpeg12Facts::scan(&stream);
        assert!(facts.is_video_stream());
        assert!(facts.is_mpeg2());
        assert_eq!(facts.dimensions(), Some((352, 288)));
        assert_eq!(facts.frame_rate(), Some((25, 1)));
        assert_eq!(facts.frame_rate_string().as_deref(), Some("25/1"));
        assert_eq!(facts.profile(), Some("Main"));
        assert_eq!(facts.level(), Some(8));
        assert_eq!(facts.chroma(), ChromaFormat::Chroma420);
        assert!(facts.interlaced());
        assert_eq!(facts.picture_count, 1);
        assert_eq!(facts.slice_count, 1);
        assert_eq!(facts.picture_type_mask, 0b0001); // I only
        assert!(!facts.has_motion_vectors());
        assert_eq!(facts.slices_with_mb_address, 1);
        assert_eq!(facts.bad_headers, 0);
        assert!(facts.sequence_end);
        assert_eq!(
            facts.gop.as_ref().map(GopHeader::time_code).as_deref(),
            Some("00:00:00:00")
        );
        assert_eq!(facts.descriptor(), "mpeg2video");
        assert_eq!(
            facts.slice.as_ref().map(|s| s.quantiser_scale_code),
            Some(8)
        );
        assert!(matches!(
            facts.decode_refusal().kind,
            media_types::error::ErrorKind::Unsupported { .. }
        ));
    }

    #[test]
    fn damage_is_counted_not_fatal() {
        let mut stream = tiny_mpeg2_stream();
        // Corrupt the sequence header's payload (after B3): flip
        // bits so the parse fails; the walk must continue and still
        // see the picture + slice.
        stream[6] ^= 0xFF;
        stream[7] ^= 0xFF;
        let facts = Mpeg12Facts::scan(&stream);
        assert!(facts.sequence.is_none());
        assert!(facts.bad_headers >= 1);
        assert_eq!(facts.picture_count, 1);
        assert_eq!(facts.slice_count, 1);
        assert!(facts.start_codes >= 5);
    }

    #[test]
    fn empty_and_garbage_inputs_are_total() {
        let empty = Mpeg12Facts::scan(&[]);
        assert_eq!(empty.start_codes, 0);
        assert!(!empty.is_video_stream());
        let garbage = Mpeg12Facts::scan(&[0x42; 4096]);
        assert_eq!(garbage.start_codes, 0);
        let zeros = Mpeg12Facts::scan(&[0; 4096]);
        assert_eq!(zeros.start_codes, 0);
        // Trailing prefix without a code byte.
        let stub = Mpeg12Facts::scan(&[0, 0, 1]);
        assert_eq!(stub.start_codes, 0);
    }

    #[test]
    fn mpeg1_stream_classifies_as_mpeg1() {
        // Sequence header only (no extension): MPEG-1.
        let mut w = BitWriter::default();
        let mut stream = vec![0, 0, 1, 0xB3];
        w.put(320, 12);
        w.put(240, 12);
        w.put(1, 4);
        w.put(4, 4);
        w.put(1000, 18);
        w.put(1, 1);
        w.put(1, 10);
        w.put(1, 1); // constrained (MPEG-1)
        w.put(0, 1);
        w.put(0, 1);
        stream.extend(w.take());
        stream.extend_from_slice(&[0, 0, 1, 0xB7]);
        let facts = Mpeg12Facts::scan(&stream);
        assert!(!facts.is_mpeg2());
        assert_eq!(facts.descriptor(), "mpeg1video");
        assert_eq!(facts.dimensions(), Some((320, 240)));
        assert_eq!(facts.frame_rate(), Some((30_000, 1001)));
        assert!(!facts.interlaced());
        assert_eq!(facts.profile(), None);
    }

    #[test]
    fn sequence_change_is_flagged() {
        let mut stream = tiny_mpeg2_stream();
        // Second sequence header with a different height: splice a
        // mutated copy before the end code.
        let mut w = BitWriter::default();
        let mut second = vec![0, 0, 1, 0xB3];
        w.put(352, 12);
        w.put(576, 12); // different
        w.put(2, 4);
        w.put(3, 4);
        w.put(0x3_FFFF, 18);
        w.put(1, 1);
        w.put(19, 10);
        w.put(0, 1);
        w.put(0, 1);
        w.put(0, 1);
        second.extend(w.take());
        let end = stream.split_off(stream.len() - 4);
        stream.extend(second);
        stream.extend(end);
        let facts = Mpeg12Facts::scan(&stream);
        assert_eq!(facts.sequence_count, 2);
        assert!(facts.sequence_changes);
        // First header wins the facts (ffprobe's identity).
        assert_eq!(facts.dimensions(), Some((352, 288)));
    }

    #[test]
    fn fuzz_lite_scan_is_total() {
        // +FB fuzz-lite: random buffers and mutations of the real
        // tiny stream — scan never panics, never hangs, and its
        // invariants (counters monotone, mask bounded) hold.
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let base = tiny_mpeg2_stream();
        for round in 0..8000u32 {
            let data: Vec<u8> = if round % 2 == 0 {
                let len = (next() % 4096) as usize;
                (0..len).map(|_| (next() % 256) as u8).collect()
            } else {
                let mut mutation = base.clone();
                for _ in 0..(next() % 6) + 1 {
                    let at = (next() as usize) % mutation.len();
                    mutation[at] = (next() % 256) as u8;
                }
                if round % 7 == 3 {
                    mutation.truncate((next() as usize) % (mutation.len() + 1));
                }
                mutation
            };
            let facts = Mpeg12Facts::scan(&data);
            assert!(facts.picture_count <= facts.start_codes);
            assert!(facts.slice_count <= facts.start_codes);
            assert!(facts.slices_with_mb_address <= facts.slice_count);
            assert!(facts.picture_type_mask <= 0b1111);
            if facts.sequence_count == 0 {
                assert!(facts.sequence.is_none());
                assert!(!facts.sequence_changes);
            }
        }
    }
}
