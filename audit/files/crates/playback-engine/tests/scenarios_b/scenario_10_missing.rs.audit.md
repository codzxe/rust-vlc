# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_10_missing.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 111 / 1 `#[test]` + refuse_open + assert_inaccessible helpers |
| File verdict | **REAL** — four typed-refusal shapes with redaction and state assertions; unix-only by construction |
| Reachable from shipped entry point? | Subject: the real command layer over the real input access path (ScenarioHost, real engine — see host.rs ARCH-5 note). Test runs under `cargo test` — **on unix only** (see F-001). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read of the full file. No cargo (static). |

## 1. One-paragraph verdict

A tight negative-path scenario: four distinct inaccessible shapes (nonexistent path, directory, dangling symlink, chmod-000 file), each asserted to produce the typed `Input`/`Inaccessible` refusal with the basename surviving and machine paths redacted (the V8 check at 46-49 explicitly asserts no `/home/` in the message), the host left `Idle` with no session, later snapshots answering the honest `session.not-ready` (Stage::Session/NotReady — a *different* stage/code than the open refusal, correctly distinguishing the two), and — the part most negative tests skip — the host proven reusable afterwards by opening the good parent fixture and closing cleanly. The chmod-000 case correctly notes the layer split (canonicalize succeeds; the access layer owns the permission refusal). One defect: the file is unix-only (`std::os::unix::fs::PermissionsExt` at 11, `std::os::unix::fs::symlink` at 83), so this scenario cannot compile on Windows — the platform the project's own mission targets.

## 2. What was checked

- All four shapes drive the same helper with exact stage/code asserts (39-40) — a mutation of the error taxonomy fails immediately.
- The redaction assert is two-sided: basename present (43), machine path absent (47).
- The not-ready snapshot check (52-59) verifies *post-failure* state, not just the refusal.
- Reusability check (94-110): opens the real fixture after four refusals — catches poisoned-host regressions.
- No tautologies, no `assert!(is_err())`-only smokes; every Err is destructured and its fields asserted.

## 3. Findings

### F-001 [S4] — the scenario is unix-only: `std::os::unix` items make the file uncompilable on Windows
Where: crates/playback-engine/tests/scenarios_b/scenario_10_missing.rs:11, 83
Code:
```rust
use std::os::unix::fs::PermissionsExt;
```
and:
```rust
    std::os::unix::fs::symlink(scratch.path().join("gone.avi"), &dangling).must();
```
What is wrong: `PermissionsExt` (chmod-000) and `std::os::unix::fs::symlink` (dangling link) are unix-only APIs with no `#[cfg(unix)]` gate — the whole `scenarios_b` test target fails to compile on a Windows host, taking scenarios 1-16's coverage with it.
Why it matters: the audit's stated target is "an MP4 plays with picture and sound on Windows"; the negative-path scenario coverage (doc 10 §6.4 rows 9-16) cannot exist on that platform. The parity baseline already records 0/252 verified on windows; this is one structural reason why.
What correct looks like: `#[cfg(unix)]` on the unix-shaped sub-cases with Windows equivalents (readonly attribute, `symlink` via `std::os::windows::fs`), or an explicit platform note in the scenario matrix.
Confidence: HIGH — Verification: read (the imports are unconditional).

## 4. Per-function table

| Item | Lines | Verdict | Notes |
|---|---|---|---|
| refuse_open | 22-34 | REAL | success is `unreachable!` — the right polarity |
| assert_inaccessible | 38-60 | REAL | stage/code/redaction/state/snapshot — five asserts |
| missing_and_unreadable_paths_refuse_typed_and_redacted | 64-111 | REAL | four shapes + reusability (§1) |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| missing_and_unreadable_paths_refuse_typed_and_redacted | YES | taxonomy, redaction both directions, state reset, cross-stage not-ready, reusability |

UNVERIFIED by run (no cargo); W00 baseline consistency applies (Linux).

## 6-9

- Doc claim "Pass criteria: typed Inaccessible with a redacted message" (4): the body enforces exactly that plus three extra invariants.
- No dead code.
- Open question for wave 17/18: any matrix row citing scenarios 9-16 as evidence must record the unix-only limitation.
