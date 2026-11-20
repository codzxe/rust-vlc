# REALITY-SCORE — the headline number

Audited tree: HEAD `727ad6b2` (branch `audit/devil-20261116`). All numbers below come from
executed runs on 2026-11-16 unless marked UNVERIFIED.

## Headline (per the audit's strict REAL definition: executed end to end by this audit)

# REAL = 4 / 252 rows = 1.6%

Full breakdown (per-row table: `audit/waves/wave-18/rows-verdicts.tsv`):

| Verdict | Rows | Share |
|---|---|---|
| REAL (executed: open+advance on a real H.264+AAC MP4, linux) | 4 | 1.6% |
| REAL-UNPROVEN (executed workspace suite passed real conformance lanes over them) | 11 | 4.4% |
| SHALLOW (implemented cells backed only by path-existence evidence) | 52 | 20.6% |
| FALSE (implemented cells citing nonexistent evidence paths) | 1 | 0.4% |
| PARTIAL (as claimed) | 45 | 17.9% |
| STUB/PAPER-ONLY (absence claims: blocked/unsupported/not_started; default-true at sweep level, not re-derived file-by-file this run) | 139 | 55.2% |

REAL+REAL-UNPROVEN (the honest "works, proven by a run I did" tier) = **15/252 = 6.0%**.

## Why the strict number is 1.6% and not the repo's 26.2%

- The matrix's own parity tool prints implemented∪verified = 66/252 (26.2%) windows — but that
  26.2% is certified to a path-existence standard that this audit PROVED accepts a 0-byte file
  as 4-platform evidence (`audit/runs/baseline/experiment-feature-audit-empty-file.txt`).
- Zero rows anywhere carry `verified` status (executed: verified-only 0/252 on every platform).
  The repo's own strict definition of parity ("verified by automated evidence") is therefore
  **0/252 = 0%**, and v0.1.0–v0.1.2 shipped on the weaker tier.
- This audit upgrades only what it executed: the MP4 container row, the H.264 rows (both),
  and the AAC-LC row — 4 rows, 1.6%, all on linux (windows/macos cells of those same rows:
  REAL-UNPROVEN; same code, not run on those OSes here).

## The delta vs the repo's own numbers

| Metric | Repo says | Audit executed |
|---|---|---|
| Parity (windows) | 64/252 (25.4%) — PARITY-REPORT.md:109 | implemented∪verified 66/252 (26.2%); REAL 4/252 (1.6%); verified-only 0/252 |
| Symmetry | "identically" 64/64/64 | 66/61/66 — asymmetric |
| feature-audit | "result: PASS" (PARITY-REPORT §3) | FAILURE, 2 R4 violations (exit 1) |
| Verified cells | 0/252 | 0/252 — CONFIRMED |
| Tests | "passing" | 1001 passed / 2 failed / 20 ignored (executed) |
| Docs claim layer (8 key docs, 180 claims) | — | REAL 91 · REAL-UNPROVEN 35 · SHALLOW 8 · PAPER-ONLY 6 · FALSE 23 · UNVERIFIED 17 |

## What the 1.6% does and does not mean

It does NOT mean "the project is 1.6% done": the shipped binary demonstrably plays an MP4
H.264+AAC end to end (picture decodes, to a null audio sink), the UI suite passes 1,705
tests, and 52 more implemented rows rest on un-executed-by-me evidence of varying quality.
It DOES mean: by this audit's evidentiary bar — I ran it, I pasted the output — only 4 of
252 claims clear it, 52 claims are certified to a standard indistinguishable from fraud
(path existence), and the repo's headline 26.2% mixes those two tiers without labeling.

## Per-category REAL/REAL-UNPROVEN rows (all executed-backed)

container: mp4-mov-3gp (REAL) · avi, wav, flac-native, mp3-raw (REAL-UNPROVEN)
video-codec: h264, h264-software (REAL) · mjpeg (REAL-UNPROVEN)
audio-codec: aac-lc (REAL) · pcm, flac, mp3, vorbis, opus, adpcm (REAL-UNPROVEN)

## Platform columns

- windows: executions here: none (linux host only). Windows cells of REAL rows = REAL-UNPROVEN.
- linux: the executed platform (all runs in `audit/runs/baseline/`).
- macos: no host; UNVERIFIED.
- web: 0/252 implemented in the matrix; wasm build not run here (target not installed) — UNVERIFIED.
