# AUDIT: crates/plugin-host-wasm/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-21 |
| Lines / functions | 84 / 12 config keys ([package]×5, [lints], [dependencies]×4, [dev-dependencies]×6, [[bin]]×2) |
| File verdict | UNREACHABLE (manifest is internally consistent and boundary-clean; the crate it defines is linked into no shipped entry point) |
| Reachable from shipped entry point? | NO — zero dependents repo-wide (proof below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Grep `plugin-host-wasm` over `**/Cargo.toml` → only its own manifest + a comment in plugin-api's; `sed -n '9,24p'`/`51,83p'` on the manifest (quote verify); `sed -n '574,605p' boundaries.toml` (entry matches manifest exactly); `sed -n '505,545p' boundaries.toml` (lumen-cli entry, no plugin-host-wasm); Read apps/cli/Cargo.toml + apps/web/Cargo.toml (neither depends on it); Grep `lumenext` outside crates/ → only worklog.md + plugins/extensions/README.md (no CI/packaging ship); Grep `7\.w22d\|auto-dispatch` → worklog.md:578-594; Read docs/dependencies/records/wasmi.md (132 lines); Grep lockfile wasmi closure → single version each (wasmi:1263, wasmi_collections:1276, wasmi_core:1285, wasmi_ir:1294, wasmparser:1303, spin:937, string-interner:957, libm:354, bitflags:63) |

## 1. One-paragraph verdict
The manifest itself is disciplined: every declared edge matches boundaries.toml verbatim (internal `[plugin-api, settings-schema]`, dev `[codec-api, media-clock, media-types, packet-queue, playback-engine, testkit]`, external `[wasmi, toml]`), `[lints] workspace = true` is present so the deny-lints bind, the wasmi pin resolves to exactly one version of every closure crate in Cargo.lock, and both `[[bin]]` targets exist. The problem is what the manifest defines: a complete WASM extension host (wasmi interpreter, capability jail, .lumenext packaging CLI) that **no shipped binary links** — not `lumen` (apps/cli/Cargo.toml:47-65 omits it), not `lumen-web` (apps/web/Cargo.toml:52-71 omits it), not `lumen-tauri` (own excluded graph). The feature matrix is honest about this (`plugin-wasm-extensions` = not_started on all four platforms), so this is dead weight, not fraud — but every `cargo build --workspace` lane compiles the 8-crate wasmi runtime closure for zero product reach.

## 2. What actually works (verified)
— Manifest ↔ boundaries.toml exact agreement: `[crates.plugin-host-wasm] internal_deps = ["plugin-api", "settings-schema"]`, `dev_deps = [...]`, `external_allowlist = ["wasmi", "toml"]` (boundaries.toml:574-586) equal the manifest's `[dependencies]`/`[dev-dependencies]` keys (manifest:51-71) — verified by side-by-side read; baseline #11 check-architecture reports the plugin-host-wasm edges compliant (the single violation is lumen-cli→streaming).
— Single-version closure: Grep of Cargo.lock shows exactly one entry each for wasmi 2.0.0, wasmi_collections, wasmi_core, wasmi_ir, wasmparser, spin, string-interner, libm — no duplicate versions.
— The crate and its 45-test suite compile and pass inside the workspace run: baseline #1 build CLEAN, #3 `cargo test --workspace` 1681 passed (worklog 7.w22d records `cargo test -p plugin-host-wasm` = 45 passed / 0 failed at commit 1ce1ec9-era gates).
— All three production deps are really used in src: `plugin_api` (instance.rs:14, api.rs:45), `settings_schema` (supervisor.rs:123), `toml` (supervisor.rs:170) — no decorative dependencies.

## 3. Findings
### F-001 [S3]
Where: crates/plugin-host-wasm/Cargo.toml:41-46,51-57,73-83
Code:
```
[package]
name = "plugin-host-wasm"
...
[[bin]]
name = "lumenext"
path = "src/bin/lumenext.rs"
```
What is wrong: Nothing in any shipped composition root depends on this crate. Grep for `plugin-host-wasm` across every `Cargo.toml` in the repo matches only this manifest and a comment in plugin-api's. The `lumen` binary's dependency list (apps/cli/Cargo.toml:48-63: audio-engine … video-engine, serde_json, toml) has no plugin-host-wasm; `lumen-web`'s (apps/web/Cargo.toml:54-71) has none; `lumen-tauri` is workspace-excluded with its own lockfile. The two `[[bin]]` tools (`lumenext`, `lumenext-examples`) are referenced only by plugins/extensions/README.md and the worklog — no workflow, script, or packaging lane ships them.
Why it matters: The entire sandboxed-WASM-extension subsystem — the wasmi 2.0.0 runtime and its 7-crate closure (Cargo.lock:1263-1303) — is compiled by every `cargo build --workspace`/clippy/test lane while being unreachable from the product. That is dead weight in the build graph and an audit surface (a whole interpreter) with no user-reachable payoff; a reader of doc 06 §5 could believe extensions ship.
What correct looks like: Either wire the host into a composition root (Phase 7 per the matrix) or keep the crate out of the default workspace lane until then (workspace-default-members), so CI lanes stop paying for code no entry point reaches.
Confidence: HIGH — Verification: read + Grep dependency scan (0 dependents) + baseline INDEX (shipped binary self-reports features with no extension surface: `lumen engine-info` → decoders mjpeg, pcm_s16le; features first-vertical:avi-mjpeg-pcm, audio.null-device, video.software-compositor).

### F-002 [S4]
Where: crates/plugin-host-wasm/Cargo.toml:9-16
Code:
```
# External deps: **wasmi 2.0.0** — the ADR-002 runtime (accepted
# 2026-10-19; full R1–R10 record at docs/dependencies/records/wasmi.md).
# Pinned per the record's recommended posture: default-features = false,
# features = ["std", "validate", "auto-dispatch"] — the audited closure
```
What is wrong: Citation drift. The record's actual recommendation (docs/dependencies/records/wasmi.md:125-130) is `no-default-features, std`, with `validate` offered conditionally and `wat` for dev/tests — `auto-dispatch` appears nowhere in the 132-line record (Grep `auto-dispatch` over it → 0 matches). The manifest attributes the full three-feature pin to "the record's recommended posture"; the real authority for `auto-dispatch` is worklog task 7.w22d (worklog.md:586: "wasmi features += `auto-dispatch` (zero-crate feature …; Cargo.lock byte-identical, closure unchanged)").
Why it matters: The record is the artifact a security reviewer reads; the stack-overflow root-cause (tail-call backend at opt-level 0/1, worklog.md:586-590) and its repair live only in a chronological worklog, so the dependency record understates the pin's rationale and drifts from the manifest that cites it.
What correct looks like: Append the 7.w22d addendum (feature, root cause, "closure unchanged" proof) to docs/dependencies/records/wasmi.md and reword the manifest line to "pinned per the record + the 7.w22d repair".
Confidence: HIGH — Verification: read of both files + Grep; worklog 7.w22d is recorded evidence, not a run I executed (rule 8).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package] name/version/edition/publish` | 41-46 | REAL | — | `plugin-host-wasm` 0.1.0, edition 2024, publish=false; consistent with boundaries `[crates.plugin-host-wasm]`. |
| `[lints] workspace = true` | 48-49 | REAL | — | Deny-lints bind (baseline #11 "unsafe-lints 39/39 inherit"); crate's own code passes `-D warnings` (baseline #5). |
| `plugin-api` dep | 52 | REAL | — | Used (instance.rs:14, api.rs:45); boundaries internal dep. |
| `settings-schema` dep | 53 | REAL | — | Used (supervisor.rs:123-140); boundaries internal dep. |
| `toml = "1"` dep | 55 | REAL | — | Used once (supervisor.rs:170 `toml::Value::Integer`); boundaries external allowlist. |
| `wasmi` dep | 57 | REAL-UNPROVEN | — | Resolves to wasmi 2.0.0, single lock version; feature set (`std, validate, auto-dispatch`) not verifiable without cargo metadata (rule 8) — baseline #1 `--locked` build CLEAN proves the names resolve; the feature narrative is worklog-backed (7.w22d). |
| `[dev-dependencies]` (6 crates) | 66-71 | REAL | — | Exact match to boundaries dev_deps (586); all test-only; playback-engine used by the §9.5 crash-isolation proof per worklog 7.w22d/7.F4. |
| `[[bin]] lumenext` | 73-75 | UNREACHABLE | S3 (part of F-001) | File exists; demo/packaging CLI, shipped nowhere. |
| `[[bin]] lumenext-examples` | 81-83 | UNREACHABLE | S3 (part of F-001) | File exists; deterministic fixture generator with `--check` drift gate (referenced plugins/extensions/README.md:41-45); shipped nowhere. |

## 5. Test honesty (if tests exist in/for this file)
No tests in this file. The crate's suite is out of scope here (src auditor's file); worklog 7.w22d claims 45/45 including the doc 06 §9.4 escalation set — NOT executed by this audit, recorded evidence only.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "v1 wires the plugin-api manifest/permission contract only" (media-types/commands edges forward-looking) | manifest:4-7 | REAL | Manifest deps are exactly plugin-api + settings-schema; no media-types/commands edges. |
| "Pinned per the record's recommended posture" | manifest:11-12 | FALSE (as stated) | Record R10 (wasmi.md:125-130) recommends std (+conditional validate); auto-dispatch absent from the record — see F-002. |
| WASM extensions status | feature-matrix.toml:2321-2331 (`plugin-wasm-extensions` not_started ×4, evidence "") | REAL (honest) | Zero dependents (F-001 proof) matches the not_started row — the matrix does not overclaim. |
| wasmi 2.0.0 pure-Rust, 8-crate closure, MIT/Apache | wasmi.md R1/R2/R7 | REAL-UNPROVEN (recorded, not re-run) | Lock closure single-version (Grep cites); license class in deny.toml allow set; baseline #25 licenses ok. |

## 7. Dead / duplicated / unreachable code
- The whole crate is unreachable from shipped entry points (F-001) — by manifest design (no app declares the edge), matching the matrix's not_started.
- No duplicated dependency versions in the wasmi closure (verified single lock entries).

## 8. Dependencies & flags
- External: wasmi 2.0.0 (default-features=false, +std +validate +auto-dispatch), toml "1". Both in boundaries external_allowlist; neither banned in deny.toml; licenses (MIT/Apache-2.0) inside deny.toml's allow list (deny.toml:84-87). deny.toml's advisories ignore-list does not cover any wasmi-closure crate (correct — none needed per baseline #25: advisories failure is the libappindicator wrapper, unrelated).
- Internal: plugin-api, settings-schema; dev: playback-engine, media-types, codec-api, media-clock, packet-queue, testkit. All match boundaries.toml:574-586 exactly.
- Features: none on the crate itself; wasmi feature set per F-002.

## 9. Open questions for the next agent
- Whoever audits crates/plugin-host-wasm/src/**: the worklog claims 45/45 incl. 6 sandbox-security tests — verify the typed-refusal claims (E_PERM/E_OOB/E_QUOTA) are real assertions, not log-line checks.
- Wave owner for doc 06: §5 describes the extension host as product surface; the matrix says not_started. Confirm no doc tells users extensions load in the shipped binary.
