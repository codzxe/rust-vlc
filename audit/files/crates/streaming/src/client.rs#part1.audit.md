# AUDIT: crates/streaming/src/client.rs#part1 (lines 1-463)
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-21 |
| Chunk boundaries | part1 = 1-463 (all production code, ends at `fetch_inner`'s closing brace; `#[cfg(test)] mod tests` at 464 belongs to part2/W10-A10). Equal-split anchor 1-499 adjusted to the natural production/tests seam. |
| Lines / functions | 463 / 11 (11 prod functions/consts) |
| File verdict | REAL (plain-HTTP leg executed in baseline; TLS leg REAL-UNPROVEN — live tests are `#[ignore]`d and did not run) |
| Reachable from shipped entry point? | YES. `lumen open http://…` → demux_host.rs:491 `SchemeRegistry::new().open_http` → schemes.rs:238 `fetch: Arc::new(UreqClient::new())` → `UreqClient::fetch` (client.rs:368). Executed: baseline orchrun-test-workspace-nofailfast.log.txt:3039 `http_url_opens_over_a_real_loopback_socket ... ok` (real binary, real socket, `Range` requests asserted). TLS: `client::tests::live_example_com_returns_200 ... ignored` (same log :4971) — never executed anywhere in baseline. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 5 |
| Commands run | `rg -n "^(pub |)fn…" client.rs` (outline/split); `rg -n "UreqClient"` (schemes.rs:238 production wiring); `sed -n` on all quoted ranges (verbatim); baseline log greps for client tests and live-test ignore status; read of ureq Cargo pin (streaming/Cargo.toml:52, rustls+webpki-roots, default-features=false). |

## 1. One-paragraph verdict
This is the best-audited file so far: the ureq adapter genuinely enforces the port's security posture — TLS verification pinned on (rustls + webpki, `disable_verification` never touched), adapter-owned redirect loop with exact counts, unconditional https→http downgrade refusal, `Authorization` dropped on every followed hop, advertised-size check before allocation, bounded `take(limit+1)` body read, and a never-silent-EOF truncation check — and the plain-HTTP leg is proven through the shipped binary over a real socket. The holes are periphery: a write deadline the transport silently ignores, a reason phrase that is canonical rather than "as sent" as the port type claims, possible duplicate `Range` headers, per-hop body amplification on redirect chains, and an agent cache with no eviction. The one contract-level gap: RFC-7231-legal relative `Location` references without a leading `/` kill the whole fetch with a typed error — honest, recorded, but a real-world lane breaker.

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt:3039` (loopback e2e through the real binary, Range asserted) and `:4971-4972` (live TLS tests correctly ignored, not silently passing).
— verified by read: the MP §12 budget is genuinely enforced twice — advertised `Content-Length` rejected before any read (288-300) and the body capped at `limit+1` bytes with `ResponseTooLarge` on overflow (304-317); short bodies are a typed `BodyTruncated`, never a silent EOF (318-326); `is_scheme_downgrade` (226-228) is the single shared gate reused by auth.rs:624 and remote_playlist (verified by rg in the auth.rs audit).

## 3. Findings

### F-001 [S3]
Where: crates/streaming/src/client.rs:429 (limit documented at 63-65)
Code:
```
let next = url.join(&location)?;
```
What is wrong: `Url::join` (fetch.rs:299-317) refuses relative references without a leading `/` ("seg001.ts", "index.php"). RFC 7231 §7.1.2 permits relative references in `Location`; real servers (classic PHP/CGI redirects) emit them. The adapter surfaces the typed `ProtocolViolation` and the entire fetch fails.
Why it matters: the shipped http open lane (`lumen open`) breaks on such servers even though the request itself is perfectly valid; the module doc honestly records the divergence ("ureq's own follower resolves more shapes; the port's typed refusal is the contract") — but the contract is the defect here, not just its documentation.
What correct looks like: resolve dot-segments/relative refs against the current path (RFC 3986 §5) inside `Url::join`, or fall back to ureq's resolver for this one shape.
Confidence: HIGH — Verification: read of `Url::join` (fetch.rs:299-317) + this call site; no execution (no live-redirect corpus in-tree).

### F-002 [S4]
Where: crates/streaming/src/client.rs:104-115 (write deadline not wired; recorded at module doc 50-59)
Code:
```
pub(crate) fn agent_config(timeouts: TimeoutConfig) -> ureq::config::Config {
    ureq::config::Config::builder()
        .timeout_connect(Some(timeouts.connect))
        .timeout_per_call(Some(timeouts.read))
```
What is wrong: `TimeoutConfig::write` is accepted by the port (`with_timeouts`), carried in every request, and silently ignored by the only real transport — ureq 3.5.0 has no write knob (documented as an upstream-blocked honest limit; the per-call budget covers writes incidentally).
Why it matters: dead configuration the port sells as a deadline; callers cannot rely on `write` meaning anything.
What correct looks like: keep the field, but make the port docs state "write is best-effort/unused by the ureq transport until an upstream knob exists" — or drop it from `TimeoutConfig` until real.
Confidence: HIGH — Verification: read (builder surface has only connect/per_call).

### F-003 [S4]
Where: crates/streaming/src/client.rs:260-268
Code:
```
for (name, value) in &request.headers {
    if drop_authorization && name.eq_ignore_ascii_case("authorization") {
        continue;
    }
    builder = builder.header(name.as_str(), value.as_str());
}
if let Some(range) = request.range {
    builder = builder.header("Range", range.to_header_value());
}
```
What is wrong: caller headers are forwarded verbatim with only `Authorization` filtered, then the transport appends its own `Range` — a caller who set `with_header("Range", …)` produces a duplicate `Range` header; a caller-set `Host`/`Content-Length` also rides through (ureq may override or duplicate). The port doc says "The transport adds Host, Connection, and Range itself" (fetch.rs:363-365) but nothing enforces the reservation.
Why it matters: duplicate/conflicting reserved headers are undefined behavior across servers; hygiene seam in an otherwise strict port.
What correct looks like: filter reserved names (`host`, `range`, `content-length`, `connection`) from caller headers (typed warning or silent drop, documented), then add the transport's own.
Confidence: HIGH — Verification: read; no duplicate-Range test exists in the suite (part2's test list has none).

### F-004 [S4]
Where: crates/streaming/src/client.rs:401-402 with 302-317 (per-hop read inside `single_hop`)
Code:
```
loop {
    let response = single_hop(&agent, &url, &request, drop_authorization)?;
```
What is wrong: each redirect hop's response body is fully read (up to `response_limit+1` bytes) before the status is examined and the body discarded — a hostile server can force `(max_redirects+1) × response_limit` bytes of transfer (default ≈ 88 MiB) for one logical fetch; the budget is honored per-response (as documented) but per-chain transfer is unbounded beyond the hop count. Likewise, non-`Authorization` custom headers are re-sent on cross-host hops (only the cross-host *follow* is policy-gated, not header hygiene).
Why it matters: bandwidth-amplification and minor credential-adjacent leakage (custom tokens) under a hostile redirect chain; mitigated by small default bodies on well-behaved servers and the 10-hop cap.
What correct looks like: for 3xx hops, read only a small head of the body (or `Content-Length: 0` fast-path) before continuing; document the custom-header cross-host behavior or filter it.
Confidence: MEDIUM — Verification: read; amplification arithmetic by hand, not executed.

### F-005 [S4]
Where: crates/streaming/src/client.rs:354-363
Code:
```
static CACHE: OnceLock<Mutex<HashMap<TimeoutConfig, Arc<Agent>>>> = OnceLock::new();
...
    guard
        .entry(timeouts)
        .or_insert_with(|| Arc::new(Agent::new_with_config(agent_config(timeouts))))
        .clone()
```
What is wrong: the process-wide agent cache has no eviction and is keyed by the full `TimeoutConfig` domain (3 free `Duration`s). Every distinct deadline triple mints an `Agent` with its own connection pool, held forever.
Why it matters: theoretical unbounded growth if any caller varies timeouts per request (today the shipped lane uses fixed defaults → 1 entry); a leak-shaped seam with a one-line bound fix.
What correct looks like: cap the map (LRU or "N configs max, evict on insert"), or key on a coarser deadline class.
Confidence: HIGH — Verification: read; `shared_agent_is_cached_per_deadline_config` (part2, 944) pins the caching behavior but no test bounds growth.

### F-006 [S4]
Where: crates/streaming/src/client.rs:330 vs fetch.rs:471-472
Code:
```
reason: status.canonical_reason().unwrap_or_default().to_string(),
```
What is wrong: the port type documents `reason` as "The reason phrase as sent" (fetch.rs:471-472); the ureq transport cannot recover the wire reason and substitutes `http::StatusCode::canonical_reason`. The divergence is recorded in this file's module doc (60-62) but the port-type doc still claims as-sent.
Why it matters: the port contract overstates fidelity; a mock-vs-real difference invisible until a non-canonical reason matters (it never does today).
What correct looks like: fix the `HttpResponse::reason` doc to "canonical reason, or empty" (the mock client does preserve wire reasons — a real mock/real asymmetry).
Confidence: HIGH — Verification: read of both sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| tls_config | 91-96 | REAL | — | rustls + WebPki roots, verification never disabled; pinned by part2 test `agent_config_pins_transport_safety` (923) |
| agent_config | 111-119 | REAL (defect) | S4 | F-002: write deadline unwired; max_redirects(0) + status-as-error(false) verified by measurement claims in docs and by part2 tests |
| map_io_error | 123-133 | REAL | — | TimedOut/WouldBlock → ReadTimeout (Linux EAGAIN noted); rest preserved in source chain |
| map_ureq_error | 143-197 | REAL | — | exhaustive incl. non_exhaustive fallback; unreachable arms documented as defense-in-depth |
| is_redirect | 202-204 | REAL | — | 301/302/303/307/308; 300/304 correctly not followed |
| is_scheme_downgrade | 226-228 | REAL | — | pub(crate) shared gate; verified reused at auth.rs:624 (single source, no drift) |
| classify | 234-245 | REAL | — | 416 unconditional (no `request.range` gate, unlike auth.rs:769) — harmless; 1xx/3xx-no-location surfaced |
| single_hop | 253-334 | REAL (defects) | S4 | F-003 header forwarding, F-004 body read per hop; Content-Length pre-check + take(limit+1) + BodyTruncated all correct (MP §12 honored) |
| UreqClient::new | 347-349 | REAL | — | trivial |
| UreqClient::shared_agent | 354-364 | REAL (defect) | S4 | F-005 no eviction; poison-tolerant lock |
| HttpFetch::fetch / fetch_with_challenge | 367-378 | REAL | — | challenge override EXISTS (fetch.rs default-delegation concern retired for this transport) |
| UreqClient::fetch_inner | 383-462 | REAL (defect) | S3/S4 | F-001 relative-Location refusal; loop/budget/downgrade/cross-host ordering correct; `drop_authorization` sticky after first hop (correct, conservative) |

## 5. Test honesty
Tests (464-998) belong to part2/W10-A10. Noted here for reachability: the two LIVE-network tests (`live_example_com_returns_200`, `live_httpbin_redirect_chain_lands_on_200`) are `#[ignore = "live network required"]` and appear as `ignored` in the baseline log (:4971-4972) — no baseline run ever executed a real TLS handshake with this transport.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "rustls TLS with verification always on" (shipped lane) | client.rs:7, protocol-http-https row description "TLS verification" | REAL (config) / REAL-UNPROVEN (handshake) | tls_config:91-96 + config-pinning test; live tests ignored in baseline (:4971); https_activation.rs:24-26 self-admits "[VERIFY: needs a networked CI lane]" |
| "Open http(s) URLs: range requests, redirects…" | feature-matrix.toml:1282 | REAL for plain HTTP | baseline loopback e2e (:3039) with Range requests asserted; redirects covered by part2 tests over mockhttp |
| "An https→http redirect is a typed refusal … same host or not" | client.rs:66-69 | REAL | 436-437 + shared `is_scheme_downgrade`; part2 tests `https_to_http_redirect_downgrade_is_refused_not_followed` (689), `scheme_downgrade_gate_pins_transport_safety` (733) |
| "reason phrase … as sent" (port type) | fetch.rs:471-472 | FALSE for this transport | F-006: canonical_reason substituted |
| `TimeoutConfig.write` "Deadline per request write" | fetch.rs:99-100 | PAPER-ONLY in the real transport | F-002: never wired |

## 7. Dead / duplicated / unreachable code
No dead code in this part. `is_redirect` duplicates the mock testkit's dialect by design (parity documented). `classify` near-duplicates auth.rs `classify_response` (the only delta: the 416 range-gate) — tracked, acceptable.

## 8. Dependencies & flags
ureq =3.5.0 exact pin, default-features=false, features=["rustls","rustls-webpki-roots"] (Cargo.toml:52) — no native-tls, no cookies, no gzip (range/Content-Length semantics preserved). The TLS provider is ring 0.17.14 (C+asm) — recorded Lead-ADR deviation (Cargo.toml:49-51). No feature gates; the transport is unconditionally compiled and shipped.

## 9. Open questions for the next agent
- W10-A10 (part2): do any tests cover a duplicate `Range` header (F-003) or relative-Location refusal through the REAL transport (F-001)? My read of the test list says no.
- Wave gate: REQUEST-ORCH-RUN candidate (not blocking): `cargo test -p streaming live_ -- --ignored --nocapture` on a networked lane to convert the TLS claim from REAL-UNPROVEN to REAL/REFUTED — the admission record cites an out-of-band measurement not reproducible from the repo.
- Whoever owns ranged.rs: confirm `RangedConfig::default()` uses fixed `TimeoutConfig` values (F-005 growth depends on it).
