# AUDIT: crates/product-config/src/patterns.rs
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-21 |
| Lines / functions | 324 / 13 (+11 tests) |
| File verdict | UNREACHABLE (real, correct, executed-in-CI code; no SHIPPED entry point reaches it — reached only by the non-shipped `lumen-control-server` binary and tests) |
| Reachable from shipped entry point? | NO from the shipped trio (`lumen` cli, `desktop`, `lumen-desktop-tauri` — packaging gate ships exactly those, scripts/packaging/gate.sh:63-90). YES from `lumen-control-server`: apps/control-server/src/main.rs:135-154 `load_product_config` → `product_config::load_file(path, &schema)` → schema.rs:272 `matcher_for(pattern)` → the matchers here. `lumen-tauri` reads `product.toml` (app.rs:243 `include_str!`) but through its own hand-rolled `[window]` parser (apps/desktop/src/window_config.rs), BYPASSING this schema/pattern layer entirely — see F-003. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n '"pattern"' product.schema.json` (exactly 10 pattern strings, all present in the registry) · `rg -n "matcher_for" crates/product-config/src/*.rs` (callers: schema.rs:79 coverage check, schema.rs:272 validate) · `rg -n "product_config" crates/settings-schema/ crates/plugin-api/` (NO code usage — their Cargo.tomls mention product-config in comments only; sole dependent = apps/control-server) · `rg -n "product.toml|ProductConfig" apps/cli/src apps/desktop/src apps/desktop-tauri/src` (tauri reads [window] only, via window_config.rs; CLI mirrors constants) · `rg -n "patterns::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (11/11 ok at :4628-4639) · `sed -n` on every quoted range |

## 1. One-paragraph verdict
This is the rare file where the hand-rolled-no-regex-engine workaround is done right: every one of the 11 matchers was checked against the regex it claims to implement and all are semantically equivalent on the interesting edge cases (GUID group walk, reverse-DNS first-segment asymmetry, title-template alternation with `{{` escapes, UTF-8 boundary safety), the schema→registry sync fails loudly (`Schema::from_json` → `check_pattern_coverage`, schema.rs:70-91), and all 11 unit tests plus the control-server e2e (which loads the real product.toml through this matcher set) passed in the no-fail-fast baseline. The problems are around the file, not in it: the validation layer it implements is bypassed by the one shipped binary that actually reads product.toml (`lumen-tauri` uses a weaker parallel validator that accepts what this file rejects — F-003), one registry entry exists for a pattern that is not in the schema (F-001), and two matchers diverge from regex semantics on line terminators (F-002).

## 2. What actually works (verified)
- All 10 schema pattern strings in `product.schema.json` (rg `"pattern"` → :28,34,52,66,122,139,279,284,289,294) have registry entries here with byte-identical (JSON-unescaped) keys; `Schema::from_json` walks the schema and errors on any unmatched pattern (schema.rs:70-91, read) — the module-doc claim at :8-9 holds.
- `patterns::tests` 11/11 ok — audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4628-4639.
- End-to-end execution on REAL data: control-server e2e `e2e_full_session_through_the_app_stack ... ok` + `repo_product_today_refuses_the_feature_gate ... ok` (same log :1486-1496, :1458) — both load the committed root `product.toml` through `product_config::load_file` → schema validate → these matchers (e2e.rs:36-90).
- Regex-equivalence of every matcher, verified by reading each against its pattern: `is_short_name` (char-count + ASCII class), `is_display_version` (first-hyphen split is safe: a hyphen in the core can only make it fail, matching `[0-9]+(\.[0-9]+){2}`), `is_reverse_dns` (first segment excludes hyphen, later segments allow it — exactly `^[a-z0-9]+(\.[a-z0-9-]+)+$`), `is_guid` (5-group walk; extra/short tails rejected; `chars().count()` + `is_ascii_hexdigit`), `is_title_template` (alternation order and the `[^{}]` brace exclusion reproduced exactly; `starts_with` before slicing = no mid-char panic), `is_hex_color` (len-guarded byte slice), `is_msgid`, `is_bcp47` (lead 2-3 lowercase, subtags 2-8 alnum, byte-walk panic-free), `is_https_url`, `is_repo_relative_path` — except the line-terminator cases in F-002.

## 3. Findings

### F-001 [S4] Registry contains a pattern that is not in `product.schema.json`; the doc claim "Every pattern string used by product.schema.json" overstates, and nothing detects dead entries
Where: crates/product-config/src/patterns.rs:39-41
Code:
```rust
        // Simple lowercase-ASCII token (used by schema test fixtures).
        "^[a-z]+$" => is_lowercase_ascii_token,
        _ => return None,
```
What is wrong: `^[a-z]+$` appears nowhere in `product.schema.json` (rg `"pattern"` → 10 strings, none is it); its only user is the inline TEST fixture schema in schema.rs:543 (inside `#[cfg(test)] mod tests`). The module doc (:14-15) says the registry holds "Every pattern string used by `product.schema.json`" and that sync is "enforce[d] … mechanically" — but the enforcement is one-directional: `Schema::from_json` catches a schema pattern missing from the registry (schema.rs:79), while a registry entry missing from the schema (this one, or any future deleted pattern) is silently retained.
Why it matters: dead matchers accumulate; the doc's "mechanically enforced" claim is half-true; a reader auditing "which patterns does product.toml actually live under" gets an inflated list.
What correct looks like: either drop the fixture pattern from the registry (test fixture schemas can use a matcher via a test-only path), or add a reverse-coverage test asserting every registry key occurs in `product.schema.json`.
Confidence: HIGH — Verification: read + rg (schema has 10 patterns; registry has 11 keys).

### F-002 [S4] `is_https_url` and `is_repo_relative_path` accept line terminators that the schema regexes reject — the hand matchers are not fully equivalent to the pattern language they implement
Where: crates/product-config/src/patterns.rs:205-211
Code:
```rust
fn is_https_url(s: &str) -> bool {
    s.starts_with("https://") && s.len() > "https://".len()
}
```
(and, same defect class, `is_repo_relative_path` at :209-211: `!s.is_empty() && !s.starts_with('/')`)
What is wrong: JSON Schema `pattern` is defined over ECMA-262 regex, where `.` excludes `\n`, `\r`, `\u2028`, `\u2029`. `^https://.+$` therefore rejects `"https://\n"` and any URL containing a newline; the matcher accepts it (len 9 > 8). Likewise `^[^/].*$` rejects `"a\nb"` (`.*` cannot cross the newline) while `is_repo_relative_path("a\nb")` is true.
Why it matters: today the divergence is latent (the same matchers are the only enforcement, so validation is self-consistent); the day a real regex engine replaces them — the stated plan, :4-5 "JSON Schema regex is not available without a regex engine" — previously-accepted configs start failing, or, viewed from the other side, the schema document promises stricter validation than the code delivers for pathological values.
What correct looks like: match the `.` semantics explicitly (reject `\n`/`\r`/`\u2028`/`\u2029` in the tail) or document the divergence in the module doc.
Confidence: HIGH — Verification: read (ECMA-262 `.` semantics are standard); no test covers newline tails (gap).

### F-003 [S3] The `title_template` rule implemented here is NOT the rule enforced on the shipped `lumen-tauri` path — the shipped validator is a weaker duplicate that accepts what this rejects
Where: crates/product-config/src/patterns.rs:118-138
Code:
```rust
fn is_title_template(s: &str) -> bool {
    // Zero or more of: "{product_name}", "{media_title}", a non-brace
    // char, or "{{". Unknown {variables} match no alternative. Advancement is
    // always on UTF-8 char boundaries (never slices mid-char).
```
What is wrong: `lumen-tauri` — the one shipped binary that reads `product.toml` — parses the `[window]` table with its own line-oriented reader (apps/desktop-tauri/src/app.rs:243 `include_str!("../../../product.toml")` → apps/desktop/src/window_config.rs) and re-implements the title-template check at window_config.rs:291-308. That re-implementation only scans for `{`…`}` pairs: a template containing `}` with no matching `{` (e.g. `"bad } brace"`) is ACCEPTED there but is REJECTED by the schema pattern implemented here (`[^{}]` excludes both braces, verified by test `title_template_rules` :259). The two implementations also differ on `{{` escapes (both reject `{{product_name}}` in practice, but via different reasoning paths). The schema-side rule this file implements is therefore decorative for the shipped product: the shipped path never runs it.
Why it matters: two validators for one product rule = drift by construction; the weaker one is the one that ships. A future product.toml whose title passes window_config.rs but violates product.schema.json will behave inconsistently between the shipped binary and every CI tool that DOES run the schema (control-server e2e, product-config tests).
What correct looks like: window_config.rs consumes the same rule (shared matcher, or the tauri shell gains the product-config dependency), or one of the two rules is deleted.
Confidence: HIGH — Verification: read of both validators (patterns.rs:118-138 vs window_config.rs:272-308); note the shipped value `"{media_title} — {product_name}"` (product.toml:82) passes both, so no live defect today.
*(Anchor note: the divergence itself lives in window_config.rs — flagged here because THIS file holds the declared rule; window_config.rs's owner should file the mirror finding.)*

### F-004 [S4] Test precision gaps: three matcher mutations no test would catch
Where: crates/product-config/src/patterns.rs:255-260, 279-284, 293-297
Code:
```rust
    fn title_template_rules() {
        assert!(is_title_template("{media_title} — {product_name}"));
        assert!(is_title_template("plain title"));
        assert!(!is_title_template("{unknown_var} — {product_name}"));
        assert!(!is_title_template("trailing } brace"));
    }
```
What is wrong: (a) no test asserts `is_title_template("a{{b") == true`, so deleting the `{{` escape branch (:128-129) leaves the suite green; (b) `bcp47_rules` (:279-284) asserts only lead-length 2 and 7 — a mutation of `!(2..=3)` to `!(2..=4)` stays green (no `!is_bcp47("engl")` assert), and subtag length bounds (1, 9 chars) are untested; (c) `repo_path_rules` (:293-297) never asserts the empty string is rejected, so removing the `!s.is_empty()` guard stays green.
Why it matters: the whole file's value proposition is exactness against a regex; its tests don't pin the exact boundaries that are most likely to be mutated wrong.
What correct looks like: add the six missing boundary asserts ({{ accepted; `engl`, 1- and 9-char subtags rejected; `""` rejected for repo path).
Confidence: HIGH — Verification: read + mutation reasoning (no code edited).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Matcher` (type alias) | 12 | REAL | — | plain fn pointer |
| `matcher_for` | 17-43 | REAL | S4 (F-001) | registry; keys byte-match the JSON-decoded schema patterns (verified against product.schema.json's 10 `pattern` values); one extra non-schema key `^[a-z]+$` |
| `is_short_name` | 45-48 | REAL | — | `chars().count()` + ASCII class ≡ `^[A-Za-z0-9-]{1,12}$` |
| `is_display_version` | 50-68 | REAL | — | first-hyphen split safe: hyphen in core ⇒ core fails the 3-part digit check, same as regex; `-` empty pre rejected; test-covered |
| `is_reverse_dns` | 70-97 | REAL | — | first segment `[a-z0-9]`, later `[a-z0-9-]`, ≥1 later segment — exact regex semantics incl. `"dev."`/`"dev"` rejection |
| `is_guid` | 99-116 | REAL | — | 8-4-4-4-12 walk; extra tail, short group, empty group all rejected; ASCII hex both cases; panic-free (split_once + chars) |
| `is_title_template` | 118-138 | REAL | S3 (F-003 — shipped path uses a divergent duplicate) | alternation + `{{` escape + `[^{}]` reproduced exactly; every loop branch advances; UTF-8 boundary-safe slicing |
| `is_hex_color` | 140-147 | REAL | — | len-guarded byte slice ≡ `^#[0-9a-f]{6}$` |
| `is_msgid` | 149-163 | REAL | — | ≡ `^[a-z0-9-]+(\.[a-z0-9-]+)+$` (first segment may be all hyphens, same as regex) |
| `is_bcp47` | 165-203 | REAL | — | byte-walk, `.get()` only — panic-free; lead 2-3 lowercase, subtags 2-8 alnum; multibyte rejected via ASCII class checks; test boundaries incomplete (F-004b) |
| `is_https_url` | 205-207 | REAL | S4 (F-002) | accepts newline tails the regex rejects |
| `is_repo_relative_path` | 209-211 | REAL | S4 (F-002) | same class; empty-string rejection untested (F-004c) |
| `is_lowercase_ascii_token` | 213-215 | REAL (dead in schema terms) | S4 (F-001) | ≡ `^[a-z]+$`; only caller is the registry entry for a pattern that exists solely in schema.rs's test fixture (:543) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `short_name_rules` :222 | YES | rejects space, empty, 16-char; accepts mixed case |
| `display_version_rules` :231 | YES | 2-part and trailing-hyphen rejected |
| `reverse_dns_rules` :240 | YES | `"dev"` (no dot) and `"dev."` (trailing dot) rejected — catches `later>=0` and empty-segment mutations |
| `guid_rules` :248 | YES | short-by-one group rejected |
| `title_template_rules` :255 | PARTIAL | unknown var + trailing `}` rejected, but the `{{` acceptance branch is never asserted (F-004a) |
| `hex_color_rules` :263 | YES | case/prefix/length all pinned |
| `msgid_rules` :271 | YES | single-segment rejected |
| `bcp47_rules` :279 | PARTIAL | lead 2 and 7 pinned; lead 4 and subtag length 1/9 not (F-004b) |
| `https_url_rules` :287 | YES | scheme + empty tail |
| `repo_path_rules` :293 | PARTIAL | rooted path pinned; empty string not (F-004c) |
| `every_matcher_is_registered_under_its_schema_string` :300 | YES | 10 schema strings resolve; unknown string → None |
All 11 ran green: orchrun-test-workspace-nofailfast.log.txt:4628-4639.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The registry below maps the *exact schema pattern string* to its matcher; `Schema::load` walks the schema and fails loudly if any pattern lacks a matcher (schema drift becomes a typed error, not silence)" | patterns.rs:7-9 | REAL (one-directional) | schema.rs:70-91 `check_pattern_coverage` called from `Schema::from_json`; reverse direction NOT enforced (F-001) |
| "Every pattern string used by `product.schema.json`, with its matcher. Keep in sync with the schema; … enforce this mechanically" | patterns.rs:14-16 | SHALLOW | registry holds 11 keys, schema holds 10 patterns — the extra is a test-fixture pattern; "mechanical" enforcement is half the sync |
| "JSON Schema regex is not available without a regex engine, and none is pre-audited" | patterns.rs:4-5 | REAL | no regex crate in crates/product-config/Cargo.toml deps (toml, thiserror, serde_json family only) |
| xtask config-validator header: "`pattern` constraints are NOT enforced — std has no regex. The `product-config` crate's committed CI suite owns them (msgid/bcp47/https/hex shapes)" | tools/xtask/src/cmd/config_validator.rs:16-18 | REAL | config_validator.rs has no matcher_for import; the crate's tests + control-server e2e do run them (baseline log :4628-4639, :1486-1496) |

## 7. Dead / duplicated / unreachable code
- Dead-in-schema: `is_lowercase_ascii_token` + its registry key (F-001).
- Duplicated product rule: title-template validation exists here (schema authority) AND in apps/desktop/src/window_config.rs:291-308 (shipped authority, weaker) — F-003.
- Unreachable from the shipped trio; single real consumer binary is `lumen-control-server` (not in the packaging payload — scripts/packaging/gate.sh:63-90 ships lumen-cli, desktop, lumen-desktop-tauri only).

## 8. Dependencies & flags
- No new deps introduced by this file (pure std). Crate deps per Cargo.toml: toml/thiserror/serde_json family — all pre-audited. No feature gates. `[lints] workspace = true`.
- Baseline: workspace build exit 0; clippy `-D warnings` exit 0; the 11 unit tests + control-server e2e green in the no-fail-fast run (cited above). `cargo xtask config-validator` (baseline #10, exit 1) fails ONLY on brand icons — its own header admits it never checks patterns, so that RED result is not this file's failure.

## 9. Open questions for the next agent
- Owner of apps/desktop/src/window_config.rs: the title-template loop (:291-308) accepts `}`-without-`{` that product.schema.json forbids — file the mirror of F-003 from the shipped side.
- Owner of apps/desktop-tauri/src/app.rs: is there a plan to route `[window]` through product-config's schema (patterns included) when the toml-crate dossier is admitted, replacing the std-only reader?
- Wave gate: control-server loads product.toml but its help/identity strings are hard-coded ("identity wiring pending", main.rs:110) — who owns the rebrand-propagation card that would make product.toml (and this file) load-bearing for a SHIPPED binary?
