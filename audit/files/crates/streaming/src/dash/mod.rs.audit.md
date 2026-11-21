# AUDIT: crates/streaming/src/dash/mod.rs
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-21 |
| Lines / functions | 39 / 8 items in table |
| File verdict | REAL (library; UNREACHABLE from shipped entry points) |
| Reachable from shipped entry point? | NO. `rg -n "streaming::dash\|dash::" apps/ tests/ --glob '*.rs'` → zero hits; within `crates/` only the streaming crate itself references the module (lib.rs:32 `pub mod dash;` + the module's own files). The shipped `lumen` binary compiles it (un-gated pub mod) and never calls it. The cargo-fuzz shim (`fuzz/fuzz_targets/fuzz_demux_mpd.rs:22 → streaming::dash::fuzz::fuzz_demux_mpd`) is a dev-workspace caller, not a shipped entry point. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "streaming::dash\|dash::" apps tests` (zero); `rg -n "fuzz_demux_mpd" fuzz/Cargo.toml fuzz/README.md fuzz/fuzz_targets/*.rs docs/formats/streaming-dash.md` (target registered at fuzz/Cargo.toml:293, README row :88, shim :22); `ls fuzz/fuzz_targets \| wc -l` → 35 (matches baseline gate); baseline: full-gate.log.txt fuzz-registry PASS ("35 targets, diff empty", INDEX.md #18). |

## 1. One-paragraph verdict
A 39-line module document: four `pub mod` declarations (abr/fuzz/resolve/sched), three re-export groups, and an accurate roadmap doc of what each child owns. The registry side checks out — the `fuzz_demux_mpd` cargo-fuzz target the doc promises exists, is registered in all three registry surfaces, and the baseline fuzz-registry gate passed with 35 targets and an empty diff. The problem is the one this whole domain shares: nothing outside the streaming crate references `dash`, so the shipped binary carries the entire DASH stack as compiled-but-unreachable code, while the feature matrix rates DASH playback "partial" on three platforms — a product-level claim the shipped binary cannot honor because no entry point dispatches into this module.

## 2. What actually works (verified)
— verified by `rg -n "fuzz_demux_mpd" …` → target file, `[[bin]]` entry (fuzz/Cargo.toml:293-294), README registry row (:88), and the shim calling the in-crate harness (`fuzz_targets/fuzz_demux_mpd.rs:22`).
— verified by baseline `audit/runs/baseline/INDEX.md` #18 → `scripts/fuzz_registry_check.sh` PASS, 35 targets, diff empty (registry↔implementation consistency machine-checked).
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt` (streaming 214-test run) → the dash child modules' tests (27 per the matrix evidence string) pass.

## 3. Findings

### F-001 [S3]
Where: crates/streaming/src/dash/mod.rs:1-2 vs docs/feature-matrix.toml:1449 (`network-dash` statuses)
Code:
```
//! DASH streaming (Phase 6, wave 6.C), built on the 6.C1a MPD
//! model in [`playlists::mpd`].
```
(and the matrix row:)
```
status = { windows = "partial", linux = "partial", macos = "partial", web = "not_started" }
```
What is wrong: the matrix rates DASH playback "partial" on windows/linux/macos, and the row's evidence cites real code and passing tests — but the shipped binary cannot begin a DASH session: no code path from `lumen` (or tauri) reaches `streaming::dash` (zero references, rg). "Partial playback" at product level implies the product plays DASH in the lanes marked landed (static manifests); it does not, because nothing dispatches an MPD URL to this stack — the CLI's URL lane opens only file/http(s) progressive sources (demux_host.rs:415-437, verified in my schemes.rs audit).
Why it matters: the row conflates library capability (real, tested) with product capability (zero, no entry point) — the same class of over-claim the parity report's 0/252 exposes at full-product level.
What correct looks like: either an MPD/MIME dispatch in the open path that constructs a DASH session (the 6.C session-integration card), or the row's statuses reflect "library landed, product wiring not started" (a dedicated wiring status or note).
Confidence: HIGH — Verification: rg over apps/ and tests/; CLI open path read (my schemes.rs#part2 audit, F-002).
Note: final matrix-row verdict belongs to the matrix-rows agent; recorded here as the module-side evidence.

### F-002 [S4]
Where: crates/streaming/src/dash/mod.rs:26
Code:
```
pub mod fuzz;
```
What is wrong: the in-crate fuzz harness is an un-gated public module, so it (and everything it pulls) compiles into the shipped `lumen` binary; its only callers are dev surfaces (the fuzz workspace's shim and tests). Same pattern as `mockhttp` (my mockhttp.rs#part1 F-001) — the test-only posture is a comment, not a gate.
Why it matters: dead weight in the release binary; zero production risk beyond size.
What correct looks like: `#[cfg(any(test, feature = "fuzz-harness"))] pub mod fuzz;` with the fuzz workspace enabling the feature.
Confidence: HIGH — Verification: read; rg confirms no non-dev caller.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-23 | REAL | S3 (F-001 context) | Accurate description of the four children (cross-checked against my mpd.rs#part1 audit of the parse core and the matrix evidence strings); the over-claim is the matrix row, not this text. |
| `pub mod abr` | 25 | REAL (lib-only) | — | ABR ladder selection; tests pass (baseline). |
| `pub mod fuzz` | 26 | REAL (lib-only) | S4 (F-002) | Harness contract: parse+resolve, panic-free; 4 in-crate tests + cargo-fuzz shim. |
| `pub mod resolve` | 27 | REAL (lib-only) | — | Template/timeline/BaseURL resolution (A06/A07 own the parts). |
| `pub mod sched` | 28 | REAL (lib-only) | — | Segment walk + live-edge arithmetic (A02 owns the file). |
| `pub use abr::{…}` | 30-34 | REAL (lib-only) | — | 10 symbols re-exported; no external consumer. |
| `pub use resolve::{…}` | 35 | REAL (lib-only) | — | 3 symbols. |
| `pub use sched::{…}` | 36-39 | REAL (lib-only) | — | 6 symbols incl. `parse_iso8601_epoch_seconds`. |

## 5. Test honesty
No tests in this file. The children's tests run in baseline (27 dash tests per the matrix evidence string; the streaming 214-test run passed 0-fail). Mutation relevance: nothing here to mutate except re-exports — a wrong re-export breaks child-module compilation, caught by build.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`fuzz` (card 6.C1c) — the in-crate fuzz harness the `fuzz_demux_mpd` cargo-fuzz target shims" | mod.rs:12-15 | REAL | Target + shim + registry verified (fuzz/Cargo.toml:293, README:88, shim:22); registry gate PASS in baseline. Note: the fuzz *campaign* itself is unexecuted ("once nightly is wired", shim comment :15) — the harness is real, the campaign is PAPER-ONLY so far. |
| "resolve … a pure function of the parsed model plus the manifest URL, bounded by typed plan budgets" | mod.rs:4-11 | REAL-UNPROVEN (here) | Parts owned by A06/A07; mpd parse core verified in my part1 audit. |
| "DASH playback … partial (win/linux/macos)" | feature-matrix.toml:1449 | FALSE at product level (REAL at library level) | Zero shipped callers (F-001); library tests pass. |
| "MPD reload loop (minimumUpdatePeriod), timeShiftBufferDepth trimming, multi-Period stitching, CENC decrypt … pending" | feature-matrix.toml:1454 notes | REAL (honest gap note) | Consistent with my mpd.rs#part1 audit (no reload logic in the parse core). |

## 7. Dead / duplicated / unreachable code
The whole module is unreachable from shipped entry points (header trace). No internal dead code; all four children are consumed by the module's own re-exports and tests.

## 8. Dependencies & flags
Children depend on `playlists` (registered production edge, streaming/Cargo.toml:42) and crate-internal modules. No feature gates (F-002). No unsafe. 39/39 lint inheritance (baseline hygiene).

## 9. Open questions for the next agent
- Matrix-rows owner: `network-dash` (feature-matrix.toml:1445-1454) — the product-level verdict needs the zero-caller fact (F-001); the row's evidence string is library-only.
- A02 (sched.rs): the ISO 8601 availability-time arithmetic is the row's riskiest un-evidenced claim (no DASH-IF vectors yet, per the row's own notes).
- Whoever owns the session-integration card: MPD URL/MIME dispatch into this stack is the missing wiring for every claim in this module.
