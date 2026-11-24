# CLAIM AUDIT: docs/adr/ADR-010-codec-patent-shipping.md
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACH | 5 / 0 / 0 / 0 / 1 / 1-stale / 0 |
| Internal contradictions found | 1 (vs current feature-matrix.toml) |

## 1. Blunt verdict
A disciplined proposed-decision record: status "proposed", "this ADR is NOT
accepted", every patent line tagged [VERIFY: counsel], negative option
retained. The implementation inventory it cites is real — I verified the
codec module tree, the dependency admission records, the typed
h264/hevc hw-required refusals, and the default=full-set feature gate.
One defect: its own "matrix drift noted honestly" paragraph is now itself
stale — the rows it says "still read blocked/not_started" have since been
refreshed to implemented/working-subset.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "crates/codecs … compiles unconditionally today" | :68-70 | REAL | crates/codecs/Cargo.toml [features] default = full 11 codec-* set (verified by read); in-house decoders unconditional |
| 2 | software decoders table paths (mjpeg/av1/vp9/dnx/pcm/adpcm/mp3/mp2/aac/flac/alac/vorbis/opus) | :76-91 | REAL | `ls crates/codecs/src/{video,audio}/` — all 16 cited modules exist exactly as named |
| 3 | h264/hevc "Decoder impl fails configure with typed h264.hw.required" | :104-105 | REAL | crates/codecs/src/video/h264/mod.rs:13, hw/mod.rs:54,478; hevc same shape |
| 4 | BLOCKED admissions "oxideav-ac3 …, oxideav-mpeg4video …, truehd …" records exist | :113-118 | REAL | docs/dependencies/records/{oxideav-ac3-0.0.11, oxideav-mpeg4video-0.1.8, truehd-0.7.2, filmcraft-dnx-0.9.0, rav1d-1.1.0, ruopus-0.1.2, rusty_vp9-0.1.3}.md all present |
| 5 | "the codec-audio-opus and codec-video-vp9 rows … still read `blocked`" / ProRes-DNxHD "`not_started`" | :92-97 | FALSE (stale) | current matrix: codec-video-vp9 = implemented x3 (:27 row block), codec-audio-opus = implemented (linux partial), codec-video-prores-dnxhd = DNxHD working subset (matrix:581-591, refreshed 2026-11-09) — ADR drift note predates/sidesteps the refresh it demanded |
| 6 | "no binaries are distributed … source-only" | :55-58 | REAL | repo tree: no installer/bundle/AppImage artifacts; release lane builds source (baseline cargo-build-release) |
| 7 | per-codec patent postures ("MP3 patents expired ~2017" etc.) | :130-160 | PAPER-ONLY by design | legal assertions tagged [VERIFY: counsel]; nothing to verify in code, and correctly not claimed as verified |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| ADR:92-97 matrix rows "still blocked/not_started" | feature-matrix.toml rows now implemented + DNxHD partial notes | matrix is true (rows refreshed per its own notes); ADR paragraph is stale (S4) |

## 4. Fabrication / inflation indicators
None — negative options kept, counsel ledger of 24 unanswered questions,
"NOT accepted" in bold. This is the anti-fabrication pattern.

Findings: F-001 [S4] ADR:92-97 drift paragraph itself stale vs current
matrix (rows it calls unrefreshed are refreshed). F-002 [S4] ADR:69-70
"per-codec Cargo features 'arrive with the registration card'" — card 7.K1
already landed external-engine feature gates (Cargo.toml lines 28-41);
only in-house gates are still pending. Legal content: PAPER-ONLY,
correctly labeled.
Commands: ls, rg, head of crates/codecs/Cargo.toml (exit 0 each).
