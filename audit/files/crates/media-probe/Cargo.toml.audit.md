# AUDIT: crates/media-probe/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-13 |
| Lines / functions | 50 / (manifest: 1 package, 1 feature, 2 deps, 2 dev-deps) |
| File verdict | REAL (manifest mechanics accurate; one consumer claim overstated; one evidence gap proven) |
| Reachable from shipped entry point? | YES — apps/cli/Cargo.toml:56 depends on media-probe; lumen-cli open path calls media_probe::probe_stream (apps/cli/src/demux_host.rs:444-446); also apps/web, crates/transcode, tools/* |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg dependent census (7 Cargo.tomls); rg "contract" across all manifests (no enabler); baseline cross-read (cargo-test-workspace.txt:3684-3688: probe_io "running 0 tests"); rg unreachable!/cfg(test) gating in media-probe src; rg CLI probe call site; `cargo test -p media-probe --features contract --test probe_io` → 10 passed (exit 0, executed by me) |

## 1. One-paragraph verdict
The manifest is mechanically accurate: zero external (crates.io) dependencies — only
workspace path deps media-types + plugin-api, dev-deps testkit + input — edition
2024, publish=false, workspace lints inherited, and the `contract` feature really is
compiled out of release builds (`#[cfg(any(test, feature = "contract"))]` at
lib.rs:160). But the evidence chain for the crate's bounded-memory claim has a hole
I can prove: the feature is enabled by NO manifest in the repo, so the crate's own
tests/probe_io.rs — the instrumented bounded-read/position-restored AC over the real
FileAccess stack, i.e. the executable proof of "the fill loop reads at most
max_lookahead bytes — there is no code path that probes more" — compiled to ZERO
tests in the recorded baseline ("running 0 tests", cargo-test-workspace.txt:3684-
3688, per-package run without --all-features). Only CI's --all-features
(ci-full.yml:169) lights it, and no CI log is in the baseline. The header's consumer
list also names "the engine wiring tests" as opting in via this feature; no such
in-repo test exists.

## 2. What actually works (verified)
- Dependency claims: [dependencies] = media-types + plugin-api (path), no crates.io
  deps — "zero external dependencies" TRUE; input is correctly dev-only.
- Lint claim: rg census of media-probe/src — every `unreachable!` (3+7+15+3+8+16+
  11+8+17 across files) sits inside `#[cfg(test)] mod tests` blocks (spot-verified
  lib.rs:238→255ff, signature/mod.rs:224→234ff, error.rs:131→140ff, report.rs:342→
  354ff); no unwrap/expect/panic/todo found in non-test code. Claim holds.
- Bounded-window constants exist: config.rs:18 DEFAULT_MAX_LOOKAHEAD=64 KiB,
  config.rs:23 MAX_LOOKAHEAD_CEILING=1 MiB, config.rs:27 DEFAULT_MIN_SCORE=60.
- Shipped wiring: apps/cli/src/demux_host.rs:444-446 calls
  media_probe::probe_stream with the builtin registry — the probe is on the open
  path of the release binary.
- Release-exclusion claim: lib.rs:160 gates contract behind any(test, feature) —
  with no features, release carries none of it (and no dependent enables it).

## 3. Findings
F-001 [S3]
Where: crates/media-probe/Cargo.toml:13-17 with tests/probe_io.rs:18 and baseline
Code:
```
# Std-only by C5: zero external dependencies. No unsafe, no
# unwrap/expect/panic/todo (workspace lints). Bounded memory (MP §12):
# the probe window is hard-ceiling-capped (doc 03 §4.2) and the fill
# loop reads at most `max_lookahead` bytes — there is no code path that
# probes more.
```
What is wrong: the bounded-read claim's executable proof (tests/probe_io.rs —
CountingStream ledger, assert_probe_depth_bounded, over input's real
FileAccess/FileStream) is `#![cfg(feature = "contract")]` (probe_io.rs:18) and the
feature is enabled by NO manifest in the repository (rg census of every Cargo.toml:
only comments mention "contract"). Consequence, proven: the orchestrator baseline
(per-package `cargo test`, no --all-features) recorded `tests/probe_io.rs ... running
0 tests ... 0 filtered out` (audit/runs/baseline/cargo-test-workspace.txt:3684-
3688) — the probe-depth AC contributed ZERO tests to the "4744 passed" headline.
The claim "there is no code path that probes more" is asserted in three places
(Manifest header, lib.rs docs, contract module) but its only executable check is
dark outside CI's --all-features lane (ci-full.yml:169), and no CI run log exists in
the baseline to substitute. Why it matters: a regression that makes the probe read
past max_lookahead would pass the entire recorded baseline. MITIGATION EXECUTED BY
ME: `cargo test -p media-probe --features contract --test probe_io` → 10 passed
(incl. the_ceiling_is_enforced_before_any_byte_is_read,
the_probe_never_consumes_real_stream_bytes,
truncated_files_prove_the_eos_fill_is_bounded) — the AC is real and passing; the
defect is the evidence chain, not the code. What correct looks like: run probe_io
unconditionally (the CountingStream is test-only anyway), or record a CI
--all-features log in the baseline evidence. Confidence: HIGH — Verification: read
+ baseline cross-read + rg census + executed the feature-on run.
F-002 [S4]
Where: crates/media-probe/Cargo.toml:30-36
Code:
```
# it via cfg(test); this feature is how integration tests, the engine
# wiring tests, the future doc 06 §7 conformance kit, and any downstream
# test harness opt in. Release binaries (no feature) carry zero of its
```
What is wrong: "the engine wiring tests" — no in-repo engine-wiring test enables the
contract feature (no manifest sets it; no source outside crates/media-probe imports
media_probe::contract — rg census; input's contract:: users are input's own module).
The consumer list names a wiring that does not exist; "the future doc 06 §7
conformance kit" is honestly future. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
N/A (manifest). Sections audited: package (name/version/edition/description/publish
— all accurate), lints (workspace=true), features (contract=[] — real, gated,
see F-001/F-002), dependencies (2 path — matches ARCH-2 claim), dev-dependencies
(2 path — matches AC-composition claim).

## 5. Test honesty
The manifest's test claims: "unit tests compile it via cfg(test)" — TRUE (lib.rs:
160 any(test, …)); "integration tests … opt in" via the feature — TRUE mechanically
(probe_io.rs:18) but the opt-in never fires outside --all-features (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "zero external dependencies" | Cargo.toml:13 | REAL | [dependencies] = 2 path deps only |
| "No unsafe, no unwrap/expect/panic/todo" (non-test) | Cargo.toml:14 | REAL | rg census; unreachable!s all cfg(test) |
| "the fill loop reads at most max_lookahead … no code path that probes more" | Cargo.toml:15-17 | REAL | constants + gate exist; I executed the feature-on probe_io suite: 10 passed (F-001 records the baseline gap) |
| "input is a DEV edge: the fixture AC probes … through the real FileAccess/FileStream stack" | Cargo.toml:5-11 | REAL | probe_io.rs does exactly this; 10/10 pass when feature-enabled (executed); dark in the recorded baseline |
| "Release binaries (no feature) carry zero of its panicky conformance vocabulary" | Cargo.toml:34-35 | REAL | cfg(any(test, feature)) at lib.rs:160; no dependent enables it |
| "the engine wiring tests" opt in via the feature | Cargo.toml:31-32 | FALSE | no such test exists (F-002) |

## 7. Dead / duplicated / unreachable code
The `contract` feature is declared and honored but enabled by no manifest in the
tree — live only under CI --all-features. Not dead (CI path), but zero in-repo
enablers.

## 8. Dependencies & flags
media-types, plugin-api (path); dev: testkit, input (path). No version pins needed
(workspace-local). [lints] workspace=true. No unsafe-related flags.

## 9. Open questions for the next agent
- CI auditor: confirm ci-full.yml actually executed (its logs are not in
  audit/runs/baseline/); outside CI, nothing in-repo runs the contract lane — my
  feature-on run (10 passed) is currently the only recorded execution.
- Whoever audits media-probe/src/{lib,config,contract}.rs: the bounded-memory
  property now has one executed witness; per-commit regression protection still
  depends on CI --all-features.
