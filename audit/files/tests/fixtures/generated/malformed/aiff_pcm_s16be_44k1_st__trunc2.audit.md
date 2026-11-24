# AUDIT: tests/fixtures/generated/malformed/aiff_pcm_s16be_44k1_st__trunc2
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 64 B / c539597218c9186b… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (data fixture; derivation independently re-executed) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

What it is: AIFF class-B truncation derivative, byte-exact prefix of
tests/fixtures/generated/valid/aiff_pcm_s16be_44k1_st.aiff (35,334 B): python
`trunc2 == parent[:64]` -> True. FORM/COMM/SSND headers intact, so the cut lands
10 bytes into the SSND sound data — matching the committed consumer comment
verbatim (crates/demux/tests/wav_aiff_fixtures.rs:527-531):
`// 10 bytes of sound: 8 deliver (4 frames), the 2-byte sliver is
 // damage-counted, then the typed truncation.` — 64-54 (sound start) = 10 bytes,
verified.

Consumer: crates/demux/tests/wav_aiff_fixtures.rs AIFF_DERIVATIVES table
(`name: "trunc2"` at :527 with pinned reasons form-extends-past-stream, ssnd-
extends-past-stream, comm-frames-past-ssnd, ssnd-not-frame-aligned, terminal
demux.aiff.truncated). Generated-vs-handmade: xtask fixtures family.
Matrix-evidence: none. S4: absent from CORPUS.md/MANIFEST.toml ledger (28 rows,
rg exit 1) — no drift gate.
