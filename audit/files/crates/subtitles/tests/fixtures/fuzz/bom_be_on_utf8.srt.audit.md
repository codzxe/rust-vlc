# AUDIT: crates/subtitles/tests/fixtures/fuzz/bom_be_on_utf8.srt
| Field | Value |
|---|---|
| Auditor | W09-A01 | Date | 2026-11-23 |
| Kind / size | binary-ish text fixture (mismatched BOM) / 146 B |
| File verdict | REAL |
| Consumed by | `srt_fuzz_survival.rs::committed_corpus_survives` (whole-dir read; baseline: 4/4 ok) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |

## Mutation encoded (byte-verified)
`fixtures/canonical/out2.srt` (144 B, 3 plain cues) with the two bytes `FE FF` — a **UTF-16 big-endian BOM — prepended to UTF-8 text** (`bom == b"\xfe\xff" + out2` verified programmatically). Sibling `bom_le_on_utf8.srt` is the `FF FE` variant; this is the BE graft. The name is accurate ("BE BOM on UTF-8").

## Executed outcome (probe, exit 0)
```
bom_be_on_utf8.srt: cues=0 damages=1 kinds=["GarbageBlock"] texts=[]
```
The BOM wins the encoding sniff → UTF-16BE decode of ASCII text → mojibake → one typed `GarbageBlock`, zero cues, no panic. Honest degrade for an encoder-mismatch input no real toolchain produces.

## Finding
F-001 [S4] The fixture pins whole-file-loss behavior, but NO test asserts the specific outcome (`GarbageBlock`, 0 cues) — `committed_corpus_survives` only checks the generic invariants, and the pinned-damage asserts for BOM cases live (if anywhere) in the srt unit suite (not verified — outside my file list). The corpus member is real; its semantics are unpinned here. Blast radius: none (documentation of intent only).
