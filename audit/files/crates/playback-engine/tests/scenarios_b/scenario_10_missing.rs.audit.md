# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_10_missing.rs
| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-17 |
| Lines / functions | 111 / 3 (2 helpers + 1 #[test]) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test file over the real command layer; the typed-refusal path (`session.open` on inaccessible sources → `Stage::Input`/`ErrorCode::Inaccessible`) is shipped CLI behavior (`lumen open` on a missing file). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | baseline cargo-test-workspace.txt:4399 (passes); sed host.rs:2075-2083 (TempDir under std::env::temp_dir); sed .github/workflows/ci.yml:40-52 (ubuntu-only blocking lane, 3-OS parked). |

## 1. One-paragraph verdict
A tight, real scenario: four inaccessible shapes (nonexistent path, directory, dangling symlink, chmod-000 file) each pinned to the same typed refusal — `Stage::Input`, `ErrorCode::Inaccessible`, basename preserved, host Idle, no session, later snapshots `NotReady` — plus a reusability check (a good open succeeds after four refusals). It passes in baseline. Two defects: the V8 "no machine path" redaction assert checks for `"/home/"` while every refusal path lives under the system temp dir (`/tmp/...`), so a leaked machine path would sail through the check — the redaction property is asserted with a needle that cannot bite; and the chmod-000 shape assumes a non-root runner (root reads chmod-000 files, making the open succeed and the test panic in `unreachable!`).

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt:4399` -> "scenario_10_missing::missing_and_unreadable_paths_refuse_typed_and_redacted ... ok".
— verified by sed host.rs:2077-2081: `TempDir::new` creates under `std::env::temp_dir()` — basis of F-001.
— The four shapes are distinct failure layers (canonicalize refusal / not-a-regular-file / resolve failure / access-layer permission), matching the header's claim.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/scenarios_b/scenario_10_missing.rs:46-49
Code:
```
    assert!(
        !error.message.contains("/home/"),
        "the message carries no machine path (V8): {error}"
    );
```
What is wrong: The redaction negative-assert searches for the literal `"/home/"`, but all four refusal paths are constructed under `TempDir::new("s10")`, which lives under `std::env::temp_dir()` (host.rs:2077-2081 — typically `/tmp/lumen-scenarios-b-s10-<pid>/…`). A redaction failure that leaked the actual machine path (`/tmp/lumen-scenarios-b-s10-1234/no-such-file.avi`) contains no `/home/` and passes the assert. The only path on this machine that could trip the needle is the fixture path (`/home/z/...`), which is never the subject of a refusal here. Why it matters: the test name and message claim "redacted"; the property is unproven for every shape it exercises. What correct looks like: `assert!(!error.message.contains(scratch.path().parent-fragment))` — use the real scratch prefix as the needle (and keep a basename positive assert). Confidence: HIGH — Verification: read + sed host.rs.
F-002 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_10_missing.rs:11,90
Code:
```
use std::os::unix::fs::PermissionsExt;
```
and
```
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000)).must();
```
What is wrong: (a) Running as root, chmod-000 files remain readable — the open would succeed and the test panics in `unreachable!("an inaccessible path must refuse, not open")`; the test silently assumes a non-root runner (CI is ubuntu-latest, so it holds today). (b) The file is unix-only (`std::os::unix::fs::symlink` too); the blocking CI lane is ubuntu-only and the 3-OS matrix is parked non-blocking (ci.yml:40-52), so a Windows `cargo test` would not compile this suite — deferred, not broken, but unreconciled with the parity report's Windows row. Confidence: HIGH — Verification: read + sed ci.yml.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| refuse_open | 22-34 | REAL | — | Dispatch + typed-error extraction; success is a panic (correct polarity). |
| assert_inaccessible | 38-60 | REAL | S3 | Stage/code/basename/Idle/no-session/NotReady all pinned; F-001 wrong redaction needle. |
| missing_and_unreadable_paths_refuse_typed_and_redacted | 63-111 | REAL | S4 | Four shapes + reusability; F-002 root/unix assumptions. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| missing_and_unreadable_paths… | YES for typed refusal, Idle, no-session, NotReady, reusability (exact equalities). NO for the redaction property (F-001 — wrong needle). Root-run environments invert it into a false panic (F-002). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| Row 10: "missing file | absent path, unreadable path | typed `Inaccessible` with redacted message | transcript" | docs/10-testing-ci-quality.md:690 | PARTIAL | Typed Inaccessible: REAL (4 shapes). Redacted message: NOT PROVEN by this test (F-001). |
| "V8 redaction: the basename survives, machine paths do not" | scenario_10:41-42 | UNPROVEN here | F-001: the negative assert cannot fail for temp-dir paths. |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
commands, testkit (Must, serialize_tests), scenarios_b host (TempDir). Unix-only APIs; no feature gates.

## 9. Open questions for the next agent
- Fix is one line (needle = scratch path). Whoever owns the V8 redaction rule: is redaction implemented at the message-construction site (search for the redaction helper) and unit-tested there? If not, the property has NO effective test anywhere.
- If ci-full.yml (3-OS) is ever promoted, this suite needs `#[cfg(unix)]` gating or a Windows-compatible shape set.
