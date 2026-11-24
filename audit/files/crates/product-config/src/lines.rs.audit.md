# AUDIT: crates/product-config/src/lines.rs
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| Lines / functions | 240 / 7 (+6 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO for the release lane (product-config is not in lumen-cli/desktop graphs) — but YES from the workspace app apps/control-server: main.rs:154 `load_file` → load.rs:77 `LineIndex::new(src)` runs on every product.toml load (error-path diagnostics). Also used by schema.rs:124, semantic.rs:97, merge.rs:170/210 (the latter two with empty indexes). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | rg LineIndex users; rg '"""' product.toml + fixtures (none — multiline gap is latent); rg '^\[' product.toml (header inventory: no array-of-tables, no quoted segments); sed quote verification |

## 1. One-paragraph verdict
A small, correct-for-its-actual-inputs line indexer with honest ancestor
fallback and exact-line tests. It indexes table headers, array-of-table
elements (`path.0.field`), dotted assignments, and strips `#` comments with
basic quote awareness; the deepest-ancestor fallback (line_of 64-76) is
exactly what the diagnostic layer needs. The scanner is not a TOML parser
and does not claim to be, but its doc header also does not disclose the two
constructs it gets wrong — multi-line strings/arrays (phantom index entries)
and array-element sub-tables (`[fruit.physical]` after `[[fruit]]` loses the
element index). Both are latent today: product.toml and every fixture avoid
them (verified by scan). The escaped-quote parity bug in strip_comment is
benign because only the text left of the first `=` is ever consumed.

## 2. What actually works (verified)
— Tests assert exact lines incl. array elements (`migrations.entries.0.from`
→ 14, `.1.to` → 19) and ancestor fallback (`identity.description_key` → the
`[identity]` header line 3; wholly-missing table → None); all passed in
baseline (product-config exit 0, cargo-test-workspace.txt:5389 region).
— Verified by scan: `rg '"""' product.toml` and over all
crates/product-config/tests/fixtures/*.toml → no multi-line strings; the
committed product.toml header inventory (`rg '^\['`) shows plain `[a.b]`
tables only — every construct the scanner mishandles is absent from real
inputs.

## 3. Findings
F-001 [S4]
Where: crates/product-config/src/lines.rs:23-57
Code:
```
        for (idx, raw) in src.lines().enumerate() {
            let lineno = (idx + 1) as u32;
            let stripped = strip_comment(raw);
            let trimmed = stripped.trim();
```
What is wrong: no multi-line construct tracking. A line inside a `"""`
string or a spanning array that happens to look like `k = v` is indexed as a
phantom assignment under the current table prefix; because
`lines.entry(dotted).or_insert(lineno)` keeps the FIRST occurrence, a phantom
can shadow a real later key and put a diagnostic on the wrong line. Latent:
no `"""` exists in product.toml or any product-config fixture (verified by
scan). The doc header (1-10) presents the scanner generically without this
caveat.
Confidence: HIGH — Verification: read + scans.
F-002 [S4]
Where: crates/product-config/src/lines.rs:34-49
Code:
```
                if is_array {
                    let count = array_counts.entry(path.clone()).or_insert(0);
                    let index = *count;
                    *count += 1;
                    let dotted = join_path(&path);
                    lines.entry(dotted).or_insert(lineno);
                    path.push(index.to_string());
                }
```
What is wrong: TOML array-element sub-tables (`[[fruit]]` followed by
`[fruit.physical]`, which per TOML 1.0 attaches to the last array element)
are indexed as `fruit.physical` — the element index is dropped, so a lookup
for `fruit.0.physical.color` falls back to the `fruit` array header line
instead of the `[fruit.physical]` line. Degraded accuracy, never a wrong
non-ancestor line. Latent: product.toml has no array-of-tables at all (the
committed `[migrations]` section's entries are commented out, product.toml:244).
Confidence: HIGH — Verification: read; header inventory by scan.
F-003 [S4]
Where: crates/product-config/src/lines.rs:135-148
Code:
```
        match c {
            '"' if !in_literal => in_basic = !in_basic,
            '\'' if !in_basic => in_literal = !in_literal,
            '#' if !in_basic && !in_literal => break,
```
What is wrong: `strip_comment` is escape-unaware — `\"` inside a basic
string flips parity, so a `#` inside the string can be treated as a comment
start (or a real comment left unstripped). Benign today: only the text left
of the FIRST `=` is consumed (assignment_key), and a header can never follow
an assignment on the same line, so mis-stripping cannot corrupt key
extraction. No test covers escaped quotes.
Confidence: HIGH — Verification: read; traced `title = "a\"b" # [ref]`.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| LineIndex::new | 23-59 | REAL | S4 | phantom-entry + sub-table gaps (F-001/F-002) |
| LineIndex::line_of | 64-76 | REAL | — | longest-prefix fallback, correct |
| join_path | 79-81 | REAL | — | |
| table_header | 89-109 | REAL | — | quoted dotted segments (`["a.b"]`) would split wrongly — latent, absent from inputs |
| assignment_key | 112-131 | REAL | — | first-'=' split; rejects '[' keys |
| strip_comment | 135-149 | REAL | S4 | F-003 escape parity |
| unquote | 152-168 | REAL | — | byte-slice safe (quote chars are 1-byte) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| finds_top_level_keys | YES | exact line assert |
| finds_table_keys | YES | nested + doubly-nested exact lines |
| hash_inside_string_is_not_a_comment | YES | the #4f8cff case from the real file |
| array_of_tables_elements_are_indexed | YES | both elements' exact lines |
| missing_key_falls_back_to_nearest_ancestor | YES | ancestor + None cases |
| table_header_line_is_the_fallback_for_missing_members | YES | header-line fallback |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "records, for every full dotted path, the line it starts on" | lines.rs:7-9 | SHALLOW | true for every construct present in real inputs; false for multi-line values and array-element sub-tables (F-001/F-002) |
| "a missing table points at line 1" | lines.rs:10 | FALSE (doc nit) | line_of returns None for a wholly-missing table (test line 231 asserts None); nothing "points at line 1" |

## 7. Dead / duplicated / unreachable code
None. All functions used (join_path/unquote via new/table_header/assignment_key).

## 8. Dependencies & flags
std only. No features.

## 9. Open questions for the next agent
- If product.toml ever grows `[[migrations.entries]]` with sub-tables, F-002
  degrades diagnostic lines — worth a scanner note in the doc header at that
  point.
