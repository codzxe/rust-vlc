# CLAIM AUDIT: docs/corpus/compat-matrix-20261110.md (333 lines) + raw-rows.jsonl evidence
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Claims extracted | 9 |
| REAL 8 / REAL-UNPROVEN 1 / FALSE 0 | |
| Internal contradictions found | 0 (one timeline note, not a contradiction) |

## 1. Blunt verdict
Machine-rendered from a real run and it shows: I re-parsed the 166-line raw-rows.jsonl and
the headline counts are EXACT (clean corpus: 84 decoded + 22 decoded-partial + 6 refused +
0 demux-fatal; malformed: 54, all typed outcomes). The document's central act is
self-demotion: five implemented rows were flipped to partial (asf, aac-lc, alac, vp9, pcm)
with reason + next step, applied through a tool that hard-rejects self-verification
(compat_matrix.rs:444-446). The oracle differential repeats the vlc-diff-report's honest
29-bit-exact/6-f32-FAIL split with matching magnitudes. Every limitation (windows/macos
blocked, wasm decoder-less, lossy codecs not byte-compared) is stated, not hidden.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Corpus files: 166 (84 decoded, 22 decoded-partial, 6 refused, 0 demux-fatal; 54 malformed all-typed / zero crashes)" | :18-20 | REAL | python3 census of raw-rows.jsonl (166 lines): non-malformed rels -> decoded 84, decoded_partial 22, refused_demux 4 + refused_probe 2 (= 6 refused), demux_fatal 0; malformed rels -> 54 rows, statuses all typed (decoded 13 / refused_demux 20 / demux_fatal 8 / refused_probe 2 / decoded_partial 11). EXACT. |
| 2 | "Oracle differential ... 29 bit-exact (MD5-prefix), 0 f32 tolerance-pass, 6 f32 tolerance-FAIL" | :19 | REAL | §5 table: 35 rows; 29 "bit-exact prefix" + 6 "f32 tolerance FAIL" — same six fixtures (0022/0247/0280/0297/0305/0330) and same magnitudes as vlc-diff-report-20261023.md §Oracle run (0297: 8.22e-5/4.63e-2 exact; 0330: 6.43e-6/1.20e-3 exact; 0305 fresh-realization same magnitude). |
| 3 | "6 transitions APPLIED ... the tool re-checked each `from` status against the live matrix, rejected nothing" | §6 | REAL | compat-matrix-transitions-applied.jsonl = 6 lines (wc -l); the five §6.1 flips verifiable in the live matrix: container-asf-wmv status linux="partial" (sed :215), vp9 row notes cite "2026-11-10T20:07:51Z compat-matrix gap triage ... linux implemented->partial" (matrix :482). |
| 4 | "no transition proposed or applied `verified` — the tool rejects `to = \"verified\"` outright" | §8 | REAL | tools/xtask/src/cmd/compat_matrix.rs:444-446: `if first.to == "verified" { ... REJECTED — no self-verification path` (code verified). |
| 5 | §3 claimed statuses map to live matrix rows | :62-78 | REAL | Spot checks: asf row claims implemented w/m + now linux partial (:215, post-flip); flac-native claimed not_started x4 in §3 and the run refused it (consistent — §6.3 "an honestly **unclaimed** row"); avi/matroska/mp4 rows carry the compat-matrix citations in evidence (:203, :216). |
| 6 | Lanes: windows/macos "blocked-by-external — no result inferred"; web-wasm "steps" (build + test exit 0, decoder-less) | :40-58 | REAL (honest env limits) | Consistent with baseline: rc-smoke wasm build passes, decode-bench "no decoder in this build"; browser-matrix 2026-11-11 records web playback failures honestly (my earlier audit). apps/web/src/capabilities.rs "reports the split honestly" (cited; not re-read this pass — REAL-UNPROVEN at file level). |
| 7 | "Post-apply `cargo xtask feature-audit`: PASS — 252 rows, ... implemented 192→187 / partial 142→147" | §6 | REAL (at its date) — timeline note | Baseline feature-audit (2026-11-23) shows partial=147 EXACT but implemented=193 and 2 NEW violations (rows 23 h264/sw, 163 advanced.tsx) — later rows (e.g. the 2026-11-15 h264 play note, W16-EE vp9) added implemented cells and two evidence paths that don't exist. Not a contradiction of this doc: the drift is post-2026-11-10 work. |
| 8 | "0385-dynres-video.mkv: FAILED ... known permanent gap ... Next step: re-author the entry self-contained" | §7 | REAL (honest failure disclosure) | The §7 row records the ffmpeg regen failure with cause + next step; also disclosed in the provenance note as the W24-P P3-7 fix path. |
| 9 | Provenance note: surgical post-run edits listed; "the raw rows were never edited" | :12-18 | REAL (disclosure) | Raw rows parse as machine output (self-consistent census; deterministic fixtures' md5s match committed records); the edit list (P2-1, P3-4, P3-7) names each surgical change with owning audit file. |

## 3. Contradictions
None found. Timeline note only (claim #7): today's feature-audit FAILURE postdates this
document's PASS; the two R4 violations are in rows 23/163, neither touched by this run.

## 4. Fabrication / inflation indicators
None. The doc's headline writes the FAILING numbers (6 tolerance-FAILs, 8 demux_fatal
malformed rows in raw data, a FAILED regen row) and self-demotes five rows. §6.2's
"claimed-supported, no gap" section carefully does NOT upgrade anything to verified
("still not `verified`: CI billing blocked").
