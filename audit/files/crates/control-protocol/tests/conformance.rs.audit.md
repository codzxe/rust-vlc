# AUDIT: crates/control-protocol/tests/conformance.rs
| Field | Value |
|---|---|
| Auditor | W02-A04 |
| Date | 2026-11-23 |
| Lines / functions | 1054 / 36 (25 tests + 11 harness/helper fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test binary; SUBJECT is serve_one — real code, but the whole control-protocol stack is NOT in the release lane (see §7) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `wc -l` (1054); `sed -n` quote verification (all quotes); direct run of prebuilt binary `./target/debug/deps/conformance-fc3f1f30343cb626` → exit 0, **25 passed; 0 failed in 0.16s**; baseline `cargo-test-workspace.txt:1610-1638` (25 passed) |

## 1. One-paragraph verdict
This is a genuine conformance suite, not an echo chamber. Every one of the 25 cases drives the REAL production session loop (`ControlServer::serve_one`, the identical method the real host calls at apps/control-server/src/run.rs:54) over a REAL loopback TCP socket with a REAL RFC 6455 upgrade handshake, and asserts on typed observable outcomes: close codes (1000/1001/1002/1007/1009/4400/4401/4402), `ProtocolErrorCode` values, raw HTTP status lines for non-upgrade peers, pong payload equality, and `SessionSummary` counters. I executed the prebuilt binary: 25 passed, 0 failed, 0.16 s. Mutation reasoning (no source edits, per spec): delete the mask enforcement, the hello-first gate, the anti-replay check, the close-echo, the lockout, or the deny-by-default gate, and specific named tests fail with the asserted error codes. The honest boundaries: the command layer is stubbed (`EchoDispatch`), the "bad masks" item of the header's coverage list is not (and cannot be) tested as claimed, the subscribe path asserts nothing, and the test oracle shares the frame codec with the subject so mirrored codec bugs are invisible.

## 2. What actually works (verified)
— verified by `./target/debug/deps/conformance-fc3f1f30343cb626` → `test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s`, EXIT=0 (run by me, 2026-11-23; matches baseline cargo-test-workspace.txt:1636-1638). The binary hash matches the baseline build (`conformance-fc3f1f30343cb626`).
— verified by `sed -n '54p' apps/control-server/src/run.rs` → `match server.serve_one(dispatch) {` — the suite drives the same entry the real host loop calls.

## 3. Findings
F-001 [S3]
Where: crates/control-protocol/tests/conformance.rs:10-13 (claim) vs 562-572 (body)
Code:
```
//! Coverage (the card's acceptance list): handshake success/failure,
//! token wrong/right, version mismatch, malformed frames (truncated
//! headers, bad masks, oversized length claims), fragmentation
```
and the case mapped to "bad masks":
```
    // Valid masked TEXT frame whose payload is not JSON.
    let garbage: [u8; 8] = [0x00, 0xff, 0x10, 0x01, 0xde, 0xad, 0xbe, 0xef];
    send(&mut client, &String::from_utf8_lossy(&garbage));
```
What is wrong: the coverage list promises a "bad masks" case; no such case exists. The mapped test sends a VALID mask with a garbage payload — its own comment (563-568) admits a wrong mask is unobservable because the mask key travels in-band. Why it matters: the suite's header is its contract (the card's acceptance list); a reader believes mask handling is conformance-tested when the actual coverage is "garbage payload → typed bad-json". What correct looks like: drop "bad masks" from the coverage list or state precisely "garbage payload after a valid mask (bad masks are unobservable by design)".
Confidence: HIGH — Verification: read

F-002 [S3]
Where: crates/control-protocol/tests/conformance.rs:993-1003
Code:
```
    // Subscribe is accepted (recorded; the event push is the host
    // engine's business — no ack frame in the doc 05 envelope).
    send(
        &mut client,
        r#"{"type":"subscribe","events":["timing.seek-completed"]}"#,
    );
    send(&mut client, &engine_info_request(1));
```
What is wrong: the subscribe sub-case asserts NOTHING about subscribe handling — no response, no error, no summary field (`SessionSummary` in server.rs:263-286 exposes no subscription state). If the server silently dropped or mis-parsed every subscribe frame, all 25 tests still pass. Why it matters: `subscribe` is half of the application envelope (client frames: hello/request/subscribe) and has zero conformance coverage; the test name (`binary_application_frames_are_rejected_but_survive`) hides it. What correct looks like: expose `subscriptions` in `SessionSummary` and assert it contains `timing.seek-completed`.
Confidence: HIGH — Verification: read (SessionSummary fields verified at server.rs:462-472)

F-003 [S3]
Where: crates/control-protocol/src/client.rs:303-311 (the suite's oracle) + crates/control-protocol/src/frame.rs:165-181
Code:
```
fn read_client_side_frame<R: Read>(stream: &mut R) -> Result<Option<Frame>, ProtocolError> {
    crate::frame::read_frame_unmasked(stream, 1 << 20).map(|option| {
```
and
```
pub fn read_frame<R: Read>(stream: &mut R, max_frame: u64) -> Result<Option<Frame>, ProtocolError> {
    read_frame_endpoint(stream, max_frame, Endpoint::Server)
}
```
What is wrong: the test oracle (TestClient) parses server frames with the SAME `read_frame_endpoint` the subject uses; a mirrored codec bug — e.g., masking XOR applied wrongly in BOTH `write_masked_frame` and the reader, or a wrong extended-length encoding in both writer and parser — passes the entire suite. The frame codec has no external anchor: its unit tests are self-roundtrips (`frame::tests::roundtrip_all_length_forms`, `server_writer_roundtrips_through_the_parser_via_masking`, baseline :1568,1584). Why it matters: "no synthetic echo chambers (D-5)" is claimed at conformance.rs:5, but at the codec layer the oracle IS the subject's mirror. (The HTTP handshake IS externally anchored: `sha1::tests::external_vectors`, `http::tests::accept_value_matches_rfc6455_example`, baseline :1581-1582.) What correct looks like: pin one RFC 6455 §5.7 fragmented-frames-plus-masking byte vector as an external anchor in frame.rs tests.
Confidence: HIGH — Verification: read

F-004 [S4]
Where: crates/control-protocol/tests/conformance.rs:125-131, 133-149
Code:
```
/// The test dispatch: serves `system.get-engine-info` (class S
/// `Completed`) and answers everything else with a class-S
/// `Completed(Unit)` (the response shape is the layer's, not the
/// policy's — the policy gate runs BEFORE this).
```
What is wrong: nothing dishonest — the seam is documented — but the consequence must be recorded: NO command execution is conformance-tested. The real host dispatch (apps/control-server/src/dispatch.rs `CatalogDispatch`, which answers most commands with a typed not-ready per lib.rs:15-17) is never exercised by this suite; a divergence between what the suite's stub returns and what the real dispatch returns is invisible here. What correct looks like: an additional e2e suite against `CatalogDispatch` (exists separately in apps/control-server/tests/e2e.rs — not this file's job, but the gap belongs in the ledger).
Confidence: HIGH — Verification: read

F-005 [S4]
Where: crates/control-protocol/tests/conformance.rs:290-297
Code:
```
    assert!(
        !head.contains("101"),
        "no protocol switch for a non-upgrade request"
    );
```
What is wrong: substring check on the whole raw response — a legitimate header value containing "101" (e.g. `Content-Length: 101`) would false-FAIL the test. Cannot false-pass, so hygiene only. Same pattern at 333-336. What correct looks like: assert the status line only (`head.starts_with("HTTP/1.1 400")` already covers it) or parse the head.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Harness::spawn | 61-63 | REAL | — | delegates to spawn_full with documented limits |
| Harness::spawn_full | 65-71 | REAL | — | Fixed(TOKEN) + policy + auth overrides |
| Harness::spawn_config | 73-106 | REAL | — | ephemeral 127.0.0.1:0, real bind + serve_one thread |
| Harness::addr | 109-114 | REAL | — | blocks on mpsc ready |
| Harness::join | 117-122 | REAL | — | returns real SessionSummary vec |
| EchoDispatch::dispatch | 133-149 | STUB (by design) | S4 | returns Completed(Unit) for all but engine-info; documented seam (F-004) |
| test_engine_info | 154-165 | REAL | — | fixture incl. live catalog_checksum() |
| connect / send / next_frame / close_code / error_then_close | 168-206 | REAL | — | typed-fail helpers; every failure fails the test |
| hello_text / request_text / engine_info_request | 209-229 | REAL | — | builds real wire JSON via serde/format |
| handshake_success_full_session_round_trip | 236-273 | REAL | — | welcome caps, request round-trip, close-1000 echo, full summary |
| handshake_failure_non_upgrade_http_gets_an_http_error | 276-300 | REAL | S4 | raw TCP, 400 asserted; brittle "101" substring (F-005) |
| dns_rebound_host_names_never_upgrade | 303-340 | REAL | — | valid WS headers + evil Host → 400; kills the rebinding fence |
| wrong_token_closes_4401_and_counts | 347-370 | REAL | — | 4401 + no token material in message + auth_failures=1 |
| frames_before_authentication_are_refused | 373-390 | REAL | — | NotHello + 4400 + requests_served=0 |
| five_failures_lock_out_even_the_right_token | 393-433 | REAL | — | 5×bad then RIGHT token still 4401; locked_out, auth_failures=5 |
| version_mismatch_is_a_typed_error_and_close_4402 | 440-457 | REAL | — | [2,3] → VersionMismatch + 4402 |
| unknown_protocol_name_is_rejected | 460-474 | REAL | — | wrong protocol string → UnknownProtocol + 4400 |
| hello_twice_closes | 477-493 | REAL | — | HelloTwice + 4400 |
| truncated_frame_header_closes_1002 | 500-521 | REAL | — | raw 3-byte head + write-shutdown → EOF mid-frame → 1002 |
| unmasked_client_frame_closes_1002 | 524-535 | REAL | — | raw unmasked bytes → 1002 |
| oversized_length_claim_closes_1009_before_allocation | 538-559 | REAL | — | 4 GiB claim → 1009, no alloc |
| garbage_masked_payload_is_a_recoverable_json_error | 562-592 | REAL | S3 | valid mask + garbage → BadJson, session survives; NOT a bad-mask test (F-001) |
| invalid_utf8_text_closes_1007 | 595-612 | REAL | — | masked 0xff 0xfe 0xfd → 1007 |
| fragmented_hello_round_trips_with_interleaved_ping | 619-655 | REAL | — | 3 fragments + mid-fragment ping → pong BEFORE reassembly, then welcome |
| ping_is_answered_by_a_matching_pong | 662-692 | REAL | — | pong payload equality + unsolicited pong tolerated + session serves after |
| close_without_code_is_echoed_empty_then_eof | 695-720 | REAL | — | empty close echoed empty + TCP EOF + PeerClosed(None) |
| non_loopback_bind_is_refused_at_construction_conformance | 727-736 | REAL | — | 0.0.0.0/10.0.0.1/example.com all refused (duplicates server.rs unit test — fine) |
| sensitive_command_denied_by_default_and_connection_survives | 743-780 | REAL | — | Denied + survive + ops_denied=1 + requests_served=1 |
| sensitive_command_allowed_when_config_grants_it | 783-829 | REAL | — | config-driven grant; sibling op still denied |
| unauthenticated_read_only_mode_serves_the_safe_subset_only | 832-879 | REAL | — | no token; media.get-info passes, playback.play denied, read_only advertised |
| request_validation_rejects_dirty_payloads_and_survives | 886-950 | REAL | — | unknown field / unknown cmd / bad arg type / JSON array → 4 typed errors, then survives; protocol_errors=4 |
| non_monotonic_request_id_closes_the_connection | 953-971 | REAL | — | id replay → NonMonotonicRequestId + 4400 |
| binary_application_frames_are_rejected_but_survive | 978-1011 | REAL | S3 | binary frame rejection verified; subscribe sub-case asserts nothing (F-002) |
| idle_session_is_closed_by_the_read_deadline_1001 | 1018-1054 | REAL | — | 150 ms deadlines; silent peer → Internal("deadline") + 1001 + GoingAway summary |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| handshake_success_full_session_round_trip | YES | asserts welcome fields, response, close-1000 echo, and all summary counters (mutation: break close echo → `assert_eq!(close_code, Some(1000))` fails) |
| handshake_failure_non_upgrade_http_gets_an_http_error | YES | raw socket asserts literal `HTTP/1.1 400` status line (upgrade removed → 101 → fail) |
| dns_rebound_host_names_never_upgrade | YES | Host-check removed → 101 → fail |
| wrong_token_closes_4401_and_counts | YES | token compare broken → welcome frame instead of protocol-error → unreachable! fires |
| frames_before_authentication_are_refused | YES | hello gate removed → Response frame → unreachable! fires |
| five_failures_lock_out_even_the_right_token | YES | lockout removed → welcome on 6th connect → unreachable! fires |
| version_mismatch_is_a_typed_error_and_close_4402 | YES | negotiation broken → wrong code/close |
| unknown_protocol_name_is_rejected | YES | name check removed → welcome → unreachable! fires |
| hello_twice_closes | YES | state machine broken → second welcome → unreachable! fires |
| truncated_frame_header_closes_1002 | YES | EOF-mid-frame unhandled → hang → client 10 s deadline fails test |
| unmasked_client_frame_closes_1002 | YES | mask enforcement removed → bad-json/welcome instead of close 1002 |
| oversized_length_claim_closes_1009_before_allocation | YES | cap removed → waits for 4 GiB → client deadline fails test |
| garbage_masked_payload_is_a_recoverable_json_error | YES (for what it tests) | bad-json handling broken → close instead of recoverable error → fail |
| invalid_utf8_text_closes_1007 | YES | utf8 gate removed/changed code → fail |
| fragmented_hello_round_trips_with_interleaved_ping | YES | assembler broken → no welcome → unreachable! fires |
| ping_is_answered_by_a_matching_pong | YES | pong payload compare is exact |
| close_without_code_is_echoed_empty_then_eof | YES | echo-with-empty broken → Some(code) → fail |
| non_loopback_bind_is_refused_at_construction_conformance | YES | bind gate removed → Ok server → unreachable! fires |
| sensitive_command_denied_by_default_and_connection_survives | YES | policy gate removed → Response frame → unreachable! fires |
| sensitive_command_allowed_when_config_grants_it | YES | grants ignored → Denied frame → unreachable! fires |
| unauthenticated_read_only_mode_serves_the_safe_subset_only | YES | read-only subset broken → denial of safe op / acceptance of playback.play → fail |
| request_validation_rejects_dirty_payloads_and_survives | YES | per-case typed codes + protocol_errors=4 |
| non_monotonic_request_id_closes_the_connection | YES | anti-replay removed → Response → unreachable! fires |
| binary_application_frames_are_rejected_but_survive | PARTIAL | binary rejection: YES; the subscribe send asserts nothing (F-002) |
| idle_session_is_closed_by_the_read_deadline_1001 | YES | deadlines removed → eternal block → client 10 s deadline fails test |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every case drives the REAL server over a REAL loopback socket … no synthetic echo chambers" | conformance.rs:3-5 | REAL (with codec-level caveat F-003) | my run: 25 passed; serve_one is the production entry (run.rs:54) |
| "Coverage … bad masks" | conformance.rs:12 | FALSE-AS-WORDED | no bad-mask case exists/can exist (F-001) |
| "Subscribe is accepted (recorded…)" | conformance.rs:993 | UNVERIFIED BY THIS SUITE | no assertion observes subscribe state (F-002) |

## 7. Dead / duplicated / unreachable code
No dead code. `non_loopback_bind_is_refused_at_construction_conformance` duplicates `server::tests::non_loopback_binds_are_refused_at_construction` (baseline :1596) — acceptable conformance redundancy. REACHABILITY OF THE SUBJECT: control-protocol is depended on ONLY by apps/control-server (rg over all Cargo.tomls); the release lane builds `-p lumen-cli -p desktop` (baseline/cargo-build-release-lumen-cli.txt:1) which do NOT depend on it, and product.toml:167 `control-server = false # Phase 5+ [PROPOSAL]`. So this suite conformance-tests real code that currently ships nowhere — correct code, unwired product.

## 8. Dependencies & flags
No dev-dependencies, no features (crates/control-protocol/Cargo.toml:31-34: commands, serde, serde_json only). No #[ignore], no cfg gates — all 25 tests ran (my execution, 0 ignored). No `unsafe` in the test file.

## 9. Open questions for the next agent
- apps/control-server/tests/e2e.rs: does it cover the `CatalogDispatch` not-ready answers and the config-file → ServerPolicy path this suite stubs?
- frame.rs: is there ANY external (non-self-roundtrip) anchor for masking/length forms? (F-003 says no for the codec.)
- SessionSummary has no `subscriptions` field — intended gap or oversight (server.rs:263-286)?
