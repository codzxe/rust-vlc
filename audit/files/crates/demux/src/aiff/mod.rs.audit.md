# AUDIT: crates/demux/src/aiff/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-21 |
| Lines / functions | 707 / 25 (incl. trait impl) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → `apps/cli` probe → `apps/cli/src/containers/mod.rs:53` `"aiff" => aiff::open(stream, opts)` → `apps/cli/src/containers/aiff.rs:56` `AiffDemuxer::new(stream)`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "AiffDemuxer"` (callers: cli containers + tests); `rg -n "aiff" apps/cli/src/containers/mod.rs` (dispatch arm); `sed -n` on every quoted range; `rg -n "fn seek|fn reanchor" crates/plugin-api/src/demux.rs` (seek contract: "Does **not** move the read position"); `rg -n "sticky" crates/demux/src/wav/mod.rs` (twin comparison); `rg -n "aiff" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (30 unit tests + doctest pass); baseline `sed -n '2223,2234p'` (differential fixture suite 8/8) |

## 1. One-paragraph verdict
Real, reachable, executed AIFF/AIFC demuxer. Header walk (FORM/COMM/SSND, skip-tolerant
chunks), CBR packetizer with pooled payloads, arithmetic seek, typed sticky errors —
all traced line by line; 30 aiff unit tests + doctest + the ffmpeg-differential
fixture suite pass in the committed baseline (no-failfast run). Two error-taxonomy
defects: structurally-short COMM/SSND chunk payloads (declared short by the file
itself, not cut by the stream) are reported as `demux.aiff.truncated` with a
**fabricated** `expected` byte count the file never declared — the parse layer's own
contract (`chunks::parse_comm` doc: "an AIFC tail too short for its compression type"
→ `bad_structure`) is contradicted by this file's pre-emption. Arithmetic, bounds,
pooling, and seek math checked; no panics reachable (all divisions guarded by
parse-enforced invariants: rate ≥ 1, channels 1..=64, width ∈ {8(raw),16,24,32}).

## 2. What actually works (verified)
— verified by `rg -n "aiff" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → lines 1523–1552: all 28 `aiff::tests`/`chunks`/`fuzz` tests `... ok`, incl. `seek_maps_frames_and_reanchors`, `declared_ssnd_past_actual_delivers_prefix_then_truncated`, `truncations_at_every_header_site_survive`.
— verified by baseline `sed -n '2223,2234p'` of same log → `every_aiff_fixture_matches_the_ffmpeg_truth ... ok` (8/8 in `tests/wav_aiff_fixtures.rs`, differential against ffmpeg raw dumps) and doctest `crates/demux/src/aiff/mod.rs - aiff::AiffDemuxer (line 142) ... ok` (line 6366).
— verified by `rg -n "pcm_s24be" crates/media-types/src/ids.rs` + reading `CodecDescriptor::new` (ids.rs:582): descriptor strings are opaque, non-empty, no control chars — all AIFF codec names accepted; no registry-lookup failure path.

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/aiff/mod.rs:337-348
Code:
```rust
                    let bytes = self.reader.read_prefix_at(want, payload_start)?;
                    let floor = if aifc {
                        chunks::COMM_AIFC_MIN_BYTES
                    } else {
                        chunks::COMM_MIN_BYTES
                    };
                    if bytes.len() < floor {
                        return Err(error::truncated(
                            payload_start + floor as u64,
                            payload_start + bytes.len() as u64,
                        ));
                    }
```
What is wrong: An AIFC COMM whose *declared* ckSize is 18–21 bytes (the size check at
line 331 only rejects `< 18`) passes `want = size`, reads exactly `size` bytes from an
intact stream, then fails `bytes.len() < 22` and mints `error::truncated` — kind
`Corrupt::Truncated`, `expected = payload_start + 22`. The stream was not cut: the
chunk declared itself short and every declared byte was delivered. `expected` is a
number the file never promised. `chunks::parse_comm`'s own documented contract
(chunks.rs:228, "an AIFC tail too short for its compression type") classifies this
exact shape as `bad_structure` (`Corrupt::InvalidStructure`), which this pre-empting
branch makes unreachable for sizes 18–21.
Why it matters: Typed error codes are the engine's recovery/diagnostic vocabulary
(the recovery-matrix flow keys on error kinds). A structural defect reported as
truncation misdirects repair ("re-feed more bytes") and reports a fabricated
expected/found pair to the user.
What correct looks like: Distinguish "extent clamped the read" (`bytes.len() <
want`, real truncation) from "declared size itself below floor" (`size < floor`,
`error::bad_structure(pos)`) — then let `parse_comm` handle the tail check.
Confidence: HIGH — Verification: read (lines 331–348 + chunks.rs:236-244, 266-273 traced; `parse_comm` floor logic confirmed)

### F-002 [S3]
Where: crates/demux/src/aiff/mod.rs:357-365
Code:
```rust
                    let want = (size.min(8) as usize)
                        .min((extent_end.saturating_sub(payload_start)) as usize);
                    let bytes = self.reader.read_prefix_at(want, payload_start)?;
                    if bytes.len() < 8 {
                        return Err(error::truncated(
                            payload_start + 8,
                            payload_start + bytes.len() as u64,
                        ));
                    }
```
What is wrong: Same pattern for SSND. A chunk that declares ckSize < 8 (e.g. 4) is
complete per its own header; `want = size.min(8) = 4`, the read delivers all 4 bytes,
and the file reports `truncated(expected = payload_start + 8, found = payload_start +
4)` — again a fabricated `expected` and the wrong `Corrupt` kind for a structurally
degenerate chunk. (`chunks::parse_ssnd`'s contract, chunks.rs:335, calls this
`bad_structure`.) Only the sub-case where the extent clamp (not the declaration) cut
the read is genuine truncation, and this branch cannot tell the two apart.
Why it matters: Identical taxonomy damage as F-001 at the other mandatory chunk; both
are reachable from one hand-edited byte in any AIFF file.
What correct looks like: `if size < 8 { bad_structure(pos) } else if bytes.len() < 8
{ truncated(...) }` — or delegate the floor check to `parse_ssnd`.
Confidence: HIGH — Verification: read (lines 353-377 + chunks.rs:336-341 traced)

### F-003 [S4]
Where: crates/demux/src/aiff/mod.rs:463-464 (also 497, 579, 607)
Code:
```rust
        let codec = CodecDescriptor::new(codec_of(&comm)).map_err(|_| error::bad_comm(0))?;
        let base = TimeBase::new(1, comm.sample_rate).map_err(|_| error::bad_comm(0))?;
```
What is wrong: Four fallback errors mint `bad_comm(0)`/`bad_structure(0)` with a
hard-coded offset 0 that corresponds to no structure. The paths are unreachable today
(`CodecDescriptor::new` only rejects empty/control-char strings; `TimeBase::new`
rejects den 0 and `parse_comm` enforces rate ≥ 1 — line 496 even says "Structurally
unreachable"), so this is dead-path hygiene, but if any invariant ever loosens the
error would point the user at byte 0 of the file.
Why it matters: Position-carrying errors that lie are worse than no position; the
`0` defeats the `CorruptDetail::InvalidStructure { offset }` field's purpose.
What correct looks like: Propagate the real offset (`pos`/chunk position) into these
fallbacks, or make the invariant un-representable (a `CommChunk` that cannot hold an
unusable rate) and delete the fallbacks.
Confidence: HIGH — Verification: read (TimeBase::new zero check at crates/media-types/src/time.rs:77-82; CodecDescriptor::new at ids.rs:582-592)

### F-004 [S4]
Where: crates/demux/src/aiff/mod.rs:570-572
Code:
```rust
        if let Some(sticky) = &self.sticky {
            return Err(sticky.mint());
        }
```
What is wrong: After one mid-stream fatal error (e.g. SSND truncated at the tail),
`locate` refuses **every** later `seek`/`reanchor` forever — including re-anchoring to
byte-0 region data that is intact and previously delivered. The WAV twin does the
same (wav/mod.rs:948-950) and its comment says "the walk frozen at the damage" —
so this is a house-wide design decision, not an accident; but for a seekable CBR
format, replay-before-damage is arithmetically exact and the refusal is stricter than
the container requires.
Why it matters: A user seeking back after an end-truncation error on a long file gets
a typed refusal where VLC-style players resume.
What correct looks like: `reanchor` (not `seek`) could clear the sticky error when
the landing byte is strictly before the damage position (the demuxer has `walk` and
`effective_end` to decide).
Confidence: MEDIUM — Verification: read (twin comparison `rg -n "sticky" crates/demux/src/wav/mod.rs`); severity tempered because the engine transitions Error→Idle and reopens rather than re-anchoring in-session (baseline INDEX.md open-session transcripts)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `probe` | 86-94 | REAL | — | 12-byte head check; short headers → false. Mirrored by media-probe phase5.rs:76 (drift risk, see §7). |
| `SoundRegion` (struct) | 97-108 | REAL | — | data-only; four absolute positions |
| `StickyShape::of` / `mint` | 118-130 | REAL | — | clone-and-replay of error shape; no source/position loss beyond kind+context |
| `AiffDemuxer::new` | 195-213 | REAL | — | rejects unknown-length/unseekable/empty via `SourceReader::new` (riff.rs:210-236); pool 2×64 KiB |
| `comm` | 217-220 | REAL | — | accessor |
| `comm_chunk_bytes` | 222-227 | REAL | — | verbatim ≤4 KiB prefix (doc'd cap); engine extra_data seam |
| `ssnd_offset_block` | 229-233 | REAL | — | accessor |
| `damage_total` | 236-240 | REAL | — | sums DamageLog.total (saturating, avi/header.rs:397-400) |
| `pool_bytes` | 242-247 | REAL | — | D-1 observable; ≤ 2×64 KiB by construction |
| `frames` | 249-256 | REAL | — | division by `frame_size` ≥ 1 (parse invariants); pre-start returns None |
| `fail` | 258-262 | REAL | — | sticky + return; all step() error paths route through it |
| `surface_damage` | 264-270 | REAL | — | `std::mem::take` — events surface exactly once |
| `emit` | 272-280 | REAL | — | damage-record-first ordering via pending queue |
| `parse_container` | 282-484 | REAL | F-001/F-002 | walk `pos+8 <= extent_end` monotonic; `saturating_add` throughout; duplicate COMM/SSND damage-noted; frames==0 → typed `no_ssnd`; F-001/F-002 sites |
| `step` | 486-562 | REAL | — | `(available / frame)` safe (frame ≥ 1); `MAX_PACKET_BYTES / frame` ≥ 1; short-read path damage-noted + prefix delivered; PTS = frames_emitted ticks on 1/rate base — exact |
| `locate` | 564-600 | REAL | F-004 | sticky refusal (F-004); `total_frames > 0` guard dead post-start (frames==0 rejected in parse) — defensive, harmless; end-landing allowed by design |
| `landing_span` | 602-615 | REAL | — | end stamp = total frames; TimeBase fallback unreachable |
| `frame_size` | 618-626 | REAL | — | channels(1..=64) × bytes(1..=8) saturating — never 0 post-parse |
| `codec_of` | 628-638 | REAL | — | classic path: sample_size ∈ {16,24,32} enforced by parse → `_` arm is the 4-byte case, correct; unreachable for width 1 |
| `Demuxer::start` | 641-660 | REAL | — | caches tracks; sticky on failure; second call returns clone |
| `Demuxer::next_event` | 662-685 | REAL | — | sticky first, pending next, idempotent EOS after `eos` flag |
| `Demuxer::seek` | 687-690 | REAL | — | dry-run per plugin-api contract (demux.rs:73 "Does **not** move the read position") — correctly non-committing |
| `Demuxer::seekability` | 692-697 | REAL | — | `Fast { indexed: false }` only post-start, clean |
| `Demuxer::reanchor` | 699-706 | REAL | — | commits walk/frames_emitted, clears pending+eos; does not clear sticky (F-004) |

## 5. Test honesty
Tests for this file live in `aiff/tests.rs` (audited separately, file 3 of this
agent), `aiff/chunks.rs` inline, `aiff/fuzz.rs`, and `tests/wav_aiff_fixtures.rs`.
Baseline evidence all green: nofailfast log lines 1523-1552 (28 tests incl.
`declared_ssnd_past_actual_delivers_prefix_then_truncated`, `truncations_at_every_header_site_survive`),
lines 2223-2234 (`every_aiff_fixture_matches_the_ffmpeg_truth` — differential vs
ffmpeg raw dumps, the strongest evidence here), line 6366 (doctest). Mutation
reasoning (no edits made): flipping `pad_for(size)` to `0` in the walk would
desynchronize every odd-payload chunk and fail `odd_comm_payload_takes_its_pad`
(baseline line 1544); breaking the PTS ladder fails `pts_ladder_and_span_count_sample_frames`
(line 1546). The suite would catch real breakage.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "COMM's numSampleFrames is the declared count; the byte extent is authority — a disagreement is damage-noted" | module doc lines 26-28 | REAL | code lines 440-456 (`comm-frames-past-ssnd` / `comm-frames-short-of-ssnd` damage notes); test `comm_declarations_disagreeing_with_ssnd_are_damage` baseline:1536 |
| "an SSND cut by the end of the file delivers its whole-frame prefix and then fails typed (error::truncated)" | module doc lines 29-32 | REAL | code lines 499-505 + test baseline:1537 |
| "denormals, the Apple pseudo-zero, infinities, NaNs, unnormals and fractional rates are typed structural refusals" | module doc lines 33-37 | REAL | chunks.rs:171-186 + test `extended80_rejects_non_normalized_shapes` baseline:1526 |
| "Bounded memory … 2 × 64 KiB, one copy per payload" | module doc lines 48-52 | REAL | `PayloadSlots::new(2, MAX_PACKET_BYTES)` line 211; `zero_alloc_read_chain` 10/10 green in baseline (log lines 2229-2242 region) |
| "Seeking is arithmetic (CBR): the landing is the exact sample frame" | module doc lines 46-47 | REAL | locate lines 587-599 exact integer math; test baseline:1547 |
| (implied) errors classify structural vs truncation correctly | chunks.rs:222-235 parse_comm doc | FALSE (at 2 sites) | F-001/F-002: mod.rs pre-empts with the wrong kind |

## 7. Dead / duplicated / unreachable code
- `locate` line 595 `total_frames > 0` guard: dead after a successful `start`
  (parse_container rejects frames==0 at lines 433-438). Harmless defense.
- `error::payload_too_large` (aiff/error.rs:141-150) has **no minting site** in
  mod.rs — its own doc admits "Vocabulary without a minting site today". Dead
  vocabulary (belongs to error.rs's audit; noted here because the doc at mod.rs:74-76
  references the ceiling it would guard).
- Probe logic duplicated across crates: `media-probe/src/signature/phase5.rs:76`
  "mirrors `demux::aiff::probe`" — two independent FORM/AIFF detections that can
  drift (probe accepts, demuxer refuses → confusing `not-aiff` after a positive
  probe). Cross-ref for the media-probe wave.

## 8. Dependencies & flags
Module uses only `media_types`, `plugin_api`, `std`, and sibling `avi::riff`/`avi::header`
primitives (crate-internal reuse, no layering violation — demux::aiff → demux::avi is
within-crate). No feature flags gate this file; no `unsafe`; no unwrap/expect outside
doc/test code (grep of read source). `#[cfg(test)] mod tests` correctly gated.

## 9. Open questions for the next agent
- Does the engine's seek flow ever call `reanchor` after a mid-stream error (F-004),
  or does the Error→Idle transition always rebuild the demuxer? Check
  playback-engine session code.
- media-probe phase5 mirror vs this `probe`: any input where they disagree (e.g.
  12-byte exact boundary)? Owner: wave auditing `crates/media-probe/src/signature/`.
- The differential fixture suite pins ffmpeg-produced files only; no fixture with a
  declared-short COMM/SSND exists — F-001/F-002 are untested paths (fuzz survives
  them, but nothing pins the error code).
