# AUDIT: crates/transcode/src/wav.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2026-11-21 |
| Lines / functions | 539 / 23 (10 production items: 4 consts, 1 struct, 5 methods, 2 free helpers; 13 test items: 2 sink doubles + 3 helpers + 8 tests) |
| File verdict | UNREACHABLE (real, heavily-tested RIFF/WAVE writer — but the crate is linked by no shipped host) |
| Reachable from shipped entry point? | NO. `WavPcmWriter` is `pub(crate)` with exactly two in-crate callers: `runner.rs:590` (WAV stream-copy plan) and `runner.rs:904` (decode path). `submit` (runner.rs) is the crate's only entry, and no app depends on `transcode` (proof in graph.rs audit F-001: no Cargo.toml edge, no source reference). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "WavPcmWriter" crates/transcode/src` → callers runner.rs:88,590,838,904 only; `rg -n "MAX_CHANNELS\|fn bytes_per_sample" -A8 crates/demux/src/wav/format.rs` → MAX_CHANNELS=64 (u16), widths 1/2/3/4/8 — invariants behind the `as` casts verified; `rg -n "finalize" crates/transcode/src/runner.rs` → returned sink always dropped (`let _ =` / error propagation); `sed -n` on quoted ranges; `rg -n "test wav::tests::" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → 8/8 ok at 5856-5863; `sed -n '5904,5917p'` same log → wav_roundtrip 5/5 ok incl. bit-exact decode→write→re-demux→re-decode |

## 1. One-paragraph verdict
This is the most rigorously defensive 240 lines of production code I have audited in this crate: every arithmetic ceiling (byte_rate past u32, payload past the classic RIFF `u32::MAX − 36` data ceiling) is checked with u64 math and refused typed — never wrapped; the two `as` casts in `write_header` are unreachable-truncation by construction (new() pre-validated; I verified MAX_CHANNELS=64 × width ≤ 8 = 512 < u16::MAX); the torn-tail check runs before the lazy header so a refused write moves zero bytes (pinned by test); and the ceiling test actually streams 8 MiB chunks through a discard sink until the typed refusal fires instead of pretending with arithmetic alone. The round-trip integration suite decodes fixtures, writes with this writer, re-demuxes and re-decodes bit-exactly — that is real, executed evidence. The only real problems: none of it is reachable from a shipped binary (the crate is an unwired phase-7 card), and `finalize` hands the sink back with the cursor parked at byte 44 without documenting that position contract.

## 2. What actually works (verified)
— `wav::tests` 8/8 green: orchrun-test-workspace-nofailfast.log.txt:5856-5863 (fact validation, canonical 44-byte header for all 6 sample kinds, lazy header, RIFF ceiling, seek/write semantics, sink failure fold, size patching, torn-tail refusal).
— End-to-end bit-exact round trip: same log 5904-5917 — `fixtures_roundtrip_decode_encode_redemux_redecode_bitexactly … ok` plus 4 more (5/5).
— Invariant verification for the casts: `demux::wav::format::MAX_CHANNELS = 64` (u16, format.rs:41) and `bytes_per_sample` ∈ {1,2,3,4,8} (format.rs:132-140) → `block_align ≤ 512` fits the `as u16` at wav.rs:231; `byte_rate ≤ u32::MAX` enforced at new (118-120) before the `as u32` at 230.
— Workspace clippy `-D warnings` clean (baseline #5/#6) — consistent with the file's zero unwrap/expect/panic in production paths (verified by reading).

## 3. Findings

### F-001 [S3] Dead lane: the writer ships in no binary (crate-level, cross-ref graph.rs F-001)
Where: crates/transcode/src/wav.rs:82-92
Code:
```rust
pub(crate) struct WavPcmWriter {
    out: Box<dyn WriteSeek>,
    /// The PCM representation the `fmt ` chunk declares.
    sample: PcmSample,
```
What is wrong: `pub(crate)` writer reachable only through `submit`/runner inside a crate that no app links (proof: graph.rs audit header — no `transcode` edge in any app Cargo.toml, no source reference in apps/).
Why it matters: a correctly-typed, bit-exactness-proven WAV encoder exists while the product exposes no record/convert path to invoke it; the matrix rows `playback-record`/`ui-dialog-convert` (not_started, empty evidence) are honest, but the code's existence invites docs to describe capability no host delivers (crate lib.rs "Honest capabilities (v1)" enumerates it unconditionally).
What correct looks like: wire a host command or crate-level banner marking the whole crate "unwired phase-7 card" (see graph.rs F-001).
Confidence: HIGH — Verification: ran rg caller census; read runner call sites.

### F-002 [S4] `finalize` returns the sink with an undocumented cursor position (44, not end-of-file)
Where: crates/transcode/src/wav.rs:200-204
Code:
```rust
        seek(&mut self.out, SeekFrom::Start(40))?;
        write_all(&mut self.out, &data_size.to_le_bytes())?;
        self.out.flush().map_err(sink_write_io)?;
        Ok(self.out)
```
What is wrong: after patching `data` size at offset 40, the sink's cursor sits at byte 44 — the START of the payload — and the doc (175-179) says only "hand the sink back". A host that writes to the returned box appends at offset 44, silently overwriting payload bytes.
Why it matters: the returned `Box<dyn WriteSeek>` invites further use by its type; today's only in-crate caller drops it (`let _ = writer.finalize()` — runner.rs:600,628,638; error path 631), so nothing breaks, but the API documents neither the position nor a "do not write" contract.
What correct looks like: either seek to `SeekFrom::End(0)` before returning, or a doc line "the sink is returned positioned after the `data` size field; do not write further".
Confidence: HIGH — Verification: read finalize + runner call sites (all four discard or propagate).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `TAG_PCM`/`TAG_IEEE_FLOAT`/`HEADER_BYTES`/`MAX_DATA_BYTES` consts | 64-73 | REAL | — | `u32::MAX − 36` derivation matches RIFF layout (12+8+16+8). |
| `struct WavPcmWriter` | 82-92 | REAL | S3 | F-001 (unreachable). 6 words of state, no payload buffer — streaming by construction. |
| `WavPcmWriter::new` | 104-129 | REAL | — | channels 1..=64 (demux ceiling reused — one accepted-set), rate≠0, byte_rate u64-checked; all refusals pinned by `fact_validation` test. |
| `WavPcmWriter::write_pcm` | 141-153 | REAL | — | Guard-before-header ordering makes refusals zero-byte (test-pinned). Note: an EMPTY first call writes the header although no payload byte moves (147-149 after 143-146) — deviates from the doc's "on the first payload write" phrasing; harmless (finalize result identical). Saturating add is belt-and-braces (ensure_room already bounded). |
| `WavPcmWriter::ensure_room` | 161-173 | REAL | — | Frame-size divisor provably nonzero (new validated channels≥1, width≥1); `is_multiple_of`; ceiling check on u64 — no wrap. Directly unit-tested without 4 GiB (test comment explains). |
| `WavPcmWriter::finalize` | 185-205 | REAL | S4 | Double-try_from defensive layer with typed overflow (unreachable via API — comment says so, arithmetic verified); F-002 cursor contract. |
| `data_bytes()` (cfg(test)) | 208-211 | REAL | — | Test-only accessor. |
| `WavPcmWriter::write_header` | 214-237 | REAL | — | Canonical layout verified field-by-field by test for all 6 sample kinds; casts safe by pre-validated invariants (verified MAX_CHANNELS/widths). |
| `fn write_all` / `fn seek` | 241-248 | REAL | — | Typed error folds, source-preserving (pinned by `sink_failures` test asserting `Error::source().is_some()`). |
| test `SharedCursor` (+sink/write/flush/seek) | 258-311 | REAL | — | Poison-recovering lock idiom; resize-before-copy prevents panics; pos semantics sound. |
| test `DiscardSink` | 316-334 | REAL | — | 4-GiB-less ceiling vehicle; seek returns pos (unused by that test). |
| test helpers `u16at`/`u32at`/`file_of` | 336-348 | REAL | — | Read-back through the Arc twin — honest seam. |
| test `header_is_the_canonical_44_bytes_per_sample_kind` | 350-384 | REAL | — | Asserts every header field × 6 sample kinds + payload identity; mutation killer. |
| test `lazy_header_means_empty_jobs_write_only_the_header` | 386-395 | REAL | — | 44-byte file, sizes 0/36. |
| test `sizes_patch_to_the_truth_about_what_was_written` | 397-411 | REAL | — | Two writes → data=30, riff=66, payload bytes verified in place. |
| test `torn_tails_are_refused_before_any_byte_moves` | 413-424 | REAL | — | 5-byte stereo-s16 payload → `transcode.wav.partial-frame`, file stays 44+0. |
| test `fact_validation_refuses_the_impossible_headers` | 426-467 | REAL | — | All four new() refusals incl. boundary MAX_CHANNELS+1 and the 9 MHz × 64ch × f64 byte-rate overflow. |
| test `riff_ceiling_is_typed_not_wrapped` | 469-502 | REAL | — | Direct guard call at u32::MAX (refused) and exactly-at-ceiling (legal), then a real 8 MiB-chunk streaming walk to the refusal with position accounting. Strong. |
| test `sink_failures_fold_typed_with_the_os_error_kept` | 504-524 | REAL | — | FailingSink → `transcode.sink.write-io` + source retained. |
| test `seek_and_write_paths_agree_on_position_semantics` | 526-539 | REAL | — | Start-only seeks land both patched fields. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| header_is_the_canonical_44_bytes_per_sample_kind | YES | Field-by-field equality on all 6 kinds — any wrong offset/tag/width fails. |
| lazy_header / sizes_patch / torn_tails / fact_validation / sink_failures / seek_and_write | YES | Each pins exact typed codes and exact byte layout; torn-tails also asserts zero side effects. |
| riff_ceiling_is_typed_not_wrapped | YES | Both the guard unit (boundary ±1) and the end-to-end streaming walk to refusal. |
| Integration: fixtures_roundtrip_decode_encode_redemux_redecode_bitexactly | YES | Bit-exactness across decode→write→demux→decode catches any serialization lie (byte order, header drift, size patching). |
All green in orchrun-test-workspace-nofailfast.log.txt:5856-5863 and 5904-5917.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "WAV PCM encode — a real RIFF/WAVE writer … conversions are identity … widenings only" | transcode lib.rs docs 109-115 | REAL (in-crate) | This file + convert.rs (W10 other agent); round-trip test bit-exact (log 5904-5917). Not shipped-reachable (F-001). |
| "rate × block_align must fit the 32-bit byte_rate; total data bytes must stay addressable by a classic RIFF u32" | wav.rs module docs 42-46 | REAL | new:116-120, ensure_room:168-171; tests fact_validation + riff_ceiling green. |
| "an empty job still finalizes to a structurally valid 44-byte WAV with data size zero" | module docs 31-33 | REAL | lazy_header test (44 bytes, sizes 0/36) green. |
| "WAV finalization rewrites the RIFF/data size fields, so a write-only pipe cannot host an honestly-sized file" | graph.rs:70-76 (cross-file claim I owed verification) | REAL | finalize:190-202 seeks to 4 and 40 — requires Seek; graph.rs `WriteSeek` bound justified. |
| matrix `playback-record` (record-to-WAV) status not_started | docs/feature-matrix.toml ~1633 | REAL (honest) | Code exists but unlinked — matrix correctly does not claim capability. |

## 7. Dead / duplicated / unreachable code
Unreachable from shipped hosts (F-001). Internally fully consumed (runner both plans). No duplication; no dead items (`data_bytes` is cfg(test) and used at 404).

## 8. Dependencies & flags
std-only + in-crate error/graph + `demux::wav::format::{MAX_CHANNELS, PcmSample}` (the L2 vocabulary reuse documented in Cargo.toml:24-25). No feature gates. Zero unsafe. No unwrap/expect in production code (the `.must()`/`unwrap_or_else(|p| p.into_inner())` forms live in tests only).

## 9. Open questions for the next agent
- runner.rs owner (W10 other agents): the returned-sink drop at 600/628/638 — confirm no future card (7.E3 ES sink) plans to reuse the box (F-002 would then bite).
- mp4mux.rs owner: same finalize-cursor question for `Mp4Remuxer::finalize` if it returns the sink.
