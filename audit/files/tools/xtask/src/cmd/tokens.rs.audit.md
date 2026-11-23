# AUDIT: tools/xtask/src/cmd/tokens.rs
| Field | Value |
|---|---|
| Auditor | W01-A08 |
| Date | 2026-11-23 |
| Lines / functions | 80 / 4 (cargo_driver, workspace_root, run, WORKSPACE_ROOT const) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev command. Dispatched via cmd/mod.rs:92 (`run: tokens::run`). Zero automated callers: rg over scripts/ and .github/ for `xtask tokens` = 0 hits; full_gate.sh has no tokens step (full_gate.txt steps 1-11); ci.yml:406 explicitly SKIPs themes/. Manual/dev invocation only. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg "tokens" scripts/full_gate.sh (0), gate.rs (0), .github/workflows (SKIP note at ci.yml:406), ui/package.json (npm tokens:generate/check exist); ls of both generated outputs; baseline citations (cargo-test-workspace.txt:6290-6295) |

## 1. One-paragraph verdict
A working subprocess bridge, and nothing more: it shells out to the real `tokens-gen` binary via `cargo run --quiet -p theme-tokens --bin tokens-gen` with inherited stdio and verbatim exit-code forwarding (signal -> 2, out-of-range -> 2). Both committed outputs exist, and the drift protection the pipeline needs is REAL — but it lives in `crates/theme-tokens/tests/diff.rs` inside `cargo test` (baseline: `committed_outputs_match_regeneration ... ok`), not in this command. The defect is the CLI contract: `run(_args: &[String])` ignores every argument, so the `cargo xtask tokens --check` invocation that docs/08-ui-figma-fidelity.md:244 claims "in CI" does not exist — typing it silently REGENERATES the committed outputs (mutating the tree) instead of checking drift, and no CI workflow runs any tokens check at all (ci.yml:406 says SKIP). The bridge also skips the --help/unknown-arg contract every sibling subcommand implements.

## 2. What actually works (verified)
— verified by citation of audit/runs/baseline/cargo-test-workspace.txt:6290-6295: the drift gate ran and passed — `committed_outputs_match_regeneration ... ok`, `the_comparison_detects_drift ... ok` (tests/diff.rs, 2 passed) — proving the committed themes/generated/tokens.css, crates/theme-tokens/src/generated/tokens.rs and the UI bridge copy are byte-stable against themes/tokens.json as of the baseline.
— verified by read: exit-code forwarding is verbatim (`Some(code) => ExitCode::from(u8::try_from(code).unwrap_or(2))`, signal -> 2), stdio inherited (plain spawn, no pipes), generator errors surface (cargo exit 101 propagates).
— verified by ls: both documented outputs exist (themes/generated/tokens.css; crates/theme-tokens/src/generated/tokens.rs), and the target binary exists (crates/theme-tokens/src/bin/tokens_gen.rs, which itself supports `--check`, exit 1 on drift, per its L19-52 usage text).

## 3. Findings
F-001 [S3]
Where: tools/xtask/src/cmd/tokens.rs:52 + docs/08-ui-figma-fidelity.md:244
Code:
```
pub fn run(_args: &[String]) -> Result<ExitCode, XtaskError> {
    let program = cargo_driver();
```
```
- `cargo xtask tokens --check` in CI: regenerated output is byte-stable (no drift between committed CSS/Rust and tokens.json).
```
What is wrong: Doc 08:244 documents a `cargo xtask tokens --check` CI check that does not exist in two ways: (a) this bridge ignores ALL arguments (`_args`) and forwards none to tokens-gen — so `cargo xtask tokens --check` does not check anything; it silently runs a plain REGENERATION, rewriting the two committed outputs (working-tree mutation from an invocation documented as a read-only check); (b) no CI workflow runs any tokens step — ci.yml:406: "SKIP themes/: the theme-token pipeline exists (tokens-gen --check) but is outside card 1.E2's ... scope — wire with the next G1 expansion." The byte-stability invariant IS enforced, but by crates/theme-tokens/tests/diff.rs wired into `cargo test` (full_gate step 3), a different mechanism than the documented one. The module's own doc (L16-18) compounds it: "the generator's exit status is forwarded verbatim (a failed regeneration must fail the gate)" — forwarding is real, but NO gate invokes this bridge (rg over scripts/ + .github/: zero callers). Why it matters: a developer following doc 08 mutates committed files when they believe they are running a check. What correct looks like: forward args to tokens-gen (which already implements --check) or implement --check in the bridge, and fix doc 08:244 to name the real enforcement (tests/diff.rs). Confidence: HIGH — Verification: read (bridge + doc + ci.yml + tokens_gen.rs usage) + baseline citation.

F-002 [S4]
Where: tools/xtask/src/cmd/tokens.rs:52
Code:
```
pub fn run(_args: &[String]) -> Result<ExitCode, XtaskError> {
```
What is wrong: Ignored parameters break the frozen CLI contract every sibling implements: no `--help`/`-h` (rebrand, check-identity, config-validator all print usage), no unknown-arg rejection (siblings exit 2 with "unexpected argument"), no flag at all. `cargo xtask tokens --anything` silently regenerates. What correct looks like: the same parse_args shape as cmd/rebrand.rs:69-114. Confidence: HIGH — Verification: read (compared against sibling subcommands).

F-003 [S4]
Where: tools/xtask/src/cmd/tokens.rs:26-46
Code:
```
const WORKSPACE_ROOT: &str = env!("CARGO_MANIFEST_DIR");
```
What is wrong: Compile-time path baked into the binary, with an in-code rationale ("the xtask binary only ever runs from inside the workspace it was built by"). Correct for this layout (manifest dir minus two components = root; verified against the tree), but it makes the binary non-relocatable and diverges from the runtime root-discovery (`find_workspace_root` walking ancestors) used by rebrand/check-identity/gate. Two root-discovery strategies in one tool. Not a defect today; structure debt. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| WORKSPACE_ROOT const | 29 | REAL | S4 | compile-time manifest dir (F-003) |
| cargo_driver | 33-38 | REAL | — | $CARGO with PATH fallback |
| workspace_root | 41-46 | REAL | — | strips 2 components; fallback arm unreachable in practice |
| run | 52-80 | REAL | S3/S4 | bridge works; ignores args (F-001/F-002); exit forwarding correct |

## 5. Test honesty
No tests in this file. The pipeline's substance is tested elsewhere and passing (baseline): crates/theme-tokens/tests/diff.rs (2/2 drift tests), pipeline.rs (12/12), semantic_lint.rs — all in cargo-test-workspace.txt:6290-6311. The bridge itself (spawn/forward logic) has NO test coverage anywhere — no integration test spawns `xtask tokens`. Its correctness is established only by code reading (this audit) — REAL-UNPROVEN for execution; the file-level verdict REAL reflects that the delegated generator and drift gate are executed and green in the baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "shells out to the crate's tokens-gen binary, which reads themes/tokens.json and (re)generates the two committed outputs" | header L9-14 | REAL | Command::args(L54-62); outputs exist; diff.rs proves regeneration matches committed files |
| "Inherited stdio keeps the generator's actionable diagnostics visible" | header L16-17 | REAL | plain spawn, no pipes (L54-68) |
| "the generator's exit status is forwarded verbatim" | header L17-18 | REAL | L75-79 |
| "(a failed regeneration must fail the gate)" | header L18 | FALSE | no gate/CI/script invokes `xtask tokens` (0 callers); the real drift gate is tests/diff.rs inside cargo test |
| "`cargo xtask tokens --check` in CI: regenerated output is byte-stable" | docs/08:244 | FALSE | bridge ignores --check and regenerates instead (F-001); ci.yml:406 SKIPs themes/ |

## 7. Dead / duplicated / unreachable code
None dead. Duplication note: second root-discovery strategy (compile-time) beside the runtime walker in sibling modules (F-003).

## 8. Dependencies & flags
std-only (path, process). No features. Requires cargo on PATH or $CARGO (inherited from the `cargo xtask` invocation). No --locked on the inner `cargo run` (a repin during a gate run could touch Cargo.lock; theoretical in a frozen workspace).

## 9. Open questions for the next agent
- ui/scripts/gen-tokens.mjs writes a THIRD output (ui/src/tokens/generated/tokens.css) and its comments reference invoking the crate pipeline — does the npm `tokens:generate` path also go through this xtask bridge, or call tokens-gen directly? (ui/ domain, unverified here.)
