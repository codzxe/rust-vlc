# DATA AUDIT: docs/baselines/raw/linux/10-upstream-tarball-sidecar-sha256.txt (credit: W19-A06, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | publisher .sha256 sidecar for vlc-3.0.23.tar.xz + provenance notes |
| Live re-fetch | DONE — all 3 claims verified against the live CDN |
| Cited by | docs/baselines/vlc-3.0.23-linux.md:170 ("The .sha256 sidecar was recorded (dump 10)") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
AUTHENTIC and CURRENTLY VERIFIED, claim by claim:
1. Sidecar hash: live `curl -sL https://download.videolan.org/vlc/3.0.23/vlc-3.0.23.tar.xz.sha256`
   returned EXACTLY `e891cae6aa3ccda69bf94173d5105cbc55c7a7d9b1d21b9b21666e69eff3e7e0
   *vlc-3.0.23.tar.xz` — byte-identical to the dump's "verbatim sidecar content" (line 19).
2. "NO .sha512 sidecar exists": live checks — get.videolan.org/...tar.xz.sha512 -> HTTP 500
   and download.videolan.org/...tar.xz.sha512 -> HTTP 404, matching the dump's recorded
   500/404 pair (lines 14-15) exactly.
3. Sidecar inventory (.asc 195 B, .md5, .sha1, .sha256) matches dump 09's live-verified
   directory listing (cross-audit: see my audit of dump 09).
4. Clean-room discipline is honestly stated: tarball NOT downloaded; sidecar recorded as
   release-provenance cross-evidence (lines 8-11) — consistent with doc 02 §2.1's stated rule.
This is the second fully-live-verified VLC evidence file in the batch (with dump 09 and
the macOS listing). Quotes byte-verbatim via `sed -n '19p'`. Zero defects.
