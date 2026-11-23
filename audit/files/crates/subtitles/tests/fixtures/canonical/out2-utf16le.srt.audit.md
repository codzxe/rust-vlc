# AUDIT: crates/subtitles/tests/fixtures/canonical/out2-utf16le.srt
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-24 |
| File verdict | REAL (exact UTF-16LE re-encoding of out2.srt with BOM; consumed and pinned by passing tests) |
| Reachable | N/A (test fixture) — read by srt_canonical_differential.rs:122 (`canonical_utf16_variants_parse_identically`) and srt_fuzz_survival.rs:101 (seeded-mutation parent, every 3rd iteration) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` → "SubRip, Unicode text, UTF-16, little-endian text"; `od` hexdump; python3: `g.decode('utf-16') == a.decode()` → True; `cargo test -p subtitles --test srt_canonical_differential` → 6 passed; `--test srt_fuzz_survival` → 4 passed |

## 1. Verdict
290 bytes: `ff fe` BOM then out2.srt's exact content as UTF-16LE with LF-only separators (byte-verified by decode-equality against out2.srt). It exercises the encoding-detection path: the test asserts `le.encoding == SourceEncoding::Utf16Le` and then the identical three cues with exact µs times. As a mutation parent it feeds the 2000-iteration seeded loop. Provenance ("iconv of out2") is consistent with the byte evidence (perfect re-encode); both consuming suites pass when run. No defects.
