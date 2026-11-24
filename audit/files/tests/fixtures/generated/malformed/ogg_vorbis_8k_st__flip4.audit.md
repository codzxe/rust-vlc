# AUDIT: tests/fixtures/generated/malformed/ogg_vorbis_8k_st__flip4
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 3245 bytes |
| File verdict | REAL (derivation verified byte-level) — the NAME misleads about k |
| Reachable from shipped entry point? | NO (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | python3 XOR diff vs parent (exit 0); rg consumer (exit 0) |

## 1. Verdict
Genuine Ogg derivative: byte head `4f676753 00020000` (`OggS` v0). VERIFIED against parent
generated/valid/ogg_vorbis_8k_st.ogg: exactly ONE differing byte (offset 2810), a single-bit
XOR — despite the name promising k=4 flips (the AVI/MP4 families' flip16 really is 16 flips;
this OGG family's flip1/flip4/flip16 each carry exactly ONE flip at different recorded
offsets — 68 / 2810 / 30, verified by me for all three). sha256 9aab807cb8551332...
(recomputed).

## 2. Consumers
crates/demux/tests/ogg_fixtures.rs DERIVATIVES table (name "flip4", verified comment:
"One flip at EOS-page body offset 2810: the audio page's CRC lies — dropped, its 14 packets
are lost, the walk ends clean") with pinned `bytes: 0, packets: 0, reasons: &["ogg: page
CRC mismatch"], damage: 442` — the demux test code is HONEST about the single flip; only
the filename follows the family naming convention. Consumer test: the ogg mutated-family
walk (baseline: demux PACKAGE exit_code=0 — ran and passed).

## 3. Findings
F-001 [S3] Misleading name: `__flip4` contains 1 flip, not 4. Anyone reasoning from the
filename (or cross-referencing the AVI/MP4 flip-k contract "k-prefix of one shuffle")
believes 4 bits were flipped. The test table + comments document the truth; the name lies.
Confidence: HIGH — ran (byte-verified all three ogg flipN files).
F-002 [S3] Same systemic manifest-gap: outside the 28-file drift gate; no generator row.
Confidence: HIGH.

## 4. Open questions
None.
