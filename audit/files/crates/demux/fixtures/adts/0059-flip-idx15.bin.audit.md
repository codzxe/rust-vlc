# AUDIT: crates/demux/fixtures/adts/0059-flip-idx15.bin (binary asset)
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-02-13 |
| Type / size | `MPEG ADTS, AAC, v4 LC, 88.2 kHz, monaural` (`file`) — real ffmpeg-encoded ADTS (carries `Lavc61.19.101` encoder tag at offset 12); 2,048 bytes |
| SHA-256 | `4454f429830534fe2898591cd0c768227aed2ac1a761b5b32aa93649c0fd29a2` |
| File verdict | REAL (genuine mutated corpus derivative; evidentiary value limited to panic-freedom) |
| Reachable from shipped entry point? | N/A (test asset); single consumer: crates/demux/src/adts/tests.rs:635 (`committed_mutation_fixtures_are_typed_or_recover_never_panics`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `file` (exit 0); `sha256sum` (exit 0); `od` head (exit 0); `cargo test -p demux --lib committed_mutation_fixtures` → 1 passed (exit 0) |

## 1. One-paragraph verdict
A genuine ADTS/AAC fixture: `ff f1` sync, MPEG-4 LC profile, ffmpeg `Lavc61.19.101`
encoder bytes — a real encoded stream (the 0059 corpus lane, 88.2 kHz mono per
docs/perf/breadth.md:55), mutated by one bit flip near index 15 (the name's `flip-idx15`)
as a fuzz-lite derivative. Its ONLY consumer feeds it to `fuzz_demux_adts` and asserts
existence + no-panic — no typed-outcome pin for this specific mutation.

## 2. What actually works (verified)
— `cargo test -p demux --lib committed_mutation_fixtures` -> 1 passed (all four adts
  mutation fixtures drain through the real ADTS fuzz harness without panic).
— Hexdump confirms real ADTS structure (`ff f1 44 78 ...` + Lavc tag), not synthetic
  filler.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/adts/tests.rs:630-641 (consumer)
Code:
```
        let path = std::path::Path::new(dir).join(name);
        let bytes = std::fs::read(&path).must();
        assert!(!bytes.is_empty(), "{name} exists");
        super::fuzz::fuzz_demux_adts(&bytes);
```
What is wrong: The fixture's entire assertion is existence + panic-freedom. Whether
`0059-flip-idx15` produces typed errors, recovery events, or a specific packet count is
unpinned — the four derivatives are interchangeable from the test's point of view, and
a behavioral regression (e.g. the flip no longer being detected as damage) would pass.
Confidence: HIGH — Verification: read + ran the test.

## 4. Usage map
- adts/tests.rs:635 — the only in-tree reference (rg over crates/apps/tools/docs).
- Provenance: derived from the 0059 corpus lane (docs/perf/breadth.md:55 "AAC-LC 0059
  (88.2 kHz mono)"); the parent corpus file is 0059-audio-only.aac
  (docs/perf/raw/r1/probe.csv:22).

## 5. Asset facts
2048 bytes; ADTS main header at 0: sync fff, ID 0 (MPEG-4), layer 0, protection_absent
1, profile 01 (LC), sampling_index 4 (88.1 kHz per `file`), channel config 1 (mono).
Consistent with the claimed corpus lane.
