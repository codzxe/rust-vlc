# AUDIT: crates/localization/src/message.rs#part1 (lines 1–537)
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-21 |
| Lines / functions | 537 (of 1070; part2 = 538–1070 is W11-A07's) / 20 |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `localization` crate has ZERO dependents: `rg 'localization' apps/*/Cargo.toml crates/*/Cargo.toml` → only `crates/localization/Cargo.toml` itself (plus a prose comment in `crates/playlists/Cargo.toml:5`). Call chain for the code that DOES reach it: `crates/localization/tests/*` → `localization::{catalog,fallback,audit}` → `Message::parse` (catalog.rs:129) / `Message::render` (fallback.rs:133) — tests only. `lumen` CLI / `lumen-tauri` never link the crate. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 (F-005 added after plural.rs cross-analysis — same audit session) |
| Commands run | `rg -n "localization" --glob Cargo.toml` (dependents: none) · `rg -n "Part::CountSelf"` (created only at :624, plural-arm ctx) · `sed -n` on every quoted range (verbatim verification) · `rg -n "SUBCOMMANDS" tools/xtask/src/cmd/mod.rs` + `sed -n '190,260p'` (no `l10n-audit` subcommand) · `rg -n "localization" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (message::tests 8/8 ok, fallback test FAILED) · `rg -n "canonical" crates/localization/src/locale.rs` (language subtag lowercased → :456 safe) |

## 1. One-paragraph verdict
The rendering half of the message-format v1 engine is genuinely implemented and genuinely tested — every render path is panic-free by construction (no unwrap/expect/index; NaN/inf/huge-float all degrade), and all 8 `message::tests` passed in the no-fail-fast baseline (`orchrun-test-workspace-nofailfast.log.txt:2708-2740`). But the entire mechanism is a ghost in the shipped product: no crate or app depends on `localization`, the only shipped catalog (`locales/en-US.toml`) uses zero select/plural/percent placeholders, and the runtime UI renders from a completely different hand-written TypeScript catalog (`ui/src/i18n/messages.ts`). Doc 07 §6.4's claimed CI driver `cargo xtask l10n-audit` does not exist in the xtask subcommand registry. Within the chunk, one real diagnostic-quality defect (select type mismatch reported as "missing argument") and one fabricated error offset.

## 2. What actually works (verified)
- `Message::render` never panics on bad args: verified by reading all of `render`/`render_into`/`render_placeholder` (no unwrap/expect/panic; NaN → `PluralCategory::Other` at :507, inf → Other, huge integral float → saturating `as i64` → valid category) and by `message::tests::rendering_never_panics_on_bad_args ... ok` (orchrun-test-workspace-nofailfast.log.txt:2714).
- `format_percent(0.45) == "45%"`, `format_percent(0.4567) == "45.67%"` etc.: `message::tests::number_formatting_helpers ... ok` (same log, :2711), matching the module-doc claim at :9.
- `Message::parse` round-trips all four v1 forms with exact expected text: `parses_all_v1_forms`, `renders_simple_and_typed_placeholders`, `renders_select_and_plural` all ok (same log, :2708-2740).
- CountSelf `None` arm at :425 is unreachable by construction: `Part::CountSelf` is pushed only under `ParseContext::PluralArm` (:623-625, part2), and plural arms always render with `Some(arg)` (:522). Verified by reading both sites.

## 3. Findings

### F-001 [S3] `select` placeholder with a non-string argument is misdiagnosed as `ArgMissing`
Where: crates/localization/src/message.rs:476-480
Code:
```rust
        PlaceholderKind::Select(arms) => {
            let Some(Arg::Str(value)) = find_arg(args, name) else {
                report_missing(out, diagnostics);
                return;
            };
```
What is wrong: if the caller supplies `("format", Arg::Int(5))` for `{format, select, …}`, `find_arg` returns `Some` but the `let`-else pattern fails, so the code falls into `report_missing` and records `RenderDiagnostic::ArgMissing` — "The placeholder's argument was not supplied" (:226-227) — which is false: the argument WAS supplied, with the wrong type. The sibling branches do this correctly: `NumberPercent` emits `ArgTypeMismatch` (:467-473) and `Plural` emits `ArgTypeMismatch` (:508-515).
Why it matters: `RenderDiagnostic` is the only debugging signal the "degrade visibly" contract (:20-22) provides; a developer chasing "missing argument" for an argument they demonstrably passed loses time. The rendered text is identical either way, so this is diagnostic corruption, not output corruption.
What correct looks like: match on `find_arg` returning `Some(Arg::Str(v))` / `Some(_)` → `ArgTypeMismatch { expected: "string" }` / `None` → `ArgMissing`, mirroring the `NumberPercent` arm.
Confidence: HIGH — Verification: read; no test covers select-with-int (test gap: `renders_select_and_plural` :1015-1033 and `rendering_never_panics_on_bad_args` :1035-1053 only pass Str args to selects).

### F-002 [S3] Entire chunk is dead code from the shipped product's perspective — declared, tested, consumed by nothing
Where: crates/localization/src/message.rs:274-286
Code:
```rust
impl Message {
    /// Parse a message source string (the value side of a catalog entry).
    pub fn parse(src: &str) -> Result<Message, MessageError> {
        let mut parser = Parser { src, pos: 0 };
```
What is wrong: the public API of this file (`Message::parse`, `render`, `placeholder_set`, `plural_arms_by_name`) is reachable only inside the `localization` crate itself: `Catalog::from_toml_str` (catalog.rs:129) and `Localizer::translate` (fallback.rs:133), which are in turn invoked only by the crate's own tests (`tests/seed_integrity.rs`, `tests/audit_proof_matrix.rs`, unit tests). No `apps/*` or `crates/*` manifest declares a dependency on `localization` (`rg 'localization' apps/*/Cargo.toml crates/*/Cargo.toml` → only the crate's own manifest). The shipped UI gets its strings from `ui/src/i18n/messages.ts` — a separate, hand-written TypeScript catalog with a disjoint key set.
Why it matters: this is the exact "config declared but never read" pattern this wave is chartered to find — a full message-format engine (parse + plural + percent + fallback) whose only consumers are its own tests, while the real product ships a parallel hand-rolled catalog. Every hour spent hardening this engine is spent on a component the binary never links into a feature.
What correct looks like: either the UI/i18n pipeline consumes `localization` (Phase-4 plan per doc 07 §6.2 `[PROPOSAL]`), or the crate is explicitly marked parked. As shipped: verdict UNREACHABLE.
Confidence: HIGH — Verification: read + rg (no dependents); baseline build `cargo build --release --workspace` compiles it (build-release-workspace.log.txt) but linking is not reachability.

### F-003 [S4] `MessageError::offset()` fabricates offset 0 for `ConflictingPlaceholderType`, contradicting the enum's own contract
Where: crates/localization/src/message.rs:216-217
Code:
```rust
            | MessageError::DuplicateArm { offset, .. } => *offset,
            MessageError::ConflictingPlaceholderType { .. } => 0,
        }
```
What is wrong: the enum doc (:152-154) promises "Variants carry the byte `offset` of the offending token within the message source so callers can map them to file positions", but `ConflictingPlaceholderType` carries no offset and `offset()` hard-returns 0. Its consumer `catalog_message_error` (catalog.rs:235-243) feeds `error.offset()` into `line_col`, so a conflicting-type error is reported at column 1 of the value — a fabricated position, not the offending placeholder's.
Why it matters: one error class always points at the wrong column; users editing catalogs chase a position that is meaningless.
What correct looks like: carry the second (conflicting) placeholder's byte offset in the variant, or document the variant as position-less and have `catalog_message_error` omit the column for it.
Confidence: HIGH — Verification: read of both files.

### F-004 [S4] Plural count is silently dropped for `select` arms nested inside plural arms — `#` divergence from ICU beyond the documented limitation
Where: crates/localization/src/message.rs:488
Code:
```rust
            if let Some((_, sub)) = chosen {
                sub.render_into(out, diagnostics, locale, args, None);
```
What is wrong: a select sub-message nested in a plural arm renders with `count: None`. The parser (part2 :623-625) correspondingly treats `#` inside such a select arm as literal text, so `{n, plural, other {# of them: {s, select, a {#} other {#}}}}` prints the inner `#` characters literally. In ICU MessageFormat, `#` is the plural argument throughout its sub-messages, including nested selects. The module doc (:16-19) documents the brace-escape arm limitation but not this one.
Why it matters: whoever ports real ICU messages in Phase 4 will hit silent mis-rendering (literal `#`) with no diagnostic; the doc gap hides it.
What correct looks like: propagate `count` into select-arm `render_into` calls and accept CountSelf in `SelectArm`-under-`PluralArm` context, or add one sentence to the module doc naming this v1 limitation.
Confidence: HIGH — Verification: read of render (:488) + parser (:601-635); no test covers nested select-in-plural `#`.

### F-005 [S3] Silent plural-category fallback: the documented `PluralCategoryMissing` diagnostic is dead code — the case it describes renders with NO diagnostic
Where: crates/localization/src/message.rs:517-531
Code:
```rust
            let chosen = arms
                .iter()
                .find(|(arm, _)| *arm == category)
                .or_else(|| arms.iter().find(|(arm, _)| *arm == PluralCategory::Other));
            match chosen {
                Some((_, sub)) => sub.render_into(out, diagnostics, locale, args, Some(arg)),
                None => {
                    // Unreachable: parse enforces the `other` arm.
                    diagnostics.push(RenderDiagnostic::PluralCategoryMissing {
```
What is wrong: the `or_else` makes `chosen` always `Some` (parse enforces an `other` arm), so the `None` branch — the ONLY emitter of `RenderDiagnostic::PluralCategoryMissing` — is unreachable, as the comment admits. But the variant's own doc (:234-236) says it reports "The selected plural category has no arm in this message; the `other` arm was used instead" — that is precisely the `or_else` fallback case, and THAT case emits nothing. The reachable path: an unknown-to-the-seed-table language (e.g. `tr`) with count 1 — `select_plural_category` (plural.rs:113) returns `One`, audit rule 5 required only an `other` arm for unknown languages (audit.rs:231-234), the catalog legally has only `other`, and the render silently uses the `other` arm with zero diagnostics.
Why it matters: the module contract (:20-22) is "every problem … records a [`RenderDiagnostic`] and degrades visibly"; this problem class is swallowed. A translator auditing why "1 item / 1 items" renders the plural form in an unseeded language gets silence from the diagnostics channel. (Root cause shared with plural.rs's render/audit asymmetry — cross-file finding filed there.)
What correct looks like: emit `PluralCategoryMissing` when the category arm is absent and the `other`-arm fallback is used (drop the `or_else`, emit in the missing-category branch, keep an `other`-absent branch for the truly unreachable case), or delete the variant and its doc if silent fallback is the intended v1 behavior.
Confidence: HIGH — Verification: read of :517-531 + plural.rs:110-116 + audit.rs:226-258; no test covers unknown-language count-1 rendering (gap).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Arg::to_display` | 44-50 | REAL | — | Str clones per render (alloc; fine at l10n scale); Int/Float correct |
| `From<&str> for Arg` | 53-57 | REAL | — | trivial, correct |
| `From<String> for Arg` | 59-63 | REAL | — | trivial, correct |
| `From<i64> for Arg` | 65-69 | REAL | — | trivial, correct |
| `From<f64> for Arg` | 71-75 | REAL | — | trivial, correct |
| `format_float` | 78-84 | REAL | — | integral <1e15 → `{:.0}`; `-0.0` renders "-0" (cosmetic); NaN/inf fall through to Display, no panic; test-covered |
| `format_percent` | 88-101 | REAL | — | rounds to 2 decimals of the percent, trims zeros; `-0.001` → "-0%" (cosmetic); inf/NaN → "inf%"/"NaN%", no panic; 7 asserts in `number_formatting_helpers` |
| `PlaceholderKind::tag` | 119-126 | REAL | — | stable tags, test-covered (`placeholder_kind_tags`) |
| `Display for PlaceholderKind` | 130-133 | REAL | — | delegates to `tag` |
| `Message::parse` | 276-286 | REAL (unreachable entry) | S3 | see F-002; validates trailing `}` → `UnmatchedClose` and type conflicts |
| `Message::placeholder_set` | 291-295 | REAL (unreachable) | — | feeds audit rule 3 (audit.rs tests); no shipped catalog consumer |
| `Message::plural_arms_by_name` | 299-303 | REAL (unreachable) | — | feeds audit rule 5; **no key in the only shipped catalog uses a plural** — dead in practice |
| `Message::collect_plural_arms` | 305-327 | REAL | — | recursive; nested select/plural arms included |
| `Message::collect_placeholders` | 329-348 | REAL | — | recursive; BTreeMap insert (same name+same tag idempotent; conflicts rejected at parse) |
| `Message::validate_placeholder_types` | 351-363 | REAL | — | reports only the FIRST conflict (`into_iter().next()`) — subsequent conflicts hidden until the first is fixed (acceptable) |
| `Message::collect_conflicts` | 365-395 | REAL | — | correct cross-nesting detection (arm placeholders vs top-level) |
| `Message::render` | 400-408 | REAL | — | panic-free verified (see §2) |
| `Message::render_into` | 410-432 | REAL | — | `CountSelf`+`None` arm (:425) unreachable by construction — verified against parser :623-625 |
| `find_arg` | 435-439 | REAL | — | linear scan (args lists are tiny); duplicate names: first wins, no diagnostic (minor) |
| `render_placeholder` | 441-534 | REAL | S3 (F-001, F-005), S4 (F-004) | locale language subtag lowercased upstream (locale.rs:90) so :456 is safe; integral-float `as i64` (:505) saturates, never truncates — benign; NaN/inf → `Other` (:507), no panic |

## 5. Test honesty
Tests for this file live in part2 (`#[cfg(test)] mod tests`, :865-1070 — W11-A07's scope); the ones exercising part1 functions:
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `number_formatting_helpers` (:1055) | YES | exact-string asserts on `format_float`/`format_percent` (7 cases incl. 0.455→"45.5%") |
| `renders_simple_and_typed_placeholders` (:1002) | YES | exact output "Hello, Lumen! 45%" + `diagnostics.is_empty()` |
| `renders_select_and_plural` (:1015) | YES | exact outputs for plural one/other and select known/fallback-to-other |
| `rendering_never_panics_on_bad_args` (:1035) | YES | asserts degraded text + exact diagnostic counts (2 missing / 1 mismatch) |
| `placeholder_kind_tags` (:1066) | YES | exact tags |
| GAP | — | no test passes a non-Str arg to a `select` (would have caught F-001); no test nests a select inside a plural arm with `#` (would have exposed F-004); no test renders a plural in an unseeded language with count 1 (would have exposed F-005's silent fallback) |
All 8 passed: `orchrun-test-workspace-nofailfast.log.txt:2708-2740` (44 passed / 1 failed in `-p localization --lib` — the failure is `fallback::tests::missing_everywhere…`, fallback.rs's owner's file, see §6 row 3).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Rendering never panics: every problem … records a [`RenderDiagnostic`] and degrades visibly" | message.rs:20-22 (module doc) / doc 07 §6.3 | REAL | read of all render paths + `rendering_never_panics_on_bad_args` ok (no-failfast log) |
| "v1 subset: `{name}`; `{name, number, percent}`; `{name, select, …}`; `{name, plural, …}`" | doc 07 §6.2 (:667) | REAL but UNREACHABLE from shipped entry points | `parses_all_v1_forms` ok; crate has zero dependents (F-002) |
| "The chain is total: a missing translation can never crash rendering" | doc 07 §6.3 | REAL at the message layer; WEAKENED at fallback layer | message layer verified; but the fallback-layer test `missing_everywhere_records_diagnostic_and_never_panics` FAILED in the release-profile baseline (renders bare key, not `missing:<key>`; orchrun log :2727-2736) — fallback.rs owner's finding, cross-noted |
| "Run by `cargo xtask l10n-audit` (backed by `crates/localization::audit_catalogs`)" | doc 07 §6.4 (:678) | **FALSE** (wiring) / REAL (functions) | xtask SUBCOMMANDS registry (tools/xtask/src/cmd/mod.rs:221+) has no `l10n-audit`; `audit_catalogs` runs only in the crate's own tests (seed_integrity 3/3 ok, audit_proof_matrix 14/14 ok) |
| doc 07 §6.2's own example `"player.error.unsupported" = "…{format, select, known {…} unknown {…}}"` (no `other` arm) | docs/07-config-driven-system.md §6.2 example block | **FALSE** (self-contradictory) | the parser the same doc specifies REQUIRES `other` (message.rs:184-186 `MissingOtherArm`); the doc's example message would be rejected by the engine the doc mandates — for the doc-07 owner, cross-noted here |

## 7. Dead / duplicated / unreachable code
- Whole chunk: unreachable from shipped entry points (F-002). `plural_arms_by_name` (:299) additionally has zero users of its data in the only shipped catalog (no plural keys in `locales/en-US.toml`).
- `to_display` (:44) is `fn`-private and used by `render_into`/:422 and `render_placeholder`/:459 — fine.
- No duplication inside the chunk; the UI's parallel i18n (ui/src/i18n/messages.ts + l10n-check.mjs) is an EXTERNAL duplication of purpose (different mechanism, disjoint keys) — belongs to the UI/docs owners.

## 8. Dependencies & flags
- Crate deps: `toml`, `thiserror` only (crates/localization/Cargo.toml; both pre-audited per its header). No feature flags gate this file. `[lints] workspace = true` inherited.
- Baseline: `cargo build --release --workspace` exit 0 (compiles); `cargo clippy --workspace --all-targets -D warnings` exit 0 (clean); tests: see §5. No cfg-gating hides this code — it simply has no callers.

## 9. Open questions for the next agent
- W11-A07 (message.rs part2): does any parser path create `Part::CountSelf` outside `ParseContext::PluralArm`? (I verified no — :623-625 — but you own the full parser.)
- Owner of fallback.rs: is the release-profile `missing:<key>` test failure (orchrun log :2727) a test defect (assumes debug) or a real "degrade visibly" violation in release builds? The shipped product IS release.
- Owner of docs/07: §6.2's `player.error.unsupported` example lacks the mandatory `other` arm — the doc contradicts the engine it specifies.
- Wave gate: is ANY plan on record to wire `localization` into a shipped surface, or is `locales/en-US.toml` + engine a permanently parked parallel system next to `ui/src/i18n/messages.ts`?
