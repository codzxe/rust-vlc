# AUDIT: crates/transcode/src/graph.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2026-11-21 |
| Lines / functions | 667 / 30 (16 production items: 1 trait+blanket impl, 3 enums, 4 structs, 2 pub traits, 4 Debug impls, 2 inherent impls w/ 3 methods incl. `validate`; 14 test items) |
| File verdict | UNREACHABLE (real, tested model code — but no shipped entry point links the crate) |
| Reachable from shipped entry point? | NO. Proof: `rg -n "transcode" apps/*/Cargo.toml apps/*/src` → 0 dependency edges, 0 code references; `rg -l "lumen-transcode\|transcode::" crates apps` → only crates/transcode/{src,tests} itself. `lumen` (apps/cli) and `lumen-tauri` (apps/desktop-tauri) never depend on the crate; it only compiles as a workspace member library (baseline #1 `Compiling transcode v0.1.0`, build-release-workspace.log.txt). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "lumen-transcode\|transcode::" crates apps` → self-references only (unreachability proof); `rg -n "transcode" apps --glob '*.toml'` → none; `rg -n "convert\|remux\|record\|transcode" apps/cli/src` → no CLI subcommand wires it; `sed -n '317,380p'` etc. → quote verification; `rg -n "max_filter_stages\|max_tracks" crates/transcode/src/limits.rs` → defaults 4/64 confirm test comments; `rg -n "deps/transcode" audit/runs/baseline/test-workspace.log.txt` → ABSENT (fail-fast stopped before it); `sed -n '5812,5890p' audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → 50/50 unit tests pass incl. all 7 `graph::tests` |

## 1. One-paragraph verdict
The graph model is honest, disciplined code: a pure, synchronous `validate` that enforces exactly the documented validation order (filter budget → filter refusal → sink/decode shape → MP4-config emptiness/budget/duplicates/sample-entry ceiling → selection/sink mismatch), typed refusals for every unsupported shape, and zero fakeness — no todo!, no unwrap, no invented facts (WAV-copy `fmt ` facts are demanded from the host, not guessed). The in-tree tests are real mutation killers pinning exact error codes, and they ran green in the orchestrator's no-fail-fast repair run. The problem is placement, not content: the crate is a phase-7 card that NO shipped binary links — `lumen` and `lumen-tauri` depend on neither the crate nor any crate that re-exports it — so this entire validated pipeline is dead code from the user's perspective, and the feature matrix's `not_started` rows for record/convert are the only truthful mirror of that.

## 2. What actually works (verified)
— `graph::tests` executed and passing: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5825-5832` — `copy_sinks_reject_decode … ok`, `filter_rows_refuse_over_budget_then_unimplemented … ok`, `legal_graphs_validate_clean … ok`, `multi_track_selection_is_remux_only … ok`, `wav_copy_without_facts_is_refused … ok`, `mp4_remux_rejects_decode_empty_duplicate_and_over_budget_configs … ok`, `wav_pcm_requires_decode_and_copy_sinks_do_not … ok`; suite result `50 passed; 0 failed` (line 5854).
— Validation order matches the module-doc pin (36-46 vs code 317-377): verified by reading both; the budget-before-unimplemented and mismatch-last orderings are each asserted by a distinct test.
— Unreachability proof (negative result, verified): grep evidence in the header row above.

## 3. Findings

### F-001 [S3] Whole module is dead code in every shipped artifact — no host ever assembles this graph
Where: crates/transcode/src/graph.rs:283-293 (and the whole 667-line file)
Code:
```rust
pub struct TranscodeGraph {
    /// Source/demux stage.
    pub source: SourceSpec,
    /// Decode stage (`None` on copy paths).
    pub decode: Option<DecodeStage>,
```
What is wrong: `TranscodeGraph`/`submit` have no caller outside the crate's own tests. No app depends on `transcode`; no CLI subcommand (`convert`, `record`, `remux`) exists in apps/cli/src; the tauri shell does not import it.
Why it matters: ~26 KB of validated pipeline model ships in the workspace but is unreachable by any user action; it will silently bit-rot while docs (crate docs "Honest capabilities (v1)") describe a working transcode surface. The feature matrix itself says `playback-record`/`ui-dialog-convert` = `not_started` on all platforms with empty evidence strings — the code's existence is not user-visible capability.
What correct looks like: either a shipped host wiring (a `lumen convert`/record command or a tauri invoke) or an explicit crate-level doc banner "not wired into any shipped host (phase 7 card)" so no reader mistakes it for capability.
Confidence: HIGH — Verification: rg proofs above (ran), baseline build log (compiles as member only).

### F-002 [S3] `validate` blesses a contradictory MP4-remux graph: `source.selection` is silently discarded, never checked
Where: crates/transcode/src/graph.rs:370-377
Code:
```rust
        // A multi-track selection feeds the MP4-remux sink (whose
        // configs derive it); every other sink consumes exactly one
        // track and must not silently drop the rest.
        if matches!(self.source.selection, TrackSelection::Tracks(_))
            && !matches!(self.sink, SinkSpec::Mp4Remux { .. })
        {
            return Err(graph_selection_tracks_mismatch());
```
What is wrong: the mismatch check is one-directional. A hand-assembled graph with `SinkSpec::Mp4Remux` and `source.selection = DefaultAudio` or `Track(x)` — all fields are `pub`, direct construction is the documented assembly style (lib.rs doc example lines 77-89) — passes `validate` clean; `submit` (runner.rs:158-169) then silently replaces the selection with one derived from `configs`. The one parameter a host sets is ignored without a typed refusal, the exact "silently drop" sin the comment decries.
Why it matters: the crate's whole design vocabulary is "typed refusal, never silent"; this is the one seam where a public API silently discards host input. A host that named a track and later reorders configs gets a different output with no error.
What correct looks like: `validate` should refuse `Mp4Remux` combined with any explicit `selection` other than the config-derived one (or document the field as ignored-and-asserted), keeping `Remuxer` the single assembly path by construction rather than by comment.
Confidence: HIGH — Verification: read graph.rs:370-377 + runner.rs:158-169; ran rg for `graph_selection_tracks_mismatch` (only this site + error.rs constructor).

### F-003 [S4] Doc comment claims duplicate `Tracks` ids are "refused at graph validation" — no such check exists
Where: crates/transcode/src/graph.rs:90-94
Code:
```rust
    /// exist in the source's track table; duplicates are refused at
    /// graph validation (the sink's config list is the de-duplicated
    /// authority — `submit` derives this selection from it).
    Tracks(Vec<TrackId>),
```
What is wrong: `validate` never inspects `TrackSelection::Tracks` for duplicates — the duplicate check at 356-360 runs over `Mp4TrackConfig`s only. A `Tracks([id, id])` selection on an Mp4Remux sink passes validation untouched (and is then discarded per F-002, which is the only reason it cannot misbehave).
Why it matters: a reader of the type doc will believe a validation guarantee that lives one branch away and in a different type.
What correct looks like: either reject duplicate ids in `TrackSelection::Tracks` at validation, or reword the doc to say duplicates are prevented *indirectly* because `submit` derives the selection from the config list.
Confidence: HIGH — Verification: read the whole `validate` body (317-380); no Tracks-duplicate branch exists.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `trait WriteSeek` + blanket impl | 77-78 | REAL | — | Marker alias `Write+Seek+Send`; std-only; used by SinkSpec outputs & test sinks. |
| `enum TrackSelection` (3 variants) | 81-95 | REAL | — | Plain data; `Tracks` dup-doc drift → F-003. |
| `trait DecoderFactory::open` | 103-110 | REAL | — | Host seam; never called during validation (comment 388-389 verified true). |
| `struct DecodeStage` + `new` + `into_factory` | 115-138 | REAL | — | `into_factory` pub(crate), consumed by runner. |
| `impl Debug for DecodeStage` | 119-125 | REAL | — | Fixed string "DecodeStage(factory)"; honest elision. |
| `struct FilterStage` | 144-150 | REAL | — | Modeled-then-refused filter row; no fake filter exists. |
| `enum FilterDomain` | 153-159 | REAL | — | Audio/Video; Video domain unusable in v1 (no video sink) — harmless vocabulary. |
| `struct PcmFacts` | 165-173 | REAL | — | channels/rate validation deferred to wav.rs writer build (doc says so; wav.rs audit to confirm). |
| `enum WavPcmTarget` | 176-187 | REAL | — | Widening-only rule enforced downstream (convert.rs), not here — correct: needs runtime sample format. |
| `enum SinkSpec` (4 variants) | 190-236 | REAL | — | All four carry `Box<dyn WriteSeek>`; docs are precise about legality conditions. |
| `impl Debug for SinkSpec` | 238-252 | REAL | — | Shape-only Debug, no stream guts; correct. |
| `struct SourceSpec` | 256-267 | REAL | — | `extra_data: Option<SharedBytes>` handed to decoder at configure time (runner). |
| `impl Debug for SourceSpec` | 269-278 | REAL | — | finish_non_exhaustive; honest. |
| `struct TranscodeGraph` | 283-293 | REAL (unreachable) | S3 | F-001. |
| `impl Debug for TranscodeGraph` | 295-304 | REAL | — | Delegates; fine. |
| `TranscodeGraph::validate` | 317-380 | REAL | S3 | Pure, no I/O, order matches docs; F-002 one-directional selection check; HashSet dedup O(n); no panics (no indexing/unwrap). |
| test `NopFactory` | 390-395 | REAL | — | `open` returns `graph_decode_required()` — misleading code reuse but never invoked during validation. |
| test `NopDemuxer` | 398-409 | REAL | — | Minimal plugin-api impl; shape only. |
| test `SeekableSink` | 413-427 | REAL | — | Write+Seek discard sink; used by all sink fixtures. |
| test helpers `sink`/`graph`/`legal_copy_graph` | 429-452 | REAL | — | Fixture builders; no assertions. |
| test `wav_pcm_requires_decode_and_copy_sinks_do_not` | 454-471 | REAL | — | Pins `transcode.graph.decode-required`; fails if the check is deleted. |
| test `copy_sinks_reject_decode` | 473-496 | REAL | — | Pins `decode-forbidden` for RawEsCopy and WavCopyPcm. |
| test `wav_copy_without_facts_is_refused` | 498-509 | REAL | — | Pins `copy-facts-missing`. |
| test `filter_rows_refuse_over_budget_then_unimplemented` | 511-536 | REAL | — | Pins budget(4)-before-unimplemented order; defaults verified in limits.rs:75-76. |
| test `legal_graphs_validate_clean` | 538-563 | REAL | — | Positive path for all sink families; `.must()` on Ok. |
| test `mp4_sink` helper | 567-575 | REAL | — | 86-byte sample entry (h264-sized). |
| test `mp4_remux_rejects_decode_empty_duplicate_and_over_budget_configs` | 577-646 | REAL | — | Pins 4 refusal codes incl. 65-config over-budget (default 64) and MAX_SAMPLE_ENTRY_BYTES+1 (64 KiB ceiling, mp4mux.rs:111). |
| test `multi_track_selection_is_remux_only` | 648-666 | REAL | — | Pins `selection-tracks-mismatch` both directions of the one check that exists. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| wav_pcm_requires_decode_and_copy_sinks_do_not | YES | Deleting the `decode_required` branch turns the asserted `assert_err` into an Ok → panic. |
| copy_sinks_reject_decode | YES | Exact code `transcode.graph.decode-forbidden` asserted on two sinks. |
| wav_copy_without_facts_is_refused | YES | `facts: None` must error with exact code. |
| filter_rows_refuse_over_budget_then_unimplemented | YES | Asserts two different codes for 5 rows vs 1 row — catches both a missing budget check and order swaps. |
| legal_graphs_validate_clean | YES (weakly) | Positive-path `.must()` catches over-refusal (e.g. rejecting legal copy graphs). |
| mp4_remux_rejects_decode_empty_duplicate_and_over_budget_configs | YES | Four independent refusal codes pinned, incl. boundary 64/65 and MAX_SAMPLE_ENTRY_BYTES±1. |
| multi_track_selection_is_remux_only | YES | Both mismatch and remux-accepts-Tracks pinned. |
Mutation reasoning done on paper only (no edits) — but all 7 ran green in `orchrun-test-workspace-nofailfast.log.txt:5825-5832`.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Stream recording: record-to-WAV … + MP4 remux" status not_started, evidence empty | docs/feature-matrix.toml `playback-record` (~line 1633) | REAL (honest: not shipped) | Crate exists (this file) but unlinked — rg proof in header; matrix correctly does NOT claim capability. |
| "Conversion UI: WAV (PCM encode) + MP4 remux … ONLY" not_started | `ui-dialog-convert` (~line 2916) | REAL (honest) | Same unreachability proof. |
| "Validation order (pinned by tests)" filter-budget→filter-refusal→sink-shape→facts | graph.rs module docs 36-46 | REAL | Code 317-377 matches order; tests pin each step. |
| "duplicates are refused at graph validation" (Tracks) | graph.rs:90-93 | FALSE (doc-level) | F-003 — no Tracks-duplicate check exists. |
| "WAV finalization rewrites RIFF/data size fields, so a write-only pipe cannot host an honestly-sized file" | graph.rs:70-76 | REAL-UNPROVEN here | Finalization lives in wav.rs (this agent's file 3 — verified there: `finalize`/patch logic confirmed). |

## 7. Dead / duplicated / unreachable code
Everything is unreachable from shipped binaries (F-001) though internally used by `submit`/`record`/tests. No duplication; no dead items inside the crate (`into_factory` consumed by runner.rs; all SinkSpec variants consumed by runner plans).

## 8. Dependencies & flags
No feature flags — crate is unconditional, std-only (Cargo.toml:55-59: codec-api, demux, media-types, plugin-api; dev: codecs, input, testkit). No cfg-gating to hide behind. Workspace lints inherited (`[lints] workspace = true`), and clippy -D warnings passed clean in baseline #5/#6 — consistent with zero unwrap/expect in this file (verified by reading).

## 9. Open questions for the next agent
- runner.rs owner: confirm `submit`'s selection override (158-169) refuses a `Tracks` selection that disagrees with the Mp4Remux config list (the graph only catches the non-remux direction).
- record.rs owner: `Remuxer` is the documented single-assembly path — check it cannot construct the contradictory graph of F-002.
- Whoever owns apps: the phase-7 wiring debt (record/convert) is the gap between this crate and the matrix's not_started rows.
