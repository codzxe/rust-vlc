# AUDIT: tests/fixtures/generated/malformed/ogg_vorbis_8k_st__trunc1
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 40 B / 0aee05d223b0cc6a… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (data fixture; derivation independently re-executed) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

What it is: Ogg class-B truncation derivative, byte-exact prefix of
tests/fixtures/generated/valid/ogg_vorbis_8k_st.ogg (3,245 B): python
`trunc1 == parent[:40]` -> True. Magic `OggS`; 40 bytes = BOS page cut mid-body —
matching the committed consumer comment verbatim
(crates/demux/tests/ogg_fixtures.rs:455-461):
`// Cut mid first-page body (40 of 58 B): the BOS page's declared
 // extent runs past the stream — typed truncation at start.
 Derivative { name: "trunc1", ... terminal: "demux.ogg.truncated-page", fails_at_start: true }`
— file is exactly 40 bytes, verified.

Consumer: crates/demux/tests/ogg_fixtures.rs OGG_DERIVATIVES table (name "trunc1"
at :462 region) asserting the typed truncation at start. Generated-vs-handmade:
xtask fixtures family. Matrix-evidence: none.
S4: absent from CORPUS.md/MANIFEST.toml ledger — no drift gate (same family gap).
