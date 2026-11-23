# AUDIT: tools/xtask/src/cmd/mod.rs
| Field | Value |
|---|---|
| Auditor | W01-A05 |
| Date | 2026-11-23 |
| Lines / functions | 263 / 8 (struct Subcommand, const SUBCOMMANDS, lookup, names + 4 tests) |
| File verdict | REAL (registry data table; unit tests executed 4/4) |
| Reachable from shipped entry point? | NO (dev tool) — but reachable from `cargo xtask <any>`: main.rs:124 `cmd::lookup(name)` -> `(sub.run)(&args[1..])`; verified by executing the compiled xtask test binaries directly |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/xtask-b619335e31412091 registry` -> 2 passed; same binary `--exact cmd::tests::all_wave1_subcommands_are_implemented cmd::tests::all_subcommands_are_implemented_invariant` -> 2 passed; grep coverage checks on tests/ (exit 0) |

## 1. One-paragraph verdict
This is a frozen-interface subcommand registry: a 17-entry const table, a
linear lookup, and two one-liner helpers. It contains no logic to get
wrong, it compiles (every `run:` function pointer resolves at compile
time), all four of its unit tests pass when executed, and the doc-comment
ownership map matches the array entry-for-entry (verified card by card).
The only defects are test-suite: two of the four tests have byte-for-byte
identical bodies (one is pure duplication), and the registry-presence test
covers only the 8 wave-1 names — 9 of 17 registered subcommands have no
presence assertion anywhere in tools/xtask/tests/, so silently deleting
e.g. the `sbom` entry from SUBCOMMANDS breaks no cargo test.

## 2. What actually works (verified)
— ran prebuilt unit-test binary (built 18:25, after the 17:35-era sources):
  filter `registry` -> "registry_contains_all_wave1_subcommands ... ok",
  "registry_names_are_unique ... ok"; `--exact` the two implemented-tests
  -> both ok. 4/4 of this file's tests pass by execution, not citation.
— cross-checked the L13-31 ownership map vs the L72-193 array: 17 modules,
  17 entries, cards match (gate 1.A3 ... web-budget 7.K1 (#49)); all 17
  `pub mod` declarations exist on disk (LS of tools/xtask/src/cmd) and
  compile (baseline: xtask built and ran feature-audit / parity-report /
  config-validator, all producing real output in audit/runs/baseline/).
— dispatch chain proven: main.rs:119-131 routes `name` -> `cmd::lookup` ->
  `(sub.run)(&args[1..])`; my config-validator test run (previous file)
  spawned this exact path 17 times successfully.

## 3. Findings
F-001 [S4]
Where: tools/xtask/src/cmd/mod.rs:238-249 and 251-262
Code:
```
    fn all_wave1_subcommands_are_implemented() {
        // Lead batch-flip at the 1.D4 merge flipped the wave-1 entries;
        ...
        for sub in SUBCOMMANDS {
            assert!(
                sub.implemented,
```
and
```
    fn all_subcommands_are_implemented_invariant() {
        // Graduated from `fixtures_stub_is_the_only_unimplemented` at the
        // 2.A3 merge: ...
        for sub in SUBCOMMANDS {
            assert!(sub.implemented, "`{}` should be implemented", sub.name);
        }
```
What is wrong: both tests iterate SUBCOMMANDS asserting `sub.implemented`
— identical coverage, identical failure condition. The second test's
comment even names the first as "the intended alarm", admitting the
redundancy. Why it matters: checklist 6.4/6.5 (duplicated test, dead
weight in a frozen file); a reader auditing "is everything implemented"
finds two answers that are the same answer. What correct looks like:
delete one (the file is frozen — fold into the next granted edit).
Confidence: HIGH — Verification: ran (both pass; mutating `implemented`
to false would fail both identically) + read.

F-002 [S4]
Where: tools/xtask/src/cmd/mod.rs:219-229 (vs the 17-entry table at L72-193)
Code:
```
    fn registry_contains_all_wave1_subcommands() {
        for expected in [
            "gate",
            "schema-gen",
            "tokens",
            "codegen",
            "check-architecture",
            "rebrand",
            "fixtures",
            "visual",
        ] {
```
What is wrong: the presence test asserts 8 of 17 names. The 9 later
subcommands (check-panics, feature-audit, config-validator, parity-report,
lint-literals, check-identity, sbom, compat-matrix, web-budget) have no
registry-presence assertion: grep of tools/xtask/tests/cli.rs for those
names -> 0 hits; per-command integration tests exist for only 5 of the 9
(check_architecture.rs, check_panics.rs, config_validator.rs,
feature_audit.rs, lint_literals.rs); parity-report, check-identity, sbom,
compat-matrix, web-budget have no test file at all. Deleting any of those
5 entries from SUBCOMMANDS fails no `cargo test -p xtask` run — only the
slower full_gate script (which invokes them) would notice. Why it matters:
the registry's stated design (L70-71 "every wave-1 consumer of xtask is
registered here from day one") makes presence an invariant, but only half
the table is pinned by tests. What correct looks like: extend the presence
loop to all 17 names.
Confidence: HIGH — Verification: ran (greps) + read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| struct Subcommand | 57-68 | REAL | — | 5 fields, all consumed (name/card/summary/implemented by help at main.rs:141-151, run by dispatch main.rs:125) |
| const SUBCOMMANDS | 72-193 | REAL | — | 17 entries; `implemented: true` for all — vestigial-by-design extension seam for future stubs; no lie (baseline shows all 17 modules exist and run) |
| lookup | 196-198 | REAL | — | linear find over 17 entries; exact match; cold path, no perf concern |
| names | 201-203 | REAL | — | Vec collect for usage/error strings; called from main.rs:128 |
| test registry_names_are_unique | 209-216 | REAL | — | genuine dup check; executed ok |
| test registry_contains_all_wave1_subcommands | 218-235 | REAL | — | 8/17 names only (F-002); executed ok |
| test all_wave1_subcommands_are_implemented | 237-249 | REAL | — | executed ok; duplicate of next (F-001) |
| test all_subcommands_are_implemented_invariant | 251-262 | REAL | — | executed ok; duplicate of previous (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| registry_names_are_unique | YES | duplicate name in table -> len mismatch |
| registry_contains_all_wave1_subcommands | YES (for 8 names) | dropped entry -> lookup None; the other 9 are unpinned (F-002) |
| all_wave1_subcommands_are_implemented | YES | implemented:false -> assert fires |
| all_subcommands_are_implemented_invariant | YES (same as above) | identical body; adds no distinct coverage (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "FROZEN after card 1.A3 ... do not edit this file" | L3-9 | MISLEADING-BUT-HONEST | the table itself documents 10+ post-freeze registrations, each with a cited Lead grant (L20-31) — the freeze is porous by documented exception, not silently violated |
| ownership map = registered subcommands | L13-31 | REAL | 17/17 cross-checked vs array cards (§2) |
| "every wave-1 consumer of xtask is registered here from day one" | L70-71 | REAL | 17 entries, all modules exist and compile |
| exit codes 0/1/2 contract | main.rs:16-23 (consumer of this table) | REAL | config-validator test run: exit 0/1/2 all observed (previous file's suite) |

## 7. Dead / duplicated / unreachable code
- `implemented` field: currently constant-true across 17 entries — dead
  data today, live seam for future stubs (documented L64, L252-258). Not
  counted as a finding.
- Duplicate test pair: F-001.
- `toml_lite` module (L52) is registered as pub mod but is a helper, not a
  subcommand — correctly absent from SUBCOMMANDS.

## 8. Dependencies & flags
std-only (`std::process::ExitCode`); zero external deps consistent with
Cargo.lock xtask entry (L1406-1408). No feature gates. `pub const` table +
fn pointers: no runtime registration, no reflection — compile-time-proven
wiring.

## 9. Open questions for the next agent
- parity-report / check-identity / sbom / compat-matrix / web-budget have
  NO test files under tools/xtask/tests/ (verified by LS) — their audits
  must rely on baseline runs (parity-report 0/252, full_gate sbom red).
- If the frozen-file rule ever relaxes, fold F-001's duplicate and extend
  F-002's presence list in the same edit.
