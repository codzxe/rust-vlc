# AUDIT: crates/playback-engine/src/session.rs (part4, lines 1834–2444 of 3058)
> Chunk scope: part4 = lines 1834–2444 (assignments.tsv LOC: part1 611 /
> part2 611 / part3 611 / **part4 611** / part5 614; 611·3+1=1834 …
> 1834+611−1=2444). Covers: the tail of `apply_command` (whole fn
> 1784–1970 — its head 1784–1833 lies in A01's part3; audited whole
> because the Close-composition macro and the receipt split are
> inseparable; boundary noted), `seek_pipeline_step`,
> `fold_preroll_discards`, `absorb_pipeline_starvation`,
> `switch_pipeline_step`, `subtitle_select_step`, `sync_selected_mirror`,
> `open_step`, and `tick_step`'s head (2428–2444; the signal translation
> and emission policies continue into A04's part5). The `apply()`
> effect interpreter (1544–1738) and the OutputHub are A01's part3 —
> read for context, findings there belong to A01.

| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-21 |
| Lines / functions | 611 (of 3058) / 9 production units |
| File verdict | REAL (open/seek halves executed in the shipped lane; switch/subtitle halves UNREACHABLE from shipped hosts — one S2 swallow on that half) |
| Reachable from shipped entry point? | SPLIT. `open_step`/`seek_pipeline_step`/`tick_step`/`apply_command`(Close) YES, EXECUTED-ADJACENT: `lumen open` → session Open (baseline `mp4-smoke-open-session.txt` `ok: accepted session=1` → `tracks 2 ready` proves open_step's wire path ran; the lifecycle Buffering→Playing→Stopped→Idle proves tick_step); `lumen seek` → dispatch.rs:131 → host.rs:1061 → Seek input → seek_pipeline_step; `close`/`stop` verbs (parse.rs:202/201) drive apply_command's composition. `switch_pipeline_step`/`subtitle_select_step` NO: SelectVideo/SelectAudio/SelectSubtitle have no shipped issuer (CLI verb table parse.rs:197-214; web host rejects `tracks.set-*` at apps/web/src/host.rs:1850-1851; control-server embeds no engine — see pipeline.rs part3 F-003 for the full proof). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | Read tool over 1478-1740, 1784-1970, 1985-2131, 2132-2344, 2339-2453 (full range + interpreter context); `sed -n '2267,2290p;2306,2326p;2379,2386p' session.rs` (quote verification); `rg -n "fn pipeline_switch" crates/playback-engine/src/tracks.rs` + read (wrapper has NO deselect-on-failure); `sed -n '1051,1092p' crates/playback-engine/src/pipeline.rs` (wire's open-time subtitle deselect — the contrast proving F-001); `sed -n '515,560p' crates/playback-engine/src/state.rs` + `rg -n "FreezeClock" state.rs session.rs` (seek row effects: FreezeClock only on the Playing row — seek_pipeline_step's doc claim verified); `sed -n '1645,1738p' session.rs` (effect order: state-changed LAST, SeekFailed inside the effect loop — ordering note §9); `grep -n "session::tests\|first_vertical" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt | head` (engine suites pass; the 7 no-fail-fast failures are desktop/first-vertical/localization, not playback-engine lib) |

## 1. One-paragraph verdict
The command-application half of the session machine is disciplined, spec-cited, and — for the lanes any shipped binary can reach — executed and passing (the new no-fail-fast baseline's only playback-engine-adjacent failures are the first-vertical integration scripts, whose journey_seek/evidence failures are decode-accounting and transcript drift, not session-machine defects). The seek interpreter honors D4 exactly as documented (resolution refusals never touch the pipeline; flush failures rebuffer honestly; the clock-generation publication on both failure and success paths keeps the scheduler gate coherent), the Close composition's generation re-stamp is a correct and well-argued fix for the intra-epoch staleness swallow it names, and open_step's zero-decodable refusal with known-format enrichment is the honest-failure pattern the audit wants everywhere. The one real defect is on the engine-API-only half: `subtitle_select_step` wraps `switch_limbs` in `if let Ok` and then, regardless, emits `TrackSelectionChanged` with the requested selection and a `Completed` receipt — so a subtitle decoder the provider refuses to build produces a *false success event*, a *false receipt*, and a silently discarded typed error, while its own comment claims "a provider that cannot build the subtitle decoder deselects honestly" — behavior that exists only in wire()'s open-time path (pipeline.rs:1083-1087), not here. Two hygiene items: open_step's `unwrap_or_default()` would strand a rejected OpenReady silently in Opening (unreachable today), and the apply() effect order puts state-changed after effect events — flagged as a question for the ordering-guard owner, not a finding.

## 2. What actually works (verified)
— Seek D4 contract — verified by trace: `resolve_target` failure and unseekable sources return BEFORE `pipeline_seek` (queues untouched, demuxer unmoved — exactly the doc's "left exactly as it stood"); flush/reanchor failures run the None-inflight branch: `end_seek` (superseded hold discarded+counted), `clock.set_generation(generation)` (the rollback re-fill not mistaken for pre-roll), `inflight_seek = None`, typed `SeekFailed`. FreezeClock-on-Playing-only verified against state.rs:525-531/540-542.
— Seek supersession — verified: a second seek's `seek_flush`→`end_seek` (pipeline.rs:1253) releases the older claim; the older generation never publishes a terminal (the doc 05 §4.2 rule); the replaced slot's origin carries forward.
— The Close composition macro (1844-1915) — verified by trace: `close_plan` steps applied in order, first rejection becomes the typed receipt, success re-stamps the whole event trail with the FINAL generation (1893-1898) — the argument on 1853-1870 (stop-half events would be swallowed by the intra-epoch drain rule) is exactly how `is_stale_at_drain` behaves (events.rs:478-507, audited in my part1 file); receipts are not re-stamped.
— The class A/S receipt split (1925-1942) — verified: class A answers `Accepted` under the post-bump `wire_generation()`, class S (Close/SelectSubtitle) `Completed` — matching doc 05 §2.3/§4.2 as cited.
— deferred_fatal drain (1953-1955) — verified: the switch row's confirmation outputs precede `fail_with`'s playback.error + state-changed→error — the C2 ordering claimed.
— `open_step` (2342-2419) — verified + EXECUTED: opening_attempted guard; start_demuxer error → fail_with; wire outcome `wired > 0` → OpenReady + open facts (duration-known/info-ready after the track list, before any playing — the doc 05 §4.3.4 pins); `wired == 0` → typed NoDecodableTrack WITH the known-format enrichment (container + every reported codec id — the zero-count wire stored the tracks, so the evidence is real); provider Err → fail_with. The baseline open transcripts (`tracks 2 ready`, `MediaInfoReady`) are this function's output shape.
— `sync_selected_mirror` — verified: re-reads pipeline truth (tracks+selected) after every swap — the `tracks.list`/snapshot `is_selected` stays honest.
— `tick_step` head (2428-2444) — verified + EXECUTED (baseline lifecycles): state captured, pipeline tick, `merge_tick`, starvation absorb sampled before signal translation can retire the pipeline (the comment's ordering rationale holds), throttle `now_ns` read once per tick.

## 3. Findings

### F-001 [S2]
Where: crates/playback-engine/src/session.rs:2267-2308 (the swallow), 2309-2325 (the false success); contrasted correct behavior at crates/playback-engine/src/pipeline.rs:1083-1087
Code (the comment and the swallow):
```rust
        // The optional limb swap at the CURRENT generation (class S:
        // the demux stamp does not move, the A/V limbs do not notice).
        // A provider that cannot build the subtitle decoder deselects
        // honestly (the optional kind — never a failure).
        let generation = self.pair().pipeline.as_queue_generation();
        let flush_budget = self.runtime.budget.flush_calls_per_limb;
        if let Some(pipeline) = self.pipeline.as_mut() {
```
```rust
            if let Ok(outcome) = pipeline.switch_limbs(
```
and the unconditional success that follows (2309-2315):
```rust
        let outputs = vec![self.emit(
            EngineEventKind::TrackSelectionChanged {
                track_kind: MediaKind::Subtitle,
                selected,
            },
            Some(command.request),
        )];
```
and the receipt (2316-2324): `result: ReceiptResult::Completed { … }`.
What is wrong: On a subtitle provider failure, `switch_limbs` returns `Err` (prepare-before-commit: `obtain_decoder`'s provider call is `?`-propagated, pipeline.rs:1765) and the `if let Ok` skips the entire bookkeeping block — but the function then emits `TrackSelectionChanged { selected }` with the REQUESTED selection and a `Completed` receipt anyway. Three defects in one path: (1) the typed error is silently discarded (the checklist's swallowed-error class — no `playback.error`, no rejection, nothing); (2) the event lies — the engine's subtitle selection did NOT change (the old limb stands, the new track's flag untouched), yet every host observer is told it did; (3) the receipt lies the same way. The mirror stays truthful (`sync_selected_mirror` runs outside the `if let Ok`), so the session's own `tracks.list` now CONTRADICTS the event the same command emitted. The comment's claim — "A provider that cannot build the subtitle decoder deselects honestly (the optional kind — never a failure)" — describes `wire()`'s open-time path (pipeline.rs:1083-1087: `Err(_error) if wired == WiredKind::Subtitle => continue`, the deselect), not this path; `pipeline_switch` (tracks.rs:384-403) is a thin wrapper with no such handling.
Why it matters: A host that wires subtitle selection (the UI's `tracks.set-subtitle-track` is in the generated catalog and demo-transported today; the matrix counts track switching as implemented — pipeline.rs part3 F-003) selects a subtitle track whose decoder the provider cannot build, receives `Completed` + `TrackSelectionChanged`, renders the new selection in its UI — and the engine keeps playing the old one (or none). No error surfaces anywhere. The class-S "never a failure" doctrine is about not failing the SESSION, not about lying about the outcome; the honest class-S answer is the deselect the comment already claims happens.
What correct looks like: Mirror wire()'s seam: on `Err`, fall back to `ResolvedSelection::Deselect` (retire the old subtitle limb, `selected = None`), emit `TrackSelectionChanged { selected: None }` and `Completed` — or reject with the typed provider error if class-S must stay non-fatal-but-truthful. Never emit the requested-selection event when the swap did not land. A test: a provider that refuses subtitle decoders + `SelectSubtitle(id)` must not observe `TrackSelectionChanged { selected: Some(id) }`.
Confidence: HIGH — Verification: read (switch_limbs error path, pipeline_switch wrapper, wire's contrasting deselect, and the event/receipt emission all traced; no in-tree test drives a failing subtitle provider through selection — the fakes always build).

### F-002 [S4]
Where: crates/playback-engine/src/session.rs:2380-2385
Code:
```rust
                        let context = InputContext::default();
                        // The rejection is unreachable (the tests pin
                        // OpenReady legal in Opening); honored honestly.
                        let mut outputs = self
                            .apply(SessionInput::OpenReady, context)
                            .unwrap_or_default();
```
What is wrong: A rejected `OpenReady` (if the table ever gains a guard making it illegal in Opening, or a future state reaches open_step with a stale state) is converted to an empty output vector with no error, no state change, and no receipt — the session would sit in `Opening` forever with `opening_attempted = true`, answering nothing. The comment asserts unreachability by test, which is a property of today's table, not of this code.
Why it matters: Defensive-code shape that converts an impossible condition into a silent wedge rather than a typed refusal — the exact anti-pattern the module's own "never a panic, never a guessed default" boundary comment (apply_command, 1785-1787) rejects for payload-less commands. Unreachable today, so S4.
What correct looks like: `.unwrap_or_else(|refused| self.fail_with(internal_open_rejected(refused)))` — or at minimum a comment that names the wedge consequence, not just the unreachability.
Confidence: HIGH — Verification: read (control flow traced; `opening_attempted` makes retry impossible).

### F-003 [S4]
Where: crates/playback-engine/src/session.rs:1949-1955 (context) — the pattern's instance in range
Code:
```rust
                let mut outputs = outputs;
                // A fatal the SwitchPipeline interpreter hit mid-row:
                // the row's confirmation is already in `outputs`; the
                // Failed flow follows it (playback.error before
                // state-changed → error — the C2 ordering).
                if let Some(error) = self.deferred_fatal.take() {
                    outputs.append(&mut self.fail_with(error));
                }
```
What is wrong: Not a behavioral defect — the ordering is exactly as documented — but the mechanism is an out-of-band side channel: `switch_pipeline_step` cannot return its error (the row already applied), so it parks it in `self.deferred_fatal` and apply_command must remember to drain it. Every future effect interpreter that can fail mid-row must replicate this discipline; nothing enforces it (a forgotten `take()` would delay the fatal to an arbitrary later command).
Why it matters: Latent structure debt on the one error path that crosses the table/effect boundary; the comment discipline is good, the mechanism is fragile.
What correct looks like: Make `Effect` interpretation return a `Result` that apply folds into the trailing fatal flow, or assert `deferred_fatal.is_none()` at apply() entry so a parked-but-undrained fatal is a loud bug, not a delayed one.
Confidence: HIGH — Verification: read (all deferred_fatal writes/reads enumerated — set at 2210, drained at 1953; no other sites).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `apply_command` (whole 1784-1970; head 1784-1833 is part3/A01's) | 1834-1970 | REAL | — | Payload-less commands typed-rejected pre-table (1788-1805); subtitle short-circuit (1812-1814); A/V selector resolution pre-mint (1826-1842, reject-before-mint asymmetry documented); Close composition (1871-1915) with generation re-stamp verified against the drain rule; class A/S receipt split; deferred_fatal drain (F-003's mechanism). Reachable: YES (play/pause/stop/close/seek from CLI). |
| `seek_pipeline_step` | 1985-2097 | REAL | — | Steps 1-4: resolution clamp, unseekable early-out (pipeline untouched — D4), `pipeline_seek`, inflight-or-failure handling with clock generation publication on both paths; `hold_cap` derived from `packets_per_limb` (`.max(1)`); `fold_preroll_discards` at both exits. No panics; every add saturating. Reachable: YES (CLI seek). |
| `fold_preroll_discards` | 2098-2113 | REAL | — | take + fold; zero-guard. Called at every seek boundary + teardown (1603/1638). |
| `absorb_pipeline_starvation` | 2114-2131 | REAL | — | Snapshot-assign into totals; called pre-tick-translation and at pipeline-retiring effects (the comment's sampling-order rationale verified). |
| `switch_pipeline_step` | 2132-2212 | REAL | S4 (F-003 mechanism) | Audio-switch clock freeze with the deselect carve-out (the documented audio-deselect drain regression); generation read post-BumpPipeline; `pipeline_switch` delegation; Err → deferred_fatal (never swallowed, never mid-row); in-flight slot replacement-as-supersession. Reachable: NO from shipped hosts (pipeline part3 F-003). |
| `subtitle_select_step` | 2221-2326 | REAL (defective) | S2 (F-001) | Legality families; selector resolution; swap at CURRENT generation (no mint — verified consistent with switch_limbs' `self.generation = generation` being a no-op); **the `if let Ok` swallow + false event/receipt** (F-001); `sync_selected_mirror` outside the Ok (mirror truthful); `end_switch` immediate (receipt-scoped completion — correct, no in-flight subtitle). Reachable: NO from shipped hosts. |
| `sync_selected_mirror` | 2331-2337 | REAL | — | Pipeline-truth re-read. |
| `open_step` | 2342-2419 | REAL | S4 (F-002) | Executed in baseline open lane; zero-decodable typed refusal with real enrichment evidence; `unwrap_or_default` wedge shape (F-002). |
| `tick_step` (head) | 2428-2444 | REAL | — | Executed; boundary at 2444 — signal translation/emissions are A04's part5. |

## 5. Test honesty (tests covering this range)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `tests/seek_flow.rs` suite (passing in no-fail-fast baseline) | YES (spot-checked shapes) | Ordered event-sequence assertions on real engine drives — the D4/rollback/supersession semantics this range implements are its subject. |
| `tests/track_switch.rs::subtitle_selection_is_class_s_over_parsed_streams` / `subtitle_swap_delivers_frames_without_av_interruption` (baseline ok) | PARTIALLY | They pin the happy path and the no-mint property, but neither drives a REFUSING subtitle provider — F-001's false-success path has no test (mutation: flip `if let Ok` to `if let Ok` + emit-none → no suite fails). |
| `tests/session_flow.rs` + scenarios_b (baseline ok) | YES | Open/close/stop composition trails asserted end-to-end. |
| first-vertical integration (journey/evidence/recovery_matrix) | SEE BASELINE | These run the real `lumen` binary; `journey_seek` and `evidence` FAIL in the no-fail-fast run (decode accounting keeps pre-seek deliveries; transcript drift) — engine-side session defects NOT implicated by either failure message; the failures belong to the decode/pipeline and evidence-transcript owners. |
| Coverage hole | — | No test anywhere: refusing subtitle provider through SelectSubtitle (F-001); a state making OpenReady illegal (F-002's premise). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "On a typed refusal the pipeline is left exactly as it stood (contract D4): the queues were not flushed, the demuxer never moved" | session.rs:1980-1984 | REAL | Resolution/unseekable early-returns precede `pipeline_seek`; flush-Err leaves demuxer unmoved (reanchor unreached) |
| "`FreezeClock` (the Playing row) has already frozen the clock at the old position" | session.rs:1975-1976 | REAL | state.rs:525-531: FreezeClock only on the Playing seek row; Paused/Buffering rows lack it (already frozen / no clock) |
| "A provider that cannot build the subtitle decoder deselects honestly (the optional kind — never a failure)" | session.rs:2269-2270 | FALSE for this path | F-001: the deselect exists only in wire() (pipeline.rs:1083-1087); selection-time swallows the Err and lies success |
| "an immediate `time-tick` after `seek-completed` … the landing truth" (M10 order) | session.rs:2540-2552 (part5 range, cross-ref) | REAL | Verified while tracing the terminal order for pipeline part3 F-001: complete_seek re-anchor → tick → release |
| "the composition's own trail is stamped with the composition's FINAL generation" (Close macro) | session.rs:1853-1864 | REAL | 1893-1898 re-stamp; the staleness rationale matches events.rs `is_stale_at_drain` behavior |
| "a rejection is unreachable by construction and honored as the typed receipt rather than a panic" (Close macro) | session.rs:1849-1851 | REAL | The loop's Err arm produces a Rejected receipt (1900-1907); no panic in range |

## 7. Dead / duplicated / unreachable code
- UNREACHABLE from shipped entry points: `switch_pipeline_step`, `subtitle_select_step` (and the A/V arms of `apply_command`'s `resolved_switch`) — engine-API-only, exercised by passing in-tree suites (12/12 track_switch). Not dead.
- No dead code found in range. Duplication: the `target`→match re-mapping in subtitle_select_step (2274-2284) converts `Option<TrackId>` to `ResolvedSelection` and back to `Option<TrackId>` — a shape round-trip the wrapper's signature forces (pipeline_switch takes ResolvedSelection, unwraps it again); cosmetic.

## 8. Dependencies & flags
Crate-internal (state/tracks/pipeline/seek/error), `media_clock` (clock ops), `commands` (receipt/envelope), `media_types`. No cfg, no unsafe, no panics in range (all arithmetic saturating; `unwrap_or_else` guards the two Option-pipeline sites with typed errors — except F-002's `unwrap_or_default`). No feature gates.

## 9. Open questions for the next agent
1. A04 (session part5): the apply() interpreter emits `state-changed` LAST (after all effect events — session.rs:1730-1737, A01's part3). For a failed Playing seek the wire order is `SeekFailed` → `buffering 0.0` → `state-changed(Playing→Buffering)`. Does doc 05 §3.3 (or the OrderingGuard, events.rs part2) pin state-changed FIRST on this edge? The contract suites pass, so presumably the last-confirmation order is the documented one — but the guard's `IllegalTransition`/ordering rows should say so explicitly.
2. A01 (session part3): the `Effect::EmitTracks` payload uses `self.tracks`/`self.selected` — confirm the mirror is always fresh at that point (open_step syncs before OpenReady applies; switch paths sync) — I traced yes, but the invariant is yours to own.
3. Track-switch host wiring (pipeline part3 F-003) compounds with F-001 here: when a host lands, the subtitle false-success and the two pipeline interleaving defects go live together — the wiring PR must carry all three fixes.
