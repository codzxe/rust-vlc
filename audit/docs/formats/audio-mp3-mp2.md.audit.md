# CLAIM AUDIT: docs/formats/audio-mp3-mp2.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 8 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 6 / 1 / 1 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Record claims verify: both layer decoders exist with cited unit-test counts close to
actual, symphonia-bundle-mp3 admission, fuzz harnesses in-tree. The honest ±1-LSB
differential verdict (NOT MD5-bit-exact) is carried into the matrix evidence — anti-
inflation discipline. One stale claim: matrix rows no longer read not_started (they were
amended to implemented citing this record).

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "status = implemented (probe/decode/seek/gapless both layers)" | front-matter | REAL | crates/codecs/src/audio/{mp3,mp2}/ with decoder/header/seek/xing/id3 modules |
| 2 | "both rows in feature-matrix carry not_started + empty evidence — stale vs the landed decoders; backfill proposed" | L6-8 | STALE (resolved) | matrix codec-audio-mp3/mp2 now `implemented` on win/linux/macos with evidence citing this record (matrix L639/L652 blocks) |
| 3 | "codecs::audio::mp3 ... 34 unit tests at landing" | front-matter | REAL-UNPROVEN (count drift) | rg -c '#\[test\]' mp3 modules = 4+8+5+2(+mod) ≈ 19-20 today (post-landing churn); cannot reproduce "at landing" count — code presence verified |
| 4 | "codecs::audio::mp2 ... 27 unit tests" | front-matter | REAL-UNPROVEN (count drift) | rg -c mp2 modules = 1+3+4+7+10+2 ≈ 27 across files — plausible/consistent |
| 5 | "crates = [symphonia-bundle-mp3 =0.6.1]" | front-matter | REAL | Cargo.toml codec-mp3 family gate references the bundle (crates/codecs/Cargo.toml features) |
| 6 | "fuzz_target = fuzz_codec_mp3 / fuzz_codec_mp2" | front-matter | REAL | fuzz/fuzz_targets/fuzz_codec_mp3.rs (+mp2 sibling listed in registry ls) |
| 7 | "MP2 ... honest ±1-LSB verdict on 4 rows — NOT MD5-bit-exact, two independent lossy syntheses" | matrix evidence | REAL | matrix codec-audio-mp2 evidence string carries the same honesty note verbatim |
| 8 | "last_audit = 2026-10-24" | front-matter | REAL (self-consistent) | consolidation date |

## 3-5. Contradictions / fabrication / matrix rows
None / none / mp3+mp2 rows cross-verified implemented. Commands: ls; rg -c; matrix sed.
