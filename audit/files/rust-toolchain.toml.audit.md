# AUDIT: rust-toolchain.toml
| Field | Value |
|---|---|
| Auditor | W01-A07 |
| Date | 2026-11-23 |
| Lines / functions | 11 / 1 section, 3 keys |
| File verdict | REAL (pin honored and cross-consistent; one S3 gap: the wasm32 target the repo's own gates need is not pinned) |
| Reachable from shipped entry point? | N/A (toolchain config). Consumed by rustup for EVERY cargo/rustc invocation in the workspace — verified live: `rustup show active-toolchain` -> "1.99.1-x86_64-unknown-linux-gnu (overridden by '/home/z/my-project/audit-workspace/rust-toolchain.toml')" |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | ~/.cargo/bin/rustup show / show active-toolchain / +1.99.1 target list --installed / +1.99.1 rustc --version (exit 0 each); rg over workflows; baseline citations |

## 1. One-paragraph verdict
A correct, honored, cross-consistent pin: the active toolchain in this workspace is exactly 1.99.1 via this file (verified live), the rustc build hash matches the file's comment verbatim ("1.99.1 (58cf8f9c3 2026-10-13)" — verified with `rustc +1.99.1 --version`), docs/web/size-record.toml's [environment] records the same toolchain, and tauri-launch.yml:105-116 actively asserts pin == CI action == effective rustc in two jobs. The one real gap: the pin guarantees fmt/clippy via `components` but does NOT guarantee the `wasm32-unknown-unknown` target that full_gate step 10 (web-budget) and the CI wasm-build job require — the baseline full_gate run failed on exactly that (E0463), the orchestrator had to install the target by hand, CI papers over it with a manual `rustup target add`, and full_gate.sh has no such step, so every fresh host red-gates the web-budget step for an environment reason.

## 2. What actually works (verified)
— `rustup show active-toolchain` (run from repo root): "1.99.1-x86_64-unknown-linux-gnu (overridden by '/home/z/my-project/audit-workspace/rust-toolchain.toml')" — the pin is live and honored.
— `~/.cargo/bin/rustc +1.99.1 --version` -> "rustc 1.99.1 (58cf8f9c3 2026-10-13)" — byte-identical to the comment at L3 and to size-record.toml's `toolchain = "rustc 1.99.1 (58cf8f9c3 2026-10-13)"` (docs/web/size-record.toml [environment]).
— profile minimal + rustfmt/clippy components: cargo-fmt and cargo-clippy present in ~/.cargo/bin; baseline ran both (cargo-fmt.txt exit 1 = check executed; cargo-clippy.txt CLEAN).
— CI consistency guard: tauri-launch.yml:106-116 pins the dtolnay action to the 1.99.1 branch with a comment "matches rust-toolchain.toml" AND a runtime assertion comparing the file's channel to the installed toolchain.

## 3. Findings
F-001 [S3]
Where: rust-toolchain.toml:5-11 vs scripts/full_gate.sh:70-74, .github/workflows/ci.yml:568-575, baseline full_gate.txt:76-84
Code:
```
# `profile`/`components` guarantee the doc 10 §2 gate tools (fmt, clippy)
# are present on a fresh rustup install of this toolchain.

[toolchain]
channel = "1.99.1"
profile = "minimal"
components = ["rustfmt", "clippy"]
```
What is wrong: the guarantee covers fmt/clippy but not the wasm32-unknown-unknown TARGET that the repo's own gates require. Proof chain: baseline full_gate.txt:76-84 — web-budget step FAILED with `error[E0463]: can't find crate for std` / "the `wasm32-unknown-unknown` target may not be installed" on the first run; rc-smoke.txt:134-146 the same; the rc-smoke addendum (rc-smoke.txt:213+) only passed after "rust-std for toolchain 1.99.1" was installed manually. CI compensates per-job (ci.yml:568-575: `rustup target add wasm32-unknown-unknown` + an installed-list verification), but full_gate.sh has no equivalent step, so a fresh host that follows the toolchain pin still red-gates step 10 for an environment gap that masquerades as a gate failure (see web_budget audit F-001 — the script cannot distinguish the two either). Why it matters: the pin file exists to make a fresh install gate-ready; it is one `targets` entry short of that. The installed rustup here is 1.29.3, which supports a `targets` key in rust-toolchain.toml (support introduced around rustup 1.28 — external knowledge, not re-verified in-sandbox to avoid mutating ~/.rustup). What correct looks like: `targets = ["wasm32-unknown-unknown"]` in [toolchain] (with the rustup-version caveat verified), or a target-install + verify step in full_gate.sh mirroring ci.yml. Confidence: HIGH on the gap (baseline-proven); MEDIUM on the exact fix mechanism (rustup feature claim). Verification: read + live rustup queries + baseline logs.

## 4. Per-key table
| Key | Value | Verdict | Notes |
|---|---|---|---|
| [toolchain].channel | "1.99.1" | REAL | honored (rustup show active-toolchain); matches comment, size-record environment, tauri-launch CI assertion |
| [toolchain].profile | "minimal" | REAL | + components gives fmt/clippy; verified present locally |
| [toolchain].components | ["rustfmt","clippy"] | REAL | guarantee claim TRUE for fmt/clippy; target gap is F-001 |

## 5. Test honesty
No tests in-file. Equivalent verification executed live (§2: rustup/rustc version + override queries) and via CI's own assertion job (tauri-launch.yml:107-116).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Verified on the Phase-1 host (2026-10-19): rustc 1.99.1 (58cf8f9c3 2026-10-13)..." | rust-toolchain.toml:2-4 | REAL | `rustc +1.99.1 --version` returns the identical string |
| "`profile`/`components` guarantee the doc 10 §2 gate tools (fmt, clippy) are present on a fresh rustup install" | rust-toolchain.toml:5-6 | REAL | fmt/clippy verified present; note the guarantee stops short of the wasm32 target (F-001) |
| "(task 1.A1; doc 10 §2.5 — Lead-owned)" | rust-toolchain.toml:1 | REAL-UNPROVEN | governance claim; not verifiable from code (no contradiction found) |

## 7. Dead / duplicated / unreachable code
None. The version "1.99.1" is duplicated by design in: the dtolnay action pin comment (tauri-launch.yml:106, asserted at runtime) and size-record.toml's environment record — both consistent today; the CI assertion catches future drift for tauri-launch, and web-budget's rebuild compares measurements (not the version string) for the record.

## 8. Dependencies & flags
Consumed by rustup (1.29.3 here). No [env] section, no rustflags — none claimed. The pin's effective scope: every cargo/rustc invocation under the repo (verified override active).

## 9. Open questions for the next agent
- Verify on a throwaway machine whether rustup 1.29.3 auto-installs `targets = [...]` from rust-toolchain.toml for an already-installed toolchain (F-001's fix mechanism — I avoided mutating ~/.rustup per the read-only rule).
- full_gate.sh owner should either add the ci.yml-style target-install/verify step or rely on the toolchain file once `targets` lands (cross-ref web_budget audit F-001: the same step should also branch on exit 1 vs 2).
