//! The Phase-5 container signatures (card 5.R3-REG): the formats
//! whose demuxer cards landed in waves 4–7, consolidated into the
//! builtin probe registry so `probe → format id → demuxer` covers
//! everything that landed (the R3 registration surface; each card's
//! worklog snippet named this exact seam).
//!
//! One struct per format, mirroring its demuxer's own
//! `probe(header)` gate byte-for-byte — the magic lives in `demux`
//! (layering forbids the crate edge), so this module is a
//! mirror-pinned twin: the round-trip suite
//! (`tests/registry_roundtrip.rs`) proves the mirror against the
//! committed fixtures the demuxer cards authored, and any drift is a
//! test failure, not a silent guess.
//!
//! ## Scoring (the doc 03 §4.2(e) ladder, honestly minimal)
//!
//! Every signature here awards [`SCORE_MAGIC_ANCHOR`] only: the
//! anchored magic is certain enough to select a demuxer, and the
//! deeper structure walks (chunk tables, PSI, EBML DocType) are the
//! demuxer's own typed domain — the probe never guesses past its
//! bytes. Track hints stay empty: the demuxers own track truth.
//!
//! ## Known content-probe gaps (recorded, not smoothed)
//!
//! - ID3-prefixed ADTS/AAC and ID3-prefixed raw MP3 files do not
//!   anchor (signatures are window-start-anchored by contract); the
//!   demuxers skip ID3 themselves, so the future open-path extension
//!   route still reaches them.
//! - Raw elementary streams (Annex-B `.264`/`.265`, raw `.mp3`) have
//!   no anchored magic at all — the ES demuxers are routed by
//!   extension in the open path, never by a content claim here.
//! - Matroska DocType (mkv vs webm) and the MP4 brand list stay
//!   demux-domain: the id is the family, `matroska`/`mp4`.

use crate::report::{FormatId, ProbeHit, SourceMeta};
use crate::signature::ProbePlugin;

use super::SCORE_MAGIC_ANCHOR;

/// The 188-byte MPEG-TS packet stride (ISO 13818-1 §2.4.3), mirrored
/// from `demux::ts::PACKET_BYTES` (no crate edge; mirror-pinned by
/// the round-trip suite).
const TS_PACKET_BYTES: usize = 188;

/// A magic-anchored hit at the ladder's anchor rung.
fn anchored(format: FormatId, consumed: usize) -> ProbeHit {
    ProbeHit {
        format,
        score: SCORE_MAGIC_ANCHOR,
        track_hints: Vec::new(),
        prefix_consumed: consumed,
    }
}

/// The WAV signature (card 5.A2): `RIFF`/`RF64`/`BW64` magic with
/// the `WAVE` form — mirrors `demux::wav::probe`.
pub struct WavSignature;

impl ProbePlugin for WavSignature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        if prefix.len() < 12 {
            return None;
        }
        let magic = &prefix[0..4];
        let form = &prefix[8..12];
        let hit = matches!(magic, b"RIFF" | b"RF64" | b"BW64") && form == b"WAVE";
        hit.then(|| anchored(FormatId::wav(), 12))
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["wav", "wave", "rf64", "bw64"]
    }
}

/// The AIFF/AIFC signature (card 5.A2): `FORM` magic with the
/// `AIFF`/`AIFC` form — mirrors `demux::aiff::probe`.
pub struct AiffSignature;

impl ProbePlugin for AiffSignature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        if prefix.len() < 12 {
            return None;
        }
        let magic = &prefix[0..4];
        let form = &prefix[8..12];
        let hit = magic == b"FORM" && (form == b"AIFF" || form == b"AIFC");
        hit.then(|| anchored(FormatId::aiff(), 12))
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["aiff", "aif", "aifc"]
    }
}

/// The Ogg signature (card 5.B1): the `OggS` page capture — mirrors
/// `demux::ogg::probe` (the version byte and CRC are the demuxer's
/// own typed domain).
pub struct OggSignature;

impl ProbePlugin for OggSignature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        (prefix.len() >= 4 && &prefix[0..4] == b"OggS").then(|| anchored(FormatId::ogg(), 4))
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ogg", "oga", "ogv", "opus", "spx"]
    }
}

/// The Matroska/WebM signature (card 5.D1): the EBML header magic
/// `0x1A45DFA3` — mirrors `demux::mkv::probe` (the DocType gate is
/// the demuxer's).
pub struct MkvSignature;

impl ProbePlugin for MkvSignature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        (prefix.len() >= 4
            && u32::from_be_bytes([prefix[0], prefix[1], prefix[2], prefix[3]]) == 0x1A45_DFA3)
            .then(|| anchored(FormatId::matroska(), 4))
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["mkv", "mka", "mk3d", "webm"]
    }
}

/// The ADTS/ADIF signature (card 5.C2b): the `ADIF` magic or the
/// 12-bit ADTS syncword with the spec-mandated zero layer bits —
/// `demux::adts::probe` plus the layer refinement, so a raw MPEG
/// audio frame (`0xFF Ex`, 11-bit sync + nonzero layer) is NOT
/// stolen: ADTS layer is always `00` (ISO 13818-7 §6.5.1), MP3's
/// Layer III is `01`.
pub struct AdtsSignature;

impl ProbePlugin for AdtsSignature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        if prefix.len() >= 4 && &prefix[0..4] == b"ADIF" {
            return Some(anchored(FormatId::adts(), 4));
        }
        (prefix.len() >= 2 && prefix[0] == 0xFF && prefix[1] & 0xF6 == 0xF0) // 12-bit sync + layer 00
            .then(|| anchored(FormatId::adts(), 2))
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["aac", "adts", "adif"]
    }
}

/// The MPEG-TS signature (card 5.I1): the `0x47` sync byte, plus the
/// second sync byte at the 188-byte stride — mirrors `demux::ts::
/// probe` (a lone sync byte proves nothing; a whole single-packet
/// file is the demuxer's own walk to judge).
pub struct TsSignature;

impl ProbePlugin for TsSignature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        if prefix.is_empty() || prefix[0] != 0x47 {
            return None;
        }
        if prefix.len() >= TS_PACKET_BYTES {
            if prefix[TS_PACKET_BYTES] != 0x47 {
                return None;
            }
            return Some(anchored(FormatId::mpegts(), TS_PACKET_BYTES + 1));
        }
        // Shorter than one packet: a lone sync byte is all the window
        // can see — the demuxer's own probe accepts it, so the claim
        // stays anchored (score 70, one byte consumed).
        Some(anchored(FormatId::mpegts(), 1))
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ts", "m2t", "mts"]
    }
}

/// The MPEG-PS signature (card 5.I2): the pack start code
/// `00 00 01 BA` — mirrors `demux::ps::probe` (MPEG-1 system and
/// MPEG-2 program streams / DVD-VOB packs share it; SCR length is
/// the demuxer's domain).
pub struct PsSignature;

impl ProbePlugin for PsSignature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        prefix
            .starts_with(&[0x00, 0x00, 0x01, 0xBA])
            .then(|| anchored(FormatId::mpegps(), 4))
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["mpg", "mpeg", "ps", "vob", "m2p"]
    }
}

/// The IVF signature (card 5.I3b): the `DKIF` magic — mirrors
/// `demux::ivf::probe` (header length, dims, rate and the AV01/VP90
/// fourcc gate are the demuxer's own typed refusals).
pub struct IvfSignature;

impl ProbePlugin for IvfSignature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        (prefix.len() >= 4 && &prefix[0..4] == b"DKIF").then(|| anchored(FormatId::ivf(), 4))
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ivf"]
    }
}
