# AUDIT: crates/transcode/src/graph.rs
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-23 |
| Lines / functions | 667 / 10 production items (trait, 4 enums/structs w/ ctors, validate, 4 Debug impls) + 13 test items |
| File verdict | REAL (validation-only module, honest typed refusals) — but the CRATE is UNREACHABLE |
| Reachable from shipped entry point? | NO — zero consumers of the `transcode` crate anywhere (proof below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -l 'transcode::' apps crates tests --glob '!crates/transcode/**'` → exit 1 (no matches); `rg -n 'transcode' --glob 'Cargo.toml' .` → only crates/transcode/Cargo.toml itself; baseline citation |

## 1. One-paragraph verdict
graph.rs is the typed data model + shape validator for transcode jobs: a `TranscodeGraph` (source/decode/filters/sink) assembled by "a host" and validated before any thread spawns. The validation logic is real, ordered, and pinned by exact-error-code tests; `FilterStage` is the honest kind of non-capability — modeled in the type, refused at validation (`filter_unimplemented()`), never silently dropped. Nothing here is faked. The fatal problem is one level up: NO crate, app, or test in the repository depends on the `transcode` crate — not apps/cli, not apps/desktop, not apps/web, not fuzz, not any other crate. `rg -l 'transcode::' apps crates tests --glob '!crates/transcode/**'` returns nothing (exit 1). The whole crate — this graph, the runner, the WAV and MP4 muxers — is an island: real code, real tests, zero shipped entry points. The doc's "plain data a host assembles" (L13) describes a host that does not exist.

## 2. What actually works (verified)
— Baseline audit/runs/baseline/cargo-test-workspace.txt (PACKAGE transcode): `test graph::tests::...` all 7 ok, `test result: ok. 50 passed; 0 failed` for the lib. Cross-checked runtime claims: the doc's deferred checks really are implemented in the runner — widening/narrowing refusal at runner.rs:897-902 (`resolve_target(format.sample, &self.target)?`, "legal widening — the honest conversion matrix"), h264/aac refusal noted at runner.rs:726 and mp4mux.rs:505-510.

## 3. Findings
F-001 [S3]
Where: crates/transcode/src/graph.rs:13-14 (and the crate at large)
Code:
```
//! The graph is **plain data a host assembles**, then hands to
//! [`submit`](crate::submit). Two shape families are legal:
```
What is wrong: no host exists. Proof: `rg -n 'transcode' --glob 'Cargo.toml' .` matches only crates/transcode/Cargo.toml; `rg -l 'transcode::' apps crates tests --glob '!crates/transcode/**'` → exit 1 (zero callers). Why it matters: 50 passing tests and a clean clippy lane give the appearance of a working transcode subsystem in the product; in reality none of it is compiled into lumen-cli or desktop, so no user can reach `validate`, `submit`, or any sink. What correct looks like: either a shipped integration (CLI subcommand / desktop export) or an explicit crate-level doc line "not yet wired into any host". Confidence: HIGH — Verification: ran both rg commands.

F-002 [S4]
Where: crates/transcode/src/graph.rs:390-395
Code:
```
    struct NopFactory;
    impl DecoderFactory for NopFactory {
        fn open(&self, _track: &Track) -> Result<Box<dyn Decoder>, EngineError> {
            Err(graph_decode_required())
        }
    }
```
What is wrong: the do-nothing factory's `open` returns `graph_decode_required()` — a graph-shape error code reused as filler for an unrelated condition. The factory is never called during validation (documented L388-389 "the graph never calls it during validation — shape only"), so today it is harmless, but the error identity is a lie if any future test asserts on it. Test-only. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory, >100 LOC)
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| WriteSeek (trait + blanket impl) | 77-78 | REAL | — | Write+Seek+Send; RIFF size-patching rationale documented |
| TrackSelection (enum) | 81-95 | REAL | — | DefaultAudio/Track/Tracks; invariants documented per variant |
| DecoderFactory::open | 103-110 | REAL | — | host seam; errors typed |
| DecodeStage::{new,into_factory} | 127-137 | REAL | — | into_factory pub(crate), moved to worker |
| DecodeStage::Debug | 119-125 | REAL | — | hides host closure |
| FilterStage / FilterDomain | 140-159 | REAL | — | modeled + refused — honest |
| PcmFacts | 161-173 | REAL | — | channels/rate validated at writer build (runner), documented |
| WavPcmTarget | 176-187 | REAL | — | Source/Sample; narrowing refused at runtime (runner.rs:897-902 verified) |
| SinkSpec (4 variants) | 189-236 | REAL | — | facts Option = typed refusal; Mp4Remux config-is-selection rule documented |
| SinkSpec::Debug | 238-252 | REAL | — | shape only |
| SourceSpec + Debug | 256-278 | REAL | — | extra_data plumbing for out-of-band config |
| TranscodeGraph::validate | 317-379 | REAL | — | order: filter budget → any filter → sink shape (per-sink rules) → selection mismatch; all typed, all tested |
| TranscodeGraph::Debug | 295-304 | REAL | — | — |
| tests: NopFactory/NopDemuxer/SeekableSink/helpers | 388-452, 567-575 | REAL | S4 | NopFactory error identity (F-002) |
| 7 tests | 454-666 | REAL | — | exact error codes asserted; ordering pinned |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| wav_pcm_requires_decode_and_copy_sinks_do_not | YES | asserts `transcode.graph.decode-required` code + clean pass |
| copy_sinks_reject_decode | YES | both copy sinks, exact code |
| wav_copy_without_facts_is_refused | YES | exact code |
| filter_rows_refuse_over_budget_then_unimplemented | YES | pins ordering: 5 rows → over-budget, 1 row → unimplemented |
| legal_graphs_validate_clean | YES | all four legal shapes must pass |
| mp4_remux_rejects_decode_empty_duplicate_and_over_budget_configs | YES | four refusals with exact codes incl. budget-before-entry-walk order |
| multi_track_selection_is_remux_only | YES | mismatch code + legal remux pass |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "typed stages, validated before any thread spawns" | graph.rs:1-2 | REAL | validate is pure; runner calls it (runner.rs:137 submit path) |
| "No other remux exists in v1 — every conversion outside the families above is a typed ErrorKind::Unsupported" | graph.rs:28-30 | REAL | SinkSpec has exactly 4 variants; refusals tested |
| "[FilterStage] exists in the model ... and validation refuses every one of them" | graph.rs:32-34 | REAL | L324-326 `if !self.filters.is_empty() { return Err(filter_unimplemented()); }` |
| "Track-level legality ... is runtime knowledge: checked right after Demuxer::start" | graph.rs:48-51 | REAL | runner.rs:726 (h264/aac), 897-902 (widening) verified |
| "plain data a host assembles" | graph.rs:13 | UNREACHABLE | no host exists — F-001 |

## 7. Dead / duplicated / unreachable code
The crate itself is unreachable (F-001). Within the file: no dead code; `into_factory` used by runner (crate-internal).

## 8. Dependencies & flags
codec_api, demux (wav::format::PcmSample), media_types, plugin_api — all workspace-internal. No features, no unsafe, no unwrap/expect (matches workspace lints). Crate Cargo.toml not in my assignment; dependency list seen only via `use` statements (UNVERIFIED beyond that).

## 9. Open questions for the next agent
- Is there a roadmap doc promising "transcode" as a shipped v0 feature? If a doc/matrix row cites transcode as working product capability, F-001 escalates to S0 for that row (evidence exists only as unreachable crate tests).
- runner.rs/job.rs/record.rs audits (other agents) should confirm the Remuxer path actually derives selection from configs as graph.rs:221-229 documents.
