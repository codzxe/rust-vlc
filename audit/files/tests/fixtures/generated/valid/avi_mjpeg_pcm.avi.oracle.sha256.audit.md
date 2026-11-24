# AUDIT: tests/fixtures/generated/valid/avi_mjpeg_pcm.avi.oracle.sha256
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 2326 B / efd79c2afd9c0f00506cf645c85107a6dfc62732507768a66c002d98b6332807 |
| File verdict | REAL (verified genuine: hashes reproduced against ffmpeg 7.1.5) |
| Reachable from shipped entry point? | NO — oracle pin sidecar; consumer scripts/oracle_check.sh (opt-in) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |

## 1. One-paragraph verdict
The pin is genuine and I reproduced it: decoding `avi_mjpeg_pcm.avi` with the sandbox's ffmpeg 7.1.5-0+deb13u1 (the exact version recorded in the sidecar's provenance) using the sidecar's documented commands yields `video 00c4f875…` and `audio 1d9f5019…` — both byte-equal to the committed rows. BUT the only consumer, scripts/oracle_check.sh, is permanently FAIL on the current tree: its perimeter check finds 15 media fixtures (wav/ogg/mkv/mov/webm) that landed without extending the pin set, prints WARNs, and `exit 1`s BEFORE comparing a single hash. The oracle discipline the file exists for cannot actually run green in this repo.

## 2. What actually works (verified)
- `ffmpeg -nostdin -v error -i tests/fixtures/generated/valid/avi_mjpeg_pcm.avi -map 0:v -pix_fmt yuv420p -f rawvideo - | sha256sum` -> `00c4f875fa15b765a2d1ffb5eaedf5c1b7647e3b3dbeeb995d9020f0ec78d9c4` == sidecar `video` row (MATCH).
- Same for `-map 0:a -f s16le` -> `1d9f5019c9b5181f0910528dce360f46e74f9c4df8d9bc9a20c2c68fd218b95b` == sidecar `audio` row (MATCH).
- Provenance is self-documenting: ffmpeg version, pin date (2026-10-22), decoded-byte geometry (6,912,000 video bytes; 352,800 audio bytes), the exact re-pin commands, and the `# streams: video audio` row-set declaration (line 31) the checker enforces.
- `bash scripts/oracle_check.sh` -> exit 1 (see F-001 — it never reached the hashes).

## 3. Findings
F-001 [S2]
Where: scripts/oracle_check.sh:176-189 (perimeter block) vs the current fixture tree
Code (verbatim, the guard that short-circuits):
```
if [[ ${#UNPINNED[@]} -gt 0 ]]; then
  echo "oracle_check: WARN — media fixtures outside the pinned perimeter {.avi,.mp4}:"
```
What is wrong: 15 valid/ fixtures with unpinned extensions (3 mkv, 1 mov, 4 ogg/opus, 6 wav, 1 webm — my run's WARN list) exist, so the script always exits 1 at the perimeter and never executes one hash comparison. The header's own rule — "a future .mkv/.mov/.webm/… fixture must extend the pin set … in the same commit" — was violated when those fixtures landed. Why it matters: the external-oracle mechanism (the M15 "echo chamber" cure) is dead-by-growth: green is impossible on a clean checkout, so the pins are unverifiable by the repo's own tooling (I verified them by hand instead). What correct looks like: pin sets extended for every landed media family, or the perimeter moved out of the fail-fast path. Confidence: HIGH — Verification: ran (exit 1; 17 output lines, zero hash lines).
F-002 [S3]
Where: scripts/oracle_check.sh:27-31 (usage header)
Code (verbatim):
```
# USAGE (opt-in — deliberately NOT wired into scripts/full_gate.sh or
# any CI gate; the local gate must never depend on ffmpeg being
```
What is wrong: even a green oracle check would gate nothing — it is opt-in, not wired into full_gate.sh or any workflow (rg: no .github reference). The pins protect only the developer who remembers to run it. Confidence: HIGH — Verification: ran (rg) + read.

## 4. Per-function table
N/A (data sidecar).

## 5. Test honesty
The sidecar itself is the most honest artifact in the fixture tree (external-engine truth, full provenance, explicit re-pin policy). The dishonesty is structural: the tool that consumes it cannot pass on this tree (F-001) and gates nothing (F-002).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "hashes of FFMPEG'S OWN DECODE … not of Lumen output" | sidecar:3-4 | REAL | reproduced both hashes with ffmpeg 7.1.5 (ran) |
| "every fixture x stream hashed equal to its pin" (exit 0 contract) | oracle_check.sh:9 | UNREACHABLE on this tree | perimeter exit 1 precedes hashing (ran) |

## 7. Dead / duplicated / unreachable code
The sidecar's hash rows are effectively unreachable through their only consumer (perimeter short-circuit) — reachable only by hand, as I did.

## 8. Dependencies & flags
Depends on system ffmpeg/sha256sum/awk at check time; sidecar pins ffmpeg 7.1.5-0+deb13u1 — sandbox matches (verified).

## 9. Open questions for the next agent
mp4_mjpeg_pcm.mp4.oracle.sha256 (not my assignment) — verify its two rows the same way; expect MATCH given the sibling's fidelity.
