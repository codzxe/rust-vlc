# AUDIT: crates/streaming/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-02-14 |
| Lines / functions | 40 / 0 (crate root: 13 `pub mod` declarations + docs) |
| File verdict | REAL (structure) / module docs STALE (two false statements) |
| Reachable from shipped entry point? | YES — the crate is linked by apps/cli (demux_host.rs uses `streaming::schemes/fetch/ranged`); this root declares every module |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | module/file existence check (ls vs `pub mod` list); rg for RTSP/ureq/response_limit; streaming lib tests already run this session (reconnect 20, dash 27, all green) |

## 1. One-paragraph verdict
The file is a thin crate root: 13 public module declarations, each backed by a real file (verified: auth, buffer, client, clock, dash/, error, fetch, hls/, mockhttp, ranged, reconnect, remote_playlist, schemes — all present). The binding claims it makes are mostly verifiable: the bounded-response contract is real (`response_limit` + `NetworkError::ResponseTooLarge` + `with_response_limit` exist in fetch.rs:373-460), the sync/no-async-runtime posture holds (no tokio/async in the dep tree; ureq is sync). Two statements are stale to the point of falsehood: the "Scope of this scaffold" paragraph says the real HTTP client (6.A1b/c), ranged resources (6.A2), auth (6.E) and fuzzing (6.F) "lands in later cards" — every one of those modules is declared 20 lines lower in this same file and has landed (only RTSP 6.G is genuinely absent, refused typed in schemes.rs); and "This crate is std-only" (repeated as "std-only, zero external dependencies" in Cargo.toml's header comment) is false — the crate's own `[dependencies]` pins `ureq = "=3.5.0"` with rustls+ring (C/asm) underneath, admitted by a later card that updated the manifest body but not the headers.

## 2. What actually works (verified)
— Module-list ↔ filesystem parity: all 13 declared modules exist (ls of crates/streaming/src). — Bounded-body claim: `response_limit` field, `DEFAULT_RESPONSE_LIMIT`, `with_response_limit`, `ResponseTooLarge` verified at fetch.rs:373/389/459-460 and error.rs. — Crate tests run this session: `cargo test -p streaming --lib reconnect::` (20 pass), `--lib dash::` (27 pass), `--test https_activation` (2 pass).

## 3. Findings
F-001 [S4]
Where: crates/streaming/src/lib.rs:3-10 vs 28-40
Code:
```
//! Everything else network — the real HTTP client
//! (ureq admission 6.A1b, implementation 6.A1c), ranged seekable
//! resources (6.A2), the scheme registry (6.A3 — landed), HLS (6.B —
//! landed), DASH (6.C — landed: [`dash`]), auth (6.E), fuzzing (6.F),
//! and RTSP (6.G) — lands in later cards on top of this port.
```
followed 18 lines later by:
```
pub mod auth;
pub mod client;
pub mod ranged;
```
What is wrong: the scope paragraph describes a scaffold where client/ranged/auth/fuzzing are future work; all four have landed (client.rs with `UreqClient`, ranged.rs, auth.rs, dash/fuzz.rs + the cargo-fuzz targets). Only RTSP (6.G) matches "lands later" — and it is typed-refused in schemes.rs ("RTSP family pending 6.G1-I"). A reader of the header alone gets the crate's maturity wrong in the direction of UNDER-claiming (harmless direction, but still a doc-vs-reality contradiction inside one file). What correct looks like: rewrite the paragraph in the style of the markers that were updated (A3/B/C "— landed") — or drop it for a module table. Confidence: HIGH — Verification: read + module existence check.
(Under-claiming doc: S4, not higher.)

F-002 [S4]
Where: crates/streaming/src/lib.rs:16-17 and crates/streaming/Cargo.toml:10
Code:
```
//! doc 01 §4.2 (engine core is threads + channels — no async runtime).
//! This crate is std-only and blocking; deadlines are plain
```
and the manifest header:
```
# C5 discipline: std-only, zero external dependencies. Sync-only per
```
against the same manifest's dependency section:
```
ureq = { version = "=3.5.0", default-features = false, features = ["rustls", "rustls-webpki-roots"] }
```
What is wrong: "std-only" is false — ureq (and transitively rustls + ring, which compiles C and asm via cc) is a first-class dependency; "zero external dependencies" is contradicted 40 lines later in the same Cargo.toml (whose inline comment even concedes "ureq stays the only external dep"). The honest claims that survive: sync-only (true), no async runtime (true), mock testkit std-only (true), bounded response memory (true). What correct looks like: headers updated when 6.A1b admitted ureq, e.g. "std-only except the Lead-ratified ureq/rustls/ring E2 exception (record ureq-3.5.0.md)". Confidence: HIGH — Verification: read + Cargo.toml.

## 4. Per-function table
Not applicable — 40-line crate root, zero functions (quality bar: "per-function table for files >100 LOC").

## 5. Test honesty
No tests in this file; the crate's suites were exercised this session (see §2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Responses are whole bodies under an explicit memory budget … fails typed" | lib.rs:22-25 | REAL | fetch.rs:373-460 + NetworkError::ResponseTooLarge |
| "no async runtime may appear in engine crates … This crate is std-only and blocking" | lib.rs:14-18 | HALF-FALSE | sync/no-async TRUE; std-only FALSE (ureq dep) — F-002 |
| "the real HTTP client … ranged … auth … fuzzing … lands in later cards" | lib.rs:6-10 | FALSE (stale) | all four landed — F-001 |
| Cargo.toml "std-only, zero external dependencies" | Cargo.toml:10 | FALSE | ureq pin at Cargo.toml:52 |

## 7. Dead / duplicated / unreachable code
None in this file. (Crate-level unreachability of individual modules — reconnect, dash — is recorded in their own audit files.)

## 8. Dependencies & flags
No feature flags. Runtime deps: media-types, plugin-api, playlists (workspace paths) + ureq 3.5.0 exact-pin (rustls, webpki-roots). Dev-deps are honestly segregated with rationale comments (testkit/demux/input/codecs dev-only).

## 9. Open questions for the next agent
- boundaries.toml registration notes in Cargo.toml mention "Lead-owned at wave close" for playlists/internal deps — has that registration landed? (Out of my file scope.)
