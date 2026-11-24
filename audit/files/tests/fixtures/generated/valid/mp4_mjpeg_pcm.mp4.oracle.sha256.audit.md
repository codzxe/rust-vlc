# AUDIT: tests/fixtures/generated/valid/mp4_mjpeg_pcm.mp4.oracle.sha256
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A05) |
| Date | 2026-11-24 |
| Lines / functions | 39 lines (provenance header + 2 digest rows) |
| File verdict | REAL (pin reproduced by execution) |
| Reachable from shipped entry point? | As evidence file: consumed by scripts/oracle_check.sh (opt-in, unwired) and docs/worklog; not a shipped artifact |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | ffmpeg -version; both pinned decode commands piped to sha256sum (digests MATCH); bash scripts/oracle_check.sh (RESULT=FAIL, exit 1) |

## 1. One-paragraph verdict
The oracle pin is genuine and reproducible: I re-ran the sidecar's own pinned commands with
the pinned ffmpeg build and both digests match byte-for-byte. The header's provenance is
complete and truthful (ffmpeg version string, exact commands, stream geometry, cross-twin
note — and the AVI twin sidecar indeed pins the identical two digests, verified by rg).

## 2. What actually works (verified)
— verified by EXECUTION in this sandbox:
`ffmpeg -nostdin -v error -i tests/fixtures/generated/valid/mp4_mjpeg_pcm.mp4 -map 0:v
-pix_fmt yuv420p -f rawvideo - | sha256sum` → `00c4f875…ec78d9c4` (== pin line 38)
`ffmpeg … -map 0:a -f s16le - | sha256sum` → `1d9f5019…d218b95b` (== pin line 39)
`ffmpeg -version` line 1 == the pin's recorded version `7.1.5-0+deb13u1 … 2000-2026`.
Cross-twin: tests/fixtures/generated/valid/avi_mjpeg_pcm.avi.oracle.sha256:37-38 carries the
same two digests, as both headers claim.

## 3. Findings
F-001 [S2]
Where: scripts/oracle_check.sh (consumer of this file) — repo-own verification tool dead
Evidence (run): `bash scripts/oracle_check.sh` → 15 WARN lines (`wav_*`, `ogg_*`, `mkv_*`,
`mp4_qt.mov`, `webm_vp9_opus.webm` … "not oracle-pinned") then
`oracle_check: RESULT = FAIL (media present outside the pinned perimeter)`, **exit 1**.
What is wrong: the only tool that verifies this pin (and its twin) cannot pass on the current
tree — media families landed without extending the pin perimeter, so the guard exits 1
BEFORE hashing anything. The committed pins are unverifiable via the repo's own mechanism;
they are only verifiable by hand (which I did). Why it matters: an oracle whose checker is
permanently red is indistinguishable from a broken oracle in any future CI run — and the
script is opt-in (wired into NO gate: header "deliberately NOT wired into
scripts/full_gate.sh or any CI gate"). What correct looks like: extend pin sidecars to the
remaining 15 valid/ fixtures in one commit, or shrink the perimeter explicitly.
Confidence: HIGH — Verification: ran (script executed, exit 1 reproduced; hand-hash
reproduction above).

## 4. Per-function table
N/A (data/evidence file). Rows: 2/2 digests reproduced.

## 5. Test honesty
N/A — but note the pin's own honesty design: "hashes of FFMPEG'S OWN DECODE … not of Lumen
output" (M15 cure) — confirmed by the reproduction method itself.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DO NOT hand-edit … hashes of FFMPEG'S OWN DECODE" | :3-4 | REAL | digests reproduced from ffmpeg, not from Lumen |
| "the two containers carry identical elementary streams … oracle hashes must agree" | :27-29 | REAL | twin sidecar rows identical (rg) |
| "verified at pin time: default / yuv420p / yuvj420p all hash equal" | :14-16 | REAL (spot) | yuv420p path reproduced equal; other two not re-run here |

## 7. Dead / duplicated / unreachable code
None in the file. Its checker is failing for environmental/perimeter reasons (F-001).

## 8. Dependencies & flags
None (text sidecar). Check path: scripts/oracle_check.sh (opt-in; exit-1 on current tree).

## 9. Open questions for the next agent
The 3gp family (my mp4_3gp_h263.3gp audit) is also outside the perimeter — when the pin set
is extended, decide whether .3gp belongs in the oracle perimeter or the UNSUPPORTED_MEDIA
list.
