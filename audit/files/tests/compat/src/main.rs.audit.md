# AUDIT: tests/compat/src/main.rs
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 204 / 5 fns + ToolError |
| File verdict | REAL-UNPROVEN (clean CLI dispatch; never executed in any committed lane — standalone workspace, config.rs F-001) |
| Reachable from shipped entry point? | NO (dev/evidence tool, tests/compat standalone workspace) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## 1. One-paragraph verdict
Small, disciplined CLI: typed ToolError (Usage/Io/Config/Subprocess) with no
panics/unwraps and `#![forbid(unsafe_code)]`, strict argv parsing (unknown
arguments refused, not ignored), honest exit-code contract (0 = completed,
2 = tool error, "a failed file is a RESULT, not a tool failure"), and the
documented separation of duties (runner emits a transitions outbox; only xtask
applies matrix edits). Zero defects found beyond a docs/usage nit.

## 2. What actually works (verified)
- Read end-to-end; dispatch table (:139-154) matches USAGE verbatim (run /
  ensure-corpus / compare-pcm / help|–help|-h); `--config` parser rejects
  missing values and unexpected tokens (:162-179) — verified line-by-line
  against the USAGE block (:60-77).
- Exit codes: tool error -> ExitCode::from(2) (:127-131) as documented (:38-44).

## 3. Findings
F-001 [S4] Where: main.rs:148 `compare-pcm` arm.
Code: `"compare-pcm" => oracle::compare_pcm_cli(&args[1..]),`
What is wrong: this arm forwards args without the strict `--config`-style
validation the other subcommands get; `compare_pcm_cli` is responsible for its
own usage errors (oracle.rs — outside this assignment, UNVERIFIED). If it
panics on empty argv the "no panics" claim (:79) fails there, not here.
Confidence: MEDIUM — read + UNVERIFIED callee.
Execution finding (shared): standalone tests/compat workspace — no committed
run of this binary or its tests exists in audit/runs/baseline/ (rg: zero
hits) — REAL-UNPROVEN.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ToolError + Display | 80-121 | REAL | — | 4 variants, context+source |
| main | 123-132 | REAL | — | exit 2 on error |
| run | 135-155 | REAL | S4 | dispatch; compare-pcm arg pass-through (F-001) |
| load_config_from | 159-181 | REAL | — | strict --config parser |
| find_workspace_root | 187-204 | REAL | — | walk-up Cargo.toml+docs rule, typed refusal |

## 5. Test honesty: no tests in this file; the crate's tests live in config.rs (5, REAL-UNPROVEN — never executed in a committed lane).
## 6. Doc claims: "runs every file through the real Lumen open/decode path ... lumen_cli::demux_host::open_source → pipeline_host HostDecoderProvider" (:4-15) — UNVERIFIED (decode.rs outside assignment); USAGE/dispatch consistency — REAL (read match).
## 7. Dead code: none.
## 8. Dependencies: std only + in-crate modules; engine crates only via decode.rs (not audited here).
## 9. Open questions: oracle.rs compare_pcm_cli robustness (F-001).
