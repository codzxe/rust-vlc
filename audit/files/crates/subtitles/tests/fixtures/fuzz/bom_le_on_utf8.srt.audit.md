# AUDIT: crates/subtitles/tests/fixtures/fuzz/bom_le_on_utf8.srt
| Field | Value |
|---|---|
| Auditor | W09-A09 | Date | 2026-11-30 |
| Type / size | text fixture (mutated SRT), 146 bytes (`file`: "Unicode text, UTF-16, little-endian text" — the BOM trap working) |
| File verdict | REAL (legitimate, exactly-named corpus member) |
| Reachable from shipped entry point? | NO — consumed only by `tests/srt_fuzz_survival.rs` directory walk |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `od -An -c` (byte dump); python equality check; `./target/debug/deps/srt_fuzz_survival-8c37ba1040decd00` → 4 passed |

## 1. Verdict
Byte-verified content: `FF FE` (UTF-16 LE BOM) prepended byte-for-byte to `canonical/out2.srt` (144 B) — python check `m == b'\xff\xfe' + out2` → True. The body is pure UTF-8/ASCII (`1 / 00:00:01,000 --> 00:00:04,000 / Hello world / 2 / 00:00:05,500 --> 00:00:09,250 / Second line here / 3 / 00:01:02,030 --> 00:01:02,050 / Quick flash cue`). The name says exactly what it is: a UTF-16LE BOM grafted onto UTF-8 content — the classic encoding-sniffing trap; `file(1)` itself mis-detects it as UTF-16. It is part of the 37-file corpus walked by `committed_corpus_survives` (executed: 4/4 pass — the parser must not treat the BOM as a UTF-16 marker and mangle the ASCII body).
## 2. Notes
- No by-name citation anywhere; collective citation tests/srt_fuzz_survival.rs:2-3 ("BOM grafts").
- Same UNVERIFIED generator-script note as the corpus at large.
