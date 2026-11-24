# AUDIT: scripts/oracle_check.sh
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / functions | 289 / 2 (decode_hash, main) |
| File verdict | REAL (logic) — but exit-0 contract UNREACHABLE on current tree |
| Reachable from shipped entry point? | NO — dev script; manual invocation only; not in full_gate.sh, not in any workflow (verified) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | `bash scripts/oracle_check.sh` -> exit 1; `bash scripts/oracle_check.sh -h` -> exit 0; `grep -rn oracle scripts/full_gate.sh .github/workflows/*.yml` -> no invocation; `ls *.oracle.sha256 | wc -l` -> 2 |

## 1. One-paragraph verdict
The script is a well-constructed external-oracle verifier: `set -euo pipefail`, tool prerequisites, explicit status checks inside the `||`-context function, malformed/duplicate/vacuous-sidecar detection, row-set completeness, and a loud unsupported-extension perimeter. I executed it: it FAILS with exit 1 on the current tree before hashing a single stream, because 15 non-{avi,mp4} fixtures and 6 unpinned .avi/.mp4 fixtures landed without extending the pin set. Its "exit 0" contract is unreachable on the committed tree; the two committed oracle pins are only verifiable by hand (W16-A10/W16-RR3 reproduced them manually). The script is opt-in and wired into nothing, so nothing in CI ever notices.

## 2. What actually works (verified)
— `bash scripts/oracle_check.sh` -> perimeter WARN block + `oracle_check: RESULT = FAIL (media present outside the pinned perimeter)`, exit 1 (fail-loud works).
— `bash scripts/oracle_check.sh -h` -> prints the header as help, exit 0.
— Read-verified: L75 `set -euo pipefail`; L93-103 tool prereqs; L123-138 explicit ffmpeg-exit/regex/zero-byte guards; L263-280 row-set completeness.

## 3. Findings
F-001 [S2] — Exit-0 contract unreachable on the committed tree.
Where: scripts/oracle_check.sh:31-36 vs current fixture tree
Code:
```
# EXIT CODE
#   0 = every fixture x stream hashed equal to its pin
#   1 = at least one FAIL (hash mismatch, missing/malformed sidecar,
#       missing fixture, ffmpeg error, an empty fixture set, a sidecar
#       whose row set does not match its header-declared stream set,
#       or a media fixture outside the pinned extension perimeter)
```
What is wrong: Executed: exit 1 with 15 WARN lines (mkv/mov/ogg/wav/webm families) — the perimeter guard aborts before any hash comparison. Even without those, 6 of 8 .avi/.mp4 fixtures (avi_mjpeg_2pcm, mp4_chapters, mp4_edts, mp4_frag, mp4_frag_sidx, mp4_h264_aac) have no `.oracle.sha256` (only 2 sidecars exist) and would FAIL at L202-207. The M15 "echo chamber" cure the script exists for cannot run green in its own repo. Why it matters: the external-oracle discipline silently stopped functioning as fixtures grew; the committed pins are dead weight no gate ever checks. Correct: extend sidecars + UNSUPPORTED_MEDIA in one commit (the header itself demands this). Confidence: HIGH — Verification: ran (exit 1 reproduced; matches W16-A10/W01-A01 prior runs).
F-002 [S3] — Opt-in, wired into no gate or workflow.
Where: scripts/oracle_check.sh:23-25 (header) vs repo reality
Code:
```
# USAGE (opt-in — deliberately NOT wired into scripts/full_gate.sh or
#   any CI gate; the local gate must never depend on ffmpeg being
```
What is wrong: verified true as claimed: no workflow or full_gate.sh invokes it (grep across .github/workflows/ and scripts/full_gate.sh -> zero invocations; ci.yml/rust.yml install ffmpeg for in-test `testkit::oracle_available` suites only). The header is honest — but it means F-001's permanently-red state is invisible to every automated gate. Confidence: HIGH — Verification: ran grep.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| decode_hash | 110-140 | REAL | — | ran via main; explicit `if !` guards defeat the `||`-context errexit trap; two-pass (hash + wc -c), no temp files |
| main (inline) | 75-108,142-289 | REAL | — | perimeter + sidecar validation executed; exit 1 path taken |

## 5. Test honesty
No tests in this file. Prior agents tamper-attacked it in /tmp (worklog archive-2026-10.md:5569, 7583: deleted-row/CRLF/duplicate/wrong-hash/uppercase variants -> rc=1 each). Not re-verified by me (would need sandbox copies; marked UNVERIFIED-by-me, verified-by-prior-log).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "0 = every fixture x stream hashed equal to its pin" | oracle_check.sh:32 | UNREACHABLE (current tree) | ran -> exit 1 |
| "deliberately NOT wired into scripts/full_gate.sh or any CI gate" | oracle_check.sh:23 | REAL | grep: zero invocations |
| "oracle_check.sh 4/4 PASS exit 0" (historical) | docs/worklog/archive-2026-10.md:2121, 5867 | TRUE then, FALSE now | ran -> exit 1; fixture growth outpaced pin set |

## 7. Dead / duplicated / unreachable code
None found; every branch in the script is reachable via malformed-input classes.

## 8. Dependencies & flags
Requires ffmpeg/sha256sum/awk in PATH (all present in sandbox: /usr/bin). No repo-internal dependencies.

## 9. Open questions for the next agent
- Who owns adding oracle sidecars for the 6 unpinned .avi/.mp4 fixtures and the 15 perimeter files? No tracked follow-up found.
