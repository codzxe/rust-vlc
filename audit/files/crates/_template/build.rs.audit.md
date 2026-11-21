# AUDIT: crates/_template/build.rs
| Field | Value |
|---|---|
| Auditor | W01-A03 |
| Date | 2026-11-21 |
| Lines / functions | 9 / 1 |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (build script). Executed by every workspace build: baseline run #1 `cargo build --release --locked --workspace` exit 0 (INDEX.md #1); package `template` confirmed a member via Cargo.lock:1109. NOT reachable from `lumen`/`lumen-tauri` binaries — no crate depends on `template` (grep `^\s*template\s*=` across all Cargo.toml: 0 hits; publish=false) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `sed -n '1,9p' crates/_template/build.rs` (quote verify) · Grep `lint_canary` repo-wide (refs: lib.rs:18, lint_canary.rs, root Cargo.toml:33, worklog) · Grep `lint_canary|canary` in .github/, scripts/, tools/ (0 enforcement hits) · Grep `^\s*template\s*=` in **/Cargo.toml (no dependents) · `grep -n -A2 '^name = "template"' Cargo.lock` (membership) |

## 1. One-paragraph verdict
A 9-line build script whose sole statement registers the `lint_canary` cfg name
so the cfg-gated canary module compiles warning-free under `-D warnings`. The
body matches its comment exactly — no hidden native code, no file I/O, no env
sniffing. The registration demonstrably works: `src/lib.rs:18` uses
`#[cfg(lint_canary)]` and baseline clippy run #5 (`cargo clippy --workspace
--all-targets --locked -- -D warnings`) exited 0 — on rustc 1.99.1 an
unregistered cfg would fire `unexpected_cfgs` and fail that run. The real
defect is not in this file but in the mechanism it serves: the canary proof it
enables is enforced by nothing — no CI workflow, script, or xtask command ever
passes `--cfg lint_canary`.

## 2. What actually works (verified)
— cfg registration effective: baseline `cargo clippy --workspace --all-targets
--locked -- -D warnings` → exit 0 (INDEX.md #5) with `#[cfg(lint_canary)]`
present at lib.rs:18; without this registration rustc 1.80+ raises
`unexpected_cfgs` → run would fail.
— script executes in workspace builds: baseline run #1 exit 0; `template`
present in Cargo.lock:1109 (`name = "template"`, version 0.1.0).

## 3. Findings
### F-001 [S3]
Where: crates/_template/build.rs:1-9 (mechanism), root Cargo.toml:33 (claim), crates/_template/src/lint_canary.rs:13-16 (proof commands)
Code:
```rust
// Template-only build script (task 1.A1): registers the `lint_canary` cfg
// name so rustc's `unexpected_cfgs` lint does not warn about the cfg-gated
// canary module in `src/lint_canary.rs`.
```
What is wrong: This script exists solely to enable the lint-inheritance
canary, and root Cargo.toml:33 states "Inheritance is proven by the canary in
`crates/_template/src/lint_canary.rs`" — but the proof commands
(`cargo clippy -p template --all-targets -- --cfg lint_canary`) are executed by
no automated gate: Grep for `lint_canary` in .github/workflows/, scripts/,
tools/ → zero hits. The only execution record is a manual run in
docs/worklog/archive-2026-10.md:672-673 (2026-10, pre-baseline).
Why it matters: The canary is the repo's claimed proof that
`[lints] workspace = true` actually propagates the deny tables. If someone
weakens or removes `[workspace.lints]`, `clippy -D warnings` still passes
(fewer lints = fewer errors) and nothing detects the drift — the "proof"
becomes permanently stale evidence.
What correct looks like: A CI job or xtask subcommand that runs both proof
commands and asserts exit 101 with the two documented errors, on every PR
touching root Cargo.toml or any Cargo.toml's `[lints]` section.
Confidence: HIGH — Verification: read + repo-wide grep (no enforcement site
exists).

### F-002 [S4]
Where: crates/_template/build.rs:7-9
Code:
```rust
fn main() {
    println!("cargo::rustc-check-cfg=cfg(lint_canary)");
}
```
What is wrong: No `cargo::rerun-if-changed` directive, so cargo re-runs the
script whenever any file in the `template` package changes, though its output
is constant.
Why it matters: Wasted rebuild steps; contradicts the file's own framing of
build scripts as "audit suspects" — a fixed-output script should pin itself
with `println!("cargo::rerun-if-changed=build.rs")`.
What correct looks like: Emit `cargo::rerun-if-changed=build.rs` (or
`rerun-if-changed=src/lint_canary.rs`) as the second line of main.
Confidence: HIGH — Verification: read (file is 9 lines; no other println).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `main` | 7-9 | REAL | S4 | Single `println!` directive; correct syntax for the modern `cargo::` key form (needs cargo ≥1.77; toolchain pin 1.99.1 satisfies); no unsafe, no fs/env access; runs in every workspace build (baseline #1). No rerun-if pin (F-002). |

## 5. Test honesty
No tests in this file. Related test `crates/_template/tests/smoke.rs:5-8`
(`template_marker_is_stable`) does not exercise build.rs; it would still pass
if the cfg registration were deleted only when clippy's `-D warnings` gate is
skipped — i.e., the script's correctness is guarded by gate #5, not by tests.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "This is the ONLY reason a build script exists here" | build.rs:3-4 | REAL | Body is a single println registering exactly one cfg (sed -n '7,9p') |
| "Inheritance is proven by the canary" | root Cargo.toml:33 | REAL-UNPROVEN (stale) | One-time manual run in worklog archive-2026-10.md:672-673; no gate re-runs it (grep: 0 enforcement sites) — see F-001 |
| "build scripts are native-code audit suspects per doc 10 §2.3" | build.rs:5-6 | REAL (for this instance) | No native code, no cc/rsbuild, no env/fs access — verified by reading all 9 lines |

## 7. Dead / duplicated / unreachable code
None. The script is live (member of every workspace build). The crate it
belongs to is intentionally unreachable from shipped binaries (no dependents;
publish=false) — by design, documented at root Cargo.toml:6-12.

## 8. Dependencies & flags
No build-dependencies (`[build-dependencies]` absent from
crates/_template/Cargo.toml — verified by reading the 23-line manifest).
Registers cfg `lint_canary`; no features declared anywhere in the template
manifest. Uses `cargo::` directive syntax — requires the 1.99.1 toolchain pin
(rust-toolchain.toml) or newer; consistent.

## 9. Open questions for the next agent
- For the agent auditing `crates/_template/src/lint_canary.rs` (wave-00 A01):
  F-001's blast radius starts there — the canary's two sanctioned violations
  are the only "consumers" of this build script.
- Does any doc besides the worklog record the canary proof output verbatim?
  If docs/quality/ or doc 10 cite it, they inherit the staleness of F-001.
