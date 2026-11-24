# AUDIT: apps/desktop/src/window_config.rs
| Field | Value |
|---|---|
| Auditor | W13-A09 |
| Date | 2025-07-14 |
| Lines / functions | 475 / 9 non-test + 5 tests |
| File verdict | REAL (parser genuinely parses and rejects typed) — but two of its reporters are dead in the shipped build, and its "consumed at window build" claim is false today |
| Reachable from shipped entry point? | PARTIAL: parse_window_table/build_spec YES via `lumen-desktop` main.rs:31-33 (parses product.toml, PRINTS the spec, then exits 86 — no window is ever built); render_media_title/idle_title_key NO (zero non-test callers; rg across apps/ + crates/) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 1 |
| Commands run | `cargo test -p desktop --lib window_config` -> 5 passed, 0 failed |

## 1. One-paragraph verdict
A real, std-only, line-oriented `[window]`-table parser with typed errors for every deviation and five honest tests that pin the shipped product.toml itself (drift is loud). Its limits are real but disclosed: comment-stripping is quote-blind (a `#` inside a quoted value silently truncates it), and string parsing accepts unquoted values. The bigger problem is what surrounds it: the only consumer is the `lumen-desktop` skeleton binary, which parses the table, prints the build spec, and exits 86 (WEBVIEW_PENDING) — no `WebviewWindowBuilder` exists anywhere — so `render_media_title`/`idle_title_key` are dead code and the "CONSUMED at window build, not just parsed" claim is false today.

## 2. What actually works (verified)
— verified by `cargo test -p desktop --lib window_config` -> `test result: ok. 5 passed` (shipped product.toml pinned: 1280x720, min 940x600, template, idle key; typed errors for out-of-range, bad bool, unknown template var, missing key).

## 3. Findings
F-001 [S3]
Where: apps/desktop/src/window_config.rs:310-318
Code:
```
/// Strip a `#` comment (quote-aware enough for this table: the two
/// string values carry no `#` in the shipped product.toml, and a `#`
/// inside quotes would be a validator concern, not the shell's).
fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(at) => &line[..at],
```
What is wrong: Not quote-aware at all — the first `#` anywhere ends the line, including inside a quoted value: `title_template = "Take #1 — {media_title}"` parses to `Take ` (parse_string trims the orphan quote). The corruption is silent, not typed — contradicting the module's own rule "every deviation is a typed [`WindowConfigError`], never a guess" (L12-13). Why it matters: a config author putting `#` in the title gets a silently wrong window title instead of an error. What correct looks like: track in-quote state while scanning for `#`, or reject a `#` that appears before the value's closing quote.
Confidence: HIGH — Verification: read (behavior follows from `find('#')` with no quote state).

F-002 [S3]
Where: apps/desktop/src/window_config.rs:129-134 (claim) vs apps/desktop/src/main.rs:31-52,89-90 (reality)
Code:
```
    /// The build spec for window construction — the exact parameter
    /// set the ADR-004 wrapper feeds `WebviewWindowBuilder` (issue #3,
    /// 4.H1 handoff c: the `[window]` table is CONSUMED at window
    /// build, not just parsed).
```
What is wrong: There is no window build. The sole consumer (`lumen-desktop` main.rs) parses the table, prints the spec lines, then unconditionally returns `ExitCode::from(WEBVIEW_PENDING)` (86) — main.rs's own doc says "`0` never (there is no window to serve)". The claim "CONSUMED at window build, not just parsed" is false today; it is parsed and printed. Why it matters: doc-vs-reality — a reader believes the config drives a real window. What correct looks like: state "parsed and printed by the skeleton; consumed by the ADR-004 wrapper when it lands".
Confidence: HIGH — Verification: read both files; rg found no other build_spec caller.

F-003 [S3]
Where: apps/desktop/src/window_config.rs:113-127
Code:
```
    pub fn idle_title_key(&self) -> &str {
        &self.title_idle_key
    }
...
    pub fn render_media_title(&self, product_name: &str, media_title: &str) -> String {
```
What is wrong: Zero non-test callers anywhere in apps/ or crates/ (rg for `render_media_title|idle_title_key` outside this file: no matches). The "wrapper calls this on `media.info-ready`-class updates" (L120-121) wrapper does not exist. Dead-in-shipped-build public API presented as a live surface.
Confidence: HIGH — Verification: read + rg.
(Downgraded from S2 only because these are pure functions with tests, i.e. latent-correct, not wrong.)

F-004 [S4]
Where: apps/desktop/src/window_config.rs:345-347
Code:
```
fn parse_string(value: &str) -> String {
    value.trim_matches('"').to_string()
}
```
What is wrong: Accepts unquoted values (`title_template = foo` → `foo`) and strips every leading/trailing quote char (`"""x"""` → `x`) — lenient where TOML grammar is strict; a malformed config passes silently.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| WindowConfigError (enum) + Display + Error | 27-87 | REAL | — | 5 typed variants, all rendered |
| WindowConfig (struct) | 91-108 | REAL | — | 8 keys, schema documented |
| idle_title_key | 114-116 | UNREACHABLE | S3(F-003) | no non-test caller |
| render_media_title | 123-127 | UNREACHABLE | S3(F-003) | no non-test caller; logic itself correct |
| build_spec | 136-146 | REAL | S3(F-002) | called by skeleton main, printed only |
| WindowTitle (enum) | 153-164 | REAL | — | consumed by main.rs display |
| WindowBuildSpec (struct) | 170-183 | REAL | — | printed only today |
| parse_window_table | 191-269 | REAL | — | section-scoped, required-key or typed error, unknown keys tolerated (documented forward-compat) |
| validate | 272-308 | REAL | — | min<width cross-fields; template var whitelist; unclosed brace typed |
| strip_comment | 313-318 | REAL | S3(F-001) | quote-blind |
| parse_u32 | 320-332 | REAL | — | range enforced |
| parse_bool | 334-343 | REAL | — | strict true/false |
| parse_string | 345-347 | REAL | S4(F-004) | lenient quotes |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_shipped_product_toml_parses | YES | include_str! of the real product.toml, exact 8-value pin — config drift or parser regression fails it |
| bounds_and_cross_fields_are_typed_errors | YES | exact error variants for 5 mutation classes |
| other_sections_and_comments_do_not_leak | YES | [brand] section + inline comments parsed correctly |
| the_shipped_product_toml_yields_the_build_spec | YES | spec fields pinned from the real file |
| media_titles_render_through_the_template | YES | both variables substituted, ordering asserted |
All 5 executed green (`cargo test -p desktop --lib window_config`). Note: none covers the quote-blind `#` truncation (F-001) — the gap the "honest scope" comment papers over.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every deviation is a typed WindowConfigError, never a guess" | L12-13 | FALSE (for quoted `#` and unquoted strings) | F-001/F-004: silent truncation/acceptance |
| "the shell consumes exactly this struct when the tauri wrapper builds the WebviewWindowBuilder" | L3-4 | PAPER-ONLY | no tauri wrapper exists; main.rs exits 86 |
| "[window] table is CONSUMED at window build, not just parsed" | L130-134 | FALSE (today) | main.rs:31-52 prints, then exit 86 |

## 7. Dead / duplicated / unreachable code
- `render_media_title`, `idle_title_key`: unreachable from any shipped entry point (F-003).
- `WindowBuildSpec.maximized`/`remember_geometry` ride along to a printer that never applies them.

## 8. Dependencies & flags
std-only (fmt). No features. Compiled into the `desktop` lib (release lane builds it: baseline cargo-build-release-desktop.txt exit 0).

## 9. Open questions for the next agent
- When ADR-004 lands the tauri wrapper, do these tests move with it, and does the quote-blind comment stripper get fixed or does the validator own it? (config-validator baseline is currently FAILING on missing brand assets — nothing catches this today.)
