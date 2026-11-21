# AUDIT: crates/localization/src/fallback.rs
| Field | Value |
|---|---|
| Auditor | W11-A07 |
| Date | 2026-11-21 |
| Lines / functions | 317 / 17 (10 items + 7 tests) |
| File verdict | UNREACHABLE (code REAL; 6/7 tests pass in baseline; 1 test FAILS in baseline — see F-001) |
| Reachable from shipped entry point? | NO — `Localizer`/`select_locale`/`Translation` are re-exported at lib.rs:110 with zero callers outside fallback.rs's own tests (`grep -rn "Localizer\|select_locale\|fallback::" crates/ apps/ tools/ tests/` minus self → only lib.rs:110). Same crate-orphan status as message.rs (see message.rs#part2 F-001). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | grep callers of `Localizer`/`select_locale` (lib.rs re-export only); grep `l10n` in tools/xtask/src (exit 1 — no gate); sed-read of ci.yml:438/468 (CI admits l10n-audit does not exist; runs `cargo test -p localization` instead); baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt (fallback test block, 44/45 lib tests, failure verbatim); sed -n verified every quote below. |

## 1. One-paragraph verdict
The fallback chain is correctly implemented per doc 07 §6.3 — order, per-key
resolution, dedup, and locale selection all verified by six passing baseline
tests — and it is dead code like the rest of the crate: nothing shipped ever
constructs a `Localizer` or calls `select_locale`. Its one hard defect is on
evidence, not on paper: the test `missing_everywhere_records_diagnostic_and_never_panics`
FAILS in the committed baseline because it hard-codes the assumption "tests
run in debug" into an unconditional assertion; under `cargo test --release`
(the profile the repo's own baseline had to use) `cfg!(debug_assertions)` is
false and the assertion breaks. The module doc also overstates CI coverage:
the §6.4 gate it leans on ("CI makes the final links unreachable") does not
exist, and the actual stand-in (`cargo test -p localization`, ci.yml:438)
join-checks only the two product.toml msgids, not the full rule-4 surface.

## 2. What actually works (verified)
— 6 of 7 tests pass in baseline: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (localization lib run, 44 passed / 1 failed) — `fallback::tests::{chain_order_matches_doc07, chain_with_distinct_default, translates_with_requested_catalog, falls_back_per_key_to_parent_then_reference, locale_selection_prefers_setting_then_os_then_default}` all `ok`.
— Chain construction hand-traced: `build` with requested=pt-BR, default=en-US yields preferences [pt-BR, pt, en-US(reference collapsed by dedup at 105–107)] → chain ["pt-BR","pt","en-US"], matching the asserted expectation at line 206 and doc 07 §6.3's chain.
— `select_locale` = `setting.or(os).unwrap_or(default).clone()` (line 28) — literally the documented three-step preference; covered by a three-branch test (309–314).
— Dependencies used all exist: `Locale::reference` (locale.rs:60), `Locale::language_primary` (locale.rs:129), `REFERENCE_LOCALE` (catalog.rs:23).

## 3. Findings

### F-001 [S2] Test hard-codes "tests run in debug" — FAILS under `cargo test --release` (reproduced in baseline)
Where: crates/localization/src/fallback.rs:291-292 (assertion), 148-152 (code under test)
Code:
```rust
        // Debug builds (tests run in debug) show the miss visibly.
        assert_eq!(translation.text, "missing:fixture.does-not-exist");
```
and the code branch it pins:
```rust
        let text = if cfg!(debug_assertions) {
            format!("missing:{key}")
        } else {
            key.as_str().to_string()
        };
```
What is wrong: The assertion is unconditional but the behavior is profile-dependent. Under `cargo test --release` — a first-class cargo invocation, and exactly what the repo's own baseline used twice (INDEX.md runs #3/#4, release-profile deviation footnote) — `cfg!(debug_assertions)` is false, `translate` returns the raw key `"fixture.does-not-exist"`, and the test panics. Verbatim baseline evidence, `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt`:
```
thread 'fallback::tests::missing_everywhere_records_diagnostic_and_never_panics' (7372) panicked at crates/localization/src/fallback.rs:292:9
assertion `left == right` failed
  left: "fixture.does-not-exist"
 right: "missing:fixture.does-not-exist"
```
The orchestrator summary lists it as failure 7 of 7: "crates/localization src/fallback.rs::fallback::tests::missing_everywhere_records_diagnostic_and_never_panics (:292 — assertion left != right) — NEVER RAN in the fail-fast baseline." (The fail-fast run #3 stopped at the desktop zero-alloc failure before localization's binary executed — so the INDEX.md headline "1681 passed / 1 failed" undercounts; the no-fail-fast repair found 4534/7/49.)
Why it matters: Any release-profile test lane (dev `--release`, disk-constrained CI like the baseline, benchmark-adjacent runs) goes red on a test whose subject code is correct. The test's environment assumption is drift: nothing in the repo enforces "tests run in debug" — CI's `cargo test --workspace --all-features` (ci.yml:234) happens to run debug today, which is the only reason lanes are green.
What correct looks like: `let expected = if cfg!(debug_assertions) { format!("missing:{key}") } else { key.as_str().to_string() }; assert_eq!(translation.text, expected);` — the remaining assertions (resolved_locale None, diagnostics len 1, "missing key" text) are profile-independent and already correct.
Confidence: HIGH — Verification: ran (baseline orchrun-test-workspace-nofailfast.log.txt, verbatim output above) + read.

### F-002 [S3] Module doc overstates CI: "CI (doc 07 §6.4) makes the final links unreachable" — the §6.4 gate does not exist and the stand-in covers a fraction of rule 4
Where: crates/localization/src/fallback.rs:11-14 (and repeated at 145-147)
Code:
```rust
//! The chain is total: a missing translation can never panic rendering.
//! Every miss records a [`FallbackDiagnostic`] — CI (doc 07 §6.4) makes
//! the final links unreachable in shipped builds, but the runtime still
//! degrades honestly instead of crashing.
```
What is wrong: Doc 07 §6.4's runner — "`cargo xtask l10n-audit` (backed by `crates/localization::audit_catalogs`)" — does not exist (`grep -rn "l10n" tools/xtask/src/` → exit 1, zero matches). CI's own skip ledger admits it: ci.yml:468 "COVERED C3 (partial): cargo test -p localization above (shipped-tree audit; **the l10n-audit xtask subcommand itself does not exist**)." The actual enforcement is ci.yml:438 `cargo test -p localization`, whose seed_integrity suite audits the real locales/ tree against rules 1–5 but join-checks rule 4 (reference-completeness) against **only** product.toml's two msgids (tests/seed_integrity.rs:38 asserts `refs.len() == 2`) — not the doc's full rule-4 surface (schemas/settings.toml, layouts/, keymaps/, profiles/, themes/, doc 05 menu tables).
Why it matters: The doc claim "final links unreachable in shipped builds" is the safety argument for rendering raw keys to users in release (line 151). With no full join check, a msgid referenced from keymaps/ or schemas/ that is absent from en-US.toml ships silently and renders as the bare key. Today moot only because nothing consumes the crate (unreachability, message.rs#part2 F-001) — but the doc+code jointly promise a guarantee nothing enforces.
What correct looks like: Either implement the `l10n-audit` xtask subcommand wrapping `audit_catalogs_with_refs` over the full ref surface, or rewrite the module doc to state the actual coverage ("enforced only for product.toml msgids by seed_integrity").
Confidence: HIGH — Verification: read (greps + ci.yml:438/468 + seed_integrity.rs:38 cited; no l10n gate stage appears anywhere in baseline INDEX or workflows).

### F-003 [S4] Per-key fallback to parent/default is silent — no diagnostic distinguishes "translated" from "fell back"
Where: crates/localization/src/fallback.rs:130-144
Code:
```rust
        for catalog in &self.chain {
            if let Some(message) = catalog.get(key) {
                let rendered = message.render(catalog.locale(), args);
                return Translation {
                    text: rendered.text,
                    resolved_locale: Some(catalog.locale().clone()),
```
What is wrong: `translate` records a `FallbackDiagnostic` only on total miss (lines 153–159). A key missing in the requested catalog but resolved by the parent/default/reference produces zero diagnostics — `resolved_locale` carries the information, but the module doc's "Every miss records a [`FallbackDiagnostic`]" (line 12) reads as covering per-catalog misses too.
Why it matters: Runtime observability: a UI showing pt text for a pt-BR user (or en-US text for everyone) looks identical to a correct translation unless the caller inspects `resolved_locale`. Translation-coverage rot becomes invisible at runtime. Minor because `resolved_locale` does expose it and rule 1 is supposed to make parent-fallback structurally impossible (catalogs ⊇ reference key set).
What correct looks like: Either a `FallbackDiagnostic::FellBack { key, from, to }` variant, or a doc line scoping "miss" to "total miss".
Confidence: HIGH — Verification: read (both return paths traced; test at 252–277 asserts no diagnostics field for parent-fallback cases — it asserts text/locale only).

### F-004 [S4] Release-mode degradation renders a bare internal key with no visible marker
Where: crates/localization/src/fallback.rs:148-152
Code:
```rust
        let text = if cfg!(debug_assertions) {
            format!("missing:{key}")
        } else {
            key.as_str().to_string()
        };
```
What is wrong: In release, a total miss renders the raw dotted key ("player.controls.play") into user-visible text with no marker at all — the diagnostic is only in the `diagnostics` vec, which the (nonexistent) caller must choose to log.
Why it matters: "Degrades visibly" (struct doc, line 54) is true in the weak sense; a user seeing "settings.area.playback.title" in a button has no way to report it as a bug rather than a weird label. Design choice, documented in the struct doc (56–58) — flagged as hygiene, not correctness.
What correct looks like: A stable sentinel prefix in release too (e.g. `‹key›` or keeping `missing:`), or a guaranteed caller-side log hook.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `select_locale` | 27–29 | REAL | — | `setting.or(os).unwrap_or(default).clone()` — exactly doc 07 §6.3's order; 3-branch test passes (baseline). |
| `enum FallbackDiagnostic` | 32–36 | REAL | — | Single variant MissingKey; used at 156. |
| `impl Display for FallbackDiagnostic` | 38–49 | REAL | — | Text asserted contains "missing key" by test (298). |
| `struct Translation` | 52–63 | REAL | — | text + resolved_locale + diagnostics; docs accurate incl. release nuance. |
| `enum RenderOrFallback` | 66–72 | REAL | — | Simple sum wrapper; only ever constructed with Render on hit, Fallback on miss. |
| `impl Display for RenderOrFallback` | 74–81 | REAL | — | Delegates; trivial. |
| `struct Localizer` | 88–91 | REAL | — | chain: Vec<Catalog>. |
| `Localizer::build` | 98–121 | REAL | — | Preferences [requested, parent?, default, reference?] then filter-by-available with per-locale dedup (111–115). Collapsing correct (test 205–206). O(P·A·C) nested scans — hygiene only (catalog counts are single digits). Duplicate-locale `available` entries: first wins — defensible given file-name=tag loading. |
| `Localizer::locales` | 124–125 | REAL | — | Test/debug accessor; used by tests only (also fine as public API). |
| `Localizer::translate` | 130–160 | REAL (defects noted) | S4 | Never panics: only `format!`, clone, vec push. F-003 (silent per-key fallback), F-004 (release raw key). Render diagnostics forwarded on hit (137–141). |
| `tests` (7) | 163–317 | REAL | S2 | F-001: one test profile-broken; other six honest, exact-string assertions, would fail on real regressions (see §5). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| chain_order_matches_doc07 (195–208) | YES | Exact chain ["pt-BR","pt","en-US"] — dropping parent push (101–103) or reference append (105–107) breaks it. |
| chain_with_distinct_default (210–230) | YES | Absent requested/parent must vanish from chain; asserts ["es","en-US"] — catches ordering and dedup bugs. |
| translates_with_requested_catalog (232–249) | YES | Exact text "Olá, Lumen!" + resolved pt-BR + empty diagnostics. |
| falls_back_per_key_to_parent_then_reference (251–278) | YES | Exact texts and resolving locales for parent and reference fallback paths. |
| missing_everywhere_records_diagnostic_and_never_panics (280–301) | YES (debug) / FAILS (release) | F-001: the text assertion is the broken one; the locale/diagnostic assertions are profile-independent and would catch regressions. |
| locale_selection_prefers_setting_then_os_then_default (303–316) | YES | All three select_locale branches asserted with exact tags. |
All seven ran in baseline no-fail-fast: 6 `ok`, 1 `FAILED` (F-001, verbatim in §3).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Fallback chain … requested locale → language-primary parent → defaults.locale → en-US → (debug builds only) the key itself, logged as missing:<key>" | docs/07-config-driven-system.md §6.3 | REAL (code) / UNREACHABLE (shipped) | build:98–121 + translate:148–152; chain test ok in baseline; no shipped caller (lib.rs:110 only). |
| "The chain is total — a missing translation can never crash rendering" | doc 07 §6.3 / module doc line 11 | REAL | translate has no panic path (§4 row translate); tested in debug; release path returns raw key (F-001 shows the test, not the code, breaks in release). |
| "Locale selection: `global.language` setting → OS locale → `defaults.locale`" | doc 07 §6.3 | REAL (fn) / UNREACHABLE (wiring) | select_locale:27–29 exact; zero callers — nothing in the shipped UI/engine reads global.language or the OS locale into it. |
| "CI (doc 07 §6.4) makes the final links unreachable in shipped builds" | fallback.rs module doc 12–13 | FALSE (as stated) — partial stand-in exists | F-002: no l10n-audit xtask (grep exit 1; ci.yml:468 admits); seed_integrity covers tree rules 1–5 + only 2 product.toml msgids (seed_integrity.rs:38). |
| "Every miss records a FallbackDiagnostic" | module doc line 12 | SHALLOW (wording) | Total miss: yes (156–158). Per-key fallback: silent (F-003). |

## 7. Dead / duplicated / unreachable code
- Whole file unreachable from shipped entry points (header chain). `RenderOrFallback` exists solely to merge two diagnostic types for a caller that does not exist.
- No intra-file duplication. Cross-file: the release/debug miss text duplicates knowledge of the `missing:` format with test line 292 and module doc line 8 (three places to keep in sync — F-001 is exactly such a desync, on the environment axis).

## 8. Dependencies & flags
- Uses only crate-internal items (catalog, key, locale, message) — no external crates, no unsafe, no feature gates, no allow-attributes. `#[cfg(test)]` module only.
- `cfg!(debug_assertions)` at line 148 is the file's only profile-dependent behavior — and the root of F-001's profile-dependent test.

## 9. Open questions for the next agent
- A08 (audit.rs): does `audit_catalogs_with_refs` expose a variant taking the full ref surface (keymaps/, schemas/, themes/) that seed_integrity simply doesn't call, or is the 2-msgid scope a hard limit of the API?
- A10 (lib.rs/catalog.rs): `load_locales_dir` is the only production loader — confirm no bin/example constructs a `Localizer` (grep says none).
- Wave-12+ (UI lane): the shipped UI's language picker (`ui/src/screens/settings/areas/language/`) — does any doc claim the Rust fallback chain backs it? locales.ts:16's comment implies the crate scans `locales/`, but the UI hard-codes `["en-US"]`.
