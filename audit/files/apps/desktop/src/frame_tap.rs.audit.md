# AUDIT: apps/desktop/src/frame_tap.rs
| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-23 |
| Lines / functions | 233 / 9 (incl. 3 tests + 2 helpers) |
| File verdict | REAL (genuine PresentTap impl over the bridge+mailbox) — UNREACHABLE from shipped binaries; carries a per-frame encode->decode->re-encode round trip and one false observability claim |
| Reachable from shipped entry point? | NO. Constructed only at shell.rs:180 (`ShellRuntime::bind_frame_channel`), whose only production caller is the workspace-excluded, non-compiling apps/desktop-tauri/src/app.rs:290 (baseline cargo-build-desktop-tauri.txt:310-345 gdk-sys failure). The engine-side seam it plugs into (`lumen_cli::pipeline_host::PresentTap`, set via host.rs `set_present_tap`) IS real and shipped in `lumen` — but the pure CLI constructs no tap (pipeline_host.rs:966-970 `present_tap: None` default). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg (multiple, exit 0); sed -n quotes (exit 0); baseline: cargo-test-workspace.txt (frame_tap tests), cargo-build-desktop-tauri.txt:310-345 |

## 1. One-paragraph verdict
This is the honest desktop half of the w31.1 `luf1-channel` lane: `FrameTap` implements lumen-cli's `PresentTap`, forwards fresh/repeat presents into `CanvasBridge` -> `MailboxTransport` -> bounded latest-wins mailbox, and `drain_records` re-emits the queued frames as complete LUF1 records for the (nonexistent) webview ticker. The plumbing is real and tested end-to-end at the unit level. Two hostile findings: (1) every tapped frame is serialized to bytes (bridge), PARSED BACK into a `ShippedFrame` (MailboxTransport::send decodes the record it just received), then RE-SERIALIZED to bytes in `drain_records` — an encode->decode->encode round trip per frame where the mailbox could store records directly; the doc calls the last hop "one memcpy", which is true but hides the first two hops; (2) the doc claims a drain-side encode skip is "observable via the mailbox/bridge stats" — it is not: `draw_next` increments `drawn`, and a skipped frame is counted nowhere as dropped (MailboxStats has only offered/drawn/dropped_overflow/dropped_stale). Like bridge.rs, none of this runs in any shipped binary.

## 2. What actually works (verified)
— verified by audit/runs/baseline/cargo-test-workspace.txt -> `frame_tap::tests::{tapped_frames_drain_as_luf1_records, a_full_mailbox_drops_oldest_never_blocks, repeat_taps_ship_the_repeat_flag}` all pass (grep 'frame_tap::tests' in baseline; same run as bridge tests :2522-2526 block).
— verified by read: latest-wins never blocks the pump (mailbox capacity 2, offer drops oldest — mailbox.rs:142-155); poison recovery on all three lock sites (:73-80, :93-96, :108-111); repeat bit routed honestly (:112-116); drain is exhaustive (test asserts second drain returns 0).

## 3. Findings
F-001 [S3]
Where: apps/desktop/src/frame_tap.rs:63-71 and :125-133
Code:
```
    /// Re-encoding (header + payload → record) is one memcpy — the
    /// mailbox stores decoded frames, the wire stores records, and
    /// the UI's decoder expects the record form (the
    /// `lumen-frames.mjs` byte-mirror).
```
```
fn encode_record(frame: &ShippedFrame, out: &mut Vec<Vec<u8>>) -> bool {
    let mut record = Vec::with_capacity(frame.header.payload_len as usize + MIN_HEADER_LEN);
    match encode_into(&frame.header, &frame.payload, &mut record) {
```
What is wrong: the full per-frame path is bridge `encode_into` (pipeline record) -> MailboxTransport::send `decode_record` (parse back + copy into pooled payload, transport.rs:222-231) -> mailbox stores `ShippedFrame` -> `encode_record` `encode_into` AGAIN (fresh `Vec::with_capacity` per drained frame). Three serialization hops and a fresh record allocation per frame where storing the record bytes in the mailbox would need one. The doc's "one memcpy" describes only the final hop. Why it matters: this is the frame hot path of the variant-B design; at 30-60 fps it is pure waste, and the zero-alloc test (zeroalloc_transport.rs) blesses the shape rather than challenging it. Blast radius today is zero (lane unreachable, F-004 in bridge audit) which caps this at S3. What correct looks like: mailbox of records, or drain returning `ShippedFrame`s and the wrapper encoding once. Confidence: HIGH — Verification: read of frame_tap.rs + transport.rs:217-231 + mailbox.rs:142-158.
F-002 [S3]
Where: apps/desktop/src/frame_tap.rs:120-133
Code:
```
/// One [`ShippedFrame`] as a complete LUF1 record appended to `out`.
/// A geometry/length inconsistency (a bridge-side bug — the encoder
/// produced the record the mailbox now holds) is SKIPPED, never
/// shipped: the consumer must not receive a record its decoder will
/// reject, and the drop is observable via the mailbox/bridge stats.
```
What is wrong: the drop is NOT observable. `draw_next` (mailbox.rs:157-161) pops the frame and increments `drawn`; `encode_record` returning false discards it with no counter — `MailboxStats` has `offered/drawn/dropped_overflow/dropped_stale/flushes` (mailbox.rs:93-101) and `BridgeStats` counts `frames_shipped` before the send — none records a drain-side skip. A silently swallowed frame looks identical in every stat to a shipped one. The doc's last clause is false. What correct looks like: a `drain_skipped` counter (or propagate the error). Confidence: HIGH — Verification: read + rg MailboxStats/draw_next in mailbox.rs:93-161.
F-003 [S4]
Where: apps/desktop/src/frame_tap.rs:100-104
Code:
```
impl Default for FrameTap {
    fn default() -> Self {
        Self::new().unwrap_or_else(|err| unreachable!("the default config is valid: {err}"))
    }
}
```
What is wrong: `unreachable!` in a Default on a config that is a constant away from changing (capacity 2); a future default-capacity edit that hits the mailbox's zero-capacity guard turns "construct a default" into a panic. shell.rs:179-183 duplicates the same pattern. Hygiene. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: apps/desktop/src/frame_tap.rs:208-214
Code:
```
        let count = tap.drain_records(&mut records);
        assert_eq!(count, 2, "only the mailbox capacity survives");
        assert!(
            count < 7,
            "the pump was never blocked: all 7 offers returned"
        );
```
What is wrong: `count < 7` is a tautology immediately after `assert_eq!(count, 2)` — the "never blocks" property is not actually asserted by it (blocking would manifest as the tap_present calls failing/panicking above, which `.must()` does cover). The redundant assert adds noise, not coverage. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FrameTap::new | 56-61 | REAL | — | MailboxTransport + CanvasBridge under one Mutex |
| FrameTap::drain_records | 72-89 | REAL | S3 | exhaustive drain + re-encode (F-001); skip path unobservable (F-002) |
| FrameTap::stats | 92-97 | REAL | — | poison-recovering accessor |
| Default::default | 100-104 | REAL | S4 | unreachable! on config error (F-003) |
| PresentTap::tap_present | 106-118 | REAL | — | repeat routed to present_repeat; errors propagate typed (never blocks: mailbox latest-wins) |
| encode_record | 125-134 | REAL | S3 | fresh Vec per record; Err(_) => false silent (F-001/F-002) |
| tests: rgba_frame/request_at | 150-168 | REAL | — | honest 4x4 RGBA fixtures |
| tapped_frames_drain_as_luf1_records | 174-195 | REAL | — | magic + length + exhaustive-drain asserts |
| a_full_mailbox_drops_oldest_never_blocks | 201-215 | REAL | S4 | capacity assert real; tautology assert (F-004) |
| repeat_taps_ship_the_repeat_flag | 220-232 | REAL | — | decodes through WireDecoder, flag bit asserted |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tapped_frames_drain_as_luf1_records | YES | 2 taps -> 2 records with magic; second drain empty |
| a_full_mailbox_drops_oldest_never_blocks | YES (for the capacity part) | 7 offers into capacity 2 -> exactly 2 drain; the `< 7` assert itself is dead weight |
| repeat_taps_ship_the_repeat_flag | YES | full decode of both records; repeat bit differs |
| GAP | — | No test exercises encode_record's Err arm (F-002's silent skip) — mutation: replace `false` with `true` (ship garbage) or drop the frame entirely; zero test outcomes change. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the drop is observable via the mailbox/bridge stats" | frame_tap.rs:124 | FALSE | MailboxStats (mailbox.rs:93-101) has no drain-skip counter; BridgeStats counts pre-send (F-002) |
| "Re-encoding ... is one memcpy" | frame_tap.rs:68-69 | PARTIALLY TRUE | true for the final hop only; the send-side decode hop and first encode are unmentioned (F-001) |
| "must NEVER block the pump — the mailbox's latest-wins bound absorbs a slow consumer" | frame_tap.rs:17-19 | REAL | offer drops oldest (mailbox.rs:142-155), tap_present cannot wait on consumer; test-pinned |
| "the ticker ships the newest and drops the rest per the channel contract (one frame per tick)" | frame_tap.rs:27-29 | REAL-UNPROVEN (design intent) | the ticker does not exist in this repo's shipped build (bridge audit F-001); contract verified only by prototype/tests |

## 7. Dead / duplicated / unreachable code
Whole module unreachable from shipped binaries (header). `FrameTap::stats` has no non-test caller in the crate (shell.rs uses `event_stats`, not tap stats) — diagnostics-only surface. No duplication.

## 8. Dependencies & flags
lumen-cli (PresentTap trait — the only desktop->cli API edge in this file), media-types, video-engine, crate-local bridge/mailbox/transport/wire. No feature gates. std-only.

## 9. Open questions for the next agent
1. zeroalloc_transport.rs (mine, next): does its "zero allocation" claim hold for the drain_records re-encode hop (fresh Vec per record) or only for send? 2. If the tauri wrapper ever lands, F-001's triple-serialization becomes a real per-frame cost — re-measure then. 3. shell.rs:179-183 unreachable! twin of F-003 belongs to the shell owner.
