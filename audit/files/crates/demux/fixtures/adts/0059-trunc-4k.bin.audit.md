# AUDIT: crates/demux/fixtures/adts/0059-trunc-4k.bin (binary asset)
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-23 |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — test fixture; consumed by exactly one test: crates/demux/src/adts/tests.rs:633 (`committed_mutation_fixtures_are_typed_or_recover_never_panics`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> "MPEG ADTS, AAC, v4 LC, 88.2 kHz, monaural"; `ls -la` (4096 bytes); `od` head/tail (ADTS sync `ff f1`, encoder string "Lavc61.19.101"); md5sum -> `ad80b2a5453fab1e6ea82d7fab247638`; grep for references across crates/docs/xtask; `cargo test -p demux committed_mutation_fixtures` -> exit 0, 1 passed |

## 1. Verdict (short, per binary-asset policy)
A genuine 4096-byte truncated ADTS/AAC-LC mutation-derivative fixture (exactly 4 KiB, matching its name; real ADTS sync `ff f1` + ffmpeg Lavc61.19.101 encoder bytes — the repo's pinned ffmpeg 7.1.x family). It is one of a four-member committed mutation family (`0059-flip-idx15.bin`, `0059-zero-range.bin`, `0110-trunc-8k.bin` alongside it). Its sole consumer runs it through `fuzz_demux_adts` asserting typed-or-recover, never panic — the test passes under my run (exit 0). No doc/matrix row cites this file as evidence (grep over crates/, docs/, xtask/: only the test reference). The consuming test asserts only non-emptiness + fuzz survival — appropriate for a committed fuzz corpus member, and consistent with how the sibling fixtures are used.

## 2. What actually works (verified)
— verified by `cargo test -p demux committed_mutation_fixtures` -> `test adts::tests::committed_mutation_fixtures_are_typed_or_recover_never_panics ... ok` (my run, exit 0).

## 3. Findings
None. Zero findings checked: file type real (not a placeholder/empty/renamed text file), size matches the name's contract (4 KiB), content is a real AAC stream head, reference set is exactly one honest fuzz consumer, digest recorded above for future cross-checks.
