# AUDIT: crates/media-probe/tests/common/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-23 |
| Lines / functions | 47 / 3 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — tests/ support module. Its consumers run: fixtures_matrix.rs (8 tests, default lane, baseline cargo-test-workspace.txt:3670-3681 all ok) and probe_io.rs (feature-gated `#![cfg(feature = "contract")]`, 0 tests in the default lane, baseline 3684-3688; runs under CI's `cargo test --workspace --all-features`, ci-full.yml:169). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg` consumers (probe_io.rs, fixtures_matrix.rs); baseline sed 3660-3688 (fixtures_matrix 8 ok, probe_io 0 tests); `rg` Cargo.toml features + ci-full.yml:169 (`--all-features` present). |

## 1. One-paragraph verdict
Three small, real helpers doing exactly what the doc says: open through
`input::FileAccess` (the engine's real open seam, not a memory stand-in),
probe a file through the real access stack with the builtin registry, and
resolve fixtures via testkit. All three execute in the default lane through
fixtures_matrix.rs (baseline: 8/8 ok). One hygiene defect: the clippy allow
is attached to the wrong function.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:3670-3681: fixtures_matrix (uses
  `fixture` + `probe_file` → which calls `open_via_input`) runs 8 tests,
  all ok — every helper in this file executes.
- read: `open_via_input` propagates URI-construction failures typed (the
  `?` on `SourceUri::from_file_path`), panicking only on non-UTF-8 paths —
  matching its documented test contract.

## 3. Findings
F-001 [S4]
Where: crates/media-probe/tests/common/mod.rs:24-25, 34-41
Code:
```
#[allow(clippy::missing_panics_doc)]
pub fn open_via_input(path: &Path) -> Result<Box<dyn plugin_api::SourceStream>, EngineError> {
```
What is wrong: the allow sits on `open_via_input`, whose `# Panics` section
exists directly above (lines 21-23) — the lint cannot fire there. The
function that CAN panic without a Panics doc is `probe_file`
(`unreachable!("fixture must open ...")` at line 37), which carries neither
the allow nor the doc section. Misplaced attribute; either dead or hiding
the wrong gap.
Confidence: HIGH — Verification: read.

## 4. Per-function table (file < 100 LOC, included for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open_via_input | 25-29 | REAL | S4 | F-001 allow placement; typed URI propagation |
| probe_file | 34-41 | REAL | — | real registry + probe_stream; undocumented panic path |
| fixture | 45-47 | REAL | — | testkit delegate |

## 5. Test honesty
Not a test file. Its honesty property: does it open through the REAL access
stack as claimed? Yes — `input::FileAccess::new().open(&uri)`, no in-memory
shortcut (verified by read).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the real-access-stack composition both suites probe through" | mod.rs:1-2 | REAL | both consumers use these helpers; fixtures_matrix green in baseline |
| "not an in-memory stand-in (the demux/codecs test shape)" | mod.rs:5-6 | REAL | FileAccess used (read) |

## 7. Dead / duplicated / unreachable code
None. (Near-duplicate of demux/tests/common's `open_via_input` — different
crates, no shared dev-dep option; acceptable.)

## 8. Dependencies & flags
dev-deps: testkit, input, media-probe, media-types, plugin-api. The
`contract` feature (media-probe/Cargo.toml:36) gates the probe_io consumer,
not this module.

## 9. Open questions for the next agent
- ci-full.yml is a parked/manual workflow (see its header) — if it is not
  on the merge-blocking path, the contract-gated probe_io suite (and this
  module's CountingStream consumers) never runs in CI in practice. Who
  runs `--all-features` on the required path?
