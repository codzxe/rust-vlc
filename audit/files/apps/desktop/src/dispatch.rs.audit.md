# AUDIT: apps/desktop/src/dispatch.rs
| Field | Value |
|---|---|
| Auditor | W13-A09 |
| Date | 2025-07-14 |
| Lines / functions | 401 / 13 non-test + 5 tests |
| File verdict | REAL code, UNREACHABLE from the shipped binary (the desktop's entire dispatch stack is library-only; `lumen-desktop` main.rs prints a report and exits 86 before serving anything) |
| Reachable from shipped entry point? | NO — main.rs imports only `host` + `window_config` + `strategy_for`; ShellHost/lumen_dispatch are consumed by shell.rs (library) and tests. YES from the desktop lib test suite (ran it). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `cargo test -p desktop --lib dispatch` -> 8 passed (5 dispatch + 3 shell); `rg enum CommandId ids.rs` -> 65 variants; SERVED count -> 15 |

## 1. One-paragraph verdict
THE shared-surface answer: the desktop dispatch is a one-line delegation to the CLI's dispatch (`lumen_cli::dispatch::lumen_dispatch`), so the desktop serves EXACTLY the CLI's 15-command `SERVED` table (session open/close, play/pause/toggle/stop/seek, media get-info/current-time/duration, tracks list, engine-info, list-commands, config get/set) out of a 65-variant catalog — Wave 2's "15/65" confirmed by counting `enum CommandId` (65) and `SERVED` (15); every other id answers a typed `unsupported-for-host` refusal naming the command (test-proven with `playback.set-rate`). The event-forward half (cursor drain + bounded drop-oldest queue with conservation stats) is real and tested. But all of it is dead in the shipped build: the `lumen-desktop` binary never starts a shell — it prints and exits 86 (ADR-004 pending), so this whole module runs only under `cargo test`.

## 2. What actually works (verified)
— verified by `cargo test -p desktop --lib dispatch` -> `test result: ok. 8 passed` (real engine open of the AVI fixture, settle, ordered once-only event drain, typed refusal with echoed request_id, catalog-command funnel, path classification).
— verified by `rg -c` counts: `enum CommandId` has 65 variants (crates/commands/src/catalog/ids.rs:96); `SERVED` (apps/cli/src/dispatch.rs:41-56) lists 15.

## 3. Findings
F-001 [S3]
Where: apps/desktop/src/dispatch.rs:100-109
Code:
```
    pub fn forward_new_events(&mut self, sink: &mut dyn EventSink) -> usize {
        let events = self.host.events();
        let mut count = 0;
        for event in &events[self.forwarded..] {
```
What is wrong: The drain cursor slices `&events[self.forwarded..]` — safe ONLY under an undocumented cross-crate invariant that `CliHost::events()` is grow-only. Verified today: host.rs pushes at :678 and never clears/truncates (rg `self.events` -> push/read only). If the CLI host ever compacts its log, this panics (start > len). Second-order: grow-only means an unbounded event Vec for a long-running desktop session — fine for the CLI's process lifetime, a leak-by-design for the desktop's intended long-lived shell (latent only because the desktop binary exits before serving). What correct looks like: document the grow-only contract at the seam (or have `events()` return a watermark type), and bound the log with an acked cursor for desktop use.
Confidence: HIGH — Verification: read + rg on host.rs.
(Downgraded from S2: unreachable from the shipped binary today.)

F-002 [S3]
Where: apps/desktop/src/dispatch.rs:211-213 (vs apps/desktop/src/main.rs:16-17,89-90)
Code:
```
pub fn lumen_dispatch(shell: &mut ShellHost, request: &WireRequest) -> WireResponse {
    lumen_cli::dispatch::lumen_dispatch(&mut shell.host, request)
}
```
What is wrong: Nothing in the delegation — what is wrong is the reachability: the shipped `lumen-desktop` binary imports only `DesktopHost, TargetOs, WebviewHostKind, WindowTitle, parse_window_table, strategy_for` (main.rs:16-17,74) and unconditionally returns `ExitCode::from(WEBVIEW_PENDING)` (86). No `ShellHost`, no `lumen_dispatch`, no invoke handler exists in any shipped entry. The entire dispatch/shell/bridge stack is compiled library code awaiting the ADR-004 tauri wrapper. Why it matters: the "15/65 shared surface" is real as a library and as tests, but no user-visible surface serves it today. What correct looks like: this is the documented honest-skeleton posture (main.rs says so) — the finding records UNREACHABLE, not fraud.
Confidence: HIGH — Verification: read + rg (no other consumer).

F-003 [S4]
Where: apps/desktop/src/dispatch.rs:190-196
Code:
```
    fn offer(&mut self, event: &WireEvent) {
        if self.queue.len() == self.capacity && self.queue.pop_front().is_some() {
            self.stats.dropped_overflow += 1;
        }
```
What is wrong: Under sustained emitter lag the OLDEST events are dropped permanently — including terminal events (end-of-media, error) the UI then never sees; the once-only drain cursor has already advanced past them. Counted honestly (`dropped_overflow`) and documented ("never a block, never a reorder"), but a UI that lags a full queue loses state truth with no recovery path.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: apps/desktop/src/dispatch.rs:89-94
Code:
```
    /// The engine-driving host (test/diagnostic access; every
    /// command still flows through [`lumen_dispatch`]).
    pub fn engine_host(&mut self) -> &mut lumen_cli::host::CliHost {
```
What is wrong: A public `&mut CliHost` escape hatch on ShellHost undermines the module's own single-command rule ("No other entry exists for UI-originated commands") — any future wrapper code holding a ShellHost can drive the engine directly. Currently used by tests only (and shell.rs), which is the documented intent, but `pub` + `&mut` invites the bypass the rule forbids.
Confidence: MEDIUM — Verification: read + rg (callers: tests + shell.rs).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ShellHost (struct) | 66-71 | REAL | — | CliHost + forwarded cursor |
| Default | 73-77 | REAL | — | delegates to new |
| ShellHost::new | 82-87 | REAL | — | fresh host, cursor 0 |
| ShellHost::engine_host | 92-94 | REAL | S4(F-004) | pub &mut escape hatch |
| ShellHost::forward_new_events | 100-109 | REAL | S3(F-001) | once-only ordered drain; grow-only dependency |
| EventSink (trait) | 115-118 | REAL | — | forward seam |
| EventQueueStats | 123-133 | REAL | — | offered/taken/overflow/stale counters |
| EventQueueStats::conserves | 139-141 | REAL | — | arithmetic identity, shell-tested |
| BoundedEventSink::with_capacity | 161-167 | REAL | — | NonZeroUsize capacity |
| BoundedEventSink::take | 171-175 | REAL | — | drains in order, counts taken |
| BoundedEventSink::stats | 179-181 | REAL | — | observable accounting |
| BoundedEventSink::note_stale | 186-188 | REAL | — | §4.2 filter count |
| BoundedEventSink::offer | 190-196 | REAL | S4(F-003) | drop-oldest overflow |
| lumen_dispatch | 211-213 | REAL | S3(F-002 reachability) | 1:1 CLI delegation — the 15/65 surface |
| open_file_request | 220-231 | REAL | — | builds catalog session.open |
| playlist_load_request | 238-246 | REAL | — | append:false |
| open_path_request | 253-261 | REAL | — | playlist > media > None; extension-based (files.rs twin) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| unserved_commands_refuse_typed_and_echo | YES | exact UnsupportedForHost code + command detail + request_id echo |
| served_commands_reach_the_engine_and_answer | YES | real engine listing + honest None session context |
| events_forward_in_order_exactly_once | YES | real fixture open, settle, counts + no-redelivery |
| file_open_funnel_is_the_catalog_command | YES | exact wire shape match |
| open_path_request_classifies_by_kind | YES | media/playlist/none all three asserted |
All executed green (`cargo test -p desktop --lib dispatch`, 5 dispatch tests + 3 shell companions).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one command, lumen_dispatch, and nothing else ever registered" | L1-3 | REAL (as library) / UNREACHABLE (shipped) | no invoke handler exists; main exits 86 |
| "The engine + session wiring is the SAME host the CLI drives" | L39-40 | REAL | L212 delegates to lumen_cli::dispatch verbatim |
| desktop serves 15 of 65 commands (Wave 2) | external claim | REAL | SERVED=15 (cli dispatch.rs:41-56), CommandId=65 (ids.rs:96); typed refusal test passes |
| "an event is delivered exactly once per sink chain" | L98-99 | REAL | cursor test; grow-only invariant verified but undocumented (F-001) |

## 7. Dead / duplicated / unreachable code
- Entire module unreachable from the shipped binary (F-002) — by design (ADR-004 pending), compiled and tested as library.
- `BoundedEventSink`/`EventQueueStats` consumed by shell.rs (also library-only).

## 8. Dependencies & flags
Depends on lumen-cli (the whole CLI crate) — the desktop "app" is a thin shell over the CLI host. commands crate for wire types. No tauri (deliberately: allowlist rule cited at L30-35). No feature gates.

## 9. Open questions for the next agent
- When the ADR-004 wrapper lands, who re-audits the grow-only events contract (F-001) and the pub engine_host bypass (F-004)?
- The TS-188 probe panic (my demux_host audit) is reachable from THIS module via open_path_request -> session.open -> probe — same latent exposure once a wrapper serves drops.
