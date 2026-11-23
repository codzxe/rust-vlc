# AUDIT: tools/xtask/tests/check_architecture.rs
| Field | Value |
|---|---|
| Auditor | W01-A08 |
| Date | 2026-11-23 |
| Lines / functions | 619 / 18 (5 helpers + 2 fixture builders + 11 #[test]) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test file). Runs under `cargo test -p xtask` via CARGO_BIN_EXE_xtask; verified executed in audit/runs/baseline/cargo-test-workspace.txt:6753-6764 (11/11 ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | rg over baseline cargo-test-workspace.txt (11/11 integration tests ok, lines 6753-6764); sed verifications; rg dev_deps/dev-dependencies in tools/xtask/src/cmd/check_architecture.rs (graph merges normal+dev edges, lines 1552/1562) |

## 1. One-paragraph verdict
This is what acceptance tests should look like: every test builds a throwaway scratch workspace under the system temp dir, spawns the REAL xtask binary (`env!("CARGO_BIN_EXE_xtask")`) with `--root <scratch>`, seeds exactly one violation class, and asserts the exact diagnostic needle plus the exact exit code (0 clean / 1 violation / 2 usage). The tests never touch the checked-out tree, and the clean fixture plants decoys ("static mut" in a comment and a string literal) to prove the scanner strips non-code. One documented scenario is a lie, though: `dev_edge_cycle_is_caught` claims to seed "a dev-edge dependency cycle (alpha dev-> beta, beta -> alpha) — the class cargo itself will not reject", but the fixture it writes puts alpha's beta edge in `[dependencies]` (a NORMAL cycle, which cargo does reject at build time); no dev-edge participates in the seeded cycle. Cycle detection itself is still genuinely proven, and the impl's own unit test covers a dev-edge-shaped adjacency map, so this is a misleading test name/comment, not a coverage hole for the algorithm.

## 2. What actually works (verified)
— verified by citation of audit/runs/baseline/cargo-test-workspace.txt:6753-6764: all 11 tests pass (`clean_run_writes_the_json_report_artifact`, `clean_scratch_workspace_passes_all_five_checks`, `dev_edge_cycle_is_caught`, `forbidden_internal_edge_is_caught`, `static_mut_declaration_is_caught`, `testkit_in_dependencies_is_caught`, `unapproved_external_dependency_is_caught`, `undeclared_and_stale_manifest_entries_are_caught`, `unlisted_same_layer_edge_is_caught`, `upward_layer_edge_is_caught`, `usage_errors_exit_two`).
— verified by citation of baseline full_gate.txt:41-52: the real-workspace run of the same checker FAILED with 1 [EDGE] violation ("`lumen-cli` depends on `streaming`, which is not in [crates.lumen-cli].internal_deps") — the gate is not a rubber stamp on the real tree.
— verified by read: fixtures write only under `std::env::temp_dir()` (L50-51) and the `--json` artifact is asserted under `fx.root.join(...)` (L222), never the repo.

## 3. Findings
F-001 [S3]
Where: tools/xtask/tests/check_architecture.rs:320-327 vs 345-360
Code:
```
/// Seeded violation (b): a dev-edge dependency cycle (alpha dev-> beta,
/// beta -> alpha) — the class cargo itself will not reject.
```
```
    // beta now depends on alpha; alpha dev-depends on beta. Both edges are
    // allowlisted, and no layers are assigned, so ONLY the cycle fires.
```
What is wrong: The comment says "alpha dev-depends on beta", but the alpha manifest this test writes declares `beta = { path = "../beta" }` under `[dependencies]` (L354-355); the only `[dev-dependencies]` are `testkit` and `beta2`. The seeded cycle is therefore alpha ->(normal) beta ->(normal) alpha — a cycle cargo itself rejects at build time, i.e. NOT "the class cargo itself will not reject". The boundaries' `dev_deps = ["beta", ...]` entry (L375) is an unused permission, not an edge. What the test proves: the checker reads manifests without cargo and catches a normal cycle. What it does NOT prove: catching a dev-edge cycle end-to-end through the manifest layer (that is covered only by the impl's adjacency-map unit test `cycle_detection_finds_dev_edge_cycles_and_self_loops`, which feeds a hand-built map, not manifests). Why it matters: the header comment (L11-13) explicitly sells the dev-edge cycle as the fixture's raison d'être ("the dev-edge cycle fixture, for one, is a graph cargo would resolve but never reject") — the shipped fixture is not that graph. What correct looks like: move `beta = { path = "../beta" }` into `[dev-dependencies]` so the comment, the name, and the fixture agree. Confidence: HIGH — Verification: read (fixture text traced; baseline cited for pass status).

F-002 [S4]
Where: tools/xtask/tests/check_architecture.rs:207-213
Code:
```
    // The decoys prove no false positives: the static-mut check passed
    // although the source mentions the forbidden words twice.
    assert!(
        out.stdout.contains("static-mut") && out.stdout.contains("PASS"),
```
What is wrong: This assertion is satisfied by ANY line containing "PASS" anywhere in stdout plus the literal "static-mut" anywhere else — it proves nothing on its own (e.g. `edges=pass` and the word "static-mut" in an error would pass it). It is rescued only by the stronger machine-line assertion at L201-206 ("static-mut=pass" exact). Why it matters: redundant weak assertions rot into false confidence when copy-pasted. What correct looks like: assert the `static-mut..........PASS` row shape or delete. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: tools/xtask/tests/check_architecture.rs:116-144 (CLEAN_BOUNDARIES) + whole file
Code:
```
[rules]
same_layer_pairs = []
dev_only = [\"testkit\"]
static_mut_allow = []
```
What is wrong: Coverage gaps in an otherwise thorough suite: (a) the positive paths of `same_layer_pairs` and `static_mut_allow` are never exercised (no test seeds an allowlisted same-layer pair or a `static_mut_allow` exemption and asserts PASS); (b) no test seeds an unapproved external DEV dependency (`external_dev_allowlist` negative case — the cycle test only uses its positive side for `beta2`); (c) no test feeds a bad `[manifest] version`. Why it matters: allowlist exemption logic is exactly where checkers grow silent false negatives. What correct looks like: one passing-fixture per exemption knob. Confidence: HIGH — Verification: read (exhaustive test inventory above).

F-004 [S4]
Where: tools/xtask/tests/check_architecture.rs:49-61
Code:
```
        let root =
            std::env::temp_dir().join(format!("lumen-arch-fixture-{}-{name}", std::process::id()));
```
What is wrong: Fixture identity is (pid, test-name). Two concurrent runs of the same test binary from different processes have different pids, and one process runs each test name once — safe today — but a rerun racing a hung prior process, or a second test added with a colliding name, silently reuses/removes the other's directory (`remove_dir_all` at L52 runs BEFORE any liveness check). Test-only blast radius. What correct looks like: append a counter or use a tempfile handle. Confidence: LOW — Verification: read (reasoned, not triggered).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run_checker | 25-41 | REAL | — | spawns CARGO_BIN_EXE_xtask, captures code/stdout/stderr |
| Fixture::new | 49-62 | REAL | S4 | F-004 collision window |
| Fixture::write | 64-76 | REAL | — | mkdir -p + write, unreachable! on IO |
| Fixture::run | 78-80 | REAL | — | delegates to run_checker |
| Drop for Fixture | 83-88 | REAL | — | best-effort cleanup, documented |
| clean_crate | 100-112 | REAL | — | lints-inheriting member manifest |
| build_clean_fixture | 149-182 | REAL | — | 3 crates + decoys incl. static-mut-in-comment/string |
| clean_scratch_workspace_passes_all_five_checks | 184-214 | REAL | S4 | strong machine-line assert; weak redundant assert F-002 |
| clean_run_writes_the_json_report_artifact | 216-238 | REAL | — | --json writes under scratch root, echoes report |
| forbidden_internal_edge_is_caught | 242-275 | REAL | — | exact [EDGE] needle + exit 1 |
| upward_layer_edge_is_caught | 279-297 | REAL | — | "points upward (layer 0 -> 1)" |
| unlisted_same_layer_edge_is_caught | 301-318 | REAL | — | same-layer pair not whitelisted |
| dev_edge_cycle_is_caught | 322-408 | REAL | S3 | F-001: comment/name describe a dev-edge cycle; fixture seeds a normal cycle |
| missing_lints_inheritance_is_caught | 412-447 | REAL | — | both needles (member + root), asserts ONLY that check fails |
| static_mut_declaration_is_caught | 452-473 | REAL | — | exact file:line needle (lib.rs:3) |
| testkit_in_dependencies_is_caught | 478-520 | REAL | — | proves dev-only rule overrides allowlist |
| undeclared_and_stale_manifest_entries_are_caught | 524-575 | REAL | — | both directions of manifest integrity |
| unapproved_external_dependency_is_caught | 578-598 | REAL | — | [EXTERNAL] needle |
| usage_errors_exit_two | 601-619 | REAL | — | exit-2 contract for usage errors |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| clean_scratch_workspace_passes_all_five_checks | YES | exit 0 + "PASS (5 of 5)" + exact machine line; decoys fail-closed if stripper breaks (would produce violations -> exit 1) |
| clean_run_writes_the_json_report_artifact | YES | artifact must exist with schema+result strings; unreachable! if missing |
| forbidden_internal_edge_is_caught | YES | exact edge diagnostic needle |
| upward_layer_edge_is_caught | YES | exact layer diagnostic |
| unlisted_same_layer_edge_is_caught | YES | exact same-layer needle |
| dev_edge_cycle_is_caught | YES (for cycles) | [CYCLE] + path render + "1 of 5 failed" — but proves the NORMAL cycle only (F-001) |
| missing_lints_inheritance_is_caught | YES | both lints needles + only-that-check-failed |
| static_mut_declaration_is_caught | YES | exact file:line |
| testkit_in_dependencies_is_caught | YES | dev-only needle despite allowlisted edge |
| undeclared_and_stale_manifest_entries_are_caught | YES | both integrity needles |
| unapproved_external_dependency_is_caught | YES | [EXTERNAL] needle |
| usage_errors_exit_two | YES | exit 2 + exact stderr needles |
No tautologies, no #[ignore], no mocks of the thing tested (the real binary is spawned). Gaps listed in F-003.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "runs the real xtask binary with --root <scratch>" | header L6-7 | REAL | L26-28 env!("CARGO_BIN_EXE_xtask") |
| "Nothing here touches the checked-in tree" | header L8-9 | REAL | all writes under std::env::temp_dir() (L51, L64-76, L222) |
| "the dev-edge cycle fixture ... is a graph cargo would resolve but never reject" | header L11-13 | FALSE for the shipped fixture | F-001: alpha's beta edge is in [dependencies] (L354-355) — cargo rejects that cycle |
| "seeded violations in scratch crates are all caught" (card 1.E1 AC) | header L1-2 | REAL | 10 violation tests, all pass (baseline 6753-6764) |

## 7. Dead / duplicated / unreachable code
None. Every helper is used; no #[ignore]; no dead constants.

## 8. Dependencies & flags
std-only (fs, path, process, env). Uses cargo-provided `CARGO_BIN_EXE_xtask` — the correct mechanism for integration-testing a binary crate. No features involved.

## 9. Open questions for the next agent
- None blocking. If someone audits tools/xtask/src/cmd/check_architecture.rs itself, cross-check F-001: does the manifest-to-adjacency builder classify [dev-dependencies] edges separately at all (impl L1552 says "normal + dev" merged)?
