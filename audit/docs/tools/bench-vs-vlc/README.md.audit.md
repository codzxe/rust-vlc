# CLAIM AUDIT: tools/bench-vs-vlc/README.md
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R3) |
| Claims extracted | 7 |
| REAL 6 / REAL-UNPROVEN 1 (harness runs not re-driven) | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The README's every structural claim checks out: all 7 named files exist at the stated
layout, the mock/real separation invariants are enforced IN CODE (asserted, not just
documented), the pinned VLC invocation is a genuine transcription of the repo's own
clean-room baseline help dump (lines 2335-2348 re-opened and matched), and the v1 report
+ 3 sibling evidence files exist. The [VERIFY] flag on first-real-oracle byte-count
confirmation is exactly the right epistemic humility for a transcription-based contract.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | layout table: run.py / mock_vlc.py / test_run.py / Cargo.toml / src/lib.rs / src/md5.rs / tests/lumen_pcm.rs | :13-21 | REAL | ls: all 7 present (+Cargo.lock, __pycache__); wc: run.py 1001, test_run.py 544, lib.rs 531, md5.rs 236 lines |
| 2 | "--oracle vlc ... --version asserted to be exactly 3.0.23 (drift aborts); without a binary the run refuses (exit 2), never falls back to the mock" | :25-28 | REAL | run.py:85 `PINNED_VLC_VERSION = "3.0.23"`, :118 VersionDrift exception, :19 "REFUSES (exit 2); it never falls" — refusal semantics in code |
| 3 | "mock ... every row/label/artifact carries [MOCK]; the JSON envelope records oracle.kind = "mock" ... run.py's report writer *asserts* these invariants" | :29-32 | REAL | run.py:843-844 `assert evidence["oracle"]["kind"] == "mock"` + `assert "[MOCK]" in markdown, "mock reports carry the [MOCK] label"` — asserted, not promised |
| 4 | "Transcribed from ... 14-options-full-help.txt L2335-2348" (vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format ...) | :39-47 | REAL | sed -n '2335,2348p' of the baseline -> `--audiofile-format {u8,s16,float32,spdif}`, `--audiofile-channels <integer [0 .. 6]>`, `--audiofile-wav, --no-audiofile-wav` present — the transcription source is real and matches |
| 5 | "[VERIFY] the first real-oracle run must confirm dump byte-counts equal Lumen's pcm_bytes" | :56-60 | REAL (flag present) | honest open verification marker; consistent with the PENDING-VLC v1 report actually shipped |
| 6 | "docs/corpus/vlc-diff-report-20261023.md ... + siblings .json, .lumen.json, .lumen.fixtures.tsv" | :142-149 | REAL | all 4 files present in docs/corpus/ (ls) — the PENDING-VLC mode described matches the shipped report |
| 7 | "the v0 demuxer filters unsupported video streams ... and still walks the audio — corpus-verified" | :66-73 | REAL-UNPROVEN (behavioral) | consistent with the AVI demux behavior recorded in the corpus reports audited in this batch (typed refusal rows + demux_recovered counters); not re-driven here |

## 3. Contradictions
None. (Cross-checked against the existing audit/docs/docs/corpus/vlc-diff-report-20261023.*
audit chain — the report's PENDING-VLC honesty matches this README's contract.)

## 4. Fabrication / inflation indicators
None — the document's core design is anti-fabrication (mock can never masquerade as
measurement; pending runs assert zero oracle digests; non-comparable rows carry reasons).

## 5. Reachability
N/A (tooling README). The harness is runnable per its own usage block; the Lumen side
drives real demux+codec crates (lib.rs 531 LOC).
Commands: ls/wc of the tree, rg run.py invariants (:85/:118/:843-844), sed of the
baseline help lines, ls docs/corpus.
