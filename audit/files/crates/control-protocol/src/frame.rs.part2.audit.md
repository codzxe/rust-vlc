# AUDIT: crates/control-protocol/src/frame.rs#part2 (lines 455–909)
| Field | Value |
|---|---|
| Auditor | W02-A06 |
| Date | 2026-11-21 |
| Lines / functions | 455 (chunk part2 of 909; part1 = 1–454, audited by W02-A04) / 6 production items + 2 test helpers + 11 tests |
| File verdict | UNREACHABLE (code itself is REAL and correct; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — neither `apps/cli` (Cargo.toml deps: no control-protocol) nor `apps/desktop-tauri` (deps: desktop, commands, tauri, serde) nor `apps/desktop` (deps: commands, lumen-cli, media-types, renderer-wgpu, video-engine) depends on control-protocol. Sole consumer: `apps/control-server` (binary `lumen-control-server`, Cargo.toml:37), which is NOT in the packaging payload (scripts/packaging ships lumen-cli / desktop / lumen-desktop-tauri) and is flagged `control-server = false # Phase 5+ [PROPOSAL]` (product.toml:167). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `sed -n` on all quoted ranges; `rg -n "control-protocol"` in apps/*/Cargo.toml (only apps/control-server); `rg -n "control-server|lumen"` scripts/packaging/* (no control-server payload); `rg -n "write_masked_frame|parse_close_payload|close_payload"` → client.rs:23,109,123,163,177,198,257; server.rs:51,52,560,871; `rg -n "read_frame\(" server.rs` → cap = `Limits::DEFAULT_MAX_FRAME_BYTES = 1024*1024` (server.rs:83); baseline test log rg `frame::tests` → 11/11 ok (test-workspace.log.txt:1422-1433) |

## 1. One-paragraph verdict
Part2 is the tail of the in-tree RFC 6455 codec: close-payload parse/build, close-code validity, masking, the masked client writer, and an 11-test suite that is genuinely hostile (truncations, RSV bits, reserved opcodes, oversize claims, dirty fragment sequences, invalid UTF-8, a sentinel-checked regression test for the phantom-mask-byte hang). The code is correct against RFC 6455 to the depth I traced: caps checked before allocation, MSB-length rejection, control-frame limits, on-wire close-code ranges, char-boundary-safe reason truncation, symmetric mask/unmask. All 11 tests executed and passed in the baseline workspace run. Two real defects: none of this is reachable from a shipped binary (the only consumer, `lumen-control-server`, is excluded from the packaging payload and disabled in product.toml), and the test named `roundtrip_all_length_forms` never round-trips the 64-bit length form — the write-side 64-bit arm executes in no test at all. Plus two hygiene items in `write_masked_frame` and `close_payload`.

## 2. What actually works (verified)
- All 11 tests passed in baseline run #3: `audit/runs/baseline/test-workspace.log.txt:1422-1433` — `frame::tests::{assembler_rejects_dirty_fragment_sequences, close_payload_validation, fragmented_text_reassembles_with_interleaved_ping, invalid_utf8_text_is_a_typed_close, oversized_length_claims_reject_before_allocating, rsv_bits_and_bad_opcode_are_rejected, server_writer_roundtrips_through_the_parser_via_masking, truncated_headers_are_typed_errors, unmasked_client_frame_is_rejected, unmasked_server_frames_parse_without_phantom_mask_bytes, roundtrip_all_length_forms}` all `ok`.
- Cap-before-allocation verified by code order: the `length > max_frame` check (part1, 258-263) precedes `read_exact(stream, length as usize)` (part1, 278), and test `oversized_length_claims_reject_before_allocating` (656-681) proves a 4 GiB claim against a 1024 cap yields `MessageTooLarge { limit: 1024, claimed: 4<<30 }` without allocating.
- On-wire close-code ranges (496-499) match RFC 6455 §7.4 (1000-1003, 1007-1011, 3000-4999; rejects 0-999, 1004, 1005, 1006, 1016-2999); the doc claim "invalid UTF-8 is a typed 1007 close" (frame.rs:15-16) traces: `validate_utf8` → `ProtocolError::InvalidUtf8` → `CloseCode::InvalidUtf8` → 1007 (crates/control-protocol/src/error.rs:28,47,159).
- The `unmasked_server_frames_parse_without_phantom_mask_bytes` regression test (833-862) proves exact byte consumption via a trailing-sentinel read (858-860) — a strong test shape.

## 3. Findings

### F-001 [S3] Entire part2 surface unreachable from any shipped entry point
Where: crates/control-protocol/src/frame.rs:526-569 (write_masked_frame), 465-517 (close helpers) — sole consumer apps/control-server
Code:
```rust
/// Write one frame in the CLIENT role: masked (RFC 6455 §5.1/§5.3)
/// with the caller-supplied key — tests pin exact masks so the wire
/// bytes are reproducible.
```
What is wrong: the only production callers of part2's functions are `crates/control-protocol/src/client.rs` (`write_masked_frame` ×5 at 109/123/163/177/198, `parse_close_payload` at 257) and `crates/control-protocol/src/server.rs` (`parse_close_payload`:560, `close_payload`:871) — and the only binary embedding that crate is `apps/control-server` (`lumen-control-server`, apps/control-server/Cargo.toml:37). That binary is not part of the packaging payload (scripts/packaging/gate.sh ships lumen-cli, desktop, lumen-desktop-tauri — "the third shipped bin"), and product.toml:167 gates the feature off: `control-server = false # Phase 5+ [PROPOSAL]`. Neither `lumen` (apps/cli/Cargo.toml:47-66 lists no control-protocol) nor the tauri shell reaches this code.
Why it matters: this is a full RFC 6455 server codec with SHA-1/base64/HTTP-upgrade siblings, tested and linted every CI run, that no user can currently execute. CI cost is real; shipped capability is zero. Any doc/matrix row implying a working remote-control surface today is contradicted by the build graph.
What correct looks like: either ship `lumen-control-server` when Phase 5 lands, or mark the crate's reachability honestly in the matrix (built-but-unshipped) so accounting is truthful.
Confidence: HIGH — Verification: read (Cargo.toml dependency graph + packaging scripts + product.toml); no build/run (forbidden) — baseline workspace build #1 compiled it, which says nothing about shipping.

### F-002 [S3] `roundtrip_all_length_forms` omits the 64-bit length form; the write-side 64-bit arm executes in no test
Where: crates/control-protocol/src/frame.rs:604-607 (test), 554-557 (untested arm)
Code:
```rust
    #[test]
    fn roundtrip_all_length_forms() {
        for len in [0usize, 1, 125, 126, 127, 65535] {
```
```rust
        _ => {
            head.push(0x80 | 127);
            head.extend_from_slice(&(len as u64).to_be_bytes());
        }
```
What is wrong: the name promises all length forms; the list stops at 65535, so the 127/64-bit form is never round-tripped. The 64-bit READ path executes only in the rejection test (`oversized_length_claims_reject_before_allocating` claims 4<<30, still < 2^63, refused at the cap) — a happy-path ≥65537 payload never passes through `read_frame_endpoint`'s 64-bit arm in any test. The 64-bit WRITE arm (554-557 here, and the twin at part1 327-330 in `write_frame`) is executed by NO test: the `client_frame` helper (582-602) supports it (594-595) but no caller supplies >65535 bytes; the other round-trip tests cap at 200 bytes.
Why it matters: mutate the 64-bit write arm (wrong prefix, wrong endianness, dropped arm) or the 64-bit read path's happy case and the suite stays green. The `1 << 63` MSB check (part1 234-239) is likewise only exercised with MSB clear.
What correct looks like: add a ≥65537-byte payload to the round-trip list and one MSB-set 64-bit claim rejection case.
Confidence: HIGH — Verification: read; all 11 test bodies inspected; baseline pass cited for current state.

### F-003 [S4] `write_masked_frame` drops the typed timeout mapping its server twin has; under-reserves the 64-bit form
Where: crates/control-protocol/src/frame.rs:562-568 (vs part1 335-348), 539
Code:
```rust
    stream
        .write_all(&head)
        .and_then(|()| stream.write_all(&masked))
        .map_err(|source| ProtocolError::Io {
            what: "writing frame",
            source,
        })
```
What is wrong: `write_frame` (part1, 335-348) maps `WouldBlock | TimedOut` to `ProtocolError::TimedOut` — the "bounded-wait guarantee … a typed close, never an eternal block" the read path also implements (part1 112-118). `write_masked_frame` maps every error to `ProtocolError::Io`, so a client-role write past a set deadline surfaces as a generic I/O failure instead of the typed timeout. Additionally `Vec::with_capacity(10)` (539) under-reserves the masked 64-bit form, whose header is 2 + 8 + 4 = 14 bytes — a harmless reallocation, but the server writer's `10` (part1, 311) is the correct unmasked bound and was evidently copy-pasted.
Why it matters: the client is the role conformance tests drive; an asymmetric error taxonomy makes deadline behavior differ by endpoint role, and the capacity mismatch is a copy-paste smell.
What correct looks like: share one header-build + error-map helper between both writers (they differ only in the mask bit, mask bytes, and payload masking).
Confidence: HIGH — Verification: read; both error arms quoted.
(Note: 335-348 and 311 are in part1's chunk; the asymmetry is only visible from part2's function, so it is recorded here for W02-A04 to reconcile.)

### F-004 [S4] `close_payload`'s truncation loop is dead structure
Where: crates/control-protocol/src/frame.rs:508-514
Code:
```rust
    let mut reason_bytes = reason.as_bytes();
    while reason_bytes.len() > 123 {
        let mut cut = 123;
        while cut > 0 && !reason.is_char_boundary(cut) {
            cut -= 1;
        }
        reason_bytes = &reason_bytes[..cut.max(1)];
    }
```
What is wrong: the outer `while` can iterate at most once — the body always reduces `reason_bytes` to ≤ 123 bytes (cut starts at 123), so it is an `if` wearing a loop. The `cut.max(1)` guard is unreachable for any valid `&str`: within the first 123 bytes of UTF-8 there is always a char boundary (max char length is 4), and position 0 is always a boundary. Output is correct; the shape lies.
Why it matters: a reader must simulate the loop to learn it runs once; the dead guard implies a multi-iteration truncation story that cannot happen.
What correct looks like: `if reason_bytes.len() > 123 { let mut cut = 123; while !reason.is_char_boundary(cut) { cut -= 1; } reason_bytes = &reason_bytes[..cut]; }`.
Confidence: HIGH — Verification: read; loop-invariant reasoning; output bounds (≤ 2 + 123 = 125) verified.

## 4. Per-function table (part2 items)
| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `validate_utf8()` | 456-463 | REAL | — | Private; `std::str::from_utf8` check → typed `InvalidUtf8`. Callers in part1 (`MessageAssembler::push` 407, 421). No panics. |
| `parse_close_payload()` | 465-494 | REAL | — | 0→None, 1→typed truncated-code error, ≥2→code+UTF-8 reason; wire-validity via `is_valid_on_wire_close_code`. `payload[2..]` slice safe (len ≥ 2 in arm). Production callers: client.rs:257, server.rs:560. |
| `is_valid_on_wire_close_code()` | 496-499 | REAL | — | Single `matches!`; RFC 6455 §7.4 ranges correct. Only caller found: `parse_close_payload` (+tests) — `pub` for the conformance suite. |
| `close_payload()` | 501-517 | REAL | S4 (F-004) | Output always ≤ 125 bytes, reason truncated on a char boundary. Dead loop structure (F-004). Caller: server.rs:871. |
| `apply_mask()` | 519-524 | REAL | — | Byte-wise XOR with repeating 4-byte key; symmetric mask/unmask per RFC 6455 §5.3. O(n) — fine for ≤1 MiB control frames (server cap, server.rs:83). No SIMD; acceptable for the control lane, would be a defect on a media lane. |
| `write_masked_frame()` | 526-569 | REAL | S4 (F-003); 64-bit arm untested (F-002) | Control-frame >125 refusal (542-546); all three length forms emit correctly (0x80|len / 0x80|126+u16 / 0x80|127+u64); `payload.to_vec()` allocates per frame — fine at control-lane volume. Error-map asymmetry + capacity nit (F-003). Callers: client.rs ×5. |
| test helper `reader()` | 575-578 | REAL | — | Cursor wrapper. |
| test helper `client_frame()` | 582-602 | REAL | — | Builds masked client frames; supports the 64-bit form (594-595) that no test uses (F-002). The `if fin {0x80} else {0x00} | opcode.as_u8()` precedence parses as `(if…) | opcode` in argument position — proven by the passing round-trip tests (a wrong parse would yield opcode 0 and fail `assert_eq!(frame.opcode, Opcode::Text)` at 613). |
| test `roundtrip_all_length_forms` | 604-620 | REAL (misnamed) | S3 (F-002) | Covers 7-bit and 16-bit forms incl. both boundaries; omits 64-bit. |
| test `unmasked_client_frame_is_rejected` | 622-630 | REAL | — | Raw-bytes negative; would fail if the mask check were dropped. |
| test `rsv_bits_and_bad_opcode_are_rejected` | 632-654 | REAL | — | RSV1 and reserved-opcode negatives. |
| test `oversized_length_claims_reject_before_allocating` | 656-681 | REAL | — | 4 GiB claim vs 1024 cap; control 200>125. Strong. |
| test `truncated_headers_are_typed_errors` | 683-696 | REAL | — | Cuts 1..=7 across header/mask/payload; asserts typed errors. |
| test `fragmented_text_reassembles_with_interleaved_ping` | 698-729 | REAL | — | 3 fragments + interleaved ping → "hello world". |
| test `assembler_rejects_dirty_fragment_sequences` | 731-782 | REAL | — | Orphan continuation, interleaved start, cap-exceeded fragments. |
| test `invalid_utf8_text_is_a_typed_close` | 784-798 | REAL | — | Masked 0xff 0xfe text → `InvalidUtf8`. |
| test `close_payload_validation` | 800-827 | REAL | — | Empty/1-byte/1005/invalid-UTF-8/4409 cases. Does not test 1004, 1006, 1016-2999 — single `matches!` fn, acceptable. |
| test `unmasked_server_frames_parse_without_phantom_mask_bytes` | 833-862 | REAL | — | Regression for the conformance hang; trailing-sentinel proves exact consumption. Strong. |
| test `server_writer_roundtrips_through_the_parser_via_masking` | 864-908 | REAL | — | Writer-vs-hand-masking byte equality + parse-back. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| roundtrip_all_length_forms | YES (for ≤16-bit forms) | Exact payload equality per length incl. boundary lengths 125/126/127/65535. NO for 64-bit-form breakage (F-002). |
| unmasked_client_frame_is_rejected | YES | Direct negative on the §5.1 mask requirement. |
| rsv_bits_and_bad_opcode_are_rejected | YES | Bit-level mutation of real frames. |
| oversized_length_claims_reject_before_allocating | YES | Asserts the typed variant AND the limit/claimed values — pins the typed refusal path. |
| truncated_headers_are_typed_errors | YES | Seven cut points across all header regions. |
| fragmented_text_reassembles_with_interleaved_ping | YES | Byte-exact reassembly assert. |
| assembler_rejects_dirty_fragment_sequences | YES | Three distinct protocol violations, each asserted by reason text. |
| invalid_utf8_text_is_a_typed_close | YES | Variant-level assert on `InvalidUtf8`. |
| close_payload_validation | YES | Five cases incl. the 1005 wire-forbidden code. |
| unmasked_server_frames_parse_without_phantom_mask_bytes | YES | Sentinel read catches any byte-count drift (this exact regression once shipped — comment 829-832). |
| server_writer_roundtrips_through_the_parser_via_masking | YES | Byte-equality against an independent hand-masking construction. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the 64-bit form's MSB must be zero and the claim is checked against the cap BEFORE any allocation: an oversized claim is a typed MessageTooLarge, never an OOM vector" | frame.rs:6-9 | REAL | Code order (cap at 258-263 before read at 278) + test 656-681, baseline-passed. MSB check itself only exercised with MSB clear (F-002 gap). |
| "close (payload = 2-byte code + UTF-8 reason, validated)" | frame.rs:12-14 | REAL | `parse_close_payload` 465-494 + test 800-827. |
| "invalid UTF-8 is a typed 1007 close" | frame.rs:15-16 | REAL | error.rs:28,47,159 (InvalidUtf8 → 1007). |
| "full frame codec incl. fragmentation and control frames" (crate purpose) | crates/control-protocol/Cargo.toml:5-7 | REAL (code) / UNREACHABLE (shipped) | 11 baseline-passed tests; sole consumer not in packaging payload (F-001). |
| control-protocol as shipped remote-control surface | crate doc (doc 01 §2.3 registry); product.toml:167 | UNREACHABLE | product.toml:167 `control-server = false # Phase 5+ [PROPOSAL]`; packaging payload has no lumen-control-server. |

## 7. Dead / duplicated / unreachable code
- The whole crate is dead-in-shipped-build (F-001) — kept deliberately for Phase 5; not proposing deletion, flagging reachability.
- `close_payload` outer loop is effectively an `if` (F-004).
- `write_frame` vs `write_masked_frame`: duplicated header construction differing only in mask handling (F-003's "what correct looks like").
- `is_valid_on_wire_close_code` is `pub` with no production caller outside the module — conformance-suite surface.

## 8. Dependencies & flags
- Crate deps: commands, serde, serde_json only (Cargo.toml:31-34) — std-only transport by ADR-011 (no tungstenite/sha1/base64 crates); claim verified: SHA-1 and base64 live in sibling modules (sha1.rs, base64.rs).
- No feature flags gate frame.rs; no `unsafe`; no panics reachable from wire input in part2 (all indexing preceded by exact-size reads; slice `payload[2..]` guarded by the `match payload.len()` arm).
- Part1 note for W02-A04: `read_exact(stream, length as usize)` (278) relies on the caller's `max_frame` being sane — server passes 1 MiB (server.rs:83), so safe in practice; a `debug_assert!(length <= usize::MAX as u64)` would pin it.

## 9. Open questions for the next agent
- W02-A04 (part1): the mask/timeout/capacity asymmetries noted in F-003 sit across our boundary — reconcile into one finding if you found them too.
- Wave auditing apps/control-server: confirm whether any doc/matrix row claims a usable remote-control server today (product.toml says Phase 5+ [PROPOSAL]); F-001's reachability facts are reusable.
- Conformance suite (wave 16 owns conformance.rs): the 64-bit length round-trip gap (F-002) may be covered there — I did not audit conformance.rs; if it covers ≥65537 payloads, downgrade F-002 to a naming nit.
