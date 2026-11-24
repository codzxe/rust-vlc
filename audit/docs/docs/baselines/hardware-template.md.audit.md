# CLAIM AUDIT: docs/baselines/hardware-template.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R3) |
| Claims extracted | 9 |
| REAL 7 / PAPER-ONLY-by-design 1 / UNVERIFIED 1 (hardware runs not yet executed) | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A run-record TEMPLATE that claims nothing it has not done: it is explicitly "Template +
runbooks only. No performance numbers", its invocation-contract table keeps 11 of 13 cells
PENDING, and every cross-reference it embeds (hashes, version strings, failure records,
smoke evidence) byte-matches the pin tree it cites. The hardware runs themselves remain
unexecuted — stated, not hidden.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Template + runbooks only. No performance numbers ... no inventory beyond the pinned smoke channels" | :7 | PAPER-ONLY-by-design | no numbers anywhere in the file; 11/13 contract rows PENDING |
| 2 | "Master pin: docs/baselines/vlc-3.0.23.md + vlc-3.0.23.json (hashes to assert...)" | :9 | REAL | both files present in docs/baselines/ |
| 3 | Windows exe SHA256 20ad1913...daaf / SHA512 FE7AA769... | :92 | REAL | byte-matches vlc-3.0.23-windows.md:45/:94-95 and vlc-3.0.23.json:63 (SHA512 head at :152) |
| 4 | macOS dmg SHA256 56ee657c...6ec9 | :119 | REAL | byte-matches vlc-3.0.23-macos.md:57/:102 and vlc-3.0.23.json:261 |
| 5 | "VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)" assert | :95, :122, :150 | REAL | raw/linux/04-vlc-version.txt line 5 verbatim; vlc-3.0.23-linux.md:18/:39/:135 same |
| 6 | "raw/linux/07, the --list-plugins failure record ... exit 1 observed" | :26-27, :74-77 | REAL | raw/linux/07-vlc-list-plugins-attempt.txt — "HONEST FAILURE RECORD", `vlc: unknown option ... --list-plugins`, `# exit=1` |
| 7 | M1 Linux "**PROVEN (subset)**: -I dummy --play-and-exit exit 0 (raw/linux/08)" | :67 | REAL | raw/linux/08-vlc-dummy-smoke.txt `# exit=0` with full verbatim log incl. the headless dbus/pulse errors (not sanitized) |
| 8 | "Binaries never enter the repo; install only on reference/CI machines" | :24 | REAL | raw/ dirs contain text dumps only (ls: 17 linux txt files, windows/macos = acquisition logs only) |
| 9 | Windows §3.1 / macOS §4.1 steps (Authenticode, codesign/spctl, NSIS /S) | :81-133 | UNVERIFIED | hardware-dependent future runs; no such artifacts exist yet in raw/windows (only acquisition-log.txt, official-sources.txt) or raw/macos — consistent with the template's own PENDING rows |

## 3. Contradictions
None. The PENDING/PONDERED statuses match the actual artifact state (raw/windows and
raw/macos contain acquisition logs, not hardware-run dumps — exactly what the template says
should happen only later).

## 4. Fabrication / inflation indicators
None — the doc's honesty rules (:25-27 "record the verbatim failure — never omit it")
are enforced by its own cited precedent (dump 07). The one PROVEN cell (:67) carries a
narrow scope qualifier ("subset") and names its evidence file.

## 5. Reachability
N/A (runbook template; no code). The evidence files it cites are real repo artifacts.
Commands: ls raw/{linux,windows,macos}, rg hash values across pin records, sed of
04/07/08 raw dumps (version line, exit codes).
