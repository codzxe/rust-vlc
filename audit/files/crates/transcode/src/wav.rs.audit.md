# AUDIT: crates/transcode/src/wav.rs
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-23 |
| Lines / functions | 539 / 6 production fns + 8 tests (3 test sink doubles) |
| File verdict | UNREACHABLE (real, high-quality WAV writer — the transcode crate has no dependents in the shipped product) |
| Reachable from shipped entry point? | NO — only caller is crates/transcode/src/runner.rs:590,904; repo-wide grep shows NO Cargo.toml depends on `transcode` (neither apps/cli, apps/desktop, nor any other crate) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p transcode --lib wav::` -> 8 passed / 0 failed (exit 0); grep for dependents; `sed -n` quote verification |

## 1. One-paragraph verdict
This is the best-code file in my assignment: a clean-room 44-byte RIFF/WAVE writer with lazy header emission, whole-frame discipline enforced before any byte moves, a typed (never wrapped) u32 RIFF size ceiling proven end-to-end with a discarding sink that writes until the refusal fires, byte-exact header assertions for all six PCM sample kinds, and typed sink-error folding that keeps the OS error in the source chain. All 8 tests pass under my run. Its only defect is that nothing shipped can ever execute it: the transcode crate is a library island — zero Cargo.toml dependents — so `lumen` users have no convert command, no UI dialog (matrix honestly says `ui-dialog-convert` = not_started), and this writer is dead code awaiting a 7.E3 host.

## 2. What actually works (verified)
— verified by `cargo test -p transcode --lib wav::` -> `test result: ok. 8 passed; 0 failed` — including `riff_ceiling_is_typed_not_wrapped` (writes 8 MiB chunks through a DiscardSink until `transcode.wav.size-overflow` fires, asserts written within one chunk of u32::MAX−36) and `header_is_the_canonical_44_bytes_per_sample_kind` (all 6 PcmSample kinds, every header field byte-exact).
— verified by grep: `WavPcmWriter` call sites = runner.rs:590, 904 only; no manifest in the repo lists `transcode` as a dependency.

## 3. Findings
F-001 [S3]
Where: crates/transcode/src/wav.rs:82 (pub(crate) struct) + repo manifests
Code:
```
pub(crate) struct WavPcmWriter {
    out: Box<dyn WriteSeek>,
```
What is wrong: not a code defect — a reachability one. The whole WAV encode lane (writer → runner.rs:590 `WavPcmWriter::new(output, facts.sample, facts.channels, facts.rate)` → transcode job graph) is reachable only from the transcode crate's own tests. No shipped binary (lumen-cli, desktop) or any other crate depends on `transcode`. The matrix is honest about it (`ui-dialog-convert` status not_started, evidence empty — docs/feature-matrix.toml:2913-2922), so this is dead-but-labeled code, not fraud. Blast radius: none for users; a whole card's (7.E1) worth of tested code sits unwired.
Confidence: HIGH — Verification: ran (manifest grep + call-site grep).
F-002 [S4]
Where: crates/transcode/src/wav.rs:230-232
Code:
```
        header[28..32].copy_from_slice(&(byte_rate as u32).to_le_bytes());
        header[32..34].copy_from_slice(&(block_align as u16).to_le_bytes());
        header[34..36].copy_from_slice(&(width as u16 * 8).to_le_bytes());
```
What is wrong: silent truncating `as` casts whose safety is an invariant established in `new()` (byte_rate ≤ u32::MAX checked at 117-120; block_align ≤ 64×8 = 512 < u16::MAX via MAX_CHANNELS = 64, demux/src/wav/format.rs:41). Correct today, but a refactor that calls `write_header` without the constructor (e.g. reconfiguring rate) truncates silently. A debug_assert or re-validation would pin it.
Confidence: HIGH — Verification: read (both bounds recomputed).
F-003 [S4]
Where: crates/transcode/src/wav.rs:141-149
Code:
```
        self.ensure_room(payload.len() as u64)?;
        if !self.header_written {
            self.write_header()?;
            self.header_written = true;
        }
        if payload.is_empty() {
            return Ok(0);
        }
```
What is wrong: an empty first `write_pcm(&[])` emits the 44-byte header even though no payload ever arrived — the doc says "written lazily, on the first payload write" (L30-31). Observable only as a 44-byte-earlier flush; `finalize` produces the identical file. Cosmetic contract wording.
Confidence: HIGH — Verification: read (control-flow order).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| WavPcmWriter::new | 104-129 | REAL | — | Validates channels 1..=64 (demux ceiling reused), rate≠0, byte_rate ≤ u32::MAX — typed refusals |
| WavPcmWriter::write_pcm | 141-153 | REAL | S4 | Guard-before-write ordering correct (torn tail refused before header); F-003 empty-write nuance |
| WavPcmWriter::ensure_room | 161-173 | REAL | — | Whole-frame check + saturating ceiling; directly testable as designed |
| WavPcmWriter::finalize | 185-205 | REAL | — | try_from (never wraps), seek-patch both size fields, flush, consumption-once; stale-size-on-failure documented |
| WavPcmWriter::write_header | 214-237 | REAL | S4 | F-002 invariant-dependent casts |
| write_all / seek (frees) | 241-248 | REAL | — | Typed error folds |
| tests (8) | 350-538 | REAL | — | See §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| header_is_the_canonical_44_bytes_per_sample_kind | YES | Every header field, 6 sample kinds, byte-exact incl. tag 1 vs 3 |
| lazy_header_means_empty_jobs_write_only_the_header | YES | 44 bytes, sizes 0/36 |
| sizes_patch_to_the_truth_about_what_was_written | YES | Two-write payload order preserved, both size fields exact |
| torn_tails_are_refused_before_any_byte_moves | YES | Proves the header is still unwritten after the refusal (sizes 0, payload empty) |
| fact_validation_refuses_the_impossible_headers | YES | 4 typed refusal codes asserted |
| riff_ceiling_is_typed_not_wrapped | YES | End-to-end DiscardSink loop until refusal; written bounded within one chunk of the ceiling; also exact-at-ceiling legal |
| sink_failures_fold_typed_with_the_os_error_kept | YES | Code + source-chain presence |
| seek_and_write_paths_agree_on_position_semantics | YES | Start-only seek sink still lands both patches |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "canonical 44-byte header, then payload bytes verbatim" | wav.rs:18-25 | REAL | Header test, byte-exact |
| "header is written lazily, on the first payload write" | wav.rs:30-31 | REAL (edge) | F-003: an empty first write also triggers it |
| "rate × block_align must fit the u32 byte_rate" | wav.rs:42-43 | REAL | new() 117-120 + refusal test |
| "total data bytes must stay addressable by a classic RIFF u32 — u32::MAX − 36" | wav.rs:44-46 | REAL | MAX_DATA_BYTES 73 + ceiling test |
| "every payload must be a whole number of sample frames" | wav.rs:48-51 | REAL | ensure_room 164-167 + torn-tail test |
| "The writer holds no payload buffer at all" | wav.rs:37-39 | REAL | Struct fields: no Vec payload (82-92) |

## 7. Dead / duplicated / unreachable code
- Whole file unreachable from shipped entry points (F-001); `data_bytes()` getter is cfg(test)-only (correctly scoped).

## 8. Dependencies & flags
- No feature gates. Deps: demux (PcmSample, MAX_CHANNELS — one accepted-set reuse), media-types, crate::graph::WriteSeek, crate::error. Std-only per crate header.

## 9. Open questions for the next agent
- convert.rs and cancel.rs (next in my queue) share this crate's island status — confirm no 7.E3 host card landed since.
- If a host lands, `WriteSeek` sinks need a real-file adapter check (runner.rs builds them — verify seek-back-on-file semantics there).
