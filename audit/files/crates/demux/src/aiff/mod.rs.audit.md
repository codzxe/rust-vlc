# AUDIT: crates/demux/src/aiff/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-02-14 |
| Lines / functions | 707 / 22 (incl. trait impl + 2 free fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen open` → apps/cli/src/host.rs:904 → demux_host.rs:378 `open_source_with` → probe (`AiffSignature` registered at crates/media-probe/src/signature/mod.rs:135) → containers/mod.rs:53 `"aiff" => aiff::open` → containers/aiff.rs:56 `AiffDemuxer::new`. Desktop embeds lumen-cli → same chain. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `target/debug/deps/demux-036dd705535b2fb3 aiff::` → 32 passed, 0 failed (exit 0); `~/.cargo/bin/cargo test -p demux --doc aiff` → 1 passed (exit 0) |

## 1. One-paragraph verdict
A faithful twin of the verified WAV demuxer with the AIFF-correct differences: big-endian walk,
80-bit-extended sample rate with normalized/finite/integral-only acceptance, AIFC compression
vocabulary (`NONE`/`sowt`/`fl32`/`fl64`/`raw `) with honest typed refusals for classic 8-bit
signed and `ulaw`/`ima4`, SSND offset/blockSize handling with the offset-past-end case typed
(tested). Bounds hold: channels 1..=64 and widths ≤ 8 keep `frame_size` ≤ 512, so
`MAX_PACKET_BYTES / frame ≥ 1` — no zero-progress packet loop; every arithmetic op on parsed
sizes saturates (no ASF-style overflow). The one carried-over defect from the WAV twin is the
short-read path dropping delivered whole frames without emitting them. No S0–S2 found; executed
32 lib tests + the doctest, all pass.

## 2. What actually works (verified)
- `target/debug/deps/demux-036dd705535b3 aiff::` → `test result: ok. 32 passed; 0 failed`
  (exit 0) — includes `big_endian_samples_arrive_verbatim`,
  `declared_ssnd_past_actual_delivers_prefix_then_truncated`,
  `zero_or_offset_past_ssnd_is_no_ssnd`, `non_normalizable_rate_is_structural`,
  `aifc_compression_vocabulary_demuxes`, seek reanchor, probe.
- `~/.cargo/bin/cargo test -p demux --doc aiff` → `test crates/demux/src/aiff/mod.rs - aiff::AiffDemuxer (line 142) ... ok`.
- `frame_size` bound re-derived: `parse_comm` refuses channels 0 and > 64 (chunks.rs:259-264) and
  resolves widths to ≤ 8 bytes/sample (chunks.rs:298-327) → frame ≤ 64×8 = 512 < 65536, so
  `want_frames = min(whole/frame, MAX_PACKET_BYTES/frame)` ≥ 1 (mod.rs:515) — the empty-packet
  infinite loop is structurally impossible here.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/aiff/mod.rs:525-537
Code:
```rust
        let delivered = payload.len() as u64;
        if delivered < want_bytes as u64 {
            let whole = (delivered / frame) * frame;
            self.damage
                .note(want_bytes as u64 - whole, "payload-read-short");
            self.surface_damage();
            if whole > 0 {
                self.walk += whole;
                self.frames_emitted += whole / frame;
            } else {
                self.walk = region.effective_end;
            }
            return Ok(None);
```
What is wrong: identical to wav/mod.rs F-001 — on a short read the delivered whole frames are
dropped (no packet emitted) while the damage note counts only `want_bytes - whole`; the
delivered-and-discarded `whole` bytes vanish unaccounted. Trigger needs a source that shrank
below the clamped extent mid-walk. What correct looks like: emit a packet of `whole` bytes or
note them as lost. Confidence: HIGH — Verification: read (no test drives a shrinking source).

F-002 [S4]
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
What is wrong: an SSND chunk with declared size < 8 (e.g. 0) on a physically-complete file is
reported as `error::truncated` — "the stream ended" — when nothing was cut; the chunk is
structurally too small for its own offset/blockSize pair (`chunks::parse_ssnd` classifies the
same shape as `bad_structure`, but is unreachable for size < 8 because `want` was clamped).
Misleading error taxonomy. What correct looks like: bad_structure for `size < 8`.
Confidence: HIGH — Verification: read.

F-003 [S4] (adjacent file — the rate parser this demuxer depends on)
Where: crates/demux/src/aiff/chunks.rs:184 (with mod.rs:33-37's claim)
What is wrong: `extended80_to_f64` computes `sign * (mantissa as f64) * powi(...)` — the u64
mantissa is rounded through f64 (53-bit precision). A hostile rate whose true value is
non-integral but whose rounded f64 image is integral (mantissa precision > 53 bits, e.g.
2^63+1 scaled down) is ACCEPTED, violating "fractional rates are typed structural refusals (no
guessed rates, ever)" (mod.rs:35-37). The accepted rate differs from the encoded value by a
sub-integer amount → microscopic PTS drift on a pathological file. What correct looks like: do
the integrality check in integer math (mantissa low bits vs exponent) before touching f64.
Confidence: MEDIUM — Verification: read (arithmetic reasoning; no fixture exercises it).

F-004 [S4]
Where: crates/demux/src/aiff/mod.rs:86-94 (`probe`) and its WAV twin crates/demux/src/wav/mod.rs:139-147
What is wrong: dead-in-product duplication — `aiff::probe` and `wav::probe` have ZERO callers
outside their own modules (rg over apps/, crates/, tools/: only their own tests/fuzz); the real
probe is media-probe's separate reimplementation (phase5.rs:56: "mirrors `demux::wav::probe`").
Two implementations of the same magic bytes can drift (the mirror comment admits the coupling).
Confidence: HIGH — Verification: rg (empty result outside own modules).

F-005 [S4]
Where: crates/demux/src/aiff/chunks.rs:269-271
What is wrong: the comment claims the AIFC pstring name is "parsed for the skip math, not kept"
— no code parses it; the walk skips the whole COMM chunk positionally by declared size. False
comment on a non-action. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe | 86-94 | REAL | S4 | works; dead in product (F-004) |
| StickyShape::of/mint | 118-130 | REAL | — | wav twin |
| AiffDemuxer::new | 195-213 | REAL | — | rejects unknown-length/unseekable/empty |
| comm / comm_chunk_bytes / ssnd_offset_block / damage_total / pool_bytes / frames | 215-256 | REAL | — | accessors; frames None before start |
| fail / surface_damage / emit | 258-280 | REAL | — | damage-first order preserved |
| parse_container | 282-484 | REAL | S4 | SSND <8 misclassification (F-002); extent clamps + COMM/SSND checks sound |
| step | 486-562 | REAL | S3 | short-read drop (F-001); ladder/duration math verified |
| locate | 564-600 | REAL | — | exact-frame landing; out-of-range typed |
| landing_span | 602-615 | REAL | — | — |
| frame_size (free) | 618-626 | REAL | — | ≤ 512 bound verified via parse_comm rules |
| codec_of (free) | 628-638 | REAL | — | classic → pcm_s16/24/32be |
| Demuxer::start / next_event | 640-685 | REAL | — | idempotent; sticky; damage-before-EOS |
| Demuxer::seek / seekability / reanchor | 687-706 | REAL | — | compute-then-commit; Fast{indexed:false} honest for CBR |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| big_endian_samples_arrive_verbatim | YES | byte-exact BE payload assertion |
| declared_ssnd_past_actual_delivers_prefix_then_truncated | YES | packets-then-typed-truncated order |
| zero_or_offset_past_ssnd_is_no_ssnd | YES | covers the offset-past-end arithmetic |
| non_normalizable_rate_is_structural | YES | hand-built 80-bit shapes refused typed |
| aifc_compression_vocabulary_demuxes | YES | per-codec descriptors + payload bytes |
| comm_declarations_disagreeing_with_ssnd_are_damage | YES | damage reasons pinned both directions |
| All 20 tests | YES (spot-checked 7) | exact codes/bytes; executed 32/32 (incl. chunks module tests) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "an SSND cut by the end of the file delivers its whole-frame prefix and then fails typed" | mod.rs:29-32 | REAL | code 499-501 + test at tests.rs:285 executed |
| "denormals, the Apple pseudo-zero, infinities, NaNs, unnormals and fractional rates are typed structural refusals (no guessed rates, ever)" | mod.rs:33-37 | REAL-in-practice / FALSE-in-corner | chunks.rs:174-185 refuses all listed shapes (tests pass); f64-rounding corner accepts a non-integral rate (F-003) |
| "Classic AIFF's 8-bit samples are signed — no registry vocabulary, the typed refusal" | mod.rs:42-43 | REAL | chunks.rs:305-311 + test `unsupported_depths_are_typed` executed |
| "COMM reads are capped (chunks::MAX_COMM_BYTES)" | mod.rs:50-51 | REAL | mod.rs:334-337, cap 4096 |

## 7. Dead / duplicated / unreachable code
- `aiff::probe` — no production caller (F-004); exercised only by its own tests + fuzz.
- Nothing else dead; module is wired end-to-end (probe signature + registry arm + container host).

## 8. Dependencies & flags
- Reuses `crate::avi::riff::{SourceReader, PayloadSlots, FourCC, pad_for}` and
  `crate::avi::header::DamageLog` — same shared plumbing as WAV; no external crates; no
  `unsafe`; no panics/unwrap in non-test code (read through; house rule upheld).
- No feature gates; release-lane compiled (baseline).

## 9. Open questions for the next agent
- `containers/aiff.rs` (CLI host): does its harvest read `comm_chunk_bytes` for the PCM decoder
  config, and does the PCM decoder accept the BE vocabulary (`pcm_s16be` …) it is handed?
- The extended80 f64 corner (F-003) — if a hostile fixture family ever lands, add a
  >53-bit-mantissa non-integral rate vector.
