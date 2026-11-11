# Network Security Review — 6.H1 (issue #15, wave 13)

**Date:** wave 13 · **Agent:** W13-Z · **Scope:** redirect handling, SSRF
surface, playlist URI traversal, header injection — code-cited from
`crates/streaming` and `crates/playlists`.

## Summary

Reviewed the fetch/client redirect machinery, the scheme/URL acceptance
surface, playlist URI resolution, and header construction. **1 Medium, 3
Low, plus 3 verified-good properties.** The redirect core is solid:
bounded budget with exact hop counts, typed cross-host refusal, loop
detection, and Authorization stripping on every hop. The Medium finding
is same-host scheme downgrade (https→http) being followed by design
(F1 — resolved wave 19, see its Status bullet); the Lows are no
private-range filtering (SSRF posture), verbatim absolute
references in remote playlists, and missing port-side CRLF validation on
builder headers.

## Findings

### F1 — Same-host scheme downgrade followed (Medium)

* **Evidence:** `crates/streaming/src/client.rs:406-409` — "Cross-host
  enforcement (`RedirectPolicy::cross_host`): hosts compare
  case-insensitively; scheme/port changes on the same host are followed
  (6.H1 owns origin widening)". `crates/streaming/src/fetch.rs:296-299`
  — `Url::join` accepts an absolute `http://` Location even when the base
  was `https://`.
* **Risk:** a network attacker or hostile server can downgrade the media
  fetch from https to http on the same host. Mitigated by the credential
  strip (F4) — no `Authorization` is re-sent on a followed hop — so this
  leaks content-integrity/confidentiality of media bytes, not secrets.
* **Fix sketch:** add `RedirectPolicy::cross_scheme` (default: refuse
  `https → http` downgrades, allow upgrades), enforced next to the
  cross-host check at client.rs:410-413 with a typed
  `NetworkError::CrossSchemeRedirect`; record the decision in the
  network-http record.
* **Status:** **RESOLVED (wave 19, `w19/sec-medium`)** — enforced as
  an unconditional typed refusal: `NetworkError::CrossSchemeRedirect`
  (`streaming.net.cross-scheme-redirect`), checked in client.rs
  *before* the cross-host gate, so an https→http hop is never
  followed, same host or not. The wave-19 brief made the review's
  default posture absolute: no `cross_scheme` opt-in knob was added
  — no caller has a legitimate reason to weaken its own transport
  (`RedirectPolicy`/fetch.rs stayed untouched); upgrades
  (http→https) remain followable. The mockhttp testkit's parity
  loop carries the same refusal so the wire-level regression test
  can pin it end-to-end (the testkit is a plaintext socket — the
  same in-tree TLS limit `https_activation.rs` records). Fix
  commit: `4bdae49`; regression tests:
  `client::tests::https_to_http_redirect_downgrade_is_refused_not_followed`
  (red pre-fix: the downgrade was followed to a 200) and
  `client::tests::scheme_downgrade_gate_pins_transport_safety`.
  **Residual:** the auth layer's manual-follow loop (auth.rs:550-554)
  enforces cross-host but not the scheme gate — flagged to 6.E1's
  owner for the same one-line refusal.

### F2 — No private/loopback address filtering (Low, desktop posture)

* **Evidence:** the fetch layer's `Url` accepts any reg-name or IPv4
  literal host (`crates/streaming/src/fetch.rs:159-172`); no
  `is_private`/loopback/link-local check exists anywhere in
  `crates/streaming/src/{fetch,client,schemes}.rs` (grepped; the only
  `127.0.0.1` hits are tests, client.rs:458-630). Only scheme filtering
  exists (`schemes.rs:294-305`: http/https/file only).
* **Risk:** a remote playlist can direct the player to fetch from
  `127.0.0.1`/RFC1918 ranges (SSRF). Blast radius is bounded by the 8 MiB
  response budget (`fetch.rs:376`) and by being a desktop player that
  *plays* rather than exfiltrates — VLC-equivalent posture. Becomes real
  SSRF if the port is embedded server-side.
* **Fix sketch:** document the desktop posture in the network-http
  record; add an opt-in `deny_private_ranges` gate on `RedirectPolicy` /
  fetch config for embedders (typed refusal naming the address class).

### F3 — Remote playlists may reference local files (Low)

* **Evidence:** `crates/playlists/src/m3u.rs:656-675` — `resolve_uri` does
  the RFC 3986 §5 join with dot-segment removal, "`..` never climbs above
  root" (line 662); URL-space traversal is therefore bounded (tests:
  `m3u.rs:661` doc + `hls.rs:685`). **But** absolute references are
  returned verbatim (`m3u.rs:683-686`), including `file://` and Windows
  drive/UNC paths (`m3u.rs:665-666`, `678-682`), and `file:` is an
  openable scheme (`schemes.rs:295` → `classify_file_uri`).
* **Risk:** an http-sourced playlist can point entries at local files,
  which the player opens. By-design VLC parity; no exfiltration path in
  the port (bytes are decoded, not echoed). This is a *disclosure to the
  local user only* vector.
* **Fix sketch:** optional origin policy — refuse `file:`/local-path
  entries when the playlist's base was http(s) — as a config flag, not a
  default (would break legitimate mixed playlists).

### F4 — Builder headers lack port-side CRLF validation (Low)

* **Evidence:** `crates/streaming/src/fetch.rs:390-392` — `with_header`
  pushes `(name, value)` with no control-byte check, unlike the URL
  (`fetch.rs:206-208`) and `Location` (`fetch.rs:301-304`) paths which
  reject spaces/control bytes typed. Values reach ureq's builder
  (`client.rs:235-239`).
* **Risk:** header injection *if* the transport accepted CR/LF. ureq is
  the exact-pinned sole external dep (ureq 3.5.0, admitted card 6.A1b,
  record `docs/dependencies/records/ureq-3.5.0.md`) and is believed to
  reject malformed header names/values [VERIFY ureq 3.5.0's exact
  validation; not audited here]. The port should not rely on that.
* **Fix sketch:** validate name/value for ASCII control bytes in
  `with_header` with a typed `NetworkError::Malformed`, mirroring the
  URL/Location checks — one small test.

### Verified-good properties (for the record)

* **Redirects bounded + typed:** `TooManyRedirects` with exact `followed`
  count (client.rs:397-403); default budget 10 (`DEFAULT_MAX_REDIRECTS`,
  fetch.rs:31) — note: 10, not 5; callers can tighten per-request.
  Cross-host refusal typed (`CrossHostRedirect`, client.rs:410-413); loop
  detection via canonical-URL revisit set (client.rs:415-422).
* **Credential stripping on redirect:** `drop_authorization = true` on
  every hop (client.rs:424), documented "never re-sent on a followed hop"
  (client.rs:35). URI-embedded userinfo is refused before dispatch
  (fetch.rs:161-163, schemes.rs:44-46) and error `Display` never embeds
  secrets (schemes.rs:677-682).
* **Response size budget:** default 8 MiB cap on whole-body reads
  (fetch.rs:376), so a hostile server cannot balloon memory unbounded.

## Honest limitations

This review read the redirect/join/header/resolve functions and grepped
the surfaces named in the 6.H1 brief; it did **not** audit ureq/rustls
internals, the mockhttp testkit's fidelity to the real client, TLS
certificate-verification configuration (that is issue #8 / ADR-011,
pending Lead), or the ranged-resource layer's window arithmetic. Every
unaudited external behavior is marked [VERIFY].
