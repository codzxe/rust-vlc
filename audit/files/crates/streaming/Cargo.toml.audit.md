# AUDIT: crates/streaming/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A01 |
| Date | 2026-11-21 |
| Lines / functions | 74 / config keys: 4 dep groups (package, lints, dependencies×4, dev-dependencies×4) |
| File verdict | REAL (manifest matches lock + boundaries; header comments drifted) |
| Reachable from shipped entry point? | YES (call chain: `apps/cli/src/demux_host.rs:491` → `streaming::schemes::SchemeRegistry::open_http` → `schemes.rs:63 use crate::client::UreqClient` — the ureq transport is linked into the shipped `lumen` binary) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `grep -n streaming apps/cli/Cargo.toml` (dep declared :62); `rg streaming apps/cli/src` (demux_host.rs:383,476-492 real usage); `rg -l 'plugin_api|media_types|playlists' crates/streaming/src` (all three internal deps used); `grep Cargo.lock` (ureq 3.5.0 ×1, ring 0.17.14, rustls 0.23.46 ×1, native-tls/flate2/socks ×0); boundaries.toml:213-218 diffed against manifest (exact match); baseline INDEX #1 (workspace build clean), #11 (arch gate violation is on the lumen-cli side, not this manifest) |

## 1. One-paragraph verdict
A clean, conservative manifest: one exact-pinned external (ureq `=3.5.0`, default-features off, rustls+webpki-roots only), three internal path deps, four dev-only path deps, all four groups matching `boundaries.toml [crates.streaming]` field-for-field and the lockfile (single ureq 3.5.0, single rustls 0.23.46, ring 0.17.14 exactly as the comment discloses, zero native-tls/flate2/socks entries). The ureq admission record (`docs/dependencies/records/ureq-3.5.0.md`) exists as cited. The defects are comment-level: the file's own header still says "std-only, zero external dependencies" and describes the crate as v0-port-only while the crate now ships ureq, HLS, DASH, reconnect, auth, and clock modules.

## 2. What actually works (verified)
— Dep/boundaries consistency: manifest `[dependencies]` (media-types, plugin-api, playlists, ureq) == boundaries.toml:215/217 `internal_deps`+`external_allowlist`; `[dev-dependencies]` (testkit, demux, input, codecs) == boundaries.toml:216 `dev_deps`. Verified by side-by-side read.
— Pin honesty: `ureq = { version = "=3.5.0", ... }` and Cargo.lock:1214-1215 `name = "ureq" / version = "3.5.0"` (exactly one ureq entry). ring 0.17.14 in lock matches the comment's disclosed deviation.
— Feature-minimality claims cross-checked against lock: 0 hits each for `native-tls`, `flate2` (gzip), `socks`.
— Reachability: `apps/cli/src/demux_host.rs:476-492` calls `streaming::ranged::RangedResource::open` / `SchemeRegistry::open_http`; `schemes.rs:63` wires `UreqClient`. Verified by rg.

## 3. Findings
### F-001 [S4]
Where: crates/streaming/Cargo.toml:3-13
Code:
```
# Registry: the `streaming` crate of doc 01 §2.4 (layer L2). v0 scope is
# the *port* only: the sync `HttpFetch` trait + request/response types,
# the typed network error vocabulary, and a std-only mock HTTP/1.1
# testkit. The real HTTP client (ureq admission, 6.A1b/6.A1c), ranged
# seekable resources (6.A2), HLS/DASH (6.B/6.C) and RTSP (6.G) land on
# top in later cards.
#
# C5 discipline: std-only, zero external dependencies.
```
What is wrong: The header is stale on both counts. (a) "zero external dependencies" is false in the same file (line 52, ureq) and is walked back only in a later comment (lines 30-31) most readers of the header will never reach. (b) "v0 scope is the port only … The real HTTP client (ureq …), ranged seekable resources (6.A2), HLS/DASH (6.B/6.C) … land on top in later cards" — those cards landed: `src/` contains `client.rs` (ureq adapter), `ranged.rs`, `hls/`, `dash/`, plus `reconnect.rs`, `auth.rs`, `clock.rs`, `buffer.rs`, `remote_playlist.rs` that the header doesn't mention at all.
Why it matters: The manifest is the first thing a reviewer/auditor reads; it under-describes the crate's real surface, and the "std-only" claim contradicts the linked-in ring C/asm TLS stack (the crate is not std-only and has FFI-adjacent code via ring).
What correct looks like: Header rewritten to current scope (port + ureq transport + ranged + HLS/DASH + reconnect), with the C5 discipline restated as "one admitted external (ureq), rustls/ring disclosed".
Confidence: HIGH — Verification: read (ls crates/streaming/src; sed quotes).

### F-002 [S4]
Where: crates/streaming/Cargo.toml:39-41 vs boundaries.toml:512-517
Code:
```
# text-in/text-out, no I/O, no new externals. NOTE: boundaries.toml
# [crates.streaming] internal_deps registration is Lead-owned at wave
# close (same posture as the A3 manifest note).
```
What is wrong: The "Lead-owned at wave close" TODO posture for boundaries registration is resolved for streaming's own outgoing edges (boundaries.toml:213-218 lists playlists), but the consuming edge is still unregistered: `[crates.lumen-cli] internal_deps` (boundaries.toml:514) omits `streaming` even though apps/cli/Cargo.toml:62 depends on it — the exact violation baseline run #11 reports (`check-architecture` FAILURE, exit 1). The manifest's TODO note is silent about that side of the registration.
Why it matters: The arch gate is failing at HEAD; a manifest note implying registration posture is handled draws attention away from the open violation.
What correct looks like: Note updated to name the still-open lumen-cli→streaming registration (or the boundaries entry fixed by its owner, agent A05's file).
Confidence: HIGH — Verification: read (boundaries.toml:512-517 diffed vs apps/cli/Cargo.toml:62; baseline INDEX #11 cited).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package] name/version/edition/description/publish` | 18-23 | REAL | — | description matches actual scope (mentions 6.A1c) — unlike the header comment (F-001). publish=false, edition 2024 consistent with workspace. |
| `[lints] workspace = true` | 25-26 | REAL | — | Inheritance verified present in all 39 members (loop-grep, file-1 audit). |
| `[dependencies] media-types` | 34 | REAL | — | Path dep; used (rg: error.rs, ranged.rs, dash/*, hls/*). |
| `[dependencies] plugin-api` | 35 | REAL | — | Used (rg: hls/master.rs, media.rs, live.rs, schemes.rs, ranged.rs). |
| `[dependencies] playlists` | 42 | REAL | — | Used (hls/*, dash/*, remote_playlist.rs, mockhttp.rs, client.rs…). Same-layer pair `streaming->playlists` whitelisted (boundaries.toml:45). |
| `[dependencies] ureq` | 52 | REAL | — | Exact pin; lock-verified single 3.5.0; features minimal (rustls, rustls-webpki-roots), default-features=false; no native-tls/flate2/socks in lock. Admission record exists. |
| `[dev-dependencies] testkit` | 57 | REAL | — | dev-only by contract (boundaries dev_only list); matches boundaries dev_deps. |
| `[dev-dependencies] demux` | 62 | REAL | — | dev-only; matches boundaries; justified (e2e WAV-over-mock-HTTP proof). |
| `[dev-dependencies] input` | 67 | REAL | — | dev-only; matches boundaries; justified (byte-parity file-vs-network proof). |
| `[dev-dependencies] codecs` | 74 | REAL | — | dev-only; matches boundaries; justified (HLS decode differential vs ffmpeg oracle). |

## 5. Test honesty
No tests in this file (manifest). The dev-dep rationale comments name specific e2e proofs (WAV-over-mock-HTTP demux parity, HLS AAC decode differential) — those live in streaming's own test suite; baseline #3 (1681 passed) covers them at workspace level; per-test honesty belongs to the streaming crate's source audit (other agents).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "std-only, zero external dependencies" | streaming/Cargo.toml:10 | FALSE | ureq at :52; ring 0.17.14 (C/asm) in lock via rustls. Walked back at :30-31 — internal contradiction. |
| "ureq … exact pin … rustls TLS, Mozilla root bundle, NO native-tls/socks/gzip" | streaming/Cargo.toml:43-52 | REAL | Lock: ureq 3.5.0 ×1, rustls 0.23.46, native-tls/flate2/socks ×0; record docs/dependencies/records/ureq-3.5.0.md exists. |
| "ring 0.17.14" | streaming/Cargo.toml:50-51 | REAL | Cargo.lock ring 0.17.14. |
| "HLS/DASH … land on top in later cards" | streaming/Cargo.toml:6-8 | Stale (REAL but misleading as future tense) | `crates/streaming/src/hls/` and `dash/` exist with mod declarations (lib.rs:32,35). |

## 7. Dead / duplicated / unreachable code
None in the manifest itself. All four deps and four dev-deps are referenced by streaming/src (rg). No unused-dep candidates found from manifest inspection.

## 8. Dependencies & flags
No `[features]` table — the crate is unconditionally compiled with its full surface; nothing feature-gated here. No build-dependencies. Workspace membership via `crates/*` glob.

## 9. Open questions for the next agent
- boundaries.toml:28 reads `anifest]` — an apparently corrupted `[manifest]` header. If the xtask TOML subset reader parses this file without error (baseline #11 produced an edge violation, not a parse failure), the reader tolerates malformed lines it claims to "reject loudly" (header comment at boundaries.toml:28-29). Cross-file finding for A05 (boundaries.toml) and W17-A02 (check_architecture.rs) — recorded in W01-A01 log.
- Is `streaming::mockhttp` (a real-socket mock HTTP server, demux_host.rs:869-881) compiled into release builds of the CLI or test-gated? `mockhttp` is a non-dev, public module (lib.rs:36) — shipping a mock server in the production binary would be worth a finding in the streaming source audit.
- RTSP (6.G) is promised in the header; no rtsp module exists yet — feature-matrix rows referencing RTSP should be checked by the matrix-rows agents.
