# AUDIT: tests/compat/src/corpus.rs
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 326 / 8 fns (no tests in file) |
| File verdict | REAL-UNPROVEN (runner code; never executed in any committed lane — standalone workspace, same F-001 as config.rs) |
| Reachable from shipped entry point? | NO (compat-matrix runner CLI, tests/compat standalone workspace) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
Corpus assembly for the compat runner: committed-root walk (BTreeMap-ordered,
dedup by workspace-relative id) plus a manifest-anchored ffmpeg regen set with
sha256 verification that never silently uses drifted bytes (drift => regenerate
=> re-verify => Failed record, excluded from the sweep but counted in the
report). Real code, honest failure records; never run in any committed lane;
one silent-exclusion nit (non-UTF8 filenames).

## 2. What actually works (verified)
- Read end-to-end; the drift discipline is genuinely implemented:
  (:216-231) deterministic rows with mismatched bytes go to `regenerate` +
  `verified` (not used silently); (:248-255) Failed rows are recorded and
  excluded from the sweep; the manifest schema is discriminated (:185-190).
- `regenerate` replaces the literal `__OUT__` marker with the target path
  (:305-313) and requires `exit 0` per step (:315-317).

## 3. Findings
F-001 [S4] Where: corpus.rs:140-141.
Code: `let Some(name) = name.to_str() else { continue };`
What is wrong: a media file whose name is not valid UTF-8 is silently skipped —
the compat matrix loses coverage with no record (contrast: every other
exclusion here is recorded or config-declared). Confidence: HIGH — read.
F-002 [S4] Where: corpus.rs:315 `if !record.outcome.starts_with("exit 0")`.
What is wrong: success detection is a string prefix on the proc record
("exit 0"); exit code 0 followed by anything (e.g. "exit 0 (timeout)") is
accepted as success while any formatting change in proc.rs's outcome string
breaks the gate. Stringly subprocess classification. Confidence: HIGH — read;
proc.rs is outside this assignment (UNVERIFIED what outcome strings exist).
Execution finding: shared with config.rs F-001 — the standalone
tests/compat workspace never ran in the committed baseline (rg over
audit/runs/baseline/: zero hits), so this whole path is REAL-UNPROVEN.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| collect_committed | 99-108 | REAL | — | malformed root walked as Malformed class |
| walk_root | 111-175 | REAL | S4 | recursion; silent non-UTF8 skip (F-001); extension match case-insensitive |
| read_manifest | 179-192 | REAL | — | schema check corpus-manifest/1 |
| ensure_regen_corpus | 199-264 | REAL | — | verify/regen matrix; Failed excluded+recorded |
| regen_failed | 269-278 | REAL | — | honest typed failure text |
| verified | 281-291 | REAL | — | sha pin compare with actionable error |
| regenerate | 295-326 | REAL | S4 | __OUT__ substitution; "exit 0" prefix (F-002) |

## 5. Test honesty: no tests in this file (validated indirectly by the runner; suite never run — F-001 config.rs).
## 6. Doc claims: "the manifest is committed, the bytes regenerate on demand ... sha256-verified for deterministic rows" (:1-4) — implemented; REAL (read).
## 7. Dead code: none.
## 8. Dependencies: serde, crate::{proc, sha256, config}; no engine crates.
## 9. Open questions: proc.rs outcome-string contract (F-002) — flag to its owner.
