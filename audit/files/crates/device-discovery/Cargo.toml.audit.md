# AUDIT: crates/device-discovery/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-21 |
| Lines / functions | 59 / 7 config keys ([package]×5, [lints], [dependencies]×1, [dev-dependencies]×1) |
| File verdict | UNREACHABLE (correct, minimal manifest; the crate it defines has zero dependents) |
| Reachable from shipped entry point? | NO — no Cargo.toml in the repo declares a dependency on `device-discovery` (proof below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | Grep `^\s*device-discovery\s*=` over `**/Cargo.toml` → 0 matches (no dependents; the platform-* mentions are comments only); `rg -n '^\[crates\.device-discovery\]' -A 10 boundaries.toml` → internal_deps = ["media-types", "plugin-api"] (:281); `sed -n '235,246p' docs/01-architecture.md` → registry row :237 still "media-types, plugin-api"; Grep `plugin_api\|plugin-api` in crates/device-discovery → only doc-comment mentions, no import; Grep `use media_types\|media_types::` in src → used (device.rs:18, volume.rs:31, error.rs:180-182); Grep `#\[allow\(` in crates/device-discovery → 0 matches; Grep platform-* Cargo.tomls → they declare only audio-output + media-types; baseline #5/#11 |

## 1. One-paragraph verdict
The manifest body is the cleanest so far: one real internal dep (media-types, verified used), one dev dep (testkit, matches boundaries), `[lints] workspace = true`, zero externals, zero `#[allow(` escape hatches in the entire crate — so its "no unwrap/expect/panic" claim actually holds here. The problem is the registry story around it: the manifest removed the plugin-api dependency on 2026-11-17 (honest NOTE, cargo-machete/cargo-shear evidence) but neither of the two "sources of truth" was updated — boundaries.toml:281 and doc 01 §2.4's registry row (01-architecture.md:237) still declare the dead edge, and the arch gate is structurally blind to over-declaration ("missing allowed-edge = fine", check_architecture.rs:12). Meanwhile the crate itself is an orphan: the three platform-* adapters that doc 01 says will consume it declare only audio-output + media-types, so `device-discovery` is compiled into every workspace build and reachable from nothing.

## 2. What actually works (verified)
— Manifest↔reality agreement for what IS declared: `media-types` is genuinely imported (src/device.rs:18 `use media_types::{EngineError, SourceUri};`, volume.rs:31, error.rs:180-182); no `plugin_api` import exists anywhere in the crate (Grep → doc comments only), so the T5 removal was correct, not a cover-up.
— Lint hygiene claim holds for this crate: zero `#[allow(` sites (Grep → 0 matches) — unlike the repo-wide 121; workspace clippy `-D warnings` passes (baseline #5).
— boundaries.toml dev_deps = ["testkit"] (:282) matches the manifest's single dev-dependency exactly.
— Crate compiles + tests run green inside the workspace suite (baseline #1 build CLEAN, #3 tests 1681 passed/1 unrelated fail).

## 3. Findings
### F-001 [S3]
Where: crates/device-discovery/Cargo.toml:47-53 (cross-ref boundaries.toml:279-284, docs/01-architecture.md:237)
Code:
```
# NOTE (audit T5, 2026-11-17): the registry row once listed plugin-api
# here, but this crate never imports it — plugin-api's discovery.rs
# stages the HOST surface and points at this crate as the domain-trait
# home (doc 06 §1.2 row 16), i.e. the reference direction is the
# reverse, so the declared dep was dead weight (cargo-machete +
# cargo-shear both flagged it). The vocabulary stays shared through
# docs and media-types; nothing in src/ changes.
```
What is wrong: The manifest removed the plugin-api dependency, but both registry artifacts still declare it: boundaries.toml:281 `internal_deps = ["media-types", "plugin-api"]` and doc 01's registry row (01-architecture.md:237 `| device-discovery | crate | L2 | media-types, plugin-api | 7 | services discovery |`). The architecture gate cannot catch this class of drift — check-architecture's edge check is allowlist-shaped ("unknown crate = error, missing allowed-edge = fine", check_architecture.rs:12), so an edge declared in boundaries.toml but absent from the manifest passes forever. This is a second, previously-unrecorded boundaries-vs-reality mismatch (baseline #11 records only the lumen-cli→streaming under-declaration).
Why it matters: boundaries.toml is the artifact the repo presents as its enforced dependency constitution; entries that outlive the code they govern silently erode it. Anyone reasoning from boundaries.toml about the plugin-api↔device-discovery coupling reaches a wrong conclusion (the NOTE says the reference direction is now the reverse — plugin-api points HERE, not vice versa).
What correct looks like: Same-PR edit discipline for registry artifacts (boundaries.toml + doc 01 row) whenever a manifest edge changes, or extend check-architecture to also fail on declared-but-nonexistent internal edges (over-declaration), which would have flagged this on the next gate run.
Confidence: HIGH — Verification: read + Grep (all three files), plus allowlist semantics read from check_architecture.rs:12.

### F-002 [S3]
Where: crates/device-discovery/Cargo.toml:12-20,43-46
Code:
```
# OS integrations for volume watching live in the L5 `platform-*`
# crates — doc 01 §2.7's registry rows list device-discovery as a dep
# of all three adapters ("OS impls behind plugin traits"). This crate
# mints the port ([`volume::VolumeSource`]) the adapters will
# implement; until those cards land every lane reports the typed
```
What is wrong: The consumer wiring is entirely future tense. Grep for `device-discovery =` as a dependency key across every Cargo.toml → 0 matches. The three platform-* adapters (which doc 01 §2.7 lists as depending on device-discovery) each declare only `audio-output` + `media-types` (platform-linux/Cargo.toml:39-41, mirrored in -windows/-macos; boundaries.toml:461-488 confirms). No app, no engine crate, no host depends on device-discovery. It is built by every `cargo build --workspace` and reachable from neither shipped entry point.
Why it matters: Another full crate of dead weight in the default workspace lane (same class as plugin-host-wasm). The crate's own header is honest about the sequencing ("until those cards land every lane reports the typed Unavailable verdict"), so this is planning debt, not fraud — but the audit must record that the "services discovery" layer has zero product reach today.
What correct looks like: Land the platform-* adapter edges (issues #41/#42 cards) or move the crate behind a workspace-default-members exclusion until its consumers exist.
Confidence: HIGH — Verification: read + Grep dependency scan + boundaries.toml:461-488.

### F-003 [S4]
Where: crates/device-discovery/Cargo.toml:4-5
Code:
```
# Registry: doc 01 §2.4 `device-discovery` (layer L2, deps
# media-types + plugin-api — exactly the Lead's registry row, no more).
```
What is wrong: The header still asserts the manifest matches "exactly the Lead's registry row, no more" — but the body (NOTE, lines 47-53) removed plugin-api while the registry row kept it. The file's own header and its own NOTE contradict each other: the manifest is now ONE dep short of the row the header claims to mirror exactly.
Why it matters: An internally contradictory manifest header misleads the next reader about which artifact is authoritative; the honest NOTE is 40 lines below the stale claim.
What correct looks like: Update the header to "deps media-types (the registry row's plugin-api edge was removed by audit T5 — see NOTE below; registry row pending update)".
Confidence: HIGH — Verification: read.

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package]` name/version/edition/publish | 33-38 | REAL | — | `device-discovery` 0.1.0, edition 2024, publish=false; description matches actual scope (services discovery, volume watcher, SSDP lane). |
| `[lints] workspace = true` | 40-41 | REAL | — | Binds (baseline #11 39/39 inherit); crate passes clippy -D warnings (baseline #5) with zero `#[allow(` (Grep → 0). |
| `media-types` dep | 46 | REAL | — | Used (device.rs:18, volume.rs:31, error.rs:180-182); L2→L0 downward edge, declared in boundaries:281. |
| (removed) plugin-api dep NOTE | 47-53 | REAL | — | Honest removal record; but see F-001 (boundaries/doc not updated) and F-003 (header contradiction). |
| `testkit` dev-dep | 58 | REAL | — | Matches boundaries dev_deps:282; MP §3.7 dev-only contract. |

## 5. Test honesty (if tests exist in/for this file)
No tests in this file. The crate's tests ran inside baseline #3 (workspace suite, 1681 passed) — per-crate breakdown not in baseline; src auditor to verify the "typed Unavailable verdict" tests are real assertions.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "deps media-types + plugin-api — exactly the Lead's registry row, no more" | manifest:4-5 | FALSE (stale self-description) | Manifest now declares media-types only (line 46); registry row 01-architecture.md:237 still lists both — see F-001/F-003. |
| "every lane reports the typed Unavailable verdict — honest capability reporting, never fake events" | manifest:17-19 | REAL-UNPROVEN | Honest design statement; the Unavailable lanes are src behavior (src auditor's file); no per-crate baseline run exists. |
| "std-only, zero external dependencies" | manifest:22 | REAL | [dependencies] has only the path dep media-types; boundaries external_allowlist = [] (:283). |
| "No unsafe, no unwrap/expect/panic (workspace lints)" | manifest:24-25 | REAL | Zero `#[allow(` in the crate (Grep → 0) — this crate does not use the escape hatch; baseline #5 clippy CLEAN. |

## 7. Dead / duplicated / unreachable code
- The whole crate is currently unreachable from shipped entry points (F-002) — zero dependents.
- The plugin-api dependency was dead weight and is removed (manifest NOTE) — the only *documented* instance of a cargo-machete/cargo-shear-driven cleanup in the manifests audited so far.

## 8. Dependencies & flags
- Internal: media-types (L0) — downward, declared, used. Dev: testkit. External: none.
- No features, no bins, no build.rs. boundaries.toml entry exists ([crates.device-discovery], :279-284) with the stale plugin-api edge (F-001).

## 9. Open questions for the next agent
- Boundaries.toml owner: how many OTHER stale over-declared edges exist? check-architecture cannot see them (allowlist semantics) — a reverse diff (boundaries internal_deps vs actual manifest deps) across all 39 members would quantify the drift. This audit found one (device-discovery→plugin-api); baseline #11 found the opposite kind (lumen-cli→streaming).
- Src auditor for crates/device-discovery/src/ssdp.rs: verify the "[VERIFY] honest feasibility verdict" is a real typed verdict, not a stub returning constant Unavailable with no socket work.
