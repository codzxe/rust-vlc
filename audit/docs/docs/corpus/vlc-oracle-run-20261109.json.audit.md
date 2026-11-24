# CLAIM AUDIT: docs/corpus/vlc-oracle-run-20261109.json
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 8 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 6 / 1 / 0 / 0 / 0 / 0 / 1 |
| Internal contradictions found | 0 (all cross-counts agree) |

## 1. Blunt verdict
A 570-line differential record of a real VLC-3.0.23-vs-Lumen PCM decode
comparison. Every internal count reproduces exactly: 35 rows, 29 window
rows all u8/s16 with head_bytes=0 (bit-exact prefix) and shortfall ==
tail_bytes arithmetic on every sampled row, 6 RMS/peak rows all float32,
16 regen-identical, 20 fresh-equals-committed, tail bounds 96..16384 B
matching the findings text. It is cited as the oracle record by
tests/compat/compat-matrix.json and PERF-VS-VLC.md. The decode runs
themselves cannot be re-executed in this sandbox (no VLC, no corpus bytes),
so run-level claims are REAL-UNPROVEN — but nothing internal contradicts.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | `"decoded_ok": 35`, `"equals_ffmpeg_7.1.5_decode": 35` | :21-22 | REAL-UNPROVEN | `python3 json.load` -> rows=35; per-row md5s unique (dup md5: 0); re-running the decode harness blocked (no corpus bytes/VLC in sandbox). |
| 2 | `"sha256_identical_to_manifest": 16`, `"rebased_fresh_realization": 19` | :16-17 | REAL | `sum(regen_sha256_match)` -> 16 true; 16+19=35 = row count. |
| 3 | `"fresh_equals_committed_20261023": 20` | :23 | REAL | `sum(lumen_half_current)` -> 20 true. |
| 4 | finding 1: "29/35 (all pcm_u8 + pcm_s16le): VLC dump is a bit-exact prefix ... shortfall is an unflushed aout tail (96-16384 B)" | :566 | REAL | 29 window rows, fmts {u8,s16}; all `head_bytes=0`; sampled shortfall `lumen_fresh_bytes - vlc_bytes` == `tail_bytes` exactly (576/384/496/128/96/576); `min/max(tail)` -> 96/16384. |
| 5 | finding 2: "6/35 (all pcm_f32le): dump not a sub-window; per-sample RMS/peak recorded" | :567 | REAL | 6 rows with `rms_dev`, all fmt float32; rms+window row counts sum to 35, no row has both/neither. |
| 6 | runner: "tools/bench-vs-vlc cargo test --test lumen_pcm -- --ignored via LUMEN_DIFF_FIXTURES/LUMEN_DIFF_OUT" | :19-20 | REAL | `ls tools/bench-vs-vlc/tests/` -> `lumen_pcm.rs` exists. |
| 7 | oracle version pin `"VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)"` | :7 | REAL | Same version string as the pinned baseline family (docs/baselines/raw/linux/16-options-vlcrc.txt:1866 embeds "VLC Media Player - 3.0.23 Vetinari"); PERF-VS-VLC.md:8 cites this file as the version-line record. |
| 8 | random fixtures "double-run sha256 <h1> vs <h2>" (non-reproducible, disclosed) | :18 | UNVERIFIED | Re-run impossible in sandbox (anoisesrc unseeded); the honesty marker is plausible and consistent with `rebased_fresh_realization: 19`. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| 29/35 bit-exact prefix + 6/35 f32 deviations | docs/PERF-VS-VLC.md:17 "29/35 bit-exact, 6/35 f32 tolerance-FAIL" | Consistent — same counts from the same record. |
| worklog.md says "(571 lines)" | `wc -l` -> 570 | Off-by-one in the worklog prose, immaterial to evidence content. |

## 4. Fabrication / inflation indicators
None. The record discloses its own non-reproducibility (unseeded anoisesrc
double-run hashes) and records deviations rather than hiding them. The
`lumen_half_current: false` on 15 rows is kept, not pruned.

## 5. Matrix rows
None directly. Downstream: tests/compat/compat-matrix.json `oracle.record`
points at this file (verified path exists), making it the committed-oracle
backing for compat-matrix cells.
