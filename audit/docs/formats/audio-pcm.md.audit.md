# CLAIM AUDIT: docs/formats/audio-pcm.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 6 / 1 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
The first-vertical audio codec record. All code citations verify (pcm module + 39 tests +
fuzz target). Matrix row already implemented, with evidence strings that include honest
tolerance-FAIL rows for f32 — the record consolidates rather than inflates.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "status = implemented (unpacked-PCM decode; in-house per D-018 ... NOT symphonia-codec-pcm" | L9-11 | REAL | crates/codecs/src/audio/pcm/{mod,decoder,error,fuzz}.rs exist; Cargo.toml pcm gate has no symphonia dep (self-built) |
| 2 | "fuzz_target = fuzz_codec_pcm" | front-matter | REAL | fuzz/fuzz_targets/fuzz_codec_pcm.rs + crates/.../pcm/fuzz.rs exist |
| 3 | "crates = [] — self-built, std-only, no unsafe" | front-matter | REAL | module present; unsafe-free claim consistent with workspace lints (deny) |
| 4 | matrix row claim: "row status implemented on win/linux/macos, not_started on web, with evidence crates/codecs/src/audio/pcm/**" | L6-9 | REAL | matrix codec-audio-pcm: `status = { windows = "implemented", linux = "partial", macos = "implemented", web = "not_started" }` — note linux is "partial" (f32 tolerance rows), slight divergence from the record's summary wording |
| 5 | "phase = 2 (first vertical, the audio path of the AVI+MJPEG+PCM slice)" | front-matter | REAL | baseline rc-smoke: headless-decode AVI/MJPEG/PCM PASS; engine-info decoders: mjpeg, pcm_s16le |
| 6 | "card 2.B5: full format matrix, bit-exact AC" | matrix evidence | REAL-UNPROVEN | tests/integration/first-vertical/evidence/ledger.json cited by matrix; not re-run (baseline covers engine tests passing) |
| 7 | "compat-matrix-20261110.md (run §5, the 6 f32 tolerance-FAIL rows + the 29 bit-exact integer rows)" | matrix evidence | REAL | file exists (docs/corpus/compat-matrix-20261110.md, 333 lines); honest FAIL rows cited as evidence — good discipline |

## 3-5. Contradictions / fabrication / matrix rows
One nuance: record says implemented on win/linux/macos; matrix linux = partial (f32
tolerance). Matrix wins (it encodes the tolerance verdicts). Commands: ls pcm; rg -c tests;
matrix sed; baseline citations.
