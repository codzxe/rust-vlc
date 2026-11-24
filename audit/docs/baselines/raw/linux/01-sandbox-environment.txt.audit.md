# CLAIM AUDIT: docs/baselines/raw/linux/01-sandbox-environment.txt
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 6 environment facts (raw dump) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 5 REAL (re-verified live) / 0 / 0 / 0 / 0 / 0 / 1 (network egress) |

## 1. Blunt verdict
Raw environment record; every re-checkable claim reproduces identically in the current
sandbox (same distro, same vlc apt policy, same uid, same NO_VLC_IN_PATH).

## 2. Claim ledger
| # | Claim | Verdict | Evidence (command I re-ran → output) |
|---|---|---|---|
| 1 | `NO_VLC_IN_PATH`; vlc not installed | REAL | `which vlc` → NO_VLC_IN_PATH (re-run now) |
| 2 | Debian GNU/Linux 13 trixie | REAL | `cat /etc/os-release` → PRETTY_NAME="Debian GNU/Linux 13 (trixie)" (re-run) |
| 3 | apt: vlc Candidate 3.0.24-0+deb13u1; 3.0.23-0+deb13u1 in trixie/main | REAL | `apt-cache policy vlc` re-run → byte-identical block (Candidate 3.0.24-0+deb13u1; 3.0.23-0+deb13u1 500 trixie/main) |
| 4 | uid 1001(z), no root | REAL | `id` → uid=1001(z) gid=1001(z) (re-run) |
| 5 | no Qt on host (dpkg grep -ci qt = 0) | REAL | re-run → 0 |
| 6 | "network egress to deb.debian.org + videolan.org verified working" | UNVERIFIED | not re-tested (no reason to hit the network); consistent with apt policy having fetched |

## 3. Contradictions
None. This record also explains the VLC 3.0.23 pin used by the whole corpus/oracle chain
(3.0.23 is the trixie/main version; 3.0.24 is security).

## 4. Fabrication / inflation indicators
None — raw dump, negative results included (no vlc, no Qt).
