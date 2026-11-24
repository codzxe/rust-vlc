# DATA AUDIT: docs/baselines/raw/linux/04-vlc-version.txt (credit: W19-A08, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | vlc --version transcript from extracted debs, full provenance header |
| Live re-verification | PARTIAL — Debian archive state checked live; binary itself gone |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## Verdict
Well-formed transcript (env, binary origin, exact invocation with LD_LIBRARY_PATH, exit=0)
whose run cannot be repeated — tmp/pinning/ is gone and no vlc exists in this sandbox
(`which vlc` -> nothing). Live checks I could make:
- TODAY both trixie/main and trixie-security carry `vlc 3.0.24-0+deb13u1` (curl of
  deb.debian.org Packages.xz, HTTP 200, both suites) — the pinned 3.0.23-0+deb13u1 has
  aged out of Debian entirely: S3 time-drift on the whole VLC-3.0.23-pin baseline lane
  (same class as the Flathub 3.0.23->3.0.24 drift I verified in dump 11's audit).
- sources.debian.org/api/src/vlc (HTTP 200) shows only `3.0.23-0+deb12u1` (bookworm) —
  the deb13u1 source is superseded/pruned; snapshot.debian.org remains 509-blocked
  (verified live), so the exact historical trixie version is unreachable from here.
S4: the version string's revision part `(3.0.23-2-0-g79128878dd)` does not match the
acquired Debian package version `3.0.23-0+deb13u1` recorded in dump 02:9-17 — the dump
never explains the "3.0.23-2" component (plausibly VLC's internal changelog-revision
convention, but UNVERIFIED — binary gone, archive blocked). "Vetinari" is correct for the
3.0.x series; gcc 14.2.0-19 is a plausible trixie toolchain (UNVERIFIED). Quotes
byte-verbatim via `sed -n '4,9p'` (output above).
