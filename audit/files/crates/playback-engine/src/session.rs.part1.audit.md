# AUDIT: crates/playback-engine/src/session.rs (part1, lines 1–611 of 3058)
> Chunk scope: part1 = lines 1–611 — module docs, the three capacity constants,
> `SessionCommandKind` (+`as_command_id`/`as_input`/`selection_kind`), `SessionCommand`,
> `ReceiptResult`/`CommandReceipt`/`EngineOutput`, `SessionHandle` (+11 methods),
> `SessionConfig`, `Slot`, `OutputHub` struct + `new` + the **Receipt arm of `push`**
> (599–611). `push`'s Event arm and the drop/compact/drain helpers (612–765) are
> W03-A02's part2; `Engine`/`Session` machinery (949+), `apply_command` (1784+) and
> in-file `mod tests` (2861+) belong to parts 2–5 (A02/A01/A09/A04). File name carries
> `.part1` to avoid colliding with the other four part audits.

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-21 |
| Lines / functions | 611 (of 3058) / 24 units (2 enums+3 impl methods, 2 structs+3 impl methods, 1 struct enum pair, 11 handle methods, 1 config ctor, Slot, OutputHub::new, push receipt arm) + 3 config constants |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/host.rs:231` `Engine::new()` → `:983` `.open(config)` → `SessionHandle` held at `host.rs:155`; baseline MP4 smoke `audit/runs/baseline/mp4-smoke-open-session.txt` shows `ok: accepted session=1 gen=0` — this chunk's `SessionHandle`/receipt machinery executed) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read of lines 1–760 (part1 + boundary context); `sed -n` quote verification (109-121, 178-187, 435-442, 599-611, 1061-1100, 1157-1201, 1784-1830, 1900-1935); `rg` callers of `as_input`/`selection_kind`/`as_command_id`/`accepted_generation` (session.rs:1827, 1922; tests/table.rs:664, 844); `rg` CLI usage (`apps/cli/src/host.rs:91-155, 231, 983`); baseline cites: `orchrun-test-workspace-nofailfast.log.txt:3740-3745` (playback_engine lib: 83 tests), `:4196-4210` (tests/table.rs: 23 tests ok), `mp4-smoke-open-session.txt` (open lane); type-inspection of `SeekTarget` (seek.rs:156+) / `TrackSelector` (tracks.rs:232+) for the Send claim. |

## 1. One-paragraph verdict
This chunk is the command-submission API surface of the single-writer engine, and it is honest code: the bounded-inbox doctrine is real (try_send, typed `InboxBusy`/`SessionClosed`, never blocks, never drops at enqueue), the payload-carrying variants (`seek`, `set_*_track`) plumb their payloads correctly, and the doc claims that can be checked against this range's code all check out (capacity constants, receipt-before-event ordering by push order, receipts pinned from event overflow). It executed in the baseline: the `lumen open` smoke lane printed this chunk's accept receipt, and the 23-test table suite that pins `as_command_id`/`as_input` against the frozen command catalog ran green. The only defects are two hygiene items: a production-dead enum arm kept deliberately ("harmlessly unreachable" — the authors' own words, in two places) and a silent `max(1)` clamp that rewrites a caller's `0` capacity into `1` without a word. One cross-boundary race (submit during the closing session's final poll) is flagged for part2's owner in §9 — its defect site is `poll`, not this range.

## 2. What actually works (verified)
— Executed by the shipped CLI: `apps/cli/src/host.rs:983` `.open(config)` returns this chunk's `SessionHandle`; baseline `audit/runs/baseline/mp4-smoke-open-session.txt` first line `ok: accepted session=1 gen=0` is the receipt minted through this machinery, and the AVI lane (`avi-firstvertical-open-session.txt`) drives full lifecycle.
— Catalog reconciliation executed: baseline `orchrun-test-workspace-nofailfast.log.txt:4196-4210` — `tests/table.rs` 23 tests `ok`, including `command_kinds_map_onto_table_inputs` (asserts `as_command_id` → `catalog_data()` id, class, gen-bumping for all 8 kinds, table.rs:663-672) and the `as_input` mapping table (table.rs:830-850).
— Bounded-inbox semantics verified by code trace: `queue` (437-442) maps `TrySendError::Full`→`InboxBusy`, `Disconnected`→`SessionClosed`; channel is `sync_channel(INBOX_CAPACITY)` (1071, part2 — boundary verified); `Disconnected` fires once the `Session` owning the `Receiver` drops (poll step 3 at 1196-1199 / `replace_live_session` at 1273 — verified by reading, those lines are part2/3's).
— `SessionHandle` Send/Clone claim: type-inspected — `SyncSender<SessionCommand>` is Send+Clone when `SessionCommand` is; `SessionCommand` = `RequestId` + `Copy` enum + `Option<SeekTarget>` (plain i64/float payload enum, seek.rs:156+) + `Option<TrackSelector>` (plain enum, tracks.rs:232+). No `Rc`/`RefCell` anywhere. No compile-time `assert::<fn() -> SessionHandle>()`-style proof exists — claim rests on type inspection (this audit's), true today.
— Receipt-arm of `push` (599-611) traced against its helper `drop_oldest_receipt` (659-667, part2): bounded at exactly `receipts_capacity`, cursor maintenance correct (`index = queue.len()` before `push_back`; `first_receipt` set only when `None`).

## 3. Findings

### F-001 [S4]
Where: crates/playback-engine/src/session.rs:181-186 (dead in production; interception at 1816-1822)
Code:
```
            // The subtitle flavor is class S: it never reaches the
            // table (the pre-table branch owns it); the mapping is
            // the A/V switch input for the two class-A flavors.
            SessionCommandKind::SelectVideo
            | SessionCommandKind::SelectAudio
            | SessionCommandKind::SelectSubtitle => SessionInput::SwitchTrack,
        }
```
What is wrong: the `SelectSubtitle` arm of `as_input` is dead in every production flow — `apply_command` intercepts `SelectSubtitle` at 1816-1822 (`if command.kind == SessionCommandKind::SelectSubtitle { return self.subtitle_select_step(command); }`) before the single call site `self.apply(command.kind.as_input(), context)` at 1922 can ever see it. The test suite pins the dead mapping as intentional (tests/table.rs:834-836: "its mapping is the same input, harmlessly unreachable").
Why it matters: a future refactor that removes or reorders the pre-table branch silently changes subtitle selection from the class-S no-mint path into a class-A pipeline invalidation — the arm is a loaded footgun kept loaded on purpose. Documented twice, so this is hygiene, not deception.
What correct looks like: make the dead state unrepresentable — `as_input` returns `Option<SessionInput>` for selection kinds, or split `as_input` (state-table kinds) from `as_switch_input` (A/V only), so the compiler enforces what comments currently assert.
Confidence: HIGH — Verification: read (call-site trace 1816-1822, 1922; test read table.rs:830-850).

### F-002 [S4]
Where: crates/playback-engine/src/session.rs:571-575
Code:
```
    fn new(capacity: usize, receipts_capacity: usize) -> OutputHub {
        OutputHub {
            queue: VecDeque::new(),
            capacity: capacity.max(1),
            receipts_capacity: receipts_capacity.max(1),
```
What is wrong: a caller passing `0` (e.g. `Engine::with_outbox_capacity(0)`, 988-991) silently gets capacity 1 — the clamp rewrites the caller's number with no log, no typed error, no debug_assert. Nothing documents the 0→1 rewrite at the clamp site.
Why it matters: a host sizing the outbox for a measured drain interval that computes 0 gets double the requested capacity (1 live event where 0 was asked); more importantly the silent clamp hides caller arithmetic bugs the typed-error discipline this crate otherwise enforces would have surfaced.
What correct looks like: `debug_assert!(capacity > 0)` plus a documented minimum, or a typed `SessionConfigError` for 0 — the crate already has that error vocabulary.
Confidence: HIGH — Verification: read; `with_outbox_capacity` (989-1000, part2) confirmed to pass the raw value through.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-57 | REAL | — | Claims audited in §6; threading-model and ordering claims verified against code in this and adjacent ranges. |
| `OUTBOX_CAPACITY` | 92-98 | REAL | — | 1024; PROPOSAL-annotated vs doc 05 §3.4's 1000/s burst bound — honest. |
| `RECEIPT_CAPACITY` | 100-109 | REAL | — | 4096; drop-oldest-receipt behavior in push (602-604) + counter (part2 `drop_oldest_receipt`). |
| `INBOX_CAPACITY` | 111-121 | REAL | — | 64; enforced via `sync_channel` (1071) + `try_send` Full mapping (439). |
| `SessionCommandKind` | 126-153 | REAL | — | 8 kinds; variant docs match catalog classes (test-pinned, table.rs:663-672). |
| `as_command_id` | 158-170 | REAL | — | Bijective mapping to `CommandId`; test-pinned against `catalog_data()`. |
| `as_input` | 173-188 | REAL | — | Correct for 7 kinds; **SelectSubtitle arm production-dead (F-001)**. |
| `selection_kind` | 197-204 | REAL | — | Lane-typed; called at 1827; `_ => None` for non-selection kinds. |
| `SessionCommand` | 209-220 | REAL | — | Copy payload struct; honestly "Not Eq" (float seek fraction). |
| `ReceiptResult` | 226-251 | REAL | — | Three-way truthful-UI split; data only. |
| `accepted_generation` | 256-263 | REAL | — | Mirror of envelope's outcome method; used by tests. |
| `CommandReceipt` | 267-275 | REAL | — | Echo-only correlation; no interpretation. |
| `EngineOutput` | 279-285 | REAL | — | Receipt \| Event sum; ordering enforced by push order (poll 1166-1171). |
| `SessionHandle` | 298-302 | REAL | — | Send/Clone verified by type inspection (no static assert — noted). |
| `session_id` | 306-309 | REAL | — | Pure delegation to epoch. |
| `submit` | 320-327 | REAL | — | Payload-less enqueue of Seek/Select* → typed `MissingPayload` at apply (1787-1806) — documented async-rejection design, not a defect. |
| `play`/`pause`/`stop`/`close` | 333-359 | REAL | — | Thin wrappers; no drift. |
| `seek` | 368-375 | REAL | — | Payload plumbed: `seek: Some(target)`. |
| `set_video_track`/`set_audio_track`/`set_subtitle_track` | 383-433 | REAL | — | Payload plumbed: `selector: Some(selector)`; class-A/A/S docs match catalog (test-pinned). |
| `queue` | 437-442 | REAL | — | try_send mapping verified; "never blocks, never drops" true at enqueue (end-to-end caveat → §9 race note). |
| `SessionConfig` | 450-481 | REAL | — | All injectable; no concrete demuxer/decoder/clock construction — claim verified by field types. |
| `SessionConfig::new` | 486-505 | REAL | — | Defaults: `QueuePlan::default()`, `TickBudget::default()`, `EventPolicy::default_policy()`; `container: None` documented. |
| `Slot` | 515-522 | REAL | — | Tombstone scheme; index-stability invariant upheld by push/drop (part2, traced). |
| `OutputHub` struct | 543-568 | REAL | — | Cursor invariants documented and upheld (`first_event` None iff `live_events == 0` — checked in push paths). |
| `OutputHub::new` | 571-586 | REAL | S4 | **Silent max(1) clamp (F-002)**. |
| `push` (Receipt arm) | 599-611 | REAL | — | Bounded-at-capacity trace correct; Event arm + helpers are part2 (read for context: tick-replacement and cursor maintenance verified correct). |

## 5. Test honesty (tests pinning this chunk live in crates/playback-engine/tests/, not in-file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `table.rs::command_kinds_map_onto_table_inputs` (≈630-675) | YES | Enum-to-catalog mapping asserted per kind against `catalog_data()` (id, class, gen_bumping); a wrong `as_command_id` arm or a catalog drift fails it. Executed: baseline `:4197` `command_kinds_map_onto_table_inputs ... ok`. |
| `table.rs` `as_input` cases (830-850) | YES for the mapping | Direct assert per kind; would catch a remapped input. Cannot catch F-001 (it *pins* the dead arm as correct). |
| `contract_suite.rs` accepted_generation asserts (1136-1225), `seek_flow.rs` (1242-1290), `replacement.rs` (909) | YES | End-to-end receipts through real sessions; seek gen-bumping receipts asserted (`Some(Generation(1))`, `Some(Generation(2))`) — these execute `submit`/`seek`/receipt minting from this chunk. All ran in baseline (playback_engine suites at log :3836+, all ok). |
| Mutation check (reasoned, not run): change `queue`'s `Full` mapping to `SessionClosed` | Caught | `session_flow.rs:1888-1910 the_inbox_refuses_typed_at_its_bound_and_drains_whole` submits `INBOX_CAPACITY + 8` pauses pre-poll and asserts exactly 64 accepted + 8 typed `InboxBusy` — the mapping arm is covered end-to-end (corrected post-commit after a deeper grep; initial draft wrongly flagged it as a coverage gap). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every mutation happens behind `&mut self` on Engine::poll … no second writer" | session.rs module docs 8-17 | REAL | All pub fns on Engine take `&mut self`; SessionHandle's only cross-thread edge is the mpsc tx (298-302). |
| "The receipt for a command precedes any event the command caused" | module docs 33-34 | REAL | poll 1166-1171 pushes receipt then outputs (part2 range, traced); causal order preserved by OutputHub append-only queue. |
| "Superseded pipeline events … suppressed at drain … while receipts always survive" | module docs 37-40 | REAL (with qualifier) | drain_into (740-763, part2) suppresses stale events only; receipts always pushed to `out`. "Always survive" excludes the separate RECEIPT_CAPACITY drop — the module doc's own line 92-94 states the qualifier, so no contradiction, but a fast reader can miss it. |
| "submitting after the session closed answers the typed SessionError::SessionClosed — never a silent drop" | SessionHandle docs 292-297 | REAL | Disconnected fires when Session (Receiver owner) drops (poll 1196-1199 / replace_live_session 1273). Race caveat at §9 — enqueue itself never silently drops; end-to-end receipt delivery has a one-poll window on a concurrently-closing session. |
| "a host that never drains loses its oldest undelivered receipt (counted in EngineStats::outbox_dropped_receipts)" | RECEIPT_CAPACITY doc 100-109 | REAL | push 602-604 + drop_oldest_receipt (659-667); counter surfaced via EngineStats (part2). |
| "set-rate is Phase 3" | SessionCommandKind doc 124-125 | REAL | No SetRate kind exists; catalog has no playback.set-rate row (frozen catalog — W02-A07's spec.rs audit). |

## 7. Dead / duplicated / unreachable code
- F-001's dead `SelectSubtitle` arm (documented, test-pinned).
- No duplication in range; `play/pause/stop/close` wrappers are deliberate thin sugar, each with distinct docs.

## 8. Dependencies & flags
- No feature gates in range; all code unconditional. Crate compiled and tested in baseline (build log; test log :3740+).
- Imports (59-90) all resolve to workspace crates (`commands`, `media_clock`, `media_types`, `plugin_api`) — matches the layering rules (playback-engine L3, no upward deps).
- `pub(crate)` on OutputHub/push — hotplug hooks (card 7.C1) named as consumer; verified `crate::hotplug` exists (used at 997).

## 9. Open questions for the next agent
- **For W03-A02 (part2, poll 1157-1201):** cross-thread submit during a closing session's final poll — a command enqueued after step 1's drain but before step 3's `self.session = None` (1198) is destroyed with the session: sender got `Ok(())`, no receipt ever mints. The window needs a concurrent submitter thread; the RECEIPT_CAPACITY doc's "host re-queries or times out" mitigation applies. Defect site is poll (part2), so not filed as this chunk's finding.
- ~~Coverage gap~~ CORRECTED: `session_flow.rs:1888-1910` (`the_inbox_refuses_typed_at_its_bound_and_drains_whole`) covers the `InboxBusy` arm end-to-end (submits INBOX_CAPACITY+8 pre-poll, asserts 64 accepted / 8 typed busy, then one-poll whole-drain order). No gap.
- `EngineEventKind::TimeTick` tick-replacement semantics (push event arm, 612-635) are part2's — read and verified correct in passing; A02 should file findings there if any.
