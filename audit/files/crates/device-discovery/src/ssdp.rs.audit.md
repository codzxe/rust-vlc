# AUDIT: crates/device-discovery/src/ssdp.rs
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-23 |
| Lines / functions | 584 / 10 prod fns + 7 unit tests (+ 5 real-socket integration tests in tests/ssdp.rs) |
| File verdict | REAL — genuine std-only UDP M-SEARCH implementation, executed by me over real sockets; UNREACHABLE from shipped entry points (crate has zero dependents) |
| Reachable from shipped entry point? | NO — no Cargo.toml in the repo depends on `device-discovery` (rg over all Cargo.tomls: only comment mentions in platform-{linux,windows,macos}); no app references it; the UI's DiscoveryScreen.tsx:28-30 states it plainly: "the engine's SSDP/UPnP searcher exists (crates/device-discovery, src/ssdp.rs) but no command exposes it to the UI yet". The crate is not even compiled into the release lane (`cargo build --release -p lumen-cli -p desktop` builds only their dep trees). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `./target/debug/deps/device_discovery-80b8532926b93caf ssdp` → 7 passed exit 0; `./target/debug/deps/ssdp-2e5d81d1a43e81af` → 5 passed / 0 failed exit 0 (real UDP loopback + multicast API probe on this host, 0.62s); rg dependents; sed quote verification |

## 1. One-paragraph verdict
Answering the hostile-focus question: the SSDP code is REAL and TESTABLE, and I executed it — the M-SEARCH codec is byte-exact UDA 1.1, the response parser is dirty-world tolerant with typed errors, and the socket searcher completed real UDP round-trips against an in-process loopback responder (send M-SEARCH → receive → parse → dedup by USN → bounded window), plus a real `join_multicast_v4` + multicast send accepted by this host's kernel. What is honestly missing and admitted at every level: no end-to-end discovery against real LAN hardware ever ran ([VERIFY] recorded, zero responders in the sandbox), no description/SOAP follow-up (v0 stops at location URLs as data), and — decisive for the audit — nothing shipped can reach any of it: the crate has zero dependents, no command exposes the searcher, and the UI screen renders an honest status line instead of a search button. The module's own five-finding header is the most honest document I have audited in this repo; the defects I found are a doc/code bound mismatch and two parser-tolerance gaps.

## 2. What actually works (verified)
— verified by `./target/debug/deps/ssdp-2e5d81d1a43e81af` → `test result: ok. 5 passed` (exit 0): loopback roundtrip collects/parses/dedups (2 distinct USNs from 3 replies), skips malformed datagrams without dying (skipped_malformed == 2), empty wire → Ok(zero), max_responses cap stops the window in <1 s, and the multicast API probe (bind + group join + TTL + send to 239.255.255.250:1900) succeeded on this host. Unit tests (7/7, exit 0) pin the M-SEARCH byte shape and the full parser polarity table.

## 3. Findings
F-001 [S3]
Where: crates/device-discovery/src/ssdp.rs:357 (search) / repo Cargo.tomls
Code: (search signature, L357)
```
    pub fn search(&self, search_target: &str) -> Result<SsdpSearchOutcome, DiscoveryError> {
```
What is wrong: the entire SSDP lane is unreachable from any shipped entry point — no crate depends on `device-discovery`, no CLI command, no desktop/web host call, and the UI says so. Unlike `playlists` (which ships via streaming) this crate is not even in the release binary. 584 lines of working, socket-tested code with no consumer. The phase7-exit gate (docs/gates/phase7-exit.md:229) calls 7.D1 "LANDED" — accurate for the crate+UI, but a reader could mistake that for user-visible discovery; it is not.
Confidence: HIGH — Verification: read + rg

F-002 [S4]
Where: crates/device-discovery/src/ssdp.rs:109-111 vs 210-215
Code: (210-215)
```
    let required = |field: &str, value: Option<String>| match value {
        Some(v) if v.len() > FIELD_CAP => {
            Err(malformed(format!("{field} exceeds {FIELD_CAP} bytes")))
        }
```
What is wrong: the `SsdpResponse::location` doc promises "≤ [`LOCATION_CAP`] bytes" (2048), but `required("LOCATION", …)` enforces `FIELD_CAP` = 256 — a legal-per-docs 300-byte LOCATION URL is a typed MalformedResponse. Two of the three bounds (DATAGRAM_CAP 4096, LOCATION_CAP 2048) are never the effective limit for LOCATION. Doc-code contradiction; realistic devices with long query-string locations would be dropped.
Confidence: HIGH — Verification: read

F-003 [S4]
Where: crates/device-discovery/src/ssdp.rs:123-127 vs 172-176
Code: (172-176)
```
        let value = value.trim();
        if value.len() > LOCATION_CAP {
            return Err(malformed(format!(
                "header {name} exceeds {LOCATION_CAP} bytes"
            )));
        }
```
What is wrong: the doc says "the malformed *optional* value degrades to `None`", but this per-line check runs BEFORE the header match, so an over-long value of an OPTIONAL or UNKNOWN header (e.g. a chatty `x-vendor:` line, or SERVER) rejects the entire datagram instead of degrading. With DATAGRAM_CAP 4096 a 2.5 KB vendor header is representable and would kill an otherwise-valid device response. Bounded, unrealistic, but the doc's degradation promise is not upheld for this malformation class.
Confidence: HIGH — Verification: read

F-004 [S4]
Where: crates/device-discovery/src/ssdp.rs:20-23
Code:
```
//! evidence on THIS sandbox (2026-11-10, recorded in the worklog):
//! ephemeral-socket send to `239.255.255.250:1900` accepted by the
//! kernel; bind + group join succeed; **zero SSDP responders exist in
```
What is wrong: "recorded in the worklog" — I could not find the 2026-11-10 SSDP sandbox evidence in worklog.md or docs/worklog/ (rg `239.255.255.250|M-SEARCH` → 0 hits in worklogs; only the crate + gate docs mention SSDP). The evidence string cites a record that is not where it says it is. Minor documentation-provenance gap, not a code defect; the equivalent live probe is reproducible (I ran it — it passes on this host).
Confidence: MEDIUM (the record may live in an archived log I did not enumerate) — Verification: UNVERIFIED (rg across worklog.md + docs/worklog/* found nothing)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| build_msearch | 93-102 | REAL | — | byte-exact UDA shape; continuation `\` strips indent (verified by test equality) |
| parse_search_response | 131-224 | REAL | S4 | F-002/F-003 tolerance gaps; case-insensitive, LF/CRLF, typed errors |
| SsdpSearchConfig::new / with_limits | 248-271 | REAL | — | clamps 1..=5 / 1..=256 / 200ms..=60s; tested |
| SsdpSearcher::new / config | 300-308 | REAL | — | |
| SsdpSearcher::probe | 317-345 | REAL | — | ephemeral bind, nothing sent; wasm → typed BlockedOnWeb |
| SsdpSearcher::search | 357-372 | REAL | — | wasm dispatch; native → search_native |
| search_native | 375-442 | REAL | — | bind/TTL/send/timeout, deadline loop, USN dedup, malformed counted; executed over real sockets |
| 7 unit tests | 473-583 | REAL | — | byte shape, parse polarity, clamps, spec constants |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| msearch_bytes_match_the_uda_shape | YES | exact string equality with the spec shape |
| spec_shaped_response_parses_every_field | YES | every field incl. max_age/server |
| parse_is_dirty_world_tolerant | YES | lowercase/LF/unknown headers; bad max-age → None |
| missing_required_headers_are_typed_errors | YES | three typed-required cases |
| bad_status_lines_and_encodings_are_typed_errors | YES | NOTIFY/500/non-UTF-8 |
| config_clamps_to_spec_plausible_bounds | YES | exact clamp values both directions |
| constants_are_the_spec_values | YES | group/port/TTL |
| (integration) loopback_roundtrip_collects_parses_and_dedups | YES | real wire bytes asserted via responder channel — not a mock echo |
| (integration) loopback_roundtrip_skips_malformed_and_keeps_going | YES | skipped_malformed == 2 and 1 good response |
| (integration) empty_wire_is_an_honest_empty_result | YES | Ok + zero (no error theater) |
| (integration) max_responses_stops_the_window_early | YES | elapsed < 1 s proves cap, not timeout |
| (integration) multicast_api_calls_succeed_or_fail_typed | WEAK | Ok and typed-Io are both accepted outcomes — cannot fail on this host class; honest about it (pins totality, not success) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Native feasibility: PROVEN FEASIBLE, std-only" | ssdp.rs:17-19 | REAL | I ran the loopback + multicast API tests (5/5) |
| "zero SSDP responders exist in this sandbox … end-to-end … `[VERIFY]` — not observable here, and not claimed" | ssdp.rs:22-25 | REAL (honest) | no claim overstates; no LAN test exists |
| "the socket searcher — whose transport logic is proven with real UDP round-trips" | ssdp.rs:29-32 | REAL | tests/ssdp.rs executed by me |
| "recorded in the worklog" | ssdp.rs:20 | UNVERIFIED | rg found no such worklog record (F-004) |
| "no command exposes it to the UI yet" | ui/src/screens/discovery/DiscoveryScreen.tsx:28-30 | REAL | rg: no app/command caller |

## 7. Dead / duplicated / unreachable code
Whole crate unreachable (F-001). No internal duplication. `Capability`/`DiscoveryError` plumbing shared with the crate's other lanes.

## 8. Dependencies & flags
std-only (UdpSocket, HashSet, Instant) + crate::capability/error. `target_family = "wasm"` gates the blocked lane — the release lane (native) compiles the full searcher. No features.

## 9. Open questions for the next agent
- Where is the 2026-11-10 sandbox evidence "recorded in the worklog"? (F-004)
- When a `devices.*` command lands, re-check F-002 (LOCATION > 256 bytes rejected) against real DLNA servers — MiniDLNA/ReadyMedia locations are short, but some stacks emit long URLs.
