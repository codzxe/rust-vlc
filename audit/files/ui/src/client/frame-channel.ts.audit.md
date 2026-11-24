# AUDIT: ui/src/client/frame-channel.ts
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 448 / 8 (parseLuf1Header, decodeLuf1Record, toSafeNumber, isFrameChannelBackend, detectFrameChannelBackend, FrameChannelShim: constructor/close/handleCallback/drain/resolveBytes) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | YES — MediaSurface.tsx:126 `new FrameChannelShim(backend, ...)` -> decodeLuf1Record -> presentLuf1Frame (putImageData) -> player screen. Runtime path requires the Tauri desktop shell (absent in sandbox: apps/desktop-tauri/INFRA-BLOCKED.md), so execution is mocked-internals-only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg wire.rs constants (all match); rg consumers; read frame-channel.test.ts structure (24 cases / 4 describes); sed MediaSurface presenter |

## 1. One-paragraph verdict
The webview half of the video-frame bridge: a strict LUF1 binary decoder plus
a hand-rolled port of @tauri-apps/api's Channel (no new npm dep). The
byte-mirror claim against apps/desktop/src/wire.rs is VERIFIED field-by-field
(magic/VERSION/MIN_HEADER_LEN/MAX_PAYLOAD_LEN 1<<30/MAX_DIMENSION 65_535,
offsets 4/6/8/9/10/12/16/20/24/32/40, little-endian on both sides). It is
consumed by MediaSurface.tsx, which paints decoded frames onto the canvas —
this is the real video-path wiring for the desktop shell. Not executed
against a live shell (no webview in sandbox); behavior pinned by 24 unit
tests over mocked internals. One genuine race at channel teardown.

## 2. What actually works (verified)
- Byte-mirror vs apps/desktop/src/wire.rs: MAGIC [0x4C,0x55,0x46,0x31] (wire.rs:70) = LUF1.MAGIC; VERSION 1 (:73); MIN_HEADER_LEN 48 (:76); MAX_PAYLOAD_LEN 1<<30 (:84); MAX_DIMENSION 65_535 (:89); field offsets in the wire.rs doc table (23-36) match the DataView offsets (122-170); writer uses to_le_bytes (:239-248), reader passes `true` for LE. rg, all confirmed.
- Consumer chain: MediaSurface.tsx:126-165 creates the shim, decodes each record, calls `session.set-frame-channel` with the channel id, presents via putImageData (BGRA swizzled draw-side, 2D path). The frame bridge answers the master question: the player canvas is wired to real engine frames in the desktop shell.
- Strictness: decodeLuf1Record refuses truncation, trailing bytes, bad magic/version/format, reserved bits, zero/oversized geometry, payload_len != w*h*4, and u64 fields beyond 2^53 (toSafeNumber:94-103). No guessed outputs.

## 3. Findings
F-001 [S3]
Where: frame-channel.ts:365-371 + 332-345
Code:
```
    if (frame.end === true) {
      // The terminal marker (tauri's channel cleanup callback): the
      // shell will send nothing after it — release the registrations.
      this.onDiagnostic(`frame channel: end marker at index ${index} — closing`);
      this.close();
      return;
```
What is wrong: the end marker calls close(), which does `this.pending.clear()`
and sets closed=true. If records with indices between nextIndex and the end
index are still parked (out-of-order fetches in flight), they are silently
discarded — the final frames of a stream can be dropped at teardown.
What correct looks like: drain parked messages up to the end index before
closing (or deliver-on-close). Why it matters: last-frame loss at
stop/eject; narrow window, visible as a stale final frame.
Confidence: MEDIUM — Verification: read; race not executed (no live shell —
UNVERIFIED runtime).
F-002 [S4]
Where: frame-channel.ts:398-403
Code:
```
        const index = this.nextIndex;
        this.pending.delete(index);
        this.nextIndex += 1;
        try {
          const record = await this.resolveBytes(message, index);
          this.onRecord(record);
```
What is wrong: no `closed` re-check between the awaited fetch and
onRecord — one record can be delivered after close() (fetch resolved during
teardown). Consumer-side effect is a single post-close putImageData.
Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| LUF1 consts | 39-52 | REAL | - | match wire.rs (verified) |
| FrameWireError | 61-69 | REAL | - | typed refusals |
| toSafeNumber | 94-103 | REAL | - | safe-integer guard |
| parseLuf1Header | 112-172 | REAL | - | validates every field; LE mirror |
| decodeLuf1Record | 185-204 | REAL | - | strict record-at-a-time |
| isFrameChannelBackend / detectFrameChannelBackend | 231-249 | REAL | - | same probe as tauri-transport |
| FrameChannelShim constructor | 310-330 | REAL | - | dual registration (2.1.x window._id + 2.12.x transformCallback) |
| close | 333-345 | REAL | S3 | clears parked records (F-001) |
| handleCallback | 348-378 | REAL | - | index/id dual key, end marker, stale dedupe |
| drain | 387-415 | REAL | S4 | guarded single pass; skip-and-continue on per-message failure; F-002 |
| resolveBytes | 418-447 | REAL | - | ArrayBuffer/view/token-fetch forms |

## 5. Test honesty
frame-channel.test.ts: 24 cases across decodeLuf1Record (byte-mirror
describe), parseLuf1Header, FrameChannelShim, detectFrameChannelBackend.
Mutation reasoning: a decoder that guessed (e.g. ignoring payload_len) would
fail the mismatch cases; the ordering queue is tested with out-of-order
arrivals (describe at 304). Honest, mocked-internals posture — the file's own
header admits the runtime shapes are "pinned by unit tests against mocked
internals" (line 34-36), which is why the file verdict is REAL-UNPROVEN, not
REAL.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "this decoder is its byte-mirror" (wire.rs) | frame-channel.ts:9-11 | REAL | field-by-field match (see §2) |
| "the npm @tauri-apps/api package is not in the dependency allowlist" | frame-channel.ts:13-15 | REAL-UNPROVEN | allowlist record not read by me (records/dependencies/... path cited; UNVERIFIED — outside my file list) |
| "in 2.12.x the shell's own eval'd script performs the fetch ... ArrayBuffer branch is the one the real shell exercises" | frame-channel.ts:28-33 | REAL-UNPROVEN | tauri 2.12.5 source not inspected in sandbox |
| "No webview exists in this sandbox (INFRA-BLOCKED.md)" | frame-channel.ts:34 | REAL | baseline confirms desktop-tauri build fails at gdk-sys |

## 7. Dead / duplicated / unreachable code
None. All exports consumed (MediaSurface.tsx, tests).

## 8. Dependencies & flags
Zero npm deps (window.__TAURI_INTERNALS__ direct binding). No feature gates.

## 9. Open questions for the next agent
- MediaSurface.tsx owner: the per-frame JS BGRA swizzle loop (2M+ iterations
  at 1080p) is a documented 2D-path cost — worth a perf finding there.
- Whether the shell ever sends records after parking an end marker (F-001
  depends on the shell's send ordering — needs frame_channel.rs audit).
