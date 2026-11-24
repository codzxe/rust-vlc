# AUDIT: fuzz/corpus/fuzz_codec_alac/seed_alac_mono16.m4a
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 24,051 bytes / d24ed4ad19b0285db3fc6b202afb7c0603f2bca396e040bde8bb7a26977afb7c |
| Type | ISO Media, Apple iTunes ALAC/AAC-LC (.M4A) Audio |
| File verdict | REAL-UNPROVEN (valid ALAC seed; consuming fuzz target never executed) |
| Reachable from shipped entry point? | NO — fuzz corpus seed |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256 batch; `file`; rg consumers |

## 1. Verdict
Genuine ALAC mono-16 seed (file(1) recognizes Apple iTunes ALAC/AAC-LC .M4A). Named to match target fuzz_codec_alac (fuzz/corpus/<target>/ convention, fuzz/Cargo.toml:306). rg over crates/tests/tools/scripts/docs: no source references the seed by name (seeds are consumed only by the libFuzzer corpus-dir convention). No committed fuzz run exists (no cargo-fuzz/libfuzzer mention in audit/runs/baseline/*) — the lane that would consume this seed has zero execution evidence.
## 2. Findings
F-001 [S3] fuzz corpus lane unexecuted in all committed baselines (see seed_wav_rf64 audit F-001; applies repo-wide to all 35 targets). Confidence: HIGH — Verification: ran (rg over baseline dir).
## 3. What actually works (verified)
`file` type output; sha256 recorded above.
