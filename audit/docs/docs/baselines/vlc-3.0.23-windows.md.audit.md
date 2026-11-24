# CLAIM AUDIT: docs/baselines/vlc-3.0.23-windows.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 6 |
| REAL 3 / REAL-UNPROVEN 2 / FALSE 0 / UNVERIFIED 1 | |

## 1. Blunt verdict
A hash-and-GPG pin record for the Windows oracle, acquired and verified from a Linux sandbox with the run itself explicitly [VERIFY-blocked-on-hardware]. The doc's discipline is exemplary: the committed acquisition log carries the exact computed SHA-256 three times (matching the doc's value byte-for-byte), the `sha256sum -c` "OK" lines exist, and every check it could NOT run (Authenticode, version string capture, install smoke) is flagged with the exact blocking reason. It even flags the GPG signing key as expired and the keyserver anchor as [VERIFY: strengthen anchor]. No inflation found; the one caveat class is that sidecar/HTTP facts (mirror listings, sidecar text) rest on network fetches whose raw text is committed (official-sources.txt).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "SHA-256 (computed by us) `20ad191348...daaf` MATCH official sidecar ... `sha256sum -c` → `OK` (exit 0)" | §2 | REAL | docs/baselines/raw/windows/acquisition-log.txt:98,127,237 carry the exact hash; :135-137 "vlc-3.0.23-win64.exe: OK" x3; :403 the .7z OK |
| 2 | "Size 45,948,080 bytes ... matches official directory listing" | §2 | REAL-UNPROVEN | size recorded in log; listing text is committed in official-sources.txt (network-derived) |
| 3 | "gpg: Good signature from 'VideoLAN Release Signing Key (2018)' [expired]" | §3 | REAL-UNPROVEN | verbatim transcript committed in doc; expired-key nuance honestly recorded; cannot re-run (no key, no network) |
| 4 | "Authenticode ... NOT CHECKED — [VERIFY-blocked-on-hardware]" | §3 | REAL | honest absence claim; matches sandbox reality (no Windows host, no osslsigncode) |
| 5 | "Status | Pin VERIFIED from official sources (hashes + GPG cross-checked). Install + run-based smoke dumps [VERIFY-blocked-on-hardware]" | header | REAL | exactly the split the evidence shows: hash/GPG verified in-log; run not performed and never claimed |
| 6 | "Mirror URL family ... (identical file set + hashes verified)" | §1 | UNVERIFIED | mirror-fetch transcript in official-sources.txt; offline sandbox cannot confirm the mirror served identical bytes |

## 3. Contradictions
None. The header status splits verified vs blocked exactly as the evidence supports.

## 4. Fabrication / inflation indicators
None. The doc repeatedly marks its own unverified limbs ([VERIFY: anchor], [VERIFY on hardware], "value is ours" for the un-sidecared SHA-512) — anti-fabrication posture.
