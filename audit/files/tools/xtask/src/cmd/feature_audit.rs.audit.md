# AUDIT: tools/xtask/src/cmd/feature_audit.rs
| Field | Value |
|---|---|
| Auditor | W01-A06 |
| Date | 2026-11-23 |
| Lines / functions | 601 / 10 non-test (+4 tests, +1 test trait) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (dev tool; not in the release lane). Dev entry `cargo xtask feature-audit` -> tools/xtask/src/cmd/mod.rs dispatch. NOT wired into any CI workflow or script — CI's merge-blocking check is the weaker inline python3 "v0 structural check" (ci.yml:284-405); this Rust tool is advisory-only (orchestrator ran it manually for the baseline). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `./target/debug/xtask feature-audit --root .` -> exact baseline reproduction (rows: 252, 2 violations, result FAILURE, exit 1); synthetic 5-row matrix in audit/runs/tmp/fa-root -> 3 violations (empty-file + prose evidence rows produced NONE), exit 1; corrupt-header matrix -> `parse: line 1: expected 'key = value' or a header`, exit 1; `cargo test -p xtask cmd::feature_audit` -> 4 passed; `awk` row-23/row-163 extraction from docs/feature-matrix.toml; `ls` path checks |

## 1. One-paragraph verdict
The tool works and I executed it: it reproduces the orchestrator baseline byte-for-byte (252 rows, 2 R4 violations, exit 1). Answers to the master prompt's three questions: (1) **R4 as implemented = two checks only**: B — any platform status `implemented`/`verified` requires a non-empty `evidence` STRING (feature_audit.rs:230-239), and C — every path-like token in that string must EXIST on disk (`root.join(path).exists()`, feature_audit.rs:495-501). That is all. No content, no relation-to-claim, no execution, no mock-labeling. (2) **Yes, an empty file satisfies it — proven by execution**: my synthetic row `fake-one-empty-file` (implemented ×4 platforms) citing a 0-byte `crates/empty/empty.rs` produced ZERO violations; the tool's own unit test does the same (writes `""` and asserts it resolves, line 572). Worse, evidence containing no path-like token at all — pure prose — also passes: row `fake-two-prose-only` with `verified` ×4 and evidence "fully implemented, trust me, the tests are elsewhere" produced ZERO violations. (3) **Baseline reconciliation**: the FAILURE with 2 missing paths is the tool working correctly, not a defect — row 23 (`codec-video-h264-software`, implemented ×3) cites `crates/codecs/src/video/h264/sw/**`, and the repo has `sw.rs` (a file) not `sw/` (a dir), so the glob-prefix check correctly fails; row 163 (`config-advanced-folding`, implemented ×3) cites `ui/src/screens/settings/areas/advanced.tsx` and no `areas/` dir exists (the settings files live flat). Both rows' other cited paths (crates/codecs/tests/h264_sw_differential.rs, docs/dependencies/records/rust_h264-0.6.1.md) do exist — the 2 violations are genuine stale citations, zero false positives. The structural checks (A: R9/R1-regex/R2-range/R3/R5/R6-emptiness, D: counts) are real and executed. The finding against this file: the spec's R4 ("artifact recording commands + actual results ... mock evidence must be labeled mock", doc 02:555, matrix:84-88) is reduced to bare existence, three specified rules (R1 prefix-map, R2 P0⇒≤6, R5 blocked_reason must reference an EXISTING doc/ADR/risk) are silently missing from both the implementation and the "NOT done" disclosure, and the tool is not wired into CI — so the only merge-blocking matrix check remains the weaker python one.

## 2. What actually works (verified)
— verified by `./target/debug/xtask feature-audit --root .` -> identical to audit/runs/baseline/xtask-feature-audit.txt (rows: 252; per-phase 2=10 3=5 4=93 5=49 6=18 7=25 8=52; 2 violations rows 23 & 163; exit 1).
— verified by synthetic-matrix execution (audit/runs/tmp/fa-root): missing-path detection, R5 (blocked without blocked_reason), R6 (partial without notes) all fire; empty-file evidence and prose evidence pass; exit codes 0/1 behave as documented.
— verified by `cargo test -p xtask cmd::feature_audit` -> 4 passed, 0 failed.

## 3. Findings
F-001 [S3]
Where: tools/xtask/src/cmd/feature_audit.rs:13-14, 495-501
Code:
```
//!   C | evidence-resolution | every repo-path token inside `evidence` resolves to an existing file/dir under the workspace root (R4 as written: "evidence resolving to an existing artifact"); `**` globs check the prefix; `inventory/…` refs map to `docs/inventory/…` |
```
```
    // Trailing `/` cites a directory.
    let path = path.trim_end_matches('/');
    if root.join(path).exists() {
        None
    } else {
        Some(path.to_string())
    }
```
What is wrong: the header calls existence-checking "R4 as written", but R4 as actually written (docs/02-phase0-audit-plan.md:555; docs/feature-matrix.toml:84-88) requires evidence to resolve to "an existing artifact **recording commands + actual results** (or a our-CI run URL); **mock evidence must be labeled mock**". The tool enforces only the existence clause — the quoted phrase is a partial quote that drops the other two. Executed proof: a 0-byte file satisfies R4-C, and evidence with no path token at all ("fully implemented, trust me...") satisfies B+C for a row claiming `verified` on all four platforms. Concrete in-tree demonstration: row 23's evidence paths exist (verified) and its non-default `codec-h264-sw` feature is real code, yet the release-lane build ships no h264 decoder (baseline engine-info: decoders mjpeg, pcm_s16le) — existence-only evidence cannot see feature-gating, content, or truth. The header's honest register partially covers this ("R10 anti-hallucination beyond path existence" not done), but "R4 as written" overstates what is delivered. Why it matters: this tool is the designated evidence auditor of the parity ledger the whole program is graded on (MP §1); its ceiling is "cited paths are not dangling". What correct looks like: rename the claim to "R4 existence clause only"; add a content floor (non-empty file, artifact-kind allowlist, mock-label detection).
Confidence: HIGH — Verification: ran (synthetic matrix + real matrix) + read.

F-002 [S3]
Where: tools/xtask/src/cmd/feature_audit.rs:17-22
Code:
```
//! NOT done here (honest register, same as the header's owed list):
//! R7 config cross-check (`config-validator` owns the product.toml side),
//! R8 inventory completeness (needs a formats.json↔row reconciliation spec),
//! R10 anti-hallucination beyond path existence, and `not_started → verified`
//! jump rejection in git history (needs a `--from` revision argument; v0
//! audits the working tree only).
```
What is wrong: "same as the header's owed list" is false — the matrix header's owed list (docs/feature-matrix.toml:10-16) also names "R1 category-prefix map, R2-P0 rule" and the rule list (matrix:96-105, doc 02 §4.6) additionally specifies R5 as "blocked_reason non-empty **and referencing an existing doc section/ADR/risk entry**". Three rule halves are implemented as nothing (R1 prefix-vs-category, R2 P0⇒phase≤6) or as emptiness-only (R5: `blocked_reason = "xx"` passes; the tool already owns `unresolved_paths` and could resolve `docs/...` refs in it) — and none of the three appears in the NOT-done disclosure. Why it matters: 129 `blocked` cells and the P0 lane currently pass unaudited; a reader of the "honest register" believes the only gaps are R7/R8/R10/git-history. What correct looks like: either implement (R5 ref-resolution is ~5 lines reusing unresolved_paths) or list all gaps.
Confidence: HIGH — Verification: read (rule texts) + ran (R5 emptiness-only confirmed by synthetic row 5 firing only the missing-key violation, none for a prose reason).
Note: the R2-P0 rule is marked `[PROPOSAL]` in the matrix (open question Q3), which softens but does not excuse the missing disclosure.

F-003 [S4]
Where: tools/xtask/src/cmd/feature_audit.rs:80, 182-189
Code:
```
    line_hint: usize,                // array index + 1 for messages
```
What is wrong: `line_hint` is a ROW ORDINAL, not a file line — baseline violations read "23:"/"163:" (rows 23/163, verified by awk extraction) but a user will grep TOML line 23 (inside the header comment block). The parse-error branch does report real file:line (toml_lite ParseError.line), so the two message styles are inconsistent; toml_lite's ArrayRow (toml_lite.rs:95-99) carries no line, which explains but does not justify the name.
Confidence: HIGH — Verification: read + ran (row-23/163 extraction matched cited evidence exactly).
F-004 [S4]
Where: tools/xtask/src/cmd/feature_audit.rs:443-453, 471-478
Code:
```
const ROOT_FILES: [&str; 9] = [
    "product.toml",
    ...
```
What is wrong: ROOT_FILES is dead in practice — `check_token` returns None for any token without `/` (line 471-473) BEFORE the ROOT_FILES lookup, and a root FILE name as the first segment of a multi-segment path (`Cargo.toml/x`) is nonsense. Evidence strings citing `product.toml` or `README.md` bare are never validated. The 9-entry list suggests a guarantee that cannot fire.
Confidence: HIGH — Verification: read (control flow).
F-005 [S4]
Where: tools/xtask/src/cmd/feature_audit.rs:335-350, 204-210
Code:
```
    let evidence = get("evidence")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
```
What is wrong: non-string evidence/notes/blocked_reason/category values silently coerce to ""/"?": a `not_started` row with `evidence = 123` yields no violation; a non-int `phase_target` (e.g. the string "5") reports `phase_target None outside 2..=8` — the actual offending value is discarded from the message.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run | 87-266 | REAL | S3 | arg parsing, meta/R9 checks, full row loop (A/B/C); executed end-to-end twice + synthetic |
| finish | 269-306 | REAL | — | report + exit code; executed (baseline reproduction) |
| fmt_counts | 308-313 | REAL | — | executed (per-phase/category/status lines match baseline) |
| sorted_platforms | 315-319 | REAL | — | |
| parse_row | 325-371 | REAL | S4 | F-005 coercions; missing-key errors work |
| valid_id | 373-389 | REAL | — | faithful ^[a-z0-9]+(-[a-z0-9]+)+$; test-verified |
| unresolved_paths | 407-415 | REAL | S3 | F-001: existence-only semantics |
| path_tokens | 418-432 | REAL | — | conservative tokenizer; prose/URL immunity test-verified |
| check_token | 457-502 | REAL | S3/S4 | F-001 (exists() only), F-004 (ROOT_FILES dead); anchor-strip, inventory mapping, glob-prefix all executed |
| find_root | 508-525 | REAL | — | Cargo.toml+docs heuristic |
| tests (4) | 557-601 | REAL | — | honest; see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| id_regex_matches_matrix_conventions | YES | 6 asserts incl. negatives (uppercase, single-segment, empty segment, underscore) |
| unresolved_paths_ignores_prose_and_urls | YES | real temp dir; "Matroska/WebM", "3/10/60/300", URL immunity |
| unresolved_paths_flags_missing_files_and_accepts_existing | YES | real FS; flags missing.rs, accepts existing — and bakes in the empty-file acceptance (writes a 0-byte file as the passing case) |
| unresolved_paths_maps_inventory_shorthand | YES | real FS; pass + fail directions |
Gap: no unit tests for run()/parse_row/finish/checks A-B — those are exercised by the integration suite tools/xtask/tests/feature_audit.rs (next file).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "CI's inline python3 check covers R9/R1/R2/R3, R4/R5/R6 emptiness conditionals; this tool adds evidence RESOLUTION and honest counting" | feature_audit.rs:5-8 | REAL | ci.yml:284-405 verified: python check is exactly that set (verified-only evidence); tool adds B-for-implemented + C + D (executed) |
| "R4 as written: 'evidence resolving to an existing artifact'" | feature_audit.rs:14 | SHALLOW (partial quote) | F-001: full R4 (doc 02:555) demands artifact "recording commands + actual results" + mock labeling — not implemented |
| "NOT done here (honest register, same as the header's owed list)" | feature_audit.rs:17-22 | FALSE (as 'same as') | F-002: matrix owed list also has R1 prefix-map, R2-P0, R5-ref-existence — absent from implementation and disclosure |
| "Exit codes: 0 pass \| 1 violations \| 2 xtask error (parse failures are exit 1 with a `parse:` line)" | feature_audit.rs:24-26 | REAL | executed: violations -> 1; corrupt matrix -> `parse: line 1: ...`, exit 1 |
| matrix: "The full tools/feature-audit (R1 category-prefix map, R2-P0 rule, R4 evidence resolution, R7, R8, R10) is OWED" | docs/feature-matrix.toml:10-16 | PARTIAL | tool delivers R4-resolution (existence clause) only; R7/R8/R10 still owed; R1-prefix/R2-P0 owed but undisclosed by the tool |
| ci.yml honest-skip ledger: "R7/R8/R10 + full R4 need tools/feature-audit" | ci.yml:403 | SHALLOW | the tool exists and works but is wired into NO CI lane — CI still runs only the weaker python check |

## 7. Dead / duplicated / unreachable code
- ROOT_FILES (F-004): effectively dead constant list.
- Nothing else dead; all functions reachable from run().

## 8. Dependencies & flags
- std-only via toml_lite (workspace-internal, 611 loc, line-tracking for parse errors only). No external crates, no feature flags, no unsafe. Uses `crate::XtaskError` for exit-2 paths (bad flags, unreadable file, no root).

## 9. Open questions for the next agent
- Who wires feature-audit into CI (the ci.yml skip ledger says "needs tools/feature-audit" — it exists since card 8.F1; the wiring ticket is open)? Until then its 2 current findings block nothing.
- Row 23's `sw/**` vs `sw.rs`: is the intended layout a `sw/` directory (matrix says so) or did the code land flat? That decides whether the matrix or the code is the stale side — feeds the h264 "implemented ×3 vs shipped-build-has-no-h264-decoder" contradiction (registry-lie thread, baseline mp4-smoke).
- tools/xtask/tests/feature_audit.rs (next file): does the integration suite cover run()/A/B checks, and are its fixtures honest?
