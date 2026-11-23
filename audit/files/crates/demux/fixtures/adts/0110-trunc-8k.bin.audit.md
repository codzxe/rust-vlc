# AUDIT: crates/demux/fixtures/adts/0110-trunc-8k.bin (binary)
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-23 |
| Size / type | 8,192 bytes — `file`: "MPEG ADTS, AAC, v4 LC, 96 kHz, monaural" (verified) |
| File verdict | REAL (genuine truncation derivative; consumed by a green fuzz-lite test) |
| Reachable from shipped entry point? | Test fixture; consumed only by `crates/demux/src/adts/tests.rs:634` (`committed_mutation_fixtures_are_typed_or_recover_never_panics`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `file`; python3 byte-level ADTS walk (below); adts suite run (32 passed incl. the fixture test) |

## 1. Verdict
The bytes are exactly what the name promises: a corpus-lane 0110 derivative truncated at 8 KiB. Independent parse: 116 complete ADTS frames walk cleanly from offset 0 (frame 0 at 0/len 115, MPEG-4 LC, protection absent), and the final frame at offset 8148 declares 54 bytes while only 44 remain — a genuine mid-frame truncation, the exact damage class the fuzz-lite suite exists to exercise. `file` independently identifies it as MPEG ADTS AAC v4 LC 96 kHz mono. The consuming test (`adts/tests.rs:629-643`) reads it, asserts non-emptiness, and drives it through `fuzz_demux_adts` — which passed in my run (no panic; the truncation surfaces as the demuxer's typed sticky refusal internally).

## 2. Evidence
- `file` -> "MPEG ADTS, AAC, v4 LC, 96 kHz, monaural"; 8192 bytes (the 8 KiB cut).
- python3 walk: `final frame at 8148: flen=54 but only 44 bytes remain -> TRUNCATED; complete frames: 116, last complete end: 8148`.
- Consumed by adts/tests.rs:634 inside the four-fixture loop; the test ran green in my execution (`32 passed` adts filter).
- sha256 prefix `0b2428d2ab5021c2` (recorded here for future digests; no in-tree digest pin exists for these derivatives — see finding).

## 3. Finding

F-001 [S4]
Where: crates/demux/fixtures/adts/0110-trunc-8k.bin (provenance), consumed at crates/demux/src/adts/tests.rs:631-640
Code (the consumer):
```rust
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/adts");
    for name in [
        "0059-trunc-4k.bin",
        "0110-trunc-8k.bin",
```
What is wrong: unlike the ASF fixture family (which pins sha256 prefixes in its digest map), these ADTS mutation derivatives carry no digest pin and no recorded generating command — their provenance ("derivative of corpus lane 0110 at cut 8192") lives only in the filename. A regenerated or accidentally-modified fixture would still pass the test (the test only checks non-emptiness + no-panic). The bytes are consistent with the claimed shape (verified above), so this is provenance hygiene, not fabrication. Confidence: HIGH — Verification: read + ran.
Confidence: HIGH — Verification: read + ran

## 4. Notes
- The three sibling fixtures (0059-flip-idx15, 0059-trunc-4k, 0059-zero-range) share the same unpinned-provenance pattern.
- Zero findings beyond the S4 provenance note; the binary does its job.
