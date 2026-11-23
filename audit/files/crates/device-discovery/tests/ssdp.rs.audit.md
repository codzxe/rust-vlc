# AUDIT: crates/device-discovery/tests/ssdp.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-02-14 |
| Lines / functions | 267 / 3 helpers + 5 tests |
| File verdict | REAL (genuine loopback-socket e2e; one tautological test, honestly labeled) |
| Reachable from shipped entry point? | NO for the crate — device-discovery's only consumers are platform-{linux,windows,macos}, which no app depends on; matrix row `sd-upnp` = blocked on all platforms |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p device-discovery --test ssdp` → 5 passed/0 failed in 1.04s (real UDP sockets, own run); rg Cargo.toml hunts |

## 1. One-paragraph verdict
This is a real integration suite, not a mock theater: an in-process responder binds a real loopback UDP socket, receives the searcher's actual M-SEARCH bytes (returned to the test over a channel as wire evidence), and replies with spec-shaped 200 responses; the tests assert USN dedup, malformed-response counting, honest empty results, and cap-based early stop. I ran all 5 green in 1.04 s. The strongest test validates the emitted M-SEARCH against the UDA 1.1 §1.2.3 wire shape (`HOST: 239.255.255.250:1900`, `MAN: "ssdp:discover"`, `ST:`) using the received bytes, not a mock's echo. Two weaknesses: the fifth test (`multicast_api_calls_succeed_or_fail_typed`) accepts both success and typed-failure outcomes and therefore cannot fail — a tautology, albeit one whose name and doc say exactly that; and the cap test's `< 1 s` wall-clock assertion is timing-flaky by construction on a loaded 2-core box. The module doc is honest that multicast-LAN behavior stays `[VERIFY]`.

## 2. What actually works (verified)
— `cargo test -p device-discovery --test ssdp` → `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.04s` (own run; real sockets — the responder thread received and channel-returned the actual datagram). Loopback roundtrip asserts 2 unique USNs from 3 replies (dedup), `max_age == Some(1800)` parsed from CACHE-CONTROL, and the exact wire shape of the request.

## 3. Findings
F-001 [S4]
Where: crates/device-discovery/tests/ssdp.rs:258-266
Code:
```
    match outcome {
        Ok(()) => { /* this host: the kernel accepted the whole path */ }
        Err(DiscoveryError::Io { context, .. }) => {
            // A hermetic environment refusing sockets is a typed,
            // honest outcome — recorded, not a test failure.
            eprintln!("multicast probe refused on this host at {context}");
        }
        Err(other) => unreachable!("the probe's failure modes are typed Io, got {other:?}"),
    }
```
What is wrong: both `Ok` and `Err(Io)` are accepted, so the test cannot fail for any behavior of the multicast API path — an implementation that never joins the group passes identically. It pins only "does not panic and errors are typed", which its name honestly states. Why it matters: if anyone ever cites this test as multicast evidence, it is evidence of nothing; today nothing does (matrix `sd-upnp` = blocked, empty evidence — verified docs/feature-matrix.toml:2617-2626). What correct looks like: assert the sandbox-specific expected branch (this host accepted the whole path — it did, per the run), or mark `#[ignore]` with a LAN-runner feature. Confidence: HIGH — Verification: ran (passed on this host via the Ok branch; no assertion either way).

F-002 [S4]
Where: crates/device-discovery/tests/ssdp.rs:201-208
Code:
```
    let started = std::time::Instant::now();
    let outcome = searcher.search("upnp:rootdevice").must();
    assert_eq!(outcome.responses.len(), 2);
    // It stopped at the cap, not at the window: far short of 2 s.
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "the cap is the stop condition, not the timeout"
    );
```
What is wrong: a wall-clock bound as the proof of early-stop. On a loaded 2-core CI box a 1 s scheduler hiccup fails this test even though the code is correct (the cap did stop the window). The functional assertion (`responses.len() == 2`) is solid; the timing one is flake surface. What correct looks like: instrument the searcher's stop-reason (cap vs window-close) and assert on that enum. Confidence: HIGH — Verification: read; test passed in 1.04 s total suite time on this host.

## 4. Per-function table (test-file scope)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Responder::spawn | 32-64 | REAL | — | real UdpSocket loopback; 3 s read deadline; replies in order; channel evidence path |
| Responder::received_request | 67-69 | REAL | — | 3 s timeout, must() — loud on silence |
| Responder::Drop | 72-78 | REAL | — | joins thread (bounded hang ≤3 s) |
| searcher_to / spec_response | 80-101 | REAL | — | config with_limits(addr, 1, cap, window); UDA-shaped reply builder |
| loopback_roundtrip_collects_parses_and_dedups | 103-143 | REAL | — | dedup + parse + wire-shape assertions; would fail if M-SEARCH, dedup, or CACHE-CONTROL parsing broke |
| loopback_roundtrip_skips_malformed_and_keeps_going | 145-161 | REAL | — | wrong method + empty LOCATION counted (2), good response survives |
| empty_wire_is_an_honest_empty_result | 163-184 | REAL | — | dead port → Ok, zero responses (note: binds-then-drops the target socket; theoretical ephemeral-port reuse race, harmless) |
| max_responses_stops_the_window_early | 186-209 | REAL | S4 | F-002 timing flake |
| multicast_api_calls_succeed_or_fail_typed | 217-267 | REAL (as labeled) / tautology | S4 | F-001; does exercise `build_msearch` on the real API path |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| loopback_roundtrip_collects_parses_and_dedups | YES | wire bytes + dedup + max-age all asserted from real roundtrip |
| loopback_roundtrip_skips_malformed_and_keeps_going | YES | skipped_malformed == 2 and 1 survivor |
| empty_wire_is_an_honest_empty_result | YES (weakly) | zero responses typed Ok — catches a hang/refusal regression |
| max_responses_stops_the_window_early | YES on the cap; NO on the timing | len==2 is real; elapsed<1s is environment |
| multicast_api_calls_succeed_or_fail_typed | NO | both outcomes accepted (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "real UDP, real wire bytes, no mocks of the socket layer" | ssdp.rs:4-5 | REAL | Responder binds UdpSocket; ran green |
| "Multicast-LAN behavior stays [VERIFY]" | ssdp.rs:10-11 | REAL (honest) | no responder exists in sandbox; tautology test doesn't fake it |
| matrix sd-upnp "blocked" all platforms | docs/feature-matrix.toml:2617-2626 | REAL (honest) | consistent with crate not shipped |

## 7. Dead / duplicated / unreachable code
None in the file. Crate-level: device-discovery → platform-* chain has no app consumer; `sd-upnp` matrix row honestly blocked. Not a defect of this file.

## 8. Dependencies & flags
Test-only deps: testkit (Must). Uses std sockets/threads only. No feature gates, no unsafe.

## 9. Open questions for the next agent
- `crates/device-discovery/src/ssdp.rs` itself (the searcher under test) is a separate assignment/agent — this file's verdicts say nothing about the production parser's internals.
