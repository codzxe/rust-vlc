# AUDIT: tools/xtask/src/cmd/toml_lite.rs
| Field | Value |
|---|---|
| Auditor | W01-A07 |
| Date | 2026-11-23 |
| Lines / functions | 612 / 18 non-test fns+impls, 6 tests (+Must trait) |
| File verdict | REAL (real parser, 5 live consumers, tests pass live; its own doc contract contains FALSE claims — see F-001/F-002/F-004) |
| Reachable from shipped entry point? | NO for the release lane. YES for dev/CI: main.rs:124-125 -> cmd/mod.rs:54 -> parse() called by feature_audit.rs:133, config_validator.rs:120+486, parity_report.rs:100, compat_matrix.rs:212+275, web_budget.rs:163+174 (all `cargo xtask` subcommands) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 3 |
| Commands run | `./target/debug/deps/xtask-e9d837ce498c05b0 toml_lite` -> 6 passed / 0 failed (exit 0); rg consumers (exit 0); sed quote verification (exit 0); baseline citations below |

## 1. One-paragraph verdict
A genuine std-only TOML-subset parser (xtask Cargo.toml has NO dependencies section — the C5 claim holds) that really parses the real product.toml, feature-matrix.toml, size-record.toml and locale catalogs: the baseline config-validator walked 16 tables of the real product.toml (xtask-config-validator.txt:8), parity-report and web-budget both reached post-parse stages on their real inputs. The core line scanner (comment stripping, string-aware `=` finding, multi-line array/inline-table continuation, escapes) is correct on everything I traced and every test passes — I ran the prebuilt test binary myself (6/6). But the module's headline contract — "Deliberately NOT supported (parse error, loudly ... never a silent skip)" — is FALSE in four distinct ways: single-line `"""x"""` silently parses to the wrong string, trailing garbage after table headers is silently dropped, duplicate tables/keys are silently accepted with inconsistent winner (get() first-wins, row iteration last-wins), and integer literals beyond i64 silently become floats. The test suite tests none of the promised rejections it fails to deliver.

## 2. What actually works (verified)
— Ran the prebuilt xtask test binary: `./target/debug/deps/xtask-e9d837ce498c05b0 toml_lite` -> `test result: ok. 6 passed; 0 failed` (comment stripping, matrix shape, multi-line arrays, nested tables, inline-table continuation, array-row order).
— Real-file parsing proven by baselines: product.toml walked to 16 tables (xtask-config-validator.txt:6-8, failures were brand assets only); docs/feature-matrix.toml parsed by parity-report (xtask-parity-report.txt ran to 0/252 verdicts — parse succeeded); size-record.toml parsed before web-budget's E0463 build failure (full_gate.txt:76-84).
— Escape handling (n/t/r/b/f/"/\\) and string-aware comment splitting traced correct incl. `#` inside quoted keys and escaped quotes before `#`.

## 3. Findings
F-001 [S3]
Where: tools/xtask/src/cmd/toml_lite.rs:14-17 (claim) vs 437-444 (code)
Code:
```
//! Deliberately NOT supported (parse error, loudly, with the line number —
//! never a silent skip): multi-line basic strings (`"""`), literal strings
```
```
    let body = t
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
```
What is wrong: `v = """x"""` does NOT error. parse_string strips exactly ONE leading and ONE trailing quote, so the body is `""x""` and the value silently parses as `Str("\"\"x\"\"")` — wrong content, no error. The doc promises this shape "fail[s] the tool, not lie[s]" — it lies. (The multi-LINE variant `"""\n...` errors only by accident — the continuation logic never triggers for quotes, so line 2 hits "expected `key = value`".) Why it matters: any hand-edited product.toml/locales/matrix row using TOML's standard multi-line string form gets silently corrupted values in every consuming gate (config-validator would validate garbage text as if it were the real string). What correct looks like: reject a value starting with `"""` before parse_string, or implement multi-line strings. Confidence: HIGH — Verification: read + code trace (strip_prefix/strip_suffix remove one quote each; deterministic).
F-002 [S3]
Where: tools/xtask/src/cmd/toml_lite.rs:170-191 and 192-208
Code:
```
            let Some(close) = trimmed.find("]]") else {
...
            let name = trimmed[2..close].trim().to_string();
```
```
            let Some(close) = trimmed.rfind(']') else {
...
            let path: Vec<String> = trimmed[1..close]
```
What is wrong: nothing after the chosen bracket is examined. `[meta] junk` silently drops ` junk` (rfind finds the first `]`... the only one) and parses as `[meta]`; `[a] junk ]` yields table path `["a] junk "]`; `[[a]b]]` yields array-row name `a]b` (find("]]") hits the last two chars). Strict TOML rejects all three. The doc's "never a silent skip" is false for header lines. Why it matters: a typo'd header in a hand-edited matrix/record silently produces a differently-named table that every `get()` then misses — the gate reports missing keys instead of the actual typo. What correct looks like: after extracting the header, verify the remainder is exactly `]`/`]]` (post comment-strip) else error. Confidence: HIGH — Verification: read + trace.
F-003 [S3]
Where: tools/xtask/src/cmd/toml_lite.rs:266-279 (attach) and 136-145 (get)
Code:
```
    } else if let Some(row) = doc.arrays.last_mut() {
        row.entries.push((key, value));
```
```
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
```
What is wrong: duplicate `[table]` headers and duplicate keys within a table/row are pushed, never rejected. `Document::get`/`table` use `find` → FIRST occurrence wins; consumers that iterate `row.entries` (web_budget parse_record L434-517) take the LAST — two consumers of the same duplicated key can read different values, and a duplicated section header silently shadows its second copy. Strict TOML rejects duplicates. Cross-ref web_budget.rs F-006. Why it matters: a duplicated `gzip_bytes` line or repeated `[[config]]`-adjacent table in a hand-mangled record is accepted, and the checker/rewriter pair can disagree about which copy is authoritative. What correct looks like: reject duplicate table paths and duplicate keys at attach time. Confidence: HIGH — Verification: read (both sides).
F-004 [S4]
Where: tools/xtask/src/cmd/toml_lite.rs:16-17 (claim), 43-45 (comment) vs 373-389 (code)
Code:
```
//! (`'...'`), dotted keys, dates, mixed-type arrays. None of the audited
//! files use them; a future file that does should fail the tool, not lie.
```
```
    /// Array of scalar values (strings/ints/bools — the files never mix).
    Array(Vec<Value>),
```
What is wrong: `v = [1, "a", true]` and `v = [[1]]` parse successfully — parse_value recurses per element with no type/uniformity check. The claim "mixed-type arrays ... parse error, loudly" is FALSE. Why it matters: permissive acceptance cannot corrupt current consumers (they type-check via as_str/as_int), but the module's contract is the opposite of its behavior, and the Value::Array doc comment ("the files never mix") states an input assumption as if it were an invariant. What correct looks like: either enforce scalar-only homogeneous arrays or fix the doc. Confidence: HIGH — Verification: read + trace.
F-005 [S4]
Where: tools/xtask/src/cmd/toml_lite.rs:423-429
Code:
```
    if let Ok(int) = t.parse::<i64>() {
        return Ok(Value::Int(int));
    }
    if let Ok(float) = t.parse::<f64>() {
        return Ok(Value::Float(float));
    }
```
What is wrong: an integer literal outside i64 range (e.g. `99999999999999999999`) falls through to the f64 branch and silently becomes `Value::Float(1e20)`; `as_int()` then returns None. Strict TOML errors on out-of-range integers. Current consumers fail loudly on the type mismatch (web_budget: "must be an integer, got float"), so the blast radius is a confusing message rather than a wrong number. What correct looks like: if the text is all-digits (with sign) but i64 parse fails, error "integer out of range". Confidence: HIGH — Verification: read.
F-006 [S4]
Where: tools/xtask/src/cmd/toml_lite.rs:199-202
Code:
```
            let path: Vec<String> = trimmed[1..close]
                .split('.')
                .map(|s| s.trim().to_string())
                .collect();
```
What is wrong: a quoted header segment — `[audio."device.config"]`, legal TOML — is naively split on '.', producing path segments `"device` and `config"` (quotes retained); `get("audio.device.config")` then never matches. Silent wrong-name table instead of a loud rejection (the subset supports quoted keys at ROOT level, L224-234, but not in headers). No audited file uses quoted header segments today. What correct looks like: reject quote characters in header paths loudly, or implement quoted segments. Confidence: HIGH — Verification: read + trace.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Display for ParseError | 31-35 | REAL | — | "line N: msg" |
| Value::as_str / as_int / as_table / type_name | 51-82 | REAL | — | plain accessors |
| Document::table | 112-117 | REAL | — | exact-path find; first-wins on duplicates (F-003) |
| Document::array_rows | 120-122 | REAL | — | source order, tested |
| Document::get | 126-146 | REAL | — | dotted lookup; first-wins (F-003); proven on real product.toml via baselines |
| Document::has_root_key | 150-152 | REAL | — | quoted catalog keys |
| parse | 156-262 | REAL | S3 | main scanner; header laxity F-002; continuation logic correct |
| attach | 266-279 | REAL | S3 | no duplicate rejection (F-003); table/array/root routing traced correct |
| strip_comment | 282-296 | REAL | — | string-aware, escape-aware; traced correct incl. `#` in quoted keys |
| find_top_level_eq | 299-309 | REAL | — | string-aware; quoted keys with '=' handled |
| brackets_closed | 313-329 | REAL | — | depth+string aware |
| join_until_closed | 333-359 | REAL | — | consumes to balance or loud EOF error; counts verified |
| parse_value | 362-434 | REAL | S4 | mixed/nested arrays accepted (F-004); int->float fallthrough (F-005); `"""` hole (F-001) |
| parse_string | 437-469 | REAL | S3 | one-quote strip each side = F-001; escapes subset correct; unterminated `\"` errors loudly |
| split_top_level | 472-495 | REAL | — | string/depth aware, correct |
| tests (6) | 497-611 | REAL | — | honest but avoid the doc's false promises (§5) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parses_matrix_shaped_document | YES | real shape: nested table get, array row, inline table, `#` inside string |
| parses_multiline_array_and_nested_tables | YES | asserts 3 items + dotted get |
| comment_after_hash_outside_string_is_stripped | YES | asserts Int(42) |
| rejects_unsupported_shapes_loudly | WEAK | tests only 4 shapes ('''raw''', date, bare key, no header); does NOT test `"""`, mixed arrays, or dotted keys — precisely the promises the code breaks (F-001, F-004) |
| inline_table_multiline_continuation | YES | asserts 2 entries across lines |
| array_rows_keep_source_order | YES | asserts [1,2,3] order |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "std-only by C5 (no toml/serde_json crates)" | toml_lite.rs:4 | REAL | tools/xtask/Cargo.toml has no [dependencies] |
| supports `[table]`, `[a.b.c]`, `[[rows]]`, key=value, strings/ints/floats/bools, single/multi-line arrays, inline tables, comments | toml_lite.rs:8-12 | REAL | 6 passing tests + real-file baselines (§2) |
| "multi-line basic strings (`\"\"\"`) ... parse error, loudly ... never a silent skip" | toml_lite.rs:14-16 | FALSE | F-001: `"""x"""` silently parses to `""x""` |
| "mixed-type arrays ... parse error, loudly" | toml_lite.rs:16-17 | FALSE | F-004: they parse |
| "literal strings ('...') / dates ... not supported" | toml_lite.rs:15-16 | REAL | parse falls through to "unsupported value"; tested L584-585 |
| "dotted keys ... not supported" | toml_lite.rs:16 | SHALLOW | bare dotted keys rejected (L238-243) but quoted header segments mis-split silently (F-006) |
| "a future file that does [use them] should fail the tool, not lie" | toml_lite.rs:17 | FALSE | F-001/F-002/F-003/F-005 are all silent |

## 7. Dead / duplicated / unreachable code
- No dead code found. `Document::table` (L112-117) — verify usage: compat_matrix/parity_report use `get`/`array_rows`; `table()` callers not confirmed in this audit's rg (UNVERIFIED whether any consumer calls Document::table — rg found no `\.table(` hit in the consumer grep output above; possible dead API).

## 8. Dependencies & flags
- Zero dependencies (std only). No feature flags. Consumers: cmd/feature_audit.rs, cmd/config_validator.rs, cmd/parity_report.rs, cmd/compat_matrix.rs, cmd/web_budget.rs (each via `use crate::cmd::toml_lite`).
- Parsed real files: docs/feature-matrix.toml, product.toml, docs/web/size-record.toml, locales reference catalog (via config_validator.rs:484-486).

## 9. Open questions for the next agent
- config_validator.rs:484-486: `std::fs::read_to_string(&reference).ok().and_then(|text| toml_lite::parse(&text).ok())` — a locale catalog that fails to PARSE is silently treated as absent, defeating this module's loud-error design at its biggest consumer. Same pattern at compat_matrix.rs:736 (`.ok()?`). Owners of those files should take this.
- No fuzz corpus exists for this parser despite fuzz/ existing in the repo root — a natural home for the malformed-input cases F-001/F-002/F-005.
- Whether `Document::table()` has any caller at all (possible dead API — UNVERIFIED, rg of consumers showed only get/array_rows use).
