# AUDIT: crates/streaming/src/client.rs
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-27 |
| Lines / functions | 998 / 15 (incl. 20 tests, 2 #[ignore] live) |
| File verdict | REAL — TLS posture verified: rustls always-on, no env override exists |
| Reachable from shipped entry point? | YES — main.rs→run::run→host.rs:904 open_source→demux_host.rs:491 SchemeRegistry::new()→schemes.rs:238 `UreqClient::new()`→fetch_inner |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | rg (env/TlsConfig/dependents) exit 0; sed -n verifications exit 0; `cargo test` IMPOSSIBLE (no cargo on PATH in sandbox) — baseline cited |

## 1. One-paragraph verdict
This is a real, hardened HTTP(S) client and the TLS posture question has a clean answer:
TLS exists, is rustls (ureq 3.5.0 pinned `default-features = false, features =
["rustls", "rustls-webpki-roots"]` — streaming/Cargo.toml), certificate verification is
always on, and there is NO insecure mode of any kind — no env var, no flag, no
`disable_verification` call anywhere in the crate. The one `TlsConfig` construction
site is `tls_config()` (client.rs:91-96) and it is wired into every agent. Wave 2's
SEC-07 "insecure-TLS env override" is confirmed PAPER-ONLY: the code has no override
to be insecure with. The adapter-owned redirect loop (cross-host policy, exact
followed counts, loop detection, unconditional https→http downgrade refusal,
Authorization dropped on hops) is genuine and tested against a real socket mock.
Weak spots: the downgrade gate is never exercised end-to-end on the real ureq
transport (mock parity + pure predicate only), the agent cache never evicts, and the
only true rustls-over-the-wire tests are `#[ignore]`d live tests.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt L5391-5734 (PACKAGE streaming
exit_code=0) incl. client::tests L5423-5443,5533 all `ok` (agent_config_pins_transport_safety,
redirect chains/budget/loops, cross-host allow/refuse, auth-drop, timeouts, budgets).
— verified by `rg "env" crates/streaming/src/ -g '*.rs'`: only `env!("CARGO_MANIFEST_DIR")`
compile-time macros in tests (mockhttp.rs:452 comment, schemes.rs:1051, dash/resolve.rs:749,
dash/fuzz.rs:94). Zero runtime env reads.
— verified by `rg "TlsConfig|disable_verification|dangerous|RootCerts" crates/streaming/src/*.rs`:
TLS surface exists only in client.rs. No disable path.
— verified by Cargo.lock L1214-1217 (ureq 3.5.0 exact) + L775-777 (rustls 0.23.46): rustls compiled in.
— attempted `cargo test -p streaming live_ -- --ignored`: IMPOSSIBLE — `cargo: No such file
or directory` in audit sandbox. Live rustls handshake UNVERIFIED (reason: no cargo/network).

## 3. Findings
F-001 [S3]
Where: crates/streaming/src/client.rs:436-438 + 682-688
Code:
```
            if is_scheme_downgrade(&url, &next) {
                return Err(NetworkError::CrossSchemeRedirect { hops: followed + 1 });
            }
```
What is wrong: the shipped transport's security gate is not covered by any executed
test on the real client. `https_to_http_redirect_downgrade_is_refused_not_followed`
(L689) runs `MockClient::new().fetch(...)` — the mock's own parity copy of the loop —
because the plaintext testkit cannot front ureq with TLS; `scheme_downgrade_gate_pins_transport_safety`
(L733) tests only the pure predicate. Deleting the call at L436 would leave every
default-suite test green. Why it matters: the MITM-downgrade defense on the path
users actually run is protected by convention, not by a test. What correct looks
like: a rustls server fixture (rcgen-style local TLS) so the ureq adapter itself is
driven through a downgrade hop. The file honestly documents the gap (L682-687).
Confidence: HIGH — Verification: read + baseline test list.
F-002 [S3]
Where: crates/streaming/src/client.rs:354-364
Code:
```
    fn shared_agent(timeouts: TimeoutConfig) -> Arc<Agent> {
        static CACHE: OnceLock<Mutex<HashMap<TimeoutConfig, Arc<Agent>>>> = OnceLock::new();
```
What is wrong: process-wide cache keyed by the 3-Duration `TimeoutConfig`, never
evicted; every distinct deadline triple mints an `Agent` with its own connection
pool, held forever. Why it matters: any future caller deriving per-request timeouts
from user input turns this into unbounded memory + socket growth (MP §12 is this
crate's own religion). Today only `RangedConfig::default()` feeds it (one agent), so
the blast radius is latent. What correct looks like: bounded cache (e.g. small LRU)
or document the invariant "timeouts are a fixed config vocabulary".
Confidence: HIGH — Verification: read (reachability of dynamic TimeoutConfig not found in CLI).
F-003 [S4]
Where: crates/streaming/src/client.rs:330
Code: `        reason: status.canonical_reason().unwrap_or_default().to_string(),`
What is wrong: reason phrase is fabricated from the status code, not the wire value
— a 200 with reason "NOPE" reports "OK". Documented divergence (module docs L60-62),
mock parity preserved. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/streaming/src/client.rs:149-152
Code:
```
        ureq::Error::TooManyRedirects => NetworkError::TooManyRedirects {
            limit: redirect_limit,
            followed: redirect_limit,
        },
```
What is wrong: fabricates `followed = limit` if ever reached; unreachable under this
adapter's config (`max_redirects(0)`), kept as defense-in-depth and labeled. If a
config regression made it reachable, the count would be a lie. Confidence: HIGH —
Verification: read.
F-005 [S4]
Where: crates/streaming/src/client.rs:288-292
Code:
```
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok());
```
What is wrong: malformed Content-Length silently degrades to `None`, skipping the
pre-allocation refusal — a hostile server sending `Content-Length: 99999999999999999999`
(unparseable, > u64) or garbage avoids the early typed error and forces the
read-to-limit+1 path instead. Memory stays bounded (take(limit+1)); only the error
quality and the fast-refusal are lost. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| tls_config | 91-96 | REAL | — | Rustls + WebPki roots; never disables verification; no env input |
| agent_config | 111-119 | REAL | — | max_redirects(0), status-as-error off, per-call timeout |
| map_io_error | 123-133 | REAL | — | EAGAIN→ReadTimeout for mid-body socket timeouts |
| map_ureq_error | 143-197 | REAL | F-004 | non_exhaustive fallback keeps Display; 416 special-cased |
| is_redirect | 202-204 | REAL | — | 301/302/303/307/308; 300/304 surface (mock parity) |
| is_scheme_downgrade | 226-228 | REAL | — | pure; reused by auth.rs + remote_playlist.rs |
| classify | 234-245 | REAL | — | 2xx Ok, 416 typed, 4xx/5xx class flags, else surface |
| single_hop | 253-334 | REAL | F-005 | CL pre-check + take(limit+1) + truncation typed; headers wire-order |
| UreqClient::new / shared_agent | 347-364 | REAL | F-002 | pool per TimeoutConfig |
| fetch / fetch_with_challenge | 368-377 | REAL | — | 401-surfacing for auth layer |
| fetch_inner | 383-461 | REAL | F-001 | typed URL parse, visited-set loop detection, exact counts, gates in right order |
| 20 tests | 464-997 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ok_200_roundtrip / range_206_partial_is_honored | YES | real socket, wire form asserted incl. logged Range header |
| status_416 / status_class_flags_are_mapped | YES | variant + code() + class flags asserted |
| redirect_chain_of_3 / too_many_redirects (budget 2) | YES | exact hop paths and wire request counts asserted |
| cross_host refused/allowed | YES | target server contact asserted (empty vs 1) |
| https_to_http downgrade refused | PARTIAL | uses MockClient (parity copy), not UreqClient — see F-001 |
| scheme_downgrade_gate | YES (predicate only) | pure fn; does not cover the call site |
| redirect loops (self + cross-host) | YES | typed variant + hops + request counts |
| manual_mode_surfaces_the_redirect_response | YES | 302 + Location surfaced, 1 request only |
| authorization_is_dropped_on_followed_hops | YES | second hop's logged headers must lack it |
| read_timeout_on_delayed_response | YES | 400ms delay vs 150ms budget, real socket |
| garbage_first_line / response_over_budget / unsupported_scheme | YES | typed variants |
| agent_config_pins_transport_safety | YES | asserts !disable_verification + max_redirects()==0 |
| shared_agent_is_cached_per_deadline_config | YES | Arc::ptr_eq same/different |
| live_example_com / live_httpbin (#[ignore]) | NOT RUN | live network; the only tests that would prove rustls handshake on the wire |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "rustls TLS with verification always on" | client.rs:7-8 | REAL | tls_config:91-96; only TlsConfig site (rg); pinning test baseline L5423 ok |
| "never touches ureq's disable_verification" | client.rs:85-87 | REAL | rg disable_verification: doc-mention only, no call |
| "ureq 3.5.0 ... rustls ... webpki-roots, NO native-tls" | streaming/Cargo.toml ureq dep | REAL | Cargo.toml features + Cargo.lock L1214/L775 |
| SEC-07 insecure-TLS env override | (Wave 2 finding, docs elsewhere) | PAPER-ONLY | zero runtime env reads in streaming/src (rg) |
| "exact followed count ... ureq exposes no redirects() accessor" | client.rs:26-30 | REAL | adapter issues hops itself, counts itself (L422-428, test asserts 3 wire requests) |
| "https→http ... refused unconditionally" | client.rs:66-69, 211-215 | REAL-UNPROVEN | code present at L436; e2e on real transport untested (F-001) |

## 7. Dead / duplicated / unreachable code
No dead code. Known duplication (documented as deliberate parity copies): the mock
client's fetch loop mirrors fetch_inner; auth.rs::AuthFetch::follow and
remote_playlist.rs re-implement manual redirect loops but reuse is_scheme_downgrade —
three redirect loops in one crate is a drift risk the crate accepts consciously.

## 8. Dependencies & flags
ureq =3.5.0 exact pin, default-features=false, features=["rustls",
"rustls-webpki-roots"] (ring 0.17 crypto provider, Lead-ADR'd in Cargo.toml comment).
No cargo features in streaming gate client.rs. std-only otherwise. No unsafe.

## 9. Open questions for the next agent
- Does any shipped surface expose user-tunable timeouts that would reach
  shared_agent with dynamic values (F-002 blast radius)? Check settings-schema →
  streaming config wiring.
- Add a local-TLS (rcgen) testkit so the ureq adapter's downgrade gate is executed,
  not just its predicate.
