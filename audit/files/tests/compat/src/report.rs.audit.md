# AUDIT: tests/compat/src/report.rs
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 2212 / 26 fns + 8 tests |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — test infra: invoked by `cargo run --manifest-path tests/compat/Cargo.toml -- run` (.github/workflows/compat-matrix.yml:154) via `main.rs:142 report::full_run(&config)`; not shipped in lumen-cli/desktop builds |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "full_run" tests/compat/src/main.rs` → main.rs:142; `rg -n "manifest-path tests/compat" .github/workflows/*.yml` → only compat-matrix.yml:154 (`cargo run`, NOT `cargo test`); `sed -n` quote verification (all quotes below verified verbatim) |

## 1. One-paragraph verdict
The compat-matrix report generator is a genuine, elaborate implementation: it drives the real open/decode path in-process, compares against a version-asserted VLC oracle with MD5-prefix + f32-tolerance methods, and renders markdown/JSON/transition outboxes. The module doc's honesty contract (never edits feature-matrix.toml; blocked lanes cited verbatim) is upheld in code — all writes go to config.reports.{json,markdown,transitions} (L789-811), none to docs/. The unit tests test the rendering layer honestly (they assert on constructed FileRecords and would fail if format_cell/steps_cell logic broke). The two real defects are process-level: the 8 unit tests are never executed by any committed lane, and the steps-lane PASS prose is still hardcoded wasm text despite a comment claiming the opposite.

## 2. What actually works (verified)
- `rg -n "full_run" tests/compat/src/main.rs` → `142: report::full_run(&config).map(|_| ExitCode::SUCCESS)` — entry wiring exists.
- Read the whole file (2212 lines): full_run assembles corpus (L249-268), asserts the oracle version before use (L273-277), decodes every file via `crate::decode::decode_file` (L296-319) with incremental JSONL appends (partial evidence survives kill — L312-317), oracle comparison with live-dump preference + committed-record applicability check (L334-471), gap-rule transitions (L1256-1448), markdown render (L1542-1863). No write to docs/feature-matrix.toml anywhere in the file.
- Decode is in-process (decode.rs decode_file calls `open_source` directly, L343-354 of decode.rs), so count_file's "a crash would have killed the run" premise (L866-873) is structurally true: FileOutcome has no crash variant.
- NOT executed here (cargo not installed). Committed evidence for the RUN (not the unit tests): audit/runs/baseline has no compat run; the CI workflow .github/workflows/compat-matrix.yml:154 runs the tool on 3 OS. Unit-test pass status = UNVERIFIED.

## 3. Findings
F-001 [S3]
Where: tests/compat/src/report.rs:1906-2212 (mod tests, 8 tests) + .github/workflows/compat-matrix.yml:154
Code:
```
        run: cargo run --manifest-path tests/compat/Cargo.toml --locked -- run --config ci-out/compat-matrix-ci.json
```
What is wrong: The 8 unit tests (mixed_row_surfaces_its_refusals, distinct_refusal_messages_are_each_counted, pure_row_has_no_refusal_clause, failed_wasm_step_renders_steps_failed, passing_wasm_steps_render_engine_core_only, steps_lane_without_records_claims_nothing, steps_failure_detail_is_parameterized_not_wasm_prose, all_refused_row_stays_refused) live in a STANDALONE workspace (tests/compat/Cargo.toml declares an empty `[workspace]` table, deliberately outside the root workspace), and NO committed lane runs `cargo test --manifest-path tests/compat/Cargo.toml`: CI runs only `cargo run`. `rg -n "manifest-path tests/compat" .github/workflows/*.yml scripts/*.sh` → exactly one hit, the `cargo run`. The baseline cargo-test-workspace.txt (root workspace, 4744 passed) cannot contain them. Why it matters: the P2-1/P2-2/R10-3 regression tests that guard the report's honesty are dead-in-CI — a regression in refusal-surfacing or steps-failure rendering would not fail any gate. What correct looks like: a CI step `cargo test --manifest-path tests/compat/Cargo.toml` (or promote the crate into the root workspace when Lead ratifies).
Confidence: HIGH — Verification: read (workflows rg'd; no cargo available).

F-002 [S3]
Where: tests/compat/src/report.rs:1075-1084 and 1208-1215 (pass-detail call sites) vs comment 920-930
Code (L920-925 comment):
```
/// (review finding P2-2): the cell text can never claim a build the
/// lane's own recorded steps contradict. Both detail strings come
/// from the CALL SITE, so the cell never embeds lane-specific prose
/// the config did not vouch for (R10-3: the failure arm used to
/// hard-code the web-wasm decoder-less clause for ANY Steps lane).
```
Code (L1078-1081 actual call):
```
            "wasm32 module builds (see the lane's recorded steps); engine core (probe + demux \
             of this family) compiled in; DECODE refused — the wasm32 module ships decoder-less \
             (codecs crate does not build for wasm32; apps/web/src/capabilities.rs reports the \
             split); no in-browser run in this matrix",
```
What is wrong: R10-3 parameterized only the FAILURE arm. The PASS arm at both call sites (format_cell L1075-1084, codec_cell L1208-1215) still hardcodes web-wasm prose for ANY Steps lane; the prose is vouched by neither the config nor the recorded steps (StepRecord carries argv/outcome, not prose). A hypothetical `cli-smoke` Steps lane whose steps all pass would render "wasm32 module builds ... ships decoder-less" — exactly the lie class R10-3 claims was fixed. The comment "the cell never embeds lane-specific prose the config did not vouch for" is FALSE as written. Why it matters: comments asserting a fixed bug that is half-fixed is how the next regression ships. What correct looks like: pass_detail sourced from the lane/config (like reason/next_step in BlockedCitation), or restrict Steps lanes to web-wasm in the config schema.
Confidence: HIGH — Verification: read; the only escape is if config.rs forbids non-web Steps lanes (UNVERIFIED — config.rs not in my assignment).

F-003 [S4]
Where: tests/compat/src/report.rs:1545-1547
Code:
```
    out.push_str(
        "# Full-corpus compatibility matrix — 2026-11-10 (issue #53, cards 8.B1–8.B2)\n\n",
    );
```
What is wrong: the rendered markdown title hardcodes the date 2026-11-10; every future run (the dynamic `generated` field elsewhere) still prints a 2026-11-10 title. Misleading evidence dating. Why it matters: this file is the evidence string cited by every gap-rule transition; dating it statically makes the evidence trail lie about when it was produced. What correct looks like: use `report.generated`'s date prefix.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: tests/compat/src/report.rs:1898-1904
Code:
```
fn truncate(text: &str, len: usize) -> String {
    if text.len() <= len {
        text.to_string()
    } else {
        format!("{}…", &text[..len])
    }
}
```
What is wrong: `&text[..len]` byte-slices a str — panics on a non-char boundary. Today inputs are hex MD5s (ASCII) so unreachable, but the function is a general helper. What correct looks like: `char_indices()` boundary check. Confidence: HIGH — Verification: read (latent; no non-ASCII input path exists in this file).

## 4. Per-function table (>100 LOC mandatory)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| full_run | 241-819 | REAL-UNPROVEN | — | orchestration; oracle live/absent branches both real; in-process decode |
| run_step | 834-863 | REAL-UNPROVEN | — | proc::run_bounded; artifacts only recorded when metadata succeeds (missing artifact silently omitted, not flagged as missing) |
| count_file | 866-882 | REAL-UNPROVEN | — | malformed_typed counts all malformed outcomes; typed-by-construction (in-process decode, no crash variant) |
| outcome_message | 885-894 | REAL-UNPROVEN | — | plain mapping |
| container_verdict | 897-918 | REAL-UNPROVEN | — | level ordering correct; refused > demux-fatal > decoded |
| steps_cell | 931-962 | REAL-UNPROVEN | S3 | failure arm parameterized; pass arm text supplied by callers (hardcoded wasm, F-002) |
| lane_steps | 969-975 | REAL-UNPROVEN | — | empty-slice fallback documented |
| format_cell | 978-1099 | REAL-UNPROVEN | — | honest denominators (valid+refused), refusal messages surfaced |
| claim_out | 1102-1110 | REAL-UNPROVEN | — | all four platform columns |
| refusal_breakdown | 1117-1129 | REAL-UNPROVEN | — | BTreeMap deterministic order |
| codec_cell | 1132-1230 | REAL-UNPROVEN | S3 | pass arm wasm prose (F-002) |
| container_gap | 1256-1344 | REAL-UNPROVEN | — | refused/demux-fatal downgrades + not_started upgrade; other claim×verdict combos silently ignored (documented scope) |
| codec_gap | 1349-1448 | REAL-UNPROVEN | — | no-decoder / concealed downgrade rules |
| audiofile_format_for / oracle_rel_for / comparable_entry | 1452-1482 | REAL-UNPROVEN | — | manifest-driven lookup |
| safe_name | 1485-1487 | REAL-UNPROVEN | — | slash→underscore |
| git_commit | 1490-1504 | REAL-UNPROVEN | — | runs `git rev-parse HEAD` via proc; absent git → None (honest) |
| iso8601_now / civil_from_days | 1508-1536 | REAL-UNPROVEN | — | Hinnant algorithm, correct |
| render_markdown | 1542-1863 | REAL-UNPROVEN | S4 | F-003 hardcoded date; rest faithful to RunReport |
| native_lane_of / native_platform_of | 1866-1885 | REAL-UNPROVEN | — | config-sourced with hardcoded fallbacks |
| short_row / truncate | 1888-1904 | REAL-UNPROVEN | S4 | truncate latent panic (F-004) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mixed_row_surfaces_its_refusals | YES | asserts exact "2/3 files open+demux", "×1" refusal count and quoted message in detail |
| distinct_refusal_messages_are_each_counted | YES | asserts exact sorted breakdown string |
| pure_row_has_no_refusal_clause | YES | asserts no "REFUSED" substring for pure row |
| failed_wasm_step_renders_steps_failed | YES | asserts status steps-failed + "exit 101" + NOT "module builds" |
| passing_wasm_steps_render_engine_core_only | YES | byte-exact detail strings + timeout arm |
| steps_lane_without_records_claims_nothing | YES | asserts not-covered + "recorded no steps" |
| steps_failure_detail_is_parameterized_not_wasm_prose | YES | asserts wasm32 absent for non-web lane failure |
| all_refused_row_stays_refused | YES | asserts refused status + "2/2 corpus files REFUSED" |
All 8 test the RENDERER only (constructed records) — honest unit tests, but none execute full_run/decode/oracle; and none are run by any committed lane (F-001). Mutation reasoning only; no execution (cargo absent).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "THIS TOOL NEVER EDITS docs/feature-matrix.toml" | module doc L26-28 | REAL (code read) | only writes are reports.{json,markdown,raw_rows,transitions} at L789-811 |
| "Both detail strings come from the CALL SITE, so the cell never embeds lane-specific prose the config did not vouch for" | doc comment L922-924 | FALSE (half) | call-site prose at L1078-1081 is hardcoded wasm, not config-sourced (F-002) |
| "a mismatched oracle is refused, never silently used" | module doc L9-10 | REAL-UNPROVEN | assert_version called L273 before use; oracle.rs not my file |

## 7. Dead / duplicated / unreachable code
- None dead. `Counts.oracle_other` conflates LumenRefused/OracleFailed/other verdicts into one bucket (lossy aggregation; markdown §5 does itemize per-row).

## 8. Dependencies & flags
- Crate standalone workspace (tests/compat/Cargo.toml `[workspace]` empty); deps: lumen-cli, commands, codec-api, media-types, plugin-api, playback-engine, bench-vs-vlc, serde, serde_json. Cargo.toml note: codecs/demux/input/media-probe direct deps removed as dead (audit T5, 2026-11-17) — consistent with imports in this file.

## 9. Open questions for the next agent
- Does tests/compat/src/config.rs schema restrict Steps lanes to web-wasm (would neutralize F-002's practical half)?
- Does tools/xtask/src/cmd/compat_matrix.rs apply-transitions verify `from` status (claimed; not my file)?
