# CLAIM AUDIT: docs/formats/audio-speex.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-S2) |
| Date | 2026-11-24 |
| Claims extracted | 6 |
| REAL 6 / FALSE 0 / UNVERIFIED 0 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A pure gap record — and every claim in it is negative, checkable, and true: the row is
blocked x4 with the exact reason string, the feasibility survey says NOT_FOUND, the
tree carries no Speex decoder anywhere (only the Ogg carriage mapping), and nothing is
admitted (no fuzz target, no crates, no fixtures). This is what an honest "not done"
record looks like.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Row codec-audio-speex status `blocked` ×4, phase_target = 8, priority P3, same blocked_reason | :7-8,:28-29 | REAL | docs/feature-matrix.toml:834-844 — `id = "codec-audio-speex"`, priority P3, phase_target 8, `status = { windows="blocked", linux="blocked", macos="blocked", web="blocked" }`, `blocked_reason = "No pure crate (speex NOT_FOUND 2026-10-19); legacy VoIP; P3 (feasibility audio-codecs §1 row 17)."` — byte-exact match to the record. |
| 2 | Feasibility §1 row 17: "None. crates.io `speex` NOT_FOUND (re-checked) [RV] … Legacy VoIP streams stay `missing`; P3" | :25-27 | REAL | docs/feasibility/sections/audio-codecs.md:49 `\| 17 \| Speex \| P3 \| None. crates.io \`speex\` NOT_FOUND (re-checked) **[RV]** \| blocked (later) \| Legacy VoIP streams stay \`missing\`; P3 \|` — quoted 1:1. |
| 3 | No decoder in-tree; only Ogg carriage side (OggCodec) | :19-21 | REAL | rg -l speex crates/ -> only crates/demux/src/ogg/mod.rs (`OggCodec::Speex => "speex"` carriage mapping, :156); no decoder crate/module exists. |
| 4 | container-ogg.md documents the speex mapping family | :20-21 | REAL | docs/formats/container-ogg.md:35 "Codec mappings carried: vorbis, opus, theora, flac-in-ogg, speex," ✓. |
| 5 | fuzz_target = none; crates = []; self_built = [] (nothing admitted) | :11-13 | REAL | No fuzz_codec_speex in fuzz/fuzz_targets (ls shows the family: aac, adpcm, alac, av1, flac, h264, hevc, mjpeg, mp2, mp3, pcm, vorbis — no speex) ✓. |
| 6 | No corpus fixtures, no results; matrix unchanged | :40-42 | REAL | No speex fixture anywhere in tests/fixtures (rg on fixture tree: 0 hits beyond the ogg mapping). |

## 3. Contradictions
None. Record, matrix row, and feasibility survey agree on every point.

## 4. Fabrication / inflation indicators
None — a record whose entire content is disclosed absence, cross-verified.

Commands run: rg -A10 codec-audio-speex matrix; rg speex in feasibility audio-codecs + crates/ + ogg mod.rs + container-ogg.md; ls fuzz/fuzz_targets (no speex target).
