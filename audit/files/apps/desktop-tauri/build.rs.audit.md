# AUDIT: apps/desktop-tauri/build.rs
| Field | Value |
|---|---|
| Auditor | W14-A10 |
| Date | 2026-11-24 |
| Lines / functions | 12 / 1 (`main`) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | N/A (build script) — runs on every compile of `lumen-desktop-tauri`; that crate currently CANNOT compile (E0425, cross-ref), and the release lane builds only `-p lumen-cli -p desktop` (crate is workspace-excluded) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `sed -n` (all quotes below, exit 0); `cat tauri.conf.json capabilities/default.json Cargo.toml` (exit 0); `rg -n 'frontendDist|TAURI_CONFIG' ~/.cargo/registry/src/*/tauri-build-2.7.4/src/lib.rs` (exit 0); `rg -n 'ui/dist|desktop-tauri' .github/workflows/ci.yml tauri-launch.yml` (exit 0); `ls apps/desktop-tauri` → no `gen/` dir (build script never ran here) |

## 1. One-paragraph verdict
Stock tauri v2 build script — `fn main() { tauri_build::build(); }` — the correct form for this crate's layout, and I verified the layout claims against the vendored tauri-build 2.7.4 source. It was never executed in this sandbox (baseline build dies at gdk-sys first: audit/runs/baseline/cargo-build-desktop-tauri.txt:310, and no `gen/schemas` exists in the crate), so its behavior is verified by source-reading only. The two defects are prose: the doc comment credits `tauri_build::build()` with embedding work that actually belongs to tauri-codegen, and the "UI first" ordering it relies on is enforced by nothing in the crate itself.

## 2. What actually works (verified)
— `tauri.conf.json` + `capabilities/` sit at the crate root and default discovery finds them: verified by reading tauri-build 2.7.4 (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-build-2.7.4/src/lib.rs:539-541`: "This defaults to a file called `tauri.conf.json` inside of the current working directory"). `generate_context!()` really is in src/app.rs:321. `frontendDist` is really `"../../ui/dist"` (tauri.conf.json). Every CI lane that compiles this crate builds the UI first — verified at .github/workflows/ci.yml:719-724, tauri-launch.yml:144-149, 333-338, 478-483, and apps/desktop-tauri/INFRA-BLOCKED.md:98-100. Nothing executed (cargo build forbidden: gdk-sys env limit + E0425 makes it moot).

## 3. Findings
F-001 [S4]
Where: apps/desktop-tauri/build.rs:3-8
Code:
```
//! `tauri_build::build()` embeds both into the context
//! `tauri::generate_context!()` (src/app.rs) reads at compile time —
//! including the ui/dist assets via `build.frontendDist`, which is why
//! every lane that compiles this crate builds the UI first.
```
What is wrong: tauri-build 2.7.4's own docs (src/lib.rs:7-16) say build() "resolves the capabilities ... writing the resulting Access Control List to the `OUT_DIR` so `tauri::generate_context!` can embed it" — it writes to OUT_DIR; generate_context! embeds. And `rg frontendDist` over tauri-build's source has ZERO hits: build() never touches ui/dist; the frontendDist asset embedding is done by tauri-codegen at macro-expansion time, which is exactly what ci.yml:677-678 says ("The lane needs the UI built because tauri-codegen embeds ui/dist (frontendDist) at compile time"). The build.rs comment credits the wrong component for the one behavior (asset embedding) that forces the UI-first ordering. Why it matters: a reader of this file gets a wrong mental model of why a missing ui/dist breaks the build late (at macro expansion in app.rs, not in this script). What correct looks like: "`tauri_build::build()` validates the config, resolves capabilities to OUT_DIR, and emits rerun-if-changed; `generate_context!()` (tauri-codegen) embeds config + capabilities + ui/dist". Confidence: HIGH — Verification: read (vendored tauri-build 2.7.4 source + ci.yml).

F-002 [S4]
Where: apps/desktop-tauri/tauri.conf.json:5-7 (consumed by this build script)
Code:
```
  "build": {
    "frontendDist": "../../ui/dist"
  },
```
What is wrong: no `beforeBuildCommand`/`beforeDevCommand` is set, and `ui/dist` does not exist in the working tree (verified: `ls /home/z/my-project/audit-workspace/ui/dist` → no such dir). The UI-before-cargo ordering exists only as out-of-band discipline in three workflow files and INFRA-BLOCKED.md. Anyone using the standard tauri flow (`cargo tauri build`/`dev`) or a bare `cargo build --manifest-path apps/desktop-tauri/Cargo.toml` on a GTK-equipped host compiles the entire tauri/GTK dependency tree and then dies at `generate_context!` with a missing-frontendDist error. Why it matters: the crate advertises "the tauri v2 standard form" but does not use tauri's own mechanism that would make that flow self-sufficient. What correct looks like: `"beforeBuildCommand": "npm --prefix ui run build"` (or a build.rs pre-check failing fast with a pointed message). Blast radius: developer ergonomics only — CI lanes are correctly ordered. Confidence: HIGH — Verification: read (tauri.conf.json + workflows + ls).

## 4. Per-function table (file < 100 LOC — informational)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `main` | 10-12 | REAL-UNPROVEN | — | Stock `tauri_build::build()`; panics/exits(1) on config error (verified in vendored source: `pub fn build()` prints error and `std::process::exit(1)`). Never ran here. |

## 5. Test honesty
No tests. The crate's `tests/single_command.rs` is a source-scanning test that does not execute the build script.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "tauri.conf.json and capabilities/ live at the crate root ... default discovery finds them" | build.rs:3-5 | REAL | tauri-build lib.rs:539-541 default path; both files exist (ls) |
| "tauri_build::build() embeds both into the context ... including the ui/dist assets" | build.rs:5-8 | FALSE (mechanism) | tauri-build source has no frontendDist handling; embedding is tauri-codegen (ci.yml:677-678) — F-001 |
| "every lane that compiles this crate builds the UI first" | build.rs:7-8 | REAL | ci.yml:719-724, tauri-launch.yml:144-149/333-338/478-483, INFRA-BLOCKED.md:98-100 |
| "a build-time file processor, links nothing extra" | Cargo.toml:86-88 | REAL-UNPROVEN | Matches tauri-build docs (config/ACL processing, cfg aliases, rerun-if-changed); not executed here |

## 7. Dead / duplicated / unreachable code
None in this file. Note: no `gen/schemas/` directory exists in the crate — tauri-build (which generates it) has never run to completion in this tree, consistent with the gdk-sys failure preceding the crate's own build script.

## 8. Dependencies & flags
`tauri-build = "2"` (Cargo.toml:64-68); Cargo.lock pins tauri-build 2.7.4 (baseline build log line 181). No features, no Attributes — defaults are correct for icons (icons/icon.png + icon.ico both exist) and the crate-root config.

## 9. Open questions for the next agent
- W14-A02 (INFRA-BLOCKED.md): does the doc's §B.1 build recipe include the UI step before cargo? (Verified lines 98-100 do — but check the whole file for a bare `cargo build` recipe that would hit F-002.)
- Whoever audits ci.yml/tauri-launch.yml: the tauri-check lane (ci.yml:731 `cargo test --manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets`) cannot pass against the current tree — E0425 at app.rs:301 (`spawn_session_pump` undefined). When did tauri-check last go green?
