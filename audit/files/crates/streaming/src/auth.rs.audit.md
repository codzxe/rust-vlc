# AUDIT: crates/streaming/src/auth.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2025-06-14 |
| Lines / functions | 1562 / 29 prod fns + 11 tests + 4 test doubles |
| File verdict | UNREACHABLE (real, tested code — zero production callers; see F-001) |
| Reachable from shipped entry point? | NO — `AuthFetch`/`Credentials`/`InMemoryCredentialStore` are constructed only by `crates/streaming/tests/auth_e2e.rs`, `tests/auth_scheme_gate.rs`, and auth.rs's own `#[cfg(test)]` mod; grep over crates/+apps/ finds no other constructor. `remote_playlist.rs:104-106` only *invites* callers to compose it. Nothing in lumen-cli/desktop/engine does. |
| Findings | S0: 0 S1: 0 S2: 2 S3: 3 S4: 3 |
| Commands run | `./target/debug/deps/auth_e2e-0123edb7a892f2b9` -> 8 passed, exit 0; `./target/debug/deps/auth_scheme_gate-9a35f0a37f5fcae5` -> 2 passed, exit 0; `./target/debug/deps/streaming-c6cf1f9f1d2bf8f0 auth::` -> 11 passed, exit 0; grep reachability sweeps (see F-001) |

## 1. One-paragraph verdict
This is genuine, carefully written code: clean-room MD5 pinned to RFC 1321 vectors, base64 pinned to RFC 4648, digest pinned to the RFC 2617 §3.5 Mufasa vector, quoted-string escaping with control-byte typed refusals, challenge-first flow with same-origin/downgrade/cross-host credential gates, redacted Debug for every credential-carrying type — and I executed all 21 tests (unit + e2e over real local TCP sockets via mockhttp's `TcpListener`), all green. But it is a library island: no shipped entry point ever constructs `AuthFetch`, so HTTP authentication does not exist in the product. The one real security defect inside the module is that the redirect trust boundary compares **host only** — port (and any scheme other than https→http downgrades) is ignored, so credentials keyed by the store to `scheme://host:port` can be re-attached to a *different port* on the same host, contradicting the module's own origin-keying discipline.

## 2. What actually works (verified)
— `./target/debug/deps/streaming-c6cf1f9f1d2bf8f0 auth::` -> "test result: ok. 11 passed" (MD5/base64/digest vectors, challenge parse, redaction, store origin-keying, downgrade + cross-host credential gates, escaping).
— `./target/debug/deps/auth_e2e-0123edb7a892f2b9` -> "test result: ok. 8 passed" (Basic/Digest round-trips, wrong-password typed failures, store-fed answer, userinfo accept/strip, unchallenged server never sees credentials) — over real 127.0.0.1 TCP sockets (mockhttp.rs:32,454).
— `./target/debug/deps/auth_scheme_gate-9a35f0a37f5fcae5` -> 2 passed (downgrade refused; same-scheme + upgrade redirects still follow).
Baseline cross-check: audit/runs/baseline/cargo-test-workspace.txt:5402-5412,5619-5633 (same tests, ok).

## 3. Findings
F-001 [S2]
Where: crates/streaming/src/auth.rs:463-470 (and repo-wide)
Code: `pub struct AuthFetch {\n    inner: Arc<dyn HttpFetch>,\n    credentials: Option<Credentials>,\n    store: Option<Arc<dyn CredentialStore>>,`
What is wrong: 1562 lines of security-critical auth code with zero production consumers. Constructors found only in: auth.rs tests, tests/auth_e2e.rs, tests/auth_scheme_gate.rs. `remote_playlist.rs:104-106` says "Compose an [`AuthFetch`](crate::auth::AuthFetch) as the `fetch` argument for authenticated sources" — no caller does; `open_remote_playlist` itself also has no production callers (grep: only remote_playlist.rs's own tests). Why it matters: the product cannot authenticate to any HTTP source today; `http://user:pass@host/stream` from a user reaches the shipped open path without this layer. The feature-matrix row is honest (`docs/feature-matrix.toml:1469-1476` `network-auth` status `not_started`, evidence `""`), so this is not a false claim — it is a major unwired surface. What correct looks like: wire AuthFetch into the shipped fetch composition point (the engine/CLI remote-open path) or mark the module experimental. Confidence: HIGH — Verification: ran (grep sweeps over crates/ + apps/, no match outside tests).

F-002 [S2]
Where: crates/streaming/src/auth.rs:591-597, 680-682
Code: `if let Some((value, allowed_host)) = &authorization\n                && url.host.eq_ignore_ascii_case(allowed_host)\n            {\n                hop_request\n                    .headers\n                    .push(("Authorization".to_string(), value.clone()));\n            }` (and) `if !final_url.host.eq_ignore_ascii_case(&original_url.host) {\n                    return Err(NetworkError::http_status(401));\n                }`
What is wrong: the credential re-attachment and challenge-answer gates compare **host only**. Port is ignored (and scheme only via the https→http downgrade refusal at :624). A redirect from `http://host:8080` to `http://host:9090` (same host, different port) carries the Authorization header to :9090, and a challenge answered at :9090 is fed credentials looked up for :8080. This contradicts the module's own store discipline — `origin_key` (auth.rs:127-129) is `scheme://host:port`, and `store_lookup_is_origin_keyed` (:1132-1135) explicitly asserts different-port lookups return `None` ("never a cross-site credential leak"). Different port = different service/process on one machine; credentials scoped per-origin leak across it. Why it matters: the store's port-sensitivity is bypassed by the redirect path it feeds. No test covers a same-host port-changing redirect (mutation: adding one would fail today). What correct looks like: compare `(scheme, host, port)` — the same origin tuple the store keys on. Confidence: HIGH — Verification: read + traced both call sites; test gap confirmed by grep over auth tests.
Refuting check (done): `is_scheme_downgrade` (client.rs:226) covers only https→http; http→https same-host is an upgrade (safe); the port dimension is unguarded on every path.

F-003 [S3]
Where: crates/streaming/src/auth.rs:507-514
Code: `fn next_cnonce(&self, nonce: &str, uri: &str) -> String {\n        let nanos = SystemTime::now()\n            .duration_since(UNIX_EPOCH)\n            .map(|d| d.as_nanos())\n            .unwrap_or(0);`
What is wrong: the client nonce is MD5 over (server nonce, URI, wall clock, counter) — fully predictable to anyone who knows the clock and can read the response timing; RFC 7616 requires the cnonce to be unpredictable to defeat chosen-nonce/precomputation attacks. The code admits it (`entropy adequacy [VERIFY]`, :506) but ships it anyway. Why it matters: with MD5 digest already [PROPOSAL]-weak, a predictable cnonce removes the one mitigation against MITM chosen-nonce dictionary work. What correct looks like: mix in process-unique entropy (address of a heap allocation, thread id, /dev/urandom where available) — or land the keychain card's RNG decision. Confidence: HIGH — Verification: read.

F-004 [S3]
Where: crates/streaming/src/auth.rs:217-219
Code: `pub(crate) fn parse_www_authenticate(value: &str) -> Option<Challenge> {\n    let (scheme, rest) = value.trim().split_once(' ')?;`
What is wrong: only the first scheme token of the header is considered. A server that lists challenges in one header — legal per RFC 9110 §11.6.1, common shape `Negotiate, Basic realm="x"` — parses scheme as `"Negotiate,"` → unknown → `None` → the wrapper refuses with ProtocolViolation even though a supported Basic challenge is present. Also `"Basic"` with no space (realm omitted, legal in RFC 7617) → `split_once(' ')` returns `None` → refusal. Why it matters: realistic Windows/IIS-style servers send combined challenges; auth fails entirely instead of using the supported scheme. What correct looks like: iterate scheme candidates across comma boundaries. Confidence: HIGH — Verification: read (no test exercises multi-scheme headers).

F-005 [S3]
Where: crates/streaming/src/auth.rs:576-581
Code: `headers: template\n                    .headers\n                    .iter()\n                    .filter(|(name, _)| !name.eq_ignore_ascii_case("authorization"))\n                    .cloned()\n                    .collect(),`
What is wrong: any caller-supplied `Authorization` header is silently stripped on every hop (the probe and the retry only ever carry the wrapper's own computed value). `HttpRequest::with_header` (fetch.rs:408) accepts the `Authorization` name, so a caller with a Bearer/static token gets silent credential loss with no typed error. Note: there is no Bearer support anywhere in this module — the feature-matrix claim is "basic/digest" only (feature-matrix.toml:1472), which matches the code (no doc fraud on that point). What correct looks like: pass the caller's header through on same-origin hops, or refuse at construction with a typed error. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/streaming/src/auth.rs:700-707
Code: `let authorization = digest_authorization(\n                            &credentials,\n                            &challenge,\n                            "GET",`
What is wrong: digest method is the literal `"GET"`, not `template.method.as_str()`. Harmless today (`HttpMethod` has exactly one variant, fetch.rs:37-40) but a latent break the day HEAD lands — the digest would hash the wrong method and every Digest auth would fail. What correct looks like: pass `request.method.as_str()`. Confidence: HIGH — Verification: read + fetch.rs:33-49 cross-checked.

F-007 [S4]
Where: crates/streaming/src/auth.rs:822-826
Code (verbatim, 2 lines):

        for &byte in text.as_bytes() {
            if matches!(byte, b'=' | b'\r' | b'\n' | b' ' | b'\t') {

What is wrong: `base64_decode` accepts `=` and whitespace anywhere (not only terminal padding), so `"Zm9v=YmFy"` decodes instead of failing. Production auth only *encodes*; the decoder's only non-test consumer is the mockhttp testkit (mockhttp.rs:38). Hygiene-level leniency. Confidence: HIGH — Verification: read + grep (consumers: mockhttp.rs:38, tests).

F-008 [S4]
Where: crates/streaming/src/auth.rs:599
Code: `if matches!(response.status, 301 | 302 | 303 | 307 | 308)`
What is wrong: 303 (See Other) is followed without the RFC-mandated method change to GET and body drop — `template.method` is reused verbatim. GET-only v0 makes this moot today; it will silently violate 303 semantics when another method exists. Confidence: HIGH — Verification: read.

## 4. Per-function table (production code)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Credentials::new/username/password | 62-77 | REAL | — | plain accessors |
| From<Userinfo> for Credentials | 80-84 | REAL | — | |
| Debug/Display for Credentials | 88-102 | REAL | — | redacted; pinned by executed test |
| origin_key | 127-129 | REAL | — | scheme://host:port — the discipline F-002 violates |
| InMemoryCredentialStore::new/lock | 141-149 | REAL | — | poison-tolerant lock |
| store lookup/put | 153-159 | REAL | — | origin-keyed, tested |
| Debug for store | 164-171 | REAL | — | count only |
| basic_authorization | 178-181 | REAL | — | RFC 7617, UTF-8 separator |
| parse_www_authenticate | 217-252 | REAL | S3 | F-004 single-scheme |
| parse_auth_params | 257-303 | REAL | — | quoted-string + escapes; lossless round-trip tested |
| digest_response | 331-349 | REAL | — | RFC 2617 §3.5 vector pinned |
| digest_authorization | 366-423 | REAL | S4 | SHA-256 typed refusal; escaping correct; hardcoded-method caller side (F-006) |
| escape_quoted_string | 439-453 | REAL | — | control-byte refusal, \\-escape |
| AuthFetch::new/with_credentials/with_store | 475-496 | REAL | — | builder |
| next_nc | 499-502 | REAL | — | 8-hex counter |
| next_cnonce | 507-514 | REAL | S3 | F-003 predictable |
| answer | 519-539 | REAL | — | Basic-only fallback, second-401 → AuthFailed |
| follow | 563-644 | REAL | S2/S3 | redirect walk; F-002 port-blind gate, F-005 strip, F-008 303 |
| HttpFetch::fetch | 648-743 | REAL | S2 | precedence userinfo>explicit>store; challenge-first; F-001 unwired |
| Debug for AuthFetch | 749-756 | REAL | — | booleans only |
| classify_response | 762-776 | REAL | — | mirrors transports |
| base64_encode | 785-808 | REAL | — | RFC 4648 vectors pass |
| base64_decode | 812-834 | REAL | S4 | F-007 lenient; testkit-only consumer |
| md5 / md5_hex | 860-921 | REAL | — | RFC 1321 clean-room, vectors pass |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| md5_rfc1321_vectors | YES | standard vectors |
| base64_rfc4648_vectors | YES | standard vectors + round-trip + invalid-input |
| digest_rfc2617_known_vector | YES | Mufasa vector incl. legacy no-qop shape |
| parse_rfc2617_challenge | YES | asserts realm/nonce/opaque/qop + Negotiate→None + nonceless→None |
| digest_authorization_carries_the_vector_response | YES | asserts response hash + SHA-256 refusal |
| redaction_never_exposes_passwords | YES | negative-asserts secret string in Debug/Display of Credentials, store, wrapper, Userinfo (stronger than W02-A09's redaction test — value-based, not name-based) |
| store_lookup_is_origin_keyed | YES | port/host sensitivity — but blind to F-002 (redirect path bypass) |
| conservative_transport_gets_basic_fallback | YES | proves probe has no header, retry has it |
| downgrade_hop_never_receives_credentials | YES | asserts typed CrossSchemeRedirect + zero http hops issued |
| cross_host_hop_never_receives_credentials | YES | asserts hop 3 credential-less |
| digest_parameters_are_escaped_and_control_bytes_refused | YES | escaping + CR/DEL/LF/NUL refusals + lossless round-trip |
| e2e suite (8, real sockets) | YES | mockhttp is a real TcpListener server; Basic/Digest round-trips, typed failures |
Gap: no same-host different-port redirect test (F-002); no multi-scheme WWW-Authenticate test (F-004).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "HTTP basic/digest auth; OS keychain credential storage; redaction tests" | docs/feature-matrix.toml:1472 | PARTIAL | basic+digest+redaction REAL (tests executed); keychain PAPER-ONLY (trait seam only, auth.rs:106-115 records it as [PROPOSAL]); matrix status honestly `not_started` (no wiring — F-001) |
| "6.E1 imp … basic + digest ([PROPOSAL]) … redaction tests prove no secrets in logs/errors" | docs/13-agent-wave-program.md:664 | REAL | module exists, tests pass (ran); digest honestly [PROPOSAL] |
| "credentials are per-server … scheme://host:port" | auth.rs:125-129 (own doc) | FALSE-in-part | the store keys on it, but the redirect gates ignore port (F-002) |
| "the password never reaches a logged URL string" | auth.rs:17-19 | REAL | `Url::parse_with_userinfo` rsplit_once('@') strips userinfo (fetch.rs:228-249) — unlike commands/error.rs's first-'@' mask (W02-A05), this one splits at the LAST '@' correctly |
| Matrix note 1289 "auth incl. the w30.4 scheme-downgrade refusal" landed in crate | docs/feature-matrix.toml:1289 | REAL (in-crate) | downgrade tests executed; but "landed in the crate" ≠ shipped (F-001) |

## 7. Dead / duplicated / unreachable code
The whole module is production-dead (F-001): only tests construct its public types. `base64_decode` is testkit-support. The conservative-transport fallback (`answer`) is exercised only by mock doubles — the two real transports implement `fetch_with_challenge` (client.rs:405-407, mockhttp), so the fallback exists for hypothetical third-party `HttpFetch` impls.

## 8. Dependencies & flags
std-only (atomic, mutex, SystemTime) — matches the crate's no-dep discipline; MD5/base64 hand-rolled to avoid deps (both vector-pinned). No feature gates. Cross-ref Wave 2: SEC-07's LUMEN_DEV_INSECURE_TLS is PAPER-ONLY (client.rs reads no env — W02-A05/W01-A04) — nothing in auth.rs touches env or TLS config, consistent.

## 9. Open questions for the next agent
- Who is *supposed* to compose AuthFetch (engine open path? remote-playlist CLI card 6.E2?) — if a later card wires it, re-check F-002 (port-blind gate) against the wiring; if not, this is 1562 lines of dead security code the matrix lists as not_started.
- `answer()` fallback re-issues Basic to the ORIGINAL URL after a 401 that may have come from a redirect-target host the transport walked to; contained today because client.rs drops Authorization on followed hops (client.rs:250-261,399-402) — but that also means the fallback can never authenticate a redirect-then-challenge flow (challenge-mode handles it). Confirm no shipped path relies on the fallback.
- CredentialStore persistence ([PROPOSAL] keychain) remains open — InMemoryCredentialStore loses everything at exit; nothing prompts the user.
