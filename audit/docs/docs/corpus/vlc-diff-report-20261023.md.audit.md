# CLAIM AUDIT: docs/corpus/vlc-diff-report-20261023.md (420 lines)
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 12 |
| REAL 7 / REAL-UNPROVEN 2 / FALSE 0 / PAPER-ONLY 0 / UNVERIFIED 3 (byte-level, corpus absent) | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The most scrupulous evidence document audited in this chunk. I re-parsed the entire
2026-11-09 oracle table programmatically: 35 rows, 29 MATCH-prefix, 6 TOLERANCE-FAIL, and
EVERY arithmetic invariant holds — all 29 "tail N B unflushed" cells equal Lumen B − VLC B,
and all 6 f32 verdicts are consistent with the ratified policy (RMS ≤ 1e-4 AND peak ≤
1e-3 failing). The committed JSON evidence artifact matches the markdown's re-verification
column exactly (6/6 rows, including the three deterministic-fixture reproductions).
Failures are headline material, not footnotes: "0/6 f32 rows PASS" sits in the title. The
2026-10-23 PENDING-VLC table is preserved as history and labeled as such.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "29/35 parity: 29 bit-exact prefix + 0/6 f32 tolerance-pass" | :1 | REAL | python3 regex parse of the 2026-11-09 results table: 35 rows = 29 MATCH-prefix + 6 TOLERANCE-FAIL, 0 parse failures. Command run, output pasted in audit log. |
| 2 | "the entire shortfall is a tail of 96–16384 B left unflushed" (per-row tail sizes) | results table | REAL | Programmatic check: for all 29 MATCH-prefix rows, tail cell == Lumen B − VLC B (0 mismatches; min 96 row 0121, max 16384 row 0194). |
| 3 | f32 policy "RMS ≤ 1e-4 (−80 dB) AND peak ≤ 1e-3 (−60 dB)" passes 0/6 | §6.6 | REAL | Recomputed from the table's RMS/peak cells: 0297 (RMS 8.22e-5 ✓ but peak 4.63e-2 ✗) and 0330 (RMS 6.43e-6 ✓ but peak 1.200e-3 ✗) fail on peak alone; other four fail both. Verdicts consistent, none pass. |
| 4 | "preserved in `docs/corpus/vlc-oracle-run-20261109.json`" + "docs/corpus/vlc-f32-tolerance-20261109.json, schema `corpus/vlc-f32-tolerance/1`" | §6.6 | REAL | Both files exist (ls docs/corpus/); python3: vlc-oracle-run-20261109.json schema "corpus/vlc-oracle-run/1"; vlc-f32-tolerance-20261109.json schema "corpus/vlc-f32-tolerance/1" (exact). |
| 5 | JSON deviation values match the md's "re-verified this card" column | §6.6 table | REAL | python3 compare: JSON rms_dev/peak_dev == md fresh-realization values EXACTLY for 0247 (1.2376e-2/5.0894e-2), 0280 (8.7608e-3/3.2807e-2), 0305 (1.2094e-1/5.4242e-1); the 3 deterministic rows (0022/0297/0330) match the 11-09 run values exactly. 6/6 consistent. |
| 6 | "recomputed with `scripts/vlc_f32_deviation.py`" | §6.6 | REAL | `ls -la scripts/vlc_f32_deviation.py` -> 7,822 bytes, present. |
| 7 | "committed `corpus-big-manifest-20261022.json`" as argv record | Input corpus section | REAL | docs/corpus/corpus-big-manifest-20261022.json exists (ls). |
| 8 | Lumen-side pins "avi_mjpeg_pcm.avi 08db7ccc... cross-verified by a Python RIFF walk ... (two implementations agree)" | :313 | REAL | Same MD5 constants live in tools/bench-vs-vlc/tests/lumen_pcm.rs:49,56 (sed-verified); independently appear as runtime output in compat-matrix-20261110.raw-rows.jsonl rows 3,4,6,7. Three independent artifacts agree. |
| 9 | "`cargo test --test lumen_pcm -- --ignored` driven via LUMEN_DIFF_FIXTURES/LUMEN_DIFF_OUT over the 35 rows — 35/35 decode ok" | Run-book section | REAL-UNPROVEN | The driver mechanism exists (tests/lumen_pcm.rs:530-560, env-driven #[ignore]d test; run.py exists); this driven run is recorded here but not re-executed by me (cargo-build-budget). |
| 10 | "16/35 regenerate sha256-identical ... 19/35 embed runtime-random lavfi sources" | Input corpus section | UNVERIFIED (byte-level) | corpus-big absent from this machine (checked by the doc; also absent now); the double-run sha256 proof (6b7a4f3a... vs aee4767e...) is plausible and 16+19=35, 20+15=35 internally consistent, but the bytes cannot be re-derived offline. |
| 11 | "VLC 3.0.23 obtained ... executed" (oracle acquisition via apt download, version asserted 3.0.23 Vetinari 3.0.23-2-0-g79128878dd) | Oracle acquisition section | REAL-UNPROVEN (no VLC in this sandbox) | `which vlc` not re-run; acquisition narrative internally consistent, version string recorded twice. Cannot re-execute the oracle here (no VLC, no network) — UNVERIFIED at binary level, recorded as such. |
| 12 | 2026-10-23 table "kept as the historical record; 19/35 of its inputs are lost ... Nothing is mocked" | :3-5 | REAL (honest framing) | The :13-51 PENDING-VLC table retains literal "PENDING-VLC" cells for all 35 rows — nothing was back-filled to fake oracle digests; the real oracle columns live only in the 11-09 section. |

## 3. Contradictions
None found. Cross-checks: the .lumen.json/.lumen.fixtures.tsv siblings exist; the run's
Lumen digests for un-starred rows equal the 2026-10-23 committed digests in the historical
table (spot-checked 0022/0096/0121/0155/0308 — same MD5 strings in both tables), exactly
as the "20/35 rows fresh == committed" note predicts for deterministic inputs.

## 4. Fabrication / inflation indicators
None — the opposite. Inflation vectors were available and refused: the title reports the
FAILING score (29/35, not "35 comparable" or "0 decode differences"); the 6 f32 gaps are
labeled "genuine parity gaps ... NOT hidden"; the lost 2026-10-23 inputs are disclosed
with a double-run proof rather than silently re-based; and the README gate that failed
("byte-counts equal") is answered in the negative ("does not hold as stated — 35/35 VLC
dumps are shorter") with cause analysis.
