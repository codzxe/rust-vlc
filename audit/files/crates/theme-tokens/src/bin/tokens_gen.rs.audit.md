# AUDIT: crates/theme-tokens/src/bin/tokens_gen.rs
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-21 |
| Lines / functions | 311 / 8 (main, run, parse_args, value_of, output_paths, find_workspace_root, drift, write_file + Failure/Options types) |
| File verdict | REAL (working regeneration + drift-check CLI; pipeline integrity verified end-to-end against committed outputs) |
| Reachable from shipped entry point? | N/A build tool — reachable from the BUILD pipeline: `cargo xtask tokens` → tools/xtask/src/cmd/tokens.rs:52-68 spawns `cargo run -p theme-tokens --bin tokens-gen` (verified). Not part of any shipped binary (correct for a codegen tool). Its outputs ARE shipped: themes/generated/tokens.css → verbatim copy ui/src/tokens/generated/tokens.css (byte-identical, diff-verified — see below). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read complete; read tools/xtask/src/cmd/tokens.rs complete (args swallowed — see F-001) + main.rs:112-125 dispatch (`&args[1..]` passed, ignored); `diff themes/generated/tokens.css ui/src/tokens/generated/tokens.css` → IDENTICAL (262 lines both); read crates/theme-tokens/tests/diff.rs:1-45 (the real drift gate: byte-compares all THREE committed outputs vs fresh emission inside `cargo test --workspace`); `rg -rn "xtask tokens" scripts/ .github/` → NO caller passes `--check` (latent bug); head ui/scripts/gen-tokens.mjs (pipeline doc: Rust side READ-ONLY source → crate CSS = bridge → verbatim copy + typed TS); baseline citations: run #3 (workspace tests, includes theme-tokens), run #24 (ui tokens:check PASS), orchrun-xtask-codegen-check (catalog-gen only — tokens NOT in that run). |

## 1. One-paragraph verdict
A clean, typed, exit-code-disciplined regeneration CLI that genuinely backs the token pipeline: the committed themes/generated/tokens.css is byte-identical to the UI bridge copy, and drift protection is real — not primarily via this binary's `--check` (which the xtask bridge never invokes, and whose args it silently swallows) but via crates/theme-tokens/tests/diff.rs byte-comparing all three committed outputs inside `cargo test --workspace`. The one defect worth a row: `cargo xtask tokens --check` does not check — it REGENERATES (writes files), because the xtask subcommand ignores its arguments, contradicting this file's own documented invocation `-- [args]`.

## 2. What actually works (verified)
— Pipeline integrity verified by evidence chain: themes/tokens.json → tokens-gen (this file) → themes/generated/tokens.css → ui/src/tokens/generated/tokens.css: `diff` → identical (262/262 lines); crates/theme-tokens/tests/diff.rs re-verifies byte equality on every `cargo test --workspace` (baseline run #3: 1681 passed includes it; w26.k18 log: "cargo test -p theme-tokens 38/38 (… 2 diff)").
— Exit-code contract verified against code: 0 success/no-drift, 1 drift (:124-129), 2 usage/IO/parse (:60-65, :92-118) — matches the header :19-21.
— Drift reporting: first differing line computed correctly (:283-289); missing output counts as drift with an actionable message (:272-278).
— `--out-dir` vs `--css-out/--rust-out` conflict rejected (:206-210); unknown args rejected (:200-202); `cargo run -p theme-tokens --bin tokens-gen` bridge verified in xtask (tokens.rs:54-63).
— UI side gated separately: baseline run #24 `npm run tokens:check` PASS.

## 3. Findings

### F-001 [S3]
Where: crates/theme-tokens/src/bin/tokens_gen.rs:15-17 (doc) with tools/xtask/src/cmd/tokens.rs:52 (the swallowing call site)
Code:
```
//! cargo run -p theme-tokens --bin tokens-gen --quiet -- [args]
```
```
pub fn run(_args: &[String]) -> Result<ExitCode, XtaskError> {
```
What is wrong: This binary documents that the xtask bridge forwards arguments (`-- [args]`), and implements a `--check` mode (dry-run drift detection, exit 1). The xtask `tokens` subcommand that shells out to it IGNORES its arguments (`_args`) — it always runs the generator in WRITE mode. `cargo xtask tokens --check` therefore silently REGENERATES the committed outputs instead of checking them.
Why it matters: A check flag that writes is a footgun: a CI lane or developer asking for a dry run mutates the tree (and could mask drift by "fixing" it in place). Mitigations found: no caller currently passes `--check` through xtask (rg over scripts/ + .github/workflows: zero), and the real drift gate is tests/diff.rs — so the bug is latent, not live.
What correct looks like: `run(args)` forwarding `args` after `--` (matching schema_gen/codegen subcommands' bridge pattern), or refusing unknown args loudly.
Confidence: HIGH — Verification: read of both files + dispatch (main.rs:125) + caller search.
(Cross-file defect recorded here because the doc contract lives in this file; the xtask crate belongs to another wave/agent.)

### F-002 [S4]
Where: crates/theme-tokens/src/bin/tokens_gen.rs:189-198
Code:
```
                match arg {
                    "--source" => options.source = Some(path),
                    "--css-out" => options.css_out = Some(path),
```
What is wrong: Duplicate flags silently override — `--source a --source b` takes b with no warning.
Why it matters: CLI hygiene; a typo'd double flag hides the first value. No current caller passes duplicates (single xtask invocation passes none).
What correct looks like: Reject a repeated flag as a usage error.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/theme-tokens/src/bin/tokens_gen.rs:249-251
Code:
```
        if let Ok(text) = fs::read_to_string(&manifest)
            && text.contains("[workspace]")
```
What is wrong: Workspace-root detection is a substring match on the manifest text — a package manifest containing the literal `[workspace]` inside a comment or a string would false-positive as the root.
Why it matters: Fragile heuristic; in-tree manifests don't trigger it (no such literals found in parents of the crate in practice — UNVERIFIED beyond the repo layout), and the failure mode is a confusing "reading …" error from the wrong root.
What correct looks like: Parse the TOML and test for a top-level `workspace` table (or walk up for the workspace's target-dir marker).
Confidence: MEDIUM — Verification: read; no false-positive demonstrated.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Failure` / `usage_error` | 54-65 | REAL | — | Typed failure + exit code 2. |
| `main` | 67-75 | REAL | — | Maps Failure → stderr + ExitCode::from(code); codes 1/2 only. |
| `run` | 77-163 | REAL | — | Orchestration: args → root → paths → parse → emit ×2 → check-or-write → counts line. All errors typed. |
| `Options` / `parse_args` | 165-212 | REAL | F-002 | Unknown args rejected; out-dir conflict rejected; duplicates silent (F-002). |
| `value_of` | 215-219 | REAL | — | Missing value → usage error. |
| `output_paths` | 222-238 | REAL | — | Defaults match the documented committed paths (themes/generated/tokens.css, crates/theme-tokens/src/generated/tokens.rs). |
| `find_workspace_root` | 242-265 | REAL | F-003 | Walk-up substring heuristic; clear failure message. |
| `drift` | 269-296 | REAL | — | Missing = drift; first-diff line; actionable message. |
| `write_file` | 298-311 | REAL | — | Creates parents; typed IO error. |

## 5. Test honesty
No tests in this file. The binary's behavior is covered indirectly:
- crates/theme-tokens/tests/diff.rs — the drift contract, byte-equality over all three committed outputs, runs in baseline run #3 (passed).
- tests/pipeline.rs (12 tests) — parse/emit pipeline (another agent's file, w26.k18 log: 38/38).
- Mutation note: if `--check` were broken to always report "no drift", tests/diff.rs would still catch actual drift (it re-emits independently) — the gate does not depend on this binary's check mode.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "emits BOTH committed outputs deterministically" | :4-8 | REAL | diff.rs byte-equality green in baseline run #3; w26.k18: "tokens-gen ×2 to /tmp byte-identical + committed == regeneration". |
| "The (frozen) xtask `tokens` subcommand shells out to this binary … `-- [args]`" | :10-17 | FALSE (args part) | tools/xtask/src/cmd/tokens.rs:52 `run(_args)` ignores args (F-001). |
| "Exit codes: 0/1/2 …" | :19-21 | REAL | Verified against code paths. |
| "committed outputs diff-tested (1.B4)" | docs/gates/phase1-exit.md:34 | REAL | tests/diff.rs read + baseline run #3. |

## 7. Dead / duplicated / unreachable code
None in this file. (`--check` mode is dead VIA THE XTASK BRIDGE (no caller) but is the documented manual workflow and is independently exercised by the crate's diff tests' contract comment — kept, correctly.)

## 8. Dependencies & flags
theme_tokens::{emit, parse} (emit.rs = A10 part1; parse.rs/model.rs = my file 10). No feature flags. Toolchain: uses let-chains (:249) — Rust 2024 edition, repo pin 1.99.1 (baseline toolchain.log).

## 9. Open questions for the next agent
- xtask crate owner (W17 lane per full_gate S3 note): confirm whether schema-gen/codegen subcommands forward args (if they do, `tokens` is the odd one out; if none do, the pattern is systemic).
- Who consumes crates/theme-tokens/src/generated/tokens.rs (the Rust constants) engine-side? doc 08 §3 claims renderer clear colors etc. — engine-lane cross-check.
