# AUDIT: crates/playback-engine/src/tracks.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2025-06-14 |
| Lines / functions | 593 / 20 (14 prod + 6 tests, incl. impls) |
| File verdict | REAL — selector vocabulary, typed lane model, and switch step-1/3/6 cores, all wired into session.rs; 6 unit tests executed and passing |
| Reachable from shipped entry point? | SPLIT. `default_track` YES: pipeline.rs:1064 uses it at open-time auto-selection (every shipped open path). `resolve_selector`/`pipeline_switch`/`complete_switch` YES at library level (session.rs:1829/2162/2695 — the `SessionHandle::set_audio_track`/`set_video_track` command path) but NO shipped app serves `tracks.set-*` (host.rs audit F-001: rg over apps/ = 0 matches) — the switch flow is engine-reachable, shipped-host-unreachable. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `cargo test -p playback-engine --lib tracks::` -> 6 passed, exit 0; rg caller sweeps (session.rs/pipeline.rs citations above) |

## 1. One-paragraph verdict
Clean, typed, well-tested policy module. The `WiredKind`/`UnwiredKind` pair is the fix for a real prior defect (a `MediaKind`-keyed `Option` lookup with a production `expect`, self-documented at lines 141-154) and pushes the lane invariant into the type system; `resolve_selector` implements the documented accept-before-mint asymmetry (kind-mismatch and unknown ids reject pre-table with `TrackNotFound`); `complete_switch` anchors the audio clock on the new track's first delivered frame pts rather than the freeze position. Unit tests cover every selector arm, the wire round-trip, and the typed refusal, and pass (my run). The one structural problem is inherited from the suite around it, not from this code: the seven-step switch flow this module implements is driven by commands no shipped host serves, so the mid-session switching machinery is production-quality code with no production trigger. The subtitle seam's stubs are honestly declared (no demuxer emits subtitle tracks, no subtitle decoder exists — lines 103-125).

## 2. What actually works (verified)
- verified by `cargo test -p playback-engine --lib tracks::` -> 6 passed, exit 0 (id/kind resolution, Auto default-then-first, flag-preference, wire round-trip, typed unwired refusal, widening).
- verified by rg: session.rs:88-89 imports the switch cores; session.rs:1829 (resolve), 2162 (pipeline_switch), 2695 (complete_switch), 2250 (subtitle seam) consume them; pipeline.rs:1064 consumes `default_track` at open.

## 3. Findings
F-001 [S2]
Where: crates/playback-engine/src/tracks.rs:5-15 (module docs) + the shipped-host gap
Code:
```
//! `tracks.set-video-track` / `tracks.set-audio-track` (class A) mint
//! a pipeline generation exactly like a seek: the switched-away
//! track's resident packets and any decoder-buffered frames are stale
```
What is wrong: the machinery this module documents (generation-minting mid-session track switches) is correct and tested, but the wire commands that trigger it (`tracks.set-audio-track`/`set-video-track`) are served by NO shipped host — apps/cli/src/dispatch.rs:75-157 routes them to `unserved` ("track switching is card 3.A3"); rg over apps/ finds zero `TracksSet*` handlers. The doc speaks of these commands as live; in every shipped binary they are a typed refusal. (The scenarios_b local host serving them — host.rs audit F-001 — is a test-only superset.)
Why it matters: the module's doc-level contract and the shipped product disagree; mid-session switching is engine/library capability only.
Confidence: HIGH — Verification: rg sweeps + CLI route read (host.rs audit F-001 evidence).

F-002 [S4]
Where: crates/playback-engine/src/tracks.rs:240-245
Code:
```
    /// Apply the product's track-selection rules. At switch time this
    /// re-resolves to the container-default track of the kind (the
    /// `default_on` flag, else first-of-kind in stream order); the
    /// full product.toml rules engine is a `[PROPOSAL]` deferred to
```
What is wrong: `Auto` is a simplified rule (default flag, else first) — the full product.toml rules engine is a deferred `[PROPOSAL]`. Honest, but the shipped "auto" behavior is a placeholder policy.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/playback-engine/src/tracks.rs:432, 441, 572, 577
Code:
```
            codec: CodecDescriptor::new("fake").unwrap_or_else(|e| unreachable!("{e}")),
```
What is wrong: four `unreachable!`s in test helpers (tree-wide pattern; test-only blast radius).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| WiredKind (enum) | 155-163 | REAL | — | audio/video/subtitle lanes |
| WiredKind::as_media_kind | 170-177 | REAL | — | total widening, tested |
| UnwiredKind + Display + Error | 184-205 | REAL | — | typed refusal for lane-less kinds |
| TryFrom<MediaKind> | 207-226 | REAL | — | the single conversion point (audit F-1 fix); exhaustive match — future variant = compile error |
| TrackSelector (enum) | 231-246 | REAL | S4 | Auto = simplified policy (F-002) |
| TrackSelector::as_wire / from_wire | 250-272 | REAL | — | round-trip tested |
| TrackSelectionError + Display | 277-296 | REAL | — | NotFound{id} only — matches doc 05 §2.5 not-found family |
| ResolvedSelection (enum) | 299-305 | REAL | — | Track/Deselect |
| default_track | 311-317 | REAL | — | default_on flag, else first-of-kind; used at shipped open (pipeline.rs:1064) |
| resolve_selector | 325-341 | REAL | — | Id must match kind (cross-kind = NotFound); None deselect; Auto re-resolve; tested all arms |
| InFlightTrackSwitch (struct) | 345-357 | REAL | — | kind/selected/generation/origin |
| SwitchOutcome (struct) | 361-371 | REAL | — | stale_frames_flushed/decoder_reused/decoder_built |
| pipeline_switch | 384-398 | REAL | — | thin delegate to Pipeline::switch_limbs (prepare-before-commit lives in pipeline.rs) |
| complete_switch | 409-419 | REAL | — | audio-only re-anchor on first-frame pts + set_generation; None anchor = publish generation only (the no-frame case, session.rs:2149 context) |
| tests::track / list | 426-454 | REAL | — | fixture builders |
| tests::id_resolves_only_within_the_requested_kind | 456-496 | REAL | — | would fail on cross-kind/unknown regressions |
| tests::none_deselects_and_auto_takes_the_default_then_the_first | 498-520 | REAL | — | covers Auto fallback + honest deselect |
| tests::auto_prefers_the_flag_even_when_it_is_not_first | 522-532 | REAL | — | flag-vs-order precedence |
| tests::selector_wire_round_trip | 534-552 | REAL | — | vocabularies can't drift |
| tests::wired_kinds_widen... / data_is_the_typed_unwired_kind | 556-592 | REAL | — | the typed-refusal contract pinned |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 6 unit tests | YES | direct assertions on resolution/round-trip/typed-refusal outcomes |
| (integration) tests/track_switch.rs, scenarios_b scenario 08 | YES (verified pass) | cover the full seven-step flow on synthetic and real fixtures — but under test-only budgets (host.rs audit F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a track switch is a PIPELINE GENERATION" (mint at accept) | tracks.rs:5-15 | REAL | session.rs switch path mints; scenarios_b scenario_08 asserts generation + no old-track frames (executed pass) |
| "an unresolvable track selector ... rejects before the mint" | tracks.rs:30-41 | REAL | resolve_selector returns Err pre-table; unit test pins it |
| subtitle seam stubs (no demuxer emits, no decoder) | tracks.rs:103-125 | REAL (honest stub) | rg: codecs crate has mjpeg+pcm only; demux AVI-only in this crate's composition |
| "the full product.toml rules engine is a [PROPOSAL]" | tracks.rs:242-244 | REAL (honest placeholder) | F-002 |

## 7. Dead / duplicated / unreachable code
- No dead code in-module. The switch cores are consumed by session.rs; `default_track` by both open and switch paths.
- Shipped-unreachable: the entire switch command flow (F-001) — library-reachable only.

## 8. Dependencies & flags
- Deps: commands (wire types), media-clock (reanchor/NANO_BASE), media-types, crate::pipeline. No features. Compiled into every shipped build (src module).

## 9. Open questions for the next agent
- pipeline.rs `switch_limbs` (the real prepare-before-commit logic, flush budget semantics) is in another agent's domain — the retire-order (flush queue → drain decoder through generation-checking sink → park instance) is where stale-frame bugs would live; tests/track_switch.rs is the evidence.
- session.rs:2149's comment ("complete_switch only publishes the generation") suggests a no-frame completion path — worth checking its terminal semantics (TrackSelectionChanged with no delivered frame) in the session audit.
