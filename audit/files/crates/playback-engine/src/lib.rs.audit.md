# AUDIT: crates/playback-engine/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-23 |
| Lines / functions | 146 / 0 functions (15 `pub mod` decls, 13 `pub use` groups = 88 root re-exports) |
| File verdict | REAL (wiring + 21/88 live exports) — with FALSE stale doc numbers and a 76%-dead advertised API surface |
| Reachable from shipped entry point? | YES — `lumen` main → `lumen_cli` → apps/cli/src/host.rs:91 `use playback_engine::{CommandReceipt, Engine, …}` → session.rs `Engine::open`/`poll`; apps/web/src/host.rs:96 same. lib.rs is the compilation root of the shipped engine. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 |
| Commands run | `rg --files/-n/-l/-c` sweeps over apps/crates/tools (cited inline); python3 classification of all 88 re-exports against 75 production src files + 921 rs files; `sed -n` quote verification (exit 0 each). No cargo runs (disk-full incident, prior log entry). |

## 1. One-paragraph verdict
lib.rs is a working crate root: every module it declares is real code (prior per-module audits: REAL or real-but-UNREACHABLE), and the 21 root exports the shipped hosts consume (Engine, Session*, SeekTarget, EventPolicy/TimeTickPolicy, DecoderProvider/FrameConsumer, QueuePlan, wire_* helpers, RejectionReason, …) are exercised in production call chains I traced. But the file's own documentation lies about its core artifact — it advertises a 26-row transition table and a 29-legal/75-rejection split while the code ships 33 rows / 33 legal / 71 rejections, pinned by an executable assert — and its advertised API is 76% dead: 67 of 88 root re-exports have zero references in any production source file, six whole re-export groups (chapters, epoch, frame_step, hotplug, recovery, tracks — 29 symbols) are 100% uncalled, and two whole `pub mod` islands (bookmarks, snapshot) have no consumer anywhere in the repo. The crate root is where the product's dead engine surface is minted and published.

## 2. What actually works (verified)
- Module graph compiles into the shipped engine: chain `lumen main → lumen_cli::host::CliHost` (apps/cli/src/host.rs:91-94 imports CommandReceipt, Engine, EngineOutput, ReceiptResult, SeekTarget, SessionCommandKind, SessionConfig, SessionError, SessionHandle, SessionState from this root) `→ session.rs Engine::open/poll` (session.rs.audit.md: YES-reachable, `./target/release/lumen open samples/sample.mp4` exit 0 full state walk). Web host (apps/web/src/host.rs:96-99) imports the same set.
- Root exports used in production CODE (21/88, exhaustive scan of 75 files under apps/*/src): SessionError, EngineEventKind, track_info_wire, wire_error_code, wire_stage, EventPolicy, TimeTickPolicy, DecoderProvider, FrameConsumer, QueuePlan, SeekTarget, CommandReceipt, Engine, EngineOutput, EngineStats, ReceiptResult, SessionCommandKind, SessionConfig, SessionHandle, RejectionReason, SessionState — verified by `rg -n` per symbol (e.g. apps/cli/src/host.rs:91-94, :376, :976-977, :1434; apps/web/src/host.rs:95-99, :1317, :1864-1917; apps/cli/src/demux_host.rs:60; apps/cli/src/pipeline_host.rs:78; apps/cli/src/dispatch.rs:31, :215).
- Doc's acceptance-criteria machinery exists and is executable: crates/playback-engine/tests/table.rs:133-137 asserts `assert_eq!(legal, TRANSITIONS.len())`, `assert_eq!(illegal, 104 - TRANSITIONS.len(), "8 states x 13 inputs")`, `assert_eq!(legal, 33, "the pinned completed-machine row count")`, `assert_eq!(illegal, 71, "71 typed rejections")`; wire conformance via `can_transition_to` at table.rs:171/201/395; the `translate_pipeline_signal` guard test at table.rs:441. The seek suite named in the docs (tests/seek_flow.rs) was EXECUTED by me: 21 passed, exit 0 (prior log).
- "8 wire-visible states" claim TRUE: state.rs:86-106 — exactly 8 variants (Idle, Opening, Buffering, Playing, Paused, Ended, Stopped, Error).
- "no unsafe" TRUE: `rg -n unsafe crates/playback-engine/src` → 1 hit = the word inside lib.rs's own doc comment (line 89); zero unsafe blocks in the crate.
- "source replacement (3.A2) IS here" TRUE at crate level: my replacement.rs audit — 6 tests executed, engine feature real (but see §7: no shipped host reaches it).

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/src/lib.rs:9-10 and :63
Code (L9-10):
```
//! and the transition table as **data** ([`TRANSITIONS`] — 26 rows,
//! every one a legal doc 05 §3.3 wire transition). Around it:
```
Code (L63):
```
//!   `(state, input)` pair (8 × 13 = 104: 29 legal rows, 75 typed
```
What is wrong: Both numbers are false against the file's own crate. TRANSITIONS has 33 rows (counted: 33 `Transition {` entries in state.rs:371-805), and the legal/rejection split is 33/71 — pinned by the executable asserts the same doc paragraph cites (table.rs:136-137 `assert_eq!(legal, 33, …)` / `assert_eq!(illegal, 71, …)`). The two doc sentences also contradict each other (26 rows at L9 vs 29 legal rows at L63). The doc predates card 3.C1's four added rows and was never updated. Why it matters: the crate root is the first thing a host author reads; it misdescribes the size of the machine's single source of truth. What correct looks like: 33 rows, 33 legal / 71 rejections (or drop the numbers and point at the pinned assert). Confidence: HIGH — Verification: ran `rg -c` row count + table.rs asserts read.
F-002 [S3]
Where: crates/playback-engine/src/lib.rs:93-145 (module decls + re-export block)
Code (L93-95 excerpt):
```
pub mod bookmarks;
pub mod chapters;
pub mod epoch;
```
What is wrong: 67 of 88 root re-exports (76%) have ZERO references in production source code (exhaustive scan: 75 files under apps/cli/src, apps/web/src, apps/desktop, apps/desktop-tauri, apps/control-server; python3 word-boundary scan, comment lines excluded). Six re-export groups are 100% dead at the root: chapters (5/5), epoch (5/5), frame_step (4/4), hotplug (2/2), recovery (9/9), tracks (4/4). Two declared modules — `bookmarks` (L93) and `snapshot` (L105) — have no consumer anywhere in the repo at all (module-path and symbol scans: only lib.rs's own `pub mod` lines; the "snapshot" hits in tests are ordinary English). This confirms and quantifies the prior agents' zero-production-caller findings: hotplug.rs / recovery.rs / bookmarks.rs / chapters.rs / frame_step.rs / snapshot.rs audits all verdict UNREACHABLE, and the root re-exports multiply that dead surface into the advertised public API. Why it matters: `pub use` is a support promise; the crate publishes 88 symbols and the shipped product calls 21. The other 67 are consumed only by the crate's own integration tests (if at all — chapters' 5 exports are consumed by nobody, not even tests). What correct looks like: gate the unused surface behind the phase cards that will wire it, or stop re-exporting at the root. Confidence: HIGH — Verification: ran (rg + python3 classification script, both commands in log).
F-003 [S3]
Where: crates/playback-engine/src/lib.rs:20-24
Code:
```
//! - [`hotplug`] — the device-hotplug hooks (card 7.C1): the
//!   engine-side entry points a host calls when the audio device
//!   vanishes mid-playback (suspend in-pipeline, bounded reopen,
//!   resume from the remembered position — never a restart) and the
//!   renderer backend report relay (`video.backend-changed`);
```
What is wrong: The doc asserts a live host contract — "entry points a host calls". No shipped host calls them: hotplug.rs.audit.md (W03-A02) proved `observe_audio_device_lost|poll_hotplug|observe_audio_reopen|observe_renderer_backend` match ONLY hotplug.rs (definitions) and tests/hotplug.rs; apps/cli, apps/desktop, apps/web: zero calls; renderer-wgpu mentions are doc comments. The described behavior is real engine code, but the sentence describes product wiring that does not exist. Why it matters: the crate root documents a recovery story ("never a restart") the shipped product cannot deliver because nothing invokes it. What correct looks like: state that the hooks are engine-side and not yet driven by any host. Confidence: HIGH — Verification: read + prior agent's caller sweep (cited).
F-004 [S4]
Where: crates/playback-engine/src/lib.rs:12-47 (module tour)
Code (L12, first bullet, and L46-47, last):
```
//! - [`epoch`] — the two-level cancellation generations of doc 03 §5
```
```
//! - [`error`] — the transport/construction failure surface (semantic
//!   rejections are typed results in [`state`], not errors).
```
What is wrong: The front-page module tour documents 12 modules but the file declares 15. `bookmarks`, `chapters`, `snapshot` — precisely the three modules with zero consumers anywhere (F-002) — are absent from the crate's own documentation. The doc was not extended when the dead modules were bolted on, hiding them from the reader. Confidence: HIGH — Verification: read (12 bullets vs 15 decls, sed-verified).
F-005 [S4]
Where: crates/playback-engine/src/lib.rs:88
Code:
```
//! Std-only, zero external dependencies, no unsafe. The core never
```
What is wrong: "zero external dependencies" is true only under the repo's private convention "no crates.io crates": Cargo.toml [dependencies] lists six path crates (codec-api, commands, media-clock, media-types, packet-queue, plugin-api). A bare reader of the crate root would conclude the crate builds alone. (Cargo.toml's own comment discloses the convention; lib.rs does not.) "no unsafe" verified TRUE (§2). Confidence: HIGH — Verification: read Cargo.toml + rg unsafe.
F-006 [S4]
Where: crates/playback-engine/src/lib.rs:126-130 (VIDEO_CONCEALMENT_BUDGET re-export) vs apps/cli/src/bench.rs:73-76
Code (bench.rs):
```
/// `VIDEO_CONCEALMENT_BUDGET` — one budget for both limbs by design;
/// a stream whose every packet fails is not being benchmarked, it is
/// broken).
const CONCEALMENT_BUDGET: u32 = 1000;
```
What is wrong: The one root export whose only production appearance is a doc comment is bypassed: the CLI bench re-declares the value locally (1000) instead of importing the engine constant (pipeline.rs:647 `pub const VIDEO_CONCEALMENT_BUDGET: u32 = 1000;`). The coupling is by comment, so a change to the engine budget silently does not change the bench's give-up threshold. Confidence: HIGH — Verification: read both constants.

## 4. Per-function table (no functions; per-module and per-re-export-group inventory is the equivalent mandate)
| Declared item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| pub mod bookmarks | 93 | UNREACHABLE island | S3(F-002) | zero consumers repo-wide; not in doc tour (F-004); not re-exported at root |
| pub mod chapters | 94 | UNREACHABLE | S3(F-002) | 5 root re-exports, 0 users anywhere |
| pub mod epoch | 95 | REAL internal / dead root exports | S3(F-002) | used via crate:: internally; all 5 root exports dead in prod |
| pub mod error | 96 | REAL | — | SessionError live; 3 of 4 root exports dead |
| pub mod event | 97 | REAL | — | EngineEventKind/track_info_wire/wire_error_code/wire_stage live; 4 of 8 dead |
| pub mod events | 98 | REAL | — | EventPolicy/TimeTickPolicy live; 8 of 10 root exports dead |
| pub mod frame_step | 99 | UNREACHABLE | S3(F-002) | 4/4 root exports dead; wire cmd `playback.step-frame` has no dispatch arm (frame_step.rs.audit.md) |
| pub mod hotplug | 100 | UNREACHABLE | S3(F-002,F-003) | 2/2 dead; doc claims host-calls that don't exist |
| pub mod pipeline | 101 | REAL | — | DecoderProvider/FrameConsumer/QueuePlan live; 8 of 11 root exports dead |
| pub mod recovery | 102 | UNREACHABLE | S3(F-002) | 9/9 root exports dead |
| pub mod seek | 103 | REAL | — | SeekTarget live; SeekFailure dead |
| pub mod session | 104 | REAL (shipped core) | — | Engine/poll via cli host.rs:448; 4 of 12 root exports dead (capacities, transition_table) |
| pub mod snapshot | 105 | UNREACHABLE island | S3(F-002) | zero consumers repo-wide; not in doc tour; not re-exported |
| pub mod state | 106 | REAL (shipped core) | — | SessionState/RejectionReason live; 10 of 12 root exports dead in prod (apply_input & co. live internally via crate:: despite root re-export) |
| pub mod tracks | 107 | REAL internal / dead root exports | S3(F-002) | 4/4 root exports dead; commands crate has its OWN TrackSelector duplicate (crates/commands/src/catalog/types.rs:86) |
| 13 pub-use groups | 109-145 | 21/88 live, 67/88 dead | S3(F-002) | full per-symbol table in §7 |

## 5. Test honesty
No tests in this file (doc-comment-only module root). The doc's acceptance-criteria claims about suites are honest where I or prior agents executed them: table.rs classification asserts read + pinned counts verified; seek_flow.rs 21/21 EXECUTED by me (prior log); epoch unit tests exist (epoch.rs:303 `minter_publishes_and_stages_observe`, :322 `interrupt_marks_even_the_current_pair_stale`); track_switch.rs audited REAL by another agent. The one dishonesty is numeric staleness (F-001), not phantom tests.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "TRANSITIONS — 26 rows" | lib.rs:9 | FALSE | 33 rows; table.rs:136 `assert_eq!(legal, 33)`; counted 33 `Transition {` in state.rs:371-805 |
| "104: 29 legal rows, 75 typed rejections" | lib.rs:63 | FALSE | table.rs:136-137 pins 33/71 |
| "hotplug … entry points a host calls" | lib.rs:20-24 | FALSE (as product wiring) | hotplug.rs.audit.md caller sweep: 0 host callers |
| "Std-only, zero external dependencies" | lib.rs:88 | MISLEADING (true only as "no crates.io deps") | Cargo.toml: 6 path deps |
| "no unsafe" | lib.rs:88 | REAL | rg unsafe → only the doc word itself |
| "source replacement (3.A2) IS here" | lib.rs:53-55 | REAL (crate-level) / UNREACHABLE (product) | my replacement.rs audit: 6 tests pass; hosts refuse live-session open (host.rs:897-903) |
| "the host's Engine::poll call IS the session thread … S1 true by construction" | lib.rs:56-57 | REAL | session.rs.audit.md: host-driven executor, engine.poll called at cli host.rs:448 |
| "8 wire-visible states" | lib.rs:7 | REAL | state.rs:86-106, 8 variants |

## 7. Dead / duplicated / unreachable code
Root re-export surface, per symbol (production-code references after comment exclusion; sources scanned: apps/cli/src, apps/web/src, apps/desktop, apps/desktop-tauri, apps/control-server = 75 files):
- LIVE (21): CommandReceipt, DecoderProvider, Engine, EngineEventKind, EngineOutput, EngineStats, EventPolicy, FrameConsumer, QueuePlan, ReceiptResult, RejectionReason, SeekTarget, SessionCommandKind, SessionConfig, SessionError, SessionHandle, SessionState, TimeTickPolicy, track_info_wire, wire_error_code, wire_stage.
- DEAD (67): chapters group (CHAPTER_RESTART_THRESHOLD_NS, chapter_at, chapter_target, next_chapter, previous_chapter — used by nobody, not even engine tests); epoch group (CancelToken, GenerationPair, PipelineGeneration, SessionEpoch, TokenMinter — engine-internal via crate:: only; transcode's CancelToken is its own type, packet-queue's mentions are doc text); error (NoDecodableKnown, SessionConfigError, config_error); event (EngineEvent, media_info_wire, source_spec_wire, wire_playback_state); events (BUFFERING_WINDOW_MS, DAMAGE_WINDOW_MS, DamageCoalescer, DrainStaleness, ProgressWindow, StalenessStamp, TIME_TICK_DEFAULT_MS, TIME_TICK_FLOOR_MS — media-clock/audio-engine RE-IMPLEMENTED ProgressWindow semantics rather than importing); frame_step group (all 4); hotplug group (both); pipeline (DEFAULT_STARVE_DEBOUNCE_TICKS, QUEUE_PACKETS_AUDIO_CEILING, QUEUE_PACKETS_FLOOR, QUEUE_PACKETS_VIDEO_CEILING, QueuePlanError, QueuePlanKind, TickBudget — TickBudget/QueuePlanError appear in prod only inside doc comments — and VIDEO_CONCEALMENT_BUDGET, duplicated in bench.rs, F-006); recovery group (all 9); seek (SeekFailure); session (INBOX_CAPACITY, OUTBOX_CAPACITY, RECEIPT_CAPACITY, transition_table); state (Effect, PipelineSignal, SessionInput, TRANSITIONS, Transition, TransitionRejected, apply_input, close_plan, rejection_reason, translate_pipeline_signal — core functions, live internally via crate::, dead at the root); tracks group (all 4; commands crate ships its own duplicate TrackSelector, crates/commands/src/catalog/types.rs:86).
- Whole dead modules: bookmarks (L93), snapshot (L105) — no consumer anywhere; also missing from the doc tour (F-004).
- Direction note: most "dead" root exports are NOT dead code underneath (apply_input, CancelToken, epoch machinery are the living core used via `crate::`); what is dead is the PUBLISHED surface — lib.rs promises an integration API the product never uses. The genuinely dead-underneath items are the six prior-agent-proven UNREACHABLE modules + the two islands.

## 8. Dependencies & flags
Cargo.toml: [dependencies] = codec-api, commands, media-clock, media-types, packet-queue, plugin-api (all path, zero crates.io — matches the C5 claim under convention). [dev-dependencies] = testkit, demux, codecs, input (test-only composition, disclosed in-file). No [features] section (`rg '\[features\]'` → no match): nothing here is feature-gated; the dead surface is unconditionally compiled and published. Release lane (`-p lumen-cli -p desktop`, no extra features) builds this whole surface in.

## 9. Open questions for the next agent
- The orchestrator cross-ref mentioned an "adjust" surface with zero production callers: NO adjust surface exists in this crate (`rg -in adjust crates/playback-engine/src` → only `A_adjusted` comments in seek.rs:55/439). The claim most plausibly maps to media-clock's drift/rate adjustment (another agent's domain, audit/files/crates/media-clock/src/lib.rs.audit.md exists) — UNVERIFIED which file was meant.
- The UI's TS demo-transport reimplements bookmarks/chapters semantics (per bookmarks.rs/chapters.rs audits) — whoever audits the UI layer should confirm whether the shipped desktop UI calls the mock or the engine.
- The 21 live exports were classified by symbol-name scan; a name-collision audit (same symbol imported from a different crate that re-exports it) was not done — LOW risk, none observed.
