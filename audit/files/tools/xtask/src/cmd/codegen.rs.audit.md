# AUDIT: tools/xtask/src/cmd/codegen.rs
| Field | Value |
|---|---|
| Auditor | W01-A06 |
| Date | 2026-11-23 |
| Lines / functions | 87 / 3 (cargo_driver, workspace_root, run) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (dev tool). Reachable via `cargo xtask codegen` -> tools/xtask/src/cmd/mod.rs:95-97 (name "codegen"), and IS wired into CI: ci.yml:280 `cargo run -p xtask --locked -- codegen --check` (schema-validation job). Neither full_gate.sh nor rc-smoke.sh runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/xtask codegen --check` (no CARGO on PATH) -> xtask error, exit 2 (error path verified); `CARGO=~/.cargo/bin/cargo ./target/debug/xtask codegen --check` -> `catalog-gen: no drift — outputs match the catalog (65 commands / 26 events)`, exit 0; `ls` of the three committed outputs; `rg` dispatch + catalog-gen declaration (crates/commands/Cargo.toml:44) |

## 1. One-paragraph verdict
A thin, honest subprocess bridge and it works: I executed it end-to-end (`codegen --check` -> "no drift — outputs match the catalog (65 commands / 26 events)", exit 0) and also verified its failure path (cargo not on PATH -> typed XtaskError with context, exit 2). It spawns `cargo run --quiet -p commands --bin catalog-gen -- <forwarded args>` at the workspace root with inherited stdio and forwards the child's exit code (signal death -> 2, oversized codes -> 2). All three claimed committed outputs exist (schemas/commands.schema.json, ui/src/generated/commands.ts, schemas/generated/commands.checksum.txt) and catalog-gen is a real declared bin (crates/commands/Cargo.toml:44). Two hygiene-level weaknesses: the workspace root is a compile-time absolute path baked at build time (inconsistent with the walk-up discovery used by gate.rs/feature_audit.rs), and the inner `cargo run` is not invoked with `--locked` even though CI invokes the outer xtask with `--locked` — lockfile enforcement does not propagate through the bridge.

## 2. What actually works (verified)
— verified by `CARGO=~/.cargo/bin/cargo ./target/debug/xtask codegen --check` -> exit 0, `catalog-gen: no drift — outputs match the catalog (65 commands / 26 events)` (real child process, forwarded exit code, inherited stdio).
— verified by `./target/debug/xtask codegen --check` without cargo on PATH -> `xtask error: spawning `cargo run -p commands --bin catalog-gen`: No such file or directory`, exit 2 (the spawn-error path at lines 72-75 works).

## 3. Findings
F-001 [S4]
Where: tools/xtask/src/cmd/codegen.rs:31-34, 46-51
Code:
```
/// Workspace root, derived from this crate's manifest location
/// (`<root>/tools/xtask`). Compile-time constant: the xtask binary only
/// ever runs from inside the workspace it was built by.
const WORKSPACE_ROOT: &str = env!("CARGO_MANIFEST_DIR");
```
What is wrong: the root is the BUILD machine's absolute path baked into the binary. Move the checkout, or run a stale prebuilt binary from a different tree, and `workspace_root()` points at a directory that no longer exists (the spawn would then run cargo against the wrong/absent root). The assumption is documented, and true for the `cargo xtask` workflow (which builds in-place), but it is the third distinct root-discovery strategy in this crate (gate.rs walks up for `[workspace]`; feature_audit.rs walks up for Cargo.toml+docs; codegen.rs trusts the build path). Why it matters: inconsistent discovery = inconsistent behavior when invoked from odd cwds; the prebuilt binary in target/ is path-pinned. What correct looks like: reuse one walk-up helper for all subcommands.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: tools/xtask/src/cmd/codegen.rs:60-69
Code:
```
    command
        .args([
            "run",
            "--quiet",
            "-p",
            "commands",
            "--bin",
            "catalog-gen",
            "--",
        ])
```
What is wrong: the inner cargo invocation carries no `--locked` while CI's outer invocation is `cargo run -p xtask --locked -- codegen --check` (ci.yml:280) — the lockfile guarantee does not survive the bridge; a Cargo.lock drift could be silently resolved inside codegen. Same pattern presumably applies to the sibling bridges (tokens.rs, schema_gen.rs) — not audited here.
Confidence: HIGH — Verification: read + ci.yml citation.

## 4. Per-function table (file is 87 loc — under the 100-LOC table bar, included for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| cargo_driver | 38-43 | REAL | — | CARGO env or PATH fallback; verified both directions |
| workspace_root | 46-51 | REAL | S4 | F-001: compile-time path pinning |
| run | 57-87 | REAL | — | spawn/wait/forward; exit-code clamping correct (signal -> 2); executed |

## 5. Test honesty
No tests in this file. Coverage of the bridge comes from CI's `codegen --check` step (wired, ci.yml:280) and the drift gates on the committed outputs. Mutation reasoning (no edit): if run() dropped the args forwarding, CI's --check mode would break loudly; if it swallowed the exit code, the CI step could not fail on drift. UNVERIFIED by local test execution (no committed test targets this file directly).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "shells out to the crate's `catalog-gen` binary, which ... (re)generates the three committed outputs" | codegen.rs:9-17 | REAL | executed: catalog-gen ran, reported 65 commands / 26 events, no drift; all three output files exist |
| "Extra args are forwarded verbatim ... exit codes are 0 ok / 1 drift / 2 error" | codegen.rs:19-23 | REAL | --check forwarded (executed, exit 0); spawn error -> 2 (executed) |
| "Inherited stdio keeps the generator's actionable diagnostics visible" | codegen.rs:21-22 | REAL | generator's report line appeared in my run's output |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
std-only (path, process). No feature flags, no unsafe, no unwrap on runtime data (the only unwrap_or paths are documented fallbacks). Reads the CARGO env var at runtime.

## 9. Open questions for the next agent
- tokens.rs and schema_gen.rs are described as the same bridge pattern (code comment line 7-8) — do they share the same two S4 weaknesses (path pinning, no --locked)?
