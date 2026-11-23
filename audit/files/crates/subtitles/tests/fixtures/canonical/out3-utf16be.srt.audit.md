# AUDIT: crates/subtitles/tests/fixtures/canonical/out3-utf16be.srt
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-23 (repo frozen) |
| Size / type | 298 B; `file`: "SubRip, Unicode text, UTF-16, big-endian text, with CRLF, LF line terminators" |
| Fixture verdict | REAL — byte-verified BOM'd UTF-16BE encoding of out3.srt |
| Cited as evidence by | srt_canonical_differential.rs:14 ("iconv of out3 (BOM'd UTF-16BE)"), :129 (`canonical_utf16_variants_parse_identically`); srt_fuzz_survival.rs:143 (`truncations_never_beat_the_parser` prefix sweep) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `od -A d -t x1z` (BOM check); `iconv -f UTF-16 -t UTF-8` + `cmp` vs out3.srt -> identical; `./target/debug/deps/srt_canonical_differential-0c3831619bbd85b6` -> 6 passed; `./target/debug/deps/srt_fuzz_survival-8c37ba1040decd00` -> 4 passed |

## 1. Blunt verdict
Real and exactly what its doc row claims. First two bytes are `fe ff`
(UTF-16BE BOM, od-verified), and `iconv -f UTF-16 -t UTF-8` of this file is
BYTE-IDENTICAL to canonical/out3.srt (`cmp` clean, my run) — the "iconv of
out3" provenance claim is executed-verified at the encoding layer. It parses
identically to out3 (differential test, 6/6 pass) and survives the full
prefix-truncation sweep (4/4 pass). Inherited caveat: the ffmpeg provenance
of the PARENT out3.srt (ffmpeg 7.1.5 `-c:s srt` from a truth VTT) is
UNVERIFIED — no truth inputs or generation script committed (W09-A02's
srt_canonical_differential.rs audit F-001, S3). That is a parent-file issue,
not a defect in this encoding derivative.

## 2. Verification
— od: `fe ff 00 31 00 0a 00 30 ...` = BOM + UTF-16BE "1\n00...".
— iconv roundtrip: `cmp` exit 0, output "ICONV_ROUNDTRIP_IDENTICAL".
— Both consumer test binaries pass (counts above).

## 3. Issues
None in the fixture.
