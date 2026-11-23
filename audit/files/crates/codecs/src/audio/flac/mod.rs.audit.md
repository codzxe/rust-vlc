# AUDIT: crates/codecs/src/audio/flac/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-23 |
| Lines / functions | 130 / 3 prod fns + 1 re-export facade + 1 test |
| File verdict | REAL (facade) with S3 dead-surface findings |
| Reachable from shipped entry point? | PARTIAL — the module's shipped surface is `FlacDecoder` only: apps/cli/src/pipeline_host.rs:58 `use codecs::audio::flac::FlacDecoder` ("flac" arm :253, FLAC-in-Ogg/FLAC-in-WAV carriers) and apps/web/src/host.rs. The native-`fLaC`-stream surface this module documents (probe/STREAMINFO/SEEKTABLE walk/seek) is reached by NO product path: apps/cli/src/containers/es.rs:258-271 `open_flac` ALWAYS returns Unsupported ("the raw-FLAC elementary-stream demuxer has not landed"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 0 |
| Commands run | `cargo test -p codecs --lib flac::` → 25 passed; `cargo test -p codecs --test flac_bitexact` → 4 passed (incl. `fixtures_decode_byte_exact_against_the_ffmpeg_reference`); `cargo test -p codecs --test flac_vlc_differential` → **linker Bus error — disk 100% full (27 MB free), environment not code**; baseline cargo-test-workspace.txt:1095-1099 shows it passing (`flac_differential_vs_vlc_or_pending_infra ... ok`). |

## 1. One-paragraph verdict
This is the FLAC module's facade: three small real functions, honest wiring-status docs, and a correct format-preview test (hand-packed STREAMINFO bits → S24/2ch/48 kHz verified). The decode engine it fronts is genuinely bit-exact against ffmpeg (4/4 flac_bitexact tests ran green here, byte equality on 16-bit/24-bit/8-bit fixtures), claxon is exact-pinned (`=0.4.3`, Cargo.toml:86) with its purity record present, and the VLC differential's "PENDING INFRA" honesty claim is accurate (no vlc/cvlc binary; the always-on Lumen-side half is asserted against the ffmpeg oracle). The hostile finding is reachability asymmetry: the module doc leads with "**Native FLAC** (the `fLaC` byte stream): the metadata chain ... frame decode ... and seeking", but the shipped CLI cannot open a native .flac file at all — the probe arm is a permanent typed refusal in another file — so the entire native-stream surface this module advertises (`is_flac`, `parse_stream`, `SeekIndex`, `frame_header_at`, `SeekPlanner`, `decode_frame_with_streaminfo`, `output_format`) is exercised only by the test suite; two of those pub functions have zero callers even there.

## 2. What actually works (verified)
— `cargo test -p codecs --lib flac::` → 25/25 passed (module test incl. the packed-STREAMINFO → S24/2ch/48k pin and the empty-packet typed-error pin).
— `cargo test -p codecs --test flac_bitexact` → 4/4 passed — byte-exact vs the committed ffmpeg references (the module's headline AC).
— claxon `=0.4.3` pin + `docs/dependencies/records/claxon-0.4.3.md` present (read).
— VLC absence verified (`which vlc cvlc` → nothing) — the "PENDING INFRA" claim's premise holds; baseline :1095-1099 records the suite green.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/flac/mod.rs:6-11 (doc) vs apps/cli/src/containers/es.rs:262-270
Code:
```rust
//! **Native FLAC** (the `fLaC` byte stream): the metadata chain
//! (STREAMINFO / SEEKTABLE parse — [`metadata`]), frame decode through
//! the pinned `claxon` 0.4.3 engine ([`decoder`], the dependency
//! decision and its purity record are documented there), and seeking —
```
vs the shipped CLI's native lane:
```rust
    Err(CommandError::new(
        Stage::Probe,
        ErrorCode::Unsupported,
        "format `flac` probes (the fLaC marker) but the raw-FLAC elementary-stream demuxer \
         has not landed: crates/demux/src/es covers h264/hevc/mp3/aac (card 5.I3a's slice) — \
```
What is wrong: the module's "Format slice (what this module owns)" headline claims the native fLaC stream as its subject, and the machinery (metadata walk, seek planner, frame-header parse) is real and tested — but no shipped entry point reaches any of it for native streams: `open_flac` is an unconditional typed refusal. Product FLAC reachability is FLAC-in-Ogg and FLAC-in-WAV only (both feed `FlacDecoder` via container-harvested STREAMINFO, never through this module's `parse_stream`/`SeekIndex`). The honesty lives in a different file's error text; a reader of this module would conclude the native lane works. The refusal itself is honest and forward-referencing — the defect is the facade doc's framing, plus a large tested-but-unreachable surface.
Confidence: HIGH — Verification: read of both files + rg over apps/ + crates/demux (only `FlacDecoder` is imported in product code).
F-002 [S3]
Where: crates/codecs/src/audio/flac/mod.rs:86-92
Code:
```rust
pub fn decode_frame_with_streaminfo(
    packet: &[u8],
    streaminfo_body: &[u8],
) -> Result<DecodedFlac, EngineError> {
    let facts = parse_streaminfo(streaminfo_body)?;
    decode_frame(packet, &facts)
}
```
What is wrong: zero callers outside its own module test (rg across the repo: only mod.rs:128). A pub "convenience composition" with no consumer — dead API kept alive by a one-line assertion.
Confidence: HIGH — Verification: rg (repo-wide, audit/ excluded).
F-003 [S3]
Where: crates/codecs/src/audio/flac/mod.rs:94-106
Code:
```rust
/// The negotiated audio shape a configured stream will emit (the
/// registry/adapters' preview of [`AudioFormat`] without decoding —
/// derived from the same left-justify mapping as [`decode_frame`]).
```
What is wrong: `output_format` has zero callers anywhere (product, demux, adapters, tests) except the module test that pins it. The doc justifies it as "the registry/adapters' preview" — no registry or adapter consumes it (the sibling codecs' `output_format` functions DO have test callers; this one does not even have that outside its own module). Misleading-justification dead code.
Confidence: HIGH — Verification: rg repo-wide.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `frame_header_at` | 74-77 | REAL (test-only product-wise) | S3 (via F-001) | pure delegation to `SeekPlanner::parse_header_at`; callers: tests/flac, flac_seek, flac_bitexact only |
| `decode_frame_with_streaminfo` | 86-92 | REAL / dead | S3 | F-002 |
| `output_format` | 98-106 | REAL / dead | S3 | F-003 |
| re-exports | 62-66 | REAL | — | `FlacDecoder`/`decode_frame` product-reachable; the rest test-only |
| tests `output_format_matches_the_decode_mapping` | 112-129 | REAL | — | correct hand-packed STREAMINFO bits (rate 48000<<44, ch-1 <<41, bps-1 23<<36) — would catch a mapping drift in `output_sample_format` |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| output_format_matches_the_decode_mapping | YES | exact S24/2ch/48k/Interleaved pins + empty-packet typed-error pin |
| (integration) flac_bitexact 4 tests | YES | byte equality vs committed ffmpeg references — executed green here |
| (integration) flac_vlc_differential | N/A here | could not execute in this sandbox (linker Bus error — disk 100% full, 27 MB free; environment, not code); baseline :1098 records it ok; its PENDING-VLC premise verified (no vlc binary) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the pinned `claxon` 0.4.3 engine" | :8-9 | REAL | Cargo.toml:86 `claxon = { version = "=0.4.3", optional = true }` + record file |
| "Native FLAC (the fLaC byte stream): the metadata chain ... frame decode ... seeking" (module owns) | :6-11 | SHALLOW | machinery real + tested; native stream NOT openable in the shipped CLI (F-001) |
| "Ogg-FLAC is out of this card's slice" | :11-12 | REAL | and it is the only shipped FLAC carrier lane plus FLAC-in-WAV |
| "differential tests decode ffmpeg-generated fixtures ... byte equality" | :41-44 | REAL | flac_bitexact 4/4 executed |
| "VLC differential hook ... registered and honestly pending infra (no VLC in the sandbox)" | :45-48 | REAL | file exists, `which vlc` empty, baseline green |
| "no panic on any input" | :32-36 | REAL-UNPROVEN | submodule claim (decoder/metadata/seek are other agents' files); fuzz survival suites exist |

## 7. Dead / duplicated / unreachable code
- `decode_frame_with_streaminfo` — dead pub API (F-002).
- `output_format` — dead pub API with misleading justification (F-003).
- Native-stream surface (`is_flac`, `parse_stream`, `SeekIndex`, `SeekPlanner`, `frame_header_at` re-export) — real, tested, unreachable from shipped entry points pending the es-demuxer landing card (F-001; the refusal text itself names the owed card).

## 8. Dependencies & flags
claxon `=0.4.3` behind feature `codec-flac` (in `default`). No unsafe here; engine's unsafe sites recorded in the dependency record (doc claim, record file present).

## 9. Open questions for the next agent
- flac/decoder.rs, metadata.rs, seek.rs auditors: does `FlacDecoder::configure` parse the container-harvested 34-byte STREAMINFO via `parse_streaminfo` (making that function product-reachable through the ogg/wav lanes, softening F-001's "test-only" scope for the STREAMINFO parse specifically)?
- feature-matrix row "FLAC (native)" vs doc 04 §2.1 — does the matrix claim native .flac open works? (parity-report already shows 0/252 verified — cross-check the row's evidence string against the es.rs refusal.)
