# DATA AUDIT: docs/baselines/raw/linux/09-upstream-vlc-dir-listing.txt (credit: W19-A08, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | curl dump of https://get.videolan.org/vlc/3.0.23/ index + FINDINGS note |
| Live re-fetch | DONE — byte-identical |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
AUTHENTIC and CURRENTLY VERIFIED. I re-fetched the live URL (`curl -s
https://get.videolan.org/vlc/3.0.23/`, HTTP 200, nginx) and diffed it against the dump's
HTML body (lines 6-21): `diff` -> IDENTICAL (exit 0). Every entry, date, and byte size
matches byte-for-byte (macosx/, os2/, win32/, win64/, winarm64/, vlc-3.0.23.tar.xz
26486988, .asc/.md5/.sha1/.sha256 sidecars). The dump's FINDINGS hold on the live tree:
(1) "no Linux dir" — the five platform dirs contain no linux/ entry, confirming
docs/02*.md:87 "get.videolan.org publishes **no Linux binaries** [verified by directory
listing]"; (2) tarball size 26,486,988 bytes matches docs/baselines/vlc-3.0.23-linux.md:35;
(3) no .sha512 sidecar exists (only .md5/.sha1/.sha256) — matches the FINDINGS note that
contradicts the "task-card assumption". Zero defects found; this is what a raw evidence
dump should look like (command + date + purpose header, verbatim body, interpretation
clearly split into FINDINGS). Quotes verified via `sed -n '6,21p'` + live diff above.
