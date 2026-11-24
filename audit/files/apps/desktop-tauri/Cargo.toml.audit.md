# AUDIT: apps/desktop-tauri/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W14-A06 |
| Date | 2026-11-24 |
| Lines / functions | 68 / 21 config keys across 6 tables ([package], [[bin]], [lints.rust] x4, [lints.clippy] x7, [dependencies] x4, [build-dependencies] x1) |
| File verdict | REAL-UNPROVEN (lock-consistent, CI-consumed manifest; in-sandbox compile blocked by gdk-sys — baseline env fact, not a repo defect) |
| Reachable from shipped entry point? | YES — it IS a shipped entry point: `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml` (release.yml:219, tauri-launch.yml:156) → bin `lumen-tauri` (L30-32) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 5 |
| Commands run | sed/rg (all quotes, exit 0); `cargo tree --manifest-path apps/desktop-tauri/Cargo.toml --locked -i tray-icon` (exit 0, "warning: nothing to print"); `cargo tree --target all` (FAILED — disk full at windows-0.62.2 unpack; 145 .crate cache files + partial extraction deleted, 63MB restored); python3 reverse-dep scan of Cargo.lock (exit 0); `cargo metadata` scratch reproductions x2 (exit 101 both — workspace walk-up mechanics); `diff` of lints blocks vs root (identical, exit 0) |

## 1. One-paragraph verdict
This manifest is honest about what it is: a workspace-EXCLUDED ADR-004 admission crate with its own lockfile, an inline lint mirror (verified byte-equivalent to the root `[workspace.lints]`), and four deps (desktop, commands, tauri, serde) plus tauri-build. The lockfile is consistent with it (`cargo tree --locked` resolved, exit 0) and pins tauri 2.12.5 / wry 0.57.2 / tao 0.37.2 / tauri-build 2.7.4. The defects are governance sloppiness, not fraud: the manifest encodes `tauri = "2"` — a range ~100x wider than the admitted `>=2.12.5, <2.13.0` — while its comment claims that "2" is "per the record's admission range" (it is not); the header edge list mislabels commands as L3 (boundaries.toml says layer = 1) and omits 2 of the 4 declared deps; the ADR-004 record still carries wry 0.57.0 while the lock ships 0.57.2; the excluded crate escapes every fmt gate; and no [profile] section tunes the shipped release binary.

## 2. What actually works (verified)
- Lints mirror is byte-identical to the root policy — verified by `diff <(sed -n '34,47p' apps/desktop-tauri/Cargo.toml | sed 's/^\[lints/[workspace.lints/') <(sed -n '53,66p' Cargo.toml)` -> no output (identical), exit 0.
- Lockfile/manifest consistency + tray-icon feature state — `cargo tree --manifest-path apps/desktop-tauri/Cargo.toml --locked -i tray-icon` -> exit 0, "warning: nothing to print." (tray-icon NOT in the host graph: defaults don't enable it; the lock's tray-icon/libappindicator/libloading entries are feature-agnostic lock entries, matching deny.toml's "NOT enabled" claim). Registry check: tauri-2.12.5 default = `["wry","compression","common-controls-v6","dynamic-acl","x11","dbus"]` — no tray-icon.
- Workspace-walk-up mechanics behind the exclusion design — reproduced in audit/scratch/W14-A06: a non-member/non-excluded package errors "current package believes it's in a workspace when it's not ... add the package to the `workspace.exclude` array, or add an empty `[workspace]` table" (exit 101). Root does exclude this crate (root Cargo.toml:51); no Cargo.toml exists above the repo root (ls verified) — so the crate is standalone and `[lints] workspace = true` could not resolve; the inline mirror is the correct workaround. (Exact "error inheriting `lints`" text NOT reproduced — no workspace-free location exists under audit/; UNVERIFIED, mechanism verified.)
- "tauri-codegen embeds frontendDist at compile time" (comment L5-6) — verified in registry tauri-codegen-2.7.4/src/context.rs:182-194: `FrontendDist::Directory(path)` -> `EmbeddedAssets`, error "...path doesn't exist".
- serde dep is real, not dead: src/app.rs:118-119 `#[derive(Debug, serde::Serialize)]` / `#[serde(tag = "error", rename_all = "kebab-case")]`.
- Version/identity consistency: package 0.1.3 == tauri.conf.json `"version": "0.1.3"` == path dep desktop 0.1.3 (lock L653-654); commands 0.1.0 == crates/commands/Cargo.toml:16.

## 3. Findings

F-001 [S3]
Where: apps/desktop-tauri/Cargo.toml:57-60, 68
Code:
```
# ADR-004 admission (records/tauri-wry-2.x.md): the OS webview. Version
# "2" per the record's admission range; wry + tao ride its own graph
# in THIS crate's lockfile.
tauri = "2"
```
(and L68 `tauri-build = "2"`)
What is wrong: The admitted ranges are pinned in allowlist.md §3a — `| tauri | 2.12.5 | >=2.12.5, <2.13.0 | ... |` and `| tauri-build | 2.7.4 | >=2.7.4, <2.8.0 | ... |` (docs/dependencies/allowlist.md:125-126). The manifest's `"2"` means `>=2.0.0, <3.0.0` — it admits an unaudited 2.0.0 or a future 2.13.x today. The comment's claim that "2" is "per the record's admission range" is false: the admission range is 100x narrower. No machine gate enforces the range: deny.toml carries no version spec for tauri (only the libloading wrapper scope, license classes, RUSTSEC ignores — deny.toml:60-240), and no xtask command validates the subcrate lock against allowlist §3a (rg over tools/xtask/src: only rebrand/check_identity/lint_literals touch allowlist, for other files). The only guards are the committed lockfile + `--locked` CI builds + the human "same-PR rule" for Cargo.lock deltas.
Why it matters: A `cargo update` inside apps/desktop-tauri silently satisfies this manifest while leaving the audited admission range — exactly the drift ADR-004's revisit trigger ("tauri/wry version drift at admission time re-runs the pre-audit", docs/adr/ADR-004-os-webview.md:126-127) exists to catch, caught only by PR review.
What correct looks like: `tauri = ">=2.12.5, <2.13.0"` and `tauri-build = ">=2.7.4, <2.8.0"` (or a deny.toml PackageSpec pin), making the manifest state the admitted range it claims to state.
Confidence: HIGH — Verification: ran (lock/allowlist/deny.toml/xtask greps + cargo tree --locked exit 0).

F-002 [S4]
Where: apps/desktop-tauri/Cargo.toml:13-18
Code:
```
# Edges (this crate only — boundaries.toml governs workspace members;
# this one is excluded, so its edges are declared here):
#   desktop  (path, L6) — the shell: ShellRuntime, files, window config
#   commands (path, L3) — the shared wire envelope (WireRequest etc.);
```
What is wrong: commands is L1, not L3. boundaries.toml:133-134 (the declared "single source of truth") reads `[crates.commands]` / `layer = 1`, under the header "# L1 — the one command layer (doc 01 §2.3...)" (L129). desktop (path, L6) is correct ([crates.desktop] layer = 6, boundaries.toml:538-539).
Why it matters: This header is the ONLY edge documentation for this crate — boundaries.toml has no desktop-tauri entry (rg: zero matches) because it is excluded. The substitute manifest misstates the one architectural fact it exists to record.
What correct looks like: "commands (path, L1)".
Confidence: HIGH — Verification: read (sed-verified both files).

F-003 [S4]
Where: apps/desktop-tauri/Cargo.toml:13-18 vs 62, 68
Code:
```
# Edges (this crate only — boundaries.toml governs workspace members;
# this one is excluded, so its edges are declared here):
```
vs L62 `serde = { version = "1", features = ["derive"] }` and L68 `tauri-build = "2"`.
What is wrong: The self-declared complete edge list names only desktop, commands, tauri — it omits serde (a normal dep) and tauri-build (a build dep). Both are documented inline further down, but the list that claims to be the crate's edge inventory under-reports by half of its non-shell deps.
Why it matters: For the one crate where boundaries.toml's allowlist does not apply, the header is the audit surface; an incomplete inventory defeats its purpose.
What correct looks like: add `serde` and `tauri-build` rows to the Edges comment.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: .github/workflows/ci.yml:155-156, 680-776; scripts/full_gate.sh:11 — touching this crate's lint-mirror claim (Cargo.toml:20-21)
Code (ci.yml:156):
```
        run: cargo fmt --all -- --check
```
What is wrong: `--all` covers workspace members only; apps/desktop-tauri is excluded (root Cargo.toml:51). The tauri-check lane (ci.yml:680-776) runs test + clippy + cargo-deny + an ldd smoke — no fmt step. full_gate.sh:11 uses the same workspace-wide `cargo fmt --all --check`. No lane anywhere formats or fmt-checks this crate.
Why it matters: The manifest mirrors the rust+clippy lint policy inline but the fmt policy silently does not reach the crate — the exclusion that motivated the lints mirror (L20-21) equally excludes fmt, and nobody mirrored or re-gated it. (Repo-wide fmt is already failing per baseline; this crate is not even in the failing set.)
What correct looks like: a `cargo fmt --manifest-path apps/desktop-tauri/Cargo.toml --check` step in tauri-check.
Confidence: HIGH — Verification: read (all workflow/gate greps exit 0).

F-005 [S4]
Where: docs/dependencies/records/tauri-wry-2.x.md:1, 98, 105 (vs apps/desktop-tauri/Cargo.lock:4961-4962) — the record this manifest cites at L57
Code:
```
# deps/records/tauri-wry-2.x.md — tauri 2.12.4 + wry 0.57.0 (pre-audit record, the ADR-004 admission vehicle)
```
vs Cargo.lock:4961-4962:
```
name = "wry"
version = "0.57.2"
```
What is wrong: The ADR-004 admission record refreshed tauri to 2.12.5 (record:169, 183) but NEVER refreshed wry: its header (L1), R7 (L98) and R8 (L105) all still say wry 0.57.0 while the admitted lockfile ships wry 0.57.2 (deny.toml:122-123 comments state "tauri 2.12.5 + wry 0.57.2" — the deny comment is more current than the record it serves). ADR-004's revisit trigger (L126-127: "tauri/wry version drift at admission time re-runs the pre-audit") was not honored for wry's 0.57.0 -> 0.57.2 drift. Also: allowlist §3a's wry row pins range `<2.13.0` (allowlist.md:127) — a range that is dimensionally wrong for a 0.x crate (it is tauri-runtime-wry's range pasted onto wry).
Why it matters: The record is the governance artifact backing this manifest's central dependency; its wry claims are stale, and the pre-audit (R8 maintenance/RustSec ledger) describes a version that is not the one shipped.
What correct looks like: record header/R7/R8 refreshed to wry 0.57.2; allowlist wry row carrying a 0.57.x range.
Confidence: HIGH — Verification: read (all sed-verified).

F-006 [S4]
Where: apps/desktop-tauri/Cargo.toml (absence — whole file has no [profile] table)
Code (the file's only build-shaping content, L23-32):
```
[package]
name = "lumen-desktop-tauri"
version = "0.1.3"
edition = "2024"
```
What is wrong: No [profile] section. As an excluded crate it cannot inherit root profiles — and the root defines none (no [profile] in root Cargo.toml or .cargo/config.toml; the only profiles in-repo are tools/vta/mpeg4/Cargo.toml:18 and fuzz/Cargo.toml:59, both separate workspaces). The shipped lumen-tauri release binary (release.yml:219) therefore builds with stock cargo defaults: no strip, no LTO, default codegen-units.
Why it matters: Hygiene/size debt on a shipped binary; also worth knowing that "profile settings" for this crate are literally nothing — any future root-level profile work silently will not reach it.
What correct looks like: a deliberate `[profile.release]` (at minimum `strip = true`) or a documented decision to keep defaults.
Confidence: HIGH — Verification: read (rg '^\[profile' across all Cargo.tomls; ls .cargo/config.toml).

## 4. Per-key table (68-line manifest; every key inventoried)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| [package] name/edition/publish | 23-28 | REAL | — | name lumen-desktop-tauri; edition 2024 OK under toolchain 1.99.1; publish=false present |
| [package] version "0.1.3" | 25 | REAL | — | == tauri.conf.json version, == desktop 0.1.3 (lock L653-654) |
| [package] description | 27 | REAL | — | claims match tauri.conf.json/CI (one command, embedded ui/dist — ci.yml:717-724) |
| [[bin]] lumen-tauri -> src/main.rs | 30-32 | REAL | — | src/main.rs exists (201 lines, W14-A03); built by release.yml:219 |
| [lints.rust] 4 keys | 34-38 | REAL | — | byte-identical to root [workspace.lints.rust] (diff verified) |
| [lints.clippy] 7 keys | 40-47 | REAL | — | byte-identical to root [workspace.lints.clippy] (diff verified) |
| dependencies: desktop (path) | 53 | REAL | — | apps/desktop member; L6 per boundaries.toml:539 |
| dependencies: commands (path) | 56 | REAL | — | crates/commands 0.1.0 matches lock L426-427; layer mislabeled in comment (F-002) |
| dependencies: tauri "2" | 60 | REAL-UNPROVEN | S3 | resolved 2.12.5 (lock L3688-3689); range too wide (F-001); defaults verified, no tray-icon |
| dependencies: serde "1" + derive | 62 | REAL | — | used at src/app.rs:118-119; serde 1.0.229 locked |
| build-dependencies: tauri-build "2" | 68 | REAL-UNPROVEN | S3 | build.rs:11 calls tauri_build::build(); resolved 2.7.4 (lock L3739-3740); range too wide (F-001) |
| [features] / [profile] / [workspace] / [dev-dependencies] | absent | — | S4 | no feature surface (good: nothing feature-gated); no profile (F-006); tests/ dir exists with zero dev-deps |

## 5. Test honesty
No tests in this file. The crate's tests (tests/single_command.rs) run only in the tauri-check CI lane (`cargo test --manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets`, ci.yml:731). In-sandbox run: UNVERIFIED (gdk-sys, no webkit2gtk — baseline environment fact).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Version '2' per the record's admission range" | Cargo.toml:58-59 | FALSE | allowlist.md:125 range is `>=2.12.5, <2.13.0`, not "2" |
| "commands (path, L3)" | Cargo.toml:16 | FALSE | boundaries.toml:134 `layer = 1` |
| "desktop (path, L6)" | Cargo.toml:15 | REAL | boundaries.toml:539 `layer = 6` |
| "[lints] workspace = true cannot resolve for an excluded crate ... policy mirrored inline" | Cargo.toml:20-21 | REAL | root excludes it (Cargo.toml:51); walk-up mechanics reproduced (cargo metadata exit 101); mirror diff-identical |
| "tauri-codegen embeds frontendDist at compile time" | Cargo.toml:5-6 | REAL | tauri-codegen-2.7.4/src/context.rs:182-194 |
| "wry + tao ride its own graph in THIS crate's lockfile" | Cargo.toml:59-60 | REAL | lock: wry 0.57.2 (L4961-2), tao 0.37.2 (L3631-2) |
| "no engine crate is touched" (scoped to the commands edge) | Cargo.toml:17 | REAL | boundaries.toml [crates.commands] internal_deps = [] |
| "a build-time file processor, links nothing extra" (tauri-build) | Cargo.toml:66-67 | REAL-UNPROVEN | build.rs:11 calls tauri_build::build(); "links nothing" not machine-verified here (cargo-build-budget) |
| "tauri 2.12.5 / wry 0.57.2" (task's ADR-004 version pair) | deny.toml:122-123, Cargo.lock | SPLIT | tauri 2.12.5 REAL (lock L3688-9; record:169,183); wry 0.57.2 REAL in lock/deny but the RECORD still says 0.57.0 everywhere (F-005) |

## 7. Dead / duplicated / unreachable code
- No dead config keys. The [lints] duplication of the root table is deliberate and documented (L20-21); it will silently drift if the root policy changes (no machine check compares them — noted for a future gate).
- tray-icon/libappindicator/libloading exist in Cargo.lock as feature-agnostic entries; cargo tree proves they are NOT in the build graph (no unreachable shipped code).

## 8. Dependencies & flags
- 4 deps + 1 build-dep, all accounted for above. tauri default features (registry tauri-2.12.5/Cargo.toml:84-91): wry, compression, common-controls-v6, dynamic-acl, x11, dbus — all shell plumbing; matches allowlist §3a "no default features beyond the shell's needs". tray-icon NOT default; deny.toml's libloading wrapper-scope story (deny.toml:175-187) verified consistent.
- Boundaries: crate exempt from boundaries.toml by design (no entry, rg zero matches); its path deps point at L6 desktop and L1 commands — both legal downward/same-layer directions for an app-shell crate.
- Toolchain: edition 2024 fine under rust-toolchain.toml channel 1.99.1.

## 9. Open questions for the next agent
- W14-A08 (tauri.conf.json): frontendDist "../../ui/dist" — ui/dist does NOT exist in-tree (ls verified); every lane must build the UI first (ci.yml:717-724 ordering). Confirm the release lanes all respect that order.
- Why did the admission use the inline-mirror pattern when tools/bench-vs-vlc/Cargo.toml:17-41 solves the same out-of-workspace lint problem with its own [workspace] + [workspace.lints] + [lints] workspace=true? Pattern divergence, not a defect.
- Who back-fills the record's wry 0.57.0 -> 0.57.2 (F-005)? That owner also owes the allowlist wry-row range fix.
- INCIDENT (environment): my `cargo tree --target all` probe downloaded ~145 crates and filled the disk (100%); I deleted the 145 new .crate cache files + the partial windows-0.62.2 extraction (63MB free now). Registry src/ (602MB) untouched. Baseline builds should be unaffected; flagging for the orchestrator.
