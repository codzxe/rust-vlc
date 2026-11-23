# AUDIT: crates/subtitles/tests/fixtures/fuzz/bom_utf8_on3.srt
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-24 |
| File verdict | REAL (exact UTF-8-BOM graft onto out3.srt; exercised by the committed-corpus survival test) |
| Reachable | N/A (test fixture) — consumed namelessly by srt_fuzz_survival.rs:56-71 `committed_corpus_survives` (read_dir over `tests/fixtures/fuzz/`, asserts ≥30 files) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` → "SubRip, Unicode text, UTF-8 (with BOM) text, with CRLF, LF line terminators"; `od` hexdump; python3: `f == b'\xef\xbb\xbf' + out3` → True; `cargo test -p subtitles --test srt_fuzz_survival` → 4 passed |

## 1. Verdict
151 bytes = `ef bb bf` + out3.srt byte-for-byte (python-verified). Content underneath: a 1 ms cue (`00:00:00,000 --> 00:00:00,001`, "A"), a multiline CRLF cue (`00:10:00,000 --> 00:11:23,456`, "Line one\r\nLine two"), and a 24 h+ rollover cue (`23:59:59,900 --> 24:00:00,100`) — so the graft puts a UTF-8 BOM in front of the hour-rollover edge sample, exercising BOM-vs-UTF-16-detection disambiguation (a naive `ff fe`/`ef bb bf` sniffer must not confuse the two). Matches the documented corpus scheme ("BOM grafts"); the survival test passes with it in the corpus. No defects.
