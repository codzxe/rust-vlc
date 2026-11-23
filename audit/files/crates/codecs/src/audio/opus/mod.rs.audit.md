# AUDIT: crates/codecs/src/audio/opus/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-23 |
| Lines / functions | 36 / 0 (module organizer: 4 mod decls + 1 re-export) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:276 `"opus" =>` arm constructs OpusDecoder; `codec-opus` in codecs' default features |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | prebuilt `./target/debug/deps/codecs-a0d7a93b7072ff58 opus::` -> 13 passed; prebuilt `opus_corpus_differential-06f3a6620b4b27bb --nocapture` -> 2 passed, BOTH via the SKIP lane ("no corpus directory"); rg/Cargo.toml verification of the engine pin; registry-source scan of ruopus 0.1.2 |

## 1. One-paragraph verdict
A 36-line organizer whose every declarative claim verifies: the module map matches the files (head/order/decoder/error), the engine pin is exactly as documented (`ruopus = { version = "=0.1.2", optional = true, default-features = false, features = ["std"] }` — Cargo.toml:137, verbatim), the "pre-constructor validation closes the ruopus constructors' documented panic windows" claim is backed by a passing executed test (`rejects_the_constructor_panic_windows_typed`) and by the engine's real asserts (ruopus src/packet.rs:150-151), and the 13 submodule unit tests pass. The one substantive defect is a coverage asymmetry: Opus is the ONLY audio codec family in this crate with ZERO committed real-stream fixtures — its sole real-stream differential (tests/opus_corpus_differential.rs) requires the /tmp-resident corpus (absent in-tree and in this sandbox), so both its corpus tests executed as SKIP-lane passes; the recorded admission numbers (pre-skip 312, raw 192,960, max |Δ| 1.378e-06) are constants no in-tree run re-verifies. Vorbis, by contrast, ships four committed .ogg fixtures with an always-on bitexact suite. ffmpeg IS present in this sandbox (/usr/bin/ffmpeg) — only the corpus is missing.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 opus::` -> 13 passed: descriptor gate, configure validation, hostile-packet isolation, seek-reanchor landing, multistream 5.1 geometry, broken-multistream rejection, constructor-panic-window rejection, channel-order bijections, error taxonomy.
- Engine pin verified: crates/codecs/Cargo.toml:137 matches the doc's claim character-for-character; Cargo.lock carries ruopus 0.1.2.
- ruopus registry source: the documented panic windows exist (src/packet.rs:150-151 `assert!(config < 32...)`, `assert!(frame_count_code < 4...)`); unsafe is confined to documented SIMD kernels (src/lib.rs:3, src/simd.rs) — mod.rs makes no forbid(unsafe) claim for ruopus, so no doc conflict.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/opus/mod.rs:1-36 (the module's verification posture) + tests/opus_corpus_differential.rs:51-55
Code:
```
    let Some(root) = corpus_dir() else {
        eprintln!("SKIP {lane}: no corpus directory (LUMEN_CORPUS unset, /tmp default absent)");
        return;
    };
```
What is wrong: Opus has no committed fixture (full `ls crates/codecs/tests/fixtures/`: alac/, av1/, breadth/, h264/, mpeg12/, vp9/, mp3/aac/vorbis/flac/adpcm files — zero opus). The module's only real-stream AC (geometry + live ffmpeg float oracle + channel-order-through-tolerance) therefore skips in every in-tree run: I executed the corpus binary with --nocapture and both lanes printed "SKIP corpus.0047/0077: no corpus directory" before passing. The decoder unit tests use one verbatim-embedded 95-byte packet (decoder.rs:614-617) — real bytes, but one packet of one stream. Consequence: a regression in the Opus decode path (pre-skip trim, end-trim, channel permutation under a real 5.1 stream) is caught by NOTHING that runs in CI; the recorded admission numbers are dead constants until a corpus appears. Why it matters: every sibling codec family has an always-on committed differential; Opus alone relies on environment-resident data. What correct looks like: commit one mono + one 5.1 .opus fixture (ffmpeg can generate them — the binary exists here) and make the corpus lane the extra, not the only, real-stream run. Confidence: HIGH — Verification: executed the skip lane myself; fixture listing; decoder.rs read.
(Noted, folded here:) decoder.rs:614-616 cites `tests/fixtures/generated/valid/ogg_opus_48k_mono.opus` — a path that does not exist anywhere in the tree (the `generated/` directory is absent); the test honestly says the packet is embedded "verbatim so the unit needs no filesystem", so nothing breaks, but the citation points at a ghost file.

## 4. Module inventory (organizer; <100 LOC so no per-function table required)
| Item | Lines | Verdict | Notes |
|---|---|---|---|
| module docs | 1-33 | REAL | Engine pin, panic-window closure, module map — all verified (§2); the doc makes no claim about in-tree differential coverage (the gap is F-001) |
| `pub mod decoder; mod error; mod head; mod order;` | 35-38 | REAL | All four files exist; decoder is the only pub module (head/order/error are pub(crate) — correct, no external consumers) |
| `pub use decoder::OpusDecoder;` | 40 | REAL | The single live export, consumed by pipeline_host.rs:276-280 |

## 5. Test honesty
No tests in this file. Submodule suites: 13 executed, pass — the hostile-packet and panic-window tests are real (would fail on regression). The corpus differential's skip is honest (message + eprintln, no false pass claim in its own docs) but see F-001: honesty about a coverage hole is not coverage.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`ruopus = { version = "=0.1.2", default-features = false, features = ["std"] }` — the zero-dependency shape" | opus/mod.rs:23-25 | REAL | Cargo.toml:137 verbatim match; Cargo.lock node present |
| "the **pre-constructor validation** that closes the ruopus constructors' documented panic windows (channels/rate)" | opus/mod.rs:9-11 | REAL | Engine asserts exist (ruopus packet.rs:150-151); `rejects_the_constructor_panic_windows_typed` executed ok |
| "Default features (spectrograms → ndarray/rustfft/regex-automata graph, python → pyo3) are OFF" | opus/mod.rs:25-27 | REAL | default-features = false in the dep line; no ndarray/pyo3 in Cargo.lock's ruopus node (single node per the crate comment) |
| doc 04 §3.1 row "Opus" (decoder shipped) | opus/mod.rs:1-2 | REAL | pipeline_host.rs:276 arm; 13 unit tests executed |

## 7. Dead / duplicated / unreachable code
None in this file. The RFC 7845 → house order permutation (order.rs) is exercised and used at the decoder seam — live code.

## 8. Dependencies & flags
ruopus =0.1.2 exact-pinned, optional behind `codec-opus` (default-on), default-features off. The engine contains documented unsafe SIMD kernels (ruopus src/simd.rs) — mod.rs claims no purity for it; the admission record owns that discussion (docs/dependencies/records/ruopus-0.1.2.md, not in my file list).

## 9. Open questions for the next agent
- opus/decoder.rs owner: F-001 — commit a mono + 5.1 fixture; also the ghost fixture path at decoder.rs:614-616.
- ruopus 0.1.2's unsafe SIMD kernels: someone should audit src/simd.rs's soundness argument (docs/unsafe.md in the crate) — outside my file list, and the crate is a dependency, but it is the only unsafe in my file's dependency closure.
- The vorbis/opus seek-helper sharing (opus/decoder.rs uses vorbis::seek exports): confirm the lapping arithmetic assumptions hold for Opus's fixed 120-sample overlap vs Vorbis's variable windows.
