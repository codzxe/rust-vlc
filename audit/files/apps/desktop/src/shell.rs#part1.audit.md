# AUDIT: apps/desktop/src/shell.rs (part1: lines 1-375 of 872 — all production code; part2 = the `#[cfg(test)] mod tests` at 376-872, assigned to W13-A09)
| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-21 |
| Lines / functions | 375 / 25 items (module docs; consts ×2; enums ×1; structs ×2; impls/methods ×17; free fn ×1; trait impl ×1) |
| File verdict (part1) | REAL |
| Reachable from shipped entry point? | YES by code, headless-verified only: production consumer is the tauri wrapper — `apps/desktop-tauri/src/app.rs:113 ShellState(pub Mutex<ShellRuntime>)`, `:144-153` the single `lumen_dispatch` command → `runtime.dispatch`, `:161-171 forward_pending` → `pump_events`/`take_pending_events`, `spawn_event_pump` → `pump_session`. That wrapper is workspace-EXCLUDED and its build is blocked in this sandbox (baseline #2 exit 101, missing gdk/gtk — INFRA-BLOCKED, infra limitation not repo defect), so no binary execution exists; in-workspace, apps/desktop lib tests execute ShellRuntime end-to-end with the real engine: 13 `shell::tests` green (orchrun-test-workspace-nofailfast.log.txt:2288-2320). The `lumen_desktop` binary (apps/desktop/src/main.rs) does NOT use ShellRuntime — it is the ADR-004 skeleton printer exiting 86. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | full Read of all 872 lines; `rg "ShellRuntime"` repo-wide → sole production consumer apps/desktop-tauri/src/app.rs; read app.rs:25-70, 95-200 (lock, single command, ticker); `rg "fn forward_new_events\|fn observe_response\|fn deliver_event\|fn open_path_request"` apps/{desktop,cli} → signatures (NOT Results — see F-001 note); read crates/commands/src/correlation.rs:108-170 (`DeliveryLog`: unbounded `violations: Vec<String>`, one push per unallocated-id response); `rg "requestId"` ui/src → JS self-allocation (ui/src/client/config-settings.ts:88-93 `nextRequestId += 1`; 15+ call sites incl. PlaylistProvider.tsx, BookmarksDialog.tsx, player/context.tsx); `rg "\.violations\(\)"` apps → host.rs tests ONLY, no production reader; `rg "RENDER_QUANTUM" apps/cli/src/pipeline_host.rs` → 1470 (PACER_TICK = 30 625 µs ✓ test pin); read apps/desktop/src/frame_tap.rs:56-71 + mailbox.rs Default (capacity NonZero 2) + transport.rs:197-207 (only zero-capacity fails) for the unreachable! analysis; `rg "shell::tests"` nofailfast → 2288-2320 all ok; sed -n verification of every quote. |

## 1. One-paragraph verdict
This is the GUI composition root's real engine seam and it is honestly built: one dispatch path (`dispatch` → `lumen_dispatch` → observe → bounded event drain), a derived-not-invented pacer cadence (`PACER_TICK` = RENDER_QUANTUM 1470 @ 48 kHz = 30 625 µs, pinned by test), the frame-channel interception documented as shell-owned state, and file-open funnels that all route through the same single command — with 13 green tests including two end-to-end proofs (a real AVI session driven to `Ended` by pacer ticks alone, and LUF1 frame records flowing through the same pump). The one systemic defect is at the composition point this file owns: `dispatch` discards the §4.3 echo-legality result (`let _ = observe_response`), and on the REAL UI path the webview allocates request ids in JavaScript (`nextRequestId += 1`) while the host's `DeliveryLog` only knows ids minted by `next_request_id` (drops/argv opens) — so every button-click command records a false "response for unallocated request" violation into an unbounded `Vec<String>` that no production code ever reads: doc 05 §4.3's echo guarantee is dead-on-arrival for the shipped desktop UI, with a slow unbounded memory leak as a side effect. Second-tier: the frame-channel bind bypasses the §4.3 bookkeeping entirely (returns before `observe_response`), an `unreachable!` panic path guards a config that is type-system-valid today but is a UI-crash if `FrameTap::new` ever grows a failure mode, and the module doc cites a nonexistent path (`apps/desktop/tauri/INFRA-BLOCKED.md`).

## 2. What actually works (verified)
- 13/13 `shell::tests` pass in the baseline rerun (orchrun-test-workspace-nofailfast.log.txt:2288-2320), including `pacer_proof_pump_once_drives_a_real_session_to_end` (:2320) — a real AVI fixture opened through `open_path` → the single `lumen_dispatch`, played, then driven to `Ended` by `pump_once` ticks only — and `frame_channel_proof_pump_once_ships_luf1_records` (:2298) — bind-first, open, play, LUF1 records drained through the pacer. These execute THIS file's `dispatch`/`open_path`/`pump_once`/`take_frame_records`/`take_pending_events` against the real engine.
- `PACER_TICK` arithmetic: 1470 × 1 000 000 / 48 000 = 30 625 µs — matches the pinned test (shell.rs:753) and the CLI host's own quantum constant (pipeline_host.rs:92). — verified by read + arithmetic.
- The single-writer claim (pump_session doc, 274-278): the wrapper enforces it — `ShellState(pub Mutex<ShellRuntime>)` (app.rs:113), the single `lumen_dispatch` command locks before `runtime.dispatch` (app.rs:148-153), the ticker locks before `pump_session` (app.rs:180+). — verified by read of the wrapper source (execution UNVERIFIED: tauri build blocked in sandbox).
- `let _ = self.shell.forward_new_events(&mut deliver)` (233) discards a `usize` count, NOT an error (dispatch.rs:100 returns usize) — no swallowed error there. — verified by signature read.
- `DEFAULT_EVENT_CAPACITY`'s `None => loop {}` is a compile-time dead end (const eval), and `NonZeroUsize::new(64)` is always `Some`. — verified by read.
- The bounded-egress discipline (drop-oldest, counted, conserves) is pinned by `egress_queue_is_bounded_drop_oldest_and_conserves` (nofailfast:2292). — test-cited.

## 3. Findings

### F-001 [S2]
Where: apps/desktop/src/shell.rs:163-165 (with cross-refs ui/src/client/config-settings.ts:88-93, apps/desktop-tauri/src/app.rs:144-153, crates/commands/src/correlation.rs:137-147)
Code:
```
        let response = lumen_dispatch(&mut self.shell, request);
        let _ = self.host.observe_response(&response);
        self.pump_events();
        response
```
What is wrong: `observe_response` returns `ResponseObservation { log_ok, generation_bump }` where `log_ok == false` means a §4.3 echo violation was recorded — and this composition point throws it away. On the real desktop UI path the violation fires on **every command**: the webview builds complete `WireRequest`s with JavaScript-allocated ids (`ui/src/client/config-settings.ts:88-93` — `let nextRequestId = 0; const requestId = () => { nextRequestId += 1; ... }`, used at 15+ call sites: PlaylistProvider.tsx:133+, BookmarksDialog.tsx:94+, player/context.tsx:365+), the wrapper's single command passes them straight through (`app.rs:148-153` — `let response = runtime.dispatch(&request);`), and the host's `DeliveryLog` only contains ids from `ShellRuntime::next_request_id` (shell.rs:301, 313 — drop/argv opens only). `DeliveryLog::observe_response` (correlation.rs:137-147) then pushes `"response for unallocated request {id}"` into `violations: Vec<String>` — **unbounded** (correlation.rs:118-120 admits "kept small — a test/diagnostic aid" but nothing caps it) — and `violations()` has no production reader (grep: host.rs tests only).
Why it matters: (a) Doc 05 §4.3's echo-legality guarantee — which this very file's doc header and the wrapper's header cite as "guarantee 2" — is systematically false-positived on the only shipped UI path: a real violation (a duplicated/ spoofed id) is indistinguishable from the 100% baseline noise. (b) Slow unbounded memory growth for the process lifetime: one `String` per UI command (a UI polling `media.get-current-time` at 4 Hz for an 8-hour session mints ~115 k violation strings ≈ MBs), plus the `allocated`/`answered` `BTreeSet`s grow one entry per id. (c) The `let _ =` discard guarantees the shell never reacts — the one place that could reconcile the two id namespaces (e.g. allocate host-side, or treat webview ids as pre-allocated) does nothing.
What correct looks like: Either the wrapper allocates ids host-side (JS asks for an id or sends id-less requests and the shell stamps them via `next_request_id`), or the shell accepts the webview's id namespace (note it allocated before dispatch), or at minimum `log_ok == false` is surfaced/logged once instead of silently accumulating. Cap `violations` while at it.
Confidence: HIGH on the code trace (all four sites read verbatim); execution UNVERIFIED — the tauri app cannot build in this sandbox (baseline #2), so the JS→wrapper→shell id mismatch is proven by source, not by a run. REQUEST-ORCH-RUN candidate: a desktop-tauri lane on a webview-equipped host running one UI command and asserting `violations()` stays empty.

### F-002 [S3]
Where: apps/desktop/src/shell.rs:160-162 and 175-195
Code:
```
        if let Command::SessionSetFrameChannel { channel } = &request.command {
            return self.bind_frame_channel(request.request_id, *channel);
        }
```
What is wrong: The intercepted frame-channel bind returns BEFORE the §4.3 bookkeeping — `bind_frame_channel` (175-195) never calls `self.host.observe_response`, so (a) the response for the (wrapper-allocated) id is never logged as answered — the allocation stays open forever; (b) if a caller dispatches a bind with an id the host never allocated, no violation is detectable (every other command's response passes through the echo check); (c) `pump_events` is skipped (benign for a class-S `Unit`, but the asymmetry is undocumented).
Why it matters: The interception creates a second response-shaping path that silently exempts one command from the correlation discipline the file's own doc header advertises ("observe the response (§4.2/§4.3 bookkeeping)" — shell.rs:8). The test `frame_channel_bind_is_intercepted_by_the_shell` (part2, line 656) asserts the Unit completion but never checks `host.violations()` or that the id was answered, so the gap is mutation-invisible.
What correct looks like: Call `let _ = self.host.observe_response(&response);` inside the bind path too (or before the early return), and add an assertion in the bind test that the log answered the id.
Confidence: HIGH — Verification: read (both code paths + host.rs observe semantics).

### F-003 [S4]
Where: apps/desktop/src/shell.rs:179-182
Code:
```
                let tap = std::sync::Arc::new(
                    crate::frame_tap::FrameTap::new()
                        .unwrap_or_else(|err| unreachable!("the default config is valid: {err}")),
                );
```
What is wrong: A panic path (`unreachable!`) inside a UI command handler. Today it is provably dead: `FrameTap::new` → `MailboxTransport::new(MailboxConfig::default())` (frame_tap.rs:56-58), the default capacity is a compile-time `NonZeroUsize` 2 (mailbox.rs:50-62), and the only `Err` is zero-capacity `MailboxMisconfigured` (transport.rs:197-204) — impossible by type construction. But the invariant is non-local (three files away) and the workspace's own lint policy bans `panic!`/`unwrap`/`expect`; if `FrameTap::new` ever gains a failure mode (a second config field, an allocation), the webview's very first frame bind crashes the whole app.
Why it matters: Latent crash-on-bind; policy-dodging macro choice where a typed answer exists — the file itself defines `refusal_response` (359-374) for exactly the "seam fails, UI still gets a wire answer" case.
What correct looks like: Map the error through `refusal_response`-style typed envelope (bind failed → wire error), or construct the tap once in `ShellRuntime::new` where a constructor failure can refuse startup honestly.
Confidence: HIGH — Verification: read (frame_tap.rs, mailbox.rs, transport.rs chains checked).

### F-004 [S4]
Where: apps/desktop/src/shell.rs:29-30
Code:
```
//! the window build itself — webkit2gtk-4.1 is absent from this sandbox
//! (ADR-004; `apps/desktop/tauri/INFRA-BLOCKED.md`). The runtime is
```
What is wrong: The cited path does not exist — the file lives at `apps/desktop-tauri/INFRA-BLOCKED.md` (the crate was renamed/moved per ADR-004; the workspace `exclude = ["apps/desktop-tauri"]`, root Cargo.toml:51). `apps/desktop/tauri/` is not a directory in the tree.
Why it matters: A reader chasing the deferral record hits a dead path; stale cross-references in the honesty-deferral doc undermine the deferral's auditability.
What correct looks like: Fix the path to `apps/desktop-tauri/INFRA-BLOCKED.md`.
Confidence: HIGH — Verification: read + `ls apps/desktop` (no `tauri/` subdir) + file exists under apps/desktop-tauri/.

### F-005 [S4]
Where: apps/desktop/src/shell.rs:136-141 and 317-323
Code:
```
    pub fn engine_host(&mut self) -> &mut lumen_cli::host::CliHost {
        self.shell.engine_host()
    }
```
```
            Err(error) => {
                // The seam's own typed failure: surfaced as the wire
                // error envelope's shape demands a response — the
                // honest refusal, echoed for the allocated id.
                Some(refusal_response(request_id, &error))
            }
```
What is wrong: Two hygiene warts. (a) `engine_host()` hands out `&mut CliHost` publicly — a bypass around the "every command flows through dispatch" claim the doc makes (shell.rs:136-137); the tauri anti-bypass test (`apps/desktop-tauri/tests/single_command.rs`) checks handler registration, not this accessor. (b) `on_files_dropped` maps ANY `DragDropHandler` error to `ErrorCode::NotReady` + `Stage::Control` — semantically "no session" — when the production handler (`PathDragDrop`, files.rs:103-114) has no `Err` path at all, so the arm is dead in production and the code choice is unreviewable against a real failure.
Why it matters: (a) one `&mut` escape hatch undoes the single-command-surface discipline if a future wrapper contributor uses it; (b) a dead error arm with a debatable code invites cargo-cult mapping of real errors to the wrong code later.
What correct looks like: (a) gate `engine_host` behind `#[cfg(test)]` or a doc'd diagnostic feature; (b) pick `UnsupportedForHost`/`Internal` per the actual failure taxonomy once one exists, or assert the production handler's infallibility in its type (`Infallible` error).
Confidence: HIGH — Verification: read + files.rs impl read.

## 4. Per-function table (part1)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-32 | REAL | S4 | honest deferral section; F-004 stale path; "every method runs under cargo test" — verified true (13 tests green) |
| `DEFAULT_EVENT_CAPACITY` | 54-65 | REAL | — | 64; const-eval dead-end arm — sound |
| `PACER_TICK` | 67-81 | REAL | — | 1470/48 kHz = 30 625 µs, derived + test-pinned; doc's 44.1 kHz tolerance argument sound (pump_session is wall-bounded) |
| `enum DialogOutcome` | 83-97 | REAL | — | boxed WireResponse per clippy — fine |
| `struct ShellRuntime` | 99-115 | REAL | — | host + shell + events + frame_tap/frame_channel |
| `Default for ShellRuntime` | 117-121 | REAL | — | delegates to new(DEFAULT_EVENT_CAPACITY) |
| `ShellRuntime::new` | 123-134 | REAL | — | plain constructor |
| `engine_host` | 136-141 | REAL | S4 | F-005(a) pub bypass accessor |
| `next_request_id` | 143-147 | REAL | — | monotonic from 1; notes allocation — the ONLY legal id source per §4.3, which F-001 shows the UI ignores |
| `dispatch` | 149-167 | REAL | S2 | F-001 (discarded §4.3 result, id-namespace mismatch); F-002 (bind early-return) |
| `bind_frame_channel` | 169-195 | REAL | S3/S4 | F-002 (no observe_response); F-003 (`unreachable!`); tap reuse + re-bind semantics correct |
| `take_frame_channel_binding` | 197-204 | REAL | — | one-shot `Option::take` — correct |
| `frame_channel_bound` | 206-212 | REAL | — | tap-existence OR — correct post-take semantics |
| `take_frame_records` | 214-224 | REAL | — | delegates to tap drain; 0 when unbound |
| `pump_events` | 226-234 | REAL | — | `let _ =` discards a usize count (not an error) — verified signature |
| `pump_once` | 236-261 | REAL | — | full-ring pump; headless-proof semantics documented; production uses pump_session — both documented and tested |
| `pump_session` | 263-282 | REAL | — | wall-bounded advance; single-writer claim verified in wrapper source (execution UNVERIFIED — tauri build blocked) |
| `take_pending_events` | 284-288 | REAL | — | drains queue |
| `event_stats` | 290-294 | REAL | — | exposes conservation stats |
| `open_path` | 296-304 | REAL | — | funnel: classify → dispatch; unclassifiable → None (honest no-op, tested) |
| `on_files_dropped` | 306-324 | REAL | S4 | F-005(b) NotReady mapping dead with production handler |
| `pick_and_open` | 326-340 | REAL | — | Canceled/Pending/Opened trichotomy — correct; tested with scripted dialog |
| `struct DeliverThrough` | 343-348 | REAL | — | composition adapter |
| `EventSink for DeliverThrough` | 350-357 | REAL | — | stale → note_stale, kept → forward — correct |
| `refusal_response` | 359-374 | REAL | — | typed wire error for seam failures; Generation(0)/session None fine pre-session |

## 5. Test honesty
Tests live in part2 (376-872, W13-A09). Cross-boundary notes from my read of them for reachability evidence only: 13 tests, all green (nofailfast:2288-2320); the two proof tests (pacer, LUF1) are genuinely end-to-end (real fixture, real engine, no mocks). Coverage gap relevant to part1: no test asserts `host.violations()` stays empty after `dispatch` (would have caught F-001 if the UI id protocol were routed through the runtime), and the bind test does not assert the §4.3 log answered the id (F-002 invisible).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one UI-originated command: … observe the response (§4.2/§4.3 bookkeeping)" | shell.rs:5-9 | SHALLOW | bookkeeping runs but its result is discarded (F-001) and one command bypasses it (F-002); on the real UI path the §4.3 half is systematically false-positived |
| "the engine's single-writer invariant holds (… poll runs only under the lock)" | shell.rs:274-277 | REAL-UNPROVEN | wrapper source enforces it (app.rs:113,148-153, ticker lock); execution unverifiable — tauri build blocked (baseline #2) |
| "every method here runs under `cargo test` today" | shell.rs:31-32 | REAL | 13 shell::tests green (nofailfast:2288-2320) |
| "the pacer test drives a session to `Ended` through it" | shell.rs:252-253 | REAL | `pacer_proof_pump_once_drives_a_real_session_to_end ... ok` (nofailfast:2320) |
| "PACER_TICK … 30 625 µs" | shell.rs:67-81 | REAL | 1470×1e6/48 000 = 30 625; pinned by `pacer_tick_is_one_render_quantum` (nofailfast:2299) |
| "overflow drops the OLDEST event, counted — order is never violated" | shell.rs:54-56 | REAL | pinned by `egress_queue_is_bounded_drop_oldest_and_conserves` (nofailfast:2292) |
| "webkit2gtk-4.1 is absent … `apps/desktop/tauri/INFRA-BLOCKED.md`" | shell.rs:29-30 | FALSE (path) | F-004: actual file apps/desktop-tauri/INFRA-BLOCKED.md |
| doc 05 §4.3 guarantee 2 (echo legality) as consumed by this shell | docs/05-command-layer.md §4.3, cited via app.rs:31 | FALSE on the UI path | F-001: JS-allocated ids → every response a recorded violation, never surfaced |

## 7. Dead / duplicated / unreachable code
None dead. `DialogOutcome::Pending` is reachable only in not-linked builds (by design, ADR-004); `on_files_dropped`'s Err arm is dead with the production handler (F-005(b)); `engine_host`'s pub visibility is a live escape hatch (F-005(a)).

## 8. Dependencies & flags
- Depends on `commands` (envelope/catalog), `lumen_cli::host::CliHost` + `pipeline_host::RENDER_QUANTUM` (the apps/cli dependency — the architecture edge flagged by baseline #11 is `lumen-cli → streaming`, not this direction), `crate::{dispatch, error, files, host, frame_tap}` — all used. No unsafe. No feature gates. `apps/desktop` is a workspace member (built + tested in baseline #1/#3).

## 9. Open questions for the next agent
- W13-A09 (shell.rs part2 + dispatch/host owners): does any test anywhere drive `dispatch` with an id NOT from `next_request_id` and assert the violation is recorded? If yes, the F-001 mechanism is at least test-observable; if no, the §4.3 machinery is entirely untested on the real protocol shape.
- Whoever owns apps/desktop-tauri (build-gated): F-001's fix belongs in the wrapper/shell contract (id stamping); the REQUEST-ORCH-RUN for a webview-equipped lane would settle execution.
- commands/correlation.rs owner: cap `violations` (and note `allocated`/`answered` BTreeSets also grow per-id forever — one entry per command id, ~8-16 bytes each).
