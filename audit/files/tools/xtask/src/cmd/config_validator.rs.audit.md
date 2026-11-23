# AUDIT: tools/xtask/src/cmd/config_validator.rs
| Field | Value |
|---|---|
| Auditor | W01-A10 |
| Date | 2026-11-23 |
| Lines / functions | 1007 / 10 top-level fns + `json` module (Json, ParseError, parse, Parser×11, utf8_len) + 3 unit tests |
| File verdict | REAL (works, executed, 17/17 integration tests pass) — but unwired (nothing in CI/gate runs it) and with one proven silent-pass hole |
| Reachable from shipped entry point? | NO (dev tool; not in release lane). YES from `cargo xtask config-validator` via cmd/mod.rs:143-149 — executed |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 4 |
| Commands run | `./target/debug/xtask config-validator` → exit 1 (4 violations, matches baseline); 3 seeded-variant runs (enum hole proof); `./target/debug/deps/config_validator-3991d8058e10136f` → 17/17 pass; unit tests via baseline cargo-test-workspace.txt:6602-6604 |

## 1. One-paragraph verdict
A genuinely working std-only validator: a JSON-subset parser (recursive-descent, depth-capped, surrogate-aware), a JSON-Schema-subset walker (required/unknown keys via additionalProperties, types, enums, not.enum, min/max, minItems/maxItems+items, $ref→$defs), and cross-file checks (brand assets, locale/theme/profile joins, msgid existence, title variables, telemetry→privacy, migration-chain contiguity, PascalCase app ids). I ran it: on the real tree it reports exactly the baseline's 4 violations (missing assets/brand/*) with exit 1, and its 17 fixture-based integration tests all pass against the real binary. Two real defects: (1) it is wired into NOTHING — no CI workflow, not `scripts/full_gate.sh`, not xtask `gate` — while product.toml's own header and doc 07 §9 claim it is build-breaking/merge-blocking; the tree is red on it right now; (2) PROVEN by execution: enum constraints are silently skipped for non-string values (`subtitle_mode = 3`, `hw_decode_policy = true` pass clean). Beyond that, its cross-file charter misses the one cross-file rule its own schema promises (accent_color ↔ theme token) — a rule the tree violates today (blue #4f8cff in product.toml vs orange #d37918 actually shipped), and its theme/profile join checks probe a `themes/*.toml` plane that does not exist (the theme system landed as themes/tokens.json), so those checks can never fire on this tree.

## 2. What actually works (verified)
- `./target/debug/xtask config-validator` (real root) → 4 violations (brand.icon_ico/icns/png_512, brand.logo_svg: `does not exist under the root`), `result: FAILURE (4 violation(s))`, exit 1 — identical to audit/runs/baseline/xtask-config-validator.txt:11-15.
- Positive control run: `subtitle_mode = "bogus-mode"` variant → `<root>.playback.subtitle_mode: "bogus-mode" not in enum ["auto", "forced-only", "on", "off"]` — the enum check works for strings.
- Integration tests: 17/17 pass (ran the prebuilt binary; also baseline).
- Unit tests (json parser shapes/truncation, pascal_case): pass — baseline cargo-test-workspace.txt:6602-6604.
- Delegation claims are real: the `product-config` crate's structural pass enforces enum type-agnostically (`crates/product-config/src/schema.rs:179-181` `!variants.contains(value)`) and its test `root_product_toml_passes_semantic_none` (tests/semantic.rs:279-291, via `common::load_root_product()` = `load_file(root product.toml, root_schema)`) validates the REAL product.toml in `cargo test`.
- msgid existence works: no violations for description_key/title_idle_key; reference catalog locales/en-US.toml parsed (no "unreadable" note).

## 3. Findings
F-001 [S2] "Build-breaking" validator wired into nothing; two authoritative docs claim CI enforcement that does not exist.
Where: product.toml:4-7; docs/07-config-driven-system.md:838-851; tools/xtask/src/cmd/config_validator.rs:1-5
Code (product.toml header, verbatim):
```
# Single source of truth for product decisions (MP §5). Validated by
# product.schema.json via `cargo xtask config-validator`; a violation is a
# build-breaking error with an actionable diagnostic (MP §5), never a silent
# fallback.
```
And doc 07 §9 (L838, L844): "**Job sequence** (each step gates the next; the whole chain is merge-blocking on Windows/Linux/macOS per MP §13): … 4  cargo xtask config-validator            (C4 — schema + semantics + D-rules)".
What is wrong: grep of `.github/workflows/*` for config-validator: zero hits; `scripts/full_gate.sh` runs check-architecture/fixtures/lint-literals/check-identity/sbom/web-budget (verified by grep of the script) — no config-validator; xtask `gate` steps (gate.rs:188-280) don't include it. The validator is RED on main (4 violations) and has been for a documented while. Mitigation, verified: docs/release-checklist.md:55 is honest — "RED today — 4 violations… `scripts/full_gate.sh` does not run this validator… do not rely on the gate to catch it". So the release process knows; product.toml's and doc 07's claims are what's false. (Also stale: the checklist's "9 steps" — the current full_gate has 11 sections incl. web-budget and packaging.)
Confidence: HIGH — Verification: ran (tool + grep of all wiring sites).

F-002 [S3] PROVEN: enum constraints silently skipped for non-string values.
Where: tools/xtask/src/cmd/config_validator.rs:285-299 (enum checked only in the `TValue::Str` arm)
Code (verbatim):
```
    match value {
        TValue::Str(s) => {
            if let Some(json::Json::Arr(items)) = lookup(schema_obj, "enum")
                && !items.is_empty()
                && !items.iter().any(|j| j.as_str() == Some(s.as_str()))
```
What is wrong: `subtitle_mode`/`hw_decode_policy`/`crash_reports` schemas carry `enum` with no `type` (idiomatic; product.schema.json:176-178, 238), so an Int/Bool value skips the type gate and lands in the Int/Bool arm where no enum check exists. Executed proof (scratch copies under audit/scratch/W01-A10/, run with `--product`): `subtitle_mode = 3` → same 4 brand violations, NO enum violation; `hw_decode_policy = true` → same; string control `"bogus-mode"` → correctly flagged. A hand-edited product.toml with a numeric/boolean enum field validates clean through this tool. Mitigation: the delegated product-config pass catches it on the real file (schema.rs:179-181 is type-agnostic; runs in cargo test). The tool's doc table (L11) claims "enum" checking without this qualifier, and the 17-test suite never seeds a non-string enum value.
Confidence: HIGH — Verification: ran (3 variant runs, outputs above).

F-003 [S3] Missing check its own schema promises: brand.accent_color must equal the theme token accent — and the tree violates it today.
Where: product.schema.json:140 (hint); product.toml:92-95; themes/tokens.json:22-24; ui/src/tokens/generated/tokens.css:31
Code (schema hint, verbatim):
```
"x-lumen-hint": "hex RGB with leading '#', lowercase, e.g. #4f8cff; must equal theme token color.accent.primary once token extraction lands (doc 07 §2.3)."
```
What is wrong: token extraction HAS landed (themes/tokens.json + generated ui tokens exist), and the two sources disagree: product.toml `accent_color = "#4f8cff"` (blue) vs themes/tokens.json `"default-accent": "orange"` → `--color-accent: #d37918` in the shipped UI CSS. No check anywhere ties them (grep: no accent cross-check in tokens.rs, gate scripts, or this validator). The one config value that names the product's accent color is wrong and decorative — its only consumers are rebrand and the literal-scan gates. This is exactly the class of cross-file rule this tool exists to enforce.
Confidence: HIGH — Verification: ran (greps) + read (all four files).

F-004 [S3] The theme/profile join checks probe a plane that does not exist — they can never fire on this tree.
Where: config_validator.rs:462-480; product.toml:106; themes/ (actual contents)
Code (verbatim):
```
    for (key, dir) in [
        ("defaults.theme", "themes"),
        ("defaults.profile", "profiles"),
    ] {
```
What is wrong: the check enumerates `themes/*.toml` stems, but the theme system landed as `themes/tokens.json` + `ui/src/tokens/generated/*` — there are no per-theme .toml files and none planned in that shape. On the current tree the check always degrades to the note "themes/ has no *.toml candidates — enum injection skipped until the plane lands" (baseline line 9; my run), i.e. `defaults.theme = "dark"` is unverifiable by this tool and by the crate's `SemanticInputs::none()` pass. product.toml's comment "Theme id: must match themes/<id>.toml" documents a convention that does not exist. The check is dead weight testing the wrong shape; the note calls a LANDED plane "parked".
Confidence: HIGH — Verification: ran (note in output) + read (themes/ contents, product.toml).

F-005 [S4] Brand-asset existence uses `.exists()` — a directory passes.
Where: config_validator.rs:446-448
Code (verbatim):
```
        if let Some(p) = get_str(key)
            && !root.join(p).exists()
```
What is wrong: `assets/brand/icon.ico` as a directory would pass existence; no `is_file()`. Magic bytes are delegated (documented, and the crate owns them) — the file-vs-dir distinction is not delegated to anyone.
Confidence: HIGH — Verification: read (`Path::exists` semantics).

F-006 [S4] JSON parser accepts invalid JSON.
Where: config_validator.rs:915-931 (number), 885-894 (raw string chars), 934-941 (utf8_len)
What is wrong: `01`, `1.`, `+1` all parse (f64::from_str accepts them; JSON forbids); literal control characters inside strings are accepted; `utf8_len` maps continuation bytes 0x80-0xBF to length 4 (mis-split codepoints become U+FFFD, no panic). Blast radius: first-party schema file only; no security impact; would silently accept a corrupt schema.
Confidence: HIGH — Verification: read + code trace.

F-007 [S4] S and X halves contradict each other on `{{` escapes.
Where: config_validator.rs:500-513 vs product.schema.json:122
Code (schema pattern, verbatim):
```
"pattern": "^(\\{product_name\\}|\\{media_title\\}|[^{}]|\\{\\{)*$",
```
What is wrong: the schema pattern explicitly allows `{{` (escaped brace), but the X variable-scan would flag `{{product_name}}` as unknown variable `{product_name`-ish. No current data uses `{{`; the two checks were written to different rules.
Confidence: HIGH — Verification: read (both).

F-008 [S4] Migration-chain gaps: first entry's `from` is unconstrained.
Where: config_validator.rs:535-558
What is wrong: entries `[{from: 3, to: 4}]` with `schema_version = 4` pass — nothing requires the chain to start at 1 or at the pre-migration version. The schema $comment (product.schema.json:271) promises "from < to, strictly ascending, no gaps" — the head of the chain is a gap this tool can't see.
Confidence: HIGH — Verification: read + code trace.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run | 52-162 | REAL | — | args, parse failures → exit 1, xtask errors → exit 2; executed |
| build_tree | 170-180 | REAL | — | merges flat tables into tree |
| insert_table | 182-219 | REAL | — | scalar-overwrite case documented; duplicate-table merge inherited from toml_lite (see sibling audit of toml_lite.rs) |
| lookup | 225-227 | REAL | — | linear find |
| resolve_ref | 229-240 | REAL | — | `#/`-internal refs only; unresolvable ref → silent no-constraints (S4-adjacent, no current case) |
| walk | 242-425 | REAL (with hole) | S3 | F-002: enum only in Str arm; everything else verified by tests + runs |
| cross_file | 431-587 | REAL (with dead join) | S3 | F-003/F-004/F-005; asset/locale/msgid/title/telemetry/migrations/appid checks all fire correctly (tested) |
| list_toml_ids | 590-603 | REAL | — | *.toml stems, sorted |
| pascal_case | 607-623 | REAL | — | unit-tested |
| find_root | 629-646 | REAL | — | walk-up for product.toml |
| json::Json + accessors | 654-678 | REAL | — | |
| json::ParseError/Display | 680-692 | REAL | — | line:col from pos scan |
| json::parse | 694-706 | REAL | — | rejects trailing content |
| Parser::{err,peek,skip_ws,expect,literal} | 714-758 | REAL | — | |
| Parser::value | 760-783 | REAL | — | depth cap 64 |
| Parser::object/array | 785-838 | REAL | — | |
| Parser::string | 840-898 | REAL | — | escapes incl. surrogate pairs (unpaired → U+FFFD) |
| Parser::hex4 / number | 900-931 | REAL | S4 | number laxity (F-006) |
| utf8_len | 934-941 | REAL | S4 | continuation-byte miscount (F-006) |
| tests (json×2, pascal_case) | 943-1007 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| good_product_passes | YES | full clean fixture, exit 0 |
| unknown_key_fails | YES | seeded `short_nam` typo → "unknown key" |
| type_mismatch_fails | YES | width = "wide" |
| missing_required_key_fails | YES | removed short_name |
| reserved_name_fails | YES | name = "VLC" → not.enum |
| out_of_range_integer_fails | YES | width = 100 < 480 |
| enum_violation_fails | YES (strings only) | crash_reports = "always" — GAP: no non-string enum seed (F-002 went uncaught) |
| non_boolean_feature_flag_fails | YES | additionalProperties schema type |
| dangling_brand_asset_fails | YES | no assets written |
| unknown_locale_fails | YES | locale = "xx-XX" |
| theme_outside_directory_listing_fails | YES | two-phase: empty listing = note/exit 0, wrong id with theme present = violation |
| missing_msgid_fails | YES | unparseable catalog = note; missing key = violation |
| unknown_title_variable_fails | YES | {app_name} |
| wrong_schema_version_fails | YES | schema_version = 7 |
| wrong_app_id_fails | YES | PascalCase mismatch |
| missing_files_are_xtask_errors | YES | exit 2 |
| unparseable_product_toml_fails_with_exit_one | YES | exit 1 + "parse:" |
All 17 ran green via the prebuilt binary. The suite is genuinely mutation-style (seeds defects into an otherwise-good fixture); its one blind spot is F-002's non-string enum case.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a violation is a build-breaking error" | product.toml:4-7 | FALSE | nothing in any build/CI/gate runs it (F-001) |
| job-sequence step 4 config-validator, "merge-blocking" | docs/07-config-driven-system.md:838-851 | FALSE | no workflow/gate invocation (F-001) |
| C4 gate = `cargo xtask config-validator` | docs/07-config-driven-system.md:835 | FALSE-in-effect | exists but unwired; red on main |
| "RED today — 4 violations… full_gate.sh does not run this validator" | docs/release-checklist.md:55 | REAL | matches my run exactly |
| checks table S/X rows (L11-12) | module doc | REAL except enum-for-strings-only | executed; F-002 |
| "pattern constraints are NOT enforced — the product-config crate's committed CI suite owns them" | L16-18 | REAL | schema.rs reads pattern; semantic.rs:279 runs on the real file |
| v0 limit: features booleans only, F1/F2 needs registry | L19-20 | REAL | feature-audit owns R7 cross-check (baseline: it FAILED separately) |

## 7. Dead / duplicated / unreachable code
- Not dead, but the theme/profile join branch (cross_file L462-480) is unreachable-in-effect on this tree (F-004): always the note path.
- json module is a private re-implementation (xtask is std-only by design — C5); a third parser sibling (toml_lite) exists by necessity. Acceptable given the zero-dep constraint.

## 8. Dependencies & flags
- std-only; relies on crate::cmd::toml_lite (audited by sibling W01 agent: duplicate-key/first-wins and `"""`-string defects there propagate into this validator's view of product.toml — cross-ref audit/files/tools/xtask/src/cmd/toml_lite.rs.audit.md).
- `--root/--product/--schema` flags all functional (used by tests + my variant runs).

## 9. Open questions for the next agent
- Who consumes `brand.accent_color` at product level (rebrand.rs rewrite + lint-literals/check-identity allowlists) — if the UI accent is orange while the allowlisted "brand accent" is blue, do the literal gates compare UI literals against a value the product doesn't actually use? (lint_literals/check_identity owners.)
- Should the crate's `root_product_toml_passes_semantic_none` also assert the accent/token equality, or should config-validator grow it (F-003)?
- doc 07 §9 job sequence steps 5 (schema-gen --check) and 6 (l10n-audit) — do those commands exist and run anywhere? (main.rs dispatch auditor: me, next file.)
