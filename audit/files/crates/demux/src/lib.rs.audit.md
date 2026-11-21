# AUDIT: crates/demux/src/lib.rs (full file, 55 lines)
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-11-21 |
| Lines / functions | 55 / 0 functions; 13 `pub mod` declarations + module doc |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (the crate root itself compiles into `lumen` via `apps/cli/src/containers/*`; 12 of 13 modules wired — see per-module table and F-002) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `rg -n '=> .*::open' apps/cli/src/containers/mod.rs` (registry arms); `rg -rn "asf\|wmv\|wma" crates/media-probe/ --ignore-case` (0 hits — no probe signature); `rg -rn "AsfDemuxer" apps/` (0 hits); house-rules sweeps: `rg '\.unwrap\(\)|panic!|todo!|unreachable!|dbg!'` over demux non-test code (all hits inside `#[cfg(test)]` modules at mkv/attachments.rs:281+, mkv/chapters.rs:598+, or doc comments); `rg -A8 '^\[dependencies\]' crates/demux/Cargo.toml` (path deps only) |

## 1. One-paragraph verdict
The crate root is a thin module-doc plus thirteen public module declarations,
and its "House rules" section is the strongest kind of claim — one I could
check, and it CHECKS OUT: zero `unwrap`/`expect`/`panic!`/`todo!`/`unreachable!`/`dbg!`
in demux non-test code (every hit is inside `#[cfg(test)]` modules or a
doc-comment example), no `unsafe`, and genuinely zero external dependencies
(path deps media-types + plugin-api, dev-only testkit). Two real defects:
the "Module map" documents only 7 of the 13 modules (adts, es, mkv, mp4,
ogg, ps are entirely absent — the doc still frames the crate as "v0 carries
the AVI 1.0 demuxer"), and the `asf` module the map DOES document is
unreachable from the shipped binary — no probe signature, no
container-registry arm, zero construction sites in apps/ — a gap the
feature-matrix's own compat-matrix triage recorded for linux (demoted to
partial) while leaving windows/macos "implemented" despite the wiring being
platform-independent code.

## 2. What actually works (verified)
- House rules, all verified by sweep:
  - `rg '\.unwrap\(\)'` non-test demux code → 0 hits.
  - `expect(`/`panic!`/`todo!`/`dbg!` → 0 non-test hits (ts/pes.rs:88 is a
    doc-comment example compiled as doctest).
  - `unreachable!` hits (mkv/attachments.rs:400+, mkv/chapters.rs:753+) are
    all inside `mod tests` (module starts at :281 / :598 respectively).
  - `unsafe` → only doc-comment mentions.
  - `[dependencies]` = media-types (path), plugin-api (path) — "zero
    external dependencies (C5)" holds.
- 12 of 13 modules are wired into the shipped open path
  (`apps/cli/src/containers/mod.rs:50-75`: adts aiff avi ivf matroska
  flac h264 hevc mp2 mp3 mp4 ogg mpegps mpegts wav — 15 probe ids).
- The no-compile-time-edge claim ("consumed at the trait seam") is
  accurate: demux depends on plugin-api's SourceStream trait, not on input.

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/lib.rs:12-33 (module map) vs :43-55 (declarations)
Code:
```rust
//! ## Module map
//!
//! - [`avi`] — the demuxer itself: RIFF chunk walk, `hdrl`/`strl`
//!   track discovery, `idx1` seek table, MJPEG+PCM packet extraction,
//!   seek-to-time.
```
(and the map continues through wav, ivf, aiff, ts, asf, error only)
```rust
pub mod adts;
pub mod aiff;
pub mod asf;
pub mod avi;
pub mod error;
pub mod es;
pub mod ivf;
pub mod mkv;
pub mod mp4;
pub mod ogg;
pub mod ps;
pub mod ts;
pub mod wav;
```
What is wrong: The module map documents 7 modules; the crate declares 13.
`adts`, `es`, `mkv`, `mp4`, `ogg`, `ps` — six landed demuxers, including
the MP4 module at the center of the project's own litmus story — are
absent from the map, and the framing sentence ("v0 carries the AVI 1.0
demuxer of the first vertical") describes a crate that stopped growing a
year of cards ago.
Why it matters: The crate-root doc is the first thing a reader (or
auditor) uses to scope the crate; it under-reports by half. Doc drift of
this kind is how evidence strings end up citing suites that "should"
exist — the AVI row's "43-test" and asf row's "454-test" counts (both
stale, both found in this wave) come from the same era of not updating
crate-level records.
What correct looks like: One map line per module (13), plus removing or
dating the "v0 carries" framing.
Confidence: HIGH — Verification: read + count.

### F-002 [S3]
Where: crates/demux/src/lib.rs:27-30 (asf map entry) + apps/cli/src/containers/mod.rs:50-75 (registry)
Code (the map entry):
```rust
//! - [`asf`] — the ASF/WMV/WMA header tree (card 6.ASF-a): the
//!   GUID+size object walk, File/Stream Properties,
//!   WAVEFORMATEX/VIDEOINFO facts, Codec List + metadata, Data
//!   Object location (packets are 6.ASF-b's).
```
Code (the shipped registry, containers/mod.rs:50-73 — no `asf` arm):
```rust
    match format {
        // The entry shape: `"<probe-id>" => <mod>::open(stream, opts),`
        "adts" => adts::open(stream, opts),
        "aiff" => aiff::open(stream, opts),
        "avi" => avi::open(stream, opts),
```
What is wrong: The `asf` module is UNREACHABLE from the shipped `lumen`
binary, three times over: (1) media-probe has no ASF/WMV/WMA signature
(`rg -rn "asf|wmv|wma" crates/media-probe/ --ignore-case` → 0 hits — an
.asf/.wmv file dies at the probe stage with `probe.no-demuxer`); (2) the
container registry has no `asf` arm (15 ids wired, asf absent); (3) no
`containers/asf.rs` exists and `AsfDemuxer` has zero references in apps/.
The fuzz registry also lacks an asf target (35 targets, no asf).
Why it matters: A whole demuxer family — asf/mod.rs 780 lines + header +
data + guid + error + a 1578-line test file + 11 committed fixtures with
a verified differential — is production-dead code. The feature-matrix row
`container-asf-wmv` records exactly this in its notes (compat-matrix
triage: "the open path REFUSES the corpus files of probe format `asf` …
probe.no-demuxer") and demoted linux to `partial`, but leaves
**windows = "implemented", macos = "implemented"** — although the missing
probe signature and registry arm are platform-independent code that
refuses identically on every OS. Per this audit's reachability standard,
the asf module's verdict is UNREACHABLE; the win/macos cells are
FALSE-as-player-capability (the row's own notes prove the refusal is
platform-independent).
What correct looks like: Land the R3 signature + registry arm (the notes'
own "next step", ivf/mpegps precedent), or demote win/macos cells to
match linux's partial until then.
Confidence: HIGH — Verification: rg sweeps (0 hits across media-probe and
apps/), registry read, matrix row read (docs/feature-matrix.toml:207-216).

## 4. Per-module reachability table (checklist rule 10, crate level)
| Module | Declared | Wired into shipped open path? | Proof |
|---|---|---|---|
| adts | :43 | YES | containers/mod.rs:52 `"adts" => adts::open` |
| aiff | :44 | YES | :53 |
| asf | :45 | **NO — UNREACHABLE** | F-002 (no probe signature, no arm, no callers) |
| avi | :46 | YES | :54 (first vertical, baseline avi-firstvertical-open-session.txt) |
| error | :47 | YES (avi codes minted on the open path) | crate-root error.rs used by avi/mod.rs |
| es | :48 | YES | :62-66 (flac/h264/hevc/mp2/mp3 lanes) |
| ivf | :49 | YES | :55 |
| mkv | :50 | YES | :57 `"matroska"` |
| mp4 | :51 | YES | :67 |
| ogg | :52 | YES | :68 |
| ps | :53 | YES | :71 `"mpegps"` |
| ts | :54 | YES | :72 `"mpegts"` |
| wav | :55 | YES | :73 |

## 5. Test honesty
No tests in this file. (The crate's 462-test suite executed 462/462 ok in
baseline — `orchrun-test-workspace-nofailfast.log.txt:1489-1955`.)

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Std-only, zero external dependencies (C5)" | lib.rs:37 | REAL | Cargo.toml [dependencies] = 2 path deps; no unsafe; sweeps clean. |
| "no `unwrap`/`expect`/`panic!`/`todo!`/`unreachable!`/`dbg!` in non-test code" | lib.rs:38-39 | REAL | Sweeps: all hits in `#[cfg(test)]` modules or doc comments (see §2). |
| "bounded memory … every read is chunk-bounded and declared sizes are ceiling-checked" | lib.rs:39-41 | REAL (spot-verified) | READ_STEP_MAX 64 KiB + MAX_CHUNK_PAYLOAD 32 MiB (avi/mod.rs:126,132), MAX_PACKET_BYTES/MAX_FMT_BYTES/MAX_INFO_BYTES (wav) — verified in my part audits of avi/mod.rs#part2 and wav/mod.rs#part1. |
| Module map completeness | lib.rs:12-33 | FALSE (incomplete) | F-001: 7 of 13 modules documented. |
| container-asf-wmv: windows=implemented, macos=implemented | docs/feature-matrix.toml:213 | FALSE-as-capability / UNREACHABLE | F-002: row's own notes prove platform-independent open-path refusal; linux already demoted to partial by the same evidence. |
| "v0 carries the AVI 1.0 demuxer of the first vertical" | lib.rs:4-5 | STALE | True historically; the crate now carries 12 demuxer families. |

## 7. Dead / duplicated / unreachable code
- `asf` module — UNREACHABLE from shipped entry points (F-002). This is
  the crate's only unreachable module. (Its internal quality is high —
  see my asf/tests.rs#part3 audit: real differential, verified sha pins —
  the defect is wiring, not the module.)

## 8. Dependencies & flags
media-types (path), plugin-api (path); dev: testkit (path). No features
declared in the crate root; workspace lints inherited (per Cargo.toml
audit, wave 1). No build.rs.

## 9. Open questions for the next agent
- The matrix-rows auditor for container-asf-wmv (windows/macos cells):
  cite F-002 — the compat-matrix triage note in the row itself is the
  proof; the cells should not survive as "implemented".
- Whoever owns media-probe: the ASF signature absence also means the
  `probe hints: extension=wmv` message in the triage note shows extension
  routing exists but content probing cannot fire — confirm extension
  routing likewise dead-ends at the registry (it must: same match).
