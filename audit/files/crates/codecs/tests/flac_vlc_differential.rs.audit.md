# AUDIT: crates/codecs/tests/flac_vlc_differential.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 253 / 6 helpers + 1 test |
| File verdict | REAL (Lumen-vs-ffmpeg half executed and asserted in the baseline; VLC half honestly PENDING-INFRA — no vlc binary exists in this sandbox, verified) |
| Reachable from shipped entry point? | N/A (test binary). The decode path under test (`codecs::audio::flac`, claxon engine) is CLI-reachable |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `which vlc cvlc` (absent, exit 1); rg baseline + reference data; sed reads |

## 1. One-paragraph verdict
An honestly-designed pending-infra differential. The always-on half is real and ran in the baseline (0.12s, :1097-1099 ok): three committed 16-bit FLAC fixtures are decoded through the product path and their PCM MD5s asserted against the ffmpeg-recorded reference (`tests/data/flac_bitexact_reference.txt` — pcm_s16le_md5 entries verified present), so a broken FLAC decoder fails this test today. The VLC half is properly gated: `vlc_binary()` finds nothing (`which vlc cvlc` → nothing in this sandbox — I re-verified), each comparable row prints PENDING-VLC with the Lumen digest, s24 rows are recorded non-comparable with the exact reason (VLC afile has no s24 passthrough), and — the best part — the closing `assert_eq!(compared, if oracle.is_some() { 3 } else { 0 })` makes a PRESENT-but-unusable oracle (wrong version, failing dump) a hard test failure instead of a quiet degrade. Nothing is fabricated; the status header matches reality exactly.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:1095-1099: `flac_differential_vs_vlc_or_pending_infra ... ok` (0.12s — real decodes, no VLC).
- `which vlc cvlc` → not found (exit 1): the PENDING-VLC claim is true in this environment.
- Reference data exists: crates/codecs/tests/data/flac_bitexact_reference.txt:56/68/93 carry `pcm_s16le_md5 = …` per fixture (ffmpeg-authored, generation commands recorded in the file header).
- Oracle strictness verified by reading: version pin `3.0.23` substring check (:107), dump status check (:121-123), unusable oracle → compared=0 vs expected 3 → test FAILS (:252).

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/flac_vlc_differential.rs:59-64
Code:
```
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}
```
What is wrong: `is_file()` does not check the executable bit. A non-executable file named `vlc` earlier on PATH (or a $VLC pointing at a data file — that path also uses `is_file()` at :53) makes `oracle.is_some()`, every dump then fails to spawn, `compared` stays 0, and the strictness assert fails the suite — converting "no usable oracle" (the documented PENDING state) into a red build on a technicality. The failure mode is loud, not silent, so this is hygiene. What correct looks like: check executable permission (unix `MetadataExt::mode & 0o111`) or treat spawn-failure as pending. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| vlc_binary | 50-56 | REAL | S4 | $VLC override / PATH walk (F-001) |
| which | 59-64 | REAL | S4 | no exec-bit check (F-001) |
| vlc_dump_args | 69-85 | REAL-UNPROVEN | — | pinned 3.E3 invocation transcribed from the pinned build's --help; UNVERIFIABLE without a VLC binary |
| Row struct + vlc_oracle_md5 | 88-127 | REAL | — | version pin, typed Err reasons, temp-file cleanup (`let _ = remove_file` — acceptable for temp) |
| flac_differential_vs_vlc_or_pending_infra | 130-253 | REAL | — | Lumen digests computed + asserted vs ffmpeg reference ALWAYS; s24 rows non-comparable with reasons; strict compared-count assertion |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| flac_differential_vs_vlc_or_pending_infra | YES (FLAC decoder breakage) | the three `assert_eq!(lumen, expect["pcm_s16le_md5"])` run unconditionally (:158-161); would also fail if a present oracle misbehaves (:252) — but the VLC-comparison half itself cannot fail while no VLC exists (that is the documented pending state, not a fake pass) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The sandbox carries no vlc/cvlc binary (verified this session)" | :8-9 | TRUE (re-verified by me) | which vlc cvlc → exit 1 |
| "the Lumen-side digests are computed for real here and asserted against the ffmpeg oracle" | :14-16 | REAL | baseline pass + reference file present + unconditional asserts |
| "VLC's afile output speaks u8|s16|float32 — no s24 passthrough" | :26-27 | REAL-UNPROVEN | consistent with VLC 3 afile documentation from knowledge; no binary to check |
| "the moment the oracle exists, all three digests must agree" | :18-19 | REAL (mechanism) | strict compared==3 assert when oracle present |

## 7. Dead / duplicated / unreachable code
None. The `Row.comparable`/`vlc_md5` Option dance is load-bearing (drives the report + strictness).

## 8. Dependencies & flags
Test-target; shares `mod common` (md5) and `mod flac` (decode_all/fixture helpers — other agents' files) with flac_bitexact.rs. No features. `$VLC` env override documented.

## 9. Open questions for the next agent
- flac/mod.rs (shared helper) auditor: confirm `decode_all` uses the real `FlacDecoder` contract path (the bitexact suite's 4/4 baseline pass suggests yes).
- When a VLC 3.0.23 binary is available (3.E3 harness), re-run: the suite will then execute the three-way digest comparison for the first time.
