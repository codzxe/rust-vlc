# AUDIT: crates/localization/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-21 |
| Lines / functions | 155 / 13 (7 `pub mod` declarations, 1 re-export block, `line_col` + 1 test; the bulk of the file is the 97-line crate doc audited claim-by-claim in §6) |
| File verdict | UNREACHABLE (crate root of an orphan crate; code REAL, its own tests baseline-executed) |
| Reachable from shipped entry point? | NO — proof: no `Cargo.toml` in the repo declares a dependency on `localization` (`rg -n "localization" --glob '**/Cargo.toml'` → only the crate's own manifest + a comment in crates/playlists/Cargo.toml:5); `rg -n "localization::" --glob '!crates/localization/**'` over the repo → hits only in `locales/en-US.toml` (header comment), `docs/07-config-driven-system.md`, and prior audit files. The entire `pub use` surface (lib.rs:108-115) has zero external consumers. Execution evidence: the crate's tests run only under `cargo test` (baseline #3 / no-fail-fast log). |
| Findings | S0:0 S1:0 S2:1 S3:0 S4:3 |
| Commands run | Read lib.rs 1-155 (complete) · `rg -n "line_col\(" crates/localization/src` (callers: catalog.rs:207,241,243; refs.rs:75 — live) · `rg -n "\.unwrap\(\)\|\.expect\(\|panic!\|todo!\|unimplemented!\|dbg!" crates/localization/src crates/localization/tests` (ZERO hits — lint-policy claim verified) · `ls ui/.eslintrc.json` → "No such file or directory"; `cat ui/eslint.config.js` (flat config; header: rules promoted to `ui/tools/eslint-plugin-local/`; line 108 `"local/i18n-no-literals": "error"`) · `rg -n "eslint" ui/package.json` (eslint 10.13.0) · `rg -n "localization\|l10n\|catalog_gen" tools/xtask/src` (only lint_literals.rs comments — NO l10n-audit subcommand exists) · baseline `orchrun-test-workspace-nofailfast.log.txt:2716` (`tests::line_col_counts_lines_and_scalar_columns ... ok`), `:2683-2691` (catalog), `:2692-2738` (fallback, incl. the FAILED release-profile test), `:2740-2771` (audit_proof_matrix 14/14, seed_integrity 3/3) · `sed -n` verification of every quote below |

## 1. One-paragraph verdict
This is the root of a well-documented, carefully-linted, thoroughly-tested crate that nothing ships: 97 lines of crate doc making accurate mechanism claims (key grammar, five audit rules, fallback chain — each backed by green or one-red baseline tests), a `line_col` helper that is correct and used, and a re-export surface with zero external consumers. The doc's one substantive silence is the release-build last link of the fallback chain — it documents `(debug builds only) … missing:<key>` and never says that release renders a bare internal key, a gap the baseline's one FAILING localization test exposes on evidence (fallback.rs:292 under `--release`). Two citations have rotted: the "TypeScript/TSX ESLint rule" is pinned to `ui/.eslintrc.json`, a file that does not exist (the rule was promoted to `ui/tools/eslint-plugin-local/`, wired via `ui/eslint.config.js:108`), and the promised `cargo xtask l10n-audit` wiring ("belongs to card 1.E2") never arrived with 1.E2's config-validator. The verified backbone: the lint-policy claim ("no unwrap/expect/panic!/todo!/unimplemented!/dbg!") is literally true — a repo-wide grep of the crate returns zero hits.

## 2. What actually works (verified)
— Verified by `rg -n "\.unwrap\(\)|\.expect\(|panic!|todo!|unimplemented!|dbg!" crates/localization/src crates/localization/tests` → ZERO matches (the only hit of the pattern set is the doc comment naming the ban itself). The lib.rs:91-97 lint-policy claim holds for every file of the crate.
— Verified by baseline `orchrun-test-workspace-nofailfast.log.txt:2716` → `tests::line_col_counts_lines_and_scalar_columns ... ok`; the helper's semantics (1-based, scalar-value columns, clamp-beyond-end) are pinned by six exact asserts I hand-checked against `"ab\ncdé\nf"` (offset 5 = the 2-byte `é`, asserted (2,3) — one column, not two).
— Verified: `line_col` has three live in-crate call sites (catalog.rs:207,241,243; refs.rs:75) — not dead code within the crate.
— Verified: the five catalog-audit rules the doc enumerates (54-73) are enforced by tests that ran green in baseline: `tests/seed_integrity.rs` 3/3 ok incl. `shipped_locales_tree_passes_every_catalog_audit_rule`, and `tests/audit_proof_matrix.rs` 14/14 ok (orchrun log 2740-2771).

## 3. Findings

### F-001 [S2]
Where: crates/localization/src/lib.rs:3-6, 108-115
Code:
```
//! Stable message keys, parameterized messages, an explicit fallback chain,
//! and catalog auditing. This crate is the *mechanism*; catalogs are data in
//! `locales/` (doc 01 §2.3): `locales/en-US.toml` is the reference catalog,
//! other catalogs are one file per BCP-47 tag (`de.toml`, `pt-BR.toml`).
```
What is wrong: the crate presents itself as THE mechanism for every user-visible string, but no workspace member depends on it — neither `lumen` (apps/cli) nor `apps/desktop`/`desktop-tauri` nor `tools/xtask` links it. The `pub use` surface (108-115: `MessageKey`, `Catalog`, `Localizer`, `select_locale`, `audit_catalogs`, …) has no consumer outside the crate's own tests. The strings users actually see come from the TypeScript stack (`ui/src/i18n/messages.ts`, checked by `npm run l10n:check`, baseline #24 PASS), which reimplements the §6.1 grammar independently.
Why it matters: this is the declared-vs-consumed question at crate scale: a complete localization mechanism (grammar newtype, message-format parser, fallback chain, five-rule auditor) documented as load-bearing and linked by nothing. The two implementations (Rust crate vs TS pipeline) can drift with no cross-check; `locales/en-US.toml`'s "source of truth" status is enforced only in `cargo test`, never at runtime.
What correct looks like: a shipped consumer (CLI `--help`/UI message generation from `locales/*.toml`) or an explicit demotion of the crate to CI-only auditor with the doc rewritten to say so.
Confidence: HIGH — Verification: read + two repo-wide greps (Cargo.toml dependents; `localization::` users) + baseline #3/#24. Cross-refs: key.rs F-001 (W11-A04), message.rs#part2 F-001, fallback.rs (W11-A07), catalog.rs F-001 (W11-A10, this agent) — same orphan status independently re-verified.

### F-002 [S4]
Where: crates/localization/src/lib.rs:46-52
Code:
```
//! requested locale → its language-primary parent (`pt-BR` → `pt`, when
//! such a catalog exists) → the product default locale → `en-US` →
//! (debug builds only) the key itself, rendered as `missing:<key>`. The
//! chain is total: a missing translation can never panic rendering; every
//! miss records a diagnostic. See [`fallback`].
```
What is wrong: the documented chain stops at the debug-only `missing:<key>` link and never states what a RELEASE build renders — the bare internal key, no marker (fallback.rs:148-152 gates the prefix on `cfg!(debug_assertions)`). The omission is on evidence: baseline no-fail-fast run, `fallback::tests::missing_everywhere_records_diagnostic_and_never_panics` FAILED at fallback.rs:292 with `left: "fixture.does-not-exist"  right: "missing:fixture.does-not-exist"` (orchrun log 2725-2733) — the release-profile behavior is the bare key.
Why it matters: a reader of the crate doc (the contract) learns the last link is `missing:<key>`; in every release artifact the actual last link is an unmarked raw key — indistinguishable from a real (if odd) message. Doc incompleteness on the exact failure mode the chain exists to make visible.
What correct looks like: document both profiles: "(debug) `missing:<key>`; (release) the bare key — CI must keep the final links unreachable" (and see W11-A07 F-002/F-004 for why that CI guarantee does not currently exist either).
Confidence: HIGH — Verification: read + baseline-executed failing test (cited verbatim above); fallback.rs analysis cross-ref W11-A07 F-001 (the test, not the code, is what breaks under `--release` — the code's release behavior is exactly the undocumented bare key).

### F-003 [S4]
Where: crates/localization/src/lib.rs:87-89
Code:
```
//! this API. The Rust heuristic sweep and the TypeScript/TSX ESLint rule
//! are separate enforcement layers (xtask `lint-literals`, `ui/.eslintrc.json`
//! stub); their fixtures are owned by the cards that land them.
```
What is wrong: the citation `ui/.eslintrc.json` names a file that does not exist (`ls ui/.eslintrc.json` → "No such file or directory"). The rule substance survived under a different address: `ui/eslint.config.js` (ESLint 10 flat config) imports `ui/tools/eslint-plugin-local/` and registers `"local/i18n-no-literals": "error"` at line 108 — and its header says the rules were "promoted verbatim from this file, unit-tested via vitest", i.e. no longer a "stub".
Why it matters: rotten pointer in the crate's own contract doc; a developer following it to add a fixture lands on a nonexistent path. The claim's substance (a TS literal-text lint exists) is TRUE; the citation and the "stub" qualifier are both stale.
What correct looks like: cite `ui/eslint.config.js` + `ui/tools/eslint-plugin-local/` (tested package).
Confidence: HIGH — Verification: ls + cat of both files (commands above).

### F-004 [S4]
Where: crates/localization/src/lib.rs:76-78
Code:
```
//! [`audit::audit_catalogs`] runs all of them over a `locales/` directory;
//! `cargo xtask l10n-audit`, doc 07 §6.4) belongs to card 1.E2 — the
```
What is wrong: the doc defers the CI gate to "card 1.E2", but 1.E2's deliverable shipped as `cargo xtask config-validator` (baseline #10, exits RED on 4 brand-icon violations) WITHOUT any `l10n-audit` subcommand — `rg -n "localization|l10n|catalog_gen" tools/xtask/src` matches only comments in lint_literals.rs. The promised wiring never landed with the card the doc names, and the doc still reads as if the gate is merely pending.
Why it matters: mild doc drift today (the "until then this crate's own test suite enforces the rules" clause at 79-81 is accurate — seed_integrity/audit_proof_matrix green in baseline), but it launders an unfulfilled plan into an architecture statement. Cross-ref W11-A07 F-002 (fallback.rs makes the stronger, false version of this claim).
What correct looks like: name the actual state: "no xtask gate exists; enforcement is `cargo test -p localization` (ci.yml)".
Confidence: HIGH — Verification: read + xtask grep + baseline #10 (config-validator exists, no l10n subcommand in its output).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| crate doc (module docs) | 1-97 | REAL with drift | S4 (F-002, F-003, F-004) | 11 doc claims extracted in §6; 8 REAL, 2 stale-pointer/drift, 1 vacuous-by-unreachability. Grammar/format/fallback/rules descriptions match audited sibling files. |
| `pub mod audit` | 99 | REAL (UNREACHABLE shipped) | — | Audited by W11-A08 (S2:1 S3:2 S4:4 per its file). |
| `pub mod catalog` | 100 | REAL (UNREACHABLE shipped) | — | Audited by this agent (catalog.rs.audit.md) — S2:1 S3:1 S4:4. |
| `pub mod fallback` | 101 | REAL (UNREACHABLE shipped) | — | Audited by W11-A07 — S2:1 (release-profile test failure) S3:1 S4:2. |
| `pub mod key` | 102 | REAL (UNREACHABLE shipped) | — | Audited by W11-A04 — S2:1 (orphan) S3:1 S4:1. |
| `pub mod locale` | 103 | REAL (UNREACHABLE shipped) | — | Locale tests green in baseline (orchrun 2703-2705); assigned to another W11 agent. |
| `pub mod message` | 104 | REAL (UNREACHABLE shipped) | — | message.rs#part1/#part2 audited (part2 F-001 = the crate-orphan S2); tests green in baseline (2707-2712). |
| `pub mod plural` | 105 | REAL (UNREACHABLE shipped) | — | Tests green in baseline (2713-2715, 2718-2719); assigned to another W11 agent. |
| `pub mod refs` | 106 | REAL (UNREACHABLE shipped) | — | Tests green in baseline (2720-2723); assigned to another W11 agent. |
| `pub use` re-exports (8 lines) | 108-115 | REAL (zero external consumers) | — | All paths resolve (baseline build CLEAN, INDEX #1); every re-exported name's callers are in-crate tests only (F-001). |
| `line_col` | 122-137 | REAL | — | Correct: 1-based, columns count Unicode scalars (pinned by green test, orchrun 2716), clamp-beyond-end pinned; a `pos` landing mid-char counts the full char (callers pass token-boundary offsets from toml spans / message offsets — no live miscount path found). O(pos) per call, fine for diagnostics. |
| `tests::line_col_counts_lines_and_scalar_columns` | 143-154 | REAL | — | Six exact asserts incl. the multi-byte `é` column and the beyond-end clamp; discriminates a byte-counting or 0-based regression. Green in baseline. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| line_col_counts_lines_and_scalar_columns | YES | Byte-vs-scalar column counting, 1- vs 0-based, and end-clamping are each pinned by a distinct exact assert; any semantic change breaks at least one. |
(Only one test lives in this file; the crate's other 44 lib tests + 17 integration tests belong to sibling files and are covered in their audits. Crate totals in the no-fail-fast baseline: 44 passed / 1 failed (fallback F-001, release profile), audit_proof_matrix 14/14, seed_integrity 3/3.)

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "This crate is the *mechanism*; catalogs are data in `locales/`" | lib.rs:4-5 | REAL code / UNREACHABLE shipped (F-001) | Zero dependents (Cargo.toml grep); only `cargo test` executes it (baseline #3). |
| Key grammar "2–5 dotted segments, each `[a-z0-9-]+`" | lib.rs:10-11 | REAL | key.rs audited (W11-A04); `accepts_valid_keys`/`rejects_invalid_keys_with_typed_errors` green in baseline (orchrun 2698-2700). |
| Message-format v1 subset (4 forms) | lib.rs:32-37 | REAL | message.rs tests green (2707-2712) incl. `parses_all_v1_forms`, `rejects_malformed_messages`. |
| Fallback chain ends "(debug builds only) … `missing:<key>`" | lib.rs:50-51 | REAL in debug / release link UNDOCUMENTED (F-002) | Code: fallback.rs:148-152 `cfg!(debug_assertions)` gate; baseline FAILED test (release) renders bare key — orchrun 2725-2733 verbatim. |
| "The chain is total: a missing translation can never panic rendering; every miss records a diagnostic" | lib.rs:51-52 | REAL | fallback.rs translate has no panic path (W11-A07 per-function table); diagnostic recorded (the failed test's OTHER asserts — resolved_locale None, diagnostics len 1 — passed). |
| Five catalog-audit rules (missing/duplicate/placeholder/reference-completeness/plural) | lib.rs:54-73 | REAL | seed_integrity 3/3 + audit_proof_matrix 14/14 green (orchrun 2740-2771). |
| "cargo xtask l10n-audit … belongs to card 1.E2" | lib.rs:76-78 | DRIFT (F-004) | 1.E2 shipped as config-validator (baseline #10) with no l10n subcommand (xtask grep). |
| "this crate's own test suite enforces the rules on the shipped tree" | lib.rs:79-81 | REAL | seed_integrity green in baseline over the real `locales/`. |
| "The translation API accepts only MessageKey values — a raw string literal cannot reach a user through this API" | lib.rs:85-87 | REAL in-crate / VACUOUS for users (F-001) | Single-constructor newtype (W11-A04 §2); no shipped caller exists, so no user-visible string passes through this API at all. |
| "separate enforcement layers (xtask `lint-literals`, `ui/.eslintrc.json` stub)" | lib.rs:87-89 | STALE POINTER (F-003) | `.eslintrc.json` absent; rule = `local/i18n-no-literals` in ui/eslint.config.js:108, promoted to tested package ui/tools/eslint-plugin-local/. |
| Lint policy: "no `unsafe`, no `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!`/`dbg!`" | lib.rs:93-94 | REAL | My grep over src+tests: zero matches. Baseline clippy CLEAN (INDEX #5). |

## 7. Dead / duplicated / unreachable code
- The whole `pub use` block (108-115) is externally dead (F-001) — internally, each re-exported name is used by the crate's own tests/audit machinery.
- `line_col` is live in-crate (catalog.rs ×3, refs.rs ×1) — not dead.
- Duplication: the TS i18n pipeline (ui/src/i18n/messages.ts + scripts/l10n-check.mjs) duplicates the §6.1 grammar — the crate's Rust enforcement and the UI's JS enforcement are parallel, unlinked implementations (see key.rs F-001 for the TS side's own claim).

## 8. Dependencies & flags
`toml` 1.1 + `thiserror` 2.0 only (Cargo.toml) — matches the pre-audited dependency policy; no features, no cfg gates except `#[cfg(test)]`; `[lints] workspace = true`; zero `#[allow]`. The crate compiles clean in baseline build #1 and clippy #5.

## 9. Open questions for the next agent
- If the crate is ever linked (F-001 resolution), the release-mode bare-key rendering (F-002) becomes a user-visible defect — fix the doc AND the sentinel before shipping a consumer.
- `ui/tools/eslint-plugin-local/` (the promoted rules) — has any wave audited the TS enforcement layer itself (rule correctness, allowlist hygiene)? Not in my assignment list; the config-system wave should claim it.
- locales/en-US.toml and keymaps/default.toml are assigned to W11 agents — their headers' "enforced by `localization::MessageKey`" claims inherit F-001's vacuity.
