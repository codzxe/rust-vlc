# AUDIT: crates/codecs/src/video/h264/hw/feed.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 284 / 6 prod fns + 6 tests |
| File verdict | REAL pure serialization helpers (tests ran live) — but unreachable in the shipped Linux lane; `annexb_stream` has no consumer anywhere |
| Reachable from shipped entry point? | NO on the Linux release lane: sole consumers are the videotoolbox session (cfg(target_os = "macos"), hw/mod.rs:96; compile-verified only per mod.rs:45-49) — session.rs:793/:160-181/:689 use avcc_sample_into/stamp_nanos/nanos_stamp/NANO_TIMESCALE. vaapi (Linux) uses none of feed.rs. `annexb_stream`: zero production callers (MF lane does not exist) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 h264::hw::feed` → 6 passed; 0 failed, exit 0; rg (callers, cfg gates, MF status) |

## 1. One-paragraph verdict
The host-testable halves of the hardware feed are exactly what they claim:
checked AVCC length prefixes (u32-space refusal, never a truncating cast —
pinned at the `u32::MAX`/`usize::MAX` boundary), a reusable-buffer hot-path
form whose capacity-settle behavior is asserted, checked capacity hints in
`annexb_stream`, and a nanosecond `CMTime` round trip including negative
stamps. All 6 tests ran live. The hostile findings are wiring and honesty:
`annexb_stream` is documented as "the form the H.264 MFT's
`MFVideoFormat_H264` input type takes" — but the Media Foundation lane
does not exist (hw/mod.rs:51-56: "**NOT IMPLEMENTED** — no MF lane exists
in this crate ... retracted"), so the function is dead code dressed as a
live wire form; and on the shipped Linux lane every consumer of this file
is macOS-gated compile-verified-only code. One test comment misstates its
own fixture sizes.

## 2. What actually works (verified)
— verified by `./target/debug/deps/codecs-bae0f1756cc0a809 h264::hw::feed`
→ `test result: ok. 6 passed; 0 failed` (exit 0): BE32 prefix walk-back
proof, buffer-reuse capacity monotonicity over 64 rounds, u32-boundary
refusals, start-code serialization (incl. intra-NAL zero runs left
untouched), empty-input behavior, stamp round trips (0/1/1e9/123456789012/
−40ms).

## 3. Findings

F-001 [S3]
Where: crates/codecs/src/video/h264/hw/feed.rs:17-19 (doc) + :91-108
Code (doc, feed.rs:17-19):
```rust
//! - Media Foundation: the **Annex-B byte stream** — every NAL
//!   prefixed with a 4-byte start code ([`annexb_stream`]); the
//!   form the H.264 MFT's `MFVideoFormat_H264` input type takes.
```
What is wrong: `annexb_stream` has zero production callers. The Media
Foundation lane it is written for is explicitly "NOT IMPLEMENTED — no MF
lane exists in this crate" (hw/mod.rs:51-56, which even retracts an
earlier false claim of a compile-verified MF FFI). feed.rs's own module
doc still presents the MF wire form in the present tense as one of "the
wire forms OS hardware decoders consume", implying a consumer that does
not exist. On Linux (the shipped lane) nothing in this file is reachable
at all: videotoolbox consumers are cfg(target_os = "macos") and
compile-verified only.
Confidence: HIGH — Verification: rg across apps/ + crates/ for all six
exported items; cfg gates read.

F-002 [S4]
Where: crates/codecs/src/video/h264/hw/feed.rs:151-155
Code:
```rust
    // Real corpus 0019 shapes (the integration suite's constants):
    // SPS 24 B, PPS 4 B, IDR 40 B.
    const SPS: &[u8] = &[0x67, 0x64, 0x00, 0x0d, 0xf3, 0x65];
    const PPS: &[u8] = &[0x68, 0xce, 0x3c, 0x80];
    const IDR: &[u8] = &[0x65, 0x88, 0x84, 0x00, 0x1a, 0xa0];
```
What is wrong: the comment claims the constants are 24/4/40 bytes ("real
corpus 0019 shapes"); the arrays are 6/4/6 bytes. The tests only use
`.len()` so they pass, but the provenance comment is false as written —
these are truncated header prefixes, not the claimed corpus NALs.
Confidence: HIGH — Verification: read (byte counts of the literals).

F-003 [S4]
Where: crates/codecs/src/video/h264/hw/feed.rs:133-138
Code:
```rust
pub fn nanos_stamp(nanos: i64) -> Stamp {
    match media_types::time::Timestamp::from_parts(nanos, 1, 1_000_000_000) {
        Ok(ts) => Stamp::Present(ts),
        Err(_) => Stamp::Missing,
    }
}
```
What is wrong: an unrepresentable nanosecond count silently degrades to
`Stamp::Missing` — a Present→Missing downgrade with no error channel. The
doc says `None` becomes the invalid CMTime on the way IN
(:110-113, `stamp_nanos`), which is symmetric and fine; but on the way
out a bad value produces a "no timestamp" stamp rather than surfacing the
corruption. Downstream A/V sync would treat a corrupt time as absent
instead of wrong.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| avcc_length_prefix | 45-47 | REAL | - | u32::try_from, boundary-tested |
| avcc_sample | 56-63 | REAL | - | macOS lane only (compile-verified consumer) |
| avcc_sample_into | 70-81 | REAL | - | hot-path reuse pinned (capacity <= settled) |
| annexb_stream | 91-108 | REAL (dead) | S3 | F-001: no consumer; MF lane absent |
| stamp_nanos | 119-128 | REAL | - | sign-preserving, tested with negatives |
| nanos_stamp | 133-138 | REAL | S4 | F-003 silent Present→Missing |
| 6 tests | 157-283 | REAL | S4 | F-002 comment/fixture mismatch |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| avcc_sample_prefixes_every_nal... | YES | byte-exact + parse-back walk |
| avcc_sample_into_reuses... | YES | capacity monotonicity asserted |
| avcc_length_prefix_refuses... | YES | exact u32::MAX / usize::MAX boundary |
| annexb_stream_prefixes... | YES | byte-exact incl. zero-run passthrough |
| empty_inputs... | YES | trivial but pinned |
| stamp_nanos_round_trips... | YES | 5 values incl. negative; constants checked |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the lanes' driver-mediated halves are compile-verified only ([VERIFY])" | feed.rs:21-24 | REAL | hw/mod.rs:45-56 documents exactly this |
| "Media Foundation: the Annex-B byte stream ... the form the H.264 MFT's input type takes" | feed.rs:17-19 | STALE/FALSE-by-implication | MF lane NOT IMPLEMENTED (mod.rs:51); no caller |
| "Real corpus 0019 shapes ... SPS 24 B, PPS 4 B, IDR 40 B" | feed.rs:151-152 | FALSE | constants are 6/4/6 bytes |
## 7. Dead / duplicated / unreachable code
`annexb_stream` — dead (F-001). Whole file unreachable in the shipped
Linux lane; macOS consumers compile-verified only.
## 8. Dependencies & flags
media-types only. Unconditional compile; consumers OS-gated in hw/mod.rs
(linux → vaapi, macos → videotoolbox).
## 9. Open questions for the next agent
- vaapi lane (not assigned): confirm it consumes the OS-neutral
  `H264HwUnit` NALs directly without feed.rs re-serialization (rg found no
  feed usage there — if true, feed.rs is macOS-exclusive dead weight in
  Linux release binaries).
- videotoolbox/session.rs:793 uses `avcc_sample_into(...).ok_or(...)` —
  verify the error it produces on u32 overflow is typed and surfaced
  (compile-verified only, so UNVERIFIED here).
