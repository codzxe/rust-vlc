# Security sweep — control surface (issue #52, card W23-F sweep 3 of 4)

Task ID `7.w23f` (agent `w23-f`, branch `w23/f`). Scope: first-pass
review of `crates/control-protocol/**` (http upgrade, frame codec,
protocol envelope, server session loop, token, policy) and
`apps/control-server/**` (config, dispatch, logging, run loop, main)
against the four named attack classes: handshake downgrade, token
timing/leak, frame-bombing, unauthenticated reachability. The deep
review is card 8.A2 — this sweep records what remains for it.

Threat model: a hostile LOCAL process (or, via DNS rebinding, a web
page the operator visits) connecting to the loopback control port.
It must not reach any command without the token, downgrade the
handshake, exhaust host memory/CPU without bound, or learn token
material.

## 1. Findings and fix

### C1 (Medium, FIXED) — DNS-rebinding via unvalidated `Host`

`http.rs::validate_head` accepted ANY non-empty `Host` header. A web
page at `evil.example` whose DNS rebinds to `127.0.0.1` opens
`ws://evil.example:<port>/` and the browser sends
`Host: evil.example:<port>` — the upgrade would have succeeded.
Default mode holds (token required → 4401), but **read-only mode**
(operator opt-in) admits unauthenticated sessions to the fixed safe
subset (`media.get-info`, `system.get-engine-info`, `tracks.list`,
`system.get-state-snapshot`) — an info-disclosure vector reachable
from any visited website. Fix (the standard RFC 6455 §4.1 server-side
Host check): `is_loopback_host` accepts only `127.0.0.1`,
`localhost`, `[::1]`, `::1`, each optionally `:port`; everything else
is refused at the handshake with the stable token `host-not-loopback`
(HTTP 400, never a 101). Regressions:
`http::tests::non_loopback_hosts_are_refused_and_loopback_forms_accepted`
(8 accepted forms incl. case-insensitivity + bracketed IPv6; 10
refused forms incl. `localhost.evil.example`,
`127.0.0.1.evil.example`, off-loopback literals) and the conformance
test `dns_rebound_host_names_never_upgrade` (raw TCP, fully valid
upgrade headers, rebound Host → 400, no session, no command). The
in-tree test client dials loopback addresses, so all 72 pre-existing
conformance/e2e tests pass unchanged.

## 2. Verified clean (per attack class)

**Handshake downgrade.** `negotiate_version` picks the highest
MUTUAL major; `SUPPORTED_VERSIONS = [1]` — there is no lower rung to
downgrade to, and a no-overlap hello is a typed 4402 close
(`conformance: version_mismatch_is_a_typed_error_and_close_4402`).
The protocol name must equal `lumen-control` exactly (`unknown_protocol…`
closes). A second `hello` after establishment closes (`hello_twice…`)
— authentication state cannot be renegotiated mid-session. `welcome`
carries the server's honest capabilities (`token_auth`, caps), so a
client cannot claim a weaker/stronger posture than negotiated.

**Token timing/leak.** Comparison is `constant_time_eq`
(content-constant; length public by wire format) — used on the hello
path; tests pin the one-bit-at-every-position behavior. Token
material never enters: `ProtocolError` Displays
(`Unauthorized` → "token rejected"), protocol-error frames
(conformance asserts `!message.contains(TOKEN)`), or the structured
log stream (`logging.rs` fields: remote/authenticated/counts/close
only — V8). The single sanctioned reader is the controlling-terminal
print in `main` (generated-token mode), documented in the binary's
process contract. Redaction: client-controlled strings entering error
messages pass `redact_name` (64-char cap); serde errors on the hello
path are replaced by the static `NotHello` (no payload echo). Token
generation: 256 bits from `/dev/urandom`, with a documented weak
fallback (`[VERIFY]`, logged with its `EntropySource` — never the
token).

**Frame-bombing.** Frame length claims are checked against the 1 MiB
cap BEFORE any allocation (64-bit length MSB must be 0;
`MessageTooLarge` closes 1009 — conformance proves a 4 GiB claim
never allocates). The assembler bounds reassembled messages at 1 MiB
across fragments; control frames are ≤ 125 bytes, non-fragmentable;
the HTTP request head is capped at 8 KiB (431 beyond). Subscriptions
are bounded by the message cap (each kind costs ≥ its JSON bytes).
CPU-waits are bounded: 60 s read / 10 s write deadlines (typed 1001).
What remains: no per-message RATE cap (a loopback peer can ping- or
request-flood within the per-item bounds) — recorded for 8.A2.

**Unauthenticated reachability.** Bind is loopback-only at
construction (`NonLoopbackBind` before any OS socket; re-verified
after bind — a hostile resolver surprise fails loudly), conformance
covers both families. Frames before authentication → `not-hello`
close; read-only mode admits only the fixed 4-command safe subset
(`policy.rs`, conformance-pinned); every sensitive op is
deny-by-default with explicit config grants (10-id closed list
asserted against the live catalog). Anti-replay: `requestId` strictly
increasing per connection. `Sec-WebSocket-Key` base64 is decoded with
the strict canonical decoder (`encode(decode(t)) == t`), so the
server's accept computation cannot drift from the client's wire
text.

## 3. ACTUAL results

`cargo test -p control-protocol` — **47 lib + 25 conformance green**
(the new rebinding regressions included). `cargo test -p
control-server` — 11 + 6 green (config/e2e through the app stack).
`cargo clippy -p control-protocol -p control-server --all-targets
-- -D warnings` clean; `cargo fmt --check` clean.

## 4. Remaining for 8.A2 (deep review)

1. Per-message/per-minute rate caps (ping floods, request floods
   within the 1 MiB bounds) — the single-connection design bounds the
   blast radius to one local peer, but a cap set is the honest fix.
2. Full `Origin` header allowlist on top of the C1 Host fence
   (defense in depth for browser-origin identification).
3. Concurrency model: multiple simultaneous connections (today:
   strictly serial `serve_one`; a second peer waits in the accept
   backlog) — review queue/backlog caps against connection-sitting.
4. `weak_fallback_token` entropy on `/dev/urandom`-less hosts (the
   Windows entropy path joins with the platform adapters).
5. Formal review of the in-tree SHA-1/base64 primitives against their
   spec test vectors beyond the shipped suites (they are
   conformance-pinned, not audited).
6. The 6.H1 open Medium (same-host https→http scheme downgrade in the
   `streaming` client — `docs/investigations/network-security-review.md`)
   is 8.A2's first triage item per the phase-8 card; not in this
   sweep's crates.
