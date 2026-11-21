# AUDIT: crates/subtitles/src/vtt/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A03 |
| Date | 2026-11-21 |
| Lines / functions | 63 / 1 harness + 2 tests |
| File verdict | REAL-UNPROVEN (harness real + deterministically executed; the cargo-fuzz shim EXISTS but no libFuzzer campaign has ever run — all "fuzz" evidence is a seeded-LCG deterministic suite) |
| Reachable from shipped entry point? | NO — crate unshipped. Callers: in-crate tests (baseline green) + `fuzz/fuzz_targets/fuzz_sub_vtt.rs:19` (the shim, fuzz-package only, compiles on stable, never run as a campaign per its own comment). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | read fuzz/fuzz_targets/fuzz_sub_vtt.rs (shim + status comment) · read tests/vtt_fuzz.rs (the real fuzz-lite) · `rg -n "MAX_\|cap\|damage" crates/subtitles/src/vtt` (no ledger count cap) · read vtt/mod.rs:132-259 (parse + damage pushes) · baseline: `orchrun-test-workspace-nofailfast.log.txt:5516-5517` (vtt::fuzz 2/2 ok), `:5683-5690` (vtt_fuzz 3/3 ok) · `sed -n` on quoted ranges |

## 1. One-paragraph verdict
The smallest honest file of the six: a three-line harness (`from_utf8_lossy` + `parse`, result discarded) whose contract is stated as absence-of-panic/hang/unbounded-allocation and nothing else, wired to an existing cargo-fuzz shim — the only one of the crate's five format harnesses (srt/ass/vtt/cc + dvb/vobsub/pgs siblings) that actually has its shim in the tree. Two doc defects only: the "damage ledger is capped" bounds claim is false as written (VTT has no ledger-count cap — only a 16-byte per-event name cap; the real bound is O(input) lines), and the wiring-status note elides that the campaign has never once executed — every "fuzz" number in evidence strings (matrix row's "7003 fuzz inputs zero panics") is the deterministic LCG/truncation suite count, which is a survival count, not fuzzing. The deterministic suite behind it is genuinely strong: 7,000 mutation rounds with model-invariant checks and a double-parse determinism assert, plus a 199 KB unterminated-tag hang probe.

## 2. What actually works (verified)
— `vtt::fuzz::tests` 2/2 ok (`orchrun-test-workspace-nofailfast.log.txt:5516-5517`); `tests/vtt_fuzz.rs` 3/3 ok (`:5683-5690`) — the deterministic survival suite executed in baseline: 4,000 seeded-mutation rounds (flip/truncate/insert/splice, header-biased) + 3,000 random buffers, accepted documents checked for interval order and percent range invariants, double-parse determinism asserted, `unreachable!` canary on any new `VttError` variant, 160 KB + 199 KB hostile documents parsed.
— Harness bounds: `String::from_utf8_lossy` borrows on valid UTF-8 (no alloc) and allocates once O(input) otherwise; `parse` is single-pass over a line Vec (mod.rs:135-138) — output ≤ O(input) holds (cues ≤ blocks ≤ lines; damage events ≤ lines).

## 3. Findings

### F-001 [S4]
Where: crates/subtitles/src/vtt/fuzz.rs:22-23
Code:
```rust
//! Parse: one pass, output ≤ O(input); the damage ledger is capped
//! and the cue count is bounded by the line count.
```
What is wrong: The VTT damage ledger has NO count cap. Every malformation path pushes straight into `doc.damage` (`pub damage: Vec<DamageEvent>`, vtt/mod.rs:99; pushes at mod.rs:186-237 and settings.rs:59-166) with no length gate — a document with N bad lines yields N events. The only cap in the VTT damage layer is per-event (16-byte captured name, damage.rs:74 "boundary to ≤16 bytes so hostile inputs cannot balloon the ledger" — that bounds event SIZE, not COUNT).
Why it matters: "capped" reads as a fixed ceiling like cc's `MAX_DAMAGE_EVENTS = 256`; the actual ceiling is input-proportional. The overall O(input) contract still holds, so this is a bounds-doc inaccuracy, not an unbounded-allocation bug.
What correct looks like: "the damage ledger grows ≤ one event per input line (each event ≤ a fixed size); the cue count is bounded by the line count."
Confidence: HIGH — Verification: read of fuzz.rs, mod.rs, settings.rs, damage.rs; rg for any MAX/cap in src/vtt (none).

### F-002 [S4]
Where: crates/subtitles/src/vtt/fuzz.rs:13-18 (vs fuzz/fuzz_targets/fuzz_sub_vtt.rs:7-10, matrix row 927)
Code:
```rust
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_sub_vtt.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's VTT suites.
```
What is wrong: The shim exists but has never been executed as a fuzzer — its own comment: "building the campaign needs the Lead's nightly wiring … until then this package compiles on stable (`cargo check` in fuzz/)". Baseline #18 (`scripts/fuzz_registry_check.sh`, PASS, 35 targets) checks REGISTRY CONSISTENCY, not campaign execution; no libFuzzer run exists anywhere in the baseline. The matrix evidence string "7003 fuzz inputs zero panics" (feature-matrix.toml:927) is the deterministic suite's input count (4,000 + 3,000 + 3) presented under the word "fuzz".
Why it matters: Evidence-string inflation: a reader of the row concludes a fuzzing campaign ran. The deterministic suite is good, but it cannot find coverage-guided discoveries (e.g. state reachable only through specific byte sequences). The in-crate doc is half-honest ("meanwhile proven on stable by the deterministic sweeps") — the inflation lives in the matrix evidence string.
What correct looks like: Matrix evidence says "7,003 deterministic LCG/truncation inputs, zero panics; libFuzzer campaign not yet run (nightly wiring pending)".
Confidence: HIGH — Verification: read of both files; baseline INDEX #18; count arithmetic on tests/vtt_fuzz.rs.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_sub_vtt` | 28-31 | REAL | — | Lossy decode + parse, result discarded per contract; no panic path (traced); O(input) allocation only |
| test `empty_and_marker_inputs_survive` | 37-42 | REAL | — | 3 assert-free smoke calls — matches the absence-of-panic contract exactly; would fail only on panic/hang (which is the point) |
| test `hostile_wide_inputs_survive` | 44-62 | REAL | — | 0xff/0x00 floods + 64 hostile timing lines; assert-free by contract; the SEMANTIC checks live in tests/vtt_fuzz.rs |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| in-crate 2 tests | Only on panic/hang — by stated contract | Contract is absence-of-panic; no semantic asserts possible (result discarded) |
| vtt_fuzz `mutated_documents_never_panic` | YES (panics, invariant violations, nondeterminism, ledger-line-number bugs) | 4,000 rounds, invariants, double-parse equality, accepted/rejected > 100 both ways |
| vtt_fuzz `random_buffers_never_panic` | YES (panics, invariants) | 3,000 random buffers with invariant checks |
| vtt_fuzz `large_document_under_200kb_is_survivable` | YES (hang, payload balloon) | 160 KB clean doc (damage==0, cues>0) + 199 KB unterminated tag (total_len==0 pinned) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The cargo-fuzz target is fuzz/fuzz_targets/fuzz_sub_vtt.rs" | fuzz.rs:15-16 | REAL (file) / campaign never run | Shim exists (read); no libFuzzer execution evidence anywhere |
| "the damage ledger is capped" | fuzz.rs:22-23 | FALSE | F-001: no count cap; per-event 16-byte name cap only |
| "output ≤ O(input)" | fuzz.rs:22 | REAL | Traced: one line Vec, one pass, per-line bounded work |
| "7003 fuzz inputs zero panics" | matrix row subtitle-vtt (feature-matrix.toml:927) | SHALLOW | Deterministic LCG count, not a fuzzing campaign (F-002) |
| "harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:7-8 | REAL | Body matches exactly |

## 7. Dead / duplicated / unreachable code
- None in-file. The shim (`fuzz_sub_vtt.rs`) is dormant-but-compiled infrastructure — the only sub_* shim in the fuzz package that is NOT dead weight (cc/pgs/dvb/vobsub harnesses have no shim at all; cross-ref the cc/fuzz.rs audit).

## 8. Dependencies & flags
- std-only; no features, no unsafe. The lossy-UTF-8 conversion means the harness can never exercise invalid-UTF-8 paths INSIDE the parser (parser input is always valid UTF-8 by construction) — that is the correct seam (parse takes &str).

## 9. Open questions for the next agent
- vtt/mod.rs owner: should `VttDocument.damage` get a real count cap (cc precedent, `MAX_DAMAGE_EVENTS`)? If yes, F-001's doc sentence becomes true instead of being fixed.
- Whoever owns the matrix evidence strings: normalize "N fuzz inputs" to distinguish LCG-deterministic from libFuzzer campaign counts across all subtitle rows (srt/ass rows use the same vocabulary).
