# AUDIT: tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__trunc1.mp4.toml
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A10) |
| Date | 2026-11-24 |
| Lines / functions | 27 lines (schema/fixture/sha/derivation/error keys) |
| File verdict | REAL (pin verified by reproduction; note field unusually precise) |
| Reachable from shipped entry point? | As evidence: media-probe fixtures_matrix + codecs fuzz_survival + first-vertical recovery_matrix (tier-2 #[ignore]d); family drift-gate-pinned in MANIFEST.toml |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256sum + stat (pin match), python3 prefix + box-offset check, rg consumers |

## 1. One-paragraph verdict
Exact pin with a spot-checked derivation note: sha256 0730b80c…1425c4 and size 64 reproduced
against the fixture; the fixture is a byte-exact 64-byte prefix of the parent, and the note
"cut mid-mvhd (movie header)" is geometrically TRUE — the parent's `moov` starts at offset 32
and `mvhd` at 36, so a 64-byte cut lands inside the movie header box (bytes 36..63 are mvhd
body). Family is among the 28 MANIFEST.toml drift-gate pins. Same v0 union-contract defect as
its siblings.

## 2. What actually works (verified)
— verified by reproduction: `sha256sum …trunc1.mp4` →
`0730b80c4b7010c89acf82b2177d7a13c688232cce164d88d372704dcd1425c4` (== pin); `stat -c%s` →
`64` (== pin); python3: `t1 == parent[:64]` (pure prefix) with parent box walk (ftyp 28 B,
moov @32, mvhd @36). Consumers: media-probe fixtures_matrix (baseline :3670-3680 8/8 ok),
codecs fuzz_survival, recovery_matrix (tier-2, LUMEN_CORPUS-gated, #[ignore] by design —
not in the standard baseline run).

## 3. Findings
F-001 [S3] Where: :20-24 `expected = "typed-error-or-bounded-recovery"` — v0 union contract
(three permitted outcomes) weakens regression detection; the specific outcome for a
mid-mvhd truncation (typed parse refusal at `demux.mp4.*`) is knowable and pinned by demuxer
conformance, not by this machine-readable file. Confidence: HIGH — Verification: read.
(Set-wide defect: flip16/trunc1/trunc3 share it; migrate together when expectation/2 lands.)

## 4. Per-function table
N/A (data/evidence file); all keys verified, error union excepted (F-001).

## 5. Test honesty
Consumers read the RECORDED TOML (fixtures_matrix.rs:246) — fixture/pin mismatch fails loudly.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cut mid-mvhd (movie header)" | :18 | REAL | mvhd box spans 36..~136 in parent; 64-byte cut lands inside it (python3 box walk) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None (TOML).

## 9. Open questions for the next agent
None beyond the F-001 set-wide migration.
