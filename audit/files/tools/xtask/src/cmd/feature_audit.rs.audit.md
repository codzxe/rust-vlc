# AUDIT: tools/xtask/src/cmd/feature_audit.rs

| Field | Value |
|---|---|
| Auditor | W01-A01 |
| Date | 2026-11-16 |
| Lines / functions | 602 / 12 (run, finish, fmt_counts, sorted_platforms, parse_row, valid_id, unresolved_paths, path_tokens, check_token, find_root + 4 test fns) |
| File verdict | REAL (as a path-existence checker) — the S0 is in what its output is used to claim |
| Reachable from shipped entry point? | NO — dev tooling, entry `cargo xtask feature-audit` (used by scripts/full_gate.sh gate 1?? no: gates 4-8; referenced by CI that has never executed, see release.yml:26-28) |
| Findings | S0:1 S1:0 S2:0 S3:2 S4:1 |
| Commands run | `cargo xtask feature-audit` → exit 1, 2 R4 violations (see `audit/runs/baseline/xtask-feature-audit.txt`); hostile experiment below → exit 0 PASS; `cargo test --workspace` includes this file's 4 tests (1001 passed overall) |

## 1. One-paragraph verdict
The file is a real, honest, well-tested TOML-structure auditor for `docs/feature-matrix.toml`. Its check C resolves evidence strings to filesystem paths and accepts anything that `exists()`. It does not — and admits at lines 17-22 it does not — verify that the evidence proves the row's status. Executed against the committed matrix it finds 2 stale-path violations and prints implemented=193 platform cells; executed against a fabricated matrix whose "implemented on all four platforms" row cites a **0-byte** file as evidence, it returns **PASS, exit 0**. The number "implemented" in this repo's paperwork therefore certifies exactly: *a path named in a string exists*. That is the load-bearing pillar of every "implemented" claim in the matrix, and it is a pillar of sand.

## 2. What actually works (verified)
- Structure checks A/B (id regex, uniqueness, priority, phase range, platform columns, evidence-presence, blocked_reason/notes conditionals) — verified by reading lines 180-261 and by the committed-matrix run flagging real violations (exit 1).
- The 4 unit tests are real tests with real temp-dir fixtures — they ran in the workspace suite (part of the 1001 passing).
- Row/parse error paths propagate real file:line info for TOML parse errors (lines 133-143).

## 3. Findings

### F-001 [S0] Evidence resolution is filesystem existence; an empty file passes
- **Where:** `tools/xtask/src/cmd/feature_audit.rs:489-501`
- **Code:**
  ```rust
      return if root.join(path).exists() {
          None
      } else {
          Some(path.to_string())
      };
  }
  ```
- **What is wrong:** The entire evidentiary standard behind matrix status "implemented"/"verified" is `Path::exists()`. No content check, no call-chain check, no execution. The tool's own header (lines 17-22) registers the gap ("R10 anti-hallucination beyond path existence" NOT done) — the code is honest; the ecosystem that cites "implemented" rows as accomplishment is not.
- **Why it matters:** Executed proof — with a fabricated root (`/tmp/devil-fake`) containing one row `status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "implemented" }` and evidence `crates/fake/decoder.rs` where that file is **0 bytes**, the real tool prints `result: PASS` and exits 0 (`audit/runs/baseline/experiment-feature-audit-empty-file.txt`). Therefore the 193 "implemented" platform cells on the committed matrix (my run, same tool) are certified to the same standard: the cited path exists. Meanwhile the shipped binary's own engine-info says `decoders: mjpeg, pcm_s16le` and decode-bench on an H.264+AAC MP4 decodes 0 frames (`audit/runs/baseline/mp4-smoke-decode-bench.txt`). "Implemented" as audited by this tool is compatible with "does not exist".
- **What correct looks like:** Check C stays (it is necessary), but matrix status must be gated on a stronger verifier: at minimum, evidence strings pointing at test entry points that the CI actually runs and fails on regression, plus the R10 admission turned into a hard error (rows claiming `implemented` for codecs the shipped binary does not register are FALSE, not implemented).
- **Confidence:** HIGH — **Verification:** ran (both runs pasted in `audit/runs/baseline/`).

### F-002 [S3] "violation: 23:" cites a row index, not a file line
- **Where:** `tools/xtask/src/cmd/feature_audit.rs:325-326`
- **Code:**
  ```rust
  fn parse_row(src: &toml_lite::ArrayRow, index: usize) -> Result<Row, String> {
      let line_hint = index + 1;
  ```
- **What is wrong:** `line_hint` is the array index + 1, printed as `violation: 23: …`. It reads as a line number (the module doc, line 26, promises "the message names the file:line") but is a row number; in the real matrix, row 23 is not line 23 (the file has ~3400 lines). Anyone jumping to line 23 lands on the wrong row.
- **Why it matters:** Wastes audit time; violates the file's own stated message contract for all non-parse violations.
- **What correct looks like:** Track the real line from the parser (toml_lite must expose it) or label it `row 23`.
- **Confidence:** HIGH — **Verification:** read + compared with the real output (`violation: 23:` vs `docs/feature-matrix.toml` line 23).

### F-003 [S4] `valid_id` accepts digit-only segments
- **Where:** `tools/xtask/src/cmd/feature_audit.rs:373-389`
- **Code:**
  ```rust
      let rest_ok = segments.all(|s| {
          !s.is_empty()
              && s.chars()
                  .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
      });
  ```
- **What is wrong:** `2x-rate` passes (the test at line 561 even asserts `hotkey-2x-rate` as valid), while the R1 doc regex `^[a-z0-9]+(-[a-z0-9]+)+$` technically also allows `2-2-2`. A semantic-id check it is not; it is a charset check. Cosmetic given the current ids.
- **Confidence:** HIGH — read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run` | 87-266 | REAL | — | arg parse, reads matrix, orchestrates A/B/C/D; parse-failure → exit 1 honest |
| `finish` | 268-306 | REAL | — | prints derived counts; exit code 0/1 by violations; nothing cached |
| `fmt_counts` | 308-313 | REAL | — | trivial formatter |
| `sorted_platforms` | 315-319 | REAL | — | trivial |
| `parse_row` | 325-371 | REAL | S3 | line_hint = row index mislabeled as line (F-002) |
| `valid_id` | 373-389 | REAL | S4 | charset-only (F-003) |
| `unresolved_paths` | 407-415 | REAL | — | the S0 surface (F-001), itself honest |
| `path_tokens` | 418-432 | REAL | — | maximal runs of path-plausible chars; prose exclusion by charset |
| `check_token` | 457-502 | REAL | — | anchors stripped, `inventory/` mapping, `**` prefix check, `exists()` |
| `find_root` | 508-525 | REAL | — | walks up for Cargo.toml+docs |
| tests (4 fns) | 557-600 | REAL | — | would fail on regression; ran in workspace suite |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `id_regex_matches_matrix_conventions` | YES | asserts concrete accept/reject pairs |
| `unresolved_paths_ignores_prose_and_urls` | YES | real temp dir, real strings |
| `unresolved_paths_flags_missing_files_and_accepts_existing` | YES | exact-equality assert on missing list |
| `unresolved_paths_maps_inventory_shorthand` | YES | exact-equality assert |
All four ran (part of the 1001-pass workspace run, 2026-11-16).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "feature-audit only verifies that file paths named in evidence strings exist" | DEVIL-AUDIT-PREMISE §0 | REAL (confirmed) | code lines 489-501 + empty-file experiment PASS |
| "Its own gap list admits anti-hallucination beyond path existence is not done" | premise §0 | REAL | lines 17-22 verbatim: "R10 anti-hallucination beyond path existence" under "NOT done here" |
| "64/252 implemented" style counts | docs/PARITY-REPORT (to be audited W19) | PARTIALLY | my run: implemented=193 *platform cells* across 252 rows; 66/252 rows implemented∪verified on windows |

## 7. Dead / duplicated / unreachable code
None found. `Must` trait duplicated from parity_report.rs tests (S4 duplication, both files lines ~531-555).

## 8. Dependencies & flags
No feature flags. Depends only on toml_lite + std. Buildable in the workspace (verified by execution).

## 9. Open questions for the next agent
- W19: which docs quote "64/252" and on what date; the number moved to 66 (windows) at HEAD.
- W18: for every matrix row whose status claims implemented, does the cited evidence contain a runnable proof? (F-001 says the tool cannot answer this; the claim ledger must.)
