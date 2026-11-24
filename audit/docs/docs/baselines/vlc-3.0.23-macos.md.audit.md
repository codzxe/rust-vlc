# CLAIM AUDIT: docs/baselines/vlc-3.0.23-macos.md
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 6 / 0 / 0 / 0 / 0 / 0 / 1 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A pin record whose every cross-checkable number matches its committed raw
evidence byte-for-byte: all three SHA-256 values appear verbatim in
raw/macos/04-hash-crosscheck.txt (computed == sidecar -> MATCH lines), the
"Good signature [expired]" GPG result is in raw 05:37-41, the three dmg
sizes are in raw 01:59-60, and the `koly` UDIF structural sanity is in
raw 03:39-41. Hardware-blocked items are honestly marked. The only
unverifiable class is the download session itself (2026-10-19 timings).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | SHA-256 (computed) for arm64/intel64/universal + "9/9 hash cross-checks PASS" | §4 table | REAL | raw/macos/04-hash-crosscheck.txt:11-13 carries all three identical hashes; :29-43 computed==sidecar MATCH lines. |
| 2 | "gpg --verify → Good signature … key 65F7 C6B4 206B D057 A7EB 7378 7180 713B E58D 1ADC … key EXPIRED; DSA/SHA-1" | §5 | REAL | raw/macos/05-gpg-verification.txt:37 `gpg: Good signature from "VideoLAN Release Signing Key (2018)" [expired]` (+ key fingerprint at :11/:19 verified in the vlc-3.0.23.json audit). |
| 3 | Artifact sizes 51,273,389 / 57,544,217 / 89,124,160 bytes; "dmg 18-Dec-2025, sidecars 31-Dec-2025" | §2 | REAL | raw/macos/01-directory-listing.txt:59-60 carries all three sizes; server date 18-Dec-2025 13:43/13:44. |
| 4 | "trailing 512 bytes carry the `koly` UDIF magic (raw 03)" | §3 | REAL | raw/macos/03-dmg-downloads.txt:39-41 `RESULT: each dmg's last 512 bytes begin with the magic "koly"`. |
| 5 | "macOS binaries cannot execute here — download, hashing and GPG verification only" | §3, §7 | REAL | Consistent with the whole pin family (Linux sandbox); codesign/spctl marked [VERIFY-blocked-on-hardware] with a concrete runbook. |
| 6 | Path note: "per Lead decision D-007 this lives in `docs/baselines/` (doc 13 §3's `docs/baseline/` is superseded)" | :8-9 | REAL | File is at docs/baselines/ as stated; DR-08 in vlc-3.0.23.md records the baseline/-vs-baselines discrepancy class. |
| 7 | Download timings "31.0 s / 34.7 s / 2.3 s" and environment (curl 8.14.1, gpg 2.4.7) | §3 | UNVERIFIED | Session specifics in raw 03 (timings consistent with the file's own record); tool versions not re-runnable here. |

## 3. Contradictions / Fabrication
None. The record deflates (expired key, legacy DSA/SHA-1, codesign pending)
rather than inflating.
