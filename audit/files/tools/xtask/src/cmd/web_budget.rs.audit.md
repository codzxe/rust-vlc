# AUDIT: tools/xtask/src/cmd/web_budget.rs
| Field | Value |
|---|---|
| Auditor | W01-A07 |
| Date | 2026-11-23 |
| Lines / functions | 1344 / 18 non-test fns + 18 tests |
| File verdict | REAL-UNPROVEN (pure logic REAL via baseline tests; live rebuild+measure path never observed passing) |
| Reachable from shipped entry point? | NO for the release lane (lumen-cli/desktop do not depend on xtask). YES for dev/CI entry: main.rs:124-125 dispatch -> cmd/mod.rs:191 `run: web_budget::run` -> `web_budget::run`; wired at scripts/full_gate.sh:71 and .github/workflows/ci.yml:659 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 5 |
| Commands run | rg (dispatch/wiring/tests/schema) exit 0; sed/od/tail quote verification exit 0; baseline citations below. No builds run (spec §2) |

## 1. One-paragraph verdict
This is a real, carefully-written gate: budgets are read from product.toml (no literal in the file — verified, the only MiB constants live in tests and product.toml:222-223), the >5% growth rule is integer-exact (`measured*20 > recorded*21`, L748-749, boundary proven by tests at exactly +5%), gzip is a real `gzip -n -c` subprocess (L670-690), sha256 is a real FIPS 180-4 implementation pinned by standard vectors incl. the million-'a' multi-block case, and record parsing is strict (unknown keys, non-hex sha, rebuild-without-package all rejected). The baseline question — does it distinguish "build failed" from "budget exceeded"? — the TOOL does: build failure returns `Err(XtaskError::internal(...))` (L610-620) which main.rs:104-107 maps to exit 2, while budget/growth verdicts return `Ok(ExitCode::from(1))` (L244, L270). Its only wired consumer, full_gate.sh, does NOT: `if cargo xtask web-budget --check` treats exit 1 and exit 2 identically ("web-budget: FAILED") — the baseline full_gate.txt:76-84 shows a missing-wasm-toolchain E0463 reported exactly like a budget breach. The end-to-end rebuild path has never been observed passing in the baseline (full_gate step 10 died on the environment; rc-smoke's addendum built the DEV profile only, rc-smoke.txt:213-230).

## 2. What actually works (verified)
— All 18 unit tests passed: audit/runs/baseline/cargo-test-workspace.txt:6725-6742 (`cmd::web_budget::tests::* ... ok`, incl. 4 FIPS sha256 vectors and the synthetic-breach/growth-boundary tests).
— Parse path proven on the REAL committed inputs: full_gate.txt:76-84 fails with E0463 from `cargo build`, which web_budget.rs only invokes at L187 AFTER `Budgets::from_product_toml` (L163) and `parse_record` (L174) succeeded — so real product.toml and real docs/web/size-record.toml both parsed clean.
— Budget values cross-checked: product.toml:222-223 (`phase2_vertical_max_mib = 3`, `full_p0_max_mib = 6`) match the record rows' ceilings; committed floor gzip 300250 < 3 MiB, external-all 704996 < 6 MiB (docs/web/size-record.toml).
— verified by rg/sed/od: registration (cmd/mod.rs:191), full_gate wiring (full_gate.sh:71-74), CI wiring (ci.yml:659), record ends LF with trailing newline (od).

## 3. Findings
F-001 [S3]
Where: tools/xtask/src/cmd/web_budget.rs:47-49 (claim), scripts/full_gate.sh:71-74 (consumer)
Code:
```
//! (the CI canary's convention). Build failures are tool errors (exit
//! 2); budget/growth verdicts are gate failures (exit 1) — the stable
//! xtask exit-code contract.
```
```sh
if cargo xtask web-budget --check > /tmp/webbudget.log 2>&1; then
  tail -1 /tmp/webbudget.log; echo "web-budget: PASS"
else
  echo "web-budget: FAILED"; tail -8 /tmp/webbudget.log; fail=1
```
What is wrong: the tool honors the exit-1/exit-2 split (verified: rebuild failure -> Err -> main.rs `ExitCode::from(2)`; verdict -> `Ok(ExitCode::from(1))`), but the only gate consumer collapses both into "web-budget: FAILED". Baseline proof: full_gate.txt:77 prints "web-budget: FAILED" for E0463 (missing wasm32 rust-std — a tool/environment error, exit 2), byte-identical to what a budget breach prints. Why it matters: the gate summary cannot tell "the tree is broken" from "the tool is broken" — exactly the distinction main.rs:14-22 says CI and scripts depend on; an operator triaging a red gate must read the raw cargo stderr to learn it's an env problem, not a size regression. What correct looks like: full_gate step 10 branches on `$?` (1 = budget failure, 2 = tool error) and prints distinct verdicts. Confidence: HIGH — Verification: read + baseline log (full_gate.txt:76-84, rc-smoke.txt:134-146).

F-002 [S4]
Where: tools/xtask/src/cmd/web_budget.rs:776-781 (claim) vs 791/834-836 (implementation)
Code:
```
/// Rewrite the `raw_bytes` / `gzip_bytes` / `sha256` lines of the named
/// `[[config]] blocks in place, preserving every other byte of the file
```
vs `for line in text.lines()` + unconditional `out.push('\n')` (L791, L831/L835).
What is wrong: `str::lines()` drops a missing trailing newline and strips `\r`, so a record without a final newline gains one, and a CRLF record is silently converted to LF throughout — "preserving every other byte" is false for those inputs. Why it matters: the maintain mode's core promise is byte-stable rewriting; it holds for the committed record (verified LF + trailing newline via od) but not universally. What correct looks like: track whether the final line ended in `\n` and preserve line terminators, or document the normalization. Confidence: HIGH — Verification: read (mutation reasoning; committed record unaffected).

F-003 [S4]
Where: tools/xtask/src/cmd/web_budget.rs:867-872
Code:
```
fn parse_name_value(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("name")?.trim_start();
    let value = rest.strip_prefix('=')?.trim();
    let inner = value.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner)
}
```
What is wrong: returns the RAW inner text; toml_lite's parse_string (toml_lite.rs:437-459) unescapes `\"` etc. A row named `"a\"b"` parses as `a"b` (parse_record) but the rewriter hunts for the literal `a\"b`, misses the block, and dies with the misleading "row ... was measured but has no [[config]] block" (L842-845). Why it matters: parser and rewriter disagree on name decoding; failure is loud but the message names the wrong defect. Escaped record names are absurd in practice — limited blast radius. What correct looks like: run the same unescape as toml_lite (or delegate). Confidence: HIGH — Verification: read both functions.

F-004 [S4]
Where: tools/xtask/src/cmd/web_budget.rs:950
Code:
```
    String::from_utf8(out).unwrap_or_default()
```
What is wrong: on the impossible UTF-8 failure this silently returns an EMPTY sha256, which would then be compared and WRITTEN into the record by maintain mode instead of failing loudly. `out` is hex-ASCII by construction so the branch is dead. Why it matters: a `unwrap_or_default()` on an invariant is a swallowed-error pattern the repo's own lint policy (main.rs:26-28 denies unwrap/expect) dodged by hiding it; if the hex table ever regressed, maintain mode would write `sha256 = ""` (parse rejects 64-hex later, so the corruption is caught on the NEXT run — still, it writes a broken record first). What correct looks like: construct the string from the HEX bytes directly (infallible) e.g. `String::from_utf8` replaced by collecting chars. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: tools/xtask/src/cmd/web_budget.rs:665-666
Code:
```
    let file = format!("{}.wasm", package.replace('-', "_"));
    Ok(target_dir.join(WASM_TARGET).join(PROFILE_DIR).join(file))
```
What is wrong: the artifact filename is assumed to be `<package with - -> _>.wasm` — true only when the crate does not set a custom `[lib] name`; a record row naming such a package (or a bin-only package) reads a missing file (loud) or, worse, a same-named STALE .wasm left by an earlier row/build (silent wrong measurement). No freshness check ties the read artifact to the build that just ran. Why it matters: the gate's honesty rests on measuring what it just built; the current record only contains `lumen-web` (default lib name `lumen_web`, apps/web/Cargo.toml:49-50 sets crate-type but no name), so today it is correct. What correct looks like: parse `[lib] name` from the package manifest or verify mtime > build start. Confidence: HIGH — Verification: read + apps/web/Cargo.toml.

F-006 [S4]
Where: tools/xtask/src/cmd/web_budget.rs:434-517 (parse loop) with toml_lite `attach` (toml_lite.rs:266-279)
What is wrong: duplicate keys inside one `[[config]]` row are not rejected (toml_lite pushes both; the match loop lets the LAST occurrence win for `name`/`gzip_bytes`/etc.), while `rewrite_measured_rows` rewrites the FIRST matching line — a hand-mangled record with duplicated `gzip_bytes` would be checked against one copy and rewritten on the other. Strict TOML rejects duplicate keys outright. Why it matters: parse/rewrite asymmetry on malformed input; requires a corrupted/hand-edited record. What correct looks like: parse_record errors on a repeated key. Confidence: MEDIUM — Verification: read (toml_lite duplicate handling fully confirmed in the toml_lite audit, cross-ref F-006 there).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run | 118-273 | REAL-UNPROVEN | — | arg parse + orchestration; e2e never observed passing (baseline env). Exit 1/2 split correct |
| find_root | 279-296 | REAL | — | walk-up to product.toml; same pattern as config_validator |
| Budgets::from_product_toml | 315-340 | REAL | — | loud on missing section (no default); proven on real product.toml via baseline E0463 ordering |
| Budgets::validate | 345-355 | REAL | — | superset rule full_p0 >= phase2; tested |
| Budgets::ceiling_bytes | 359-372 | REAL | — | saturating_mul harmless (schema caps 1..=64/1024 elsewhere); unknown class errors loudly |
| Budgets::describe | 375-383 | REAL | — | display only |
| parse_record | 412-565 | REAL | — | strict; duplicate keys NOT rejected (F-006); ran clean on committed record (baseline) |
| rebuild_and_measure | 582-647 | REAL-UNPROVEN | — | real cargo build; build-fail -> exit 2 (correct); never observed completing in baseline |
| wasm_artifact_path | 653-667 | REAL | S4 | F-005 filename assumption; honors CARGO_TARGET_DIR |
| gzip_size | 670-690 | REAL | — | real `gzip -n -c` subprocess, never re-implemented (claim TRUE) |
| evaluate | 716-770 | REAL | — | pure; integer-exact 5% (L748-749); breach on recorded rows for non-rebuild rows per contract; tested incl. boundary |
| rewrite_measured_rows | 781-849 | REAL | S4 | F-002 newline/CRLF normalization; tested for committed shape |
| finish_block | 853-864 | REAL | — | loud on partial rewrite (inline comments rejected — tested) |
| parse_name_value | 867-872 | REAL | S4 | F-003 no unescaping |
| replace_int_value | 876-885 | REAL | — | indent-preserving; rejects lookalikes (tested) |
| replace_str_value | 889-899 | REAL | — | rejects embedded quotes (loud) |
| sha256_hex | 919-951 | REAL | — | FIPS 180-4 correct; 4 canonical vectors + 100-byte two-block-tail + million-'a' pass (baseline) |
| compress | 954-1004 | REAL | — | correct round constants/message schedule; defensive `unwrap_or(&[])` unreachable (blocks always 64B) |
| 18 tests | 1010-1343 | REAL | — | honest, would fail on mutation (see §5) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| sha256_empty_string / _abc / _448_bits / _multi_block_and_two_block_tail | YES | canonical FIPS vectors, incl. 1M-'a' multi-block and 128B-tail path; a wrong compress/padding fails them |
| budgets_parse_from_product_toml_shape | YES | asserts exact parsed struct from TOML text |
| budgets_missing_section_is_a_loud_error | YES | asserts Err on absent [web.size_budgets] |
| budgets_full_p0_below_phase2_is_rejected | YES | asserts Err on inverted ceilings |
| ceiling_bytes_maps_classes_and_rejects_unknowns | YES | asserts exact byte ceilings + Err on unknown class |
| evaluate_passes_the_committed_shape | YES | mirrors committed record; asserts source tags measured/recorded |
| evaluate_fails_a_synthetic_budget_breach | YES | 4 MiB vs 3 MiB must fail naming row+ceiling |
| evaluate_fails_synthetic_growth_just_over_five_percent | YES | 315001 vs 300000 must fail (first integer >5%) |
| evaluate_passes_growth_exactly_five_percent_and_notes_sha_drift | YES | 315000 = exactly 5% must pass + sha note emitted |
| parse_record_reads_the_committed_shape | YES | synthetic text, asserts 2 rows + package/features |
| parse_record_rejects_wrong_schema_and_rebuild_without_package | YES | asserts Err both cases |
| rewrite_touches_only_the_named_block | YES | asserts untouched second block verbatim |
| rewrite_is_byte_stable_for_identical_measurements | YES | asserts full-string equality |
| rewrite_fails_when_a_measured_row_has_no_block | YES | asserts Err for ghost row |
| int_line_replacement_preserves_indent_and_rejects_lookalikes | YES | asserts indent kept, `raw_bytes_extra`/inline-comment rejected |
Weak spot: tests never run against the REAL docs/web/size-record.toml (inline fixtures only); real-record coverage exists only via the gate run, which the baseline never completed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ceilings are READ FROM CONFIG (product.toml [web.size_budgets])... no budget literal exists in this file" | web_budget.rs:10-16 | REAL | L315-340 reads via toml_lite; only budget numbers in file are in `mod tests` (L1014-1019) |
| ">5% growth ... integer-exact comparison measured*20 > recorded*21" | web_budget.rs:17-20, 108-111 | REAL | L748-749; boundary tests L1157-1193 |
| "gzip is measured by running ... `gzip -n -c` as a subprocess — the gate never re-implements the compressor" | web_budget.rs:42-45 | REAL | L670-690 |
| "sha256 IS implemented here (std-only) against the FIPS 180-4 vectors" | web_budget.rs:44-46 | REAL | L919-1004 + 4 passing vector tests (baseline 6739-6742) |
| "Build failures are tool errors (exit 2); budget/growth verdicts are gate failures (exit 1)" | web_budget.rs:47-49 | REAL (tool) / consumer conflation | L610-620 -> main.rs:104-107 exit 2; L244/L270 exit 1; full_gate.sh:71-74 ignores the split (F-001) |
| "Wired into scripts/full_gate.sh (step 10) and the CI wasm-build job" | web_budget.rs:37-38 | REAL | full_gate.sh:70-74; ci.yml:659 |
| "rewrites this file byte-stably" / "preserving every other byte" | web_budget.rs:30, 776-778 | SHALLOW | true for committed LF record + tested shape; false for missing-trailing-newline/CRLF input (F-002) |
| "validated by product.schema.json" (budgets) | web_budget.rs:12-14 | SHALLOW | THIS tool checks only >0 + superset rule; range enforcement (1..=64 / 1..=1024) lives in config-validator's schema walk (product.schema.json:217, 223; config_validator.rs:11) |

## 7. Dead / duplicated / unreachable code
- L208-210 `if v.ceiling_bytes == 0` "n/a" pct branch: unreachable — budgets are validated `> 0` (L319), so ceiling is never 0. Dead defensive display branch.
- Not dead: everything else is called. Not reachable from the SHIPPED release lane (xtask is dev tooling); reachable from `cargo xtask` / full_gate / CI as proven in the header table.

## 8. Dependencies & flags
- Depends only on std + `super::toml_lite` + `XtaskError` (declared no-external-deps posture holds; Cargo.toml of xtask not re-verified here — cross-ref toml_lite audit).
- External processes: `cargo build -p <pkg> --target wasm32-unknown-unknown --release --locked` (+ optional `--features`), `gzip -n -c`. Environment assumptions: gzip on PATH, wasm target installed (the exact assumption the baseline full_gate violated).
- Reads CARGO_TARGET_DIR (never printed/logged — spec-compliant).

## 9. Open questions for the next agent
- The committed record's release numbers (floor raw 1294311 / gzip 300250, sha 4a0f33...) cite toolchain "rustc 1.99.1" and have NO baseline re-verification — full_gate never got past E0463 and rc-smoke only built the DEV profile. Was `web-budget --check` ever green anywhere? UNVERIFIED (needs a wasm-target-equipped run).
- full_gate.sh (not this agent's file) should branch on exit code 2 vs 1 (F-001) — owner of scripts/full_gate.sh should take it.
- ci.yml:659 fails the Actions step on any non-zero exit — same conflation, lower stakes (CI red is CI red).
