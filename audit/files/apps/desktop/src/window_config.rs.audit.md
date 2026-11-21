# AUDIT: apps/desktop/src/window_config.rs
| Field | Value |
|---|---|
| Auditor | W13-A10 |
| Date | 2026-11-21 |
| Lines / functions | 475 / 14 (1 Display impl, 3 WindowConfig methods, 6 free fns, 5 tests; plus 3 type defs) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES for `parse_window_table`/`build_spec` (call chain: `lumen-desktop` binary apps/desktop/src/main.rs:31-33 parses product.toml at startup and prints the spec; the tauri wrapper apps/desktop-tauri/src/app.rs:243-248 applies the spec at window build — wrapper build UNVERIFIED in sandbox, baseline #2). `render_media_title` and `idle_title_key`: NO production caller (test-only) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "parse_window_table\|build_spec\|WindowBuildSpec\|render_media_title\|WindowTitle" apps/desktop/src apps/desktop-tauri/src` (callers: main.rs:17,31-33; app.rs:94,243-248; render_media_title zero production callers); `sed -n '235,275p' apps/desktop-tauri/src/app.rs` (spec consumption: set_size/set_min_size/maximize, title NOT applied); `sed -n '26,60p' apps/desktop/src/main.rs` (skeleton binary consumption); `rg -n "window_config" audit/runs/baseline/test-workspace.log.txt` (all 5 tests ok :2342-2346 in run #3 AND no-failfast :2301-2305); `rg -n "min_width\|title_template" product.toml` (shipped values 1280/720/940/600 match the tests' assertions); Read/sed on every quoted range (verbatim verified) |

## 1. One-paragraph verdict
A small, genuinely tested std-only TOML-subset reader for product.toml's `[window]` table, consumed at startup by the workspace-shipped `lumen-desktop` skeleton binary and at window build by the tauri wrapper (unverified build, sandbox limitation). The parser is real, its five tests all passed in BOTH baseline test runs (the desktop lib section ran before run #3's failure), and the shipped-file tests parse the real product.toml so config drift is loud. Two claims are false: the module header advertises a "comment- and quote-aware" reader while `strip_comment` cuts at the first `#` even inside a quoted value (a valid TOML title like `"Track #1 — {product_name}"` silently truncates to `"Track"` — a guess, exactly what the header forbids six lines later), and `render_media_title`'s doc claims "The wrapper calls this on `media.info-ready`-class updates" when no production code calls it at all (the wrapper defers title binding: app.rs:74-75 "until then the conf's first-paint title stands").

## 2. What actually works (verified)
- All 5 tests → `ok` in `audit/runs/baseline/test-workspace.log.txt:2342-2346` (run #3, desktop lib) and `orchrun-test-workspace-nofailfast.log.txt:2301-2305`.
- `lumen engine-info`-class consumption verified by code trace: `lumen-desktop` main.rs:31 `parse_window_table(include_str!("../../../product.toml"))` → `build_spec()` → printed; tauri app.rs:243-258 `apply_window_spec` feeds `set_size`/`set_min_size`/`maximize` from the spec. The shipped product.toml values (1280/720/940/600) asserted by the tests match the real file (product.toml:63-72).
- Typed error surface exercised: `bounds_and_cross_fields_are_typed_errors` asserts exact `BadNumber`/`BadBool`/`UnknownTemplateVariable`/`MissingKey` variants.

## 3. Findings
### F-001 [S3]
Where: apps/desktop/src/window_config.rs:12-13 and 310-318
Code:
```
//! is line-oriented, section-scoped, comment- and quote-aware; every
//! deviation is a typed [`WindowConfigError`], never a guess.
```
and
```
/// Strip a `#` comment (quote-aware enough for this table: the two
/// string values carry no `#` in the shipped product.toml, and a `#`
/// inside quotes would be a validator concern, not the shell's).
fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(at) => &line[..at],
        None => line,
    }
}
```
What is wrong: The module header claims the reader is "comment- and quote-aware" and that "every deviation is a typed `WindowConfigError`, never a guess". `strip_comment` is not quote-aware: it cuts at the first `#` anywhere, including inside a quoted string. For `title_template = "Track #1 — {product_name}"` (valid TOML — `#` inside a basic string is not a comment), the line becomes `title_template = "Track `, `parse_string` yields `"Track"`, and `parse_window_table` returns `Ok` with a silently truncated template. The function's own doc admits the limitation and punts to "the validator" — but no validator sees the [window] table here (the config-validator xtask checks assets, baseline #10, not this table's strings).
Why it matters: The header's honesty rule is violated by the very function it describes: a valid config edit produces a silently wrong window title with zero error. Blast radius is limited to [window] string fields with `#` (the shipped file has none), so S3 not S2.
What correct looks like: Track in-quote state while scanning for `#` (skip `#` between unescaped quotes), or reject any `#` inside a quoted value with `UnexpectedShape` — a typed refusal instead of a guess.
Confidence: HIGH — Verification: read (traced the exact truncation path through split_once/parse_string/validate).

### F-002 [S3]
Where: apps/desktop/src/window_config.rs:118-127
Code:
```
    /// Render the title template with media open (the `{product_name}`
    /// / `{media_title}` variables — the only two [`validate`]
    /// admits). The wrapper calls this on `media.info-ready`-class
    /// updates and sets the window title to the result.
    #[must_use = "pure reporter"]
    pub fn render_media_title(&self, product_name: &str, media_title: &str) -> String {
```
What is wrong: The doc claims a production consumer ("The wrapper calls this on `media.info-ready`-class updates and sets the window title to the result"). rg over apps/desktop, apps/desktop-tauri, and ui finds ZERO production callers — only the test `media_titles_render_through_the_template` (and the re-export at lib.rs:115). The tauri wrapper explicitly defers: apps/desktop-tauri/src/app.rs:74-75 "The idle-title → l10n-catalog binding joins with the presenter card; until then the conf's first-paint title stands" — `apply_window_spec` applies sizes/maximize only, never a title.
Why it matters: The function is UNREACHABLE from any shipped entry point while its doc asserts live wiring — a reader auditing title behavior would believe media-title updates reach the window. Same class as the engine-info literal: docs asserting integration the code does not have.
What correct looks like: Either wire the call in the wrapper's media-info-ready path or change the doc to "[PROPOSAL]: the presenter card's binding — unwired today".
Confidence: HIGH — Verification: read + rg (no caller in apps/desktop-tauri/src, apps/desktop/src, ui/src).

### F-003 [S4]
Where: apps/desktop/src/window_config.rs:241 and 345-347
Code:
```
            // Unknown keys inside [window] are tolerated the way TOML
            // tolerates them: forward-compat, not an error. The
            // config-validator owns full-schema strictness.
            _ => {}
```
and
```
fn parse_string(value: &str) -> String {
    value.trim_matches('"').to_string()
}
```
What is wrong: Two TOML-conformance gaps: (a) duplicate keys silently last-win (TOML forbids duplicates — `width = 1280` twice parses fine); (b) `trim_matches('"')` strips ALL leading/trailing quotes (`""x""` → `x`; an unmatched closing quote is silently kept). Also `idle_title_key()` (lines 113-116) has no production caller — the spec carries the key via `WindowTitle::Idle`, making the accessor test-only.
Why it matters: Limited blast radius (the shipped file is well-formed; the config-validator owns strictness), but the parser's deviations from TOML are undocumented at the claim level ("the way TOML tolerates them" is true for unknown keys, false for duplicates).
What correct looks like: Track seen keys and error on duplicates (`UnexpectedShape`), single-pair quote stripping, drop or test-mark the idle accessor.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `WindowConfigError` (enum + `Display` + `Error`) | 27-87 | REAL | — | 5 typed variants; Display messages carry key/reason; no panics. |
| `WindowConfig` (struct) | 91-108 | REAL | — | 8 flat fields matching product.toml [window]. |
| `WindowConfig::idle_title_key` | 113-116 | REAL | S4 | Test-only accessor (F-003c); must_use. |
| `WindowConfig::render_media_title` | 118-127 | REAL-UNPROVEN (test-verified) | S3 | Pure two-pass replace; sequential order prevents substitution loops; NO production caller (F-002). |
| `WindowConfig::build_spec` | 136-146 | REAL | — | Verbatim spec; consumed by main.rs:33 + app.rs:248; must_use. |
| `WindowTitle` (enum) | 153-164 | REAL | — | Idle{key}/Media{rendered}; Media arm only constructed by tests (wrapper never renders). |
| `WindowBuildSpec` (struct) | 170-183 | REAL | — | Consumed sizes/maximize/remember_geometry (title unused by wrapper). |
| `parse_window_table` | 191-269 | REAL | S3 | Line-oriented; section-scoped via `in_window`; `split_once('=')` else UnexpectedShape; array/table values refused; required keys enforced (no default-invented window — verified by test); unknown keys tolerated (documented); duplicate keys last-win (F-003a). |
| `validate` | 272-308 | REAL | — | Cross-field min<width/height; template walk admits exactly `{product_name}`/`{media_title}`; unclosed brace → typed error; byte-index slicing safe (ASCII braces in UTF-8 are single-byte — find returns char boundary for these). |
| `strip_comment` | 313-318 | REAL | S3 | Not quote-aware despite header claim (F-001). |
| `parse_u32` | 320-332 | REAL | — | Typed parse + range check; negative values fail parse with a slightly imprecise "not an integer" message. |
| `parse_bool` | 334-343 | REAL | — | Strict true/false; typed BadBool. |
| `parse_string` | 345-347 | REAL | S4 | Over-strips quotes (F-003b). |
| `tests::the_shipped_product_toml_parses` | 355-368 | REAL | — | Parses the REAL product.toml via include_str; drift is loud. |
| `tests::bounds_and_cross_fields_are_typed_errors` | 370-421 | REAL | — | Exact variant assertions incl. the range sentence. |
| `tests::other_sections_and_comments_do_not_leak` | 425-437 | REAL | — | [brand] `accent_color = "#4f8cff"` exercises the `#` path but only OUTSIDE [window] — the in-table `#`-in-quotes case (F-001) is untested. |
| `tests::the_shipped_product_toml_yields_the_build_spec` | 443-459 | REAL | — | Real file → exact spec fields. |
| `tests::media_titles_render_through_the_template` | 463-474 | REAL | — | Both variables, both directions, from the real template. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| the_shipped_product_toml_parses | YES | Real-file parse with exact field assertions (1280/720/940/600, template, idle key). |
| bounds_and_cross_fields_are_typed_errors | YES | Exact error equality on 5 mutations (range, cross-field, bool, template var, missing key). |
| other_sections_and_comments_do_not_leak | YES | Asserts width from the [window] section while [brand] carries a `#`-prefixed value. |
| the_shipped_product_toml_yields_the_build_spec | YES | Exact spec tuple/title assertions from the real file. |
| media_titles_render_through_the_template | YES | Exact rendered strings both directions. |
Coverage gap: no test for `#` inside a quoted [window] value (F-001's silent-truncation path), none for duplicate keys.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the reader is line-oriented, section-scoped, comment- and quote-aware; every deviation is a typed error, never a guess" | module docs :12-14 | FALSE (quote-aware half) | strip_comment:313-318 cuts inside quotes (F-001). |
| "the shell consumes exactly this struct when the tauri wrapper builds the WebviewWindowBuilder" | module docs :2-4 | REAL | app.rs:243-258 applies spec sizes/maximize (wrapper build itself UNVERIFIED, baseline #2). |
| "the [window] table is CONSUMED at window build, not just parsed" (4.H1 handoff c) | build_spec doc :129-134, main.rs:26-28 | REAL | main.rs:31-33 + app.rs:243-248. |
| "The wrapper calls this [render_media_title] on media.info-ready-class updates" | :120-121 | FALSE | Zero production callers; app.rs:74-75 defers title binding (F-002). |
| "min_width < width, min_height < height" schema | module docs :18-19 | REAL | validate:272-290 enforces; test covers. |
| "unknown variables are a validation error" | module docs :20-21 | REAL | validate:300-304; test asserts `os_name` refusal. |

## 7. Dead / duplicated / unreachable code
- `render_media_title` — no production caller (F-002); test-only today.
- `idle_title_key` — test-only accessor (spec carries the key).
- `WindowTitle::Media` — never constructed outside tests.
All three are pub API of a lib consumed by the (unverified-build) tauri wrapper, so they are prospective seams rather than rot; flagged, not counted as unreachable-file.

## 8. Dependencies & flags
std-only (fmt only) — consistent with the crate's C5 std-only claim (no toml crate; the reader IS the documented seam for a future TOML dep). No feature gates; compiled into the workspace `desktop` lib unconditionally (baseline #1 built it). Tests use include_str! of the real product.toml (no fixture copies).

## 9. Open questions for the next agent
- apps/desktop-tauri/src/app.rs owner: when the presenter card lands, wire `render_media_title` (F-002) and confirm the conf's `windows[0]` title never diverges from `title_idle_key`.
- Is any xtask validator supposed to catch `#`-in-quotes in [window] strings (F-001's "validator concern" punt)? `cargo xtask config-validator` (baseline #10) fails on missing brand icons, not string syntax — nobody owns this today.
