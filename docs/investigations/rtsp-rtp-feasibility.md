# RTSP/RTP Feasibility — Investigation 6.G1-I (issue #15)

**Date:** wave 13 · **Agent:** W13-Z · **Status:** verdict recorded

## Summary

The RTSP/RTP family (`rtsp://`, `rtsps://`, `rtp://`, `rtcp://`, `udp://`) is
**typed-refused today** by the streaming port's `SchemeRegistry`, and this
investigation recommends keeping it that way: **DEFER**, with the typed
refusal as the product surface, re-evaluated only when a lead customer needs
camera/live input. There is no mature, dependency-free pure-Rust RTSP client
stack to adopt; building one is a multi-wave effort (per-codec RTP
depacketizers + RTSP session state machine + timers/NAT handling) that does
not fit the current phase's scope.

## Evidence

### Current refusal behavior (source-checked)

* `crates/streaming/src/schemes.rs:20-24` — every non-`http/https/file`
  scheme (including `rtsp:`, `ftp:`, `data:`) maps to the typed refusal
  `SchemeError::Unsupported` naming the scheme; per-scheme *policy* lives in
  the record `docs/formats/network-http.md`, "the registry is uniform, the
  record is the decision ledger."
* `crates/streaming/src/schemes.rs:560-563` — refusal table test rows:
  `ftp`, `rtsp://cam.example.org/stream.sdp`, `rtp://224.1.1.1:5004`,
  `udp://@239.255.12.42:1234`.
* `crates/streaming/src/schemes.rs:677-682` — userinfo-bearing RTSP URL is
  refused and the error `Display` never embeds the secret (credential-free
  logging, card 6.A3).
* `crates/streaming/src/schemes.rs:104` — `Unsupported` carries the refused
  scheme lowercased, so callers can branch on it (typed refusal, not a
  panic/stringly error).

### Protocol scope (from RFCs; standard, not repo-checkable — cited by section)

* **RTSP 1.0 — RFC 2326; RTSP 2.0 — RFC 7826.** RTSP is a *control*
  protocol, not a transport: the client issues `DESCRIBE` (retrieve SDP
  session description), `SETUP` (negotiate transport per track, get session
  id), `PLAY`/`PAUSE` (delivery control with RTP timestamps as
  start/stop ranges), `TEARDOWN` (end session). It is stateful — session ids,
  per-session timers, and 2.0 adds connection-token state. A correct client
  is a state machine, not a `GET`.
* **RTP — RFC 3550 (A/V profile RFC 3551).** Media arrives as RTP packets
  (sequence number, timestamp, SSRC, payload-type) plus RTCP companion
  traffic. Depacketization is *per-codec*: H.264 (RFC 6184, FU-A/NAL
  fragmentation, SPS/PPS in-band), H.265/HEVC (RFC 7798), Opus (RFC 7587),
  MPEG-4 AAC (RFC 3640 LATM), each with its own edge cases and jitter-buffer
  interplay. [VERIFY exact RFC numbers for the codecs actually wanted by the
  lead customer, if this is ever un-deferred.]
* **Interleaved vs UDP transport.** RFC 2326 §10.12: media can ride the RTSP
  TCP connection as `$`-framed interleaved frames (one 4-byte header:
  channel + length), or go over separate UDP port pairs (RTP even / RTCP
  odd) negotiated in `SETUP` `Transport:`.
* **NAT realities.** UDP-mode RTP/RTCP through consumer NAT is unreliable:
  symmetric NAT breaks the RTCP-learned address reflexion, pinholes close on
  idle, and multicast (`rtp://` to 224/4 or 239/4 style URLs) simply does
  not traverse NAT at all. De facto, internet-facing RTSP deployments
  converge on TCP interleaved transport. This argues that *if* we ever build
  RTSP, interleaved-only is the viable v1.

### Ecosystem (cannot source-check offline — all claims [VERIFY] before relying)

* **No mature pure-Rust RTSP client stack** that this port could adopt
  dependency-free:
  * `retina` (Scott Lamb / Moonfire NVR) — the closest thing to a real
    pure-Rust RTSP *client*: its own RTSP state machine and RTP
    depacketization, but H.264/JPEG-focused, single-maintainer cadence, and
    not a "stack" (no HEVC/AV1, no RTSP 2.0, auth surface partial).
    [VERIFY current crate version, maintenance status, codec list.]
  * `rtsp` / `librtsp`-family crates — thin partial implementations, demo
    quality or unmaintained. [VERIFY each crate's state; none is currently
    fit for a hardening-focused port.]
  * GStreamer/FFmpeg bindings provide RTSP but are C dependencies,
    contradicting the port's pure-Rust/no-unsafe posture.
* The port's own `NO deps` rule means even a good crate would need a
  vendored, audited, `no_std`-friendly adaptation before it could land.

### Cost if built (estimate)

Rough shape, each item ≈ a wave:

1. RTSP client state machine (DESCRIBE/SETUP/PLAY/TEARDOWN, session id +
   keepalive timers, SDP parse — can reuse the port's SDP work [VERIFY what
   the mp4/mpegts layers already expose]),
2. Interleaved TCP framing + RTP/RTCP header layer (RFC 3550 seq/ts/SSRC
   validation, reorder tolerance),
3. Per-codec depacketizers (start with the one codec a lead customer names;
   H.264 FU-A is the usual first),
4. UDP transport + NAT caveats — recommend **never** in v1.

≈ **multi-wave**, 3+ waves for a single-codec interleaved-only client, more
for a general one.

## Recommendation

1. **DEFER** — keep `rtsp://`/`rtp://`/`udp://` typed-refused via
   `SchemeError::Unsupported` (already the behavior: schemes.rs:20-24,
   560-563, 677-682). The typed refusal is the correct product surface: it
   names the scheme, carries no secrets, and lets callers branch.
2. Record the decision in the feature matrix blocked rows (done alongside
   this doc; see `docs/feature-matrix.toml`) and in
   `docs/formats/network-http.md` [VERIFY record updated].
3. Re-evaluation trigger: a **lead customer** needing camera or live
   low-latency input. At that point: interleaved-TCP-only, single-codec
   first, estimate per the cost model above.
4. Do **not** adopt C-linked RTSP (FFmpeg/GStreamer) even then — it would
   break the port's safety posture.

*Honest limitation: this investigation was performed offline; every
ecosystem claim above is marked [VERIFY] and should be re-checked against
crates.io/docs.rs before any un-deferral decision.*
