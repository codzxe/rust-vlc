# AUDIT: apps/desktop/src/dispatch.rs
| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-21 |
| Lines / functions | 401 / 17 (12 production items + 5 tests + 2 test helpers) |
| File verdict | REAL (thin, correct delegation + bounded egress; one unbounded-growth design gap) |
| Reachable from shipped entry point? | YES (lumen-tauri chain: desktop-tauri/src/app.rs:293 `invoke_handler(tauri::generate_handler![lumen_dispatch])` → app.rs `lumen_dispatch` command → ShellRuntime::dispatch (shell.rs, imports `crate::dispatch::lumen_dispatch` at shell.rs:46) → this file:211 → `lumen_cli::dispatch::lumen_dispatch`. `open_path_request` ← shell.rs:302 (drop/pick funnel); `BoundedEventSink`/`forward_new_events` ← shell.rs:104/130/233 (event egress + pump). CAVEAT: lumen-tauri binary UNVERIFIED-BUILD in baseline #2 (missing gtk/webkit dev libs); desktop crate compiled (baseline #1) and all 5 in-file tests executed ok — orchrun-test-workspace-nofailfast.log.txt:2267-2270, 2293.) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | Read dispatch.rs 1-401 (full); rg `dispatch::lumen_dispatch|open_path_request|BoundedEventSink|forward_new_events` apps/desktop/src (shell.rs is the production consumer); rg `generate_handler|invoke_handler` apps/desktop-tauri/src (app.rs:293 — exactly one command registered); Read desktop-tauri/tests/single_command.rs 1-79 (anti-bypass test exists, source-scanning; workspace-excluded crate → not in baseline test run); rg `pub fn events\(` apps/cli/src/host.rs:320 + rg `events\.clear|events\.truncate|events\.drain|events\.remove|events\.push` host.rs (only push at 678 — log never pruned); Read host.rs 650-694 (event push is staleness-filtered); grep baseline logs 2267-2270/2293 (5 dispatch tests ok) + 2288-2300 (shell tests exercising BoundedEventSink). |

## 1. One-paragraph verdict
A disciplined 250-line production half: `ShellHost` (the CLI's `CliHost` plus a once-only forward cursor), a bounded drop-oldest event queue with honest conservation arithmetic, and four pure request-builders — everything else is a one-line delegation to the CLI's own dispatcher, so the desktop inherits the command layer verbatim (no re-implementation, verified: dispatch.rs:212 is the only engine call). The single-command rule the module exists to enforce is real in the shipped shell (app.rs:293 registers exactly `lumen_dispatch`), and the anti-bypass test exists (desktop-tauri/tests/single_command.rs) though its crate is workspace-excluded and its build failed in the sandbox, so the test itself has no recorded execution. The one substantive defect is inherited, not local: the host-side event log grows for the process lifetime (only ever pushed, host.rs:678; never pruned), which the `BoundedEventSink`'s webview-side bound does nothing about — a slow unbounded growth in exactly the long-lived GUI process this module is the composition root for. A test name promises order verification its body never performs, and the `pub engine_host()` accessor is an unrestricted bypass seam documented as "test/diagnostic access" while being production-load-bearing.

## 2. What actually works (verified)
- All 5 in-file tests executed and passed in baseline: `dispatch::tests::{unserved_commands_refuse_typed_and_echo, served_commands_reach_the_engine_and_answer, file_open_funnel_is_the_catalog_command, open_path_request_classifies_by_kind, events_forward_in_order_exactly_once}` — orchrun-test-workspace-nofailfast.log.txt:2267-2270, 2293.
- `events_forward_in_order_exactly_once` drives a REAL `session.open` of the committed AVI fixture through the REAL dispatch and a REAL settle — not a mock (dispatch.rs:335-356; fixture path `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi`).
- `BoundedEventSink`'s bounded drop-oldest + conservation identity is executed by `shell::tests::egress_queue_is_bounded_drop_oldest_and_conserves ... ok` (log:2292) — shell.rs's test exercises this file's `offer`/`take`/`stats`.
- The single-command rule: app.rs:293 `.invoke_handler(tauri::generate_handler![lumen_dispatch])` — exactly one entry (rg-verified, one generate_handler site).
- The forward cursor is panic-safe by construction: `CliHost::events()` returns `&self.events` (host.rs:320-322) and the Vec is only ever `push`ed (host.rs:678; no clear/truncate/drain — rg-verified), so `&events[self.forwarded..]` can never underflow.

## 3. Findings

### F-001 [S3] — Host-side event log grows for process lifetime; the bounded sink bounds only the webview half
Where: apps/desktop/src/dispatch.rs:66-71, 100-109 (consumer) vs apps/cli/src/host.rs:172, 678 (owner)
Code:
```
pub struct ShellHost {
    host: lumen_cli::host::CliHost,
    /// How many of `host.events()` have already been forwarded —
    /// the once-only, in-order drain cursor.
    forwarded: usize,
}
```
```
    pub fn forward_new_events(&mut self, sink: &mut dyn EventSink) -> usize {
        let events = self.host.events();
        let mut count = 0;
        for event in &events[self.forwarded..] {
            sink.forward(event);
            count += 1;
        }
        self.forwarded = events.len();
        count
    }
```
What is wrong: `CliHost.events` is a `Vec<WireEvent>` that is only ever appended (host.rs:678, staleness-filtered) and never pruned — no `clear`/`truncate`/`drain` exists anywhere in host.rs (rg-verified). `forward_new_events` advances a cursor but the forwarded events STAY in the Vec. The `BoundedEventSink` (this file, 152-197) bounds only the drain→webview queue; the source log behind it is unbounded. The CLI process is short-lived so the growth is invisible there; the desktop shell — the long-lived GUI process this module is the composition root for — retains every wire event (plus `state_history`, host.rs:326, same pattern) for the whole process lifetime.
Why it matters: A GUI app left open for days with repeated open/play/seek cycles accumulates event history monotonically. The staleness filter (doc 05 §4.2) keeps the rate low, so this is a slow leak, not a crash — but the cursor already proves which prefix is dead (delivered) and prunable, and nothing prunes it. This is the "unbounded queue" checklist item wearing a Vec costume.
What correct looks like: Either `CliHost` offers `retain_from(n)`/`prune_forwarded(usize)` that ShellHost calls after each drain, or the shell's egress path takes events by value (a `VecDeque` drain the host owns) so delivery implies release.
Confidence: HIGH — Verification: read (owner-side push-only verified by rg over host.rs; growth path traced). Impact estimate: LOW-rate (staleness filter), hence S3 not S2.
Cross-ref: `CliHost` is apps/cli/src/host.rs (another wave's file) — the fix lands there or at this seam.

### F-002 [S4] — `events_forward_in_order_exactly_once` never asserts ORDER
Where: apps/desktop/src/dispatch.rs:335-356
Code:
```
        let first = shell.forward_new_events(&mut sink);
        assert!(first > 0, "a settled open must have produced events");
        assert_eq!(sink.seen.len(), first);
        // Monotonic cursor: nothing is re-delivered.
        let second = shell.forward_new_events(&mut sink);
        assert_eq!(second, 0);
        assert_eq!(sink.seen.len(), first);
```
What is wrong: The test's name (and the module doc's "in engine order, exactly once" claim, 51-52, 96-99) promises order verification. The body asserts only counts and once-only-ness. `RecordingSink` even collects `event.seq` (272-278) — the exact data an order assertion needs — and never checks it.
Why it matters: A regression that forwards events out of engine order (e.g., a future change from slice-cursor to per-event popping under a lock) passes this test while violating the §4.3 ordering guarantee the doc leans on.
What correct looks like: `assert!(sink.seen.windows(2).all(|w| w[0] < w[1]), "engine order: {:?}", sink.seen);`
Confidence: HIGH — Verification: read.

### F-003 [S4] — `pub engine_host()` is an unrestricted bypass seam documented as "test/diagnostic access" while production-load-bearing
Where: apps/desktop/src/dispatch.rs:89-94
Code:
```
    /// The engine-driving host (test/diagnostic access; every
    /// command still flows through [`lumen_dispatch`]).
    #[must_use = "the adopted composition pattern"]
    pub fn engine_host(&mut self) -> &mut lumen_cli::host::CliHost {
        &mut self.host
    }
```
What is wrong: The doc says "test/diagnostic access", but production shell.rs calls it for engine-driving operations that bypass the command layer: `set_present_tap` (shell.rs:187), `settle`/`pump`/pacer ticks (shell.rs:233 region, 347 region). The §5.1 single-command rule ("no other entry exists for UI-originated commands", 208-210) is therefore enforced by convention only — any future UI-originated command can be wired through this accessor with nothing failing.
Why it matters: The module's headline architecture claim has a pub escape hatch whose own doc understates its production use. The bypasses used today (tap/pacer/settle) are legitimately non-command engine operations, but the seam makes the boundary unauditable from this file.
What correct looks like: Split the accessor: a narrow `pub(crate)` pacer/tap surface (the operations the shell legitimately needs) and keep the broad `&mut CliHost` under `#[cfg(test)]` — or correct the doc to name the production uses.
Confidence: HIGH — Verification: read + rg (shell.rs:187, 233, 347 are production call sites).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ShellHost` (struct) | 66-71 | REAL | S3 (F-001) | Cursor field; the wrapped host's log is the unbounded part. |
| `Default for ShellHost` | 73-77 | REAL | — | Delegates to new(). |
| `ShellHost::new` | 82-87 | REAL | — | Fresh CliHost + zero cursor. |
| `ShellHost::engine_host` | 92-94 | REAL | S4 (F-003) | Bypass seam; production-used despite test/diagnostic doc. |
| `ShellHost::forward_new_events` | 100-109 | REAL | S3 (F-001) | Once-only cursor correct (append-only invariant verified); delivered events retained. |
| `EventSink` (trait) | 115-118 | REAL | — | One method; implemented by BoundedEventSink + tests' RecordingSink. |
| `EventQueueStats` (struct) | 123-133 | REAL | — | Pure counters. |
| `EventQueueStats::conserves` | 139-141 | REAL | — | Correct identity (offered == taken + dropped_overflow); exercised via shell::tests (log:2292). |
| `BoundedEventSink` (struct) | 152-156 | REAL | — | VecDeque + stats. |
| `BoundedEventSink::with_capacity` | 161-167 | REAL | — | NonZeroUsize → capacity ≥ 1 by type; pre-allocates. |
| `BoundedEventSink::take` | 171-175 | REAL | — | Drains in order; taken counted before drain. |
| `BoundedEventSink::stats` | 179-181 | REAL | — | Accessor. |
| `BoundedEventSink::note_stale` | 186-188 | REAL | — | Filter-refusal counting; called by shell.rs's DeliverThrough (§4.2 filter). |
| `BoundedEventSink::offer` (private) | 190-196 | REAL | — | Drop-oldest correct; `pop_front().is_some()` is always true when len==capacity≥1 — redundant condition, harmless. |
| `EventSink for BoundedEventSink` | 199-203 | REAL | — | Delegation. |
| `lumen_dispatch` (free fn) | 211-213 | REAL | — | The entire module's engine path: one-line delegation to `lumen_cli::dispatch::lumen_dispatch` — reuse, not re-implementation, verified. |
| `open_file_request` | 220-231 | REAL | — | Catalog's own wire form (SessionOpen, default options). |
| `playlist_load_request` | 238-246 | REAL | — | append:false documented and asserted in tests. |
| `open_path_request` | 253-261 | REAL | — | Playlist-before-media order; honest None for anything else (the wave-prompt's "subtitles refused at dispatch": a non-media path is a no-op HERE; the catalog-level subtitle-command refusals live in lumen_cli::dispatch — cross-ref). |
| `tests::RecordingSink` + `request` (helpers) | 271-286 | REAL | — | seq-collecting sink (under-used, F-002). |
| `unserved_commands_refuse_typed_and_echo` | 293-310 | REAL | — | Ran ok (log:2270). Typed refusal + request_id echo. |
| `served_commands_reach_the_engine_and_answer` | 316-328 | REAL | — | Ran ok (log:2269). Real catalog listing; no-session None context asserted. |
| `events_forward_in_order_exactly_once` | 335-356 | REAL | S4 (F-002) | Ran ok (log:2293). Real fixture + real dispatch; order unasserted. |
| `file_open_funnel_is_the_catalog_command` | 361-373 | REAL | — | Ran ok (log:2267). Structural pin. |
| `open_path_request_classifies_by_kind` | 379-400 | REAL | — | Ran ok (log:2268). mkv/m3u/txt three-way classification. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| unserved_commands_refuse_typed_and_echo | YES | Asserts specific code + command detail + request_id echo on the refusal path. |
| served_commands_reach_the_engine_and_answer | YES | Real command listing, non-empty, plus session_id None. |
| events_forward_in_order_exactly_once | PARTIAL | Catches double-delivery (second==0) and empty drains (first>0); does NOT catch reordering (F-002). |
| file_open_funnel_is_the_catalog_command | YES | Exact wire-form match on command variant + path. |
| open_path_request_classifies_by_kind | YES | All three classes with concrete extensions; a classifier regression fails at least one arm. |
| (external) shell::tests::egress_queue_is_bounded_drop_oldest_and_conserves | YES | Overflow + conservation identity over this file's sink (log:2292). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one command, `lumen_dispatch`, and nothing else ever registered on the invoke handler" | dispatch.rs:1-3 | REAL | app.rs:293 registers exactly one (rg-verified single generate_handler site). |
| "an architecture test reads the shell's invoke_handler and fails if any command other than lumen_dispatch is registered" | dispatch.rs:10-13 | REAL-UNPROVEN | Test exists (desktop-tauri/tests/single_command.rs, source-scanning, 2 tests) but its crate is workspace-excluded and the tauri build FAILED in baseline #2 — no recorded execution. The doc honestly marks it "a CI-row, not a sandbox row". |
| "each event is handed to the EventSink exactly once, in engine order" | dispatch.rs:51-52, 96-99 | REAL-UNPROVEN (once-only) / UNVERIFIED (order) | Once-only is code-correct (append-only + cursor, verified) and test-pinned; ORDER has no assertion (F-002) and no executed proof. |
| "Reuse, not re-implementation (ARCH-5): the engine + session wiring is the SAME host the CLI drives" | dispatch.rs:37-43 | REAL | dispatch.rs:212 delegates; ShellHost embeds `lumen_cli::host::CliHost` (line 67); no engine calls elsewhere in the file (rg-verified). |
| "the mailbox pattern's conservation discipline, applied to engine events" (EventQueueStats) | dispatch.rs:120-121 | REAL | Identity correct; executed via shell::tests (log:2292). |

## 7. Dead / duplicated / unreachable code
- No dead code. No duplication (request-builders differ honestly; `offer`'s redundant `is_some()` noted in the table).
- Everything in the file has a consumer: dispatch path (shell.rs:46), funnel (shell.rs:302), egress (shell.rs:104/130/233), tests.

## 8. Dependencies & flags
- `lumen_cli` (CliHost, dispatch) — the desktop→CLI reuse edge, declared in apps/desktop/Cargo.toml (audited separately as Cargo.toml.audit.md). `commands` (envelope/catalog types). `std::collections::VecDeque`, `NonZeroUsize`. No feature gates, no unsafe, no tauri import in this half (the whole point — webkit-free, verified: no `tauri::` in this file).

## 9. Open questions for the next agent
- host.rs owner (CLI wave): confirm `state_history` (host.rs:326) shares the never-pruned pattern of `events` — F-001's blast radius doubles if so.
- shell.rs owner (W13-A01, part1): the §4.2 staleness filter that feeds `note_stale` — verify the filter actually runs before `offer` in the pump path (shell.rs:226-233), i.e., that `dropped_stale` counts exist in a real session (the conservation test covers overflow, not staleness).
- lumen_cli::dispatch owner: the served-command table (which commands answer vs refuse `unsupported-for-host`) is the desktop's real capability surface — the wave prompt's "subtitles are refused at dispatch" lands there, not here.
