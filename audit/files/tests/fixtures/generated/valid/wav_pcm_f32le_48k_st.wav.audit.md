# AUDIT: tests/fixtures/generated/valid/wav_pcm_f32le_48k_st.wav
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 76892 bytes |
| File verdict | REAL (structure parsed byte-level by me) |
| Reachable from shipped entry point? | NO (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 RIFF walk + fmt parse (exit 0); rg consumers (exit 0) |

## 1. Verdict
Genuine IEEE-float WAV: `fmt ` 18 bytes (tag 0x0003, 2 channels, 48000 Hz, 32-bit), `fact` 4
(sample-count chunk — correctly present for non-PCM), `LIST` 26, `data` 76800 = 9600 frames ×
2ch × 4B. sha256 c440d4c88986b0f6... (recomputed). Structure internally consistent.

## 2. Consumers / provenance
- Consumers (rg, exit 0): apps/cli/src/containers/wav.rs:248 (`all_wav_fixture_flavors_open`),
  crates/transcode/tests/record.rs + crates/transcode/tests/wav_roundtrip.rs (f32 lane),
  crates/demux/tests/wav_aiff_fixtures.rs, docs/corpus jsonl/json.
- Provenance: outside MANIFEST.toml's 28-file drift gate; no generator row
  (same as wav_extensible audit — hand/agent-authored).

## 3. Findings
F-001 [S3] — same systemic manifest/drift-gate gap (CORPUS.md:10-11 claim FALSE for this
file). Confidence HIGH. Verification: ran (manifest parse + rg).

## 4. Open questions
- The transcode f32 round-trip tests consume this — the W-next transcode audits should
  confirm the digest pinning uses engine output, not the fixture's own bytes.
