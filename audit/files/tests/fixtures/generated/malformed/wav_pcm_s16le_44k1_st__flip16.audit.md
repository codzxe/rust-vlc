# AUDIT: tests/fixtures/generated/malformed/wav_pcm_s16le_44k1_st__flip16
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 35,358 B / 2e26a85e1e0577dc9cc046adca767b0bc56c7581dabda78cc6f4bbd98383cdec |
| File verdict | REAL (data fixture; derivation independently re-executed) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

Assignment rows 3026-3030 (5 chunks -> one audit). What it is: WAV (RIFF/WAVE,
fmt PCM s16le 44.1 kHz stereo + LIST INFO) class-B malformed derivative of
tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav with 16 seeded header
bit-flips (extension-less name, family convention).

Derivation re-executed (python vs valid parent): 15 differing bytes at offsets
[6,20,38,41,56,97,122,146,147,151,192,218,236,241,252] — byte 6 flips RIFF size
bit 0x80 (`16 8a 80 00` vs `16 8a 00 00`), byte 20 flips fmt wFormatTag 0x01 ->
0x41 'A', byte 38 flips the LIST chunk id ('LIST' -> 'LI[T'), etc. That is 15
bytes changed = 16 bit flips when one byte takes two flips; consistent with the
k=16 seeded-flip op, and with the consumer's pinned expectation
(crates/demux/tests/wav_aiff_fixtures.rs:440-446):
`// Sixteen flips, fmt tag among them: typed refusal.
 WavDerivative { name: "flip16", bytes: 0, reasons: &[], terminal: "demux.wav.unsupported-format", fails_at_start: true }`
— byte 20 (wFormatTag=0x41) is exactly the flip that pins this refusal; verified
in bytes. Consumer: crates/demux/tests/wav_aiff_fixtures.rs:453 builds
`generated/malformed/wav_pcm_s16le_44k1_st__{name}` and asserts the pinned
outcome.

Generated-vs-handmade: generator-derived (same xtask fixtures family; k-prefix
property flip1<flip4<flip16 documented tools/xtask/src/cmd/fixtures.rs:2465).
NOT in tests/fixtures/CORPUS.md's 28-row ledger nor MANIFEST.toml (28 `path =`
entries, rg exit 1 for this name) — the WAV/AIFF/OGG/other families are outside
the `xtask fixtures --check` drift gate. S4: ledger coverage gap (blast radius:
silent byte-drift in these fixtures would not be caught by the gate).
Matrix-evidence: none for this file (only the valid parent appears in
docs/corpus/compat-matrix-20261110.raw-rows.jsonl:79).
