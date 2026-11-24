# AUDIT: docs/gates/logs/phase3-exit-fmt-initial-drift.log (DATA AUDIT — log)
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines | 403 |
| File verdict | REAL (authentic `cargo fmt --check` drift output; exit code embedded) |
| Reachable from shipped entry point? | N/A (log data; cited by docs/gates/phase3-exit.md:51,210) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 re.findall count (32 entries / 16 path strings / 15 real files); tail -> `FMT_EXIT_CODE:1` |

## 1. One-paragraph verdict
Genuine rustfmt drift log from worktree `w7-gate`: 32 `Diff in` entries over 15
real files (16 path strings — `examples/../src` and `src` resolve to the same
decoder.rs), ending in a recorded `FMT_EXIT_CODE:1`. The citing doc's claim
"FAIL — 32 diffs across 15 files" (phase3-exit.md:51) is EXACT — I recounted
independently. The log is honest red data. Context finding: the CURRENT tree's
fmt failure (baseline cargo-fmt.txt: diff in crates/plugin-api/tests/testkit_wiring.rs:40)
is in DIFFERENT files than this log — fmt drift re-accumulated after the phase-3
fix, consistent with CI-red-today.

## 2. What actually works (verified)
- `python3` count: entries=32, unique path strings=16, real files=15 (`examples/../src/video/mjpeg/decoder.rs` == `src/video/mjpeg/decoder.rs`).
- `tail -c 300` -> final line `FMT_EXIT_CODE:1` — the exit code is IN the log (honest, unlike the playwright log which omits it).

## 3. Findings
F-001 [S4]
Where: docs/gates/logs/phase3-exit-fmt-initial-drift.log:1-403 (structure)
Code:
```
Diff in /home/z/my-project/worktrees/w7-gate/crates/codecs/examples/../src/video/mjpeg/decoder.rs:1712:
```
What is wrong: drift entries carry the double-path form `examples/../src/...` for
the same file also listed as `src/...` — cosmetic noise that makes file counts
ambiguous (doc says 15, naive unique-path count says 16). Why it matters: future
auditors recounting "15 files" must deduplicate by hand. Confidence: HIGH —
Verification: ran the recount.

## 4. Data integrity checks
- 32 diffs / 15 files matches phase3-exit.md:51 verbatim claim — no inflation.
- Exit code recorded: FMT_EXIT_CODE:1.
- Provenance: worktree `w7-gate` (path prefix), i.e. pre-fix evidence snapshot; cannot re-run fmt (forbidden), but the CURRENT tree's independent fmt failure (different files) corroborates the drift-recurrence story.
- Fmt diff content is plausible rustfmt output (line-joining and import-reordering diffs only; no semantic markers) — read.
