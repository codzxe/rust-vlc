#![allow(dead_code)]
//! Shared test support for the 2.B4 acceptance suites (std-only; the
//! codecs crate deliberately carries no dev-dependencies — the corpus
//! is read straight from the committed `tests/fixtures` tree via
//! `CARGO_MANIFEST_DIR`, and every helper here is small enough to
//! audit in one screen).
//!
//! (Each test binary compiles this module; not every helper is used
//! by every binary, hence the module-level allow above.)
//!
//! Contents:
//! - fixture access (`fixture_path` / `fixture_bytes`);
//! - a std-only SHA-256 (`sha256_hex`) pinned against the corpus's
//!   committed whole-file digests by the first test that uses it;
//! - a purpose-built reader for the class-A expectation TOML (the
//!   fields the decoder ACs consume);
//! - minimal, honest container walkers used to **extract** the 60
//!   committed JPEG frames from each parent fixture: a RIFF `movi`
//!   walk for the AVI parent and an ISO-BMFF `stbl` walk for the MP4
//!   parent. These are test-side extraction aids, not a demuxer: their
//!   correctness is machine-checked by the per-frame sha256 match
//!   against the expectation's `video_sample_sha256` (a wrong walk
//!   cannot produce the writer's digests). The production demuxer is
//!   card 2.B2's `demux` crate (not a dependency of this branch).

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Repository fixtures root, relative to this crate's manifest.
fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

/// Absolute path of a committed fixture (e.g.
/// `"generated/valid/avi_mjpeg_pcm.avi"`).
pub fn fixture_path(relative: &str) -> PathBuf {
    fixtures_root().join(relative)
}

/// Read a committed fixture's bytes.
///
/// # Errors
/// Propagates filesystem errors verbatim (the tests render them).
pub fn fixture_bytes(relative: &str) -> Result<Vec<u8>, String> {
    std::fs::read(fixture_path(relative))
        .map_err(|e| format!("{}: {e}", fixture_path(relative).display()))
}

// ---- SHA-256 (FIPS 180-4, public specification) -------------------------

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256 of `data`, lowercase hex (pinned against the corpus's
/// committed digests by `expectation_sha_pins_the_reader`).
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 64];
    for block in message.chunks(64) {
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = String::with_capacity(64);
    for word in h {
        out.push_str(&format!("{word:08x}"));
    }
    out
}

// ---- expectation/1 reader (the fields the decoder ACs consume) ----------

/// The class-A parent expectation fields consumed by the 2.B4 ACs.
#[derive(Debug, Clone)]
pub struct ParentExpectation {
    /// Committed whole-file sha256 (pins the SHA-256 implementation).
    pub sha256: String,
    /// Committed file size in bytes.
    pub size_bytes: u64,
    /// Declared video frame count (60).
    pub video_sample_count: u64,
    /// Writer-declared per-frame JPEG sha256 (the extraction oracle).
    pub video_sample_sha256: Vec<String>,
    /// `[first_frame]` luma block-mean descriptor digest (doc 10
    /// §5.4; recorded from Lumen's first compliant decode, card 2.B4).
    pub first_frame_hash_y: Option<String>,
    /// `[first_frame]` presentation stamp, microseconds.
    pub first_frame_stamp_us: Option<u64>,
    /// `[essence_v0]` audio-stream sha256 (the generator's own PCM —
    /// the bit-exact decode oracle for card 2.B5).
    pub audio_stream_sha256: String,
    /// `[essence_v0]` audio-stream byte length.
    pub audio_stream_bytes: u64,
    /// `[audio_probe]` first-samples MD5 (doc 10 §5.4; derived by the
    /// 2.A3 writer from its own samples, filled in by card 2.B5).
    pub audio_probe_md5: Option<String>,
}

/// Parse the subset of a class-A expectation TOML the decoder ACs
/// consume. Purpose-built (the corpus files are generated, ASCII,
/// single-key-per-line); unknown fields are ignored.
///
/// # Errors
/// A human-readable reason for every structural surprise.
pub fn parse_parent_expectation(relative: &str) -> Result<ParentExpectation, String> {
    let text = String::from_utf8(fixture_bytes(relative)?)
        .map_err(|e| format!("{}: not UTF-8: {e}", fixture_path(relative).display()))?;
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    let mut section = String::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed[1..trimmed.len() - 1].to_string();
            continue;
        }
        let Some((raw_key, first_value)) = trimmed.split_once('=') else {
            return Err(format!("unparsable line: {trimmed:?}"));
        };
        let key = raw_key.trim().to_string();
        let mut value = first_value.trim().to_string();
        if value.starts_with('[') && !value.ends_with(']') {
            // Multi-line array: consume until the closing bracket.
            for follow in lines.by_ref() {
                let follow = follow.trim();
                value.push(' ');
                value.push_str(follow);
                if follow.ends_with(']') {
                    break;
                }
            }
        }
        fields.insert(format!("{section}.{key}"), value);
    }

    let string_of = |key: &str| -> Result<String, String> {
        fields
            .get(key)
            .map(|v| v.trim_matches('"').to_string())
            .ok_or_else(|| format!("missing field {key:?}"))
    };
    let uint_of = |key: &str| -> Result<u64, String> {
        let raw = string_of(key)?;
        raw.replace('_', "")
            .parse::<u64>()
            .map_err(|e| format!("{key}: {e}"))
    };

    let video_sample_sha256 = fields
        .get("essence_v0.video_sample_sha256")
        .map(|block| {
            block
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(|entry| entry.trim().trim_matches('"').to_string())
                .filter(|entry| !entry.is_empty())
                .collect::<Vec<_>>()
        })
        .ok_or_else(|| "missing field essence_v0.video_sample_sha256".to_string())?;

    Ok(ParentExpectation {
        sha256: string_of(".sha256")?,
        size_bytes: uint_of(".size_bytes")?,
        video_sample_count: uint_of("essence_v0.video_sample_count")?,
        first_frame_hash_y: fields
            .get("first_frame.hash_y")
            .map(|v| v.trim_matches('"').to_string()),
        first_frame_stamp_us: match fields.get("first_frame.stamp_us") {
            Some(v) => Some(
                v.replace('_', "")
                    .parse::<u64>()
                    .map_err(|e| format!("stamp_us: {e}"))?,
            ),
            None => None,
        },
        audio_stream_sha256: string_of("essence_v0.audio_stream_sha256")?,
        audio_stream_bytes: uint_of("essence_v0.audio_stream_bytes")?,
        audio_probe_md5: fields
            .get("audio_probe.first_samples_md5")
            .map(|v| v.trim_matches('"').to_string()),
        video_sample_sha256,
    })
}

// ---- AVI extraction: the RIFF movi walk (test-side) ----------------------

fn u32le_at(data: &[u8], pos: usize) -> Result<u32, String> {
    let bytes = data
        .get(pos..pos + 4)
        .ok_or_else(|| format!("u32le read past end at {pos}"))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn fourcc_at(data: &[u8], pos: usize) -> Result<[u8; 4], String> {
    data.get(pos..pos + 4)
        .map(|s| [s[0], s[1], s[2], s[3]])
        .ok_or_else(|| format!("fourcc read past end at {pos}"))
}

/// Extract the `00dc` (MJPEG) chunk payloads from the AVI parent's
/// `movi` list, in stream order (each is one complete JPEG frame).
///
/// Walk semantics (RIFF/AVI 1.0): chunks are `fourcc + u32le size`,
/// data follows, and an odd `size` carries one pad byte so the next
/// chunk stays even-aligned. The walk is strict enough that a damaged
/// file errors instead of yielding wrong bytes — the per-frame sha256
/// match is the real oracle.
///
/// # Errors
/// Structural surprises (not RIFF/AVI, truncated, runaway).
pub fn extract_avi_mjpeg_frames(data: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    extract_avi_movi_chunks(data, b"00dc")
}

/// Extract the `01wb` (PCM) chunk payloads from the AVI parent's
/// `movi` list, in stream order (card 2.B5's audio AC — the per-chunk
/// concatenation is sha256-pinned against the expectation's
/// `audio_stream_sha256`).
///
/// # Errors
/// Structural surprises (not RIFF/AVI, truncated, runaway).
pub fn extract_avi_pcm_chunks(data: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    extract_avi_movi_chunks(data, b"01wb")
}

/// The `movi` walk itself, parameterized by the stream chunk id (the
/// 2.B4 walker factored one level down; both callers are digest-pinned
/// by their own suites).
///
/// # Errors
/// Structural surprises (not RIFF/AVI, truncated, runaway).
fn extract_avi_movi_chunks(data: &[u8], chunk_id: &[u8; 4]) -> Result<Vec<Vec<u8>>, String> {
    if fourcc_at(data, 0)? != *b"RIFF" || fourcc_at(data, 8)? != *b"AVI " {
        return Err("not an AVI RIFF file".to_string());
    }
    // Locate the movi list: walk top-level chunks inside RIFF.
    let riff_end = 8u64 + u64::from(u32le_at(data, 4)?);
    let mut pos = 12usize;
    let mut movi = None;
    while (pos as u64) < riff_end {
        let id = fourcc_at(data, pos)?;
        let size = u32le_at(data, pos + 4)? as usize;
        let body = pos + 8;
        if id == *b"LIST" && fourcc_at(data, body)? == *b"movi" {
            movi = Some((body + 4, body + size));
            break;
        }
        let advanced = body + size + (size & 1);
        if advanced <= pos {
            return Err(format!("non-advancing top-level chunk at {pos}"));
        }
        pos = advanced;
    }
    let Some((start, end)) = movi else {
        return Err("no movi list".to_string());
    };
    let mut chunks = Vec::new();
    let mut cursor = start;
    while cursor + 8 <= end && cursor + 8 <= data.len() {
        let id = fourcc_at(data, cursor)?;
        let size = u32le_at(data, cursor + 4)? as usize;
        let body = cursor + 8;
        if body.checked_add(size).is_none_or(|last| last > data.len()) {
            return Err(format!("movi chunk at {cursor} overruns the file"));
        }
        if id == *chunk_id {
            chunks.push(data[body..body + size].to_vec());
        }
        let advanced = body + size + (size & 1);
        if advanced <= cursor {
            return Err(format!("non-advancing movi chunk at {cursor}"));
        }
        cursor = advanced;
    }
    Ok(chunks)
}

/// The `strf` (WAVEFORMATEX) payload of the AVI parent's **audio**
/// stream: walk `LIST hdrl` → each `LIST strl` → the `strh` whose
/// fccType is `auds`, and return its sibling `strf` bytes verbatim
/// (card 2.B5 configures the decoder from the container's own init
/// bytes — not a synthesized stand-in).
///
/// # Errors
/// Structural surprises (no hdrl, no auds stream, no strf).
pub fn extract_avi_audio_strf(data: &[u8]) -> Result<Vec<u8>, String> {
    if fourcc_at(data, 0)? != *b"RIFF" || fourcc_at(data, 8)? != *b"AVI " {
        return Err("not an AVI RIFF file".to_string());
    }
    let riff_end = 8u64 + u64::from(u32le_at(data, 4)?);
    let mut pos = 12usize;
    let mut hdrl = None;
    while (pos as u64) < riff_end {
        let id = fourcc_at(data, pos)?;
        let size = u32le_at(data, pos + 4)? as usize;
        let body = pos + 8;
        if id == *b"LIST" && fourcc_at(data, body)? == *b"hdrl" {
            hdrl = Some((body + 4, body + size));
            break;
        }
        let advanced = body + size + (size & 1);
        if advanced <= pos {
            return Err(format!("non-advancing top-level chunk at {pos}"));
        }
        pos = advanced;
    }
    let Some((start, end)) = hdrl else {
        return Err("no hdrl list".to_string());
    };
    // Walk the strl lists inside hdrl.
    let mut cursor = start;
    while cursor + 8 <= end && cursor + 8 <= data.len() {
        let id = fourcc_at(data, cursor)?;
        let size = u32le_at(data, cursor + 4)? as usize;
        let body = cursor + 8;
        if id == *b"LIST" && fourcc_at(data, body)? == *b"strl" {
            let strl_start = body + 4;
            let strl_end = body + size;
            let mut strh = None;
            let mut strf = None;
            let mut inner = strl_start;
            while inner + 8 <= strl_end && inner + 8 <= data.len() {
                let sub_id = fourcc_at(data, inner)?;
                let sub_size = u32le_at(data, inner + 4)? as usize;
                let sub_body = inner + 8;
                if sub_body
                    .checked_add(sub_size)
                    .is_none_or(|last| last > data.len())
                {
                    return Err(format!("strl chunk at {inner} overruns the file"));
                }
                if sub_id == *b"strh" {
                    strh = Some(data[sub_body..sub_body + sub_size].to_vec());
                } else if sub_id == *b"strf" {
                    strf = Some(data[sub_body..sub_body + sub_size].to_vec());
                }
                let advanced = sub_body + sub_size + (sub_size & 1);
                if advanced <= inner {
                    return Err(format!("non-advancing strl chunk at {inner}"));
                }
                inner = advanced;
            }
            let is_audio = strh
                .as_deref()
                .and_then(|bytes| bytes.get(..4))
                .is_some_and(|fcc| fcc == b"auds");
            if is_audio {
                return strf.ok_or_else(|| "auds stream without strf".to_string());
            }
        }
        let advanced = body + size + (size & 1);
        if advanced <= cursor {
            return Err(format!("non-advancing hdrl chunk at {cursor}"));
        }
        cursor = advanced;
    }
    Err("no auds stream in hdrl".to_string())
}

// ---- MP4 extraction: the ISO-BMFF stbl walk (test-side) -----------------

fn u32be_at(data: &[u8], pos: usize) -> Result<u32, String> {
    let bytes = data
        .get(pos..pos + 4)
        .ok_or_else(|| format!("u32be read past end at {pos}"))?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// One ISO-BMFF box: type + payload range.
#[derive(Debug, Clone)]
struct BoxRange {
    kind: [u8; 4],
    start: usize,
    end: usize,
}

/// Iterate the boxes in `data[start..end]` (a container box payload).
fn iter_boxes(data: &[u8], start: usize, end: usize) -> Result<Vec<BoxRange>, String> {
    let mut out = Vec::new();
    let mut pos = start;
    while pos + 8 <= end && pos + 8 <= data.len() {
        let size32 = u32be_at(data, pos)? as usize;
        let kind = fourcc_at(data, pos + 4)?;
        let size = if size32 == 1 {
            // 64-bit sizes never occur in the corpus; reject loudly.
            return Err("64-bit box size in fixture (not emitted by the writer)".to_string());
        } else if size32 == 0 {
            end - pos
        } else {
            size32
        };
        let box_end = pos
            .checked_add(size)
            .ok_or_else(|| format!("box size overflow at {pos}"))?;
        if box_end > end || box_end > data.len() {
            return Err(format!("box at {pos} overruns its container"));
        }
        out.push(BoxRange {
            kind,
            start: pos + 8,
            end: box_end,
        });
        if size == 0 {
            break;
        }
        pos = box_end;
    }
    Ok(out)
}

/// Locate the first sub-box of the given type in a container payload.
fn find_box<'a>(boxes: &'a [BoxRange], kind: &[u8; 4]) -> Option<&'a BoxRange> {
    boxes.iter().find(|b| &b.kind == kind)
}

/// Sample table of one trak: chunk offsets (stco), sample sizes
/// (stsz), samples-per-chunk (stsc) — enough to compute per-sample
/// file offsets.
struct SampleTable {
    chunk_offsets: Vec<u64>,
    sample_sizes: Vec<u64>,
    /// (first_chunk 1-based, samples_per_chunk) runs.
    stsc: Vec<(u64, u64)>,
}

fn parse_stbl(data: &[u8], start: usize, end: usize) -> Result<SampleTable, String> {
    let boxes = iter_boxes(data, start, end)?;
    let mut table = SampleTable {
        chunk_offsets: Vec::new(),
        sample_sizes: Vec::new(),
        stsc: Vec::new(),
    };
    for b in &boxes {
        let payload = data
            .get(b.start..b.end)
            .ok_or_else(|| format!("stbl box range {}..{}", b.start, b.end))?;
        match &b.kind {
            b"stco" => {
                let count = u32be_at(payload, 4)? as usize;
                for i in 0..count {
                    table
                        .chunk_offsets
                        .push(u64::from(u32be_at(payload, 8 + i * 4)?));
                }
            }
            b"stsz" => {
                let fixed = u32be_at(payload, 4)?;
                let count = u32be_at(payload, 8)? as usize;
                if fixed != 0 {
                    for _ in 0..count {
                        table.sample_sizes.push(u64::from(fixed));
                    }
                } else {
                    for i in 0..count {
                        table
                            .sample_sizes
                            .push(u64::from(u32be_at(payload, 12 + i * 4)?));
                    }
                }
            }
            b"stsc" => {
                let count = u32be_at(payload, 4)? as usize;
                for i in 0..count {
                    let first_chunk = u64::from(u32be_at(payload, 8 + i * 12)?);
                    let per_chunk = u64::from(u32be_at(payload, 12 + i * 12)?);
                    table.stsc.push((first_chunk, per_chunk));
                }
            }
            _ => {}
        }
    }
    Ok(table)
}

/// Walk one `trak` down to its `stbl` (the shared moov→trak→mdia→
/// minf→stbl descent; the 2.B4 walker factored one level down for the
/// 2.B5 audio walkers).
///
/// # Errors
/// Structural surprises inside the trak (bad boxes).
fn trak_stbl(data: &[u8], trak: &BoxRange) -> Result<Option<BoxRange>, String> {
    let trak_boxes = iter_boxes(data, trak.start, trak.end)?;
    let Some(mdia) = find_box(&trak_boxes, b"mdia") else {
        return Ok(None);
    };
    let mdia_boxes = iter_boxes(data, mdia.start, mdia.end)?;
    let Some(minf) = find_box(&mdia_boxes, b"minf") else {
        return Ok(None);
    };
    let minf_boxes = iter_boxes(data, minf.start, minf.end)?;
    Ok(find_box(&minf_boxes, b"stbl").cloned())
}

/// Expand a parsed sample table to its samples, in decode order.
/// `None` when the table is internally inconsistent (stsc/stsz
/// mismatch) — the caller may try another trak.
///
/// # Errors
/// Structural surprises (sample overruns the file, size overflow).
fn expand_stbl_samples(data: &[u8], table: &SampleTable) -> Result<Option<Vec<Vec<u8>>>, String> {
    // Expand stsc to per-chunk sample counts (the run whose
    // first_chunk is the largest ≤ this chunk's 1-based index).
    let mut per_chunk = vec![0u64; table.chunk_offsets.len()];
    for (index, slot) in per_chunk.iter_mut().enumerate() {
        let chunk_1based = index as u64 + 1;
        let mut count = 0u64;
        for &(first, samples) in &table.stsc {
            if first <= chunk_1based {
                count = samples;
            }
        }
        *slot = count;
    }
    let mut samples: Vec<Vec<u8>> = Vec::new();
    for (index, &chunk_offset) in table.chunk_offsets.iter().enumerate() {
        let mut offset = chunk_offset;
        for _ in 0..per_chunk[index] {
            let Some(&size) = table.sample_sizes.get(samples.len()) else {
                return Ok(None);
            };
            let start = offset as usize;
            let end = start
                .checked_add(size as usize)
                .ok_or_else(|| "sample size overflow".to_string())?;
            let bytes = data
                .get(start..end)
                .ok_or_else(|| format!("sample {} overruns the file", samples.len()))?;
            samples.push(bytes.to_vec());
            offset += size;
        }
    }
    if samples.len() == table.sample_sizes.len() {
        Ok(Some(samples))
    } else {
        Ok(None)
    }
}

/// Extract the video (MJPEG) samples of the MP4 parent, in decode
/// order: walk `moov` → each `trak` → `mdia` → `minf` → `stbl`, parse
/// `stco`/`stsz`/`stsc`, expand to per-sample file offsets, and keep
/// the trak whose first sample starts with the JPEG SOI. Strict
/// enough to error on damage; the sha256 match is the real oracle.
///
/// # Errors
/// Structural surprises (not MP4, missing tables, overrun).
pub fn extract_mp4_mjpeg_frames(data: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    let top = iter_boxes(data, 0, data.len())?;
    let moov = find_box(&top, b"moov").ok_or("no moov box")?;
    let moov_boxes = iter_boxes(data, moov.start, moov.end)?;
    for trak in moov_boxes.iter().filter(|b| &b.kind == b"trak") {
        let Some(stbl) = trak_stbl(data, trak)? else {
            continue;
        };
        let table = parse_stbl(data, stbl.start, stbl.end)?;
        if table.chunk_offsets.is_empty() || table.sample_sizes.is_empty() {
            continue;
        }
        let Some(samples) = expand_stbl_samples(data, &table)? else {
            continue;
        };
        if samples
            .first()
            .is_some_and(|bytes| bytes.get(..2) == Some(&[0xff, 0xd8][..]))
        {
            return Ok(samples);
        }
    }
    Err("no trak yielded a complete MJPEG sample table".to_string())
}

/// The audio facts the MP4 parent's `sowt` AudioSampleEntry declares
/// (the stsd side of the decoder-config seam for card 2.B5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mp4AudioFacts {
    /// Channel count (AudioSampleEntry channelcount).
    pub channels: u16,
    /// Samples per second (samplerate, 16.16 fixed point).
    pub rate: u32,
    /// Bits per sample (samplesize).
    pub bits: u16,
}

/// Locate the `sowt` audio trak of the MP4 parent and return its stbl
/// range + the AudioSampleEntry facts (channels/rate/bits read from
/// the stsd's first entry — the writer's fixed v0 layout).
///
/// # Errors
/// Structural surprises (no moov, no sowt trak, truncated entry).
fn mp4_audio_stbl(data: &[u8]) -> Result<(BoxRange, Mp4AudioFacts), String> {
    let top = iter_boxes(data, 0, data.len())?;
    let moov = find_box(&top, b"moov").ok_or("no moov box")?;
    let moov_boxes = iter_boxes(data, moov.start, moov.end)?;
    for trak in moov_boxes.iter().filter(|b| &b.kind == b"trak") {
        let Some(stbl) = trak_stbl(data, trak)? else {
            continue;
        };
        let stbl_boxes = iter_boxes(data, stbl.start, stbl.end)?;
        let Some(stsd) = find_box(&stbl_boxes, b"stsd") else {
            continue;
        };
        let payload = data
            .get(stsd.start..stsd.end)
            .ok_or_else(|| format!("stsd range {}..{}", stsd.start, stsd.end))?;
        // stsd payload: version/flags (4) + entry_count (4) + first
        // entry: size (4) + format (4) + AudioSampleEntry body.
        let format = payload.get(12..16).ok_or("stsd first entry truncated")?;
        if format != b"sowt" {
            continue;
        }
        let u16be = |at: usize| -> Result<u16, String> {
            let bytes = payload
                .get(at..at + 2)
                .ok_or_else(|| format!("sowt entry field at {at} truncated"))?;
            Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
        };
        let rate_fixed = payload
            .get(16 + 24..16 + 24 + 4)
            .ok_or("sowt samplerate truncated")?;
        let rate =
            u32::from_be_bytes([rate_fixed[0], rate_fixed[1], rate_fixed[2], rate_fixed[3]]) >> 16;
        let facts = Mp4AudioFacts {
            channels: u16be(16 + 16)?,
            rate,
            bits: u16be(16 + 18)?,
        };
        return Ok((stbl, facts));
    }
    Err("no sowt audio trak".to_string())
}

/// Extract the audio (PCM) samples of the MP4 parent, in decode
/// order — the `sowt` trak's stbl expansion (card 2.B5's audio AC;
/// the concatenation is sha256-pinned against the expectation's
/// `audio_stream_sha256`).
///
/// # Errors
/// Structural surprises (not MP4, no sowt trak, overrun).
pub fn extract_mp4_pcm_chunks(data: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    let (stbl, _facts) = mp4_audio_stbl(data)?;
    let table = parse_stbl(data, stbl.start, stbl.end)?;
    expand_stbl_samples(data, &table)?.ok_or_else(|| "sowt sample table inconsistent".to_string())
}

/// The audio stream facts the MP4 parent declares (channels, rate,
/// bits per sample) — the stsd equivalent of the AVI `strf`.
///
/// # Errors
/// Structural surprises (no sowt trak, truncated entry).
pub fn extract_mp4_audio_facts(data: &[u8]) -> Result<Mp4AudioFacts, String> {
    mp4_audio_stbl(data).map(|(_, facts)| facts)
}

// ---- reference digest corpus --------------------------------------------

/// The committed decode-reference digests: one sha256 per frame, over
/// the decoder's I420 output (`Y || Cb || Cr`, 320×240 + 160×120 +
/// 160×120), frame order = stream order. Provenance and the ffmpeg
/// oracle tolerance are recorded in the file header (worklog 2.B4).
///
/// # Errors
/// Missing/unparsable file.
pub fn reference_digests() -> Result<Vec<String>, String> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/mjpeg_decode_reference.txt");
    let text =
        String::from_utf8(std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?)
            .map_err(|e| format!("{}: not UTF-8: {e}", path.display()))?;
    Ok(text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|line| line.trim().to_string())
        .collect())
}

// ---- first-frame descriptor (doc 10 §5.4 `hash_y`) ----------------------

/// The luma 8×8-block mean descriptor of doc 10 §5.4 (doc 03 §8
/// FrameRecorder's shape): each 8×8 block of the luma plane reduces
/// to its mean rounded half-up (a u8 — means live in 0..=255), in
/// raster order; the digest is the SHA-256 of those bytes.
///
/// # Errors
/// Dimensions not a multiple of 8, or a plane-length mismatch.
pub fn luma_block_mean_digest(y: &[u8], width: usize, height: usize) -> Result<String, String> {
    if width == 0 || height == 0 || !width.is_multiple_of(8) || !height.is_multiple_of(8) {
        return Err(format!("luma {width}x{height} is not a multiple of 8"));
    }
    if y.len() != width * height {
        return Err(format!(
            "luma plane is {} bytes, want {}",
            y.len(),
            width * height
        ));
    }
    let mut means = Vec::with_capacity((width / 8) * (height / 8));
    for by in 0..height / 8 {
        for bx in 0..width / 8 {
            let mut sum = 0u32;
            for r in 0..8 {
                let row_start = (by * 8 + r) * width + bx * 8;
                sum += y[row_start..row_start + 8]
                    .iter()
                    .map(|&b| u32::from(b))
                    .sum::<u32>();
            }
            means.push(((sum + 32) / 64) as u8);
        }
    }
    Ok(sha256_hex(&means))
}

// ---- MD5 (RFC 1321) + [audio_probe] (doc 10 §5.4, card 2.B5) ------------

/// MD5 (RFC 1321), std-only by hand — the [audio_probe] mirror of the
/// xtask writer's implementation (the codecs crate deliberately
/// carries no dev-dependencies). Identity digest for test
/// comparability, not a security primitive. Pinned against the RFC's
/// own test suite by `md5_reader_pinned_against_rfc1321_vectors` in
/// the 2.B5 AC suite; the writer and this copy pin each other through
/// the committed expectation value.
pub fn md5_hex(data: &[u8]) -> String {
    // Per-round shift amounts (RFC 1321 §3.4).
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    // Sine-derived table (RFC 1321 §3.4, the public spec table).
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];
    let mut state: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
    // Pad: 0x80, zeros to 56 mod 64, 8-byte LITTLE-endian bit length.
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_le_bytes());

    for block in message.chunks(64) {
        let mut m = [0u32; 16];
        for (i, word) in m.iter_mut().enumerate() {
            *word = u32::from_le_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        let [mut a, mut b, mut c, mut d] = state;
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                (a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g])).rotate_left(S[i]),
            );
            a = temp;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }
    // Digest words serialize LITTLE-endian (RFC 1321 §3.5).
    let mut out = String::with_capacity(32);
    for word in state {
        out.push_str(&format!("{:08x}", word.swap_bytes()));
    }
    out
}

/// The `[audio_probe]` sample count (doc 10 §5.4 "first 4800 samples",
/// pinned as sample-instants — the writer's `AUDIO_PROBE_INSTANTS`
/// is the same value and the definition's home).
pub const AUDIO_PROBE_INSTANTS: usize = 4800;

/// The decoder-side `[audio_probe]` digest (doc 10 §5.4): the pinned
/// conversion of the first [`AUDIO_PROBE_INSTANTS`] sample-instants of
/// a decoded interleaved S16 stream — each scalar read in the host's
/// native byte order (the decoder normalizes output to native), pinned
/// converted to IEEE-754 f32 as `v / 32768.0` (exact — every i16 is
/// exact in f32, the divisor is a power of two), serialized
/// little-endian, MD5. Byte-for-byte the same value the xtask writer
/// derives from its own generated s16le samples: writer and decoder
/// pin each other through the committed expectation.
///
/// # Errors
/// Fewer than [`AUDIO_PROBE_INSTANTS`] whole sample-instants present.
pub fn audio_probe_md5(interleaved_s16_native: &[u8], channels: u32) -> Result<String, String> {
    let frame_bytes = channels as usize * 2;
    let need = AUDIO_PROBE_INSTANTS * frame_bytes;
    if interleaved_s16_native.len() < need {
        return Err(format!(
            "probe needs {need} bytes for {AUDIO_PROBE_INSTANTS} instants, got {}",
            interleaved_s16_native.len()
        ));
    }
    let mut probe = Vec::with_capacity(AUDIO_PROBE_INSTANTS * frame_bytes * 2);
    for pair in interleaved_s16_native[..need].as_chunks::<2>().0 {
        let sample = i16::from_ne_bytes(*pair);
        probe.extend_from_slice(&(f32::from(sample) / 32_768.0).to_le_bytes());
    }
    Ok(md5_hex(&probe))
}
