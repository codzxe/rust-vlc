# AUDIT: crates/playlists/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2026-11-21 |
| Lines / functions | 119 / 13 items (1 doc block, 8 `mod` decls, 8 `pub use` blocks, 2 `pub mod` decls) — a pure façade; zero logic |
| File verdict | UNREACHABLE (the crate compiles into the shipped `lumen` binary via streaming, but every parse entry re-exported here is dead code in that binary) |
| Reachable from shipped entry point? | NO for every exported parse lane. Proof per lane: m3u `parse`/`parse_bytes` + xspf `parse_xspf_bytes` → only would-be consumer `streaming::remote_playlist::open_remote_playlist` (remote_playlist.rs:114,194), which itself has zero app callers; `parse_pls*` → no caller outside tests (pls.rs audit F-001); `parse_mpd*` → test/fuzz-only callers (mpd.rs#part2 audit F-001); `parse_media_playlist`/`parse_master_playlist` → streaming hls sessions, which no app opens (live.rs#part3 audit F-001); `parse_cue*` and `probe`/`walk` → zero callers outside the crate's own tests (rg census, header of device-discovery-style: no hits outside crates/playlists). The crate IS in the shipped binary's dependency graph (apps/cli → streaming → playlists), so the code ships but never runs. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "parse_cue\|probe_path\|walk\(" crates apps` (scoped, excluding playlists' own tests) → no external production callers; `rg -n "unsafe" crates/playlists/src` → doc mentions only (2 hits, both comments) — the no-unsafe claim holds; `rg -n "roundtrip" crates/playlists/tests/m3u_suite.rs` → assert_roundtrip harness at 20, used at 147/195/212; `rg -n "fuzz\|SplitMix" crates/playlists/tests/m3u_suite.rs` → 10k fuzz-lite at 397-460; suite results from orchrun-test-workspace-nofailfast.log.txt: lib 46/0 (4259-4296), cue 20/0 (4311+), hls_attrs 20/0 (4337+), m3u 18/0 (4363+), pls 14/0 (4387+), xspf 14/0 (4407+) — 132 tests, 0 failures; `sed -n` on quoted ranges |

## 1. One-paragraph verdict
A clean re-export façade with unusually honest module docs — every claim I could check locally is true and test-pinned: the roundtrip law has a real `assert_roundtrip` harness over golden, dirty, and fuzz inputs (18/18 green), the no-panic claim is backed by 10k-input fuzz-lite sweeps, the crate is genuinely std+thiserror with zero unsafe, and the deliberate I/O-free posture is documented with its reason. The two problems are scope drift and product deadness: the crate-level docs still introduce this as "M3U/M3U8 playlist reading/writing" and never mention that the façade also exports a DASH MPD model, a CUE parser, a directory-walking probe module, and a public fuzz surface — three public modules undocumented at the front door — and although the crate is compiled into the shipped `lumen` binary through the streaming dependency, not one of the ~50 exported items is reachable from a user action: every parse lane terminates inside library code that no app ever calls, while the feature matrix stamps PLS/XSPF/CUE "implemented".

## 2. What actually works (verified)
— All six test binaries green: `orchrun-test-workspace-nofailfast.log.txt` — playlists lib 46 passed/0 failed, cue_suite 20/0, hls_attrs 20/0, m3u_suite 18/0, pls_suite 14/0, xspf_suite 14/0 (132 total, 0 failures, 0 ignored).
— Roundtrip-law claim: `assert_roundtrip` (tests/m3u_suite.rs:20-30) pins `parse(write(parse(x))) == parse(x)` on golden (147), dirty corpus (195), and fuzz-generated inputs (212); the doc's exact sentence is backed.
— "Std + thiserror only" (lib.rs:53): Cargo.toml:18-19 — single dependency `thiserror = "2"`; "no `unsafe`" (lib.rs:54): rg over src returns only the two doc comments that mention the policy.
— No-panic claim: fuzz-lite in m3u_suite (random bytes 10k + mutations, 397-460) and the in-crate `fuzz::tests` (both green in the lib run) — plus per-format suites in tests/.

## 3. Findings

### F-001 [S3] The façade exports five format parsers and a directory walker; the shipped binary can execute none of them
Where: crates/playlists/src/lib.rs:77-97
Code:
```rust
pub use m3u::{MAX_LINE_BYTES, Parsed, parse, parse_bytes, parse_detailed, resolve_uri, write};
pub use mpd::{
    AdaptationSet, ContentProtection, Descriptor, MAX_MPD_BYTES, MAX_MPD_DEPTH, Mpd, MpdError,
    MpdParsed, MpdType, MpdWarning, Period, Representation, SegmentBase, SegmentList,
    SegmentTemplate, SegmentTimeline, SegmentTimelineEntry, SegmentUrl, parse_mpd, parse_mpd_bytes,
};
```
What is wrong: every export lane's caller chain dead-ends in library code with no app wiring (census in the header row — m3u/xspf at remote_playlist with no app caller; pls/mpd/cue/probe with no production caller at all). The crate ships inside `lumen` only as dead weight of the streaming dependency.
Why it matters: docs/feature-matrix.toml stamps `plfmt-pls`, `plfmt-xspf`, `plfmt-cue` as `implemented` on all desktop platforms and `plfmt-m3u` as `partial` — every evidence sentence is true of the library and false of the product: no shipped surface lets a user open any playlist file. This façade is where a host would wire in; nothing ever did.
What correct looks like: one production consumer — the desktop shell's `playlist.load` actually parsing through these exports (the classification already exists in apps/desktop/src/files.rs), or matrix notes marking the family "library-complete, no shipped host".
Confidence: HIGH — Verification: per-lane rg census (header; details in my pls.rs / mpd.rs#part2 / live.rs#part3 audits).

### F-002 [S4] Crate-level docs are three modules and one era behind the façade
Where: crates/playlists/src/lib.rs:1-7
Code:
```rust
//! # playlists — M3U/M3U8 playlist reading/writing (Lumen cards 4.D1 + 5.O1a)
//!
//! Rust crate for parsing and writing M3U playlists (extended and plain)
//! plus the HLS `#EXT-X-*` media-playlist subset — the substrate for the
//! upcoming playlist panel and queue semantics (those land after 4.C2
//! merges — this crate is deliberately I/O-free, pure text-in/text-out,
```
What is wrong: the title and Scope section describe M3U + HLS + (in a later paragraph) PLS/XSPF — but the façade also exports the MPD model (6.C1a, a 2,854-line module), the CUE parser (5.O2), the probe/walk directory module (5.O3, with its own doc block at 106-110 but no Scope mention), and the public `fuzz` surface. The "upcoming playlist panel … land after 4.C2 merges" promise is stale by the repo's own wave history (waves 1-9 shipped; no panel consumer exists). `mod cue;` (99) is also declared after the other seven mods and its re-export block breaks the file's grouping — cosmetic drift of the same kind.
Why it matters: lib.rs is the front door a host author reads; it under-reports the crate's actual surface (and its over-promised consumer), so integration decisions get made on a scope description that is two cards stale.
What correct looks like: a Scope bullet per public module (m3u/hls, pls, xspf, mpd, cue, probe, fuzz) and deletion of the 4.C2-era timeline promise.
Confidence: HIGH — Verification: read the whole file; cross-checked module list vs docs.

## 4. Per-function table (facade items; no logic to mutate)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-67 | REAL | S4 | Every testable claim verified true (§2) — but stale scope (F-002). |
| `mod` decls error/hls/m3u/mpd/pls/types/xspf | 69-75 | REAL | — | Private mods, re-exported below; compile-verified names. |
| `pub use error` | 77 | REAL | — | ParseError/ParseWarning. |
| `pub use hls` | 78-82 | REAL | — | Master/media playlist surface (consumed by streaming hls sessions — library-dead in product, F-001). |
| `pub use m3u` | 83 | REAL | — | The one lane with a would-be production consumer (remote_playlist) — still app-dead. |
| `pub use mpd` | 84-88 | REAL | — | 20 items; see mpd.rs#part2 F-001. |
| `pub use pls` | 89-92 | REAL | — | See pls.rs F-001. |
| `pub use types` | 93 | REAL | — | Shared Entry/Playlist vocabulary. |
| `pub use xspf` | 94-97 | REAL | — | MAX_XML_DEPTH exported for the fuzz surface. |
| `mod cue;` + `pub use cue` | 99-104 | REAL | — | Works; placement breaks the file's own grouping (F-002). |
| `pub mod probe` + re-export | 106-111, 115-119 | REAL | — | 14 items; zero external callers (census). |
| `pub mod fuzz` | 113 | REAL | — | 4 one-line no-panic wrappers, deliberately public for fuzz/fuzz_targets (wiring status honestly documented in fuzz.rs:20-25); ships in the binary — 4 trivial fns, acceptable. |

## 5. Test honesty
No tests in this file (façade). The crate's suites that back its doc claims all ran green (§2). lib.rs itself is compile-verified by the workspace build (baseline #1).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Roundtrip law: parse(write(parse(x))) == parse(x) … pinned by tests, incl. dirty corpus, ffmpeg fixtures, fuzz-lite" | lib.rs:25-27 | REAL | m3u_suite assert_roundtrip green (18/18, log 4363-4386). |
| "No panics on any byte input — pinned by deterministic fuzz-lite harnesses" | lib.rs:50-52 | REAL | fuzz::tests + 10k m3u fuzz-lite green (lib 46/0, m3u 18/0). |
| "Std + thiserror only … no unsafe" | lib.rs:53-54 | REAL | Cargo.toml:19 single dep; rg unsafe → comments only. |
| plfmt-pls / plfmt-xspf / plfmt-cue "implemented" (win/linux/macos) | feature-matrix.toml ~1172/1184/1223 | REAL-in-library / UNREACHABLE-in-product | 14/14, 14/14, 20/20 suites green; zero shipped callers (F-001 census). |
| "the substrate for the upcoming playlist panel and queue semantics (those land after 4.C2 merges)" | lib.rs:4-6 | PAPER-ONLY | No panel/queue consumer exists anywhere (waves 1-10 census). |

## 7. Dead / duplicated / unreachable code
The entire exported surface is product-dead (F-001) though library- and test-alive. No duplication; the façade pattern itself is clean (one re-export per module, no renames beyond the pls/xspf `parse as parse_*` disambiguation).

## 8. Dependencies & flags
thiserror 2 only (Cargo.toml:19); workspace lints inherited; no feature gates anywhere in the crate (verified: no `cfg(feature` in src). No unsafe.

## 9. Open questions for the next agent
- The one wiring that would revive this whole crate family: apps/desktop `playlist.load` — files.rs already classifies the suffixes; the command is unserved (see pls.rs F-001). Whoever owns that host owes the matrix rows their reachability.
- xspf.rs / cue.rs / m3u.rs / probe.rs owners (waves 8/9/16?): my CRLF-cap finding in pls.rs (cap check before `\r` strip) is worth checking in m3u.rs — same era, same pattern likely.
