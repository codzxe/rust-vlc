# DATA AUDIT: docs/baselines/raw/linux/17-format-probes.txt
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / shape | 1,664 lines; 61 probe sections + 1 reconciliation section |
| File verdict | REAL (self-consolidated oracle record; harness committed; gaps self-flagged) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg censuses; sed spot-quotes; ls probe-work/; cat run_probes.sh |

## 1. What it is / provenance
Consolidated record (Task 0.B4-R) of 61 format-selection probe sessions against the pinned
VLC 3.0.23 oracle (3.0.23-2-0-g79128878dd — same hash as dumps 04/13/14/16). Each section:
command, fixture description, exit code, and "decisive lines (verbatim rg output)" — the
demux/packetizer/decoder module selections. Header (sed-verified) self-describes the
clean-room discipline: fixtures are own synthetic files (gen_py.py, committed at
docs/baselines/raw/linux/probe-work/gen_py.py — PRESENT), oracle observed via -vv logs only,
media NOT committed (gitignored tmp/).
Harness claim vs artifact: the quoted run() semantics match the committed
probe-work/run_probes.sh byte-for-byte (timeout 25, -I dummy --play-and-exit --aout dummy
--vout dummy -vv, status.txt append).

## 2. Data verification (executed)
- Section census: 61 probe `## ` sections + 1 `## Cross-checks / reconciliation` = 62 total ✓.
- Exit-code census: 56 × "exit code: 0", 1 × "exit code: 124" (rtp_ts — live-stream timeout,
  decoding still observed, disclosed at :1586), 4 × "exit code: not recorded (no status.txt
  entry)" (smoke_wav, http_server, rtp_ffmpeg, udp_ffmpeg) — EXACTLY the four the header
  names as status-less. 56+1 = 57 status entries == the header's "status.txt (57 entries)" ✓.
- Reconciliation section's own claim "probes with module-selection evidence: 58 of 61 logs"
  is consistent with the 3 named empty/daemon logs (rtp_ffmpeg, udp_ffmpeg 0-byte; http_server
  daemon log).
- Negative results are preserved verbatim (pl_cue_sample "nothing to play", midi "Codec
  `MIDI' is not supported", sub_* standalone zero-track, ac3/eac3/dts spdif caveat) — no
  success-only filtering.
- 4 self-flagged [VERIFY] gaps, e.g. "the exact ffmpeg command lines were NOT preserved by
  the prior 0.B4 session [VERIFY]" (:7-8) and the spdif passthrough caveat (reconciliation).

## 3. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "The logs directory holds 61 .log files + status.txt (57 status entries)." | :18 | REAL-UNPROVEN | logs/ dir lives in uncommitted tmp/; internal census consistent (56+1=57) |
| 2 | "Present here: all 61, in probe order." | :19 | REAL | 61 probe sections counted |
| 3 | "Exit codes: 0 for all status-tracked runs except rtp_ts (124 …)" | :31-32 | REAL | census: 56×0, 1×124 |
| 4 | "preserved at probe-work/gen_py.py" | :5 | REAL | file present; also run_probes.sh, dir/tone.wav, pls/{asx,b4s,cue,m3u} committed |
| 5 | "media fixtures are NOT committed (gitignored tmp/)" | :9 | REAL | no tmp/ in repo |
| 6 | fixture ffmpeg command lines "NOT preserved … [VERIFY]" | :7-8 | REAL | honest gap, self-flagged (4 markers) |

## 4. Findings
F-001 [S4] The per-probe media fixtures and the 61 .log files themselves are uncommitted
(tmp/ gitignored) — the consolidated text is the only surviving evidence layer, and its
"decisive lines" cannot be re-derived today. The doc admits this but a consumer citing
"probe-work/logs/*.log" as evidence cites an absent path. Confidence: HIGH — Verification: ls.

## 5. Consumers
docs/plans/asf-selfbuild.md, docs/gates/phase0-exit-report.md, docs/inventory/{formats.md,
corpus-plan.md, security-phase0.md}, docs/worklog/archive-2026-10.md, and the docs/formats/
records' oracle-behavior claims. Prior W19-A08 audit (old tree) concurs: "REAL, self-flagged
[VERIFY] gaps".
