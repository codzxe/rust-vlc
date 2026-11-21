# AUDIT: crates/localization/src/audit.rs
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-21 |
| Lines / functions | 514 (wc -l) / 13 non-test items + 6 tests |
| File verdict | UNREACHABLE (real, tested code; zero dependents; no shipped entry point reaches it; baseline runs never executed it) |
| Reachable from shipped entry point? | NO — no crate in the repo depends on `localization` (rg over every Cargo.toml: only self-references). Only callers are the crate's own tests (`tests/seed_integrity.rs:27,39`, `tests/audit_proof_matrix.rs`) and CI's `cargo test -p localization` (`.github/workflows/ci.yml:438`) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 3 |
| Commands run | rg "audit_catalogs\|catalog_error_issue" across repo (callers: tests only); rg "l10n" in tools/xtask/src + .github/ (absent); ls tools/xtask/src/cmd (17 commands, no l10n-audit); rg "localization" in **/Cargo.toml (no dependents); rg description/msgid counts in schemas/settings.toml (121) vs locales/en-US.toml (14 keys, 0 `settings.*`); rg seed_integrity/audit_proof_matrix in audit/runs/baseline/test-workspace*.log.txt (0 matches) and in orchrun-test-workspace-nofailfast.log.txt (ran, passed); sed -n verification of every quote below |

## 1. One-paragraph verdict
The five catalog-audit rules are genuinely implemented, internally consistent, honestly documented, and now VERIFIED by execution: the orchestrator repair run (audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2740-2766) ran `audit_proof_matrix` (14/14 ok) and `seed_integrity` (3/3 ok, shipped tree clean). But the engine is a mechanism without a consumer: nothing depends on the `localization` crate, no shipped binary reaches any function here, the documented CI command (`cargo xtask l10n-audit`) does not exist, and the documented rule-4 join covers only 2 of 6 declared config sources in practice — if run over `schemas/settings.toml` as doc 07 §6.4 demands, it would emit **121 UnresolvedReference issues today**. The same repair run also exposed a profile-coupled failing lib test in the crate (F-007).

## 2. What actually works (verified)
- Implementation of rules 1–5 traced against `plural.rs::categories_for_language` (real CLDR seed table), `message.rs::placeholder_set`/`plural_arms_by_name` (lines 291/299), `catalog.rs::load_locales_dir`/`keys`/`contains_key`, `refs.rs::MSGID_FIELDS` — all present and real; the audit functions call them correctly.
- Test suite structure verified by reading `tests/audit_proof_matrix.rs` (369 lines) + all 8 fixture directories exist on disk (locales-{good,missing,duplicate,orphan,placeholder,plural-ru,plural-ja,plural-unknown}, refs/{resolved,unresolved}.toml) — the proof matrix tests assert exact issue variants, keys, and message substrings in both directions.
- Verified by execution (baseline evidence): audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt — `Running tests/audit_proof_matrix.rs` → "running 14 tests … all ok" (lines 2740-2758) and `Running tests/seed_integrity.rs` → "running 3 tests" with `shipped_locales_tree_passes_every_catalog_audit_rule ... ok` and `product_toml_msgids_resolve_in_the_reference_catalog ... ok` (lines 2760-2766). The shipped locales tree passes every rule; the doc 07 §6.4 rule machinery fires correctly on all 8 violation fixtures.

## 3. Findings

### F-001 [S2]
Where: crates/localization/src/audit.rs:206-218 (scope), evidence in tests/seed_integrity.rs:37-38 and schemas/settings.toml
Code:
```rust
/// Rule 4 — Reference-completeness: every msgid referenced from the
/// declarative configs must resolve to a reference-catalog key
/// (doc 07 §6.4). References come from [`crate::refs`].
```
and (the actual shipped-tree wiring, tests/seed_integrity.rs:37-38):
```rust
    let refs = extract_msgid_refs_from_file(&repo("product.toml"))?.unwrap_or_default();
    assert_eq!(refs.len(), 2, "product.toml msgid refs: {refs:?}");
```
What is wrong: Rule 4 is declared (doc 07 §6.4 rule 4) over `product.toml`, `schemas/settings.toml`, `layouts/*`, `keymaps/*`, `profiles/*`, `themes/*` — but the only shipped-tree enforcement scans `product.toml` alone (asserts exactly 2 refs). `schemas/settings.toml` declares **121** `description = "settings.…"` msgids (rg count = 121, e.g. line 66 `description = "settings.global.interface-language"`); `locales/en-US.toml` contains **zero** `settings.*` keys (14 keys total: product.description, window.title.idle, 11 xtask.*, ci.job.stub-suffix — verified by rg '^"' locales/en-US.toml). `config-validator` also checks only the two product.toml msgids (tools/xtask/src/cmd/config_validator.rs:487).
Why it matters: 121 declared msgids are consumed by nothing and validated by nothing — the exact Wave-11 "declared but never read" defect. en-US.toml's own header promises "When a surface lands, its keys land WITH it (rule 4 then enforces the join)" — the settings surface HAS landed (121 entries; generated artifacts committed under schemas/generated/) and the keys did not land, and the join is not enforced. If `audit_catalogs_with_refs` were run with refs extracted from schemas/settings.toml today, it would return 121 `UnresolvedReference` issues. Doc 07 §6.3's "en-US … complete, source of truth" is contradicted for the entire settings plane.
What correct looks like: either seed the 121 `settings.*` keys into locales/en-US.toml and extend seed_integrity (or the l10n gate) to extract refs from schemas/settings.toml, keymaps/, profiles/, themes/ — or mark the settings msgids as not-yet-contract in schema-gen until the catalog side lands. As shipped, the declared join is fiction.
Confidence: HIGH — Verification: read (rg counts) + traced extractor grammar: `settings.global.interface-language` matches `MessageKey` grammar, so `extract_msgid_refs` would flag it.

### F-002 [S3]
Where: crates/localization/src/audit.rs:15-16 (disclosure), docs/07-config-driven-system.md:678,846 (claim), .github/workflows/ci.yml:468 (admission)
Code:
```rust
//! CI wiring (`cargo xtask l10n-audit`, doc 07 §6.4) is card 1.E2; until
//! then this crate's test suite runs the same audits on the shipped tree.
```
vs docs/07-config-driven-system.md:678: `Run by \`cargo xtask l10n-audit\` (backed by \`crates/localization::audit_catalogs\`, doc 01 §2.3):` and §9 job sequence step 6 `cargo xtask l10n-audit (C3)` — "merge-blocking".
vs ci.yml:468: `echo "COVERED C3 (partial): cargo test -p localization above (shipped-tree audit; the l10n-audit xtask subcommand itself does not exist)."`
What is wrong: The documented merge-blocking gate command does not exist. tools/xtask/src/cmd/mod.rs:72-193 registers 17 subcommands; none is `l10n-audit` (verified by ls + rg "l10n" tools/xtask/src → 0 hits). The xtask registry is declared FROZEN, so the wiring cannot land without Lead-granted ownership.
Why it matters: Doc 07 §9 presents an 11-step merge-blocking chain in which step 6 is this audit; consumers of the doc (and any auditor trusting it) believe C3 is gated. Reality: partial coverage via `cargo test -p localization` (ci.yml:438) — which itself only enforces rules 1/2/3/5 over a 1-file locales/ dir and rule 4 over product.toml only (see F-001).
What correct looks like: Either implement the subcommand (one dispatch entry + a call to `audit_catalogs_with_refs` over all §6.4 rule-4 sources) or correct doc 07 §6.4/§9 to name the real enforcement path. The crate-side honesty (audit.rs:15-16) does not repair the doc drift.
Confidence: HIGH — Verification: read (registry listing, ci.yml echo, doc lines).

### F-003 [S4]
Where: audit/runs/baseline/INDEX.md (rows 3–4) vs audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt
Code (INDEX.md:13, the cited evidence run):
```
| 3 | `cargo test --workspace --locked` (release profile*) | 101 | **1681 passed / 1 failed / 24 ignored** — fail: `apps/desktop/tests/zero_alloc_transport.rs:98` zero-alloc claim (608 bytes allocated) | test-workspace.log.txt |
```
What is wrong: The two INDEX-cited workspace runs lack `--no-fail-fast` and abort at the first failing binary — `rg "deps/seed_integrity-"` and `"deps/audit_proof_matrix-"` → 0 matches in both logs; the localization, product-config and settings-schema suites are absent from both. The orchestrator's repair run (orchrun-test-workspace-nofailfast.log.txt, "CMD: cargo test --workspace --release --locked --no-fail-fast", 4534 passed / 7 failed / 49 ignored) covers them — audit suites all pass — but INDEX.md does not list this repair run, so an agent citing only the INDEX table concludes these suites never executed.
Why it matters: The INDEX's "1681 passed" understates the suite by ~2850 tests and hides 6 of 7 failures (the log's own ORCHESTRATOR SUMMARY says exactly this); audit evidence chains built on INDEX rows 3–4 are incomplete by construction.
What correct looks like: INDEX.md gains a row for the no-fail-fast repair run with its totals and its 7 failures (including F-007 below).
Confidence: HIGH — Verification: read (rg counts over all three logs).

### F-004 [S3]
Where: crates/localization/src/audit.rs:343-350
Code:
```rust
/// Map a load failure to its structured issue (exposed for callers that
/// load catalogs themselves, e.g. the future `xtask l10n-audit`).
pub fn catalog_error_issue(file: &str, error: &CatalogError) -> CatalogIssue {
```
What is wrong: `catalog_error_issue` has zero callers repo-wide (rg "catalog_error_issue" → only its definition and lib.rs doc mention; not re-exported from lib.rs:108, not used by tests).
Why it matters: Dead public API written for a consumer ("the future xtask l10n-audit") that does not exist (F-002). Ship-shape repo rule: no speculative public surface.
What correct looks like: Remove it until the consumer lands, or make the consumer land.
Confidence: HIGH — Verification: read (rg across repo).

### F-005 [S4]
Where: crates/localization/src/audit.rs:228-234
Code:
```rust
    let language = catalog
        .locale()
        .as_str()
        .split('-')
        .next()
        .unwrap_or_default()
        .to_string();
```
What is wrong: Re-implements `Locale::language_primary` (locale.rs:129-137, which exists precisely for this split) inline; `.unwrap_or_default()` on `split().next()` is dead defensiveness — `str::split` always yields at least one item, and `Locale::parse` guarantees a non-empty leading language subtag.
Why it matters: Duplicated canonicalization logic drifts (if the locale grammar grows, e.g. grandfathered tags, two code paths diverge). Correctness today: unaffected — `Locale::parse` canonicalizes language to lowercase (locale.rs:90), which the plural table requires.
What correct looks like: `let language = catalog.locale().language_primary();` then `categories_for_language(language.as_str())`.
Confidence: HIGH — Verification: read both sites.

### F-006 [S4]
Where: crates/localization/src/audit.rs:118-126, doc 07 §6.4 rule 1
Code:
```rust
    /// Coverage as a percentage (0–100). A missing reference yields 0.
    pub fn percent(&self) -> f32 {
```
What is wrong: Doc 07 §6.4 rule 1: "a coverage percentage is also printed for maintainers `[PROPOSAL]`". `Coverage::percent` exists but no non-test code computes or prints coverage for any real catalog — with only en-US.toml shipped, there is no non-reference catalog to measure.
Why it matters: Declared maintainer output never produced anywhere; the [PROPOSAL] marker makes this a soft drift, but the API is currently test-only surface.
What correct looks like: Wire the coverage print into the l10n gate when it lands (same fix as F-002), or drop the API until then.
Confidence: HIGH — Verification: read (rg "catalog_coverage\|percent()" → tests only).

### F-007 [S3] (cross-file — owner: fallback.rs, not in W11-A08 list)
Where: crates/localization/src/fallback.rs:148-151 vs fallback.rs:291-292
Code (fallback.rs:146-151):
```rust
        // `missing:<key>` (doc 07 §6.3); release builds render the raw
        // key — CI makes both unreachable, but the runtime stays honest.
        let text = if cfg!(debug_assertions) {
            format!("missing:{key}")
        } else {
            key.as_str().to_string()
```
and the failing assertion (fallback.rs:291-292):
```rust
        // Debug builds (tests run in debug) show the miss visibly.
        assert_eq!(translation.text, "missing:fixture.does-not-exist");
```
What is wrong: The test asserts the debug-only `missing:` marker unconditionally; under the documented infrastructure deviation (all baseline test runs use `--release`, INDEX.md footnote), `cfg!(debug_assertions)` is false and the test FAILS — observed: orchrun-test-workspace-nofailfast.log.txt:2727-2737 (`left: "fixture.does-not-exist" / right: "missing:fixture.does-not-exist"`, `-p localization --lib` 44 passed / 1 FAILED). The test's comment "tests run in debug" is false for every evidence run in the repo.
Why it matters: The crate ships a test that fails under the repo's own documented test profile; the fail-fast baseline hid it entirely ("NEVER RAN in the fail-fast baseline" — the orchestrator summary's words). Any release-profile CI run (e.g. a future perf lane) goes red on a non-defect, training developers to ignore failures.
What correct looks like: Gate the assertion on `cfg!(debug_assertions)` (assert debug form when debug, raw key when release), or run this test with `--cfg`-independent expectations. Recorded here because it surfaced during this file's evidence verification; fallback.rs belongs to another W11 agent.
Confidence: HIGH — Verification: read (fallback.rs) + ran-evidence citation (orchrun log lines 2727-2737).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `impl Display for CatalogIssue::fmt` | 62-106 | REAL | — | All 7 variants rendered; rule numbers + doc refs in text; no panic paths |
| `Coverage::percent` | 118-126 | REAL-UNPROVEN | S4 | Zero-divide guarded (returns 0.0); test-only caller (F-006) |
| `audit_missing_keys` | 130-139 | REAL | — | Correct filter/map; file attribution via source_name; tested exactly |
| `catalog_coverage` | 142-151 | REAL | S4 | Correct count; test-only consumer |
| `audit_orphan_keys` | 157-166 | REAL | — | Mirror of missing_keys; tested |
| `audit_placeholders` | 172-193 | REAL | — | Compares name+type set (BTreeMap) per shared key; skips rule-1/rule-2 territory correctly; tested with doc 07's exact example |
| `format_placeholder_set` (private) | 195-204 | REAL | — | Deterministic output; empty-set label |
| `audit_references` | 209-218 | REAL | S2 (scope, F-001) | Logic correct; wiring only feeds it product.toml refs |
| `audit_plural_conformance` | 226-259 | REAL | S4 | Logic correct (missing-arms always flagged; extra arms only for known languages); language extraction duplicates `language_primary` (F-005) |
| `plural_conformance_detail` (private) | 261-294 | REAL | — | Correct; singular/plural wording handled; no panics |
| `audit_catalogs` | 301-303 | REAL | — | Thin delegate |
| `audit_catalogs_with_refs` | 307-341 | REAL | S2 (F-001 via callers) | Correct ordering: load failures → reference-missing early-return → reference rule 5+4 → per-catalog rules 1/2/3/5; skips reference in per-catalog loop correctly |
| `catalog_error_issue` | 345-350 | DEAD | S3 | Zero callers (F-004) |
| tests (6) | 352-514 | REAL | — | See §5 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `rule1_reports_every_missing_key` | YES | Asserts exact 4-key ordered list; mutation (drop the filter) changes the list; mutation (return empty) fails assert_eq |
| `rule1_counts_missing_keys_correctly` | YES | Exact counts 4/2-of-6 + float compare on percent |
| `rule2_reports_orphan_keys` | YES | Asserts exact OrphanKey variant + file/key + Display substring |
| `rule3_reports_placeholder_mismatches` | YES | Doc 07's own example; asserts single issue, exact expected/actual substrings |
| `rule4_reports_unresolved_references` | YES | One resolving + one unresolved ref; asserts exactly 1 issue with field/key |
| `a_perfect_catalog_produces_no_issues` | YES (negative control) | Empty-results assertion catches over-firing rules |
| (integration) `tests/audit_proof_matrix.rs` (14 tests) | YES | Both directions per rule + fixture-drift sanity test (`fixture_reference_catalogs_are_identical_across_directories`) — VERIFIED PASSING: orchrun-test-workspace-nofailfast.log.txt:2740-2758 (14/14 ok) |
| (integration) `tests/seed_integrity.rs` (3 tests) | YES | Shipped-tree negative control; VERIFIED PASSING: orchrun log:2760-2766 (3/3 ok); but its rule-4 test is scoped to product.toml only (F-001) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Run by `cargo xtask l10n-audit`" (five rules) | doc 07 §6.4:678 | FALSE | tools/xtask/src/cmd/mod.rs:72-193 — no such subcommand; ci.yml:468 admits it |
| CI job sequence step 6 `cargo xtask l10n-audit (C3)` merge-blocking | doc 07 §9:846 | FALSE | Same; CI runs `cargo test -p localization` instead (ci.yml:438) |
| Rule 4 covers product.toml + schemas/settings.toml + layouts/ + keymaps/ + profiles/ + themes/ | doc 07 §6.4:683 | FALSE (in enforcement) | seed_integrity.rs:37 scans product.toml only; config_validator.rs:487 checks 2 keys only; 121 settings msgids unresolved (F-001) |
| "a coverage percentage is also printed for maintainers" | doc 07 §6.4:680 [PROPOSAL] | PAPER-ONLY | No non-test caller of `catalog_coverage`/`percent` |
| "also run by config-validator §3.4" (rule 4) | doc 07 §6.4:683 | PARTIAL/FALSE | config-validator re-implements a 2-key msgid existence check with its own toml_lite reader instead of using this crate (config_validator.rs:482-495) |
| Rules 1/2/3/5 library functions + proof fixtures exist | audit.rs module doc | REAL | Code + fixtures verified by reading; executed and passing: orchrun-test-workspace-nofailfast.log.txt (audit_proof_matrix 14/14, seed_integrity 3/3) |

## 7. Dead / duplicated / unreachable code
- Entire module unreachable from shipped entry points: zero dependents of the `localization` crate (rg over all Cargo.toml). Only the crate's own tests call it.
- `catalog_error_issue` (343-350): dead public fn (F-004).
- `Coverage`/`catalog_coverage`: test-only surface (F-006).
- `audit_plural_conformance` duplicates `Locale::language_primary` (F-005).
- xtask `config_validator.rs:482-495` duplicates rule-4 logic (2-key subset) with a private TOML-lite reader instead of depending on this crate — cross-file duplication; the localization crate was built for exactly this consumer and remains unused by it.

## 8. Dependencies & flags
- Crate deps: `toml = "1.1"`, `thiserror = "2.0"` (Cargo.toml:23-25) — no feature flags; no unsafe; workspace lints inherited (`[lints] workspace = true`).
- No feature gates hide any function in this file. The whole crate is compiled into the workspace but linked by no binary.

## 9. Open questions for the next agent
- F-001 quantified for schemas/settings.toml (121/121 unresolved). Do the OTHER rule-4 sources (keymaps/default.toml has no msgid fields — verified; profiles/, layouts/ do not exist) stay empty by design, and does any wave own seeding the `settings.*` catalog keys?
- F-007: fallback.rs:148/292 profile-coupled test failure — flag to the W11 agent owning crates/localization/src/fallback.rs; is the release/raw-key rendering itself doc-conformant (doc 07 §6.3 says debug-only marker) or should release also mark misses?
- Who owns card 1.E2 (l10n-audit wiring), and is the frozen xtask registry (cmd/mod.rs FROZEN note) the recorded blocker? Check docs/13-agent-wave-program.md.
- The orchestrator repair run (orchrun-test-workspace-nofailfast.log.txt) also ran the product-config suites (merge/semantic/fixtures_matrix/golden_s35/rebrand_host/web_budgets at lines 4693-4757) and settings-schema integration (line 4927) — W11 agents for those crates should cite it, not INDEX rows 3–4.
