# AUDIT: crates/plugin-api/src/metadata.rs
| Field | Value |
|---|---|
| Auditor | W12-A02 |
| Date | 2026-11-21 |
| Lines / functions | 204 / 8 (2 types + 1 trait with 3 methods + 1 helper fn + 1 test fixture impl + 3 tests) |
| File verdict | UNREACHABLE (a well-formed, self-tested contract that nothing implements and nothing calls) |
| Reachable from shipped entry point? | NO. Repo-wide `impl MetadataProvider` → exactly one: the in-file test fixture `PairProvider` (metadata.rs:108). No caller of `extract`/`lookup`/`provides`/`lookup_not_served` outside the file's own tests. The shipped binaries that link plugin-api (apps/cli, apps/web) use only the demux/access contracts (`rg plugin_api::` in apps/cli/src, apps/web/src → Access/Demuxer/DemuxEvent/SourceStream/SeekSupport/SeekOrigin — zero metadata references). plugin-host-wasm references the KIND (`PluginKind::MetadataProvider`) for limits/manifest vocabulary but never the trait. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 0 |
| Commands run | rg `MetadataProvider\|lookup_not_served` over crates/ apps/ tests/ tools/ → 6 files, all plugin-api/plugin-host-wasm kind/manifest/limits references, zero trait usage; rg `impl MetadataProvider` repo → only metadata.rs:108; rg `plugin_api::` in apps/cli/src + apps/web/src → demux/access only; read crates/media-probe/src/report.rs:262-267 (the "later card" admission); read doc 06 §1.2 row 14 (:67), §2.2 (:218), §2.3(c) (:313-337); read tests/plugin_examples_conformance.rs:187-260 (the example's real extraction model); baseline: `metadata::tests::*` 3/3 ok (orchrun-test-workspace-nofailfast.log.txt:4454-4457). |

## 1. One-paragraph verdict
This is the kind-14 contract as pure paperwork: a clean trait (probe-window extraction, capability-honest `provides`, on-demand lookups), typed error helper, and three exact-value tests that all ran green in baseline — and exactly one implementer in the entire repository, its own test fixture. The documented "builtin home" (media-probe) admits in its own code that metadata extraction "is a later card; v0 reports the format, the confidence and the track hints only" (media-probe/src/report.rs:265-267); the real in-tree tag extraction (crates/codecs flac metadata blocks) does not use this trait; and the repo's one real kind-14 artifact — the committed example extension — runs an extraction model the trait forbids (path argument + jailed fs I/O instead of a no-I/O probe-window pull). No shipped binary can reach any of it. The contract is well-written prose about a subsystem that does not exist.

## 2. What actually works (verified)
— Baseline `metadata::tests::{absence_is_data_and_damage_is_error, extraction_reads_pairs, lookup_gates_on_served_kinds} ... ok` (orchrun-test-workspace-nofailfast.log.txt:4454-4457) — the trait is usable and its absence-vs-damage semantics are pinned.
— `lookup_not_served` produces a correctly-shaped `EngineError` (stage Plugin, kind Unsupported) — asserted by `lookup_gates_on_served_kinds` (:198-202).
— The types (`Metadata`, `Tag`) are coherent: absence as `Ok(vec![])` vs structural damage as `Err` (MP §8 discipline), non-empty tag values by convention.

## 3. Findings

### F-001 [S3] Zero implementers, zero callers — the contract is dead surface even inside the plugin ecosystem it defines
Where: crates/plugin-api/src/metadata.rs:56-85 (the trait)
Code:
```rust
pub trait MetadataProvider: Send {
    fn extract(&mut self, probe_window: &SharedBytes) -> Result<Vec<Metadata>, EngineError>;
```
What is wrong: The only `impl MetadataProvider` in the repository is the test fixture `PairProvider` (metadata.rs:108). No engine component, no probe, no host adapter, no CLI/web code calls any trait method. `lookup_not_served` (metadata.rs:88-99) has no non-test caller. plugin-host-wasm knows the KIND (limits, manifest vocabulary, state fixtures) but never adapts a WASM extension onto this trait — the wasm guest ABI (lumen:v1.*) has no metadata surface at all.
Why it matters: The wave question — "is there a real plugin loaded anywhere?" — for kind 14 the answer is that not even the CONTRACT is loaded: the shipped binaries link plugin-api but use only the demux/access contracts, so this module compiles into `lumen` as unreachable weight while doc 06 §1.2 presents it as the kind-14 lane (`builtin + WASM`).
What correct looks like: Either media-probe implements the trait (its documented home), or the host gains a `lumen:v1.metadata` import family + adapter onto this trait, or the module is marked unwired.
Confidence: HIGH — Verification: read + repo-wide rg (commands above).
### F-002 [S3] "Builtin home: media-probe (L2)" is false — media-probe's own code defers metadata extraction to a later card
Where: crates/plugin-api/src/metadata.rs:1-3 (the claim); crates/media-probe/src/report.rs:265-267 (the refutation)
Code:
```rust
//! The metadata-provider contract (doc 06 §1.2 kind 14): tag / cover
//! art extraction at probe time, plus on-demand lookups. Builtin
//! home: `media-probe` (L2); allowed as a WASM extension in v1 (§1.4).
```
What is wrong: media-probe contains no metadata extraction: its `ProbeReport` doc says "Metadata extraction (tags, cover art — the kind-14 plugin surface whose builtin home is this crate) is a later card; v0 reports the format, the confidence and the track hints only" (report.rs:265-267). Doc 06 §1.2 row 14 repeats the claim ("builtin home `media-probe`, L2 … builtin + WASM"). The builtin lane does not exist.
Why it matters: Doc 06's kind table is the architecture's contract inventory; row 14's builtin half is fiction today. The matrix rows for plugins say not_started (feature-matrix.toml:2329), so the shipped-claim story is honest — but this file's and doc 06's "builtin home" wording overstates.
What correct looks like: "(builtin home: media-probe, planned — v0 reports format/confidence/track-hints only)" or the implementation lands.
Confidence: HIGH — Verification: read of both files.
### F-003 [S3] The repo's one real kind-14 artifact contradicts the trait's extraction model — the contract cannot express what the flagship example does
Where: crates/plugin-api/src/metadata.rs:57-65 (the mandate); plugins/extensions/metadata-provider + tests/plugin_examples_conformance.rs:227-246 (the divergence)
Code:
```rust
    /// Extract what this provider can from a probe window (the first
    /// bytes of the media, already read by the engine — the provider
    /// does no I/O here).
```
What is wrong: The committed example extension (`ext.example.metadata`, kind metadata-provider) implements `extract(ptr, len)` where the host writes a MEDIA PATH into guest memory, the guest opens the file through the jailed `fs_open` import (the conformance test asserts the `fs::fs_open` import trace, plugin_examples_conformance.rs:250-256), and returns the sidecar's raw text. That is a path-argument + I/O pull — the exact opposite of "bytes the engine already holds … does no I/O here". The trait has no API for the sidecar-file mode its own flagship demonstrates; doc 06 §2.3(c)'s example (network musicbrainz lookups) matches the trait's `lookup`, but nothing in the tree implements that either.
Why it matters: A contract that its own reference artifact violates is not a contract — any future implementer must guess which model is real (probe-window pull, sidecar fs pull, or network lookup), and the host adapter that would settle it does not exist (F-001).
What correct looks like: The trait grows a source model matching the example (or the example is relabeled a demo of the fs permission, not of kind-14 extraction semantics), and doc 06 §1.2/§2.3 reconciled.
Confidence: HIGH — Verification: read of the trait, the example manifest, and the conformance test body (all quoted/located above).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Metadata` (enum) | 23-36 | REAL | — | Tag/CoverArt{mime,SharedBytes}/Lyrics; zero-copy sharing; coherent. No consumer outside tests. |
| `Tag` (struct) | 38-46 | REAL | — | name/value convention documented; "never empty" is convention, unenforced (no validation — acceptable for a contract type). |
| `MetadataProvider::extract` | 57-65 | REAL | F-001, F-003 | contract only; probe-window/no-I/O mandate contradicted by the example artifact. |
| `MetadataProvider::provides` | 67-70 | REAL | F-001 | "must be a subset of the manifest's provides" — nothing enforces the subset rule (no caller exists). |
| `MetadataProvider::lookup` | 72-84 | REAL | F-001 | on-demand network lookup contract; matches doc 06 §2.3(c) vision; zero implementers. |
| `lookup_not_served` | 87-99 | REAL | F-001 | correctly-shaped typed error; test-only caller. |
| `tests::PairProvider` (fixture) | 106-151 | REAL | — | the repo's sole impl; adequate fixture. |
| `tests::extraction_reads_pairs` / `absence_is_data_and_damage_is_error` / `lookup_gates_on_served_kinds` | 153-203 | REAL | — | exact-value asserts; see §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| extraction_reads_pairs | YES | exact count + name/value asserts through the trait. |
| absence_is_data_and_damage_is_error | YES | pins absence=Ok(vec![]) vs damage=Err — the MP §8 distinction. |
| lookup_gates_on_served_kinds | YES | pins stage/kind of the refusal. |
| (all) | — | Caveat: these test the contract against its own fixture; with zero external implementers they prove nothing about the system — a broken kind-14 subsystem fails no test anywhere. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Builtin home: media-probe (L2); allowed as a WASM extension in v1" | module doc 1-3; doc 06 §1.2 row 14 (:67) | FALSE (builtin half) | media-probe report.rs:265-267 "is a later card; v0 reports the format, the confidence and the track hints only". |
| "kind 14 … builtin + WASM (MP §6)" | doc 06 §1.2 :67 | FALSE (builtin) / REAL-UNPROVEN (WASM) | builtin absent; the WASM example exists and executes (conformance, baseline :4574) but does not use this trait (F-003). |
| "Extraction is a pull from bytes the engine already holds … the provider does no I/O here" | module doc 5-10; extract doc 57-59 | FALSE (as the ecosystem's working model) | the example extension's extract does jailed fs I/O (conformance test :227-256). |
| "network lookups ride the engine's timeout budget … a plugin never opens sockets" | module doc 8-10 | REAL-UNPROVEN | model matches doc 06 §2.3(c) + the host's http allowlist design (imports.rs part1 audit); no implementer exists to prove it. |
| matrix `plugin-builtin-registry`/`plugin-wasm-extensions` = not_started | feature-matrix.toml:2304/:2329 | REAL (shipping view) | consistent with F-001: nothing ships. |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable from every shipped entry point (F-001); it is exported through `lib.rs:97` and the plugin-api prelude (`lib.rs:113`) for consumers that do not exist.
- No duplication in-tree (nothing else models metadata contracts — codecs/flac's real tag parsing is a separate internal surface, not a duplicate of this trait).

## 8. Dependencies & flags
- `media-types` (SharedBytes/EngineError/PipelineStage) + std BTreeMap. No feature gates, no unsafe, no unwrap/expect/panic in production code (tests use `unreachable!` on Err — house test pattern). Dev-dep: none beyond the workspace's.

## 9. Open questions for the next agent
- A07 (kind.rs/manifest.rs owner): the `sources` capability key (manifest.rs:545 lists `["provides", "sources"]` for kind 14) has no representation in this trait — decide where the sources model lives (F-003).
- A01 (example manifests): the metadata example manifest grants `fs-sandboxed-read` for a sidecar pull that doc 06 §2.3(c) does not describe — is the example intentionally a fs-capability demo rather than a kind-14 semantics demo? If so, its "metadata-provider" kind label overstates.
- If a future wave wires media-probe onto this trait, F-001/F-002/F-003 all collapse — re-audit then.
