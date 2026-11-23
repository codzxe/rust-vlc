# AUDIT: crates/streaming/tests/https_activation.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-02-14 |
| Lines / functions | 119 / 2 tests |
| File verdict | REAL (routing/composition proof) with an honestly-documented hole: NO real TLS is tested |
| Reachable from shipped entry point? | The behavior under test is: `SchemeRegistry::classify` + `open_http` — YES, this is the shipped CLI https open lane (demux_host.rs:491) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p streaming --test https_activation` → 2 passed/0 failed (own run, 0.14s); rg verification of cross-referenced artifacts |

## 1. One-paragraph verdict
Direct answer to the hostile question: this suite does NOT test real TLS. There is no local TLS server, no certificate handling, and no skip-when-no-certs logic — the TLS handshake is simply never exercised. The transport is the crate's plaintext loopback `MockServer`/`MockClient`, and the test fabricates an `https://` source by string-replacing the mock's `http://` URL; the mock client is scheme-agnostic, so the https URL rides the classify → `open_http` → probe → ranged-window → `SourceStream` pipeline and the test asserts a byte-identical 65,535-byte body over ≥30 real range requests. What it proves is real and I ran it green; what it cannot prove — the rustls/ring handshake — is stated in the module doc itself ("[VERIFY: needs a networked CI lane]"), pinned only at the config layer by `agent_config_pins_transport_safety` (verified to exist at client.rs:923 and to assert verification-on / zero redirects), and backed by a one-time out-of-band measurement recorded in `docs/dependencies/records/ureq-3.5.0.md` R10 (verified: "live fetch of `https://example.com/` returned 200"). The matrix row's notes are equally honest. The residual defect: ADR-011 "activation" has no repeatable in-tree handshake proof, so a regression in the TLS adapter wiring would pass CI — a coverage gap, not a lie.

## 2. What actually works (verified)
— `cargo test -p streaming --test https_activation` → `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s` (own run). Test 1: 3 https vectors classify to `AccessKind::Http` with scheme preserved, default port 443 explicit, empty authority refused Malformed. Test 2: full composition with real ranged requests (≥30) and `got == body` byte equality.
— Cross-references verified: `agent_config_pins_transport_safety` (client.rs:923) asserts `!tls.disable_verification()`, `max_redirects()==0`, `!http_status_as_error()`; `HttpsNotAdmitted`/`HTTPS_ADR011_ADMITTED` exist only in comments documenting their deletion (rg: 5 hits, all comment/doc lines).

## 3. Findings
F-001 [S3]
Where: crates/streaming/tests/https_activation.rs:19-27 (and 74-119)
Code:
```
//! What it cannot prove in-tree: a real TLS handshake. The mock
//! testkit is a plaintext socket by design and this sandbox has no
//! network egress, so the rustls (ring 0.17.14) round-trip is
//! [VERIFY: needs a networked CI lane]. The transport's TLS *config*
//! is pinned separately (always-on verification, webpki roots,
```
What is wrong: not fraud — the limitation is documented here AND in the feature-matrix notes ("NOT yet proven in-tree: a real TLS handshake round-trip … [VERIFY: needs a networked CI lane]", docs/feature-matrix.toml:1428). But the gap is real: the only in-tree TLS assertions are config-shaped (flags on a `TlsConfig`), and the sole handshake evidence is a non-repeatable, out-of-band manual fetch recorded in a dependency-review document. A broken rustls provider registration, SNI regression, or certificate-verification bug in the adapter would not fail any CI test. Why it matters: https is the shipped remote-open lane; its security-critical path has zero automated coverage. What correct looks like: a local TLS server (e.g., a self-signed rustls-server `TcpListener` + `rustls::ServerConfig` in the testkit, client pinning the test CA) — no network egress needed; the "no egress" rationale covers remote hosts, not loopback TLS. Confidence: HIGH — Verification: read + ran; no rustls server exists anywhere in the testkit (rg over crates/streaming/src/mockhttp.rs shows a plaintext TcpServer).
Severity S3 (coverage gap, honestly labeled) rather than S0 because no doc claims in-tree handshake proof.

F-002 [S4]
Where: crates/streaming/tests/https_activation.rs:84
Code:
```
    let https_source = server.url("media.bin").replacen("http://", "https://", 1);
```
What is wrong: the https-ness of the test source is a string substitution on the mock's http URL, and `MockClient` is scheme-blind, so the composition test would pass even if the production ureq transport refused every https URL — it pins the dispatch layer only, never the transport. The comment at L81-83 says exactly this, so it is an honest limitation, but readers of the test name ("https_source_rides_the_open_http_composition") may over-read it. Confidence: HIGH — Verification: read; MockClient::fetch_inner parses and routes without scheme checks (mockhttp.rs:893-925).

## 4. Per-function table (test file; <130 LOC — abbreviated)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| https_routes_to_the_http_access_strategy | 41-66 | REAL | — | would fail if classify grew a scheme-specific refusal; port/path assertions |
| https_source_rides_the_open_http_composition | 74-119 | REAL | S4 | byte-exact body + ≥30 range requests; F-002 scheme-blind transport caveat |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| https_routes_to_the_http_access_strategy | YES | classify behavior + URL grammar + default port pinned |
| https_source_rides_the_open_http_composition | YES for the dispatch/ranged layers (would fail if open_http dropped windows or corrupted bytes); NO for the TLS layer (mock is scheme-blind — F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the pre-ratification HttpsNotAdmitted refusal vocabulary no longer exists" | https_activation.rs:9-12 | REAL | rg: only comment mentions remain |
| "a real HTTPS round-trip was measured out-of-band" | https_activation.rs:26-27 | REAL-UNPROVEN | record exists (ureq-3.5.0.md R10, "live fetch … returned 200") but is not repeatable from the tree |
| matrix: "NOT yet proven in-tree: a real TLS handshake round-trip" | docs/feature-matrix.toml:1428 | REAL (honest) | matches F-001 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Test-only: streaming (schemes/ranged/mockhttp/fetch), plugin_api (SourceStream), std. No feature gates; no unsafe; no network egress required (loopback only).

## 9. Open questions for the next agent
- The loopback-TLS testkit server proposed in F-001 would also let `net_interrupted.rs`/`auth_e2e.rs` run over real TLS — worth one card.
- Is there any CI lane with egress (GitHub Actions runners have it)? The "[VERIFY: needs a networked CI lane]" marker implies the lane doesn't exist yet — confirm in .github/workflows.
