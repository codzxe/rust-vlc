# AUDIT: crates/streaming/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-12 |
| Lines / functions | 74 (comments 1-27, [package] 18-26, [lints] 25-26, [dependencies] 28-52, [dev-dependencies] 54-74) |
| File verdict | REAL (with stale self-description and one half-false reuse claim) |
| Reachable from shipped entry point? | YES — lumen-cli depends on this crate (apps/cli/Cargo.toml:62) and builds it in the release lane (baseline cargo-build-release-lumen-cli.txt:201 "Compiling streaming v0.1.0"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | full read; rg (boundaries.toml, Cargo.lock ureq/ring, lumen-cli dep); sed -n; baseline citations |

## 1. One-paragraph verdict
The manifest's substantive security/correctness claims all verify: ureq is pinned exactly (`=3.5.0`, L52) with `default-features = false, features = ["rustls", "rustls-webpki-roots"]`, and Cargo.lock confirms ureq 3.5.0 resolving rustls + webpki-roots with NO native-tls, NO socks/gzip/brotli/cookies deps — the "raw bytes preserve range/Content-Length semantics" posture is real at the lockfile level, and ring is exactly 0.17.14 as the comment states (Cargo.lock:749-751). Dev-dependencies (testkit/demux/input/codecs) are dev-only and match boundaries.toml's registered dev_deps. The defects are documentation-level: the [package] description still says the crate is a "std-only mock HTTP/1.1 testkit ... real client lands in 6.A1c" while the real client (client.rs + the ureq edge in this same file) has landed; the C5 header line "std-only, zero external dependencies" is contradicted by the ureq edge one screen below (rescued only by a clarifying comment); the "no second M3U parser" REUSE claim is half-false (master playlists fork a line parser in playlists/src/hls.rs:447-528); and the "boundaries.toml registration is Lead-owned at wave close" note is stale — the registration exists (boundaries.toml:213-218).

## 2. What actually works (verified)
— verified by Cargo.lock: `[[package]] name = "ureq" version = "3.5.0"` with dependencies exactly `base64, log, percent-encoding, rustls, rustls-pki-types, ureq-proto, utf8-zero, webpki-roots` (no native-tls, no gzip/brotli, no cookies/socks) — the minimal-feature claim (L46-48) is lockfile-true. `ring 0.17.14` (Cargo.lock:749-751) matches L50-51.
— verified by baseline: release lane compiles this crate (cargo-build-release-lumen-cli.txt:201), clippy --all-features CLEAN.
— verified by boundaries.toml:213-218: `internal_deps = ["media-types", "plugin-api", "playlists"]`, `dev_deps = ["testkit", "demux", "input", "codecs"]`, `external_allowlist = ["ureq"]` — every dependency edge in this manifest is registered.

## 3. Findings
F-001 [S3]
Where: crates/streaming/Cargo.toml:36-42
Code:
```
# playlists: workspace-internal (card 6.B1a) — the HLS session types
# the playlist through the 4.D1/5.O1a M3U parser's HLS model instead
# of forking a second M3U parser (the wave plan's REUSE rule). Pure
```
What is wrong: the REUSE claim is half-false. Media playlists do reuse `m3u::parse_detailed` (playlists/src/hls.rs:223 via `parse_media_playlist`), but master playlists (the playlist-of-playlists case) are parsed by a second, hand-rolled line loop in playlists/src/hls.rs:447-528 (`parse_master_playlist`: its own `text.lines()` scan, its own `split_attr_list`, its own tag dispatch, its own `#EXTM3U` detection). boundaries.toml:207-208 repeats the same "one parser, no fork" claim. Two M3U line-parsers exist; the fork is real for the master case. Why it matters: the comment is the recorded justification for the dependency edge, and it misdescribes the architecture it justifies — divergence between the two scanners (e.g. line-cap enforcement, BOM/CRLF tolerance) is invisible to anyone trusting this note. What correct looks like: amend the comment to "media playlists reuse the M3U parser; master playlists have their own row scanner (recorded)". Confidence: HIGH — Verification: read of both parser entry points.

F-002 [S4]
Where: crates/streaming/Cargo.toml:21-22
Code:
```
description = "Lumen network streaming layer: sync HttpFetch port, typed network errors, and a std-only mock HTTP/1.1 testkit (card 6.A1a; real client lands in 6.A1c)."
```
What is wrong: the description is stale — the real client HAS landed (client.rs, 990+ lines, and the ureq dependency at L52 of this same file), the ranged resource (6.A2a/A2b) and HLS/DASH sessions exist. Anyone reading the manifest description believes this crate has no real HTTP client. Confidence: HIGH — Verification: read + client.rs existence.

F-003 [S4]
Where: crates/streaming/Cargo.toml:10-11
Code:
```
# C5 discipline: std-only, zero external dependencies. Sync-only per
# doc 01 §4.2 (engine core is threads + channels — no async runtime).
```
What is wrong: "std-only, zero external dependencies" is false in the same file (ureq edge, L52). The rescue at L30-33 ("the 'zero external dependencies' discipline is about crates.io — ureq stays the only external dep") shows the intent, but the headline sentence as written contradicts the manifest body; the registry note should have been rewritten when ureq was admitted. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/streaming/Cargo.toml:39-41
Code:
```
# of forking a second M3U parser (the wave plan's REUSE rule). Pure
# text-in/text-out, no I/O, no new externals. NOTE: boundaries.toml
# [crates.streaming] internal_deps registration is Lead-owned at wave
```
What is wrong: the note says the boundaries.toml registration is pending ("Lead-owned at wave close") — but boundaries.toml:213-218 already registers internal_deps/dev_deps/external_allowlist for streaming. Stale note; a reader would conclude the arch gate is unaddressed when it is closed (the still-open ARCH-2 edge is lumen-cli's own entry, not this crate's). Confidence: HIGH — Verification: boundaries.toml read.

## 4. Section table (74-line manifest)
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Header comments | 1-16 | REAL | S4 | F-003 stale C5 line; history otherwise accurate |
| [package] | 18-23 | REAL | S4 | F-002 stale description; publish=false good |
| [lints] | 25-26 | REAL | — | workspace lints (deny unwrap/panic) — clippy CLEAN per baseline |
| [dependencies] media-types/plugin-api | 34-35 | REAL | — | registered in boundaries |
| [dependencies] playlists | 42 | REAL | S3 | F-001 half-false REUSE comment; dep itself sound |
| [dependencies] ureq | 43-52 | REAL | — | exact pin, minimal features — lockfile-verified; TLS = rustls, never native-tls |
| [dev-dependencies] | 54-74 | REAL | — | all dev-only, reasons documented, boundaries-registered |

## 5. Test honesty
N/A (manifest). The dev-dep reasons (demux for the wav e2e, input for byte-parity, codecs for the HLS decode differential) match real test files (tests/ranged_wav_e2e.rs, hls_*_e2e.rs — all baseline-passing per PACKAGE streaming results).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ureq 3.5.0 ... Minimal feature set: rustls TLS with the compiled-in Mozilla root bundle — NO native-tls, NO socks-proxy, NO gzip/brotli, NO cookies/charset/json/multipart" | Cargo.toml:43-48 | REAL | Cargo.lock ureq 3.5.0 dep list (rustls, webpki-roots; none of the excluded features resolve) |
| "the rustls crypto provider is ring 0.17.14 (C + asm compiled by cc)" | Cargo.toml:50-51 | REAL | Cargo.lock:749-751 ring 0.17.14 |
| "NO gzip/brotli (raw bytes preserve range/Content-Length semantics)" | Cargo.toml:47-48 | REAL | no flate2/brotli in ureq's lock deps; client.rs reads raw bodies |
| "real client lands in 6.A1c" (as future) | Cargo.toml:22 | FALSE (stale) | client.rs exists; ureq is a current dependency |
| "instead of forking a second M3U parser (REUSE rule)" | Cargo.toml:37-38 | HALF-FALSE | F-001: hls.rs:447-528 is a second line parser for master playlists |
| "C5 discipline: std-only, zero external dependencies" | Cargo.toml:10 | FALSE (as written) | F-003: ureq edge in the same file |

## 7. Dead / duplicated / unreachable code
No dead deps: every production dep (media-types, plugin-api, playlists, ureq) is used by shipped-reachable code (ranged/schemes/client/hls all reachable from lumen-cli's http arm except hls, which is compiled but not entered — see m3u.rs audit for that lane). No unused dev-deps (each names its e2e suite).

## 8. Dependencies & flags
Exactly one external (ureq, allowlisted); three internal (registered); four dev (registered). Feature set is minimal and lockfile-verified. `edition = "2024"`, `publish = false`.

## 9. Open questions for the next agent
- The ARCH-2 edge in the baseline gate ("lumen-cli depends on streaming, which is not in [crates.lumen-cli].internal_deps", full_gate.txt:43) — who backfills apps/cli's boundaries entry?
- Does ureq's `rustls` feature without `rustls-no-provider` guarantee the ring provider is active at runtime (provider installed by default)? The client.rs test pins the config; a runtime handshake remains networked-CI-only (https_activation.rs admission).
