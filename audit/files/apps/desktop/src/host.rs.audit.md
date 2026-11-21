# AUDIT: apps/desktop/src/host.rs
| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-21 |
| Lines / functions | 396 / 25 items |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen-tauri` → `#[tauri::command] lumen_dispatch` (apps/desktop-tauri/src/app.rs:145-155) → `ShellRuntime::dispatch` (apps/desktop/src/shell.rs:153) → `DesktopHost::{next_request_id, observe_response, deliver_event}`; `DesktopHost::active_presentation_path` is called by the `lumen-desktop` skeleton binary itself (apps/desktop/src/main.rs:61-62). All 5 unit tests passed in the PRIMARY baseline run (test-workspace.log.txt:2320-2324) — this file's tests, unlike lumen-cli's, ran before the zero_alloc_transport abort.) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `rg -n "DesktopHost|observe_response|deliver_event" apps/desktop/src apps/desktop-tauri/src` → ShellRuntime wiring (shell.rs:102,128,145-147,164,352) and tauri app.rs:145-155; `rg -n "flush_stale|generation_bump" apps/desktop/src apps/desktop-tauri/src apps/desktop/tests` → flush_stale callers: mailbox.rs (definition) + tests/pipeline.rs ONLY — zero production callers; Read crates/commands/src/correlation.rs:50-164 (StalenessFilter raises watermark from ANY response; DeliveryLog flags `seq <= last` only — gaps are NOT violations, so stale-dropped events create no phantom violations); `rg host::tests baseline/test-workspace.log.txt` → 5/5 ok; Read docs/adr/ADR-004-os-webview.md:1-12 (ACCEPTED 2026-10-22; per-OS verdicts updated 2026-11-13, "BUILT and LAUNCHED on all three OSes") |

## 1. One-paragraph verdict
The correlation half is real and tested: monotonic request ids with echo-validity violations, a per-session generation watermark with bump detection, a strict staleness filter for events, and an honestly-typed never-fake webview pending state — all five tests are mutation-breaking and passed in the primary baseline run. The headline defect is a wiring lie anchored in this file's own contract docs: `generation_bump` is documented as "the signal the wiring turns into a frame-mailbox flush (`flush_stale`)" — but the production wiring (`ShellRuntime::dispatch`, shell.rs:164) discards the entire `ResponseObservation` with `let _ =`, and `flush_stale` has zero production callers (only the integration test calls it, manually). Every seek/stop/track-switch on the desktop shell can therefore ship a stale in-flight frame to the webview. Secondarily, an echo-invalid ("ghost") response still mutates the staleness watermark before the §4.3 verdict lands, and the "ADR-004 pending" status strings are stale — the repo's own ADR-004 was accepted 2026-10-22 and the shell built and launched per its verdicts table.

## 2. What actually works (verified)
- 5/5 `host::tests` passed in the primary baseline run — verified by `rg "host::tests" audit/runs/baseline/test-workspace.log.txt` → lines 2320-2324.
- `DesktopHost` is the live correlation state of the shipped shell: `ShellRuntime` holds it (shell.rs:102,128); the tauri command's `runtime.dispatch(&request)` (app.rs:151) reaches `observe_response`/`deliver_event` on every UI command.
- Staleness/seq semantics cross-checked against the commands crate: `filter_event` drops strictly-lower generations and raises the watermark on current ones; `DeliveryLog::observe_event` flags only `seq <= last`, so this host's stale-drops (whose seq never reaches the log) correctly produce no phantom violations — the code comment at 188-190 is accurate.

## 3. Findings

### F-001 [S2] The generation-bump mailbox flush is documented as wired but exists only in a test — stale in-flight frames survive every class-A command
Where: apps/desktop/src/host.rs:121-126 (claim); the unwired consumer is apps/desktop/src/shell.rs:163-166
Code (claim, host.rs):
```rust
    /// A generation raise on a pipeline-scoped session — the signal
    /// the wiring turns into a frame-mailbox flush (`flush_stale`):
    /// frames in flight under the old generation are stale by
    /// construction.
    pub generation_bump: Option<(SessionId, Generation)>,
```
Code (reality, shell.rs:163-166):
```rust
        let response = lumen_dispatch(&mut self.shell, request);
        let _ = self.host.observe_response(&response);
        self.pump_events();
        response
```
What is wrong: the production dispatch path computes the `ResponseObservation` (echo validity + generation bump) and throws it away. `rg "flush_stale"` across `apps/desktop/src` and `apps/desktop-tauri/src` finds callers only in `mailbox.rs` (the definition and its own tests) and `apps/desktop/tests/pipeline.rs:299` — where the TEST performs the flush manually ("// The wiring flushes on the signal. let flushed = bridge.transport_mut().mailbox_mut().flush_stale();"). The LUF1 `FrameHeader` carries no generation field (only a monotonic `frame_id` that continues across seeks), so neither the webview nor the ticker can filter stale frames; the host-side flush is the only mechanism for in-flight presented frames, and it is not wired.
Why it matters: on every class-A acceptance (seek/stop/track switch), frames already presented into the `FrameMailbox` (capacity 2 by default) under the old generation remain queued; the tauri ticker drains the mailbox and ships the LAST record each tick (shell.rs:214-218) — so up to one stale pre-seek frame is drawn by the webview after a seek lands. Wrong behavior in the most common interactive action, plus a documented safety mechanism that silently does not exist. (The engine-side purge — pipeline_host `seek_boundary`/`video_hold` — covers frames not yet presented; it cannot recall frames already handed to the tap.)
What correct looks like: `ShellRuntime::dispatch` should consume the observation: `if let Some((_, gen)) = obs.generation_bump { self.frame_tap flush via flush_stale() }` (the tap's mailbox is reachable the same way the test reaches it), or the doc must stop claiming the wiring flushes.
Confidence: HIGH — Verification: read (full call chain tauri app.rs:151 → shell.rs:153-166 → host.rs observe_response; `rg flush_stale` proves zero production callers; FrameHeader layout checked in wire.rs:182-204 — no generation field).
Cross-ref: W13-A01 owns shell.rs#part1 (the `let _ =` site); W13-A04 owns mailbox.rs (`flush_stale`).

### F-002 [S4] A ghost (echo-invalid) response still raises the staleness watermark and can mint a generation bump before the §4.3 verdict is known
Where: apps/desktop/src/host.rs:167-180
Code:
```rust
    pub fn observe_response<R>(&mut self, response: &CommandResponse<R>) -> ResponseObservation {
        let before = response
            .session
            .and_then(|session| self.staleness.watermark(session));
        self.staleness.observe_response(response);
```
What is wrong: `StalenessFilter::observe_response` (correlation.rs:59-70) raises the watermark from ANY response's accepted generation with no echo-validity check; the echo verdict (`log_ok`, host.rs:180) is computed last and does not gate anything. A response whose `request_id` was never allocated — a §4.3 violation this host records — still advances the §4.2 watermark and can produce `generation_bump: Some(..)`. No test combines a ghost id with a generation raise (`ids_are_monotonic…` uses a ghost at the SAME generation 1 as the legal response, so the bump path is not exercised for ghosts).
Why it matters: the two documented contract layers (§4.2 watermark, §4.3 echo validity) are described as independent validations, but state from a contract-violating response is admitted into the filter that gates user-visible event delivery; a buggy or hostile engine response could both trigger spurious flush signals and raise the staleness bar.
What correct looks like: compute `log_ok` first and skip (or flag) the watermark raise for echo-invalid responses; add the ghost+raise test.
Confidence: MEDIUM — Verification: read (semantics traced through correlation.rs; the combined case unexecuted — no test covers it).
Cross-ref: the commands crate (StalenessFilter) is another wave's file; the ordering choice lives here.

### F-003 [S4] "ADR-004 pending" status strings are stale — the repo's own ADR-004 was accepted and the shell built and launched
Where: apps/desktop/src/host.rs:212-214 and 222-224
Code:
```rust
    pub fn active_presentation_path() -> &'static str {
        "skeleton: canvas-stream protocol core only — no webview host compiled (ADR-004 pending; docs/prototypes/video-in-webview.md)"
    }
```
(and `open_webview` returning `HostError::WebviewPending { adr: "ADR-004" }` at 222-224)
What is wrong: `docs/adr/ADR-004-os-webview.md` says "**Status:** accepted (2026-10-22 …); Per-OS webview verdicts (updated 2026-11-13) … the admission is now BUILT and LAUNCHED on all three OSes". The tauri wrapper exists (`apps/desktop-tauri`, INFRA-BLOCKED.md "ADMITTED (code half) — 2026-11-13"). The crate-level truth ("this desktop crate compiles no webview host" — it is workspace-excluded) is fine, but attributing that to "ADR-004 pending" misstates the decision state; a user reading the skeleton's output or the typed error would believe the webview decision is still open.
Why it matters: this file's stated purpose is "truthful capability reporting as data" (module docs, line 13-17); a stale decision-status string in the honesty surface is precisely the drift this audit exists to catch.
What correct looks like: "no webview host compiled into this crate (ADR-004 accepted 2026-10-22; the wrapper lives in apps/desktop-tauri, workspace-excluded)".
Confidence: HIGH — Verification: read (ADR-004 header quoted; INFRA-BLOCKED.md header quoted).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-17 | REAL (1 claim FALSE) | S2/S4 | "the generation-bump signal that the wiring turns into a frame-mailbox flush" — F-001. |
| `PresentationVariant` | 29-41 | REAL | — | Data-only enum; PrototypePending honest. |
| `PresentationVariant::label` | 46-52 | REAL | — | Static labels. |
| `TargetOs` | 57-64 | REAL | — | — |
| `TargetOs::webview` | 69-75 | REAL | — | Static strings. |
| `StrategyReport` | 80-87 | REAL | — | — |
| `strategy_for` | 93-113 | REAL | — | Per-OS table matches ADR-003 story; statuses honest about the sandbox ("unmeasured in this sandbox"). External facts ("Tauri >= 2.12.1 drawsBackground fix", "WebCodecs compiled into Debian's 2.52.6") UNVERIFIABLE from the repo — see §6. |
| `ResponseObservation` | 118-126 | REAL | S2 | F-001 claim on the `generation_bump` field. |
| `EventDelivery` | 130-136 | REAL | — | Two-state, no fake. |
| `DesktopHost` | 140-144 | REAL | — | Composition of three commands-crate types. |
| `DesktopHost::new` | 149-155 | REAL | — | — |
| `next_request_id` | 159-163 | REAL | — | Allocates + teaches the log (echo legality). Monotonic pinned by test. |
| `observe_response` | 167-185 | REAL | S4 | Bump = raise OR appear (both arms pinned by test). F-002 ghost interaction. No panic paths; `R` generic — payload-independent. |
| `deliver_event` | 191-201 | REAL | — | Stale drop BEFORE log (comment's seq-slot note verified accurate against correlation.rs:151-163 — gaps are not violations); `let _ =` on the log verdict is the documented non-gating validator, violations surfaced via `violations()`. |
| `violations` | 205-207 | REAL | — | Diagnostic accessor. |
| `active_presentation_path` | 212-214 | REAL | S4 | Honest for THIS crate; F-003 stale ADR attribution. Called by the skeleton main.rs. |
| `open_webview` | 222-224 | REAL | — | Always typed `WebviewPending` — never a fake window (2.C3). No production caller (the tauri wrapper builds its own window); intentional public seam. |
| tests: FakeCommand/FakeResult/FakeEvent | 236-240 | REAL | — | Payload types, not mocks of the subject (the subject is payload-generic). |
| tests::accepted_response | 242-256 | REAL | — | Builder. |
| tests::pipeline_event | 258-266 | REAL | — | Builder; `u32::try_from(seq)` guarded. |
| tests::ids_are_monotonic_and_responses_echo_legally | 269-289 | REAL | — | Ghost id 99 → violation containing "99". Would fail if allocator/log broke. Passed (baseline:2321). |
| tests::generation_bumps_signal_the_mailbox_flush | 292-310 | REAL | — | Pins appear-raise, no-raise, raise. NOTE: name says "signal the mailbox flush" but the test never flushes a mailbox — the flush half lives only in tests/pipeline.rs:261-299 (F-001). Passed (baseline:2320). |
| tests::stale_events_are_dropped_current_ones_kept | 313-349 | REAL | — | C6 strictly-lower pinned (equal stays current); non-monotonic seq recorded not gating. Passed (baseline:2322). |
| tests::strategy_table_is_the_adr_landing | 352-377 | REAL | — | Pins all three variants + webview strings + labels; status assertions are only non-empty (weak but the strings are quoted verbatim elsewhere). Passed (baseline:2323). |
| tests::webview_open_is_a_typed_pending_never_a_fake | 380-395 | REAL | — | Typed error + envelope round-trip. Passed (baseline:2324). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ids_are_monotonic_and_responses_echo_legally | YES | Break allocator monotonicity or the log's echo check → asserts fail. |
| generation_bumps_signal_the_mailbox_flush | YES (host half) | Break bump detection → `assert_eq!(obs.generation_bump, …)` fails. It cannot catch F-001 — the flush itself is out of the test's scope (and out of production's, which is the finding). |
| stale_events_are_dropped_current_ones_kept | YES | Invert C6 or gate delivery on seq → delivery verdicts change. |
| strategy_table_is_the_adr_landing | YES | Swap a variant → assert fails. |
| webview_open_is_a_typed_pending_never_a_fake | YES | A fake Ok would fail `err().must()`. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Evidence |
|---|---|---|---|
| "the generation-bump signal that the wiring turns into a frame-mailbox flush (mailbox.rs)" | host.rs:7-8, 121-125 | FALSE (as wired) | shell.rs:164 `let _ =`; zero production `flush_stale` callers; only tests/pipeline.rs:299. F-001. |
| "What CAN be built headlessly is exactly this half: request-id allocation, the §4.2 staleness filter, the §4.3 delivery-log validation" | host.rs:5-7 | REAL | Code + 5 passing tests (baseline:2320-2324). |
| "no webview host compiled (ADR-004 pending …)" | host.rs:213 | HALF-FALSE | Crate-level: true (workspace build has no tauri). "ADR-004 pending": FALSE — ADR-004 accepted 2026-10-22, verdicts updated 2026-11-13. F-003. |
| macOS landing "accepted per ADR-003 (sibling NSView; Tauri >= 2.12.1 drawsBackground fix)" | host.rs:97 | REAL-UNPROVEN | ADR-003 story consistent; the Tauri version/fix claim is external — UNVERIFIED in-repo. |
| Linux landing "WebCodecs compiled into Debian's 2.52.6" | host.rs:105 | UNVERIFIED | External distro fact; no in-repo evidence cited. |
| Windows "A-or-B (prototype decides)" | host.rs:99-101 | REAL-UNPROVEN | Matches PresentationVariant::PrototypePending; the deciding matrix run is future work per its own status. |

## 7. Dead / duplicated / unreachable code
- `open_webview` — no production caller (the tauri wrapper builds the window itself); intentional typed-pending seam, exercised by its test. Not dead by intent, unreachable in shipped binaries today.
- No duplication; the correlation logic correctly lives in `commands` and is composed here.

## 8. Dependencies & flags
- `commands` (envelope + correlation types), `crate::error::HostError`. No feature gates; part of the workspace `desktop` crate (built clean in baseline #1). Tests use `testkit::Must`.

## 9. Open questions for the next agent
- W13-A01 (shell.rs): F-001's `let _ = self.host.observe_response(&response)` — is the discard deliberate (e.g., the ticker's latest-wins deemed sufficient)? Nothing in shell.rs's own docs says so; host.rs's docs say the opposite.
- W13-A04 (mailbox.rs): `flush_stale` — does anything else (engine-side) flush the FrameMailbox on generation bumps? My trace says no (pipeline_host purges pre-presentation frames only).
- Engine wave: whether the ticker's ship-last-record rule truly bounds the stale leak to ≤1 frame per seek (mailbox capacity 2 → 1 shipped + 1 dropped), or whether a slow tick can ship both across two ticks.
- apps/desktop-tauri is workspace-EXCLUDED with its own lockfile — the baseline workspace runs say nothing about it; its "tauri-check" CI lane is a doc claim (INFRA-BLOCKED.md) I could not verify.
