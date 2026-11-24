# AUDIT: tests/fixtures/generated/malformed/wav_pcm_s16le_44k1_st__trunc1
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 24 bytes / 8b2e94b7c155a3bfc372335bbebc9156e389de9a3f3747386cf5b3f4d4bbb002 |
| Type | RIFF WAVE header fragment (fmt declared 16B, only 10 present) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 prefix check; rg consumers (exact-name AND format-string) |

## 1. Verdict
24-byte exact prefix of parent `generated/valid/wav_pcm_s16le_44k1_st.wav` (python3: `t == w[:24]`, parent 35,358 B): `RIFF` size 0x0000_8a16, `WAVE fmt ` size 16 declared, only 10 bytes of fmt follow → cut mid-fmt. Consumed ONLY via the runtime-composed string `format!("generated/malformed/wav_pcm_s16le_44k1_st__{}", deriv.name)` (crates/demux/tests/wav_aiff_fixtures.rs:453; `trunc1` row at :379-382 pins terminal `demux.wav.truncated`). rg for the literal filename finds NOTHING — see F-001.
## 2. Findings
F-001 [S3]
Where: whole file vs repo tooling
What is wrong: (a) no MANIFEST.toml row (28-entry manifest does not cover the wav/aiff family — drift gate blind); (b) exact-name searchability is zero (name composed at runtime) — a stale/missing file fails at test runtime via testkit must(), which is acceptable, but there is no committed ledger row (CORPUS.md table does not list this family). Why it matters: `cargo xtask fixtures --check` verified only 55 of 164 files under tests/fixtures (audit/runs/baseline/full_gate.txt:54), so this file's integrity is unchecked by any gate. Confidence: HIGH — Verification: ran (rg exact name → no match; python3 MANIFEST scan → NOT IN MANIFEST).
## 3. What actually works (verified)
python3 prefix equality; consumer test `wav_aiff_fixtures.rs` passes in baseline (cargo-test-workspace.txt:2436-2441, 8 tests ok, incl. derivative pinning).
