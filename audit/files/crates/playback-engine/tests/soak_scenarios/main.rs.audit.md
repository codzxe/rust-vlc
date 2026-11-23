# AUDIT: crates/playback-engine/tests/soak_scenarios/main.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-23 |
| Lines / functions | 606 / 17 (2 suite tests + 10 pure helpers + hand-rolled JSON validator + 2 config tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test binary). Runs in the plain `cargo test --workspace` merge gate (baseline cargo-test-workspace.txt:4502-4534: 32 tests, 39.22s, all ok). Drives the real engine stack through `lumen_soak_dispatch`/`SoakHost` (runner.rs:261) — the same command-layer seam the CLI uses. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p playback-engine --test soak_scenarios --release soak_scenario_suite -- --nocapture` -> 1 passed, `PASS — every scenario stayed inside its configured liveness and memory budgets`, 39.03s (exit 0); baseline citation above |

## 1. One-paragraph verdict
A real, config-driven soak harness, not a ritual: the single suite test runs the five scenario kinds (play-loop, seek-storm, playlist-cycle, open-close-churn, reopen-stress) from a TOML manifest against the real engine through the command-layer seam, samples RSS/live-bytes/fd counts on a counting global allocator, records (not throws) budget/liveness violations per scenario for triage-complete reports, then asserts the violation list empty. I ran the CI-short profile live: PASS in 39 s with genuine measurements (reopen-stress: 20 cycles, 17 measured, fd 4→4, VmRSS 1868 kB→1868 kB, live bytes 13669→13669, 220 frames / 13.8 MB decoded at the sinks). The manifest loader enforces a Rust-side schema contract with loud refusals and a shipped-profiles validation test including loader/schema drift checks. Defects are hygiene-level: the env-filter's refusal paths are documented but never tested, and the validator hand-rolls 196 lines of JSON parsing that duplicates what the manifest loader's own TOML discipline already guarantees for the artifact it actually gates.

## 2. What actually works (verified)
— verified by `cargo test -p playback-engine --test soak_scenarios --release soak_scenario_suite -- --nocapture` -> exit 0, full per-scenario report printed (cycles, ops, decoding frames/payload bytes, fd/RSS/live-bytes budgets, slope verdicts), final line `PASS — every scenario stayed inside its configured liveness and memory budgets`.
— verified by baseline cargo-test-workspace.txt:4502-4534 (debug lane, 32 tests incl. manifest refusals, memory slope math, report dates: all ok, 39.22s) — the suite is wired into the merge gate, not a skipped/ignored lane.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/soak_scenarios/main.rs:583-605
Code:
```
    fn only_scenario_filter_shapes_refuse_loudly() {
        let _serial = serial();
        // The filter contract is env-driven and process-global: the
        // selection over the real default manifest is consistent with
        // whatever the environment asks for (unset = all — the CI and
        // nightly posture).
```
What is wrong: The test name promises "refuse loudly" but the body never exercises a refusal: it only checks that selection is *consistent* with the ambient environment (None → all, Some → 1). Neither the set-but-empty refusal (lines 139-145) nor the set-but-unknown-id refusal with the legal-ids sentence (lines 162-172) has any test — the doc's "A set-but-unknown id is a loud refusal" (line 25) is untested. rg over the binary's tests confirms no other caller covers `select_scenarios`' Err arm.
Why it matters: The sandbox chunking knob's failure mode (typo'd id → silently measuring nothing?) is exactly the path the doc claims is loud; if a future edit flips the `.find` to a defaulting `.next()`, no test fails.
What correct looks like: Factor the filter decision into a pure fn (env string → Result) and table-test both refusal sentences.
Confidence: HIGH — Verification: read + rg (no coverage exists).

F-002 [S4]
Where: crates/playback-engine/tests/soak_scenarios/main.rs:319-522
Code:
```
// ---------------------------------------------------------------------------
// A minimal JSON syntax validator (test-only; no dependency)
// ---------------------------------------------------------------------------
```
What is wrong: 196 lines of hand-rolled RFC 8259 parser (skip_ws/parse_value/object/array/string/literal/number) to answer one question — "is the schema file syntactically valid JSON" — while the *semantic* drift checks are substring containments (`schema.contains(&format!("\"{key}\""))`, line 271-275). The containment approach means a key named inside a description string would satisfy the drift check; the validator cannot catch that (by its own admission, "No data extraction"). Not wrong, but the strongest guarantees this machinery can give are weaker than they look: it proves parseability, and nothing about key placement.
Why it matters: Maintenance surface (a second parser in the tree to keep correct) for a guarantee the file's own consumers (editors, CI JSON tooling) mostly already give; the drift check's real teeth are the Rust loader, which IS tested (manifest.rs refusal tests).
What correct looks like: Either serde_json in dev-deps for a real structural check (keys at their schema paths), or drop the validator and rely on the loader + a JSON Schema validator invocation.
Confidence: HIGH — Verification: read; validator has its own accept/reject table test (passes in baseline).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mod dispatch/memory/scenario `#[path]` includes | 52-60 | REAL | — | Shared verbatim from tests/soak/ (not forked); `#[allow(dead_code)]` for the unused subset — documented, deliberate; lineage recorded in runner docs. |
| live_bytes | 79-81 | REAL | — | Counting-allocator snapshot reader; process-global instrument (ED-FIX-2). |
| manifest_path | 93-108 | REAL | — | Env → path; set-but-empty falls to default (documented: empty = unset); absolute or workspace-relative. |
| report_path | 115-127 | REAL | — | Same resolution contract; write failures are loud. |
| only_scenario | 134-149 | REAL | S4 | Empty-value refusal untested (F-001). |
| select_scenarios | 157-174 | REAL | S4 | Unknown-id refusal untested (F-001). |
| soak_scenario_suite | 184-245 | REAL | — | The suite: manifest → scenarios → report → assert violations empty; harness breaks fail fast via unreachable!; ran live PASS. |
| shipped_manifests_validate_against_the_schema_contract | 258-317 | REAL | — | All four shipped profiles load; schema names every loader key/kind; shape-version const pinned; empty-profile-is-a-ritual check (D-6). |
| json_is_syntactically_valid + 8 parse helpers | 327-522 | REAL | S4 | Correct-looking RFC 8259 syntax parser (strict control chars, six escapes + \uXXXX, number grammar); F-002 on cost/benefit. |
| tests::serial | 535-537 | REAL | — | serialize_tests() wrapper (allocator discipline). |
| json_validator_accepts_and_rejects_real_shapes | 539-581 | REAL | — | 10 accept / 11 reject shapes — real positive+negative table. |
| only_scenario_filter_shapes_refuse_loudly | 583-605 | SHALLOW | S4 | F-001: consistency-only, no refusal exercised. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| soak_scenario_suite | YES | Real engine driven end-to-end; any budget/liveness violation (leak slope > ceiling, skip rate, min cycles, fd/RSS/live growth) fails the final assert — verified by the live run's per-scenario budget lines. A harness break (refused command, missing terminal) fails fast. |
| shipped_manifests_validate… | YES | Loads all 4 profiles through the strict loader; loader/schema key drift fails containment. Partially blunt (substring containment — see F-002) but the loader refusals are independently unit-tested (manifest::tests, 14 tests, baseline). |
| json_validator_accepts_and_rejects… | YES | Real accept/reject corpus incl. trailing junk, bad escapes, raw control chars. |
| only_scenario_filter_shapes_refuse_loudly | NO (for the refusal paths) | Only checks consistency with ambient env; both refusal arms unexecuted (F-001). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Budget and liveness violations are RECORDED per scenario … and asserted empty at the end" | header, lines 37-40 | REAL | Code at 233-244; live run printed per-scenario budgets then PASS. |
| "three FILES, not three code paths" (CI/nightly/sandbox profiles) | header, lines 7-17 | REAL | scenarios{,-nightly,-sandbox}.toml all exist and load (validation test). |
| "A set-but-unknown id is a loud refusal" | header, line 25 | REAL-UNPROVEN | Code refuses (162-172) but no test drives it — F-001. |
| "the counting allocator is process-global: every test … takes serialize_tests() first" | header, lines 29-31 | REAL | Both suite tests + both config tests take the guard (lines 186, 260, 541, 585). |
| "the same command-layer seam (lumen_soak_dispatch over a SoakHost)" | header, lines 3-4 | REAL | runner.rs:261 calls it per op; live run shows real decoding counters. |

## 7. Dead / duplicated / unreachable code
- The `#[allow(dead_code)]` on the three `#[path]` includes hides the unused subset of the shared soak harness API — a documented, bounded allow (the shared file must not grow this binary's private items); noted, not counted.
- No dead items inside this file; every helper is used by the two suite tests or the two config tests.

## 8. Dependencies & flags
- testkit (Must, alloc_count::{CountingGlobalAlloc, serialize_tests}, workspace_root), the shared ../soak modules, no new external deps (the JSON validator exists precisely to avoid one). No feature gates; runs in the plain test lane (baseline + my live run).

## 9. Open questions for the next agent
- runner.rs (1414 loc) and manifest.rs (974 loc) carry the real measurement logic and loader refusals — assigned elsewhere? If unassigned, they are the load-bearing half of this suite; my audit covered the driver file only.
- docs/soak/ reports are cited as the freeze-or-revise decision home for `target_mb_per_h = 5.0` [PROPOSAL] — the nightly tier is the only place it is asserted; whether the nightly workflow actually runs (`.github/workflows/soak-nightly.yml`) is a CI-audit question.
