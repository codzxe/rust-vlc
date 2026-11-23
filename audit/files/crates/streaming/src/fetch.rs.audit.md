# AUDIT: crates/streaming/src/fetch.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-23 |
| Lines / functions | 730 / 29 (22 production fns/impls, 7 tests) |
| File verdict | REAL (one reachable, typed HTTP port; two real interop defects) |
| Reachable from shipped entry point? | YES — `lumen open http://…` → apps/cli/src/parse.rs:510-518 → apps/cli/src/demux_host.rs:480 `streaming::fetch::Url::parse(url)` and :491 `SchemeRegistry::new().open_http(...)` → crates/streaming/src/client.rs `UreqClient: HttpFetch` (fetch.rs:507-536 is the trait it implements; client.rs:429 `url.join(&location)` is the redirect consumer of `Url::join`). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 4 |

## 1. One-paragraph verdict
This is the port that the shipped binary actually runs its HTTP lane over, and it is a real, disciplined surface: strict typed URL parsing, header-injection validation at the port (not left to the transport), bounded response budgets by default, redacted `Userinfo` debug. The 7 unit tests pass (I ran them). Two genuine interop defects: `Url::join` refuses RFC 7231-legal relative `Location` references, and the shipped ureq adapter follows redirects through exactly that join — so any server answering a redirect with `Location: next.ts` (no scheme, no leading slash) fails typed; and the URL parser silently mis-parses `http://host?a=b` and `http://host#frag` (query/fragment land in the HOST), directly contradicting the doc's "fragments … are rejected rather than mis-parsed".

## 2. What actually works (verified)
— `cargo test -p streaming --lib fetch::` → `7 passed; 0 failed; 0 ignored; 207 filtered out`. Wire range forms, URL accept/reject tables (ftp, `host:notaport`, IPv6 literal, userinfo, space, NUL), join accept/reject, bounded defaults, header-injection refusals (CR/LF, controls, `:`/non-ASCII/empty names) — exact-match assertions that would fail on regression.

## 3. Findings
F-001 [S2]
Where: crates/streaming/src/fetch.rs:299-317 (join), consumer crates/streaming/src/client.rs:429
Code:
```
        Err(NetworkError::ProtocolViolation {
            detail: "relative Location references without a leading '/' are unsupported in v0",
        })
```
What is wrong: RFC 7231 §7.1.2 allows `Location` to be a relative reference; plenty of real servers send `Location: next.ts` or `Location: folder/next.ts`. `Url::join` only resolves absolute and root-relative references, and the shipped redirect follower consumes exactly this join (client.rs:429 `let next = url.join(&location)?;`), so every relative-Location redirect fails the fetch typed. Why it matters: a legal, common server behavior makes the reachable HTTP open lane fail where VLC (curl semantics) follows. What correct looks like: resolve relative references against `self.path_and_query`'s directory (the playlists crate already has an RFC 3986 §5 `resolve_uri` — reuse it).
Confidence: HIGH — Verification: read both sides + tests (join's reject list includes `"seg001.ts"` — the rejection is intended, the interop consequence is the defect).

F-002 [S3]
Where: crates/streaming/src/fetch.rs:159-163 (doc) vs 223-226 (code)
Code:
```
/// path+query. IPv6 literals, userinfo, and fragments are later cards'
/// scope and are rejected rather than mis-parsed (MP §8: no silent
/// failure).
```
vs
```
        let (authority, path_and_query) = match rest.split_once('/') {
            Some((authority, tail)) => (authority, format!("/{tail}")),
            None => (rest, "/".to_string()),
        };
```
What is wrong: the doc claims fragments are REJECTED. They are not: `Url::parse("http://host#frag")` has no `/` after the authority, so `rest.split_once('/')` returns None and the ENTIRE string `host#frag` becomes the authority → host = `"host#frag"`, port 80, path `"/"`. Same for a query with no path: `http://host?a=b` → host = `"host?a=b"` (the query is silently swallowed into the host; `Host: host?a=b:80` goes on the wire). This is a silent mis-parse of a legal URL shape, exactly what MP §8 (quoted in the doc) forbids. Why it matters: doc is FALSE; a `http://example.com?src=1` open (no slash) mis-targets DNS and the Host header. What correct looks like: split the authority at the first `/`, `?`, or `#`; reject `#` typed (fragments carry no fetch semantics).
Confidence: HIGH — Verification: read (control-flow); not executed — no test covers a slash-less query/fragment URL (test list confirmed).

F-003 [S3]
Where: crates/streaming/src/fetch.rs:258-268
Code:
```
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => {
                let port = port
                    .parse::<u16>()
```
What is wrong: with two colons (`http://host:8080:9090/`), `rsplit_once(':')` yields host = `"host:8080"`, port = 9090 — a host containing `:` is accepted unvalidated (only emptiness and `[`/`]` are checked at :253-257, :269-273). The mis-shaped host flows into DNS and the `Host:` header. Not a security hole (no injection — `:` in a value position), but a malformed-URL acceptance where every sibling shape is typed. What correct looks like: reject a host containing `:` after the port split.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/streaming/src/fetch.rs:75-83
Code:
```
            } => format!("bytes={start}-{}", end_exclusive.saturating_sub(1)),
```
What is wrong: `Bounded { start: 0, end_exclusive: 0 }` (an EMPTY range) serializes as `bytes=0-0` (a 1-byte request); `end_exclusive <= start` in general produces inverted/nonsense ranges with no validation (`bytes=5-4`). Server answers 416 or ignores. Why it matters: semantics of the end-exclusive model break at the empty/underflow edge. What correct looks like: refuse `end_exclusive <= start` at construction or serialize empty as typed error.
Confidence: HIGH — Verification: read (arithmetic trivial).

F-005 [S4]
Where: crates/streaming/src/fetch.rs:459-462
Code:
```
    pub fn with_response_limit(mut self, limit: u64) -> Self {
        self.response_limit = limit;
        self
    }
```
What is wrong: no validation — a 0 limit makes every non-empty response fail `ResponseTooLarge` (HLS configs re-validate at their layer, live.rs:187-197, but the port itself accepts nonsense budgets). Minor consistency gap vs the port's typed-everything posture.
Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/streaming/src/fetch.rs:44-49
Code:
```
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
        }
```
What is wrong: single-variant enum as designed ("grow it later") — currently the method field is a constant; no caller can issue anything but GET (HEAD would let the ranged resource probe sizes without transfer). Design note, not a defect per se — recorded because the doc sells it as forward-compatible growth that has not happened.
Confidence: HIGH — Verification: read + rg (no other variant).

F-007 [S4]
Where: crates/streaming/src/fetch.rs:533-535
Code:
```
    fn fetch_with_challenge(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
        self.fetch(request)
    }
```
What is wrong: the trait default silently re-classifies 401 as `NetworkError::HttpStatus` — documented (L522-527) and the real client overrides it (client.rs:375), so this is honest degradation, but any third-party transport plugged into the port loses challenge surfacing without a compile-time signal. Blast radius: auth lane only.
Confidence: HIGH — Verification: read + rg overrides (client.rs:375, mockhttp.rs:900).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HttpMethod::as_str | 44-48 | REAL | S4 | F-006 constant surface |
| ByteRange::to_header_value | 75-83 | REAL | S4 | F-004 empty-range edge |
| TimeoutConfig::Default | 103-111 | REAL | — | 5s/5s/5s |
| RedirectPolicy::Default | 149-157 | REAL | — | 10 redirects, cross-host on |
| Url::parse | 184-186 | REAL | S3 | F-002/F-003 via parse_options |
| Url::parse_with_userinfo | 199-201 | REAL | — | auth surface, secret redacted in Userinfo Debug |
| Url::parse_options | 205-283 | REAL | S3 | F-002 query/fragment into host; F-003 double-colon host; control-byte + IPv6 + userinfo-v0 refusals correct |
| Url::authority | 286-288 | REAL | — | |
| Url::join | 299-317 | REAL | S2 | F-001 relative Location refused; consumed by shipped redirect follower |
| Display for Url | 320-330 | REAL | — | round-trips (explicit default port) |
| Debug for Userinfo | 346-353 | REAL | — | password "[redacted]" — verified in code |
| HttpRequest::get | 381-391 | REAL | — | bounded defaults |
| HttpRequest::with_header | 408-431 | REAL | — | AUD-15a injection validation, tested |
| HttpRequest::with_range | 434-437 | REAL | — | |
| HttpRequest::with_timeouts | 440-443 | REAL | — | |
| HttpRequest::with_redirects | 446-449 | REAL | — | |
| HttpRequest::with_redirect_mode | 453-456 | REAL | — | |
| HttpRequest::with_response_limit | 459-462 | REAL | S4 | F-005 no validation |
| HttpResponse::is_success | 482-484 | REAL | — | |
| HttpResponse::header | 487-492 | REAL | — | case-insensitive first-match |
| HttpFetch::fetch | 507-513 | REAL | — | object-safe, Send+Sync pinned by test |
| HttpFetch::fetch_with_challenge | 533-535 | REAL | S4 | F-007 default degrades |
| 7 test fns | 545-729 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| port_is_object_safe_send_sync | YES (compile-level) | dyn-compat + bounds pinned |
| byte_range_wire_forms | YES | exact wire strings |
| url_parse_accepts_http_and_https | YES | fields + defaults + Display round-trip |
| url_parse_rejects_hostile_shapes_typed | YES | 7 reject shapes, exact variant |
| url_join_resolves_redirect_targets | YES | accept + 3 reject refs |
| request_defaults_are_bounded | YES | every default asserted |
| with_header_rejects_control_bytes_and_bad_names_typed | YES | injection matrix + legit builds |
Gap: no test for slash-less query/fragment URLs (F-002), no relative-Location acceptance test (F-001 — rejection is asserted, interop not).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "IPv6 literals, userinfo, and fragments … are rejected rather than mis-parsed" | fetch.rs:161-163 | FALSE (fragments) | F-002: `http://host#frag` → Ok with host="host#frag"; IPv6/userinfo refusals ARE real (test :610-611) |
| "no panic, no silent truncation, no unbounded allocation on hostile input" | fetch.rs:505-506 | REAL | port contract; enforcement measured in client.rs lane (other agent) |
| "relative Location references … unsupported in v0" | fetch.rs:315-316 | REAL (honest) | but see F-001 for the interop cost |

## 7. Dead / duplicated / unreachable code
None dead. All builders used by crate callers (rg with_range/with_timeouts/with_redirects consumers in ranged.rs/client.rs/auth.rs).

## 8. Dependencies & flags
No external deps — pure std + crate::error::NetworkError. No feature gates. Layering clean (port only).

## 9. Open questions for the next agent
- client.rs (ureq adapter) audit: does its Content-Length pre-allocation check interplay correctly with `response_limit`? (out of my file set)
- Does any server-facing test cover relative-Location redirects end-to-end through UreqClient? (auth_e2e/netfault lanes)
