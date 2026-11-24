# CLAIM AUDIT: docs/baselines/raw/linux/05-vlc-list.txt
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 4 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 2 / 1 / 0 / 0 / 0 / 0 / 1 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A terse `vlc --list` capture whose only quantitative claim — "Module count:
502 (one line per module)" — reproduces exactly (502 two-space-indented
module lines). Module names are genuine VLC 3.0.23 plugin-set entries (mad,
spatialaudio, speex_resampler, libbluray, wl_shm, vdpau family, vaapi
family). It is the cited source for docs/inventory/modules-summary.md's
"Total module registrations (Linux, measured) 502" and for
platform-linux.md's VA-API corroboration. Capture-level reproduction
(re-running VLC) is impossible here.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Module count: 502 (one line per module)" | :8 | REAL | `awk '/^  /{n++} END{print}'` -> 502; `rg -c '^  '` -> 502. Cross-cited as "measured 502" in docs/inventory/modules-summary.md:17. |
| 2 | "$ ./usr/bin/vlc --list ... # exit=0" | :9,511 | REAL-UNPROVEN | Body is a coherent `--list` output (name + one-line description per row); the run itself not re-executable (no VLC binary in sandbox). |
| 3 | "Plugin set: vlc-plugin-base + vlc-plugin-video-output (Debian 3.0.23-0+deb13u1)" | :4 | REAL | Consistent with vlcrc dump 16's Debian pin (3.0.23 Vetinari) and with 10 headless-excluded packages listed at :5-6 (all real Debian vlc-plugin package names: -qt, -skins2, -notify, -samba, -jack, -svg, -fluidsynth, -visualization, -video-splitter, -access-extra). |
| 4 | "Date: 2026-10-19 12:01 UTC" | :2 | UNVERIFIED | Date-only claim, no in-file evidence; not contradicted. |

## 3. Contradictions / Fabrication
None. Duplicated module names (spatialaudio, speex_resampler, chain, soxr,
dummy x2) reflect per-capability registration, consistent with
modules-summary.md:31-36's counting rule — not dump corruption.

## 4. Matrix rows
None directly; feeds the module census (docs/inventory/modules-linux.json)
that backs matrix platform columns.
