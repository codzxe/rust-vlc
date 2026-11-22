# AUDIT: crates/playback-engine/tests/bounded_alloc.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 581 / 1 test + 12 helpers/fakes |
| File verdict | REAL — measured boundedness proof (counting allocator) of the session's command/output path under 10k-command storms, with exact cap arithmetic |
| Reachable from shipped entry point? | N/A (test binary; W00 workspace suite green). The tested subject (`Engine` outbox/inbox caps) is the shipped engine |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep only (not executed by me) |

## 1. One-paragraph verdict

The D-1 boundedness claims for the engine's command/output path,
proven at runtime rather than argued: after hub warm-up, 10k rejected
commands allocate **zero bytes** (guard.assert_zero, :377-379), and
10k accepted commands (receipt + event each) hold memory **flat**
(`delta.live_bytes == 0` with a `delta.alloc_calls > 0` sanity check
that the storm really ran, :488-499) — the transient per-command
output vecs allocate and free, the hub ring does not grow. The drop
arithmetic is asserted exactly (`10_000 − RECEIPT_CAPACITY`,
`10_000 − OUTBOX_CAPACITY`, :388-397, :503-513), the newest survivors
are delivered in submit order (:408-417, :522-529), every receipt is
the honest typed rejection/accept (:418-437, :570-580), and the
receipt→event causal adjacency survives the drops for all
`OUTBOX_CAPACITY` surviving pairs (:539-566). This is the strongest
test shape in the crate: measured, exact, and adversarial.

## 2. What actually works (verified)

— The fakes (`FakeDemuxer`, `FakeDecoder`, `FakeProvider`,
`Collector`) are seams, not mocks of the subject — the subject is the
real `Engine` + hub, driven through the real `SessionHandle` and
`Engine::poll`.
— `storm_pauses` (:305-328) polls only when the inbox refuses
(`SessionError::InboxBusy`), and both phases assert `busy > 0`
(:380-383, :484-487) — the typed overflow refusal is exercised, not
assumed.
— `poll_until` fails loudly on a hang (:294-298) — no silent timeout.
— Mutation reasoning (UNVERIFIED — no cargo): removing the receipt
cap (unbounded Vec) makes `assert_eq!(…, RECEIPT_CAPACITY)` fail at
the first warm-up drain; introducing a per-push allocation fails
`assert_zero`; breaking drop-ordering fails the `windows(2)` monotony
asserts; breaking receipt/event pairing fails the adjacency count.

## 3. Findings

### F-001 [S4] Phase-2's `live_bytes == 0` depends on the harness's transient-vec framing, which the message documents but the assert cannot distinguish
Where: bounded_alloc.rs:488-499

Code:
```
    assert_eq!(
        delta.live_bytes, 0,
        "memory is flat across the 10k-command storm (allocation events: \
         {} — the per-command output vecs, all freed; the hub ring is \
         already at its bound)",
        delta.alloc_calls
    );
```
What is wrong: nothing in the test — a note for downstream readers:
`live_bytes == 0` proves net-zero growth, not zero allocation churn
(`alloc_calls > 0` is asserted separately as the sanity). A
pathological constant-size leak smaller than the freed transients
could in principle mask behind freeing transients within the same
window. The 12k warm-up before the window makes this improbable in
practice (a leak would already show in the warm-up's ring bound), and
the exact cap arithmetic independently pins the populations.
What correct looks like: also assert `delta.dealloc_calls ==
delta.alloc_calls` for the window (all transients returned), if
testkit exposes it.
Confidence: MEDIUM (reasoning only; allocator internals are testkit's
— another agent's file).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SharedNow` (MonotonicSource) | 71-77 | REAL | — | frozen-at-0 virtual time, no sleeps |
| `Collector` (FrameConsumer) | 80-93 | REAL | — | poison-recovering lock (unwrap_or_else into_inner) |
| `FakeDemuxer` | 97-121 | REAL | — | finite script; EOS at exhaustion; seek honestly unsupported |
| `FakeDecoder` | 152-164 / 123-149 | REAL | — | generation-checking; one frame per packet |
| `FakeProvider` | 152-164 | REAL | — | |
| builders (`base`…`frozen_throttle`) | 170-245 | REAL | — | tiny queues (16 packets) + tiny budget |
| `open_audio` | 249-277 | REAL | — | unpolled engine; caller drives to state |
| `poll_until` | 282-299 | REAL | — | bounded, loud failure |
| `storm_pauses` | 305-328 | REAL | — | poll-on-refusal host shape; busy count returned |
| `receipts_of` | 330-338 | REAL | — | |
| `the_session_output_path_is_bounded_under_10k_storms` | 344-581 | REAL | S4 | two phases; measured windows; exact arithmetic (F-001 note) |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `the_session_output_path_is_bounded_under_10k_storms` | YES | zero-alloc guard (phase 1), flat-memory + exact drop arithmetic + order + adjacency (phase 2); mutation analysis §2 |

Not executed by me (no cargo); W00 ran the workspace suite green.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "10k pushes allocate nothing" (receipt storm) | :16-17 | REAL (per the test) | guard.assert_zero :379 |
| "memory stays flat across the whole storm" | :23-24 | REAL | delta.live_bytes == 0 :489 |
| "receipt→event causal adjacency survives the drops" | :26-27 | REAL | paired == OUTBOX_CAPACITY :558-561 |
| non-measuring halves live in session_flow.rs / session.rs unit tests | :29-35 | UNVERIFIED | those files belong to other agents |

## 7. Dead / duplicated / unreachable code

None. The fakes are trimmed copies of session_flow.rs's (admitted at
:65-66) — duplication by design for test isolation; acceptable.

## 8. Dependencies & flags

`playback_engine`, `codec_api`, `commands`, `media_clock`,
`media_types`, `packet_queue`, `plugin_api`, `testkit` (alloc_count).
No feature gates.

## 9. Open questions for the next agent

- Wave 16: verify `testkit::alloc_count`'s guard/delta semantics
  (shared dependency of this and zero_alloc_quantum proofs).
