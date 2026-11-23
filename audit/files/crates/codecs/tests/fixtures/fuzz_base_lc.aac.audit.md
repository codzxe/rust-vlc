# AUDIT: crates/codecs/tests/fixtures/fuzz_base_lc.aac (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A10 (recovery: executed by W06-R1, 2026-11-23) |
| Date | 2026-11-23 |
| Type / size / sha256 | MPEG ADTS, AAC v4 LC, 22.05 kHz, mono / 2,500 bytes / 1027bc9b5bd43503dd257236d9b3603f89219c33f6ab3a0cc11e32c184d1029c (md5 a3b9728432998ff6135c5b8ed1ce7c1f) |
| File verdict | REAL (genuine, structurally clean ffmpeg-authored ADTS stream; the committed fuzz-survival substrate, live-exercised) |
| Reachable from shipped entry point? | N/A (test fixture; ADTS/AAC-LC is a shipped open path) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `stat`; `sha256sum`/`md5sum`; `ffprobe` (aac LC, 22050, 1ch, duration 1.067977s); python3 ADTS walk (23 frames, sizes 81-151, 2,500/2,500 bytes consumed, no desync); `./target/debug/deps/aac_fuzz_survival-f05bc8c1f3beb83c` -> 2/2 pass (exit 0, 0.52s) |

## 1. One-paragraph verdict
A genuine, truthfully-named fuzz substrate: `file` + ffprobe confirm ADTS
AAC-LC mono 22.05 kHz (the name's "lc" and container); my own ADTS walk
consumed all 2,500 bytes as exactly 23 well-formed frames (sizes 81-151 B,
no desync — a clean stream, which is what a mutation BASE must be), and
23 × 1024 / 22050 = 1.0676s matches ffprobe's 1.067977s duration to the
sample. The "fuzz_base" role is real: aac_fuzz_survival.rs:57/:110 mutate
these pristine bytes through the real `fuzz_aac_stream` harness (80 seeded
mutations + sync-storm/garbage-interleave cases; the authored record at
worklog:8897: 32 decoded / 48 typed refusals, codes ⊆ {aac.adts.param_
switch, aac.frame.decode, aac.stream.no_frames}, zero panics). I executed
the suite live: 2/2 passed in 0.52s. The size appears verbatim in the
committed budget ledger (aac_bitexact_reference.txt:77 "fuzz_base_lc.aac
2 500 B", total 40,920 B < the 200 KB mandate).

## 2. What actually works (verified)
- python3 ADTS frame walk (above) — 23/23 frames, full byte coverage.
- `./target/debug/deps/aac_fuzz_survival-f05bc8c1f3beb83c` ->
  `mutated_fixtures_never_panic_and_settle_typed ... ok`,
  `sync_storms_and_garbage_interleaves_never_panic ... ok` (2 passed,
  exit 0).

## 3. Findings
None. Container type, profile, channel count, rate, duration, frame
structure, budget-ledger size, and the fuzz-substrate role all verified;
the fixture is live-exercised.

## 4-9. (binary asset — short-form)
- Cited as evidence by: aac_fuzz_survival.rs:2-8/:57/:110 (the mutation
  substrate), aac_bitexact_reference.txt:77 (budget ledger),
  worklog:8897 (authored +FB record). Honest use throughout.
- Cross-file observation (not this file's defect): the fuzz-suite doc
  comment (aac_fuzz_survival.rs:3) calls the stream "16 kbps" while the
  file's measured average is ~18.7 kbps (ffprobe bit_rate 18727;
  2500 B / 1.068 s) — encoder-target vs ADTS-overhead actual; the
  fixture's own name makes no bitrate claim.
- Untouched on disk.
