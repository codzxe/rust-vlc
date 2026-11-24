# AUDIT: fuzz/corpus/fuzz_codec_alac/seed_cookie_badver.bin
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 26 B / 7783aa1d898eff8aed2d850af17d4f58d846c03da917c0f98f119b5dca4599ac |
| File verdict | REAL (as data); DEAD — no consumer, no twin, executed nowhere |
| Reachable from shipped entry point? | NO — fuzz corpus seed |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |

## 1. Verdict (corpus-seed audit)
A 26-byte ALAC-config-cookie seed ("badver" = bad version) intended for the `fuzz_codec_alac` target's `parse_alac_config` n-walk. Its bytes appear NOWHERE else in the repo (verified by a python repo-wide byte scan for its first 10 magic bytes: the only hit is the seed file itself) — unlike every other corpus seed of mine it has NO fixture twin, NO inline literal, and NO stable sweep that reads it. It is consumed only by the unwired nightly campaign and counted as "present" by the registry gate. Its bytes have never been executed by anything committed.

## 2. What was checked and how
- batch.py: 26 B, sha256 7783aa1d…, magic `000010000110280a0e02…`.
- Twin search: python scan over crates/codecs/{src,tests}, fuzz/ for the byte prefix -> only the seed file itself (ran).
- Consumer search: rg for the filename -> zero hits in crates/tests/tools/scripts (ran).
- Registry gate (existence only) run by me: exit 0 — "corpora present" counts this dead seed.

## 3. Findings
F-001 [S2]
Where: fuzz/corpus/fuzz_codec_alac/seed_cookie_badver.bin (the whole file)
What is wrong: the seed is real hostile material (bad-version cookie for the ALAC config parser) but is not referenced, duplicated, or executed anywhere — the ALAC fuzz-survival suite (crates/codecs/tests/alac_fuzz_survival.rs) mutates cookies from the m4a fixtures instead, and the corpus file's only "consumer" is the non-empty existence check in scripts/fuzz_registry_check.sh. Why it matters: this is the purest instance of the corpus-lane gap — committed fuzzing evidence that gates nothing and runs nowhere; the registry report calls the corpora "present", which a reader can misread as coverage. What correct looks like: the alac fuzz-survival sweep should read the corpus seeds (as netfuzz_sweep does its twins), or the nightly campaign should be wired.
Confidence: HIGH — Verification: ran (byte-scan + rg + registry check).
