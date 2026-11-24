# AUDIT: tests/compat/src/config.rs
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 550 / Config + 10 structs/enums + load + 5 tests |
| File verdict | REAL-UNPROVEN (genuine, well-designed validation tests; NEVER executed in any committed run — standalone workspace, see F-001) |
| Reachable from shipped entry point? | NO (tests/compat standalone runner crate, not a root-workspace member, not shipped) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |

## 1. One-paragraph verdict
A clean serde config schema (deny_unknown_fields everywhere) with a serious
validation story (schema discriminator, exactly-one native lane, platform
whitelist, duplicate-lane refusal) and five targeted tests that genuinely load
config documents from temp files and assert the refusal texts. The design is
honest (old schemas refused, never defaulted; race-safe unique temp names with
a documented thread rationale). But none of it has ever run in any committed
lane: tests/compat is a standalone workspace excluded from the root workspace,
and no compat-suite run exists in audit/runs/baseline/.

## 2. What actually works (verified)
- Read end-to-end; the 5 tests each construct configs and call the real `load`
  (not mocks). Test bodies verified against implementation: schema check
  (:291-297) vs wrong_schema_is_refused (:462-466); platform whitelist
  (:318-328) vs web_native_platform_is_refused (:472-479); native==1
  (:307-317) vs two_native_lanes_are_refused (:484-496); dup/empty checks
  (:298-335) vs structural_rejections (:501-533).
- Committed config: tests/compat/compat-matrix.json (schema compat-matrix/
  config/2) — consumed by main.rs (same suite).

## 3. Findings
F-001 [S2] Where: tests/compat/Cargo.toml:5-8 (standalone workspace) vs the
committed baseline.
Code (tests/compat/Cargo.toml header): "Deliberately a STANDALONE workspace
(empty [workspace] table below), NOT a member of the root workspace".
What is wrong: the root-workspace test baseline (audit/runs/baseline/
cargo-test-workspace.txt, 7,135 lines) contains NO PACKAGE for this crate, and
no separate compat-suite run is committed anywhere under audit/runs/baseline/
(rg "tests/compat|compat-matrix/config" over all baseline files: zero hits).
So all five config validation tests — and by extension the whole compat-matrix
runner that produced docs/corpus/compat-matrix-20261110.jsonl — are
REAL-UNPROVEN: no committed execution. Why it matters: the compat matrix is
the project's format-support evidence chain; its runner's own tests never ran
in the recorded baseline. What correct looks like: a recorded `cargo test`
run inside the standalone workspace (or Lead-approved workspace membership).
Confidence: HIGH — Verification: read + baseline grep (no hits).
F-002 [S4] Where: config.rs:248-256.
Code: `pub fn status_for(&self, platform: &str) -> &str { match platform { "windows" => &self.windows, "linux" => &self.linux, "macos" => &self.macos, "web" => &self.web, _ => &self.linux } }`
What is wrong: the catch-all arm silently answers the LINUX claim for any
unknown platform string. The doc comment says validation makes it unreachable,
but status_for is `pub` — any caller outside the validated path (e.g. a
config-driven string) gets a wrong-platform answer with no error. A
wrong-attribution guard that itself silently mis-attributes. Confidence: HIGH —
read.
F-003 [S4] Where: config.rs:449-457, 501-512.
What is wrong: `insert_lane`/`structural_rejections` do string surgery on the
JSON template (find "\"lanes\": [" and splice) — brittle vs a real JSON builder,
though each splice is marker-asserted (`unreachable!` if absent). Confidence:
HIGH — read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Config + 10 schema structs | 26-283 | REAL | — | deny_unknown_fields on every struct; doc comments carry the schema history |
| MatrixClaim::status_for | 240-257 | REAL | S4 | silent linux fallback (F-002) |
| load | 286-338 | REAL | — | 6 typed validations incl. schema discriminator |
| Config::rel | 341-345 | REAL | — | workspace-root join |
| valid_json/load_json/insert_lane helpers | 354-457 | REAL | S4 | string surgery (F-003); race-safe unique ids (documented) |
| wrong_schema_is_refused | 461-466 | REAL-UNPROVEN | — | asserts both schema strings |
| web_native_platform_is_refused | 471-479 | REAL-UNPROVEN | — | the config/2 platform rule |
| two_native_lanes_are_refused | 483-496 | REAL-UNPROVEN | — | exactly-one-native invariant |
| structural_rejections | 500-533 | REAL-UNPROVEN | — | empty lanes / dup ids / empty extensions |
| minimal_valid_config_loads | 537-549 | REAL-UNPROVEN | — | positive control incl. root resolution |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 5 | YES (if executed) | each drives the real load() on real temp files and asserts refusal texts / positive fields |
## 6. Doc claims: "the schema string is bumped because old configs (no platform) must be refused, not silently defaulted" (:10-17) — implemented at :291-297; REAL (read).
## 7. Dead code: none.
## 8. Dependencies: serde, serde_json, crate::ToolError — no engine crates (deliberate isolation).
## 9. Open questions: is there ANY recorded run of the standalone compat suite? (UNVERIFIED — none found in audit/runs/baseline/.)
