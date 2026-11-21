# AUDIT: apps/desktop-tauri/build.rs
| Field | Value |
|---|---|
| Auditor | W14-A04 |
| Date | 2026-11-21 |
| Lines / functions | 12 / 1 (`main`) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | YES — build-time only. Cargo auto-runs it on every compile of package `lumen-desktop-tauri` (bin `lumen-tauri`, Cargo.toml:30-32): Cargo.toml:68 `[build-dependencies] tauri-build = "2"` → build.rs:11 `tauri_build::build()` → OUT_DIR/ACL artifacts → `tauri::generate_context!()` at src/app.rs:321 ← src/main.rs:50 `lumen_desktop_tauri::app::run()`. NO runtime path: nothing in this file can cause or cure the GUI file-open hang under investigation (that is app.rs/W14-A01 territory); its only influence on the symptom is *what UI bytes get baked in at compile time*. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read build.rs + `sed -n '1,13p'` (verbatim verify); LS apps/desktop-tauri (tauri.conf.json, capabilities/, icons/, src/, Cargo.lock present; NO gen/ — gitignored, .gitignore:118); Read Cargo.toml (build-dep :64-68), tauri.conf.json (frontendDist `../../ui/dist` :6-8, no beforeBuildCommand/devUrl), capabilities/default.json (core:default + 4 window grants); `rg -n generate_context` src/ → app.rs:321 only; `rg -n "app::run\|fn main"` main.rs → :45,:50; `ls ui/dist` → ABSENT in tree (gitignored: root .gitignore `ui/dist/`, ui/.gitignore `dist/`); `rg -n` release.yml/tauri-launch.yml/ci.yml for UI-build + cargo-build steps → 9 compile sites, all UI-first (ci.yml:719-731; release.yml:194-219, 475-499, 628-652, 779-803, 927-954; tauri-launch.yml:144-156, 333-345, 478-490); `rg -n desktop-tauri` over all other workflows + scripts/ → packaging scripts stage prebuilt binary only (stage-linux-payload.sh:61-63), no compile site; `rg tauri_build` repo-wide → build.rs:5,11 only; `rg` Cargo.lock → tauri 2.12.5 (:3688), tauri-build 2.7.4 (:3739), tauri-codegen 2.7.4 (:3762), tauri-macros 2.7.4 (:3789), tauri-utils 2.10.2 (:3853), `sed -n '3739,3790p'` (dep sets); Read baseline build-desktop-tauri.log.txt (exit 101 at gdk-3.0.pc); Read INFRA-BLOCKED.md, docs/dependencies/records/tauri-wry-2.x.md, docs/packaging/evidence/w26m-t4-release-tauri.md (static-only); `ls` docs/gates/logs + docs/packaging/evidence (no tauri-check run log committed); `rg -n "OUT_DIR\|include!"` src/+tests/ → 0 matches; cross-ref audit/files/apps/desktop-tauri/Cargo.toml.audit.md (W01-A02) |

## 1. One-paragraph verdict
The file is the upstream-standard tauri v2 build script — a single delegation call, no custom logic — and its wiring is real: tauri.conf.json and capabilities/ sit at the crate root where default discovery expects them, the build-dependency is declared and pinned by the crate's own lockfile (tauri-build 2.7.4, inside the allowlist §3a range per W01-A02), and `tauri::generate_context!()` consumes the pipeline output at app.rs:321. Its one load-bearing claim — "every lane that compiles this crate builds the UI first" — is verified at all nine compile sites across ci.yml/release.yml/tauri-launch.yml. But nothing this script does has ever executed in committed evidence: the only baseline attempt died at `gdk-3.0.pc` before the package's build-script stage, and the CI runs that allegedly compiled it are referenced by run ID only. The doc comment also mis-attributes the frontendDist embedding to the build script where the repo's own CI comments attribute it to tauri-codegen (macro side). REAL-UNPROVEN: genuine, correctly wired, never executed in anything citable.

## 2. What actually works (verified)
— Layout claim holds: `ls apps/desktop-tauri` → tauri.conf.json + capabilities/default.json at crate root; `gen/` absent and gitignored (.gitignore:118), consistent with tauri-build generating ACL schemas at build time (capabilities/default.json `$schema` points at `../gen/schemas/desktop-schema.json`).
— Compile-time chain is real: src/main.rs:50 → `lumen_desktop_tauri::app::run()` → src/app.rs:321 `.run(tauri::generate_context!())`; tauri.conf.json `frontendDist: "../../ui/dist"` (tauri.conf.json:6-8) matches the comment's embed source.
— "Every lane builds the UI first" — REAL, exhaustively: ci.yml tauri-check builds UI (:719-724, with `test -f ui/dist/index.html` guard) before `cargo test --manifest-path` (:731); release.yml's five lanes (194-219, 475-499, 628-652, 779-803, 927-954) each run UI build → guard → `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml`; tauri-launch.yml's three lanes (144-156, 333-345, 478-490) same order. Repo-wide search found no other compile site (packaging scripts stage the prebuilt binary; stage-linux-payload.sh:61-63 refuses loudly if absent).
— Uniqueness claim (Cargo.toml:66-67 "This crate is the only place it runs") holds: `rg tauri_build` → build.rs:5,11 only.
— Version coherence: tauri-build 2.7.4 = tauri-codegen 2.7.4 = tauri-macros 2.7.4, one shared codegen version, no duplicates (Cargo.lock:3739-3790); tauri 2.12.5 + tauri-utils 2.10.2 in the same lock; workspace graph remains tauri-free (W01-A02 verified the root lock has zero tauri entries).

## 3. Findings

### F-001 [S3] The build script has never executed in any committed evidence — its success is asserted, never captured
Where: apps/desktop-tauri/build.rs:10-12 (entire behavior) + audit/runs/baseline/build-desktop-tauri.log.txt:3-9
Code:
```
fn main() {
    tauri_build::build();
}
```
and baseline:
```
  The file `gdk-3.0.pc` needs to be installed and the PKG_CONFIG_PATH environment variable must contain its parent directory.
  The PKG_CONFIG_PATH environment variable is not set.
```
```
warning: build failed, waiting for other jobs to finish...
EXIT=101
```
What is wrong: the only in-repo execution attempt (baseline run #2, `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml`) failed inside a *dependency* sys-crate's pkg-config probe (gdk-3.0.pc). Cargo aborts the graph before compiling `lumen-desktop-tauri` itself, so the package build script — this file — never ran in any citable artifact. INFRA-BLOCKED.md:4-5 claims "the crate compiles in its dedicated CI lane (`tauri-check`, Linux + webkit2gtk)" citing live Actions runs (ci.yml:752,:759 reference runs 43453921560 / 43455796103), but no CI log is committed anywhere (docs/gates/logs/ has phase-exit logs only; docs/packaging/evidence/w26m-t4-release-tauri.md is explicitly "static-only … NOT run").
Why it matters: the wave's core question — does the GUI shell build, and what does it embed — rests on a pipeline step whose success is claimed but never captured. A tauri-build/config schema mismatch (tauri 2.12.5 vs tauri-build 2.7.4 vs tauri-utils 2.10.2), a capabilities/ACL codegen rejection, or a frontendDist failure would surface exactly at this script and nowhere in the committed evidence. Every downstream "the GUI compiles" claim inherits this hole (same gap already flagged by W01-A02 for the manifest).
What correct looks like: a committed tauri-check lane log (the repo's own docs/gates/logs pattern) showing `Compiling lumen-desktop-tauri v0.1.3` and the build script completing — or an orchestrator run on a webkit-equipped host.
Confidence: HIGH — Verification: read (searched audit/runs/baseline/, docs/gates/logs/, docs/packaging/evidence/; nothing executes this script).

### F-002 [S4] Doc comment mis-attributes the frontendDist embedding to the build script; the repo's own CI comments attribute it to tauri-codegen (macro side) — one of the two is wrong
Where: apps/desktop-tauri/build.rs:4-8
Code:
```
//! `tauri_build::build()` embeds both into the context
//! `tauri::generate_context!()` (src/app.rs) reads at compile time —
//! including the ui/dist assets via `build.frontendDist`, which is why
//! every lane that compiles this crate builds the UI first.
```
Contradicting in-repo sources:
```
  # green first). The lane needs the UI built because tauri-codegen embeds
  # ui/dist (frontendDist) at compile time.
```
(.github/workflows/ci.yml:678-679) and
```
          # ORDER IS LOAD-BEARING (W26-M): tauri.conf.json frontendDist
          # is ../../ui/dist and tauri-codegen embeds it AT COMPILE
          # TIME — a missing/stale ui/dist would bake an empty or
```
(.github/workflows/release.yml:196-198).
What is wrong: build.rs claims `tauri_build::build()` embeds tauri.conf.json + capabilities + the ui/dist assets; ci.yml and release.yml consistently say tauri-codegen (i.e., the `generate_context!` proc-macro expansion at app.rs:321) does the embedding. The dependency sets support the CI reading: tauri-build 2.7.4's closure is config/ACL machinery (schemars, json-patch, cargo_toml, glob, walkdir — Cargo.lock:3743-3759), while the asset-embedding dep set (ico, png, brotli, base64 — Cargo.lock:3766-3786) hangs off tauri-codegen. The doc comment is the file's entire informational content; its central attribution is contested inside the same repo.
Why it matters: a maintainer debugging "the shipped GUI shows the wrong UI" who trusts build.rs's comment will chase the build-script stage (rerun rules, OUT_DIR) when the embed actually happens at macro expansion — and vice versa. Doc precision on the one file whose only job is to explain this pipeline.
What correct looks like: "tauri_build::build() validates tauri.conf.json and compiles the capabilities/ ACL into OUT_DIR; `tauri::generate_context!()` (src/app.rs:321) then embeds the config, capabilities, and the ui/dist assets (build.frontendDist) at macro-expansion time — which is why every lane builds the UI first."
Confidence: MEDIUM — Verification: read (in-repo contradiction is HIGH; which side is strictly correct is UNVERIFIED — tauri-build 2.7.4 source not inspectable: ~/.cargo/registry absent in sandbox, no network, no build possible).

### F-003 [S4] Change-tracking (cargo:rerun-if-changed) is 100% delegated to tauri-build with zero in-repo assertion of what it covers
Where: apps/desktop-tauri/build.rs:10-12
Code:
```
fn main() {
    tauri_build::build();
}
```
What is wrong: the file (and the crate's docs) state nothing about which inputs trigger re-codegen. Whether editing tauri.conf.json or capabilities/default.json re-runs the pipeline depends entirely on tauri-build 2.7.4 internals that neither this repo nor this audit can verify (registry source absent, no build possible). If the directives were absent or narrow, an incremental rebuild could serve a stale embedded config — including a stale capabilities/ACL, i.e., the IPC security boundary (capabilities/default.json is the permission set A08 audits).
Why it matters: the classic silent-stale-embed hazard lands on a security-relevant input; nobody reading this crate can tell whether config edits are guaranteed to re-embed.
What correct looks like: at minimum a comment stating the expected rerun-if set (tauri.conf.json, capabilities/**), or a committed lane proving a config edit re-embeds. Note: the form matches the upstream create-tauri-app v2 template exactly, so this is upstream-standard posture — the finding is the undocumented reliance, not a deviation.
Confidence: MEDIUM — Verification: UNVERIFIED (tauri-build internals not inspectable in this sandbox; the hazard is conditional on upstream behavior I cannot trace).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `main` | 10-12 | REAL-UNPROVEN | S3 | Single delegation to `tauri_build::build()` (tauri-build 2.7.4, Cargo.lock:3739-3741, registry checksum `adddb0e6…`); no directives, no inputs, no error handling of its own — all behavior owned by the build-dep. Runs before every compile of the crate (cargo auto-discovery). Never executed in committed evidence (F-001). No ignored params, no unsafe, no panics of its own (tauri-build panics on invalid config — unverifiable here). |

## 5. Test honesty
No tests exist in or for this file (build scripts are not unit-testable in-tree). Notably, no test anywhere asserts the embedded context matches tauri.conf.json/capabilities on disk (a `#[test]` comparing the runtime `AppHandle` config against the JSON would catch stale-embed, F-003); the crate's only test file (tests/single_command.rs) targets runtime command registration, not the build pipeline. Nothing to mutate-test.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "tauri.conf.json and capabilities/ live at the crate root (the conventional layout), so default discovery finds them" | build.rs:3-4 | REAL | ls: both exist at apps/desktop-tauri/; gen/ (ACL schemas) gitignored .gitignore:118, generated by tauri-build at build time |
| "`tauri_build::build()` embeds both into the context `generate_context!()` reads … including the ui/dist assets via `build.frontendDist`" | build.rs:4-8 | REAL-UNPROVEN (attribution imprecise, F-002) | Pipeline endpoints verified (frontendDist `../../ui/dist` tauri.conf.json:6-8; macro at app.rs:321); WHO embeds is contested by ci.yml:678-679 / release.yml:196-198; never executed in evidence (F-001) |
| "every lane that compiles this crate builds the UI first" | build.rs:7-8 | REAL | All 9 compile sites verified UI-first: ci.yml:719-731; release.yml:194-219, 475-499, 628-652, 779-803, 927-954; tauri-launch.yml:144-156, 333-345, 478-490; exhaustive repo search found no other compile site |
| "Standard tauri v2 build script … This crate is the only place it runs" | Cargo.toml:64-68 | REAL | `rg tauri_build` → build.rs:5,11 only |
| "the crate compiles in its dedicated CI lane (`tauri-check`, Linux + webkit2gtk)" | INFRA-BLOCKED.md:4-5 | REAL-UNPROVEN | Lane exists (ci.yml:680-775); zero committed run logs; referenced run IDs (ci.yml:752,:759) not archived in-repo |
| "tauri-codegen embeds ui/dist at compile time … missing/stale ui/dist would bake an empty or stale frontend" | release.yml:196-199, ci.yml:678-679 | REAL-UNPROVEN | Consistent with frontendDist config; the "bake an empty frontend" failure mode (vs a loud codegen error on missing dir) is UNVERIFIED — see REQUEST-ORCH-RUN |

## 7. Dead / duplicated / unreachable code
None. The file is minimal and required: deleting it drops the ACL/capability codegen that `generate_context!` (app.rs:321) depends on — the upstream-required pairing. No duplication (only build script in the repo).

## 8. Dependencies & flags
- Build-dep: `tauri-build = "2"` (Cargo.toml:68) → resolved 2.7.4 (Cargo.lock:3739), inside allowlist §3a range `>=2.7.4,<2.8.0` (per W01-A02's Cargo.toml audit). One shared tauri-codegen 2.7.4 for both tauri-build and tauri-macros — no version skew in the codegen pair.
- Same lock: tauri 2.12.5, tauri-utils 2.10.2, wry 0.57.2, tao 0.37.2 (Cargo.lock:3631,:3688,:3853,:4961).
- Workspace-excluded crate (ADR-004): root `--workspace` lanes structurally never run this script — the standing UNVERIFIED-BUILD is deliberate posture, not an accident (root lock verifiably tauri-free, W01-A02).
- No feature flags in this file; no unsafe; crate lints (Cargo.toml:34-47) don't apply to build scripts' own behavior.

## 9. Open questions for the next agent
- W14-A01 (app.rs): does anything at runtime re-read tauri.conf.json or capabilities from disk, or is the compiled-in context the only source of truth? If the former, F-002's mis-attribution gains runtime weight.
- W14-A07 (tauri.conf.json): frontendDist carries NO `beforeBuildCommand`/`devUrl` — a bare local `cargo build` in this crate with ui/dist absent (fresh checkout: ui/dist is gitignored) hits the unresolved failure mode below.
- W14-A08 (capabilities): if the ACL embed can go stale (F-003), the shipped IPC permission set drifts from capabilities/default.json — a lane diffing embedded ACL vs the JSON would close it.
- REQUEST-ORCH-RUN (needs a webkit2gtk-equipped host — sandbox cannot, per wave prompt): (a) `npm --prefix ui ci && npm --prefix ui run build && cargo build --manifest-path apps/desktop-tauri/Cargo.toml --locked` — capture the build script actually executing; (b) negative control: same cargo build with ui/dist ABSENT — settle whether a missing frontendDist fails loudly or "bakes an empty frontend" (release.yml:198-199's claim); the answer decides whether the UI-first ordering is correctness-critical or hygiene.
