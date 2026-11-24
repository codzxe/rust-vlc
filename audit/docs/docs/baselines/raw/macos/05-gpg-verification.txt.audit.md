# DATA AUDIT: docs/baselines/raw/macos/05-gpg-verification.txt
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A06) |
| Type | Raw GPG verification log (60 lines), agent 0.A3, 2026-10-19 |
| Re-runnable? | NO — dmg + .asc artifacts not committed; sandbox keyring empty now |
| Cross-referenced | 03-dmg-downloads.txt, 04-hash-crosscheck.txt (both present) |

## 1. Blunt verdict
Internally consistent raw evidence whose checkable details verify: the packet
timestamp 1767192566 = 2025-12-31 14:49:26 UTC (epoch-recomputed, matches the
verify line), 2025-12-31 is a Wednesday, gpg 2.4.7 is the sandbox's actual
version. The GPG "Good signature" outcomes themselves are not re-runnable
(artifacts not committed; keyring now empty) — they rest on this log plus the
committed 04 hash cross-check. The caveat section (expired key, DSA/SHA-1) is
unusually honest.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "created 1767192566" -> "Signature made Wed Dec 31 14:49:26 2025 UTC" | :9,47 | REAL | python epoch: 1767192566 -> 2025-12-31 14:49:26+00:00; Dec 31 2025 = Wednesday |
| 2 | "Host: Linux sandbox, gpg (GnuPG) 2.4.7" | :2 | REAL | `gpg --version` in this sandbox -> "gpg (GnuPG) 2.4.7" |
| 3 | key 65F7C6B4206BD057A7EB73787180713BE58D1ADC = VideoLAN Release Signing Key | :11,25-27 | UNVERIFIED | keyring empty (`gpg --list-keys` -> "No public key"; fresh keybox auto-created by my probe); no network fetch permitted |
| 4 | "key was already present in this sandbox's keyring" | :25 | UNVERIFIED | not reproducible in the current sandbox state |
| 5 | "matching sidecar hashes on the official server (raw evidence 04)" | :53-54 | REAL | 04-hash-crosscheck.txt present, computes sha256/512/1/md5 for all three dmgs + official sidecars |
| 6 | "All three detached GPG signatures verify as GOOD" | :50 | UNVERIFIED | .dmg/.asc not committed; verification not re-runnable — log-only evidence |
| 7 | CAVEATs: expired key; DSA/SHA-1 legacy | :51-57 | REAL | reasoning is sound and self-limiting; consistent with the [expired] tags in the verbatim block |
| 8 | "Apple codesign/spctl checks remain [VERIFY-blocked-on-hardware]" | :59 | REAL | honest limit; Linux sandbox cannot run macOS tools (statement :3) |

## 3. Contradictions
None. Cross-file consistency with 03 (downloads) and 04 (hashes) holds.

## 4. Fabrication / inflation indicators
None found. The outcome is scoped exactly to what the sandbox could do
("VERIFIED in-sandbox via GPG + hash sidecars"). Findings: none.
