# AUDIT: crates/streaming/src/fetch.rs
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-21 |
| Lines / functions | 730 / 30 (24 prod + 7 tests + consts) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open http://…` → apps/cli/src/demux_host.rs:413-418 `"http" \| "https"` arm → `open_http_source` (demux_host.rs:482-498) → `streaming::fetch::Url::parse` (fetch.rs:184) and `SchemeRegistry::new().open_http` (schemes.rs:238 wires `Arc::new(UreqClient::new())`)). Executed: baseline `orchrun-test-workspace-nofailfast.log.txt:3039` — `test http_url_opens_over_a_real_loopback_socket ... ok` (real binary, real loopback socket, Range-header use asserted in the test body at apps/cli/tests/unsupported_path.rs:230-251). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 2 |
| Commands run | `rg -n "HttpFetch|fetch_with_challenge"` (impls/users: client.rs:367, reconnect.rs:773, mockhttp, hls, schemes, apps/cli demux_host.rs:383/476); `rg -n "UreqClient"` (schemes.rs:238 = production wiring, no feature gate); `sed -n` on all quoted ranges (verified verbatim); `rg -in "cookie" crates/streaming apps/cli/src` (no cookie code — only codec "magic cookie" hits); `rg -n "http_url_opens_over_a_real_loopback_socket"` in baseline logs (found in orchrun-test-workspace-nofailfast.log.txt:3039, absent from test-workspace.log.txt which aborted at the desktop zero_alloc failure). |

## 1. One-paragraph verdict
This is the port layer the whole network story hangs off, and it is genuine: typed request/response/builder types, a hand-rolled strict URL parser, an object-safe sync trait, executed end-to-end through the shipped `lumen` binary over a real loopback socket in the baseline run. But the parser contradicts its own contract in two places (fragments silently kept though the doc claims typed rejection; multi-colon authorities silently mis-split into a host containing `:`), the `ByteRange` wire form emits garbage headers for inverted/empty bounds with no typed refusal, and the feature-matrix row that cites this port claims "cookies" which exist nowhere in the crate. None of these break the proven loopback lane; all of them violate the crate's own "no silent mis-parse" doctrine (MP §8).

## 2. What actually works (verified)
— verified by baseline run `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3039` → `test http_url_opens_over_a_real_loopback_socket ... ok` and `:2847` → `test demux_host::tests::http_url_opens_through_the_ranged_resource ... ok`: the fetch port's `Url::parse` + `HttpRequest`/`HttpFetch` (via `UreqClient`) run inside the shipped `lumen` binary, open a media URL over a real socket, and the test asserts at least one request carried a `Range` header.
— verified by read: builder validation (`with_header`) genuinely refuses CR/LF, control bytes, `:`-bearing, non-ASCII, and empty names; `Userinfo`'s hand-written `Debug` redacts the password (fetch.rs:346-353).

## 3. Findings

### F-001 [S2]
Where: crates/streaming/src/fetch.rs:223-226 (gap; contract stated at 159-163)
Code:
```
/// path+query. IPv6 literals, userinfo, and fragments are later cards'
/// scope and are rejected rather than mis-parsed (MP §8: no silent
/// failure).
...
let (authority, path_and_query) = match rest.split_once('/') {
    Some((authority, tail)) => (authority, format!("/{tail}")),
    None => (rest, "/".to_string()),
};
```
What is wrong: the module doc promises fragments are "rejected rather than mis-parsed". `parse_options` has no fragment check anywhere (verified: `rg -n "fragment|'#'"` over `crates/streaming/src` finds only the two doc comments at fetch.rs:161 and fetch.rs:293). `Url::parse("http://host/media.mp4#t=1.5")` returns `Ok` with `path_and_query = "/media.mp4#t=1.5"`; the fragment rides the request line (RFC 9110 request-target forbids `#`), and nothing downstream strips it.
Why it matters: media-fragment URLs (`#t=…`) are a realistic user spelling; the shipped open lane sends the fragment to the server, typically yielding a 404/wrong resource instead of either stripping it or refusing typed. It is exactly the silent mis-parse MP §8 forbids and this file's own doc denies.
What correct looks like: reject `#` in `parse_options` with `NetworkError::ProtocolViolation` (matching the doc), or strip it; add the shape to `url_parse_rejects_hostile_shapes_typed`.
Confidence: HIGH — Verification: read + traced callers (`open_http_source` demux_host.rs:485-489 parses user URLs verbatim through this grammar).

### F-002 [S3]
Where: crates/streaming/src/fetch.rs:258-268
Code:
```
let (host, port) = match authority.rsplit_once(':') {
    Some((host, port)) => {
        let port = port
            .parse::<u16>()
            .map_err(|_| NetworkError::ProtocolViolation {
                detail: "invalid port",
            })?;
        (host, port)
    }
```
What is wrong: after splitting at the last `:`, the host side is never validated for `:`. `http://host:8080:90/` parses `Ok` with `host="host:8080", port=90` — a host string containing a colon is accepted silently.
Why it matters: the malformed URL surfaces later as a confusing DNS/connect error (or a garbage `Host:` header) instead of the typed parse refusal this parser exists to give; contradicts the "typed validation surface" contract at line 160.
What correct looks like: reject when the remaining host contains `:` (only the port side may), or reject port `0`.
Confidence: HIGH — Verification: read (rsplit_once semantics traced by hand; no execution).

### F-003 [S3]
Where: crates/streaming/src/fetch.rs:75-84
Code:
```
} => format!("bytes={start}-{}", end_exclusive.saturating_sub(1)),
```
What is wrong: `ByteRange::Bounded` never validates `end_exclusive > start`. Inverted (`start=100,end_exclusive=50` → `bytes=100-49`) and empty (`start=5,end_exclusive=5` → `bytes=5-4`; `end_exclusive=0` → `bytes=5-0`) ranges produce syntactically invalid `Range:` headers with no typed refusal.
Why it matters: GIGO at the port boundary while the crate's doctrine is typed refusals for invalid shapes; server-controlled HLS `EXT-X-BYTERANGE` values flow into `Bounded` (hls/media.rs:549 caller), so a hostile playlist can drive a nonsense range onto the wire instead of a typed error.
What correct looks like: a `Bounded::new` (or `to_header_value`) returning `Result` refusing `end_exclusive <= start`.
Confidence: HIGH — Verification: read + `rg -n "ByteRange::Bounded"` callers (ranged.rs:567/747, hls/media.rs:549, reconnect.rs:1260, fuzz).

### F-004 [S4]
Where: crates/streaming/src/fetch.rs:424-428
Code:
```
if value.bytes().any(|byte| byte < 0x20 || byte == 0x7F) {
    return Err(NetworkError::ProtocolViolation {
        detail: "header value contains control bytes (header injection)",
    });
}
```
What is wrong: HTAB (0x09) is a legal header-value byte (RFC 9110 field-content allows SP/HTAB), but this check refuses it (and the test at line 687 pins that refusal: `"tab\there"`).
Why it matters: over-strict, deliberately conservative — safe direction, but rejects legal `Accept`-style values containing tabs and mislabels them "header injection".
What correct looks like: permit 0x09 in values (keep refusing other C0/0x7F), or document the deviation from RFC 9110 as intentional.
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: crates/streaming/src/fetch.rs:285-288
Code:
```
pub fn authority(&self) -> String {
    format!("{}:{}", self.host, self.port)
}
```
What is wrong: `authority()`/`Display` always emit the explicit port, including scheme defaults (`http://example.com:80/…`). URLs are never normalized to omit default ports.
Why it matters: non-canonical round-trips (redirect identity, session URI, `Host:` header carries `example.com:80`); RFC-legal but noisy and diverges from what every mainstream client sends.
What correct looks like: omit the port when it equals the scheme default.
Confidence: MEDIUM — Verification: read; no server-interaction evidence either way in baseline (loopback mock accepts anything).

### F-006 [S2] (claim defect on the row that cites this file)
Where: docs/feature-matrix.toml:1282 (row `protocol-http-https`) vs the code this row cites
Code:
```
description = "Open http(s) URLs: range requests, redirects, cookies, TLS verification"
```
What is wrong: the row claims "cookies". The streaming crate has zero cookie code (`rg -in "cookie" crates/streaming/src apps/cli/src` → only codec "magic cookie" hits in containers/), and ureq was admitted with cookies explicitly excluded (crates/streaming/Cargo.toml:47-48: "NO cookies/charset/json/multipart").
Why it matters: the row's `status = implemented` on windows/linux/macos packages a capability ("cookies") that does not exist anywhere in the repo — an inflated claim on a P0 row; TLS verification is likewise config-pinned but never round-tripped in-tree (https_activation.rs:24-26 admits a real TLS handshake is `[VERIFY: needs a networked CI lane]`).
What correct looks like: description limited to what exists: range requests, redirects, TLS verification (config-pinned, live handshake unverified); cookies removed or marked not-implemented.
Confidence: HIGH — Verification: read + rg over the whole crate pair; baseline loopback e2e is plain-HTTP only.
(Note for the wave-10 matrix-rows owner: this row belongs to your ledger; recorded here because fetch.rs is the cited evidence.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DEFAULT_RESPONSE_LIMIT | 28 | REAL | — | 8 MiB budget; enforced by transports (client.rs), referenced by hls/* |
| DEFAULT_MAX_REDIRECTS | 31 | REAL | — | used by RedirectPolicy::default |
| HttpMethod::as_str | 44-48 | REAL | — | single Get arm, trivial correct |
| ByteRange::to_header_value | 75-84 | REAL (defect) | S3 | F-003: no bound validation; saturating_sub masks end_exclusive=0 |
| TimeoutConfig::default | 103-111 | REAL | — | 5s/5s/5s; pinned by test request_defaults_are_bounded |
| RedirectPolicy::default | 149-157 | REAL | — | 10 redirects, cross_host=true, Follow |
| Url::parse | 184-186 | REAL | — | thin wrapper over parse_options |
| Url::parse_with_userinfo | 199-201 | REAL | — | auth-layer surface; userinfo returned separately, never re-embedded |
| Url::parse_options | 205-283 | REAL (defects) | S2/S3 | F-001 fragment gap, F-002 colon-host gap; control/space, scheme, empty-host, IPv6, userinfo(strict) refusals all real |
| Url::authority | 286-288 | REAL (defect) | S4 | F-005: always explicit port |
| Url::join | 299-317 | REAL | — | absolute + root-relative resolve; bare-relative/fragment refs refused typed; scheme-relative `//host` correctly refused (no open-redirect shape) |
| Display for Url | 321-330 | REAL | S4 | rides F-005 |
| Debug for Userinfo | 347-353 | REAL | — | password redacted; verified by read |
| HttpRequest::get | 381-391 | REAL | — | defaults bounded; pinned by test |
| HttpRequest::with_header | 408-431 | REAL (defect) | S4 | injection-shaped refusals genuine (AUD-15a); F-004 HTAB over-strict |
| HttpRequest::with_range | 434-437 | REAL | — | trivial setter; no validation (F-003 lives in to_header_value) |
| HttpRequest::with_timeouts | 440-443 | REAL | — | trivial setter |
| HttpRequest::with_redirects | 446-449 | REAL | — | trivial setter |
| HttpRequest::with_redirect_mode | 453-456 | REAL | — | mode-only setter, budget preserved (pinned by test line 656-659) |
| HttpRequest::with_response_limit | 459-462 | REAL | — | trivial setter |
| HttpResponse::is_success | 482-484 | REAL | — | 2xx range check |
| HttpResponse::header | 487-492 | REAL | — | first match, ASCII case-insensitive; correct |
| HttpFetch::fetch (trait) | 507-513 | REAL | — | object-safe, Send+Sync; pinned by port_is_object_safe_send_sync |
| HttpFetch::fetch_with_challenge (default) | 533-535 | REAL | — | default delegates to fetch → transports that don't override never surface 401; UreqClient DOES override (client.rs:375) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| port_is_object_safe_send_sync (546) | YES | fn-pointer coercion + MockClient bounds fail at compile time if trait loses object safety/Send+Sync |
| byte_range_wire_forms (559) | YES | exact string asserts on both arms; would catch inclusive/exclusive flip |
| url_parse_accepts_http_and_https (576) | YES | field-level asserts incl. default ports and path/query split |
| url_parse_rejects_hostile_shapes_typed (600) | YES | 7 hostile shapes asserted typed; WOULD NOT catch F-001/F-002 shapes (fragment, colon-host absent from the list) — test gap, not tautology |
| url_join_resolves_redirect_targets (625) | YES | asserts resolved fields and 3 refused refs (incl. "#frag" for join — the asymmetry with parse/F-001 is real) |
| request_defaults_are_bounded (648) | YES | every default asserted |
| with_header_rejects_control_bytes_and_bad_names_typed (671) | YES | 4 value shapes + 4 name shapes refused, legit shapes built and asserted; pins the error code string |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "IPv6 literals, userinfo, and fragments are … rejected rather than mis-parsed" | fetch.rs:161-163 module doc | FALSE (fragments) | F-001: no fragment check in parse_options; `#frag` accepted into path_and_query. IPv6 (253-257) and userinfo (229-233) ARE rejected |
| "Open http(s) URLs: range requests, redirects, cookies, TLS verification" | feature-matrix.toml:1279-1289 `protocol-http-https` | cookies: FALSE; range/redirects on plain HTTP: REAL; TLS: REAL-UNPROVEN | F-006; baseline orchrun-test-workspace-nofailfast.log.txt:3039 (loopback, plain HTTP, Range asserted); TLS config pinned by client.rs test, live handshake unverified (https_activation.rs:24-26) |
| "8 MiB … bounded memory is a contract" (MP §12) | fetch.rs:26-28 | REAL (port side) | response_limit carried and defaulted here; enforcement audited in client.rs (W10-A10's part) |
| lib.rs: "std-only … no async runtime" | crates/streaming/src/lib.rs:12-18 | REAL for this file | no async/await/tokio tokens in fetch.rs (read) |

## 7. Dead / duplicated / unreachable code
No dead code found. Everything in the file is either used by transports/HLS/auth or is the trait seam itself. `apps/web/src/io/fetch_port.rs` mirrors this port for the wasm host (separate lane, doc 7.H4) — duplication is documented as deliberate mirroring, not drift, but see W-10 web-lane agents for whether the two grammars diverge.

## 8. Dependencies & flags
File is std-only (`std::time::Duration` + `crate::error::NetworkError`). No feature gates anywhere in the crate (Cargo.toml has no `[features]`); ureq is an unconditional dependency, so the real transport cannot be compiled out of shipped builds. Architecture-check violation "lumen-cli depends on streaming, not declared in boundaries.toml" (baseline gate-check_architecture.log.txt) is the registration debt for exactly this reachability chain.

## 9. Open questions for the next agent
- W10-A10 (client.rs part2): does `UreqClient` enforce `response_limit` against `Content-Length` BEFORE body read (the MP §12 claim)? Does it re-validate the `Host` it sends (F-005 consequence)?
- Wave-10 ranged.rs owner: does `RangedResource` ever construct `Bounded` from server-controlled values without the F-003 validation?
- Matrix-rows owner: fold F-006 (cookies claim) into the `protocol-http-https` row verdict.
