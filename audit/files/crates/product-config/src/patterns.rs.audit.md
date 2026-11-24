# AUDIT: crates/product-config/src/patterns.rs
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 325 / 12 (+ 11 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES, outside the release lane: apps/control-server `main.rs` `run` -> `load_product` (main.rs:131-156) -> `product_config::Schema` parse (`Schema::from_json` -> `check_pattern_coverage` -> `matcher_for`, schema.rs:74-93) and `load_file` -> validate -> `matcher_for` -> `is_*` (schema.rs:271-276). NOT in the lumen-cli/desktop release lane (no app in that lane depends on product-config); also exercised by product-config's own test suite (fixtures_matrix). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p product-config --lib patterns` -> 11 passed, 0 failed; `rg '"pattern"' product.schema.json` -> exactly 10 patterns, all byte-identical to the 10 registered keys (11th registry entry is test-only); sed verification of all quotes |

## 1. One-paragraph verdict
A disciplined hand-rolled regex-replacement module: every one of the 10 pattern strings in product.schema.json has a byte-exact registry key (verified by rg against the schema), each matcher was traced against its regex and found semantically equivalent including the tricky cases (greedy `[a-z]{2,3}` lead in BCP-47, GUID group splitting, `{{` escape alternation, first-hyphen split for semver prerelease), schema drift fails loudly at load (`check_pattern_coverage` walks the JSON and errors on any unmatched pattern — verified in schema.rs:74-93), and the unit tests assert both accepts and rejects. The only defects found are hygiene-level: two matchers are more permissive than the regex they mirror for newline characters, and the coverage test hand-copies the schema pattern list instead of reading the schema.

## 2. What actually works (verified)
— `cargo test -p product-config --lib patterns` -> `test result: ok. 11 passed; 0 failed` (all matcher rule tests + registry test).
— `rg '"pattern"' product.schema.json` -> 10 hits, each string byte-identical to a `matcher_for` arm (patterns.rs:20-40); `Schema::load`/`from_json` rejects unknown schema patterns (schema.rs:79 `matcher_for(pattern).is_none()` -> `SchemaError::UnmatchedPattern`), so registry/schema drift is a typed error, not silence.
— Semantic-equivalence traces (read, all passed): `1.2.3-dev-beta` rejected by both (regex class lacks `-`; split_once pre-check rejects); `"abcd"` rejected by both (regex `{2,3}` caps the lead run, hand `lead=4` fails the 2..=3 range); GUID `"…701-xx"` and 11-hex-digit tails rejected by both; `"{{"` consumed as escape by both while `"{"` alone fails both; non-ASCII bytes fail `is_hex_color` byte ranges exactly as `[0-9a-f]` would.

## 3. Findings
F-001 [S4]
Where: crates/product-config/src/patterns.rs:205-207
Code: (verbatim)
```
fn is_https_url(s: &str) -> bool {
    s.starts_with("https://") && s.len() > "https://".len()
}
```
What is wrong: the mirrored regex `^https://.+$` (schema L289) under standard semantics (JS/Rust regex) does not let `.` match `\n`, so `https://\n` and `https://a\nb` FAIL the schema pattern but PASS this matcher (a TOML multi-line string can carry a raw newline). The hand matcher is more permissive than the contract it claims to implement. Why it matters: any consumer believing "schema-regex validated" gets values the schema would reject; realistic only for hostile/corrupt product.toml. What correct looks like: `s[8..].chars().all(|c| c != '\n') && !s[8..].is_empty()`. Confidence: HIGH — Verification: read (UNVERIFIED by execution — no test exercises newline URLs).
F-002 [S4]
Where: crates/product-config/src/patterns.rs:209-211
Code: (verbatim)
```
fn is_repo_relative_path(s: &str) -> bool {
    !s.is_empty() && !s.starts_with('/')
}
```
What is wrong: same newline divergence (`^[^/].*$` — `.*` stops at `\n`, so `"a\nb"` fails the regex, passes the matcher). Additionally the mirrored pattern itself is weak — `C:\Users\x` and `\\server\share` pass (first char is not `/`) — but that is the schema's contract, faithfully mirrored, not this file's lie. What correct looks like: reject embedded `\n`; raise the schema-pattern weakness with the schema owner. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/product-config/src/patterns.rs:14-16, 300-314
Code: (verbatim)
```
/// Every pattern string used by `product.schema.json`, with its matcher.
/// Keep in sync with the schema; the `schema_coverage` test plus the
/// `Schema::load` check enforce this mechanically.
```
What is wrong: two doc/test imprecisions. (a) The registry also contains `"^[a-z]+$"` -> `is_lowercase_ascii_token` which is NOT in product.schema.json (rg: 10 patterns only); it exists for a schema.rs test fixture (schema.rs:543) — the L39 comment admits this, but the module doc "Every pattern string used by product.schema.json" overstates. (b) `every_matcher_is_registered_under_its_schema_string` (L300-323) hand-copies the 10 strings into a local array; it cannot detect a NEW schema pattern lacking a matcher (only `Schema::load`'s walk can, and does). The test name promises schema-wide coverage it does not mechanically have. What correct looks like: read product.schema.json in the test and assert every pattern resolves. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| matcher_for | 17-43 | REAL | S4 | 11 arms; 10 mirror schema (byte-verified), 1 test-only; unknown -> None |
| is_short_name | 45-48 | REAL | — | char-count 1..=12; equivalent to regex |
| is_display_version | 50-68 | REAL | — | first-hyphen split; `-dev-beta` rejected by both; `dev..` accepted by both (regex class allows '.') |
| is_reverse_dns | 70-97 | REAL | — | first seg no hyphen, later segs hyphen-ok, ≥1 later; equivalent |
| is_guid | 99-116 | REAL | — | 8-4-4-4-12 hex; trailing/duplicate hyphen tails rejected; equivalent |
| is_title_template | 118-138 | REAL | — | UTF-8 safe advancement; `{{` escape; lone `{`/`}` rejected; equivalent |
| is_hex_color | 140-147 | REAL | — | byte-level, len-gated before slice; equivalent |
| is_msgid | 149-163 | REAL | — | ≥2 segments [a-z0-9-]; equivalent |
| is_bcp47 | 165-203 | REAL | — | greedy lead == regex {2,3} cap (lead>3 fails both); subtags 2..=8; equivalent |
| is_https_url | 205-207 | REAL | S4 | F-001 newline divergence |
| is_repo_relative_path | 209-211 | REAL | S4 | F-002 newline divergence + mirrors weak pattern |
| is_lowercase_ascii_token | 213-215 | REAL | — | test-fixture-only matcher; exercised via schema.rs:543 |
| tests (11 fns) | 217-324 | REAL | S4 | accept+reject pairs for every matcher; F-003 hand-copied list |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| short_name_rules … repo_path_rules (10) | YES | each asserts ≥2 accepts and ≥2 rejects (incl. boundary lengths, case, missing separators) |
| every_matcher_is_registered_under_its_schema_string | PARTIALLY | catches a matcher_for arm deleted for one of the 10 listed strings; does NOT catch a schema pattern added without a matcher (hand-copied list — F-003); `matcher_for("^unknown$").is_none()` guard is real |
Real drift protection lives in `Schema::from_json`/`check_pattern_coverage` (schema.rs:74-93) — verified by reading; the schema.rs test suite (fixtures) exercises the walk.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "JSON Schema regex is not available without a regex engine, and none is pre-audited" (L4-5) | module doc | REAL | Cargo.toml deps: toml + thiserror only (no regex crate); deny.toml/preaudit rationale not re-verified (UNVERIFIED — out of scope) |
| "`Schema::load` walks the schema and fails loudly if any pattern lacks a matcher" (L8-9) | module doc | REAL | schema.rs:74-93 `check_pattern_coverage` -> `SchemaError::UnmatchedPattern` |
| "Every pattern string used by `product.schema.json`, with its matcher" (L14) | module doc | REAL (with one extra test-only entry — F-003a) | rg: 10 schema patterns == 10 mirrored arms |

## 7. Dead / duplicated / unreachable code
- No dead matchers: all 11 arms reachable (10 via schema validation, 1 via schema.rs:543 test fixture).
- Duplication: the 10 pattern strings exist in three places (product.schema.json, matcher_for arms, test array L303-314) — mechanical sync relies on check_pattern_coverage, which covers 2 of the 3.

## 8. Dependencies & flags
No external deps beyond crate-level toml/thiserror. No unsafe, no panics (all slice/index accesses gated by length checks — `bytes[1..]` behind `bytes.len() == 7`), no feature gates. Linear-time matchers, no allocation.

## 9. Open questions for the next agent
- Schema owner: `^[^/].*$` for repo_path accepts Windows-absolute and UNC paths — intentional (repo-relative only checked on POSIX-style) or a hole the config-validator compensates for? (config_validator.rs checks brand asset paths exist — a Windows-absolute path would fail existence, so practical impact is low.)
- If a regex crate is ever pre-audited (the module's stated reason for hand-writing), this file should be deleted, not maintained — confirm the Phase-4 dependency decision covers it.
