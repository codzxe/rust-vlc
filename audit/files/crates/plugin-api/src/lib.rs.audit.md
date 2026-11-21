# AUDIT: crates/plugin-api/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W12-A10 |
| Date | 2026-11-21 |
| Lines / functions | 138 / 1 (test) + 12 `pub mod` decls + re-export block + prelude + 1 const |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/containers/*.rs` `use plugin_api::{Demuxer, SourceStream, DemuxEvent, SeekSupport}` (mp4.rs:79, avi.rs:17, ts.rs:88, …) → these crate-root paths resolve through this file's `pub use` lines 88–100; `apps/cli/src/demux_host.rs:61`) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | `wc -l` (138 lines); `rg plugin_api` (23 dependent Cargo.tomls, incl. apps/cli, apps/web — re-export surface exercised); `rg ENGINE_PLUGIN_API_MAJOR` (used only by plugin-host-wasm src+tests, never by CI/xtask); `rg "pub enum PluginKind"` + variants in kind.rs (16 — matches doc claim); `sed -n` on lib.rs:119-138, docs/06:583-586, .github/workflows/ci.yml:405-409 (quote verification); `rg "pub trait"` access/demux/subtitle/kind.rs (Send bounds match §Threading claims); workspace Cargo.toml:53-65 (lint deny list); LS plugins/ (NO plugins/builtin/, only plugins/extensions/); baseline orchrun-test-workspace-nofailfast.log.txt:4427-4495 |

## 1. One-paragraph verdict
This is the crate-root facade of the plugin contract: 12 module declarations (all files exist on disk), a headline re-export block, an identical `prelude`, one public constant, one compile-test. It compiles clean in the baseline workspace build (INDEX #1) and its re-export surface is what the shipped `lumen` CLI demux lane actually `use`s — so the file is REAL and reachable, not paper. The one real defect: the doc comment on `ENGINE_PLUGIN_API_MAJOR` asserts a CI enforcement ("CI checks every `plugins/builtin/*.toml` … a mismatch is a build break, never a runtime surprise") that does not exist anywhere in the tree — `plugins/builtin/` does not exist, no workflow or xtask references the constant, the repo's own ci.yml:407 skips "plugin manifests: none of these files exist in the tree yet", and doc 06:583-586 admits the bijection check "has no runner today". Mismatches are in fact a *runtime* typed refusal (plugin-host-wasm `error.rs` `ApiVersion`), not a build break.

## 2. What actually works (verified)
— All 42 plugin-api unit tests + 6 doc06_examples + 2 testkit_wiring green in baseline `orchrun-test-workspace-nofailfast.log.txt:4427-4495` (incl. `tests::prelude_reexports_the_whole_contract ... ok` at :4469).
— Workspace build CLEAN (baseline INDEX #1) → every re-export at lines 88–100/104–116 resolves; facade is not lying about its item list.
— Headline re-exports (88–100) and `prelude` (104–116) carry identical item sets — the "keep in sync" claim at line 86-87 verified by direct comparison.
— Doc claim "sixteen-kind taxonomy": kind.rs:27-75 has exactly 16 variants (Access…ServiceDiscovery); test `sixteen_kinds_in_doc_order` green (baseline :4437).
— Doc claim "no external semver crate": Cargo.toml deps are media-types, codec-api, serde, toml, thiserror only.
— Doc claim "workspace lints deny `panic!`/`unwrap`/`expect`": workspace Cargo.toml:60-62 (`unwrap_used`/`expect_used`/`panic` = "deny").
— Threading doc claim: `SourceStream: Send` / `Access: Send` (access.rs:29,119), `Demuxer: Send` (demux.rs:52), `SubtitleParser: Send` (subtitle.rs:26), sinks deliberately not `Send` (`PacketSink` demux.rs:172, `SubtitleSink` subtitle.rs:68) — exactly as lib.rs:38-40 states.

## 3. Findings
### F-001 [S3]
Where: crates/plugin-api/src/lib.rs:122-124
Code:
```
/// Builtins are compiled against it by construction; CI checks every
/// `plugins/builtin/*.toml` carries exactly this value (§2.2: a
/// mismatch is a build break, never a runtime surprise).
```
What is wrong: The asserted CI check does not exist. `rg ENGINE_PLUGIN_API_MAJOR` over the whole repo hits only plugin-api itself, plugin-host-wasm src/tests, and docs — zero CI workflows, zero xtask tasks. `plugins/builtin/` does not exist (LS: only `plugins/extensions/{ui-panel,metadata-provider}`). The repo's own CI prints the skip at `.github/workflows/ci.yml:407`: "SKIP profiles/, layouts/, plugin manifests: none of these files exist in the tree yet", and docs/06-plugin-architecture.md:583-586 states the bijection check "has no runner today (nothing to check yet: `plugins/builtin/` does not exist)".
Why it matters: The comment advertises an enforcement safety net ("build break, never a runtime surprise") that is fiction today; a maintainer reading the crate root believes builtin-manifest/api_version drift is mechanically prevented. It also misdescribes the real mechanism: a mismatch IS a runtime surprise handled by the typed `HostError` path (plugin-host-wasm/src/error.rs:15, api.rs `negotiate_api_version`), not a build break.
What correct looks like: State the truth: "builtin manifests will be CI-checked against this value when `plugins/builtin/` lands (doc 06 §4, no runner today); at runtime `negotiate_api_version` refuses out-of-window values with a typed error."
Confidence: HIGH — Verification: read (grep + sed on ci.yml:405-409, docs/06:583-586; LS plugins/).

### F-002 [S4]
Where: crates/plugin-api/src/lib.rs:131-137
Code:
```
    fn prelude_reexports_the_whole_contract() {
        // One `use` compiles the whole v1 surface (doc 06 §1.2 L1 rows).
        let _: PluginKind = PluginKind::Demuxer;
        let api: u32 = ENGINE_PLUGIN_API_MAJOR;
        assert_eq!(api, 1, "v1 of the plugin API (doc 06 §5.5)");
    }
```
What is wrong: The body asserts a compile-time constant against the literal `1` (tautology) and touches one enum variant; "the whole contract" is only proven in the sense that `use super::*` (line 129) forces the re-export/prelude items to *resolve*. No behavior is tested.
Why it matters: Test name promises contract coverage the body does not deliver; a reader of the test list overestimates what is guarded.
What correct looks like: Either rename to `prelude_compiles` or add one real assertion per headline type (e.g. construct each trait-object type or check `std::any::type_name` presence) so the name matches the body.
Confidence: HIGH — Verification: read; test green in baseline :4469 proves only compilation.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| module decls (12 `pub mod`) | 73-84 | REAL | — | access, codec, demux, discovery, filter, kind, manifest, metadata, playlist, semver, subtitle, ui — all files exist (LS); all compiled in baseline build #1 |
| headline re-export block | 88-100 | REAL | — | 29 items; exercised by apps/cli (`use plugin_api::{Demuxer, SourceStream, …}` at containers/mp4.rs:79 etc.) and apps/web (staged_source.rs:63) |
| `pub mod prelude` | 103-117 | REAL | — | Item set identical to headline block (compared item-by-item; "keep in sync" holds) |
| `ENGINE_PLUGIN_API_MAJOR: u32 = 1` | 125 | REAL | S3 (doc claim, F-001) | Value consumed by plugin-host-wasm api.rs:98 (runtime window) and tests; CI-consumption claim false |
| `tests::prelude_reexports_the_whole_contract` | 131-137 | SHALLOW | S4 (F-002) | Compile-check + constant tautology; green in baseline :4469 |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `prelude_reexports_the_whole_contract` | Only if the re-export surface stopped compiling (e.g. an item removed from `access`/`demux`/…). | Mutation check: delete `SeekSupport` from demux.rs → `pub use demux::{… SeekSupport}` fails to compile → test fails at compile time. But mutating the constant's value (1→2) fails nothing user-visible; the assert is against the literal it already equals. No behavioral coverage. |
| (cross-file, for context) plugin-api 42 unit + 6 doc06 + 2 testkit tests | Yes (per-module) | Green in baseline orchrun log :4427-4495; per-module audits belong to the agents holding those files. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "sixteen-kind `PluginKind` taxonomy" | lib.rs:1-2 (doc 06 §1.2) | REAL | kind.rs:27-75 = 16 variants; test `sixteen_kinds_in_doc_order` green (baseline :4437) |
| "unknown fields rejected" manifest model | lib.rs:2-4 (doc 06 §2) | REAL | manifest test `unknown_fields_rejected` green (baseline :4455) |
| "no external semver crate — dependency discipline" | lib.rs:46-48 | REAL | plugin-api Cargo.toml deps: media-types, codec-api, serde, toml, thiserror |
| "workspace lints deny `panic!`/`unwrap`/`expect`" | lib.rs:37-38 | REAL | workspace Cargo.toml:60-62 deny list |
| Threading: host traits `Send`, sinks not `Send` | lib.rs:31-40 | REAL | access.rs:29,119; demux.rs:52,172; subtitle.rs:26,68 trait declarations |
| "CI checks every `plugins/builtin/*.toml` carries exactly this value … a mismatch is a build break, never a runtime surprise" | lib.rs:122-124 (doc 06 §2.2, §4) | FALSE | No such CI check (grep ENGINE_PLUGIN_API_MAJOR: zero workflow/xtask hits); `plugins/builtin/` absent; ci.yml:407 skips plugin manifests; doc 06:583-586 admits no runner; real mechanism is runtime typed error (plugin-host-wasm error.rs:15) |
| "Builtins are compiled against it by construction" | lib.rs:122 | PAPER-ONLY | Zero builtin plugins exist to compile against (`plugins/` holds only 2 example extensions) |

## 7. Dead / duplicated / unreachable code
None in this file. Duplication-by-design: headline re-export block vs `prelude` (identical 29 items, lines 88-100 vs 104-116) — deliberate and documented ("Keep in sync with `prelude` below"), verified in sync; a future drift between the two is not mechanically checked (no test diffs them) — hygiene note only, folded into F-002's scope.

## 8. Dependencies & flags
- Crate deps (Cargo.toml): media-types (path), codec-api (path), serde 1 +derive, toml 1.1, thiserror 2.0; dev-dep testkit. Matches the "only pre-audited config family" claim in the Cargo.toml header. No feature flags in this crate.
- Cross-ref: baseline fmt-check FAILED on `crates/plugin-api/tests/testkit_wiring.rs` (INDEX #7) — that file is another agent's assignment; noted here because it is the only red gate touching this crate.
- `[lints] workspace = true` inherited (INDEX: 39/40 crates).

## 9. Open questions for the next agent
- kind.rs / manifest.rs / access.rs / demux.rs … holders: the facade's doc claims I verified only at the declaration/test-name level; per-item behavioral audit is yours.
- plugin-host-wasm holder: confirm the runtime `negotiate_api_version` window (api.rs:97-99) and its `ApiVersion` error are the *only* api_version enforcement (I found no other).
- Whoever audits docs/06 §4: the lib.rs:122-124 FALSE claim vs doc 06:583-586's honest "no runner today" is an internal-doc contradiction worth a row in the doc audit.
