# DATA AUDIT: docs/baselines/raw/linux/01-sandbox-environment.txt (credit: W19-A02, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | Raw environment capture log (51 lines): 7 verbatim command transcripts, Debian trixie sandbox |
| Live re-verified? | YES — all 7 probes re-run 2026-11-24, outputs match (hostname-only divergence) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
Authentic and still-current environment capture; every probe re-runs today to the same
answer:
- `which vlc` -> NO_VLC_IN_PATH (identical) — the no-VLC-in-sandbox fact underpinning the
  family-wide "VLC oracle PENDING-INFRA" honesty notes.
- `cat /etc/os-release` -> identical 8 lines (Debian GNU/Linux 13 trixie,
  DEBIAN_VERSION_FULL=13.4).
- `uname -a` -> same kernel/build (`5.10.134-013.15.kangaroo.al8.x86_64 #1 SMP Thu Jul
  30 12:54:59 UTC 2026 x86_64`); only the container hostname differs
  (c-6ad5e567-... then vs c-6b05c3f1-... today) — expected per-instance value, not a
  record defect.
- `id` -> uid=1001(z) gid=1001(z) identical.
- `dpkg -l | grep -ci qt` -> 0 identical; `ldconfig -p | grep -cE 'libGL|libEGL|libxcb-xv'`
  -> 4 identical.
- `apt-cache policy vlc` -> IDENTICAL table today (Installed none, Candidate
  3.0.24-0+deb13u1, both 3.0.24-security and 3.0.23-trixie rows) — corroborates both
  this record and W19-R7's 04-vlc-version.txt audit (the 3.0.23 pin has aged out of
  the candidate slot; the record never claimed otherwise).
- `apt list --installed | grep -i vlc` -> 0 matches today (record: no output).
No defects. Checked: all 7 commands re-executed and diffed; network-egress note
consistent with this run's own live curl/downloads (videolan.org + deb.debian.org both
reachable).
