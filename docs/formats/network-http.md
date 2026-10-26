# Protocol record — HTTP(S) access + scheme registry (network stack)

Status: **implemented (library stack)** — the fetch port (6.A1a), the
admitted ureq client (6.A1b/6.A1c), the ranged seekable resource
(6.A2a) with the no-range linear-scan fallback (6.A2b), and the scheme
dispatch registry (6.A3) are landed and tested in `crates/streaming`.
**The CLI open path does not dispatch through the registry yet** — the
composition card (probe-over-network + product-limits join; the
`open-path-for-http` handoff named by 6.A2a/6.A2b) owns that wiring;
today every `SourceSpec::Url` still refuses at
`apps/cli/src/demux_host.rs` (`UnsupportedForHost`, "needs the
streaming crate"). Front-matter keys per doc 04 §6:

- id = `protocol-http-https` (the access row; `network-http-client`
  and `network-tls-policy` cover the client/TLS feature rows)
- status = implemented (library); platform-independent code, evidence
  Linux-only (mock-server tests + the two `--ignored` live-net tests)
- phase = 6 (waves 6.A1–6.A3)
- fuzz_target = `fuzz_scheme_classify` (in-crate
  `streaming::schemes::fuzz_scheme_classify` + deterministic sweeps;
  the cargo-fuzz target file stays the Lead-owned dependency decision
  — the ADTS/AVI/WAV precedent, recorded in each fuzz module)
- crates = [`ureq =3.5.0` (admission record
  `docs/dependencies/records/ureq-3.5.0.md`; rustls + ring tree and
  the Lead-ADR ring deviation live there)]
- self_built = [`crates/streaming/src/{fetch,error,client,ranged,
  schemes,mockhttp}.rs`]
- last_audit = 2026-10-31 (card 6.A3)

## What landed when (the stack this record covers)

| Card | Landed | Where |
|---|---|---|
| 6.A1a | Sync `HttpFetch` port (`HttpRequest`/`HttpResponse`, `Url`, ranges, deadlines, redirect policy, 8 MiB response budget), 16-class typed `NetworkError`, std-only mock HTTP/1.1 testkit | `fetch.rs`, `error.rs`, `mockhttp.rs` |
| 6.A1b | ureq 3.5.0 admission (minimal pin: rustls + webpki-roots, verification never off) + `UreqClient` adapter | `client.rs`, `docs/dependencies/records/ureq-3.5.0.md` |
| 6.A1c | Adapter-owned redirect following: cross-host policy, loop detection, manual mode, exact counts, agent pool | `client.rs` |
| 6.A2a | `RangedResource`: bounded byte-range windows over the port, serving `plugin_api::SourceStream` (e2e: wav-over-http demuxed byte-exact) | `ranged.rs`, `tests/ranged_wav_e2e.rs` |
| 6.A2b | No-range fallback: `NoRangePolicy::FallbackToScan` (default), mid-stream 200 resync, refetch accounting | `ranged.rs` |
| 6.A3 | **Scheme dispatch registry**: the single scheme → access-strategy point, URL parsing/normalization, credential-free error surfaces, this record | `schemes.rs` |

## Scheme registry (6.A3) — supported + refused

The registry (`SchemeRegistry::classify`) is uniform in code: `file`
and `http`/`https` dispatch to strategies, everything else is the
typed refusal `streaming.scheme.unsupported` naming the scheme. The
*policy* behind each refusal is this table's truth. The table is
machine-checked: `schemes.rs`'s `record_table_matches_the_registry`
re-parses it and cross-checks every row against the implementation.

<!-- scheme-registry-begin -->
| scheme | status | probe | policy / landed-when |
|---|---|---|---|
| `file` | supported | file:///media/a.wav | `LocalFile` strategy (6.A3: authority folding, percent-decoding); opened by `input`'s `FileAccess` (2.A1) at the composition site — same-layer L2, the app owns the join |
| `http` | supported | http://example.com/media/a.wav | the full stack above (6.A1a→6.A3); `open_http` = ureq client feeding `RangedResource` |
| `https` | supported | https://example.com/media/a.wav | same stack; rustls TLS, verification always on (A1b record R1/R10; the one `TlsConfig` site is test-pinned) |
| `ftp` | refused | ftp://example.com/media/a.wav | `streaming.scheme.unsupported` — **blocked v1** (card 6.A3 `[PROPOSAL: blocked v1]` decision below) |
| `sftp` | refused | sftp://example.org/media/a.wav | blocked v1 with ftp (matrix row `protocol-ftp-sftp`, P3) |
| `rtsp` | refused | rtsp://cam.example.org/stream.sdp | blocked pending 6.G1-I (payload = hw-only codec chain; async boundary; camera-quirk tail — matrix row + feasibility §2.3) |
| `rtp` | refused | rtp://224.1.1.1:5004 | blocked with the RTSP family (6.G1-I owns the registry rows; web: no raw sockets, MP §10) |
| `udp` | refused | udp://@239.255.12.42:1234 | blocked (raw-socket playback whose payload is hw-only; matrix row `protocol-udp`) |
| `data` | refused | data:audio/wav;base64,UklGRg== | no v1 card (unbounded in-line payloads vs MP §12 budgets; P3) |
| `gopher` | refused | gopher://example.org/0/media | no card, no demand (census-tail row) |
| `mms` | refused | mms://stream.example.org/live | blocked (legacy Microsoft streaming; matrix row `protocol-mmsh-mms`) |
| `mmsh` | refused | mmsh://stream.example.org/live | blocked with mms |
| `smb` | refused | smb://nas/share/a.wav | no v1 card (census-tail row) |
<!-- scheme-registry-end -->

### The ftp decision (card 6.A3 `[PROPOSAL: blocked v1]`)

Blocked for v1, recorded here and in the matrix row
`protocol-ftp-sftp` (status `blocked`, P3, phase 8): no pure-Rust FTP
*fetch* path has been assessed (suppa-ftp is the known candidate,
`[VERIFY]` when a card exists); the honest v1 position is the typed
refusal naming `ftp` over a guessed partial implementation. Revisit
trigger: user demand + a validate-then-admit card in the 6.A1b shape.

### Disambiguation rules (recorded with the registry)

- A **single ASCII letter before the first `:`** (`c:/x`, the Windows
  drive-letter shape) is a local path, not a scheme.
- A prefix that fails the RFC 3986 scheme grammar (`/media/x:y` — the
  colon sits inside a path segment) leaves the whole reference a local
  path. A *valid* scheme prefix (`name.tar:x`) is a URI per RFC 3986
  §4.2's first-segment rule and refuses typed as unsupported — the
  recorded cost of the rule.
- Bare paths pass through verbatim; canonicalization stays with the
  composition site (the existing CLI open path's job).

## Normalization (6.A3 "URL parsing/normalization")

- Schemes are matched ASCII case-insensitively, stored lowercase
  (RFC 3986 §3.1) — `FILE:///x`, `HTTP://x` normalize.
- `http`/`https`: the port's `Url::parse` — default ports (80/443)
  made explicit, path defaults to `/`, userinfo and IPv6 literals
  refused rather than mis-parsed (MP §8).
- `file:` RFC 8089 shapes fold: `file:/p` ≡ `file:///p`;
  `file://localhost/p` ≡ `file:///p`; the path is percent-decoded.
- A `file:` URI naming any other authority is the typed
  `streaming.scheme.file-host` refusal (a network-filesystem
  reference, not a local file).

## Credential-free logging (6.A3)

- `SchemeError` never embeds the source reference: `Display` is
  structured data (scheme name / static detail / authority host) plus
  the dotted code. No URI, no userinfo, no query.
- URI-shaped credentials never enter dispatch: the port's `Url::parse`
  rejects `user:pw@` outright; the refused error carries none of it.
- The log-facing URI form for composition sites is
  `media_types::ids::SourceUri::redacted` (MP §12): userinfo collapses
  to `***@`, `file:` paths keep only the final component,
  credential-shaped query values are masked.
- The transport vocabulary (`NetworkError`) is no-secrets by the same
  rule (6.A1a); `RangedResource` mints its `SourceUri` per source.

## Error paths (typed, each test-pinned)

| Condition | Code | Class |
|---|---|---|
| Scheme has no access strategy (ftp, rtsp, data, …) | `streaming.scheme.unsupported` | `SchemeError::Unsupported` |
| Malformed source (empty, control bytes, bad file URI, refused http URL) | `streaming.scheme.malformed` | `SchemeError::Malformed` (port's `NetworkError` in the `source` chain) |
| `file:` URI with a non-local authority | `streaming.scheme.file-host` | `SchemeError::RemoteFileHost` |
| Engine bridge (stage `Open`, scheme in `ErrorKind::Unsupported`'s format info) | same codes | `SchemeError::into_engine_error` |
| Transport-level failures (DNS/deadlines/TLS/redirects/416/bounds) | `streaming.net.*` (16 classes) | see `error.rs` |
| Ranged-resource failures (no-range policy, truncation, over-budget) | `streaming.ranged.*` | see `ranged.rs` |

## +FB (6.A3)

`fuzz_scheme_classify` (`streaming::schemes`): one pass = classify
one untrusted source-reference byte string; the contract is
absence-of-panic/hang/unbounded-allocation over arbitrary bytes
(64 KiB input ceiling, non-UTF-8 returns early). Proven by the
deterministic in-crate sweeps (bit-flip / truncation / splice families
over the grammar's whole seed surface — `fuzz_lite_mutations_never_
panic`, >300 passes per run). The cargo-fuzz target file remains the
Lead-owned `libfuzzer-sys` decision, exactly as the demux/codec cards
recorded.

## Gates (6.A3, exact)

Recorded in the 6.A3 worklog entry (this record's sibling): `cargo
test -p streaming`, `cargo clippy -p streaming --all-targets
--all-features -- -D warnings`, `cargo fmt -p streaming -- --check`;
live-net tests stay `--ignored` by default (the 6.A1b/6.A1c
precedent — network-up runs recorded there when performed).

## Honest gaps (6.A3)

- **CLI dispatch not wired**: the registry landed in `streaming`; the
  open path still refuses every URL at `demux_host.rs`. Wiring it is
  *not* a one-line join: `apps/cli` would gain the `streaming`
  dependency (pulling the ureq/rustls/ring tree into the CLI binary)
  and needs the `boundaries.toml` delta — `[crates.streaming]` has no
  entry yet (pre-existing since 6.A1a; `check-architecture` ownership
  is the Lead's) — plus the probe-over-network, product-limits and
  moov-at-end composition the A2a/A2b worklogs scoped to the
  open-path-for-http card. Recorded as the follow-up, not snuck in.
- Host-case normalization: the port's `Url` preserves host case
  (`HTTP://Example.com` → host `Example.com`); DNS is
  case-insensitive, servers see the original form. Recorded, not
  silently folded.
- Non-UTF-8 `file:` URI paths refuse typed (the bare-path arm passes
  OS strings through natively; only the URI grammar is UTF-8 here).
- Matrix rows (`protocol-http-https`, `network-http-client`) still
  read `not_started` with empty evidence — this record is that
  evidence; amendment recorded in-file for the Lead (the 5.REC-1
  convention; agents do not edit the matrix).
- Live-network evidence: the two `--ignored` live tests (example.com,
  httpbin redirect chain) were last run green by 6.A1c; 6.A3 added no
  new live test (the registry's dispatch surface is fully mock- and
  table-driven — the composition card's e2e will re-exercise the wire).
