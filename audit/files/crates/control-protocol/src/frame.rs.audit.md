# AUDIT: crates/control-protocol/src/frame.rs
| Field | Value |
|---|---|
| Auditor | W02-A06 |
| Date | 2026-11-27 |
| Lines / functions | 909 / 20 prod fns + 11 tests (chunks part1 L1-799, part2 L800-909) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — see §7 (reachable only from the non-shipped `lumen-control-server` bin) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 6 |
| Commands run | `cargo test -p control-protocol frame::` -> 11 passed, EXIT=0; `cargo test -p control-protocol --test conformance` -> 25 passed, EXIT=0; `rustc` precedence probe -> 0x81/0x01, exit 0 |

## 1. One-paragraph verdict
This is a real RFC 6455 frame codec, not a toy. Length-prefix trust is handled correctly: the 64-bit MSB is rejected (L234-239), the claim is checked against `max_frame` BEFORE the payload allocation (L258-263 precede the `read_exact(stream, length as usize)` at L278), all three length forms decode (unit-tested to 65535), control frames are capped at 125 and require FIN, masking is enforced in both directions, partial reads/truncation are typed errors, and there is not one `unwrap`/`expect`/`panic!` in production code. I executed the 11 codec unit tests and the 25-test socket conformance suite first-hand — all pass. The defects that remain: the message assembler skips its cap on the unfragmented path (latent API bug, not exploitable in-tree), the 64-bit length form's happy path (>65535 bytes) has zero test coverage anywhere, and a set of hygiene items. Separately, the sandbox's Bash output channel rewrites the two bytes `[m` to nothing — an apparent "syntax error" at L274 was an output-channel artifact, disproven by hex dump (bytes on disk are `53 6f 6d 65 28 5b 6d` = `Some([m`) and by the crate compiling and passing tests. Details in the run log.

## 2. What actually works (verified)
- verified by `cargo test -p control-protocol frame::` -> `test result: ok. 11 passed; 0 failed` EXIT=0 (all length forms, unmasked/RSV/opcode rejection, oversize-claim rejection, truncation, fragmentation+interleaved ping, cap enforcement on fragments, invalid UTF-8, close payload, phantom-mask regression, masked writer roundtrip).
- verified by `cargo test -p control-protocol --test conformance` -> `test result: ok. 25 passed; 0 failed` EXIT=0 — the codec runs over real loopback TCP against the real server (ping/pong echo, fragmented hello, oversize-claim close 1009, truncated header close 1002, unmasked close 1002).
- verified by `rustc -O` probe of `if fin { 0x80 } else { 0x00 } | opcode.as_u8()` -> `0x81`/`0x01`, exit 0: the head byte precedence is `(if..)|opcode`, correct wire bytes.
- Baseline cross-check: audit/runs/baseline/cargo-test-workspace.txt L1552-1648 (`### PACKAGE control-protocol exit_code=0`, 47+25 passed).

## 3. Findings
F-001 [S3]
Where: frame.rs:412-430 (vs 443-453)
Code:
```
            Opcode::Text | Opcode::Binary => {
                if self.opcode.is_some() {
                    return Err(ProtocolError::Frame {
                        code: CloseCode::ProtocolError,
                        reason: "new data frame while a fragmented message is open".to_owned(),
                    });
                }
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
What is wrong: the unfragmented path returns the message WITHOUT checking `self.cap`; the cap is enforced only in `append()` (fragment path, L444-450). `MessageAssembler::new(cap)` is documented "allowing messages up to `cap` bytes total" — false for single-frame messages. Why it matters: any caller with `cap < frame cap` gets the cap silently ignored; the doc lies. In-tree impact: none — the only production caller (server.rs:544-546) passes `max_message_bytes == max_frame_bytes == 1 MiB` (server.rs:963-964 asserts both), and `read_frame` already rejected frames above the frame cap, so no frame reaching the assembler can exceed the message cap. What correct looks like: check `frame.payload.len() as u64 > self.cap` in the FIN branch before returning.
Confidence: HIGH — Verification: read.

F-002 [S3]
Where: frame.rs:604-620 (and the whole test section; grep of control-protocol + apps/control-server for payloads >65535)
Code:
```
    fn roundtrip_all_length_forms() {
        for len in [0usize, 1, 125, 126, 127, 65535] {
```
What is wrong: the 64-bit length form's SUCCESS path (payload 65536..1048576) is never tested, on either side. `roundtrip_all_length_forms` stops at 65535 (16-bit max); every other test uses ≤200-byte payloads; the only 64-bit-form tests are rejections of a 4 GiB claim (L659-669) and conformance's `oversized_length_claim_closes_1009_before_allocation`. The writer emits the 64-bit form for len > 65535 (L327-330) but no test writes one either. Why it matters: a byte-order or off-by-one bug in `u64::from_be_bytes` (L233) or the `127` write branch would ship green: a swapped 70000-length decode yields >1 MiB and closes 1009 — nothing fails in CI. What correct looks like: add 65536 and ~1 MiB roundtrips (reader and writer).
Confidence: HIGH — Verification: read + `rg '65536|655_36|70000|1 << 20'` across both crates -> only cap constants, no test payload.
Note: by inspection the code at L227-243 and L327-330 is correct (from_be_bytes / to_be_bytes); this is a coverage hole, not a decode bug.

F-003 [S4]
Where: frame.rs:90-91, 278
Code:
```
fn read_exact<R: Read>(stream: &mut R, n: usize) -> Result<Option<Vec<u8>>, ProtocolError> {
    let mut buf = vec![0u8; n];
```
What is wrong: eager allocation sized by the peer's length claim before any payload byte arrives — a peer claiming just-under-cap (1 MiB) forces a 1 MiB alloc and can hold it by trickle-feeding. Bounded per connection by `max_frame` (D-1) and the server is single-connection (server.rs Timeouts doc: "the single-connection server"), so amplification is capped at 1 MiB. Correct look: grow the buffer as bytes arrive, or cap-then-read.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: frame.rs:278
Code: `let Some(mut payload) = read_exact(stream, length as usize)? else {`
What is wrong: `u64 as usize` truncates on 32-bit targets if `max_frame` exceeded 4 GiB; `n` would silently shrink and mis-frame the stream. Unreachable in-tree (cap 1 MiB, 64-bit release lane). A `usize::try_from(length)` would make it total.
Confidence: HIGH — Verification: read.

F-005 [S4]
Where: frame.rs:562-569 (vs write_frame's 332-349)
Code:
```
    stream
        .write_all(&head)
        .and_then(|()| stream.write_all(&masked))
        .map_err(|source| ProtocolError::Io {
            what: "writing frame",
            source,
        })
```
What is wrong: `write_masked_frame` does not map `WouldBlock|TimedOut` to `ProtocolError::TimedOut`, unlike `write_frame` (L336-342). Taxonomy inconsistency; only the test client uses this writer (client.rs:23, "Not part of any production path").
Confidence: HIGH — Verification: read.

F-006 [S4]
Where: frame.rs:283-285, 520-524, 560
Code:
```
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i & 3];
        }
```
What is wrong: byte-at-a-time XOR unmask (no u32/word chunking) plus `payload.to_vec()` per write in `write_masked_frame` (L560). Cold path (1 MiB cap, loopback control plane), so no user-visible cost; pure hygiene.
Confidence: HIGH — Verification: read.

F-007 [S4]
Where: frame.rs:220-243
Code:
```
    let length: u64 = match len7 {
        126 => {
```
What is wrong: non-minimal length encodings (16-bit form declaring <126, 64-bit form declaring <65536) are accepted; RFC 6455 §5.2 requires the minimal form. Lenient-accept, no security impact (claim still capped), but a conformance deviation worth recording.
Confidence: HIGH — Verification: read.

F-008 [S4]
Where: frame.rs:508-514
Code:
```
    while reason_bytes.len() > 123 {
        let mut cut = 123;
        while cut > 0 && !reason.is_char_boundary(cut) {
            cut -= 1;
        }
        reason_bytes = &reason_bytes[..cut.max(1)];
```
What is wrong: `cut.max(1)` is dead — `is_char_boundary(0)` is always true and UTF-8 chars are ≤4 bytes, so `cut` can never fall below 120 when starting at 123; the `.max(1)` arm is unreachable. Had it been reachable with a multi-byte first char it would slice mid-char and panic. Harmless today, misleading armor.
Confidence: HIGH — Verification: read.

## 4. Per-function table (production code)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Opcode::from_u8 | 48-58 | REAL | — | total match, None on reserved |
| Opcode::as_u8 | 61-70 | REAL | — | inverse of from_u8, verified by roundtrip test |
| Opcode::is_control | 73-75 | REAL | — | close/ping/pong only |
| read_exact | 90-135 | REAL | S4(F-003) | partial-read loop, clean-EOF vs mid-frame distinction, TimedOut/Interrupted handling; eager alloc |
| is_disconnect | 138-143 | REAL | — | ConnectionReset/BrokenPipe; RST-before-first-byte treated as clean EOF (design choice) |
| read_frame | 165-167 | REAL | — | server role wrapper |
| read_frame_unmasked | 176-181 | REAL | — | client role wrapper |
| read_frame_endpoint | 183-292 | REAL | S4(F-004, F-007) | the core: RSV/opcode/mask-direction checks, 3 length forms, 64-bit MSB reject, cap BEFORE alloc (L258→L278), control ≤125+FIN, conditional mask read (regression L264-277), unmask |
| truncated | 294-299 | REAL | — | error ctor |
| write_frame | 305-350 | REAL | — | unmasked server writer; control >125 refused; minimal length forms; TimedOut mapped |
| MessageAssembler::new | 372-378 | REAL | — | doc overstates cap (F-001) |
| MessageAssembler::is_mid_message | 381-383 | REAL | — | |
| MessageAssembler::push | 392-439 | REAL | S3(F-001) | fragment state machine: orphan continuation, interleaved data, UTF-8 on completion; FIN single-frame path skips cap |
| MessageAssembler::append | 443-453 | REAL | — | cap checked before extend; returns frame.fin (caller's `if fin && done` is redundant-but-harmless) |
| validate_utf8 | 457-463 | REAL | — | |
| parse_close_payload | 473-494 | REAL | — | 0/1-byte, on-wire code validity, UTF-8 reason; tested |
| is_valid_on_wire_close_code | 497-499 | REAL | — | 1000-1003, 1007-1011, 3000-4999 per §7.4 |
| close_payload | 502-517 | REAL | S4(F-008) | 123-byte char-boundary cap |
| apply_mask | 520-524 | REAL | S4(F-006) | |
| write_masked_frame | 532-569 | REAL | S4(F-005, F-006) | test-client writer; mask bit on every length byte |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| roundtrip_all_length_forms | YES | real Cursor reads, exact payload compare through all length forms to 65535 |
| unmasked_client_frame_is_rejected | YES | raw 0x81 0x02 wire bytes; any acceptance hits unreachable! |
| rsv_bits_and_bad_opcode_are_rejected | YES | bit-level mutations of real frames |
| oversized_length_claims_reject_before_allocating | YES | asserts the TYPED MessageTooLarge{limit,claimed}; removing the cap check yields a different error (eof) -> unreachable! fires |
| truncated_headers_are_typed_errors | YES | 1..=7-byte prefixes of a real 11-byte frame |
| fragmented_text_reassembles_with_interleaved_ping | YES | real parser + assembler over one concatenated wire buffer; exact b"hello world" |
| assembler_rejects_dirty_fragment_sequences | YES | orphan continuation, interleave, fragment cap (4 vs 5 asserted) |
| invalid_utf8_text_is_a_typed_close | YES | 0xff 0xfe payload through parser+assembler |
| close_payload_validation | YES | empty/1-byte/bad-UTF-8/1005/4409 all distinct arms |
| unmasked_server_frames_parse_without_phantom_mask_bytes | YES | sentinel: asserts ZERO trailing bytes after each parsed frame — catches over/under-read |
| server_writer_roundtrips_through_the_parser_via_masking | YES | hand-masked bytes compared byte-equal to write_masked_frame output, then parsed back |
Mutation note: `client_frame` helper is an INDEPENDENT frame builder (does not call the writer), so reader tests are not goldens of the code under test. Test-coverage gap F-002 is the one dishonesty-by-omission.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "all three length forms (7-bit, 16-bit, 64-bit)" | frame.rs:6 | REAL | L220-243; tests pass (my run) |
| "the 64-bit form's MSB must be zero and the claim is checked against the cap BEFORE any allocation" | frame.rs:6-9 | REAL | L234-239 MSB reject; L258-263 before L278/91 alloc; oversize test asserts typed error |
| "fragmentation ... reassembled by MessageAssembler, with interleaved control frames" | frame.rs:10-11 | REAL | L392-454 + server.rs:551-557 dispatches control before assembler |
| "ping/pong (payload ≤ 125, answered by the session loop)" | frame.rs:12-13 | REAL | L251-256; server.rs:552-557 (conformance ping test passed) |
| "close (payload = 2-byte code + UTF-8 reason, validated)" | frame.rs:13-14 | REAL | parse_close_payload L473-494 |
| "invalid UTF-8 is a typed 1007 close" | frame.rs:15-16 | REAL | validate_utf8 → ProtocolError::InvalidUtf8 → CloseCode 1007 (error.rs; conformance invalid_utf8_text_closes_1007 ok) |
| "hard caps on single frames and reassembled messages (D-1)" | frame.rs:17 | REAL (with caveat) | frames: L258; reassembled: L445 — but see F-001: unfragmented messages skip the message cap (moot at equal in-tree caps) |
| "Every dirty-world input ... typed ProtocolError — never a panic" | frame.rs:19-21 | REAL | zero unwrap/expect/panic in prod code (rg, workspace lints deny them); conformance suite passes |
| "tests pin exact masks so the wire bytes are reproducible" | frame.rs:527-528 | REAL | L881 fixed mask [0xa5,0x5a,0x0f,0xf0], byte-equal assert L894-897 |

## 7. Dead / duplicated / unreachable code
- No dead production code found. `Endpoint` is pub and both variants used. `write_masked_frame`/`read_frame_unmasked` are used only by client.rs (declared "Not part of any production path") — test-only surface, deliberately pub for integration tests.
- REACHABILITY from the SHIPPED lane: NO. `rg control-protocol apps/cli apps/desktop` -> only a comment (apps/cli/src/run.rs:338) and a placeholder string (apps/cli/src/host.rs:1635 "unwired (control-protocol is a later card)"). lumen-cli's Cargo.toml deps do not include control-protocol. product.toml:167 `control-server = false  # Phase 5+ [PROPOSAL]`. Release lane builds only `-p lumen-cli -p desktop` (AGENT-SPEC §7). The real chain that DOES reach frame.rs: `lumen-control-server` bin (apps/control-server/src/main.rs:21-31) -> control_server::serve_loop (run.rs:12) -> control_protocol::ControlServer -> server.rs run() L546 read_frame / L554+841 write_frame / L544 MessageAssembler. That binary is a workspace member (Cargo.toml members `apps/*`) but not in the release lane and its product feature gate is OFF.

## 8. Dependencies & flags
- std-only (`std::io::Read/Write`); no external deps in this file. Error types from crate::error (CloseCode/ProtocolError). No feature gates on any of this code (no cfg in L1-569 outside `#[cfg(test)]` at L571).
- Cargo.toml-level flags: none affecting this file (control-protocol has no features — see its manifest; not re-verified line-by-line here, no cfg(attr) present in file).

## 9. Open questions for the next agent
- server.rs `send_json` (L836-843) writes text frames with NO length check — the reader side enforces 1 MiB but the writer can emit larger frames; if any catalog response JSON exceeds 1 MiB (or 65535, entering the untested 64-bit form, F-002) the client's `read_frame_unmasked(..., 1 << 20)` (client.rs:304) or any conforming peer would reject it. Check the catalog response size in apps/control-server.
- server.rs:568 echoes the peer's raw close code — parse_close_payload already rejects invalid wire codes, but confirm no other path writes a close with a CloseCode the reader itself would reject.
- The single-connection claim ("the single-connection server", server.rs Timeouts doc) bounds F-003's amplification — verify serve_loop really is accept-one-at-a-time.
