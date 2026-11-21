# AUDIT: crates/streaming/src/client.rs#part2 (lines 500-998)
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-21 |
| Chunk boundary | part2 = lines 500-998, per wave-00 assignments.tsv LOC split (part1=499 → A01: 1-499 = module docs, `tls_config`, `agent_config`, error maps, `is_redirect`, `is_scheme_downgrade`, `classify`, `single_hop`, `UreqClient`, `shared_agent`, `impl HttpFetch`, `fetch_inner`, test helpers `spawn`/`fast_timeouts`/`follow_policy`, and the setup of the first test). My chunk = the ENTIRE test body of the module: the asserts of `ok_200_roundtrip` (its setup 494-499 is A01's) and 21 further tests through `live_httpbin_redirect_chain_lands_on_200` (ends 997; 998 is the closing brace). No production code in range. |
| Lines / functions | 499 / 22 test functions (1 split across the boundary + 21 fully in-range) |
| File verdict | REAL (the tests exercise the REAL production transport — `UreqClient` over loopback sockets — and are discriminating; the TLS claim they cannot cover is REAL-UNPROVEN, see F-001) |
| Reachable from shipped entry point? | YES for the code under test: `UreqClient` is the production transport (`SchemeRegistry::new()` → schemes.rs:238 `Arc::new(UreqClient::new())` ← demux_host.rs:491 `open_http_source` ← `lumen open http(s)://…`). Tests executed: 20 ok + 2 ignored in baseline `orchrun-test-workspace-nofailfast.log.txt:4965-4985,5075`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `sed -n '77,125p' crates/streaming/src/client.rs` (tls_config/agent_config, part1 context) · `rg -n "is_scheme_downgrade" crates/` (production gate shared by auth.rs:624 + remote_playlist.rs:150 — but NOT by mockhttp) · `sed -n '935,975p' crates/streaming/src/mockhttp.rs` (mock re-derives the check inline) · `head -80 crates/streaming/tests/https_activation.rs` (the suite's own "cannot prove in-tree: a real TLS handshake" admission) · baseline `orchrun-test-workspace-nofailfast.log.txt:4965-4985,5075` (client tests), `:5220-5227` (https_activation 2/2 ok) |

## 1. One-paragraph verdict
These are the tests for the one piece of network code the shipped binary actually runs, and they are good at what they cover: exact wire-form assertions (`bytes=2-5` on the server's request log), exact error payloads (`TooManyRedirects { limit: 2, followed: 2 }` with the wire-request count to match), negative-space proofs (a refused cross-host or downgraded target's request log must stay EMPTY), and the credential-stripping pin that a followed hop never re-sends `Authorization`. Twenty of them ran green against the real ureq client over loopback sockets in the baseline run. Two holes matter: the entire TLS story — the wave's headline question — rests on two `#[ignore]`d live-network tests that the baseline records as `ignored`, so zero TLS handshakes exist in the recorded evidence and the claim is config-pins plus an out-of-band admission note; and the https→http downgrade wire test runs on the mock client's hand-copied loop, which does not even share the production gate function, so the production refusal branch has no in-tree execution path at all.

## 2. What actually works (verified)
— 20/20 executed `client::tests` green in baseline `orchrun-test-workspace-nofailfast.log.txt:4965-4985,5075` (the 2 live tests `ignored`, :4971-4972) + `https_activation.rs` 2/2 ok (`:5220-5227`) — all against the REAL `UreqClient` (not the mock client) over real loopback TCP, except the two tests that intentionally target the mock (see F-002).
— Hand-traced the redirect-loop expectations against `fetch_inner` (part1, 383-462): 5-hop chain with budget 2 → `TooManyRedirects { limit: 2, followed: 2 }` with exactly 3 wire requests (test 608-617 ✓); self-loop → `RedirectLoop { hops: 1 }` (visited-set size 1, one request; test 801-805 ✓); A→B(self-loop) → `RedirectLoop { hops: 2 }` with one request per server (test 781-787 ✓); manual mode → 302 + `Location: /r1` surfaced, one request (test 818-823 ✓).
— The credential pin (830-849): first logged request carries `Authorization: Bearer secret-token`, the followed hop's log entry has none — `drop_authorization = true` (fetch_inner:458) verified on the wire.
— `agent_config_pins_transport_safety` (923-939) pins the three config invariants: verification on, `max_redirects(0)` (ureq never follows — the adapter owns hops), `http_status_as_error(false)`.
— `shared_agent_is_cached_per_deadline_config` (944-956): `Arc::ptr_eq` both directions — same agent per deadline config, distinct agents for distinct deadlines.

## 3. Findings

### F-001 [S3]
Where: crates/streaming/src/client.rs:958-997 (both live tests), with crates/streaming/tests/https_activation.rs:21-26
Code:
```
    /// Measures the admission's real-TLS claim: rustls + webpki-roots
    /// against a public HTTPS endpoint, expecting 200.
    #[test]
    #[ignore = "live network required: run with --ignored"]
    fn live_example_com_returns_200() {
```
What is wrong: the only tests in the workspace that execute a TLS handshake through `UreqClient` are these two `#[ignore]`d live-network tests. The baseline records both as `ignored` (`orchrun-test-workspace-nofailfast.log.txt:4971-4972`) — the recorded evidence contains ZERO TLS handshakes. Every executed client test rides the plaintext mock loopback; `https_activation.rs` states it outright (lines 21-26): "What it cannot prove in-tree: a real TLS handshake … the rustls (ring 0.17.14) round-trip is [VERIFY: needs a networked CI lane]." The TLS *configuration* is pinned (`agent_config_pins_transport_safety`: `!tls.disable_verification()`), but no run proves the configured stack performs a handshake, loads the webpki-roots bundle, or verifies a real certificate chain.
Why it matters: the shipped `lumen` binary's `https` lane is real and reachable (`lumen open https://…` → ureq/rustls). If the TLS stack regresses (a webpki-roots version bump, a ring build failure on a platform, a ureq API change silently skipping the tls_config), every https source fails at runtime and NO gate in the repository catches it — the two tests that would are skipped by design and no networked CI lane exists.
What correct looks like: a scheduled networked CI lane running `cargo test -p streaming live_ -- --ignored` (the run command is already documented in the test), or an in-tree local TLS proof (a rustls test server with a self-signed cert added to a custom root store — no network needed).
Confidence: HIGH — Verification: read + baseline log lines 4971-4972 (ignored) + https_activation.rs's own admission.

### F-002 [S3]
Where: crates/streaming/src/client.rs:689-723 (test on MockClient) and 725-731 (its justification comment), vs crates/streaming/src/mockhttp.rs:948-953
Code:
```
        let error = assert_err(
            MockClient::new().fetch(request),
            "an https-to-http redirect must be refused, never followed",
        );
```
```
                if url.scheme == "https" && next.scheme == "http" {
                    return Err(NetworkError::CrossSchemeRedirect { hops: followed + 1 });
```
What is wrong: the only WIRE-level test of the https→http downgrade refusal runs on `MockClient`, whose fetch loop re-derives the downgrade check inline (mockhttp.rs:950-951) instead of calling the production gate `is_scheme_downgrade` (client.rs:226) — a gate that IS shared by the other two manual loops (auth.rs:624, remote_playlist.rs:150) but NOT by the mock. The test's comment (682-687) justifies this as "the testkit client, whose fetch loop is the documented parity copy of this adapter's" — but the parity is by convention, and it has already drifted structurally (inline check; `visited` is a `Vec` with `push` in the mock vs a `HashSet` in the adapter). Consequence: weakening or removing the production gate call at fetch_inner:436 would NOT fail this wire test; only the pure-function test `scheme_downgrade_gate_pins_transport_safety` (733-759) would notice. Worse, the production branch `return Err(NetworkError::CrossSchemeRedirect { .. })` at client.rs:437 is UNREACHABLE in-tree: no test can complete an https request against the plaintext mock to obtain a redirect FROM it, so the adapter's own refusal statement has zero execution coverage anywhere.
Why it matters: this is the security review 6.H1 F1 MITM defense; its only end-to-end proof lives in a hand-copied loop that shares no code with the shipped path.
What correct looks like: the mock's loop calls `crate::client::is_scheme_downgrade` (deleting the inline copy), and the wire test's comment then tells the truth — same gate function, different loop — or an in-tree TLS-capable loopback (self-signed rustls server) exercises the adapter's own branch.
Confidence: HIGH — Verification: read + rg (`is_scheme_downgrade` callers: client.rs self, auth.rs:624, remote_playlist.rs:150 — mockhttp absent).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ok_200_roundtrip` (asserts; setup 494-499 is part1) | 500-504 | REAL | — | status/body/Content-Length; line 503 `server.requests();` is a documented keep-alive no-op, not a missing assert. |
| `range_206_partial_is_honored` | 508-527 | REAL | — | pins wire form `bytes=2-5` (server log) + `Content-Range: bytes 2-5/10` echo + sliced body. |
| `status_416_maps_to_range_not_satisfiable` | 530-546 | REAL | — | variant + stable code string. |
| `status_class_flags_are_mapped` | 550-576 | REAL | — | 404/503 with both class flags, `status_code()`, `!is_timeout()`. |
| `redirect_chain_of_3_lands_on_final_body` | 580-593 | REAL | — | hop paths `["/start","/r1","/r2","/r3"]` — proves the ADAPTER followed. |
| `too_many_redirects_when_bound_exceeded` | 598-618 | REAL | — | `limit: 2, followed: 2` exact + 3 wire requests. |
| `cross_host_redirect_refused_when_policy_forbids` | 623-655 | REAL | — | 127.0.0.1→localhost host-string change; target's log must be EMPTY. |
| `cross_host_redirect_followed_when_policy_allows` | 659-675 | REAL | — | same topology, permissive policy, both logs == 1. |
| `https_to_http_redirect_downgrade_is_refused_not_followed` | 688-723 | REAL (on the mock) | S3 (F-002) | MockClient; origin once, target EMPTY; `!is_retryable()` pinned. |
| `scheme_downgrade_gate_pins_transport_safety` | 732-759 | REAL | — | pure gate: 2 downgrade shapes true, upgrade/same-scheme/plain-http-start false. |
| `cross_host_chain_into_loop_is_detected_as_redirect_loop` | 767-788 | REAL | — | `hops: 2`, one request per server. |
| `self_loop_is_detected_as_redirect_loop` | 792-807 | REAL | — | `hops: 1`, one request. |
| `manual_mode_surfaces_the_redirect_response` | 811-824 | REAL | — | 302 + Location surfaced, no follow. |
| `authorization_is_dropped_on_followed_hops` | 829-849 | REAL | — | credential present on hop 1, absent on hop 2 (wire log). |
| `read_timeout_on_delayed_response` | 853-869 | REAL | — | 150 ms read budget vs 400 ms delay, `is_timeout()`. |
| `garbage_first_line_maps_to_protocol_violation` | 872-883 | REAL | — | typed framing violation. |
| `response_over_budget_is_rejected` | 887-901 | REAL | — | `limit: 10, actual: 100` — enforced against advertised Content-Length before allocation. |
| `unsupported_scheme_is_typed` | 905-915 | REAL | — | `ftp` → `UnsupportedScheme` before transport. |
| `agent_config_pins_transport_safety` | 922-939 | REAL | — | config-level pins only — no handshake (F-001). |
| `shared_agent_is_cached_per_deadline_config` | 943-956 | REAL | — | `Arc::ptr_eq` both directions. |
| `live_example_com_returns_200` | 968-984 | REAL-UNPROVEN | S3 (F-001) | `#[ignore]`d; baseline `ignored`; "Measured live 2026-10-30" comment unverifiable from evidence. |
| `live_httpbin_redirect_chain_lands_on_200` | 989-997 | REAL-UNPROVEN | S3 (F-001) | `#[ignore]`d; baseline `ignored`. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ok_200_roundtrip / range_206 / status_416 / status_class_flags | YES | exact statuses, bodies, headers, error payloads, code strings. |
| redirect_chain_of_3 / too_many_redirects | YES | exact hop-path vectors and exact `followed` counts cross-checked against the server log. |
| cross_host refused/followed pair | YES | the refused target's request log must be empty — a follow would leave evidence. |
| https_to_http downgrade (wire) | YES — but against the MOCK's copy | the production gate is only covered by the pure test (F-002). |
| scheme_downgrade_gate (pure) | YES | boolean table over all gate shapes. |
| loop tests (self, cross-host chain) | YES | hops counts + per-server request counts. |
| manual_mode / authorization_dropped | YES | surfaced Location, no follow; credential present-then-absent on the wire. |
| read_timeout / garbage_first_line / over_budget / unsupported_scheme | YES | typed variants with payload fields. |
| agent_config_pins | YES (for the config) | config accessors directly asserted. |
| shared_agent_cache | YES | pointer identity both ways. |
| live_example_com / live_httpbin | NEVER RUNS by default | `#[ignore]` — the TLS claim has no default-suite coverage (F-001). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "rustls TLS with verification always on" (transport) | client.rs:4-7 module doc (part1), ADR-011 | REAL-UNPROVEN | config pinned (test 923-939); zero handshakes in evidence — F-001. |
| "an https hop never follows a redirect down to http … whatever the host policy says" | client.rs:431-435 | REAL (gate) / UNTESTED (adapter branch) | pure gate tested 733-759; adapter branch client.rs:436-438 has no in-tree execution path — F-002. |
| "RedirectAuthHeaders::Never parity" (credentials never re-sent) | client.rs:36 (part1 doc) | REAL | test 829-849, wire-level. |
| "the testkit client, whose fetch loop is the documented parity copy of this adapter's" | client.rs:685-687 | SHALLOW | parity by convention; already structurally drifted (F-002). |
| network-http-client / network-tls-policy matrix rows | docs/feature-matrix.toml:1405+, 1418+ | (A01/matrix auditors) | my evidence feeds theirs: executed proof is plaintext-only. |

## 7. Dead / duplicated / unreachable code
No dead code in the chunk. Cross-file duplication flagged: mockhttp.rs:950-951 duplicates `is_scheme_downgrade` inline (F-002) while auth.rs and remote_playlist.rs correctly reuse the shared gate.

## 8. Dependencies & flags
Tests use `mockhttp` (MockClient/MockServer/MockConfig), `testkit::assert_err`, real `std::time` durations. The production code under test (part1) links ureq 3.5.0 + rustls + webpki-roots (tls_config:91-96). No feature gates; no unsafe. 20 ok / 2 ignored in baseline.

## 9. Open questions for the next agent
- A01 (part1): `single_hop`'s body read path — does it enforce `response_limit` against a missing/lying Content-Length the way `read_bounded_body`'s callers do (mockhttp's leniency, my mockhttp#part3 F-004)? Also whether `shared_agent`'s cache is bounded.
- Orchestrator: REQUEST-ORCH-RUN candidate (needs network): `cargo test -p streaming live_ -- --ignored --nocapture` — the single run that would convert the TLS claim from REAL-UNPROVEN to REAL (or refute it).
- A05/A08 (mockhttp parts): the inline downgrade check at 950-951 should call `crate::client::is_scheme_downgrade` — one-line fix deleting the parity copy.
