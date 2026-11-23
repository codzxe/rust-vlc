# AUDIT: crates/demux/src/asf/header.rs
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-02-14 |
| Lines / functions | 1085 / 18 (12 walk/parse fns + 6 small helpers/accessors) |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `demux::asf` has no consumer outside `crates/demux`: apps/cli/src/containers/mod.rs:50-75 registers no `"asf"` arm (an .asf/.wmv/.wma never dispatches to it), and media-probe's `ProbeRegistry::with_builtins()` (crates/media-probe/src/signature/mod.rs:130-147) registers 14 signatures — AVI, MP4, WAV, AIFF, Ogg, Mkv, Adts, Ts, Ps, Ivf, Mp3, Mp2, Annexb, Flac — with NO ASF signature (rg for `asf|wmv|wma` in crates/media-probe: zero matches). `lumen open foo.wmv` dies at the probe stage before any demuxer is constructed. Desktop embeds lumen-cli → same. |
| Findings | S0: 0 S1: 1 S2: 1 S3: 3 S4: 3 |
| Commands run | `target/debug/deps/demux-036dd705535b2fb3 asf::` → 36 passed, 0 failed (exit 0); rg searches (asf/wmv/wma in media-probe → none; AsfDemuxer users outside demux → none; fuzz/Cargo.toml → no fuzz_demux_asf) |

## 1. One-paragraph verdict
The header walk itself is real and heavily tested (36 asf tests pass, including an ffprobe
differential and sha-pinned committed fixtures), with genuine bounds discipline: 4 MiB header
ceiling, prefix-capped reads, iteration guards, field caps on every metadata string. But it
contains a crafted-input PANIC: the child-object walk adds a raw parsed u64 size to the offset
with plain `+` (line 558); a child size ≥ 2^64 − 4 MiB overflows, wraps past the
`end > header_size` guard, and the subsequent slice `&bytes[off + 24..end as usize]` has
start > end → panic in release, and the `+` itself panics in debug. That violates the module's
own house rule ("no `unwrap`/`expect`/`panic!` in non-test code", asf/mod.rs:34-41). A second
unchecked add of the same shape sits in the Header-Extension walk (line 996; debug-panic /
release-bounded-spin). Both are unreachable from the shipped CLI because the entire `demux::asf`
module is dead weight in the product: no probe signature, no registry arm — while the module doc
claims "the [`probe`] gate and the media-probe signature" landed. The signature claim is false.

## 2. What actually works (verified)
- `target/debug/deps/demux-036dd705535b2fb3 asf::` → `test result: ok. 36 passed; 0 failed`
  (exit 0) — includes `fixture_family_cross_checks_against_live_ffprobe`,
  `broadcast_fixture_refuses_typed_where_ffprobe_refuses`, sha256 pins, packet grammar, seek.
- Bounds re-derived by reading: `MAX_HEADER_BYTES` 4 MiB enforced before the big read
  (513-515); the whole-header read is prefix-capped (530-537); `MAX_TSD_BYTES` 64 KiB enforced
  (737-739); every metadata field clamped to `MAX_*` caps and to the object's bytes (885-905,
  944-959); all walk loops progress ≥ 24 bytes/iteration with count guards; top-level walk uses
  `checked_add` correctly (1054).

## 3. Findings
F-001 [S1]
Where: crates/demux/src/asf/header.rs:553-562
Code:
```rust
            let child = Guid::read(&bytes, off);
            let size = guid::u64le(&bytes, off + 16);
            if size < 24 {
                return Err(error::bad_object_size(header_offset + off as u64));
            }
            let end = off as u64 + size;
            if end > header_size {
                return Err(error::bad_object_size(header_offset + off as u64));
            }
            let payload = &bytes[off + 24..end as usize];
```
What is wrong: `off as u64 + size` is an unchecked u64 add on a fully attacker-controlled 8-byte
field. With `size ≥ 2^64 − off` the sum overflows: debug builds panic at the `+`; release builds
wrap to `end < off + 24`, which passes `end > header_size` and then
`&bytes[off + 24..end as usize]` panics ("range start ... > end"). Why it matters: a
malformed-header panic in a public library API — exactly the "unwrap on parsed field" class the
spec's S1 names, and a direct violation of the module's no-panic house rule (asf/mod.rs:34-41).
The 36-test suite cannot catch it (no test feeds a near-u64::MAX child size), and there is no
asf cargo-fuzz target (F-004) to catch it either. Not triggerable through the shipped lumen CLI
only because the whole module is unreachable there. What correct looks like:
`off.checked_add(size)` (as walk_top_level:1054 already does) → typed `bad_object_size`.
Confidence: HIGH — Verification: read (arithmetic proof; no test/fuzz coverage of the input
class — UNVERIFIED by execution: writing a repro requires a new test file, forbidden by
READ-ONLY rule).

F-002 [S2]
Where: crates/demux/src/asf/header.rs:994-1008
Code:
```rust
        while off + 24 <= end && count < MAX_EXTENSION_CHILDREN {
            let size = guid::u64le(payload, off + 16);
            if size < 24 || off as u64 + size > end as u64 {
                noted += (end - off) as u64;
                break;
            }
            if Guid::read(payload, off) != EXTENDED_STREAM_PROPERTIES {
                // Non-ESP children (language lists, payload
                // extension systems the plan flagged): counted as
                // ignored structure, damage-noted.
                noted += 1;
            }
            count += 1;
            off += size as usize;
```
What is wrong: same unchecked `off as u64 + size` (line 996) and unchecked `off += size as usize`
(line 1007). A near-u64::MAX extension-child size panics debug builds at the add; in release the
wrap makes `off` go backwards and the loop re-reads the same size until the
`MAX_EXTENSION_CHILDREN` guard exits (≤1024 wasted iterations — bounded, not a hang). What
correct looks like: checked_add on both. Confidence: HIGH — Verification: read.

F-003 [S3] (adjacent file — the module this header serves)
Where: crates/demux/src/asf/mod.rs:5-9 (claim) vs crates/media-probe/src/signature/mod.rs:130-147 (reality)
Code (the claim):
```rust
//! table. ASF-a landed the header walk — File Properties, Stream
//! Properties (WAVEFORMATEX / short-VIDEOINFOHEADER facts), Codec
//! List, Extended Content Description, a bounded Header Extension
//! child walk — plus the [`probe`] gate and the media-probe
//! signature. **ASF-b landed the Data Object packet walk**:
```
What is wrong: "the media-probe signature" does not exist. `with_builtins()` registers 14
signatures, none ASF; rg for `asf|wmv|wma` over crates/media-probe returns nothing; the CLI
container registry (apps/cli/src/containers/mod.rs:50-75) has no asf arm. The doc asserts a
shipped integration that was never wired — the whole 4000+ line ASF demuxer (a/b/c cards) is
dead code in the product. What correct looks like: either the signature + registry arm land or
the doc says "unwired". Confidence: HIGH — Verification: read + rg (both empty).

F-004 [S3]
Where: crates/demux/src/asf/header.rs:609-617 (pattern repeated at 619-623)
Code:
```rust
                CODEC_LIST => {
                    let (entries, noted) = Self::parse_codec_list(payload)?;
                    damage.total = damage.total.saturating_add(noted);
                    codec_list.extend(entries);
                }
                EXTENDED_CONTENT_DESCRIPTION => {
                    let (pairs, noted) = Self::parse_extended_content_description(payload)?;
                    damage.total = damage.total.saturating_add(noted);
```
What is wrong: `noted` is added straight to `damage.total` with NO event pushed, breaking
DamageLog's documented invariant "Total skipped bytes (the sum of `events`)"
(crates/demux/src/avi/header.rs:390-391). Worse, `noted` counts *entries* in some paths
(`noted += 1` per non-string ECD value at line 967) and *bytes* in others — `damage_total()`
(asf/mod.rs:363-364) is a sum of mixed units. The wav module surfaces damage honestly; this
module's observable is corrupt. What correct looks like: push events with a unit-stating reason,
or keep a separate counter. Confidence: HIGH — Verification: read.

F-005 [S3]
Where: crates/demux/src/asf/header.rs:798-805
Code:
```rust
        if wfx.channels == 0 || wfx.samples_per_sec == 0 {
            return Err(error::bad_waveformatex(payload_offset));
        }
        let tail = wfx.cb_size as usize;
        if 18 + tail > tsd.len() {
            return Err(error::bad_waveformatex(payload_offset));
        }
        Ok(wfx)
```
What is wrong: no channel ceiling and no `block_align == channels × width` check. The engine's
PCM decoder refuses channels > 64 (crates/codecs/src/audio/pcm/decoder.rs:407,
`MAX_PCM_CHANNELS = 64`) — an ASF file declaring 100-channel PCM enumerates a track the decode
path always rejects. This is exactly the "demux-ok/decode-fail divergence" the WAV module's
doctrine prohibits (wav/mod.rs:44-49, "one accepted-set"). Library-level only (module
unreachable in the product). What correct looks like: mirror the wav fmt rules.
Confidence: HIGH — Verification: read + rg of the decoder ceiling.

F-006 [S4] (adjacent file)
Where: crates/demux/src/asf/fuzz.rs:12-17
Code:
```rust
//! # Wiring status (honest)
//!
//! The cargo-fuzz target lives at
//! `fuzz/fuzz_targets/fuzz_demux_asf.rs` (Lead-owned nightly wiring,
//! see `fuzz/README.md`); the harness logic is meanwhile proven on
//! stable by the deterministic sweeps below.
```
What is wrong: `fuzz/fuzz_targets/fuzz_demux_asf.rs` does NOT exist (`ls` → No such file) and
fuzz/Cargo.toml declares 14 fuzz_demux_* bins, none asf. The "wiring status (honest)" paragraph
asserts a file that is absent — fabricated wiring status. The in-crate harness itself is real
and runs (its deterministic sweeps execute in the asf test suite). Confidence: HIGH —
Verification: ls + grep of fuzz/Cargo.toml.

F-007 [S4]
Where: crates/demux/src/asf/header.rs:854-921 (`parse_codec_list`) and 927-974 (`parse_extended_content_description`)
What is wrong: inconsistent malformed-metadata policy — codec-list payloads < 20 bytes return
silently with no damage note (858-860); a truncated codec-list info-length is FATAL
(`return Err(error::bad_object_size(0))` at 899-901) while truncated strings are damage-tolerant
breaks; ECD truncated name/type/value lengths are all fatal (939-941, 949-951). One corrupt
metadata object kills the entire file parse despite the module's damage-noting philosophy.
Also `ecd_len` is read and discarded (`let _ = ecd_len;`, line 748) with no consistency check
against the SP object's size. Confidence: HIGH — Verification: read.

F-008 [S4]
Where: crates/demux/src/asf/header.rs:1032-1034 (`walk_top_level`)
What is wrong: ANY top-level object (including junk trailing the Data Object) with declared
size < 24 is a FATAL `bad_object_size` — one corrupt trailing object size aborts `start()` for
an otherwise fully-demuxable file. The WAV walk's precedent damage-notes and ends the walk.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| utf16_lossy | 361-373 | REAL | — | lossy decode + NUL strip; odd trailing byte dropped |
| codec_descriptor | 381-452 | REAL | — | known→vocabulary, unknown→raw passthrough (honest refusal downstream) |
| AsfHeader::parse | 471-492 | REAL | — | 0-or-+12 anchor; short GUID reads are zero-safe (guid.rs) |
| AsfHeader::parse_at | 495-701 | REAL | S1 | the child walk's unchecked add (F-001); rest of bounds sound |
| parse_file_properties | 705-719 | REAL | — | pure field reads, all offsets < 80 checked by caller |
| parse_stream_properties | 725-775 | REAL | S4 | ecd_len discarded (F-007); TSD clamps + typed truncation correct |
| parse_waveformatex | 779-806 | REAL | S3 | missing channel/align parity (F-005) |
| parse_video_info | 816-849 | REAL | — | biSize window 40..=data_size; all indices ≤ 40 verified in bounds |
| parse_codec_list | 854-921 | REAL | S4 | F-007 inconsistencies; field caps work |
| parse_extended_content_description | 927-974 | REAL | S4 | fatal-on-truncation policy; caps work; type≠0 counted not guessed |
| walk_header_extension | 982-1010 | REAL | S2 | F-002 unchecked adds; bounded otherwise |
| walk_top_level | 1014-1084 | REAL | S4 | checked_add correct; fatal size<24 (F-008); honest no-data-object note |
| FileProperties::is_broadcast / declares_seekable | 149-162 | REAL | — | bit tests |
| VideoInfo::fourcc | 206-222 | REAL | — | lossy ASCII, observability only |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| minimal_audio_file_walks_to_one_pcm_track | YES | asserts full track table + codec descriptor |
| truncated_head_is_truncation_not_not_asf | YES | exact error code pinned |
| header_size_below_floor_and_overrun_refuse_bad_object_size | YES | both refusal codes pinned |
| header_above_memory_ceiling_refuses_typed | YES | 4 MiB ceiling pinned |
| broadcast_flags_refuse_typed_where_ffprobe_refuses | YES | fixture-driven, ffprobe parity |
| fixture_family_walks_match_the_measured_truth | YES | committed fixtures + sha256 pins + object trees |
| fixture_family_cross_checks_against_live_ffprobe | YES | runs live ffprobe (passed in this sandbox) |
| All 29 asf tests | YES (spot-checked 8) | exact codes/facts asserted; executed 36/36 pass |
| (gap) | — | NO test feeds a near-u64::MAX child size → F-001 survives the suite; no asf cargo-fuzz target exists to catch it (F-006) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "plus the [`probe`] gate and the media-probe signature" | asf/mod.rs:8-9 | FALSE | no ASF signature in media-probe (signature/mod.rs:130-147; rg zero hits) — F-003 |
| "The cargo-fuzz target lives at `fuzz/fuzz_targets/fuzz_demux_asf.rs`" | asf/fuzz.rs:14-16 | FALSE | file absent, fuzz/Cargo.toml has no such bin — F-006 |
| "no `unwrap`/`expect`/`panic!` in non-test code" (house rule) | asf/mod.rs:37-38 | FALSE-in-practice | F-001 panics on crafted input |
| "the header read is ceiling-checked before it is trusted into an allocation" | asf/mod.rs:40-42 | REAL | header.rs:513-537 verified |
| "every layout ... byte-verified against ffmpeg 7.1.5 output and closes its objects exactly" | header.rs:14-18 | REAL | fixture differential + sha pins pass (36/36) |
| field-map tables (FPO 80 B, SP ≥ 54, short video form, codec list, ECD order) | header.rs:20-56 | REAL | pinned by the passing fixture family tests |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable from shipped entry points (F-003) — `AsfDemuxer` has zero
  users outside crates/demux (rg over apps/, tools/, crates/: only src/asf/* and its tests).
- `asf::EXTENSIONS` (mod.rs) and `asf::probe` — zero callers outside the module's own tests
  (rg: none) — dead product code, kept for the never-landed wiring.
- `damage.events` for asf never surfaces anywhere (only `.total` read, mod.rs:364) — the
  event half of the DamageLog is dead data in this module.

## 8. Dependencies & flags
- Depends on `crate::avi::riff::SourceReader` and `crate::avi::header::DamageLog` (same
  cross-format reuse as wav); `super::guid`, `super::error`. No external crates; no `unsafe`;
  no feature gates; compiles in the release lane (baseline: release lane builds clean).
- `slice::as_chunks` (utf16_lossy:363) — stable-since-1.88 API; toolchain must be ≥1.88
  (compiles per baseline).

## 9. Open questions for the next agent
- ASF-b (`data.rs`) consumes `DataObjectLocation.size`/`packet_count` — does its packet walk
  re-check sizes against the stream (the "data-size-overrun" note claims per-packet clamping)?
- If ASF ever gets wired (probe signature + registry arm), F-001 becomes a remotely triggerable
  panic via `lumen open crafted.wmv` — fix before wiring, not after.
- The broadcast `$H`-prefix lane: the +12 anchor accepts a file whose first 16 bytes are
  anything — confirm nothing else in the tree treats offset-12 GUIDs as a probe signal.
