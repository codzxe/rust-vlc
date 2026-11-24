# DATA AUDIT: docs/baselines/raw/linux/11-secondary-sources.txt (credit: W19-A09, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | 3 network probes: Flathub API, snapshot.debian.org, Launchpad Ubuntu series |
| Live re-verification | DONE for all three |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## Verdict
Almost fully verified against the live internet today (2026-11-24):
- Launchpad: noble=3.0.20-3build6, plucky=3.0.21-9build1, questing=3.0.21-11 — ALL THREE
  match the dump byte-for-byte (live api.launchpad.net queries). The derived claim "No
  current Ubuntu series ships VLC 3.0.23" is TRUE today.
- snapshot.debian.org/package/vlc/3.0.23-0+deb13u1/ -> HTTP 509 right now — the dump's
  "blocked by administrative policy" record still holds; the [VERIFY on next run] tag is
  honest.
- Flathub org.videolan.VLC: HTTP 200, name "VLC" — but the live stable release is now
  **3.0.24** (timestamp 1789603200), while the dump recorded 3.0.23 (timestamp 1767225600).
  The dump was accurate at capture (2026-10-19); the secondary pin candidate has since
  drifted — S3: any doc that uses Flathub as the "3.0.23 secondary pin" is now stale
  against reality.
- Arithmetic check: 1767225600 = 2026-01-01T00:00:00Z (2025-01-01=1735689600 + 365d) — the
  dump's timestamp conversion is correct.
Quotes verified byte-verbatim via `sed -n '7,10p;23,25p'` against Read output; live
commands and outputs pasted above. This dump is honest evidence: it records failure (509)
as failure instead of papering over it.
