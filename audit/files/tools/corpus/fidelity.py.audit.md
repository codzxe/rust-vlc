# AUDIT: tools/corpus/fidelity.py
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / functions | 243 / 9 (sh, video_scores, audio_scores, probe_media, decode_raw, main + 3 regexes) |
| File verdict | SHALLOW — runs and scores, but headline "per-frame series" is PAPER-ONLY (frames=0 on pinned ffmpeg) and the audio "dBFS" metric is wrong by +42.14 dB |
| Reachable from shipped entry point? | NO — dev/test infra; not invoked by any CI workflow or script; README + gen_corpus.py docstring reference it only as a "companion" |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | `python3 tools/corpus/fidelity.py --a a.raw --b b.raw --video --pix-fmt yuv420p --size 64x48 --fps 25` (exit 0); same with `--audio --fmt s16le --rate 48000 --channels 2` (exit 0); raw `ffmpeg ... [0:v][1:v]psnr` stderr inspected (ffmpeg 7.1.5) |

## 1. One-paragraph verdict
This is the v3/v4 corpus suite's decode-fidelity scorer. It genuinely shells out
to ffmpeg's psnr/ssim filters (the external-oracle principle holds) and its
summary averages are real — I ran it and got a correct 48.13 dB average on an
LSB-noise pair. But two of its advertised outputs are broken or lying: the
per-frame series NEVER parses on the pinned ffmpeg 7.1.5 (every frame line is
absent from stderr at default loglevel; `frames=0`, `min=None`, and the tool
prints `mean=inf` which in PSNR semantics falsely reads "identical"), and the
audio scorer normalizes s16le by 2^8 instead of 2^15, so its printed "dBFS" is
42.14 dB off (verified: true error -63.2 dBFS printed as -21.1 dBFS). The
docstring's "two media files, auto-decoded" mode does not exist in argparse
(dead `decode_raw`/`probe_media`), which tools/corpus/README.md §10 honestly
admits. Nothing in CI ever invokes it, so no lane depends on these bugs.

## 2. What actually works (verified)
— `python3 tools/corpus/fidelity.py --a a.raw --b b.raw --video --size 64x48`
  → `Video Fidelity: 48.130804 dB (imperceptible)`; summary average matches
  ffmpeg's own `[Parsed_psnr_0 …] average:48.130804` (raw ffmpeg run compared).
— `… --a a.pcm --b b.pcm --audio --fmt s16le` → `RMS error -21.1 dBFS, peak
  0.125000, exact-equal 50.0000% (96000 samples)` — arithmetic is exactly
  reproducible by hand (the *unit label* is wrong, see F-001).
— exit code 0 in both runs; PSNR<28→exit-1 branch is simple and reads correct.

## 3. Findings
F-001 [S2]
Where: tools/corpus/fidelity.py:106-107
Code:
        else:
            scale, center = float(2 ** (8 * (bytes_per_sample - 1))), 0.0
What is wrong: for s16le, bytes_per_sample=2 → scale=256, but s16 full scale
is 32768 (2^15). s32le gets 2^24 instead of 2^31. Every integer-sample error
is divided by 128× the correct scale, so `rms_db` = 20·log10(rms) is not dBFS —
it is inflated by 42.14 dB. RAN: synthetic stream with true RMS error −63.2 dBFS
(delta 32 LSB on one channel) prints `Audio Fidelity: RMS error -21.1 dBFS`
(−21.1 − (−63.2) = +42.1 dB). Why it matters: this is a measurement oracle for
decode correctness; any human or CI reading the number gets a 42 dB error
(pessimistic — real errors look 42 dB louder than they are), and any future
audio threshold wired on "dBFS" would be miscalibrated. What correct looks like:
`2 ** (8 * bytes_per_sample - 1)`, or fmt-specific 32768/2147483648.
Confidence: HIGH — Verification: ran (output above).

F-002 [S3]
Where: tools/corpus/fidelity.py:5-6 (docstring) vs 174-188 (argparse)
Code:
    "Did it exit without crashing?" is not a fidelity answer.  This tool puts a
    NUMBER on decode correctness by comparing two raw streams (or two media
    files, auto-decoded) with ffmpeg's own PSNR / SSIM filters — the external
    oracle computes the score, never Lumen.
What is wrong: the "two media files, auto-decoded" mode does not exist. argparse
has only --a/--b/--video/--audio/--pix-fmt/--size/--fps/--fmt/--rate/--channels/
--json (verified lines 175-188); `decode_raw` (L162-171) and `probe_media`
(L146-159) are defined and never called anywhere in this file. The doc claims a
feature the CLI lacks; ~30 lines are dead. tools/corpus/README.md §10 states it
flat out: "`fidelity.py` auto-decode mode (`--media-a/--media-b` in the
docstring) is not implemented in argparse — decode to raw first." Why it
matters: doc-vs-code contradiction in the test-infra oracle. What correct looks
like: wire decode_raw into main or delete it and fix the docstring.
Confidence: HIGH — Verification: read (argparse inventory) + README admission.

F-003 [S3]
Where: tools/corpus/fidelity.py:13-14 (docstring) vs 213-221 (output)
Code:
    Video:  ffmpeg psnr + ssim filters over two rawvideo inputs; per-frame
            series parsed and reported (min/median/mean + worst-frame index).
What is wrong: three untruths. (a) No median and no worst-frame index exist in
any output branch (only min/max/mean; `rg -n median tools/corpus/fidelity.py`
→ 0 hits outside this docstring). (b) Per-frame parsing is broken in practice:
on ffmpeg 7.1.5 (the version the corpus suite pins — README §9 #14 "ffmpeg
7.1.5"), the psnr filter prints ONLY the summary line at default loglevel; I
ran the exact ffmpeg command video_scores builds and `rg -c 'Parsed_psnr'` = 1.
RAN: `PSNR avg=48.130804 min=None mean=inf frames=0 inf(identical)=0`. (c) When
`psnr['mean']` is None, main prints `mean=inf` (L215-218), which in this tool's
own legend means "identical frames" (L88 `"inf_frames": … # identical frames`) —
a display lie: nothing was parsed, but the output reads as bit-exact. README
§10 registers the parsing gap ("observed live: frames=0, min=None while the
summary average is correct — the regex needs fixing") but the `mean=inf`
mislabel is NOT registered anywhere. What correct looks like: parse
`stats_file=-` output or drop the claim; print `mean=n/a` when unparsed.
Confidence: HIGH — Verification: ran (frames=0 output above) + read (README).

F-004 [S4]
Where: tools/corpus/fidelity.py:73-77
Code:
                m = SSIM_RE.search(line)
                if m:
                    per_frame.append(float(m.group(4)))  # All:
                    # the last "SSIM ..." line is the summary
                    summary = float(m.group(4))
What is wrong: the final summary line also matches SSIM_RE, so when per-frame
lines DO print, the summary is appended to per_frame as if it were a frame —
frames count +1, min/max/mean biased toward the global average. Latent (on
ffmpeg 7.1.5 only the summary prints, so per_frame == [summary] exactly — my
run showed all=min=max=mean=0.999906, i.e. one "frame"), but it is an
off-by-one by construction. PSNR does not have this issue (its summary line
does not match PSNR_RE; per-frame lines instead never print — F-003).
Confidence: HIGH on mechanism — Verification: read + ran.

F-005 [S4]
Where: tools/corpus/fidelity.py:97-98
Code:
    n = min(a.stat().st_size, b.stat().st_size)
    aligned = (n // frame) * frame
What is wrong: `n` and `aligned` are computed and never used (the chunk loop
re-derives truncation via `k = (k // frame) * frame`). Dead variables.
Confidence: HIGH — Verification: read (rg for `aligned` → only L98).

F-006 [S4]
Where: tools/corpus/fidelity.py:141
Code:
    db = 20 * math.log10(rms) if rms > 0 else float("inf")
What is wrong: for bit-identical audio, output is `RMS error inf dBFS` — +inf
reads as worst-possible error for a perfect stream (README documents the
intended reading: "identical files → inf dB" for video where inf=∞PSNR=good;
the same token in an *error* metric inverts the meaning).
Confidence: MEDIUM — Verification: read (not run for identical pair).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| sh() | 49-50 | REAL | — | ran via both entry points |
| video_scores() | 53-90 | SHALLOW | S3 (F-003/004) | summary real; per-frame parse dead on ffmpeg 7.1.5; mean=inf display lie |
| audio_scores() | 93-143 | REAL-BUT-WRONG-UNITS | S2 (F-001) | RMS/peak/exact math exact; normalization off by 2^7 |
| probe_media() | 146-159 | UNREACHABLE | S3→counted in F-002 | never called in file; no external importer (rg over tools/ scripts/ .github/: none) |
| decode_raw() | 162-171 | UNREACHABLE | S3 (F-002) | docstring-only feature |
| main() | 174-239 | REAL | — | ran, exit codes correct |

## 5. Test honesty
No tests exist for this tool (no pytest, no CI step, no committed evidence file
cites a fidelity.py invocation — `rg "fidelity" docs/perf/parity-v4-20261024.md`
→ only "PSNR … out of scope here" at L143). The tool itself is the test infra.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "comparing two raw streams (or two media files, auto-decoded)" | fidelity.py:5-6 | FALSE (auto-decode mode absent) | argparse L175-188; README §10 |
| "per-frame series parsed and reported (min/median/mean + worst-frame index)" | fidelity.py:13-14 | FALSE (no median, no worst-frame idx, frames=0 live) | ran: `frames=0 min=None mean=inf` |
| "< 28 FAIL (exit code 1 — CI-wirable)" | tools/corpus/README.md:279 | REAL-UNPROVEN (code path correct, never wired) | read L235-238; rg .github/ scripts/ → no invocation |
| "audio clean → 100% exact-equal, noisy → −19.3 dBFS RMS" | README.md:296 | REAL (matches F-001's wrong scale — the −19.3 number was produced by the buggy /256 normalization) | ran: -21.1 dBFS on a −63.2 dBFS stream |

## 7. Dead / duplicated / unreachable code
probe_media() + decode_raw() (~30 LOC dead); n/aligned vars dead.

## 8. Dependencies & flags
External: ffmpeg, ffprobe (present in sandbox, 7.1.5). Stdlib only otherwise.
No feature gates. Not reachable from shipped build (dev tool).

## 9. Open questions for the next agent
- Was any committed docs/perf/raw/parity-v4-* evidence produced by fidelity.py,
  or all by run_harness.py digest lanes? (W19-A04 owns those files.)
- corpus-v4 absence (per my brief): gen_corpus.py defaults `--out corpus`;
  .gitignore L96-107 ignores `/corpus/`, `/corpus-smoke/`, `/corpus-big/`,
  `corpus-v4/` with the stated reason "generated, not committed — the
  generator + manifest commands in-repo are the reproducibility anchor" → the
  absence is BY DESIGN; the vacuous-green risk lives in whatever lane reads
  those dirs without regenerating (compat-matrix.yml has an explicit
  "ffmpeg corpus regen" step — W16-A10/W19 territory).
