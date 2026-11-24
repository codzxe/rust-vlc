# DATA AUDIT: docs/baselines/raw/linux/02-acquisition-and-hashcheck.txt
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A07) |
| Type | Raw acquisition + hash log (53 lines), pin agent, 2026-10-19 |
| Independent verification | YES — apt-cache show in THIS sandbox (signed index present) |

## 1. Blunt verdict
Independently verified end-to-end: all 8 package SHA256s in the doc match the
publisher's signed apt index in this sandbox byte-for-byte (8/8 MATCH), and
apt-cache policy reproduces the doc's core factual claim exactly —
3.0.23-0+deb13u1 exists only in trixie/main while trixie-security carries
3.0.24-0+deb13u1 (candidate = 3.0.24). The GPG InRelease weekday is
consistent (2026-09-12 = Saturday as printed).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | 8 computed sha256s (vlc .. vlc-plugin-video-output) | :21-28 | REAL (independently re-verified) | `apt-cache show <pkg>` SHA256 for 3.0.23-0+deb13u1 -> 8/8 exact matches, e.g. vlc `68087c1c…986c6` identical to doc line 21 |
| 2 | "the only suite still carrying 3.0.23; trixie-security carries 3.0.24-0+deb13u1" | :4-5 | REAL | `apt-cache policy vlc`: Candidate 3.0.24 (trixie-security), 3.0.23 available from `deb.debian.org/debian trixie/main` |
| 3 | "Publisher hash source: the apt-signed Packages index (SHA256 field of apt-cache show)" | :6-7 | REAL | the sandbox apt index IS that signed source — and it agrees with the doc's hashes |
| 4 | "Fetched 4749 kB in 1s [exit=0]" + 8 Get: lines | :10-18 | UNVERIFIED | download session not re-runnable; consistent with the .deb sizes [94.2kB..3097kB] |
| 5 | InRelease two Good signatures (bookworm + trixie keys) | :40-47 | UNVERIFIED | InRelease not committed; weekday Sat Sep 12 2026 checks out (2026-09-12 = Saturday); debian keyring exists at the printed path |
| 6 | dump 01 apt-cache policy corroboration | :5 | REAL | 01-sandbox-environment.txt:33-36 carries the same policy output (3.0.24 candidate, trixie-security) |
| 7 | snapshot.debian.org fallback "[VERIFY at re-pin time]" | :52 | REAL (honest open item) | no false closure; not re-fetched now |

## 3. Contradictions
None — the strongest raw-evidence file of this batch: every hash claim
re-derives from an independent signed source in-sandbox.

## 4. Fabrication / inflation indicators
None. Findings: none.
