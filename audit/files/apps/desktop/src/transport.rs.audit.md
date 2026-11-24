# AUDIT: apps/desktop/src/transport.rs
| Field | Value |
|---|---|
| Auditor | W13-A07 |
| Date | 2026-11-23 |
| Lines / functions | 278 / 18 (trait + 3 impls + 3 impl-block helpers + error plumbing + status fn) |
| File verdict | REAL |
| Reachable from shipped entry point? | SPLIT: `WebviewHostKind` YES (lumen-desktop bin main.rs:16,66-68 — executed); `MailboxTransport` YES via apps/desktop-tauri/src/app.rs:93 → shell.rs:109-183 FrameTap (GUI lane — NOT buildable in this sandbox, gdk-sys; UNVERIFIED by execution there), exercised directly by apps/desktop/tests/zero_alloc_transport.rs (executed here); release-lane `lumen-desktop` bin never constructs a transport (main prints status, exits 86); `NullTransport`/`CaptureTransport`/`ConsumerFrame` NO shipped caller |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/zero_alloc_transport-360d7af48bb13431` → 2 passed / 0 failed (exit 0); `./target/debug/lumen-desktop` → status report + `EXIT=86` (typed pending); rg for consumers (NullTransport/ConsumerFrame/CaptureTransport). Binaries dated 2026-11-23 18:07, newer than source 17:35. |

## 1. One-paragraph verdict
An honest, small seam file. The `FrameTransport` contract (never block the presenter; typed failures) is upheld by the one real implementation: `MailboxTransport::send` decodes the record with a single reused `WireDecoder` and offers to the bounded latest-wins `FrameMailbox`, whose `offer` is O(1) and drops-oldest on overflow (mailbox.rs:142-153) — no blocking path exists, and the zero-allocation claim is proven by a test I executed (2/2 pass). The pending webview hosts are typed enumerations with truthful status strings, printed verbatim by the shipped `lumen-desktop` binary (executed, exit 86). The rot is dead code: `NullTransport` has zero constructors anywhere in the tree (its "throughput evidence" role is historical), `ConsumerFrame` is an unre-exported alias with zero users, and `CaptureTransport` ships in the production lib while only `#[cfg(test)]` code constructs it.

## 2. What actually works (verified)
— `./target/debug/deps/zero_alloc_transport-360d7af48bb13431` → `mailbox_frame_path_is_zero_allocation_after_warm_up … ok`, `streaming_decoder_path_is_zero_allocation_after_warm_up … ok` — the C7 zero-alloc claim (one decoder, in-place decode, pooled buffers) is measured, not asserted.
— `./target/debug/lumen-desktop` → prints `tauri-ipc pending ADR-004: tauri/wry not imported …`, `lumen-proto pending ADR-004 + wire protocol v2 …`, `shared-mem pending wire protocol v2 …` (transport.rs:261-272 strings verbatim), exit 86 — the honest pending state, executed.

## 3. Findings
F-001 [S3]
Where: apps/desktop/src/transport.rs:93-132
Code:
```
/// The measurement transport: accepts and discards, counts bytes and
/// records (the encode-side half of the throughput evidence in
/// `docs/prototypes/video-in-webview.md` §4).
```
What is wrong: `NullTransport` has zero constructors in the entire tree (rg over apps/ + crates/ → only its definition and the lib.rs:111 re-export). Its documented purpose points at prototype-era evidence; in the current tree it is dead code compiled into the shipped lib. Why it matters: dead public API in the transport seam invites cargo-cult use; the "throughput evidence" citation has no live producer. What correct looks like: delete it or gate it behind the prototype/bench harness that actually measures. Confidence: HIGH — Verification: read (exhaustive rg).
F-002 [S4]
Where: apps/desktop/src/transport.rs:276-278
Code:
```
/// What the consumer side of a [`ShippedFrame`] hand-off observes
/// (re-exported for transport users; see `mailbox.rs` for the type).
pub type ConsumerFrame = ShippedFrame;
```
What is wrong: the comment says "re-exported for transport users" but it is NOT in lib.rs's re-export list (lib.rs:110-113) and has zero users anywhere. Dead alias + false comment in one line. Confidence: HIGH — Verification: read (rg).
F-003 [S4]
Where: apps/desktop/src/transport.rs:134-165
Code:
```
/// The conformance transport: keeps every encoded record (bounded by
/// the caller's discipline — it is a test/fixture type, never wired
/// into a playback session).
```
What is wrong: honest doc, but the type ships un-gated in the production lib while its only constructors are bridge.rs `#[cfg(test)]` tests (bridge.rs:291,327,355,374,394). Test-fixture code in the shipped surface. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FrameTransport::send (trait) | 40-50 | REAL | — | contract: never block, typed failure |
| FrameTransport::name (trait) | 47-49 | REAL | — | capability-reporting name (ADR-003 d4) |
| TransportError + Display/Error/From | 55-91 | REAL | — | small typed surface; From<MailboxError> wired |
| NullTransport::{new,records,bytes,send,name} | 96-132 | UNREACHABLE | S3 | F-001; implementation itself correct (counters) |
| CaptureTransport::{new,records,send,name} | 137-165 | UNREACHABLE (test-only consumers) | S4 | F-003; unbounded Vec, documented caller discipline |
| MailboxTransport::new | 197-202 | REAL | — | propagates zero-capacity misconfiguration typed |
| MailboxTransport::{mailbox, mailbox_mut} | 204-214 | REAL | — | consumer-side accessors |
| MailboxTransport::{send, name} | 217-237 | REAL | — | reused WireDecoder; parse failure → Hosted (never ships garbage); offer never blocks (mailbox.rs:148-151 drop-oldest) |
| WebviewHostKind + status + GATING_ADR | 242-274 | REAL | — | executed via lumen-desktop; truthful pending strings |
| ConsumerFrame | 276-278 | UNREACHABLE | S4 | F-002 |

## 5. Test honesty
No tests inside this file. Consumers' tests: zero_alloc_transport.rs (2 tests, executed, allocation-counting global allocator — would fail on a per-send allocation regression: genuine); bridge.rs tests use CaptureTransport to assert record contents (would fail on encode/bridge bugs). WebviewHostKind has no test but is executed by the shipped binary's output (verified by run).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "send never blocks the presenter … absorbed by buffering inside the transport" | transport.rs:30-39 | REAL | MailboxTransport::send → offer (mailbox.rs:142-153): O(1), drop-oldest, no waiting |
| "one WireDecoder lives for the transport's whole life … zero allocation after warm-up (measured by tests/zero_alloc_transport.rs)" | transport.rs:172-179 | REAL | decoder field at 188, constructed once at 200; test executed 2/2 ok |
| "A record the protocol cannot parse … surface it as a hosted transport failure rather than shipping garbage" | transport.rs:219-224 | REAL | send maps WireError → TransportError::Hosted (222-230) |
| NullTransport "the encode-side half of the throughput evidence in docs/prototypes/video-in-webview.md §4" | transport.rs:93-95 | STALE | zero live constructors (F-001) |
| "re-exported for transport users" (ConsumerFrame) | transport.rs:276-277 | FALSE | not in lib.rs re-export; zero users (F-002) |
| Tauri/WebSocket/SAB transports pending ADR-004 | transport.rs:10-22, 243-274 | REAL | executed: lumen-desktop prints pending statuses, exit 86; desktop-tauri build fails at gdk-sys in sandbox (AGENT-SPEC §13, environment limit) |

## 7. Dead / duplicated / unreachable code
- NullTransport (F-001), ConsumerFrame (F-002), CaptureTransport-in-prod-lib (F-003).
- No duplication found.

## 8. Dependencies & flags
- crate::{error::WireError, mailbox, wire::WireDecoder} — internal only, no external crates, no feature gates. Compiled into release lane via `-p desktop` (baseline: release build exit 0).

## 9. Open questions for the next agent
- Whoever owns apps/desktop-tauri: confirm ShellRuntime actually constructs FrameTap in the GUI lane (app.rs:93 imports ShellRuntime; frame_tap construction read at shell.rs:176-183) once a GTK-capable environment exists — currently UNVERIFIED (gdk-sys, sandbox limit).
- If NullTransport is kept for future throughput evidence, wire it or #[cfg] it out of the shipped lib.
