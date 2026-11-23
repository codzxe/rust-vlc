# AUDIT: crates/playlists/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-15 |
| Lines / functions | 119 / 0 functions — 11 module decls, 10 re-export groups |
| File verdict | REAL crate root (compiles, all re-exports exist) — crate UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO — only `streaming` depends on `playlists`; streaming's shipped modules (fetch/ranged/schemes/client) never import it; media-probe cannot identify any playlist format |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p playlists` → 104 passed/0 failed across 4 binaries (exit 0); rg dependency graph; sed quote checks |

## 1. One-paragraph verdict
A clean crate root: 11 modules (error, hls, m3u, mpd, pls, types, xspf, cue,
probe, fuzz) with complete re-export surface — I ran the whole crate's tests:
104/104 pass. The problem is not the file, it is the crate's position: the ONLY
dependent is `streaming`, and the only streaming modules reachable from the
shipped CLI (fetch/ranged/schemes over the bare UreqClient) never touch
`playlists`. `media-probe`'s format table (avi/mp4/wav/aiff/ogg/…) has no
m3u/pls/xspf/cue/mpd signature, so `lumen open <anything>.m3u` (or an HLS/DASH
URL) refuses at the probe stage: every playlist format this crate parses — six
formats plus CUE plus directory walking — is dead code in the shipped product.
The crate-level doc header still titles the crate "M3U/M3U8 playlist
reading/writing (cards 4.D1 + 5.O1a)" and talks about an "upcoming playlist
panel", understating a crate that now carries 7 format modules.

## 2. What actually works (verified)
— verified by `cargo test -p playlists` → 46+20+20+18 = 104 passed; 0 failed
(exit 0). Cargo.toml confirms the doc's dependency claim: thiserror 2 only,
dev-dependencies intentionally empty.

## 3. Findings
F-001 [S3]
Where: crates/playlists/src/lib.rs:69-119 (module tree)
Code:
```
mod error;
mod hls;
mod m3u;
mod mpd;
mod pls;
mod types;
mod xspf;
```
What is wrong: Crate unreachable from any shipped entry point. Chain: only
`crates/streaming/Cargo.toml` lists `playlists`; the CLI's open path reaches
streaming only through `SchemeRegistry::new()`/`RangedResource`/`fetch`, none of
which import playlists (rg over fetch.rs/ranged.rs/schemes.rs/client.rs: one
comment mention, zero code imports); and media-probe has no playlist signature
so even a local .m3u dies at probe. All six format parsers + probe + fuzz =
dead code in the shipped product. Why it matters: any matrix row citing
playlists evidence as user-visible playback is inflated (network-hls and
network-dash rows cite playlists/src/hls.rs and mpd.rs — those lanes are
library-only). What correct looks like: either wire playlist opening into the
CLI open path (probe signature + a playlist session) or mark the rows honestly.
Confidence: HIGH — Verification: read (full rg chain, my run of open-path code
reading in demux_host.rs:391-458).
F-002 [S4]
Where: crates/playlists/src/lib.rs:1-7
Code:
```
//! # playlists — M3U/M3U8 playlist reading/writing (Lumen cards 4.D1 + 5.O1a)
//!
//! Rust crate for parsing and writing M3U playlists (extended and plain)
//! plus the HLS `#EXT-X-*` media-playlist subset — the substrate for the
//! upcoming playlist panel and queue semantics (those land after 4.C2
//! merges — this crate is deliberately I/O-free, pure text-in/text-out,
//! so the UI layer owns file/dialog policy).
```
What is wrong: Stale header. The crate now also carries MPD (DASH), PLS, XSPF,
CUE, directory walking (probe), and a fuzz module — none mentioned; "upcoming
playlist panel… after 4.C2 merges" reads as imminent while no panel exists in
any shipped app. Same staleness in Cargo.toml's description ("extended/plain
M3U parsing… card 4.D1"). Confidence: HIGH — Verification: read (ls src/).
F-003 [S4]
Where: crates/playlists/src/lib.rs:99-104
Code:
```
mod cue;

pub use cue::{
```
What is wrong: `mod cue;` is declared below the other re-export blocks,
splitting the module declaration group (error/hls/m3u/mpd/pls/types/xspf at
L69-75) — a structural wart that suggests the module was appended without
reorganizing; makes the module inventory easy to misread. Confidence: HIGH —
Verification: read.

## 4. Per-function table (no functions; item inventory)
| Item | Lines | Verdict | Notes |
|---|---|---|---|
| crate docs | 1-67 | REAL | accurate on M3U/HLS/PLS/XSPF discipline; stale on scope (F-002) |
| mod decls ×11 | 69-75, 99, 111, 113 | REAL | all files exist (ls src/) |
| re-exports error/hls/m3u/mpd | 77-88 | REAL | compile proves symbols exist |
| re-exports pls/types/xspf/cue | 89-104 | REAL | same |
| probe re-export + docs | 106-119 | REAL | I/O-free posture maintained via caller-supplied impls |

## 5. Test honesty
No tests in this file. Crate-level claims are tested by the per-module suites
(104/104 pass, my run); the roundtrip law and fuzz claims are covered in
m3u/pls/xspf/mpd modules (spot-verified in the mpd.rs audit; others
UNVERIFIED here — other agents' files).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std + thiserror only" | lib.rs:53-54 | REAL | Cargo.toml [dependencies] thiserror = "2" |
| "no unsafe, per the workspace lint policy" | lib.rs:53-54 | UNVERIFIED | workspace lint config not checked; clippy baseline clean |
| "Roundtrip law… pinned by tests" | lib.rs:25-27 | REAL-UNPROVEN at crate level | mpd has no write path (parse-only by design); m3u roundtrip tests not re-run individually here (104/104 aggregate pass) |
| "I/O-free, pure text-in/text-out" | lib.rs:6-7, 95-98 | REAL | probe takes caller impls (L106-110); parse fns take &str/&[u8] |

## 7. Dead / duplicated / unreachable code
- Whole crate unreachable in product (F-001). No internal dead code found.

## 8. Dependencies & flags
thiserror 2 only; workspace lints; no features. Dev-deps intentionally empty.

## 9. Open questions for the next agent
- Is there a card for wiring `probe::probe_path`/playlist opening into the CLI
  (the "playlist panel" 4.C2)? If yes, F-001 is a sequencing gap, not a lie.
- Does the UI (ui/) reference playlist formats the backend cannot open?
