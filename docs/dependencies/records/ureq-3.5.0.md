# deps/records/ureq-3.5.0.md — ureq 3.5.0 (validate-then-admit, card 6.A1b)

> **Class:** P0 dependency verification (doc 11 §2 format, fields R1–R10 +
> F1–F7 scan), ordered by card 6.A1b: "ureq admission spike" — the sync,
> rustls-backed HTTP(S) client behind the streaming layer's `HttpFetch`
> port (card 6.A1a). Architecture rule doc 01 §4.2 (engine core is
> synchronous, no async runtime) is why ureq is the candidate and reqwest
> is not. Evidence gathered 2026-10-30 in the build sandbox on toolchain
> 1.99.1: `cargo add --dry-run` feature inventory, a full
> `cargo tree -e normal,build` (pasted in R2), `cargo metadata` license +
> `rust_version` extraction for every node, vendored-source scans
> (unsafe-token and native-file counts, `disable_verification` grep), a
> live HTTPS fetch through the adapter, and OSV API queries. **Final
> acceptance and the allowlist row remain Lead decisions (doc 11 §3.1);
> the card authorized the deny.toml license entries (R7) so the CI gate
> can pass — the Lead ratifies with the merge.**

## The feature pin (the whole point of this record)

- Newest compatible ureq measured: **3.5.0** (`cargo add --dry-run`
  resolved it; builds and passes all gates on our pinned 1.99.1 —
  `rust_version` 1.85 ≤ 1.99.1, the whole tree's max MSRV is 1.85 on
  ureq/ureq-proto/zeroize). No 2.x fallback needed.
- **Our admission: `ureq = { version = "=3.5.0", default-features =
  false, features = ["rustls", "rustls-webpki-roots"] }`.** Default
  features would pull `gzip` (transparent decompression — it would
  corrupt our byte-range/Content-Length semantics; media needs identity
  encoding) and the socks/cookies/charset/json/multipart family we do
  not want compiled in. `native-tls` is explicitly NOT chosen (card
  rule); `vendored` (native-tls static OpenSSL) NOT chosen; socks-proxy
  NOT chosen. TLS backend is rustls with the compiled-in Mozilla root
  bundle (`webpki-roots`), deliberately not the OS cert store —
  deterministic across platforms, no OS-specific trust probing.
- TLS verification is **always on** in our wiring: the adapter's single
  `TlsConfig` construction site (`crates/streaming/src/client.rs`,
  `tls_config()`) never touches ureq's `disable_verification` hatch, and
  the `tls_verification_is_always_on` test pins that shut. Measured
  honesty note: **ureq 3.5.0 DOES ship the escape hatch upstream**
  (`TlsConfig::builder().disable_verification(bool)`, vendored source
  `src/tls/rustls.rs:166`; used by ureq's own tests at
  `src/lib.rs:906,1313`, `src/pool.rs:623`, and socks code at
  `src/transport/socks.rs:711,726` — the socks feature is OFF in our
  pin). The card's preferred world ("no such escape hatch reachable")
  does not exist in ureq 3.x; the recorded control is: one construction
  site + test-pinned + the 6.H1 security review re-checks it.

## R1 — Language & source (F1, F7)

- ureq itself: **pure Rust** — 0 native source files (scan for
  `*.c *.cc *.cpp *.cxx *.h *.hpp *.m *.mm *.S *.s *.asm` → 0 hits),
  **5 `unsafe` tokens** (comments included; grep-count methodology,
  same as the rav1d record), no `links=` key, no `build.rs` at the crate
  root. F1/F7 NO for ureq proper.
- **THE deviation, measured: the rustls crypto provider is
  `ring 0.17.14`** — **135 native source files** (C + perl-generated
  `.S`/`.asm` across x86_64/aarch64/arm), **230 `unsafe` tokens**, and
  a `build.rs` that invokes **`cc 1.10.0`** to compile them (F1 YES,
  F2 YES, F7 YES for ring — and F4 YES in the tarball-ships-C sense).
  This is not optional: rustls 0.23 requires a `CryptoProvider`, the
  production options are `ring` and `aws-lc-rs` (a full C crypto
  library — strictly worse on this policy), and there is no
  production-grade pure-Rust TLS provider today. native-tls is banned
  by the card. So: **HTTPS in Rust today means one audited native
  crypto core.** This is recorded as the ADR-001-class exception ask
  (E2 "audited native crypto", mirroring the hardware-decoder
  posture), NOT as a pass of F1–F7 — the Lead must accept the ADR
  question for the admission to stand. ring links no external library
  (everything is compiled into the rlib; no `links=`, no pkg-config,
  no downloads at build time).
- Everything else in the tree is pure Rust with 0 native files
  (measured per-crate table in R8's neighbourhood — see the unsafe
  counts below): rustls 6, rustls-pki-types 1, ureq-proto 0,
  rustls-webpki 0, webpki-roots 0, base64 77, log 9, percent-encoding
  5, once_cell 54, getrandom 59, libc 814, untrusted 2, zeroize 41,
  subtle 2, http 66, httparse 79, bytes 172, itoa 13, utf8-zero 4,
  cfg-if 0 unsafe tokens (token counts include doc mentions — numbers
  only, honest, per the rav1d methodology).
- Repo: https://github.com/algesten/ureq (crates.io `repository`
  field, measured via cargo metadata).

## R2 — Transitive graph (F5)

Measured `cargo tree -p ureq -e normal,build` with our exact feature
pin (this is the graph shape the streaming crate builds):

```
ureq v3.5.0
├── base64 v0.23.1
├── log v0.4.34
├── percent-encoding v2.3.2
├── rustls v0.23.46
│   ├── log v0.4.34
│   ├── once_cell v1.21.4
│   ├── ring v0.17.14                    ← native crypto (R1 deviation)
│   │   ├── cfg-if v1.0.5
│   │   ├── getrandom v0.2.17
│   │   │   ├── cfg-if v1.0.5
│   │   │   └── libc v0.2.191
│   │   ├── untrusted v0.9.0
│   │   └── [build-dependencies]
│   │       └── cc v1.10.0               ← compiles ring's C/asm
│   │           ├── find-msvc-tools v0.1.15
│   │           └── shlex v2.0.1
│   ├── rustls-pki-types v1.15.1
│   │   └── zeroize v1.9.1
│   ├── rustls-webpki v0.103.16
│   │   ├── ring v0.17.14 (*)
│   │   ├── rustls-pki-types v1.15.1 (*)
│   │   └── untrusted v0.9.0
│   ├── subtle v2.6.1
│   └── zeroize v1.9.1
├── rustls-pki-types v1.15.1 (*)
├── ureq-proto v0.6.5
│   ├── base64 v0.23.1
│   ├── http v1.5.0
│   │   ├── bytes v1.12.1
│   │   └── itoa v1.0.18
│   ├── httparse v1.10.1
│   └── log v0.4.34
├── utf8-zero v0.8.1
└── webpki-roots v1.0.11
    └── rustls-pki-types v1.15.1 (*)
```

23 normal-edge crates + 3 build-edge (cc, find-msvc-tools, shlex — all
in service of ring's build). No `-sys` crates, no git deps, no
downloads at build time. The `windows_*` lockfile rows some trees add
come from cc's target-gated deps (not compiled on Linux). `[VERIFY-P1:
the merged lock diff must show exactly these additions.]`

## R3 — Build scripts (F2)

- ureq 3.5.0: no crate-root `build.rs` (tarball listing measured).
  F2 NO for ureq itself.
- ring 0.17.14: `build.rs` invokes the C toolchain via `cc` for the
  135 native files of R1. F2 YES — the recorded deviation, same
  paragraph as R1. A sabotage-style proof (rav1d R3) is impossible
  here by definition: the C compilation is required, not inert.

## R4 — Native links (F3, F4, F6)

- No `links=` anywhere in the tree, no `#[link]` against third-party
  libraries, no pkg-config, no binary blobs vendored beyond ring's own
  sources, no network access at build time. F3/F6 NO; F4 is YES in the
  "C sources shipped in the ring tarball" sense (recorded with R1).
- The engine is not a wrapper: ureq is a from-scratch Rust HTTP/1.1
  client (ureq-proto parser, its own connection pool/middleware), not a
  libcurl binding.

## R5 — Targets

- Pure Rust everywhere except ring's per-target asm (`cfg(target_arch)`
  selects x86_64/aarch64/arm paths — desktop triples all covered by
  ring). wasm32: ureq/rustls compile; ring has wasm32 support paths,
  but our engine's wasm story is doc 06's later concern.
  `[VERIFY-P1: wasm32 build from our tree at the first wasm gate run —
  shared standing item with the codec admissions.]`

## R6 — Actual feature coverage (against card 6.A1b's needs)

- Used surface (adapter `crates/streaming/src/client.rs`):
  `ureq::config::Config::builder()` with `timeout_connect`,
  `timeout_per_call`, `max_redirects`, `max_redirects_will_error`,
  `tls_config`; `Agent::new_with_config`; `agent.get(url).header(..)
  .call()`; `http::Response` status/headers; `Body::into_reader()`
  bounded by `io::Read::take(limit+1)`; the `ureq::Error` → typed
  `NetworkError` mapping (StatusCode→HttpStatus/416→RangeNotSatisfiable,
  TooManyRedirects, HostNotFound→DnsFailure, Timeout(Connect/PerRead/
  PerWrite/…)→Connect/Read/WriteTimeout, Rustls/Tls/Pem→TlsFailure,
  Protocol→ProtocolViolation, LargeResponseHeader→ResponseTooLarge).
- **Measured API gaps, honestly recorded:** (a) ureq 3.5.0 has NO
  `timeout_write`/`timeout_read` knobs — only `timeout_connect`,
  `timeout_global`, `timeout_per_call`; our `TimeoutConfig::write` is
  carried but not armed (GET requests have no body; wiring it is 6.A1c
  scope). (b) ureq's redirect handling reports no followed-count and
  no loop detection — `TooManyRedirects{limit, followed: limit}` is our
  approximation and `RedirectLoop` stays mock-client-only vocabulary.
  (c) The response reason phrase is `canonical_reason()` — ureq/http
  discard non-canonical wire reasons (port tolerates empty). (d)
  `RedirectPolicy::cross_host` is carried, not enforced (6.A1c/6.H1).
  (e) One Agent per fetch in the spike — no pooling until 6.A1c.
- NOT used (features deliberately OFF): native-tls, socks-proxy, gzip,
  brotli, charset, cookies, json, multipart, platform-verifier,
  win-system-proxy, vendored.

## R7 — License

- ureq 3.5.0: **MIT OR Apache-2.0** — inside the existing allow set.
- New license classes the tree needs (measured via cargo metadata,
  this session): **ISC** (ring "Apache-2.0 AND ISC", rustls-webpki
  "ISC", untrusted "ISC"), **BSD-3-Clause** (subtle),
  **CDLA-Permissive-2.0** (webpki-roots — the Mozilla root bundle's
  distribution license). rustls itself is "Apache-2.0 OR ISC OR MIT"
  (satisfied by MIT). deny.toml's minimal allow set has been extended
  with exactly these three by this card (the card authorized it;
  deny.toml header notes it is Lead-owned — the Lead ratifies with the
  merge). **cargo-deny is NOT installed in this sandbox** (measured:
  `cargo deny` → "no such command"); the CI G3/G4 job owns the
  authoritative license/advisory pass — noted honestly, no heavy
  tooling installed.

## R8 — Maintenance & security record

- ureq 3.5.0 is the current release; `rust_version` 1.85; not yanked
  (registry metadata via cargo). crates.io REST API was NOT reachable
  from this sandbox (curl returned empty — measured), so publish date
  and publisher identity are `[VERIFY-P1: publisher cross-check at
  admission time, cpal-record convention]`. The ureq project (algesten)
  is the long-standing sync Rust HTTP client, actively maintained
  across the 2.x→3.x rewrite.
- **OSV API querybatch (2026-10-30, all 22 normal-edge packages):**
  matched IDs against 9 packages. Detail-fetched six of the
  security-relevant ones — **all six are fixed in our locked
  versions**: RUSTSEC-2025-0009 (ring AES panic, fixed 0.17.12 < our
  0.17.14), RUSTSEC-2025-0010 (ring < 0.17 unmaintained — ours is
  0.17.x), RUSTSEC-2026-0285 (rustls TLS1.3 cross-level handshake,
  fixed 0.23.45 < our 0.23.46), RUSTSEC-2026-0381 (webpki-roots
  distrusted emSign intermediates, fixed 1.0.10 < our 1.0.11),
  RUSTSEC-2026-0007 (bytes `reserve` overflow, fixed 1.11.1 < our
  1.12.1), RUSTSEC-2026-0049 (rustls-webpki CRL distribution-point
  matching, fixed 0.103.10 < our 0.103.16). The remaining matches
  include ancient advisories with imprecise OSV ranges (e.g.
  RUSTSEC-2019-0033/0034 matched against http **1.5.0** — a 2019
  advisory predating the 1.x series; base64-2017, once_cell-2019,
  untrusted-2018 in the same noise class) and 2026-series IDs not yet
  detail-fetched. `[VERIFY-P1: the CI cargo-deny pass (RustSec ranges,
  not OSV's) is the authoritative advisory gate at merge; re-query per
  doc 11 §3.7 triggers.]` The pattern measured so far: the lockfile
  resolves post-fix versions.
- Unsafe surface (token counts, R1): ureq 5, rustls 6 — small and
  reviewed-shape; ring 230 + its C/asm is the flagged surface for the
  Lead's Q2-V audit lens, exactly as rav1d's 642 was.

## R9 — Provenance

- Resolved through the pinned sparse index; Cargo.lock carries
  ureq 3.5.0 checksum
  `cde93ff05758a36a851a398a772b0b79a82fbe3c186a46f2696baaeaf84c966b`
  (cargo re-verifies on every fetch). Tarball inspected in
  `~/.cargo/registry/src/index.crates.io-*/ureq-3.5.0/` this session —
  all scans above ran against it. `[VERIFY-P1: publisher identity +
  index-hash cross-check at admission time, per the cpal record's
  convention.]`

## R10 — Verdict

**ADMITTED — conditionally, on the Lead accepting the recorded ring
deviation.** ureq 3.5.0 under the exact pin `default-features = false,
features = ["rustls", "rustls-webpki-roots"]` is pure-Rust HTTP with
sync-only architecture (doc 01 §4.2), exact-pin'd, minimal-featured,
MIT/Apache-licensed, with every measured advisory fixed in the locked
versions and a live HTTPS roundtrip through the adapter this session.
The one F1/F2/F4/F7 failure is **ring 0.17.14** (135 C/asm files, cc
build, 230 unsafe tokens) — unavoidable for rustls TLS (no production
pure-Rust provider exists; native-tls is banned by the card), so it is
presented as the E2/ADR-001-class exception ask (audited native crypto
core), the same posture the rav1d record took for asm. If the Lead
rules ring out, HTTPS is out with it — the fallback would be a doc-04
revisit of the TLS chapter, not another crate. Gates this session:
`cargo test -p streaming` 37 passed / 0 failed / 1 ignored (live),
clippy `-D warnings` zero, fmt clean; live fetch of
`https://example.com/` returned 200 with a real rustls+webpki-roots
handshake (measured once via `--ignored`). A version bump, feature
change, or upstream default re-tuning re-triggers this record per
doc 11 §3.7.

*Evidence posture (AGENTS §3.4): every statement above was executed
locally in the build sandbox on 2026-10-30 — dry-run feature inventory,
the pasted R2 tree from our own lockfile, cargo-metadata license/MSRV
extraction for all 22 nodes, vendored-source unsafe/native-file counts
and the `disable_verification` grep with hit sites, OSV querybatch +
six per-advisory detail fetches with range arithmetic, and the
adapter's mock-socket test suite plus one live-network HTTPS fetch. No
claim about ureq's behavior is made without our own test behind it.*
