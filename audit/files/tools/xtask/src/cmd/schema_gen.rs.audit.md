# AUDIT: tools/xtask/src/cmd/schema_gen.rs
| Field | Value |
|---|---|
| Auditor | W01-A05 |
| Date | 2026-11-23 |
| Lines / functions | 86 / 3 (cargo_driver, workspace_root, run) |
| File verdict | REAL (bridge executed end-to-end: --help exit 0, --check exit 0 no-drift) |
| Reachable from shipped entry point? | NO (dev tool) — reachable via `cargo xtask schema-gen` (alias in .cargo/config.toml): main.rs:124 lookup -> `schema_gen::run`; EXECUTED through the compiled xtask binary |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `./target/debug/xtask schema-gen --help` -> exit 2 (cargo not on sandbox PATH — reproduces the fallback gap); `env CARGO=~/.cargo/bin/cargo ./target/debug/xtask schema-gen --help` -> generator usage printed, exit 0; same + `--check` -> "no drift ... (121 entries)", exit 0; sed quote checks |

## 1. One-paragraph verdict
This is a 30-line subprocess bridge that shells out to the real
`settings-schema` crate's `schema-gen` binary via `cargo run`, forwarding
args, stdio and exit status. It is not a stub: I executed the full chain
through the compiled xtask binary — `--help` printed the generator's usage
(exit 0) and `--check` reported "no drift — outputs match ... (121
entries)" (exit 0), proving dispatch, cwd resolution, arg forwarding,
inherited stdio and status forwarding all work, and that the three
committed outputs in schemas/generated/ are currently in sync. The three
defects are robustness/hygiene: the PATH fallback for the cargo driver
reproducibly breaks when the binary is invoked outside `cargo xtask` in a
shell without cargo on PATH (my first run: exit 2); workspace_root()
silently degrades to the wrong directory instead of erroring; and the
bridge has no dispatch test (the identical codegen pattern has one).

## 2. What actually works (verified)
— `env CARGO=$HOME/.cargo/bin/cargo ./target/debug/xtask schema-gen
  --help` -> generator's full usage text on stdout (inherited stdio
  confirmed), exit 0 forwarded through ExitCode.
— same invocation + `--check` -> "schema-gen: no drift — outputs match
  /home/z/my-project/audit-workspace/schemas/settings.toml (121 entries)"
  + "121 entries across 11 screen(s) of 14 known areas; validated against
  .../product.toml", exit 0. The doc header's claims (reads
  schemas/settings.toml, joins product.toml, regenerates three committed
  outputs) are confirmed by the tool's own live output.
— committed outputs exist: schemas/generated/{settings.d.ts,
  settings-help.txt, settings-docs.md} (ls) and are drift-free (--check).
— exit-code mapping read-verified: child exit -> u8 (try_from, overflow ->
  2), signal (code None) -> 2, spawn/wait io errors -> typed XtaskError::Io
  (main.rs maps to exit 2). Consistent with main.rs's 0/1/2 contract.

## 3. Findings
F-001 [S4]
Where: tools/xtask/src/cmd/schema_gen.rs:35-42
Code:
```
/// The `cargo` driver that invoked us, when available (`cargo xtask`),
/// falling back to a plain `cargo` lookup on PATH.
fn cargo_driver() -> String {
    match std::env::var("CARGO") {
        Ok(path) => path,
        Err(_) => String::from("cargo"),
    }
}
```
What is wrong: the fallback assumes `cargo` is on PATH. Reproduced: in
this sandbox (cargo exists at ~/.cargo/bin but is NOT on PATH), running
the xtask binary directly -> "xtask error: spawning `cargo run -p
settings-schema --bin schema-gen`: No such file or directory (os error
2)", exit 2. The canonical entry (`cargo xtask`, alias per
.cargo/config.toml) always sets CARGO, so impact is limited to
direct-binary invocations (scripts, debugging) — but that is exactly when
a human runs it by hand. The spawn error also does not distinguish
"cargo missing" from any other ENOENT. What correct looks like: probe the
fallback with `Command::new("cargo").arg("--version")` or embed a clear
"`cargo` not found on PATH (invoke via `cargo xtask`)" usage message.
Confidence: HIGH — Verification: ran (reproduced exit 2, then fixed the
env and it worked).
F-002 [S4]
Where: tools/xtask/src/cmd/schema_gen.rs:44-50
Code:
```
/// Workspace root at runtime: `<root>/tools/xtask` minus two components.
fn workspace_root() -> &'static Path {
    Path::new(WORKSPACE_ROOT)
        .parent()
        .and_then(Path::parent)
        .unwrap_or(Path::new(WORKSPACE_ROOT))
}
```
What is wrong: on a too-shallow manifest dir the function silently
returns `<root>/tools/xtask` as the "workspace root" — a wrong-directory
degradation instead of a typed error (checklist 6.2 swallowed-error
shape). Unreachable in practice: CARGO_MANIFEST_DIR is a compile-time
constant of this repo's fixed layout, so this is a latent pattern
question, not a live bug. What correct looks like: expect() is banned by
workspace lints, but an `XtaskError::Internal("manifest dir ... is not
<root>/tools/xtask")` return is available and honest.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: tools/xtask/src/cmd/schema_gen.rs:56-86 (absence of a test; cf. tools/xtask/tests/cli.rs:80-98)
Code:
```
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
```
What is wrong: the bridge has no dispatch test. The byte-identical
pattern for `codegen` has one (tests/cli.rs:80-98
`codegen_bridge_dispatches_to_catalog_gen`, which spawns the binary with
`--help` and asserts exit 0 + output). schema-gen's only test presence is
the name appearing in the help listing (cli.rs:45). If the `cargo run -p
settings-schema --bin schema-gen` invocation string drifted (e.g. bin
renamed), no cargo test would fail — only a human or CI gate run. What
correct looks like: the same 8-line `--help` probe test as codegen (the
generator's --help writes nothing).
Confidence: HIGH — Verification: ran (grep of tests/ for schema-gen
dispatch) + read.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| cargo_driver | 37-42 | REAL | S4 (F-001) | CARGO env or PATH fallback; fallback reproducibly breaks off-cargo |
| workspace_root | 45-50 | REAL | S4 (F-002) | parent().parent() with silent wrong-dir fallback; correct in practice (executed: --check resolved the real root) |
| run | 56-86 | REAL | — | spawn/forward/wait; executed end-to-end exit 0; signal->2 and drift->1 branches read-verified only |

## 5. Test honesty
No tests in this file. Coverage elsewhere: name-in-help only (cli.rs:45).
My execution substitutes for the missing dispatch test (see §2) — but that
evidence lives in this audit, not in the repo's suite (F-003).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "shells out to the crate's schema-gen binary" | header L6-10 | REAL | executed: bridge ran the real binary (target/debug/schema-gen exists; bin target at crates/settings-schema/Cargo.toml:41-43) |
| "reads schemas/settings.toml (+ product.toml for the rule-D1 join)" | header L10-11 | REAL | --check output: "outputs match .../schemas/settings.toml (121 entries)" + "validated against .../product.toml" |
| "(re)generates the three committed outputs" G1/G2/G3 | header L12-16 | REAL | schemas/generated/{settings.d.ts,settings-help.txt,settings-docs.md} exist; --check no-drift |
| "exit codes are 0 ok / 1 drift / 2 error" | header L19-20 | REAL-UNPROVEN | 0 observed twice by execution; 1 (drift) and 2 (error) branches not observed (would require mutating outputs — forbidden; signal->2 read-verified) |
| "Inherited stdio keeps the generator's actionable diagnostics visible" | header L20-22 | REAL | generator usage text appeared unbuffered through the bridge |

## 7. Dead / duplicated / unreachable code
None. All three functions are called (run via registry; cargo_driver and
workspace_root from run). No duplicated blocks.

## 8. Dependencies & flags
std-only (path, process, ExitCode) + crate::XtaskError — consistent with
xtask's zero-external-dep lockfile posture. The bridge itself depends on
the `settings-schema` package having a `schema-gen` bin — verified in its
manifest. No feature gates. WORKSPACE_ROOT via env! macro is compile-time
— correct for a tool that only runs inside its workspace.

## 9. Open questions for the next agent
- The generator's own logic (D1–D6 no-duplicate rules, 14 known areas,
  "11 screens") lives in crates/settings-schema/src/bin/schema_gen.rs —
  not my file; my --check run proves it executes and reports, not that
  its rules are correct.
- schemas/generated/commands.checksum.txt (351 bytes) is a fourth file in
  the generated dir the header does not mention — owner: codegen agent.
- If CI ever invokes the xtask binary directly (not via `cargo xtask`),
  F-001 becomes an S2; check .github/workflows for direct-binary paths.
