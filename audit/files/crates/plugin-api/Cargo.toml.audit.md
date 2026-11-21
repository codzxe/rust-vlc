# AUDIT: crates/plugin-api/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-21 |
| Lines / functions | 34 / 8 config keys ([package]×5, [lints], [dependencies]×5, [dev-dependencies]×1) |
| File verdict | REAL (boundaries exact, all deps used, lockfile at allowlist floors, reachable and executed in the baseline binary) |
| Reachable from shipped entry point? | YES (call chain: `lumen` bin → lumen_cli (apps/cli/Cargo.toml:59 `plugin-api`) → demux_host.rs:61 `use plugin_api::{Access, DemuxEvent, Demuxer, SeekSupport, access::SourceStream}` → the MP4/AVI open path; executed: baseline MP4 smoke demuxed 2 tracks, AVI lane decoded) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Grep boundaries.toml `[crates.plugin-api]` → internal [media-types, codec-api], dev [testkit], external [serde, toml, thiserror] (:80-85) = exact manifest match; Grep allowlist.md → serde/toml/thiserror rows all verdict `pass` (:63-66); Grep Cargo.lock → serde 1.0.229 (:853), thiserror 2.0.21 (:1124), toml 1.1.8+spec-1.1.0 (:1150), single versions each, all at the allowlist floors; Grep src for `serde\|toml\|thiserror` usage → manifest.rs:35/307, semver.rs:201 (all three used); Grep `#\[allow\(\|unsafe` in crate → exactly 1 style escape (kind.rs:124 `#[allow(clippy::match_same_arms)]`), zero unsafe; `rg -n 'pub enum PluginKind'` kind.rs:27 (16-kind taxonomy exists, numbered doc rows); Grep apps/cli/src `plugin_api` → demux_host.rs:61, bench.rs:66, containers/*.rs (heavily used); baseline #1/#3/#5/#11/#20 |

## 1. One-paragraph verdict
An honest, tight manifest: two internal deps (media-types, codec-api) declared exactly in boundaries.toml both directions; three external deps that are genuinely used (serde for the manifest model's custom ser/de, toml for `PluginManifest` parsing at manifest.rs:307, thiserror for the semver error type at semver.rs:201), each locked at exactly the allowlist's floor version with verdict `pass`; one style-only lint escape and zero unsafe in the whole crate. Unlike most of this batch, the crate is not just reachable but load-bearing: the CLI's entire container lane (`apps/cli/src/containers/*.rs`, demux_host.rs) runs through plugin-api's `Demuxer`/`SourceStream` traits, and the baseline smoke proves that path executes. The single finding is the systemic manifest-float-vs-allowlist-floor gap (same class as the desktop-tauri S2, but with an accurate comment and lockfile discipline, hence S4).

## 2. What actually works (verified)
— Reachability + execution: `lumen` → lumen-cli → plugin-api (apps/cli/Cargo.toml:59); the demux contract (`plugin_api::{Demuxer, SourceStream}` — demux_host.rs:61, containers/avi.rs:17, containers/mp4.rs et al.) is the code path the baseline MP4 smoke exercised (INDEX.md: open session → "event: tracks 2 ready … MediaInfoReady { tracks: [Video h264, Audio aac] }" — demux works through these traits; AVI lane `v_decoded=60 a_decoded=60`).
— Manifest ↔ boundaries exact two-way match (boundaries.toml:80-85 vs manifest:23-33).
— Allowlist claims verified: serde/toml/thiserror all verdict `pass` (allowlist.md:63-66); lockfile holds each at its floor (serde 1.0.229, toml 1.1.8+spec-1.1.0, thiserror 2.0.21 — Cargo.lock:853/:1150/:1124), single version each.
— All three externals are real dependencies (used in src — no decorative deps): serde (manifest.rs:35 `use serde::{Deserialize, Serialize}` + custom ser/de impls), toml (manifest.rs:307 `toml::from_str(src)`), thiserror (semver.rs:201 `#[derive(… thiserror::Error)]`).
— Scope fence holds: no registry/selection/host/enforcement code in this crate (Grep of src surface confirms contract types only; the host lives in the orphaned plugin-host-wasm).

## 3. Findings
### F-001 [S4]
Where: crates/plugin-api/Cargo.toml:27-29
Code:
```
serde = { version = "1", features = ["derive"] }
toml = "1.1"
thiserror = "2.0"
```
What is wrong: The requirements are broader than the audited floors: allowlist.md:63-66 admits serde >=1.0.229, toml >=1.1.8, thiserror >=2.0.21 — the manifest's `"1"`/`"1.1"`/`"2.0"` would also accept serde 1.0.0, toml 1.1.0, thiserror 2.0.0 (below every audited floor). Only the committed Cargo.lock (all three pinned exactly at the floors) + `--locked` builds hold the line; an unlocked resolve in a fresh environment can drift below the audited set with no gate failing.
Why it matters: Same structural gap as the desktop-tauri tauri="2" case (this batch's file 3, S2 there) — manifests do not encode the allowlist ranges, and no checker diffs requirement-vs-allowlist. Stakes here are low (floors within the same major; pure-Rust pass-verdict family), hence S4, but the pattern is systemic across every manifest with externals audited so far.
What correct looks like: Either floors in the manifests (`serde = "1.0.229"`, `toml = "1.1.8"`, `thiserror = "2.0.21"`) or an xtask check that fails when a manifest requirement is wider than its allowlist row.
Confidence: HIGH — Verification: read of manifest + allowlist.md:63-66 + Cargo.lock versions.

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package]` name/version/edition/publish | 13-18 | REAL | — | `plugin-api` 0.1.0, edition 2024, publish=false; "16-kind taxonomy" matches kind.rs:27 `pub enum PluginKind` with numbered doc rows. |
| `[lints] workspace = true` | 20-21 | REAL | — | Binds (baseline #11 39/39); one style escape only (kind.rs:124 match_same_arms), zero unsafe. |
| `media-types` dep | 24 | REAL | — | L1→L0 downward; declared (boundaries:82); used (SourceUri etc.). |
| `codec-api` dep | 25 | REAL | — | L1→L1 same-layer pair (boundaries.toml:45 `same_layer_pairs` includes "plugin-api->codec-api"); used (Decoder re-use per kind.rs docs). |
| `serde` dep | 27 | REAL | — | Used (manifest.rs:35+); allowlist pass; lock at floor. |
| `toml` dep | 28 | REAL | — | Used (manifest.rs:307); allowlist pass; lock at floor. |
| `thiserror` dep | 29 | REAL | — | Used (semver.rs:201); allowlist pass; lock at floor. |
| `testkit` dev-dep | 33 | REAL | — | Matches boundaries dev_deps:83; MP §3.7 contract. |

## 5. Test honesty (if tests exist in/for this file)
No tests in this file. The crate's suites ran inside baseline #3 (workspace, 1681 passed). Note: crates/plugin-api/tests/testkit_wiring.rs is the ONE file that fails `cargo fmt --all --check` (baseline #7) — a hygiene drift in this crate's test tree, for the src auditor to record.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The ONLY external crates are the pre-audited config family (allowlist verdict `pass`): serde (derive), toml, thiserror" | manifest:4-6 | REAL | Exactly those three in [dependencies]; all verdict `pass` (allowlist.md:63-66); boundaries external_allowlist matches (:84). |
| "the same set product-config/settings-schema use" | manifest:6 | REAL | allowlist.md:45-47 lists the same crates for product-config/settings-schema (plugin-api is an additional consumer, same family). |
| "no registry, no selection policy, no WASM host, no enforcement tooling, no conformance kit" (scope fence) | manifest:8-11 | REAL | Grep of crate surface: contract types only; the WASM host is a separate (orphaned) crate. |
| Registry row: plugin-api deps media-types + codec-api | doc 01 §2.2 / manifest:3-4 | REAL | Manifest declares exactly those two internals; boundaries:82 matches. |

## 7. Dead / duplicated / unreachable code
- None in the manifest. Note for the naming question: "plugin-api" is the de-facto demux/access contract layer for the whole engine (the CLI's containers implement `plugin_api::Demuxer`), broader than its name suggests — documented design (doc 01 §2.2), not a defect, but worth one line in the wave summary: the crate every container depends on is named after the (unwired) plugin system.

## 8. Dependencies & flags
- Internal: media-types (L0), codec-api (same-layer, whitelisted pair boundaries.toml:45). Dev: testkit. External: serde(+derive), toml, thiserror — all allowlist `pass`, all lockfile-pinned at floors, none banned by deny.toml (pure-Rust MIT/Apache family inside the license allow set).
- Features: none beyond serde derive.

## 9. Open questions for the next agent
- Systemic (root-manifest/xtask owner): requirement-vs-allowlist drift check — this file (S4) and desktop-tauri (S2) both show manifests wider than audited ranges; a single xtask gate comparing `Cargo.toml` requirements against allowlist.md rows would close the class.
- Src auditor: kind.rs:124's `match_same_arms` allow — check the 16-kind match arms really are intentionally identical (the allow may hide a copy-paste arm).
