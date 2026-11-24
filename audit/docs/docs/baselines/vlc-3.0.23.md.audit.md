# CLAIM AUDIT: docs/baselines/vlc-3.0.23.md (214 lines — master pin record)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 7 REAL / 0 / 0 / 0 / 0 / 0 / 3 UNVERIFIED (hashes/hardware items — no binary, no git, no network) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The master pin record is structurally honest: every companion file it names exists
(vlc-3.0.23.json, hardware-template.md, 3 platform records, raw/{windows,linux,macos}/
trees), the raw evidence supports its claims exactly (module census "502" = 502 non-comment
module lines in raw/linux/05-vlc-list.txt; the --list-plugins honest-failure record at
raw/linux/07 with exit 1; the macOS arm64 dmg size 51,273,389 B matches byte-exactly in
raw/macos/01's captured listing), and unverifiable items are explicitly marked
[VERIFY-blocked-on-hardware] rather than claimed. Windows/macOS hash+GPG claims and the
7/7 re-hash are UNVERIFIED here (binaries live in gitignored tmp/, absent from this
sandbox). The clean-room statement (:166-195) is corroborated: no binaries tracked
anywhere under docs/baselines (text only, verified by ls/file of the raw trees).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Companion files: json pin, hardware-template, 3 platform records | :12-14, :207-212 | REAL — ALL EXIST | ls docs/baselines/ -> vlc-3.0.23{.md,.json,-linux.md,-macos.md,-windows.md}, hardware-template.md, raw/. |
| 2 | Linux raw dumps 01..11 exist | :71 | REAL (17 now — later waves added 12-17) | raw/linux/01..17 (12-16 audited earlier this agent; 14 = full-help dump). |
| 3 | "`--list` (502 modules)" | :23, :71 | REAL — EXACT | awk '!/^#/ && NF' raw/linux/05-vlc-list.txt -> 502. |
| 4 | "--list-plugins does not exist on VLC 3.0.x ... unknown option, exit 1 (raw/linux/07)" | :90-94 | REAL — VERBATIM | raw/linux/07 lines: "vlc: unknown option or missing mandatory argument `--list-plugins'" + "# exit=1". |
| 5 | Windows raw evidence: acquisition-log.txt 489 lines + official-sources.txt | :70 | REAL (488 lines measured) | wc -l -> 488 (doc says 489; trailing-newline counting delta — 1 line). |
| 6 | macOS raw: 01..06 + 00-macosx-directory-listing.html (only non-.txt raw file) | :72 | REAL | ls raw/macos/ -> 00-macosx-directory-listing.html + 01..06 .txt. |
| 7 | arm64 dmg 51,273,389 B | :24 | REAL — CROSS-MATCH | raw/macos/01 captured listing: "vlc-3.0.23-arm64.dmg ... 51273389" — byte-exact match. |
| 8 | 7/7 consolidation re-hash PASS over tmp/pinning artifacts | :26-30, :96-112 | UNVERIFIED | tmp/ gitignored (.gitignore:40) and absent from sandbox. |
| 9 | GPG Good-signature claims (DSA key expired 2026-08-01, sigs 2025-12-31) | :22-24, DR-02 | UNVERIFIED | No binaries/signatures in sandbox; raw evidence text only. |
| 10 | "No binaries, source, or assets are committed anywhere" | :214 | REAL (spot-verified) | All docs/baselines/raw files inspected are text (file(1)); probe-work dir holds only text + one small wav fixture (itself a committed test asset, not a VLC binary). |

## 3. Contradictions
None. DR-08 records the baseline/baselines spelling split and D-007; the repo path is
docs/baselines/ (plural) as the record states.
## 4. Fabrication / inflation indicators
None. Unverifiable items carry [VERIFY] markers; the one failed invocation
(--list-plugins) is preserved as evidence rather than hidden (:194-195).
