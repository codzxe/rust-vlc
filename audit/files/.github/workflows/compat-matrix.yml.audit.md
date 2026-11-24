# AUDIT: .github/workflows/compat-matrix.yml
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines / jobs | 161 / 1 job (compat-matrix) × 3-OS matrix with explicit platform column, fail-fast: false |
| File verdict | REAL (honest parked-by-design full-corpus matrix lane; core refusal logic executed) |
| Reachable from shipped entry point? | N/A — CI infra; trigger = workflow_dispatch ONLY (parsed) — manual, never on PR/push, never required |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | python3 yaml parse (trigger/matrix/permissions); derive_config.py 'web' refusal test (rc 1, no writes); rg over tests/compat |

## 1. One-paragraph verdict
A parked-by-design full-corpus compat sweep: dispatch-only trigger (parsed —
it can never gate a merge and the header says exactly that), 3-OS matrix with
an EXPLICIT `platform:` lane attribute (the "attribution is never inferred
from the host" rule, materialized as matrix include entries), fail-fast:
false, least-privilege permissions, byte-identical action pins, per-OS
check-first ffmpeg install with a loud unknown-RUNNER_OS error, and a real
derive step: I executed tests/compat/ci/derive_config.py's refusal path
(`web` -> exit 1, message verbatim, zero files written). The workflow's
honesty rules are structurally real: the derive script reads the LIVE
docs/feature-matrix.toml (LIVE_MATRIX constant), writes only under ci-out/,
and the transitions outbox goes to the uploaded artifact, never to
docs/feature-matrix.toml. The 2026-11-10 sandbox end-to-end claim (166 rows,
wasm lane, exit 0) is UNVERIFIABLE now (no cargo in this sandbox). Findings
are hygiene-level only: the stale .cargo/config.toml comment (same family as
ci.yml/ci-full.yml), unpinned ffmpeg installs, and if-no-files-found: warn
on the evidence upload (deliberate, documented).

## 2. What actually works (verified)
- yaml.safe_load -> `on:` = workflow_dispatch ONLY; matrix include = [{ubuntu-latest, linux}, {windows-latest, windows}, {macos-latest, macos}]; fail-fast: false; permissions contents: read; upload step `if: always()`.
- Refusal executed: `python3 tests/compat/ci/derive_config.py web /tmp/never.json /tmp` -> `derive_config: platform must be one of windows/linux/macos (got 'web' — `web` is the steps lane, never a native sweep)` rc 1; `ls ci-out` -> absent (no side effects on refusal).
- Runner exists: tests/compat/Cargo.toml `name = "compat-matrix"` with the full-corpus runner description; derive_config.py:59 `PLATFORMS = ("windows", "linux", "macos")`, :60 LIVE_MATRIX = "docs/feature-matrix.toml".
- Action pins byte-identical to ci.yml's four pins (lines 58-61 vs ci.yml:90-93,141,154,171).

## 3. Findings
F-001 [S4]
Where: .github/workflows/compat-matrix.yml:52-54, 85-87
Code:
```
# Toolchain: identical pin/assert/cache posture to ci.yml + ci-full.yml
...
# overrides the sandbox-path build.target-dir pinned in the committed
```
What is wrong: stale .cargo/config.toml reconciliation comment — the
committed config now has the restored alias and NO target-dir (cat verified).
Same comment-rot family as ci.yml:82-86. Confidence: HIGH — Verification: cat.
F-002 [S4]
Where: .github/workflows/compat-matrix.yml:127-144
Code:
```
              command -v ffmpeg >/dev/null 2>&1 || choco install -y --no-progress ffmpeg
```
What is wrong: unpinned ffmpeg installs on all three OSes (apt/choco/brew) —
the regenerable corpus regeneration depends on a floating ffmpeg version;
a runner-image bump changes corpus generation behavior mid-history.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: .github/workflows/compat-matrix.yml:161
Code:
```
          if-no-files-found: warn # a crash before the report write still uploads nothing — the job log is the evidence then
```
What is wrong: a crashed sweep leaves NO artifact (warn only) — weaker than
tauri-launch.yml's `error` posture. Deliberate + documented rationale, so
hygiene-level: the "evidence" promise ("a failed run uploads for triage too",
line 155) is conditional on the runner having written ci-out before dying.
Confidence: HIGH — Verification: read.

## 4. Per-job table (mandatory; file >100 LOC)
| Job | Lines | Steps | Verdict | Notes |
|---|---|---|---|---|
| compat-matrix | 90-161 | checkout / toolchain assert / rustup wasm32 / cache / ffmpeg / derive / run / upload-always | REAL-UNPROVEN end-to-end (runner needs cargo+ffmpeg; sandbox lacks cargo) | refusal path of the derive step EXECUTED (rc 1); dispatch-only |

## 5. Test honesty
The workflow's own honesty rules (lines 32-48) are the test contract:
attribution explicit (verified in matrix include), never edits
docs/feature-matrix.toml (derive writes ci-out only — read), oracle absence
recorded verbatim (runner behavior — UNVERIFIED without execution).
Mutation reasoning: if derive_config.py accepted a bogus platform, my refusal
test would have exited 0 — it did not.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Trigger is MANUAL DISPATCH ONLY" | :13 | TRUE | parsed `on:` = workflow_dispatch only |
| "the derive script refuses anything outside windows/linux/macos" | :97-98 | TRUE (executed) | web -> rc 1 + message, no writes |
| "refreshes the claims snapshot from the LIVE docs/feature-matrix.toml (never a stale snapshot)" | :34-37 | TRUE-STRUCTURAL | derive_config.py:60 LIVE_MATRIX constant; read |
| "This job NEVER edits docs/feature-matrix.toml" | :42-43 | TRUE-STRUCTURAL | derive writes out-config + reports-dir only (param plumbing read); apply-transitions is a separate xtask command |
| "the derived-config + full-run flow was executed end-to-end, 2026-11-10, 166 rows + wasm lane steps exit 0" | :27-29 | UNVERIFIED | needs cargo + ffmpeg + corpus regen; no cargo in this sandbox |
| "windows/macos lanes have NEVER executed on real runners" | :26-30 | UNVERIFIED | run history needs repo access |

## 7. Dead / duplicated / unreachable code
Toolchain-assert block duplicated once (single job) — acceptable. Nothing dead.

## 8. Dependencies & flags
Same pins as ci-full.yml (header claims; verified byte-identical for the 4
actions used). External: ffmpeg per-OS (floating), rustup wasm32 target,
python3/python fallback for Windows bash (lines 149-150 — matches
common.sh's PY probe pattern). CARGO_TARGET_DIR=target env retained against
a fixed config.toml issue (stale rationale, harmless).

## 9. Open questions for the next agent
- Does the compat-matrix runner's `run --config` subcommand actually include
  the wasm lane (the workflow adds the wasm32 target for it — line 124's
  parenthetical)? Verify in tests/compat/src when that crate is audited.
- First green 3-OS runs are to be recorded per the header — check
  docs/corpus/compat-matrix-*.md for any recorded runs (dispatch-only means
  zero unless someone ran it manually).
