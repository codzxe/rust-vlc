# AUDIT: crates/plugin-api/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W12-A02 |
| Date | 2026-11-24 |
| Lines / functions | 138 / 1 const, 12 module decls, 35 top-level re-exports, prelude (34), 1 test |
| File verdict | REAL (crate root; compiled into shipped lumen-cli/web via demux/access traits); two doc claims false |
| Reachable from shipped entry point? | YES — apps/cli/Cargo.toml:59 and apps/web/Cargo.toml:61 depend on plugin-api; `use plugin_api::{Access, DemuxEvent, Demuxer, ...}` at apps/cli/src/demux_host.rs:61 compiles this root. The manifest/semver/kind/ui/metadata/... modules ride along but only demux/access types are invoked by shipped code. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -q -p plugin-api` -> 42+6+2 passed, 0 failed, exit 0 (includes this file's test); `ls plugins/` -> extensions only; rg ENGINE_PLUGIN_API_MAJOR / plugins/builtin (results below) |

## 1. One-paragraph verdict
A clean crate root: 12 modules, disciplined re-export surface, honest scope fence (no registry / no WASM host / no enforcement — all verified true for this crate). Two doc lies: it claims CI verifies every `plugins/builtin/*.toml` against `ENGINE_PLUGIN_API_MAJOR`, but `plugins/builtin/` does not exist anywhere in the tree and no such CI check exists (doc 06:584 itself admits "nothing to check yet"); and the prelude bills itself as "the whole plugin contract in one `use`" while omitting `SeekSupport`, violating the file's own "Keep in sync with prelude" instruction one line above the divergence.

## 2. What actually works (verified)
— verified by `cargo test -q -p plugin-api` -> 42+6+2 passed (includes `prelude_reexports_the_whole_contract`: `use super::*` compiles, api==1). Scope-fence claims verified by rg: no registry/selection code in this crate; WASM host is in crates/plugin-host-wasm; the constant's runtime consumers (imports.rs:277, api.rs:98) are all in plugin-host-wasm.

## 3. Findings
F-001 [S3]
Where: crates/plugin-api/src/lib.rs:119-125
Code:
```
/// Builtins are compiled against it by construction; CI checks every
/// `plugins/builtin/*.toml` carries exactly this value (§2.2: a
/// mismatch is a build break, never a runtime surprise).
pub const ENGINE_PLUGIN_API_MAJOR: u32 = 1;
```
What is wrong: `plugins/builtin/` does not exist (`ls plugins/` -> `extensions` only), and rg shows no CI/xtask gate referencing it. docs/06-plugin-architecture.md:584 states it plainly: "runner today (nothing to check yet: `plugins/builtin/` does not exist)". The comment asserts an active CI safety net that is vapor. Why it matters: a reader trusts that api_version drift is build-caught; nothing catches it. What correct looks like: "no builtin manifests exist yet; the check lands with plugins/builtin/ (doc 06 §2.2)". Confidence: HIGH — Verification: ran ls + rg.

F-002 [S4]
Where: crates/plugin-api/src/lib.rs:86-90 vs 103-106
Code:
```
// Headline re-exports — flat access for the contract surface every
// plugin crate speaks. Keep in sync with `prelude` below.
pub use access::{Access, SeekOrigin, SourcePosition, SourceStream};
```
vs prelude L106: `pub use crate::demux::{DemuxEvent, Demuxer, PacketSink};` — no `SeekSupport`.
What is wrong: Top level re-exports `SeekSupport` (L90); the prelude does not. The file's own comment demands sync; the prelude doc says "the whole plugin contract in one `use`" (L102) — false. `SeekSupport` is a live contract type (consumed by apps/web/src/open.rs:358, apps/cli containers). Also absent from prelude: `ManifestError`, `PackageHash`, `Signature`. Confidence: HIGH — Verification: read (diff of the two lists).

F-003 [S4]
Where: crates/plugin-api/src/lib.rs:131-137
Code:
```
    fn prelude_reexports_the_whole_contract() {
        // One `use` compiles the whole v1 surface (doc 06 §1.2 L1 rows).
        let _: PluginKind = PluginKind::Demuxer;
        let api: u32 = ENGINE_PLUGIN_API_MAJOR;
```
What is wrong: Name promises the whole contract; body touches 2 items. Mitigation: `use super::*` (L129) does force every re-export path to resolve at compile time, so broken paths would fail the build — but the "test" proves nothing beyond compilation of two bindings. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module decls (12) | 73-84 | REAL | – | all 12 files exist (kind, semver, manifest, access, demux, codec, filter, subtitle, playlist, metadata, ui, discovery) |
| headline re-exports (35 items) | 88-100 | REAL | S4 | includes dead `PacketSink` (see demux.rs audit); SeekSupport present here |
| prelude (34 items) | 103-117 | REAL | S4 | F-002: missing SeekSupport vs top level |
| ENGINE_PLUGIN_API_MAJOR | 125 | REAL (const) | S3 | F-001: CI claim false; consumers all in unshipped plugin-host-wasm |
| tests::prelude_reexports_the_whole_contract | 131-137 | SHALLOW | S4 | F-003 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| prelude_reexports_the_whole_contract | Only for missing/misnamed re-export paths (compile) or api≠1 | assert set is 2 items; nothing behavioral |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "CI checks every `plugins/builtin/*.toml` carries exactly this value" | lib.rs:122-124 | FALSE | plugins/builtin absent (ls); no gate in rg; doc 06:584 concedes |
| "the whole plugin contract in one `use`" (prelude) | lib.rs:102 | FALSE | SeekSupport/ManifestError/PackageHash not in prelude |
| Scope fence: no registry, no WASM host, no enforcement, no conformance kit in this crate | lib.rs:14-25 | REAL | rg: registry/host/enforcement live elsewhere; kit is plugin-host-wasm/tests |
| "sinks are created and consumed on the caller's thread and deliberately are not Send" | lib.rs:39-40 | REAL (demux.rs L169-171 consistent) | read; PacketSink has no Send bound — though the trait is dead (demux.rs F-001) |

## 7. Dead / duplicated / unreachable code
The root re-exports the dead `PacketSink` (demux.rs audit F-001) — dead API kept alive at the crate surface and in the prelude. `ENGINE_PLUGIN_API_MAJOR`'s only runtime consumers are in unshipped plugin-host-wasm.

## 8. Dependencies & flags
media-types + codec-api path deps; serde/toml/thiserror external (manifest module); dev-dep testkit. No features. No unsafe. `[lints] workspace = true`.

## 9. Open questions for the next agent
- kind.rs / semver.rs / access.rs / filter.rs / subtitle.rs / playlist.rs / metadata.rs / ui.rs / discovery.rs are NOT in my assignment — my domain evidence says only `access` + `demux` types are invoked by shipped code; whoever takes those files should check each host trait for live implementors (the `ui`/`metadata`/`discovery` traits risk the same dead-seam fate as demux::PacketSink — plugin-host-wasm's imports.rs implements the WASM side, but nothing in a shipped app calls them).
