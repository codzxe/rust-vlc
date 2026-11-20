# AUDIT: tools/xtask/src/cmd/toml_lite.rs

| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-16 |
| Lines / functions | 611 / 18 (9 free fns + 9 methods; 6 unit tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (`cargo xtask feature-audit` / `parity-report` / `config-validator` / `sbom` → those modules call `toml_lite::parse` on docs/feature-matrix.toml / product.toml / Cargo.lock — executed in the baseline runs of feature-audit, parity-report, config-validator, sbom) |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:1 |
| Commands run | Read full file; construct-grep of every consumer data file: `grep -n '"""' docs/feature-matrix.toml` (0), `grep -n "= '" docs/feature-matrix.toml` (0), `grep -n '^"' docs/feature-matrix.toml` (0 — no quoted keys), `grep -nE '^[a-z_]+\.[a-z_-]+ =' docs/feature-matrix.toml` (0 — no dotted keys), `grep -nE '= \{[^}]*$'` (0 — no multiline inline tables), `grep -c '\\\\' Cargo.lock product.toml` (0 escapes); `od -c` around the `[meta]` header (well-formed). All read-only. |

## 1. One-paragraph verdict (blunt: what it claims vs what it does)

toml_lite claims to be an honest std-only TOML-subset reader that "never silently skips" — unsupported constructs fail loudly with line numbers, and it covers exactly the shapes the audited files use. For the four files that actually flow through it (docs/feature-matrix.toml, Cargo.lock, product.toml — and the locale catalogs referenced in comments), this is TRUE: I grepped each construct class the module declares unsupported and found ZERO instances in the real data, and the inline-table/array/multiline-array shapes the files DO use are genuinely implemented with string-aware, comment-aware, bracket-balanced logic. The parser really parses — the baseline runs (feature-audit: 252 rows, implemented=193 cells; parity-report: 66+61+66+0=193 matching the same parsed doc; sbom: 158 components) are consistent with a correct parse, and two independent tools produced cross-consistent counts from the same parsed Document. Two real defects: duplicate `[table]` headers are accepted silently (first wins on lookup), contradicting the "never a silent skip" posture, and one of the four string-aware helpers omits escape handling the other three have.

## 2. What actually works (verified) — with the command you ran and its result

- **Subset-vs-reality match**: the module's doc (lines 14-17) lists what it rejects and asserts "None of the audited files use them". Verified construct-by-construct with grep against the real files: no `"""`, no `'...'` values, no quoted keys, no dotted keys, no multiline inline tables, no dates, no `\u` escapes (only `\\` and `\"`-adjacent escapes, both supported by `parse_string`, lines 452-459). Every real construct present (`[meta]` headers, `[[feature]]` × 252, single-line inline tables `status = { ... }`, single-line and multi-line arrays in Cargo.lock, basic strings with `\\` escapes at feature-matrix.toml:915 and :3144) is implemented by the parser.
- **Cross-tool consistency from the same parse**: baseline `xtask-feature-audit.txt` reports 252 rows and implemented=193 platform cells; baseline `xtask-parity-report.txt` machine line reports windows=66/252 linux=61/252 macos=66/252 web=0/252 (66+61+66+0=193). Two different tools, one shared `toml_lite::Document` — the status inline tables (the construct the hostile question worried about) parse correctly or both counts would diverge.
- **The committed SBOM built from this parse** matches the lockfile exactly: python3 cross-check of Cargo.lock vs docs/security/sbom/cyclonedx.json → lockfile packages: 158, sbom components: 158, sets equal: True, subject dependsOn: 39 = path-source rows 39. Cargo.lock's multi-line `dependencies = [\n "x",\n]` arrays (the construct most likely to trip a naive reader) are handled by `join_until_closed` + `parse_value` (lines 247-256, 333-359).
- Loud-failure design verified in code: unknown value → `ParseError` with line (430-433); unsupported key shape → error (238-243); unterminated multiline → error (355-358); the module's own tests (583-588) assert the rejects.

## 3. Findings

### F-T01 [S3] — Duplicate `[table]` headers accepted silently; first wins on `get`
Where: tools/xtask/src/cmd/toml_lite.rs:192-215 (header handling), 126-146 (`Document::get`)
Code (192-215, excerpt):
```rust
        if trimmed.starts_with('[') {
            let Some(close) = trimmed.rfind(']') else { ... };
            let path: Vec<String> = trimmed[1..close]
                .split('.')
                .map(|s| s.trim().to_string())
                .collect();
            ...
            doc.tables.push(Table {
                path: path.clone(),
                entries: Vec::new(),
            });
```
What is wrong: nothing checks whether a table with the same path was already pushed. TOML forbids duplicate `[table]` headers; this parser stores both, and `Document::get`/`table` (112-117, 126-146) return the FIRST match — entries from the second header are silently unreachable through the lookup API (only visible via `doc.tables` iteration). Contrast: check_architecture.rs's separate reader rejects duplicates loudly (`parse_toml`, line 332-334: "duplicate table header").
Why it matters: a duplicated `[meta]` or a copy-pasted `[identity]` in product.toml/feature-matrix.toml would silently shadow data — exactly the "silent skip" the module header (line 15) promises never happens. Consumers keyed by name (sbom's `identity.*` reads, config-validator cross-refs) would read stale/first values without any diagnostic.
What correct looks like: reject a repeated non-array header path with `ParseError { line, "duplicate table header" }`, mirroring check_architecture.rs.
Confidence: HIGH (code path is unambiguous)
Verification: read (no run — cargo test serialized by orchestrator; behavior is directly visible in the code).

### F-T02 [S3] — `find_top_level_eq` is the only string-aware helper without escape handling
Where: tools/xtask/src/cmd/toml_lite.rs:299-309
Code:
```rust
fn find_top_level_eq(line: &str) -> Option<usize> {
    let mut in_string = false;
    for (idx, c) in line.char_indices() {
        match c {
            '"' => in_string = !in_string,
            '=' if !in_string => return Some(idx),
            _ => {}
        }
    }
    None
}
```
What is wrong: `strip_comment` (282-296), `brackets_closed` (313-329) and `split_top_level` (472-495) all skip the char after a `\` inside strings; `find_top_level_eq` does not. An escaped `\"` inside a string flips `in_string` prematurely, so a subsequent `=` inside a later string on the same line can be picked as the separator. The sibling helpers got the fix; this one did not.
Why it matters: for a line like `key = "a\" = b"` the separator is still found correctly because the FIRST `=` precedes any string content — the defect only fires when an escaped quote appears before the real separator, i.e. inside a quoted key or in exotic inline-table keys (`parse_value`'s inline-table branch at 405-412 reuses this helper). No real consumer file uses quoted keys (verified by grep), so today it is latent. Latent divergence between four helpers that must agree on string semantics is how future misparses are born.
What correct looks like: the same `'\\' if in_string => { chars.next(); }` guard used by the other three.
Confidence: HIGH (inconsistency), LOW (real-world trigger — none exists today)
Verification: read.

### F-T03 [S4] — `brackets_closed` treats negative depth as closed
Where: tools/xtask/src/cmd/toml_lite.rs:313-329, return at 328
Code (327-328):
```rust
    }
    depth <= 0
}
```
What is wrong: a value fragment whose close brackets outnumber opens (e.g. a corrupted `= ] ... [`) is classified "closed", so no continuation is joined and `parse_value` gets the raw fragment. The follow-on failure IS loud (`bad array literal` / `bad string literal`), so no silent skip results — but the depth bookkeeping is sloppy and the classification is only accidentally safe.
Why it matters: robustness debt on a parser that four production tools trust; any future caller that uses `brackets_closed` to make a skip/continue decision gets wrong answers for over-closed text.
What correct looks like: track depth as "open count" and require exactly zero, or explicitly document why over-close is acceptable.
Confidence: MEDIUM
Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Display::fmt` (ParseError) | 31-35 | REAL | — | "line N: msg". |
| `Value::as_str` / `as_int` / `as_table` / `type_name` | 51-82 | REAL | — | Type guards; used by consumers. |
| `Document::table` | 112-117 | REAL | S3 | First-match only — part of F-T01 exposure. |
| `Document::array_rows` | 120-122 | REAL | — | Source order preserved (tested at 601-610). |
| `Document::get` | 126-146 | REAL | S3 | First matching table wins — F-T01. |
| `Document::has_root_key` | 150-152 | REAL | — | Quoted-key root entries. |
| `parse` | 156-262 | REAL | S3 | Core loop; missing duplicate-header rejection (F-T01); multiline continuation OK. |
| `attach` | 266-279 | REAL | — | Correct TOML scoping (current table → last array row → root). |
| `strip_comment` | 282-296 | REAL | — | String-aware, escape-aware. |
| `find_top_level_eq` | 299-309 | REAL | S3 | Missing escape guard (F-T02). |
| `brackets_closed` | 313-329 | REAL | S4 | `depth <= 0` (F-T03). |
| `join_until_closed` | 333-359 | REAL | — | Joins continuation lines, per-line comment strip; loud unterminated error. |
| `parse_value` | 362-434 | REAL | — | Strings/arrays/inline tables/bool/int/float; loud unsupported. |
| `parse_string` | 437-469 | REAL | — | Full escape set incl. `\\`, `\"`; loud on `\u`-style unknowns. |
| `split_top_level` | 472-495 | REAL | — | Bracket-depth-aware, escape-aware. |
| tests (6) | 497-611 | HONEST | — | Assert real parsed values (matrix shape, multiline arrays, inline tables, ordering) and rejects. Would fail if parsing regressed. |

## 5. Test honesty

Yes — the six tests assert concrete parsed values (`meta.schema_version == Int(1)`, status table of 4 entries with values, evidence string containing `#`, multiline array of 3 items, array-row ordering) and concrete rejections (`'''raw'''`, dates, bare garbage). If the parser misparsed or silently dropped any tested construct, the assertions fail. Not covered by tests: duplicate table headers (F-T01) and escaped-quote key separation (F-T02) — the two real gaps.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Deliberately NOT supported (parse error, loudly … never a silent skip): multi-line basic strings, literal strings, dotted keys, dates, mixed-type arrays. None of the audited files use them" | toml_lite.rs:14-17 | TRUE for all audited files | Construct greps (see Commands run) return zero instances in docs/feature-matrix.toml, Cargo.lock, product.toml. |
| "shared … reader for the issue-#20 tools (feature-audit / config-validator / parity-report)" | toml_lite.rs:1-2 | TRUE, and understated — sbom.rs also uses it | `use super::toml_lite` in sbom.rs:73; registry comments mod.rs:26-29. |
| The parity-report "TOTAL 0/252 vs summary 66/252" contradiction handed to this audit as a toml_lite suspect | orchestrator context | NOT a toml_lite defect | The two tables use two definitions: implemented+verified (66/61/66/0) vs verified-only (all 0 — the matrix has zero `verified` cells; feature-audit baseline cell counts blocked=129 implemented=193 not_started=503 partial=147 unsupported=36, no verified). Same parsed Document yields both; parser is consistent across consumers. |

## 7. Dead / duplicated / unreachable code

- `Value::Float` variant: produced by `parse_value` (427-429) but no consumer matches on it; harmless schema completeness.
- `Document::table()` (112-117): used only by consumers I did not audit (config_validator possibly); not dead within the crate's four consumers — UNVERIFIED per-consumer call sites outside my six files.
- No unreachable branches found.

## 8. Dependencies & flags

- std only (`fmt`). No flags — pure library function `parse(&str) -> Result<Document, ParseError>`. No I/O (callers read files); 100% testable in isolation, and it is.

## 9. Open questions for the next agent

- Does `config_validator.rs` or `compat_matrix.rs` rely on `Document::get` first-match behavior in a way where a duplicate header would change a verdict? (F-T01 becomes exploitable the moment a writer tool trusts `get` over a merge.)
- `strip_comment` toggles only on `"` — a `#` inside a (rejected-anyway) literal string truncates the line BEFORE the value parser can reject it, turning "loud unsupported" into "loud unsupported with a confusing message". Worth one test to pin the failure text.
- Cargo.lock v4 uses `version = 4` root entry — parse handles it (Int path). If the lockfile ever moves to v5 with new keys, sbom.rs:337-343 fails loudly by design; toml_lite needs no change, but the sbom reader does.
