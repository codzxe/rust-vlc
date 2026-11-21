# AUDIT: crates/control-protocol/src/frame.rs#part1 (lines 1–454 of 909)
| Field | Value |
|---|---|
| Auditor | W02-A04 |
| Date | 2026-11-21 |
| Lines / functions | 454 (chunk 1–454; boundary: `impl MessageAssembler` closes at 454; `validate_utf8` at 457+ belongs to A06's part2) / 15 (12 fns/methods + 3 types) |
| File verdict | UNREACHABLE (real, tested code — but no shipped entry point reaches it; see reachability row) |
| Reachable from shipped entry point? | NO. Proof: `rg -n "control-protocol" apps/cli/Cargo.toml apps/web/Cargo.toml apps/desktop-tauri/Cargo.toml` → no dependency in any shipped app; apps/cli/src/host.rs:1635 self-reports `protocol_version: "unwired (control-protocol is a later card)"`; the only consumer is apps/control-server (bin `lumen-control-server`), which is NOT in the deb/rpm payload (scripts/packaging/stage-linux-payload.sh stages only $BIN_CLI + $BIN_TAURI + docs + icon) and whose product gate is OFF (product.toml:167 `control-server = false  # Phase 5+ [PROPOSAL]`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `rg -n "read_frame\|write_frame\|MessageAssembler" crates apps` (callers: server.rs:49-54,544-546,554,571,841,872; client.rs:304); `sed -n '530,600p' crates/control-protocol/src/server.rs` (session loop wiring + Limits); `sed -n '60,95p' crates/control-protocol/src/server.rs` (DEFAULT caps 1 MiB); `sed -n '195,310p' crates/control-protocol/src/client.rs` (client wrappers, hard-coded cap); `rg -n "set_read_timeout" server.rs` (blocking socket + deadline → read_exact's TimedOut arm is the documented bounded-wait); `rg -c "#\[test\]" crates/control-protocol/src/*.rs` (3+4+6+5+3+6+3+6+11 = 47 == baseline "47 passed" → all frame tests executed); `rg -n "control" product.toml docs/05-command-layer.md` (gating + claims); `sed -n '1,60p' scripts/packaging/stage-linux-payload.sh` (payload contents). |

## 1. One-paragraph verdict
A genuinely competent RFC 6455 server-side frame codec: cap-before-allocation is real (length check at line 258 precedes the payload allocation at line 278 — the module's central claim holds), the phantom-mask-bytes bug is documented and fixed (mask read only when the bit is set, lines 264-277), MSB-set 64-bit lengths are rejected, control frames enforce fin + ≤125, fragmentation reassembly is cap-checked in u64 before extend, and every dirty input I threw at it by reading (RSV, garbage opcode, unmasked-client/masked-server, truncation at each header stage) maps to a typed error. Its 11 tests (in part2, lines 572-909) are real assertions with exact limit/claimed values and were EXECUTED in baseline (control_protocol unittests: 47 passed, 0 failed — count cross-check: 47 = 3+4+6+5+3+6+3+6+11). Two conformance/contract defects survive the tests: the reader accepts non-minimal length encodings that RFC 6455 §5.2 says MUST be rejected, and `MessageAssembler`'s single-frame fast path silently bypasses the message cap it documents. None of it ships: the only consumer is the product-gated-off control-server app, absent from the deb/rpm payload.

## 2. What actually works (verified)
— All 11 frame.rs tests executed and passed in baseline: `audit/runs/baseline/test-workspace.log.txt` line 1411 "Running unittests src/lib.rs (control_protocol…)" → "test result: ok. 47 passed; 0 failed" (and again in the all-features run, line 1370). The 47 count equals the crate's total `#[test]` census, so every frame test ran. — Test bodies assert real behavior (read `oversized_length_claims_reject_before_allocating`, frame.rs:656-679: asserts exact `limit`/`claimed` values and typed error variants). — Cap-before-allocation verified by line order: 258 (`if length > max_frame`) precedes 278 (`read_exact(stream, length as usize)`).

## 3. Findings

### F-001 [S3]
Where: crates/control-protocol/src/frame.rs:220-243
Code:
```rust
    let length: u64 = match len7 {
        126 => {
            let Some(bytes) = read_exact(stream, 2)? else {
                return Err(truncated("16-bit length"));
            };
            u16::from_be_bytes([bytes[0], bytes[1]]) as u64
        }
```
What is wrong: RFC 6455 §5.2 requires the minimal number of bytes to encode the length (a 124-byte payload MUST use the 7-bit form, not the 126 form). This reader accepts the 126 form carrying any value < 126 and the 127 form carrying any value ≤ 65535 without complaint.
Why it matters: The module header (line 1) claims "RFC 6455 §5 frame codec". A conformance suite (Autobahn) treats non-minimal length encodings as protocol errors the server must fail with 1002; this codec silently accepts them. Worse, the asymmetry with the writer (write_frame lines 321-331 always emits the minimal form) means a fuzzing/loopback harness built on this crate will never exercise its own leniency.
What correct looks like: After decoding, reject `len7 == 126 && length < 126` and `len7 == 127 && length <= 0xFFFF` with the same typed 1002 Frame error used for RSV/garbage opcodes; add the two cases to the part2 test module.
Confidence: HIGH — Verification: read (both arms); RFC 6455 §5.2 minimal-encoding requirement.

### F-002 [S3]
Where: crates/control-protocol/src/frame.rs:419-427
Code:
```rust
                if frame.fin {
                    if frame.opcode == Opcode::Text {
                        validate_utf8(&frame.payload)?;
                    }
                    return Ok(Some(Message {
                        opcode: frame.opcode,
                        payload: frame.payload,
                    }));
                }
```
What is wrong: The single-frame fast path returns a `Message` without ever comparing `frame.payload.len()` against `self.cap`. Only the fragmented path (`append`, lines 444-450) enforces the cap. `MessageAssembler::new`'s doc (line 371) promises "allowing messages up to `cap` bytes total".
Why it matters: A single-frame message larger than `cap` is delivered whenever the frame-level cap (`max_frame_bytes` at the read_frame layer) exceeds the assembler cap. Today's only caller uses equal defaults (server.rs:83-85: both 1 MiB), so the hole is latent — but the API contract is violated for any `Limits { max_frame_bytes: X, max_message_bytes: Y<X }`, and the struct fields are `pub`, inviting exactly that configuration.
What correct looks like: Check `frame.payload.len() as u64 > self.cap` in the fast path and return `ProtocolError::MessageTooLarge { limit: self.cap, claimed }`.
Confidence: HIGH — Verification: read of push/append/new + server.rs Limits defaults.

### F-003 [S4]
Where: crates/control-protocol/src/frame.rs:401-403
Code:
```rust
                let fin = frame.fin;
                let done = self.append(&frame)?;
                if fin && done {
```
What is wrong: `append` (line 452) returns `Ok(frame.fin)` unconditionally, so `done == fin` always; `fin && done` is a dead double-check that can never be `true && false`.
Why it matters: It reads as if `append` could disagree with `frame.fin` (a state the code cannot produce), misleading the next maintainer into believing there is a case where a fin continuation does NOT complete the message.
What correct looks like: `if frame.fin {` and have `append` return `()`, or keep the bool but drop the redundant conjunct.
Confidence: HIGH — Verification: read of append (443-453).

### F-004 [S4]
Where: crates/control-protocol/src/client.rs:302-305 (caller of this chunk's `read_frame_unmasked`)
Code:
```rust
fn read_client_side_frame<R: Read>(stream: &mut R) -> Result<Option<Frame>, ProtocolError> {
    crate::frame::read_frame_unmasked(stream, 1 << 20).map(|option| {
        option.map(|frame| Frame {
```
What is wrong: The client's frame cap is a hard-coded `1 << 20` magic number duplicating `Limits::DEFAULT_MAX_FRAME_BYTES` (server.rs:83, `1024 * 1024`), with no shared constant.
Why it matters: doc 05 §5.4 pins "Every frame: max 1 MiB" for BOTH ends; two independent literals can drift (a future Limits change silently leaves the client at the old cap). The values also don't even share a spelling (`1 << 20` vs `1024 * 1024`), so a grep for one misses the other.
What correct looks like: One `pub const MAX_FRAME_BYTES` in this crate (or frame.rs), referenced by server Limits and client reader alike.
Confidence: HIGH — Verification: read of both sites.

### F-005 [S4]
Where: crates/control-protocol/src/frame.rs:278
Code:
```rust
    let Some(mut payload) = read_exact(stream, length as usize)? else {
        return Err(truncated("payload"));
    };
```
What is wrong: `length` is a u64 narrowed with `as usize`. On a 32-bit target a length ≥ 2^32 truncates and the reader would allocate/read the wrapped-around count, desynchronizing the stream (not memory-unsafe — the cap check at 258 already bounded `length ≤ max_frame`).
Why it matters: The repo ships a web/wasm32 column (feature matrix; apps/web), and `as` truncation is exactly the class the audit checklist names. Reachable only if a 32-bit build configures max_frame ≥ 4 GiB — unrealistic, hence S4 not S3.
What correct looks like: `usize::try_from(length).map_err(|_| ProtocolError::MessageTooLarge { … })` before the read.
Confidence: HIGH — Verification: read; arithmetic reasoning.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Opcode (enum) | 31-44 | REAL | — | All 6 RFC 6455 §5.2 opcodes. |
| Opcode::from_u8 | 48-58 | REAL | — | Correct table; None for reserved. Executed by rsv_bits_and_bad_opcode test. |
| Opcode::as_u8 | 61-70 | REAL | — | Correct inverse. |
| Opcode::is_control | 73-75 | REAL | — | Close/Ping/Pong per §5.5. |
| Frame (struct) | 79-87 | REAL | — | Payload unmasked by the time it is stored. |
| read_exact | 90-135 | REAL | — | Clean-EOF vs mid-frame EOF split correct; WouldBlock/TimedOut → typed TimedOut (bounded-wait; server.rs:449 sets a read deadline on a blocking socket, so partial-frame data loss on timeout is the documented design); Interrupted retried. Zero-length n returns Some(empty) without touching the stream — correct. |
| is_disconnect | 138-143 | REAL | — | ConnectionReset/BrokenPipe at frame start → clean EOF. |
| Endpoint (enum) | 148-154 | REAL | — | Mirror strictness for the two read roles. |
| read_frame | 165-167 | REAL | — | Server-role wrapper. Caller: server.rs:546. |
| read_frame_unmasked | 176-181 | REAL | — | Client-role wrapper. Caller: client.rs:304 (hard-coded 1 MiB, F-004). |
| read_frame_endpoint | 183-292 | REAL | S3/S4 | Core parser. Cap-before-alloc verified (258 < 278). MSB check (234) correct. Control ≤125 + fin (244-257) correct. Mask-only-when-bit (270-277) correct. F-001 (non-minimal length), F-005 (`as usize`). |
| truncated | 294-299 | REAL | — | Error helper. |
| write_frame | 305-350 | REAL | — | Minimal length encoding on write (asymmetric with reader, F-001); refuses oversized control frames (314-320); `if fin { 0x80 } else { 0x00 } \| opcode` precedence is safe — proven by the passing roundtrip tests (fin+opcode frames round-trip). Callers: server.rs:554,571,841,872. |
| Message (struct) | 353-359 | REAL | — | opcode only ever Text/Binary from push. |
| MessageAssembler::new | 372-378 | REAL | — | cap stored; F-002 documents the fast-path bypass. |
| MessageAssembler::is_mid_message | 381-383 | REAL | — | Trivial. |
| MessageAssembler::push | 392-439 | REAL | S3/S4 | Continuation-without-start and data-during-reassembly typed errors correct; UTF-8 validated at reassembly for fragmented text (406-408) and immediately for single text frames (420-422) — both RFC-correct. F-002 (cap bypass), F-003 (dead `fin && done`). No reset() after a mid-message error — caller must drop the assembler (session ends on error in server.rs; acceptable, S4-level note). |
| MessageAssembler::append | 443-453 | REAL | — | u64 total, cap check BEFORE extend — no OOM; returns frame.fin (F-003 redundancy). |

(Chunk boundary note: `validate_utf8` (457+), `parse_close_payload`, `apply_mask`, `write_masked_frame` and the 11-test module live in part2, A06's chunk. `validate_utf8` is called from my chunk's `push` — its body audit belongs to A06.)

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| roundtrip_all_length_forms (part2) | YES | Writes via write_frame, parses via read_frame across 7/16/64-bit forms — catches length-encoding and fin/opcode byte bugs. Executed in baseline (47/47). |
| unmasked_client_frame_is_rejected (part2) | YES | Asserts typed 1002 error. Executed. |
| rsv_bits_and_bad_opcode_are_rejected (part2) | YES | Verified body (frame.rs:650-654): asserts reason strings. Executed. |
| oversized_length_claims_reject_before_allocating (part2) | YES | Verified body (frame.rs:656-679): asserts exact `limit`/`claimed` (1024, 4<<30) and typed variants. Executed. |
| truncated_headers_are_typed_errors (part2) | YES | Cuts 1..=7 of a real client frame; asserts typed errors each time (body read at 680+). Executed. |
| fragmented_text_reassembles_with_interleaved_ping / assembler_rejects_dirty_fragment_sequences / invalid_utf8_text_is_a_typed_close / close_payload_validation / unmasked_server_frames_parse_without_phantom_mask_bytes / server_writer_roundtrips_through_the_parser_via_masking (part2) | YES (by name+coverage) | Bodies are A06's to audit; names map 1:1 onto this chunk's error paths; all executed in the 47/47 baseline run. |
| GAP: no test covers non-minimal length encodings (F-001) or a single-frame message above the assembler cap (F-002) — neither defect is detectable by the existing suite. | — | — |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every dirty-world input … is a typed [`ProtocolError`] — never a panic" | frame.rs:19-21 (module doc) | REAL | Traced every path in read_frame_endpoint/push: all typed errors, no unwrap/panic in the chunk. |
| "the claim is checked against the cap BEFORE any allocation: an oversized claim is a typed MessageTooLarge, never an OOM vector" | frame.rs:7-9 (module doc) | REAL | Line 258 check precedes line 278 allocation; test at 656-679 asserts it. |
| "control-protocol (the wire: in-tree WS + handshake + policy)" | apps/control-server/Cargo.toml:9 | REAL (code) / UNREACHABLE (shipping) | Crate implements it; app not in payload, product.toml:167 gate false. |
| "Every frame: max 1 MiB" | docs/05-command-layer.md:542 | REAL | server.rs:83-85 defaults; client.rs:304 hard-codes the same value (F-004). |
| "Transport: WebSocket with JSON text frames … defined in crates/control-protocol as typed serde enums" `[PROPOSAL]` | docs/05-command-layer.md:540 | REAL | protocol.rs (serde), http.rs (handshake) in same crate; marked PROPOSAL — docs honestly flag the phase. |

## 7. Dead / duplicated / unreachable code
The whole file is unreachable from shipped entry points (see header row) — real code, real tests, zero shipped callers. Within the chunk: no dead functions (both read roles + writer have callers in server.rs/client.rs). Duplication: client.rs mirrors the Frame struct and re-states the cap (F-004).

## 8. Dependencies & flags
std-only I/O (std::io::Read/Write); crate deps on `error.rs` types only (CloseCode, ProtocolError) — checked imports lines 23-27. No feature gates. `[lints] workspace = true` per crate Cargo.toml.

## 9. Open questions for the next agent
1. A06 (part2): does `parse_close_payload` enforce the §5.5.1 close-code validity (1000/1003-1009 etc.) — `is_valid_on_wire_close_code` exists; verify the server actually uses it on inbound close frames (server.rs:562 calls parse_close_payload — check the validation is inside).
2. A06 (part2): `write_masked_frame` — is the mask applied with the same `i & 3` discipline and does any test cover a masked client frame with payload length not a multiple of 4 crossing the wrap?
3. Whoever owns product shipping: if the control-server is ever product-gated ON, F-002 becomes reachable with non-default Limits — fix before Phase 5.
