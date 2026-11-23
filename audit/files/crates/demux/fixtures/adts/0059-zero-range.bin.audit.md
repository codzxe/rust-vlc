# AUDIT: crates/demux/fixtures/adts/0059-zero-range.bin (binary asset)
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-23 |
| Size / type | 2048 bytes; `file` → "MPEG ADTS, AAC, v4 LC, 88.2 kHz, monaural" (exit 0) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — committed test fixture; consumed only by adts/tests.rs:636 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `od` header/tail dumps; python3 frame walk; `rg` consumers + doc citations; baseline cargo-test-workspace.txt:1768 |

## 1. Verdict (short-form per binary-asset rule)
Real ffmpeg-authored ADTS bytes: first frame header `ff f1 44 40 46 9f fc`
decodes to MPEG-4 LC, sampling_index 1 (88.2 kHz), mono — matching `file`'s
identification — and the payload carries the encoder string "Lavc61.19.101"
(ffmpeg 7.1.x). Structure (python3 walk): 3 complete frames at offsets
0/564/836 (lengths 564/272/280) = 1116 bytes, then a 932-byte corrupt tail
starting with a zero run at 1116 and containing NO further 0xFFF0 syncword —
the "zero-range" mutation shape. md5 7f1b1fb454e450ee000acef86cdbe9e4,
sha256-16 19c4d184fd30fbc5.

## 2. Usage honesty
Used only by `committed_mutation_fixtures_are_typed_or_recover_never_panics`
(adts/tests.rs:629-643): asserts existence, non-emptiness, and
`fuzz_demux_adts` survival — the absence-of-panic contract, honestly labeled.
That test ran and passed in the baseline (cargo-test-workspace.txt:1768).
No doc/matrix row cites this .bin directly (docs cite the parent
0059-audio-only.aac, e.g. docs/formats/container-adts.md:45) — no inflated
evidence claims to audit. Nothing wrong with the asset.
