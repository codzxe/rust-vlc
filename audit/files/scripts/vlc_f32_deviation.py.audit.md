# AUDIT: scripts/vlc_f32_deviation.py
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Lines / functions | 196 / 4 (_walk_chunks, riff_first_audio_payload, rms_peak, main) |
| File verdict | REAL (executed 5 case runs against a synthetic AVI I constructed: math exact, PASS/FAIL/exit-code contracts all behave; committed evidence JSON cross-checked) |
| Reachable from shipped entry point? | CI/evidence lane, not a binary: cited as the measurement tool by docs/corpus/vlc-diff-report-20261023.md:396 and docs/PERF-VS-VLC.md:83,247; output committed at docs/corpus/vlc-f32-tolerance-20261109.json (worklog.md:504) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | 5 synthetic runs (PASS / FAIL / FAIL-big / bad-digest exit 2 / truncated-dump) — outputs below; json.load of the committed evidence file; construction of a spec-shaped RIFF AVI with auds strh + 00wb f32 payload |

## 1. One-paragraph verdict
The script does exactly what its docstring promises and the math is exact. I
built a minimal spec-shaped AVI (RIFF/hdrl/strl/strh-auds/movi/00wb + f32
payload) plus a matching Lumen-JSON and ran five cases: deviation
(4e-4,9e-4) -> measured RMS 0.0004020792 vs hand-computed
sqrt((4e-4^2+9e-4^2)/6)=0.000402 EXACT, verdict FAIL (RMS over 1e-4) with
exit 0 (PASS/FAIL is data); small deviation -> PASS; big deviation (0.02)
-> RMS 0.008164958 = sqrt(0.02^2/6) exact; wrong harness digest ->
`[LUMEN-DIGEST-ASSERT-FAILED]` and exit 2 (the documented assert contract);
truncated dump (12 B) -> head-aligned comparison over 3 samples (the
documented shorter-side window). The committed evidence
(docs/corpus/vlc-f32-tolerance-20261109.json) reports 0/6 pass — and the
consuming doc (vlc-diff-report §6.6) states the same 0/6 FAIL honestly
("genuine parity gaps ... NOT hidden"), so no consumer lies about the data.
Two S4 parser limitations: the strl walk inspects only the FIRST chunk of each
strl (strf-before-strh files would mis-detect the audio stream), and the
":"-separated triple format cannot express Windows paths.

## 2. What actually works (verified)
- Synthetic-run outputs (verbatim lines):
  - `t.avi  RMS 0.0004020792   peak 0.0009000003   over 6 samples  FAIL` exit 0 (matches hand-computed 0.000402... exactly)
  - `t.avi  RMS 4.136694e-07   peak 1.013279e-06   over 6 samples  PASS` exit 0
  - `t.avi  RMS 0.008164958    peak 0.01999998     over 6 samples  FAIL` exit 0 (sqrt(0.02^2/6)=0.008165 exact)
  - bad digest: `... FAIL  [LUMEN-DIGEST-ASSERT-FAILED]` exit 2 — the documented assert path
  - truncated dump (12 B vs 24 B): `over 3 samples` — min-window head-alignment works
- out.json written with schema `corpus/vlc-f32-tolerance/1`, per-row payload_md5 + assert flag + thresholds + verdict, summary {pass,fail} — same shape as the committed evidence file.
- RIFF walk handles: word-aligned chunk padding (`size & 1`), payload end clamped to container end, one-plus-deep `rec ` nesting via explicit stack, first-auds track selection.
- The committed evidence docs/corpus/vlc-f32-tolerance-20261109.json: 6 rows, all `lumen_digest_assert: true`, all FAIL, summary {pass:0, fail:6} — and vlc-diff-report-20261023.md §Findings:2 states "all 6 rows FAIL the ratified tolerances — recorded as genuine parity gaps ... NOT hidden". Script, evidence, and doc agree.

## 3. Findings
F-001 [S4]
Where: scripts/vlc_f32_deviation.py:79-85
Code:
```
                for n3, s3, e3 in _walk_chunks(buf, s2 + 4, e2):
                    if n3 == "strh" and buf[s3 : s3 + 4] == b"auds":
                        stream = ordinal  # first audio stream (track rule)
                    break
```
What is wrong: the `break` is unconditional after the FIRST chunk of each
strl — only the first chunk is ever inspected for `strh/auds`. ffmpeg/AVI
convention puts strh first (my synthetic test follows it and works), but a
valid AVI whose strl starts with strf/strd would report "no first-audio-track
... payload found" — a false negative on well-formed input. Why it matters:
the tool is a parity-evidence generator; a silently-unsupported-but-legal
shape aborts the measurement with a misleading error. What correct looks like:
scan all chunks in the strl until strh (or none). Confidence: HIGH —
Verification: read (executed only the strh-first shape).
F-002 [S4]
Where: scripts/vlc_f32_deviation.py:147-148
Code:
```
    for triple in args.rows.split(","):
        rel, avi, dump = triple.split(":")
```
What is wrong: ":"-delimited triples cannot express Windows absolute paths
(drive-letter colon) — ValueError on a 3-way unpack. Linux-sandbox tool by
design (REL paths in the doc), but undocumented platform assumption.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| _walk_chunks | 56-64 | REAL | — | bounds-checked, word-aligned, clamp to end |
| riff_first_audio_payload | 67-101 | REAL-FLAWED | S4 | F-001 first-chunk-only strl scan; magic check; rec-nesting OK |
| rms_peak | 104-126 | REAL | — | exact math (verified vs hand computation); truncates to f32 sample boundary; NaN -> FAIL (fail-closed); count 0 -> None -> FAIL |
| main | 129-192 | REAL | S4 | digest assert -> exit 2 contract executed; verdict PASS iff both bounds (None fails closed); out-json schema matches committed evidence |

## 5. Test honesty
No in-file tests — the honesty lives in the committed evidence chain: the
script's output JSON (schema, thresholds, verdicts) is committed and the
consuming doc quotes the same FAILs. My 5-case execution substitutes for unit
tests: mutation reasoning — an RMS computed over the full (not windowed)
length would show different numbers on the truncated-dump case (it did window
correctly: 3 samples); a broken digest assert would flip exit code 2 (it did).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Exit code: 0 always ... unless an ASSERT fails ..., which is exit 2" | :38-40 | REAL | bad-digest run -> exit 2; FAIL rows -> exit 0 |
| "Lumen side ... ASSERTS the payload MD5 equals the Lumen harness digest ... before using it" | :22-24 | REAL | assert flag + exit 2 executed |
| "samples are compared head-aligned over the VLC-delivered window (min(payload, dump) bytes / 4)" | :29-31 | REAL | truncated-dump run -> 3 samples |
| "nothing is mocked" (both sides from actual PCM) | :16-17 | REAL | run against real bytes; committed JSON carries measured values |
| thresholds RMS <= 1e-4 / peak <= 1e-3 are Lead-ratified 2026-11-09 | :12-13,52-53 | REAL-UNPROVEN (decision provenance) | recorded in vlc-diff-report §6.6 decision record; decision itself is process history, not re-runnable |

## 7. Dead / duplicated / unreachable code
None. `--rms-max`/`--peak-max` overrides exist and are carried into each
row's output (used by the re-verification cards).

## 8. Dependencies & flags
stdlib only (argparse/hashlib/json/struct/sys/pathlib). No side effects beyond
--out-json. Reads arbitrary files (fixtures) — safe, read-only.

## 9. Open questions for the next agent
- The 6 f32 tolerance-FAILs are open parity gaps per the diff report's
  follow-up; any doc later claiming f32 parity for those fixtures would
  contradict the committed evidence — check PERF-VS-VLC.md when audited (it
  cites this script at :83 and :247; its verdict text was not in my file
  list, flag for its owner).
