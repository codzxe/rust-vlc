//! Shared test support for the 2.B4 acceptance suites (std-only; the
//! codecs crate deliberately carries no dev-dependencies — the corpus
//! is read straight from the committed `tests/fixtures` tree via
//! `CARGO_MANIFEST_DIR`, and every helper here is small enough to
//! audit in one screen).
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
            Some(v) => {
                Some(v.replace('_', "").parse::<u64>().map_err(|e| format!("stamp_us: {e}"))?)
            }
            None => None,
        },
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
    let mut frames = Vec::new();
    let mut cursor = start;
    while cursor + 8 <= end && cursor + 8 <= data.len() {
        let id = fourcc_at(data, cursor)?;
        let size = u32le_at(data, cursor + 4)? as usize;
        let body = cursor + 8;
        if body.checked_add(size).is_none_or(|last| last > data.len()) {
            return Err(format!("movi chunk at {cursor} overruns the file"));
        }
        if id == *b"00dc" {
            frames.push(data[body..body + size].to_vec());
        }
        let advanced = body + size + (size & 1);
        if advanced <= cursor {
            return Err(format!("non-advancing movi chunk at {cursor}"));
        }
        cursor = advanced;
    }
    Ok(frames)
}

// ---- MP4 extraction: the ISO-BMFF stbl walk (test-side) -----------------

fn u32be_at(data: &[u8], pos: usize) -> Result<u32, String> {
    let bytes = data
        .get(pos..pos + 4)
        .ok_or_else(|| format!("u32be read past end at {pos}"))?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// One ISO-BMFF box: type + payload range.
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
        let trak_boxes = iter_boxes(data, trak.start, trak.end)?;
        let Some(mdia) = find_box(&trak_boxes, b"mdia") else {
            continue;
        };
        let mdia_boxes = iter_boxes(data, mdia.start, mdia.end)?;
        let Some(minf) = find_box(&mdia_boxes, b"minf") else {
            continue;
        };
        let minf_boxes = iter_boxes(data, minf.start, minf.end)?;
        let Some(stbl) = find_box(&minf_boxes, b"stbl") else {
            continue;
        };
        let table = parse_stbl(data, stbl.start, stbl.end)?;
        if table.chunk_offsets.is_empty() || table.sample_sizes.is_empty() {
            continue;
        }
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
        let mut frames: Vec<Vec<u8>> = Vec::new();
        let mut sample = 0usize;
        let mut plausible = true;
        'chunks: for (index, &chunk_offset) in table.chunk_offsets.iter().enumerate() {
            let mut offset = chunk_offset;
            for _ in 0..per_chunk[index] {
                let Some(&size) = table.sample_sizes.get(sample) else {
                    plausible = false;
                    break 'chunks;
                };
                let start = offset as usize;
                let end = start
                    .checked_add(size as usize)
                    .ok_or_else(|| "sample size overflow".to_string())?;
                let bytes = data
                    .get(start..end)
                    .ok_or_else(|| format!("sample {sample} overruns the file"))?;
                if sample == 0 && bytes.get(..2) != Some(&[0xff, 0xd8][..]) {
                    plausible = false;
                    break 'chunks;
                }
                frames.push(bytes.to_vec());
                sample += 1;
                offset += size;
            }
        }
        if plausible && sample == table.sample_sizes.len() {
            return Ok(frames);
        }
    }
    Err("no trak yielded a complete MJPEG sample table".to_string())
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
    let text = String::from_utf8(std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?)
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
    if width == 0 || height == 0 || width % 8 != 0 || height % 8 != 0 {
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
