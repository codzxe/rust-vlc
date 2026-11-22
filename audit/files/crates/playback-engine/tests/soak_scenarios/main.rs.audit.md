# AUDIT: crates/playback-engine/tests/soak_scenarios/main.rs

| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-22 |
| Lines / functions | 606 / 4 `#[test]` + env/config resolvers + a ~200-line hand-rolled JSON syntax validator + `#[path]` module includes |
| File verdict | REAL (the suite root; CI-short profile runs in the plain merge gate — verified) |
| Reachable from shipped entry point? | N/A (test binary root). It mounts the process-global counting allocator and drives the real engine through the shared 3.E4 harness. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | Read (full file); LS `tests/soak/` (scenarios.toml, -nightly, -sandbox, -forced-threshold, scenarios.schema.json all present); Grep `DEFAULTS_KEYS\|SCENARIO_KEYS` (manifest.rs:522/:618 — the drift-check key lists exist); `grep -c "\"const\": 1"` on the schema → 1; cross-checks against my runner.rs audit (same binary). No cargo — W00 baseline cited. |

## 1. One-paragraph verdict

The root of the soak-scenario test binary, and the wiring is honest:
the CI-short profile really is the default (`manifest_path()` →
workspace `tests/soak/scenarios.toml`, verified present with 5
scenarios totaling ~1 min), so `soak_scenario_suite` runs as a plain
ungated `#[test]` inside `cargo test --workspace` — it RAN in the W00
baseline's 1751, hammering the real engine with the process-global
counting allocator mounted and all four tests serialized against
allocator pollution. The second test loads all four shipped manifests
through the strict loader and drift-checks the JSON schema file
against the loader's key lists (both verified to exist). The ~200-line
dependency-free JSON syntax validator is real and unit-tested both
directions. One test-honesty defect: `only_scenario_filter_shapes_refuse_loudly`
never exercises a refusal — its name and doc promise two loud refusals
(set-but-empty filter, unknown id), but the body only covers the happy
paths the ambient environment happens to select; in the W00 baseline
(env unset) it asserted one trivial equality. And the "drift check" is
substring containment, not schema-aware — a key named in a description
satisfies it (an admitted-by-design weakness).

## 2. What actually works (verified)

— `soak_scenario_suite` (:184-245): loads the default CI-short manifest (present on disk, 5 scenarios: play-loop 0.25 min, seek-storm 0.2, playlist-cycle 0.2, open-close-churn 30 cycles, reopen-stress 20 cycles), runs each through `run_scenario` (my runner.rs audit: real engine, real fixtures), prints the markdown report, and asserts the collected violation list is empty WITH every sentence in the failure text — a red suite is a finding, not noise. Ran in W00.
— `#[global_allocator] CountingGlobalAlloc` (:74-75) + `serialize_tests()` in every test (:186, :260, :541, :585) — the process-global instrument cannot be polluted by sibling test threads.
— `shipped_manifests_validate_against_the_schema_contract` (:258-317): all FOUR shipped profiles (CI-short, nightly, sandbox, forced-threshold — all present on disk) must load through the strict loader (unknown keys/wrong types/out-of-range/duplicate ids are refusals per manifest.rs, W03-A04's audit), the schema file must be syntactically valid JSON, must name every loader key (DEFAULTS_KEYS ×10, SCENARIO_KEYS ×8 — verified at manifest.rs:522/:618), every kind, and the pinned `"const": 1` (verified present in the schema).
— The JSON validator (:327-522): a real RFC-8259 grammar checker (objects, arrays, strings with all six structural escapes + \uXXXX + control-char rejection, numbers with the `-?(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?` shape), unit-tested with 10 accepted and 11 rejected shapes (:539-581) — including `[1,]`, `{"a"}`, `\q`, raw newlines, `1 2`, `{,}`. No dependency (C5 purity), honest scope note (:324-326: containment checks by design, no data extraction).
— Env resolution is loud, never defaulting: an empty `SOAK_SCENARIOS`/`SOAK_SCENARIOS_REPORT`/`SOAK_SCENARIOS_ONLY` value is handled explicitly; an unknown `SOAK_SCENARIOS_ONLY` id refuses with the legal ids listed (:162-172).
— The `#[path]`-included harness modules (dispatch/memory/scenario, :52-60) share the 3.E4 codebase verbatim with the `soak` binary — the `#[allow(dead_code)]` is justified and documented (this driver uses a subset of the shared API; the shared files must not grow this binary's private items).

## 3. Findings

### F-001 [S3] The filter test's name promises "refuse loudly" but no refusal path is ever exercised

Where: crates/playback-engine/tests/soak_scenarios/main.rs:583-605
Code:
```rust
    #[test]
    fn only_scenario_filter_shapes_refuse_loudly() {
        let _serial = serial();
        // The filter contract is env-driven and process-global: the
        // selection over the real default manifest is consistent with
        // whatever the environment asks for (unset = all — the CI and
        // nightly posture).
        let root = testkit::workspace_root().must();
        let manifest = manifest::load(&root.join("tests/soak/scenarios.toml"))
            .unwrap_or_else(|error| unreachable!("default manifest: {error}"));
        let selected = select_scenarios(&manifest).must();
        match only_scenario().must() {
```
What is wrong: the test never sets `SOAK_SCENARIOS_ONLY`, so it only ever observes the ambient environment. In the W00 baseline (env unset) it executed exactly one trivial assertion: `selected.len() == manifest.scenarios.len()` (:595-599). The two refusal contracts the test's name advertises — set-but-empty (`only_scenario` :139-148) and unknown id (`select_scenarios` :162-172) — have ZERO coverage: no test in the repo drives them (env mutation from a parallel test thread is the understandable reason, but then the test should not be named after the refusals). If a refactor made the empty filter silently mean "all" (a dirty-world default — exactly what the suite's own discipline forbids), nothing would fail.
Why it matters: a named loud-refusal contract with no test is precisely the §3.4 "test names that promise more than the body checks" shape; the sandbox chunking knob's safety depends on these refusals.
What correct looks like: extract the filter logic into a pure fn taking `Option<&str>` and unit-test the three branches directly (the env read stays a one-line wrapper); or spawn a child process with the env set.
Confidence: HIGH — Verification: read (whole test body; no `set_var` anywhere in the file — grep).

### F-002 [S4] The loader/schema "drift check" is substring containment

Where: crates/playback-engine/tests/soak_scenarios/main.rs:270-293
Code:
```rust
    for key in manifest::DEFAULTS_KEYS {
        assert!(
            schema.contains(&format!("\"{key}\"")),
            "the schema does not name the [defaults] key {key:?} — loader/schema drift"
        );
    }
```
What is wrong: the check proves the string `"key"` appears SOMEWHERE in the schema file — in a description, an example, or a `$comment` — not that the schema actually constrains that key. A schema edit that deletes a property but leaves the key's name in a doc comment passes; a schema that constrains a key under the wrong object also passes. The doc comment (:251-257) sells this as "loader and schema move together or not at all", which the mechanism cannot deliver (the :324-326 note admits "containment checks by design" — the strength mismatch is between the two claims).
Why it matters: the drift guard is the only thing keeping the JSON-Schema contract honest for external tooling; substring matching is a weak oracle for that job.
What correct looks like: parse the schema (the validator already exists — extend it to extract property names, or do a structural walk) and assert the key sets are equal in both directions (schema keys the loader rejects, loader keys absent from the schema).
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module includes dispatch/memory/scenario | 52-60 | REAL | — | shared verbatim via `#[path]`; `#[allow(dead_code)]` justified + documented |
| `ALLOC`/`live_bytes` | 74-81 | REAL | — | the process-global instrument + a non-allocating reader |
| `manifest_path` | 93-108 | REAL | — | default = workspace `tests/soak/scenarios.toml` (present); loud on unusable values |
| `report_path` | 115-127 | REAL | — | workspace-root resolution (CWD note honest) |
| `only_scenario` | 134-149 | REAL | S3 | refusal branches uncovered (F-001) |
| `select_scenarios` | 157-174 | REAL | S3 | unknown-id refusal uncovered (F-001); refusal text lists legal ids |
| test `soak_scenario_suite` | 184-245 | REAL | — | the CI-short merge-gate run; asserts failures empty with all sentences |
| test `shipped_manifests_validate_against_the_schema_contract` | 258-317 | REAL | S4 | four profiles load; containment drift check (F-002); `"const": 1` pinned |
| `json_is_syntactically_valid` + 7 parse fns | 327-522 | REAL | — | real RFC-8259 grammar; recursion depth unbounded (schema is a trusted committed file — acceptable) |
| test `json_validator_accepts_and_rejects_real_shapes` | 539-581 | REAL | — | 10 accept / 11 reject shapes |
| test `only_scenario_filter_shapes_refuse_loudly` | 583-605 | SHALLOW | S3 | F-001: happy-path-only, environment-dependent |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| soak_scenario_suite | YES | violations asserted empty; harness breaks fail fast (`unreachable!` with sentences) — modulo the drain hole in runner.rs F-001 (a wedged cycle passes) |
| shipped_manifests_validate... | YES for loader strictness and schema presence/syntax; WEAK for actual schema-key drift | F-002 containment |
| json_validator_accepts_and_rejects... | YES | concrete accept/reject shapes both directions |
| only_scenario_filter_shapes_refuse_loudly | NO for the advertised refusals; trivially yes for the unset path | F-001 |

All four ran in the W00 baseline (ungated; env unset there → default CI-short manifest).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "CI-short (merge gate) … who runs it: `cargo test --workspace`" | header table :13-17 | REAL | ungated `#[test]` + default manifest present; W00 baseline ran it |
| "Parse/refusal errors fail loudly, never default" | header :23 | REAL for parse; UNTESTED for the filter refusals | manifest load → `unreachable!` (:188-189); but F-001: the two filter refusals have no test |
| "shared verbatim, not forked" (the 3.E4 modules) | header :32-34 | REAL | `#[path]` includes of the same files — though runner.rs still forks two helpers (my runner.rs F-003) |
| "loader and schema move together or not at all" | :255-257 | SHALLOW | substring containment cannot detect several drift shapes (F-002) |

## 7. Dead / duplicated / unreachable code

- None dead in this file. The `#[allow(dead_code)]` trio is justified by the shared-module design.

## 8. Dependencies & flags

No feature flags. Env knobs (`SOAK_SCENARIOS`, `SOAK_SCENARIOS_REPORT`, `SOAK_SCENARIOS_ONLY`) select/diagnose — defaults run the CI-short profile in every plain test invocation, which is why the W00 baseline absorbed a ~1-minute soak into its 1751. Nightly tier wired at .github/workflows/soak-nightly.yml:264 (env-gated, not in the baseline).

## 9. Open questions for the next agent

- F-001's uncovered refusals: if the claim ledger cites "loud refusal on bad soak config", the evidence is code-read only, not a passing test.
- The suite's default runtime (~1 min) is part of every `cargo test --workspace` run — worth confirming the baseline log's total wall time accounted for it (baseline run log, audit/runs/baseline/).
