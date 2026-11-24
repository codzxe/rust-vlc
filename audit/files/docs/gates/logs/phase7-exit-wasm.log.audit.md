# AUDIT: docs/gates/logs/phase7-exit-wasm.log
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Date | 2026-10-30 |
| Lines / functions | 27 / 4 recorded invocations |
| File verdict | REAL data — but it proves cache freshness, not a build |
| Reachable from shipped entry point? | N/A — committed gate-evidence artifact |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `rg` over apps/web/Cargo.toml, root Cargo.toml, ci.yml (all exit 0). Log not re-runnable (wasm rebuild = cargo-build-budget; baseline already covers it: "wasm build passes with the wasm32 rust-std component installed"). |

## 1. One-paragraph verdict
A short, well-formed evidence log: every command has a Cmd header, date stamps, and an explicit exit marker (`WASM_EXIT=0`, `WASM_CANARY_EXIT=0`, locked variants). But the main "build" lines (`Finished dev profile … in 0.12s`, `in 0.06s`, `in 0.02s`) are pure cache hits — cargo verified freshness and did nothing — and the cited artifact's mtime (Nov 10 **16:16**) PREDATES both logged commands (16:56:57, 17:12:23). The log therefore proves cargo exited 0 on an already-built tree, not that the recorded commands produced the artifact. Only the theme-tokens canary (3.16s, real `Compiling` lines) shows actual compilation.

## 2. What actually works (verified)
- `-p lumen-web` is a real workspace package: apps/web/Cargo.toml:37 `name = "lumen-web"`.
- The mirrored CI job exists: .github/workflows/ci.yml:548 `wasm-build:` "wasm-build (G12 canary — theme-tokens, wasm32-unknown-unknown)" — the log's "mirrors CI G12 wasm-build job" claim (:10) lands.
- Exit markers are explicit, not inferred — better hygiene than phase3's log.

## 3. Findings
F-001 [S2]
Where: docs/gates/logs/phase7-exit-wasm.log:3-8
Code:
```
# Date: 2026-11-10T16:56:57Z
# Cmd: cargo build -p lumen-web --target wasm32-unknown-unknown
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
WASM_EXIT=0
# Artifact:
-rwxrwxr-x 2 z z 5998961 Nov 10 16:16 /home/z/my-project/rust-vlc/target/wasm32-unknown-unknown/debug/lumen_web.wasm
```
What is wrong: the artifact's timestamp (16:16) is 40 minutes OLDER than the command that claims it (16:56:57); the 0.12s "Finished" confirms zero compilation — the recorded command did not produce the artifact it lists. Why it matters: as exit-gate evidence this log cannot prove the wasm32 composition root builds — a stale or corrupted incremental cache yields the same output; the artifact was built by an unlogged earlier invocation. What correct looks like: `touch` a source or `cargo clean -p lumen-web` + rebuild so the log shows real Compiling lines, or cite the 16:16 build's own log. Confidence: HIGH — Verification: read (timestamps in the log itself).

F-002 [S3]
Where: docs/gates/logs/phase7-exit-wasm.log:20-26
Code:
```
# Cmd: cargo build -p lumen-web --target wasm32-unknown-unknown --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
WASM_LOCKED_EXIT=0
```
What is wrong: the --locked CI-parity run (:18 "doc 11 §3.7: CI builds run --locked") is ALSO a 0.06s cache hit with the same 16:16 artifact re-listed at :26 — so the "CI-parity" evidence has the same nullity: --locked only re-verifies the lockfile is fresh when there is nothing to compile. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: docs/gates/logs/phase7-exit-wasm.log:2, 8
Code:
```
# Worktree: /home/z/my-project/wt/w24f (branch w24/f @ a4e35e7)
```
What is wrong: build ran in worktree wt/w24f but the artifact lives in the ORIGINAL checkout's shared target dir (`/home/z/my-project/rust-vlc/target/…`) — same unrecorded-CARGO_TARGET_DIR pattern as the phase3 log (no env/state capture in the log). Confidence: HIGH — Verification: read.

## 4. Per-function table
N/A (data log). Structure inventory:
| Section | Lines | Content | Verified |
|---|---|---|---|
| wasm build | 1-8 | Cmd+date+exit, artifact listing | read; artifact predates cmd (F-001) |
| canary | 10-16 | theme-tokens real compile (3.16s) | read |
| locked parity | 18-26 | two cache-hit exits + same artifact | read (F-002) |

## 5. Test honesty
N/A — the log IS the evidence; its honesty flaw is the cache-hit/nullity issue above, not fabrication: nothing here looks invented (sizes, seconds, and paths are internally consistent).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| wasm32 build of lumen-web exit 0 | log:4-6 | REAL-UNPROVEN | the exit marker is recorded; but the run compiled nothing (0.12s) and the artifact predates it — cannot attest build health from this log; baseline corroborates wasm builds pass on current main |
| "mirrors CI G12 wasm-build job" | log:10 | REAL | ci.yml:548 `wasm-build (G12 canary …)` |
| `--locked` CI-parity builds | log:18-25 | REAL-UNPROVEN | same nullity; exits recorded but nothing compiled |

## 7. Dead / duplicated / unreachable code
The artifact line is duplicated at :8 and :26 (same size/mtime) — a copy of the earlier build's listing, not evidence of the later runs.

## 8. Dependencies & flags
wasm32-unknown-unknown target, lumen-web + theme-tokens packages, shared CARGO_TARGET_DIR (unrecorded).

## 9. Open questions for next agent
- Find the 16:16 invocation that actually built lumen_web.wasm — its log (with real Compiling output) is the true phase-7 wasm evidence; this file is only a cache-validation transcript.
- If phase7-exit.md cites this log as "wasm build green", that is technically true and evidentially empty — call it out there.
