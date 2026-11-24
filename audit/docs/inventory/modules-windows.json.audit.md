# CLAIM AUDIT: docs/inventory/modules-windows.json
| Field | Value |
|---|---|
| Auditor | W18-A06 |
| Date | 2026-02-11 |
| Claims extracted | 13 |
| REAL | 11 |
| REAL-UNPROVEN | 0 |
| SHALLOW | 0 |
| STUB | 0 |
| PAPER-ONLY | 0 |
| FALSE | 0 |
| UNVERIFIED | 2 (external wiki/release-notes sources; skins2-still-shipped) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This file is the anti-pattern the rest of the repo should follow for
un-executable inventories: it labels itself "SKELETON, expected-only" in the
first field, sets status "blocked-on-hardware", and carries an explicit
warning that NO COUNTS ARE MEASURED and nothing in it may be cited as
observed behavior. I verified that warning is accurate — the file contains
zero census counts, only expected module names with per-item [VERIFY]
markers. Every checkable cross-reference holds: the blocked_reason's
cited pin/hashes/runbook files exist, the "--list-plugins does not exist on
3.0.x" claim is backed by an in-repo honest-failure dump, the Linux-census
side-facts (smb missing because vlc-plugin-samba excluded; ffmpeghw score
10001; xcb_apps 'Screen capture'; alsa/pulse/xcb/vaapi/vdpau/udev present)
all reproduce against modules-linux.json and the raw dumps. The only
unverifiable content is the external wiki/release-notes sourcing of the 11
expected modules — the file itself flags each as [VERIFY] until a hardware
dump exists.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| W-01 | "VLC 3.0.23 module/plugin census — Windows (SKELETON, expected-only)" | modules-windows.json:2 | REAL | File content matches the label: no measured counts anywhere; 11 expected modules with verify=blocked-on-hardware each. |
| W-02 | "status": "blocked-on-hardware"; "No Windows machine in the sandbox." | :3-4 | REAL | `uname -s` -> Linux. Sandbox is Linux-only; hardware dump impossible. |
| W-03 | "NO COUNTS IN THIS FILE ARE MEASURED … Nothing here may be cited as observed behavior." | :6 | REAL (accurate warning) | Read the whole file: 119 lines, zero numeric census data; only names/capabilities/sources/notes. |
| W-04 | "The pinned win64 installer and hashes exist (docs/baselines/vlc-3.0.23.md, platforms.windows)" | :5 | REAL | vlc-3.0.23.md:45 Windows record [VERIFY-blocked-on-hardware] + :70 "raw/windows/acquisition-log.txt (489 lines…)" — `ls docs/baselines/raw/windows/` -> acquisition-log.txt, official-sources.txt. |
| W-05 | runbook "docs/baselines/hardware-template.md §3 (dumps 05/06 equivalents …)" | :5 | REAL | File exists; hardware_run_replaces section re-cites parser docs/inventory/raw/modules/parse-vlc-list.py — also exists (`ls` verified). |
| W-06 | "a --list-plugins probe that does NOT exist on 3.0.x" | :5 | REAL | 07-vlc-list-plugins-attempt.txt:3-8 "On VLC 3.0.23 this flag does NOT exist (it is a VLC 4.x flag)" + verbatim error "vlc: unknown option or missing mandatory argument `--list-plugins'". |
| W-07 | smb: "On the Linux census SMB is MISSING only because Debian ships it in vlc-plugin-samba, one of the 10 headless-excluded packages" | :90-92 | REAL | modules-linux.json excluded_debs_headless includes "vlc-plugin-samba" (10 packages); `rg -c smb 05-vlc-list.txt` -> 0 matches. |
| W-08 | "avcodec ffmpeghw registration (score 10001 in the Linux census)" | :115 | REAL | modules-linux.json video decoder: {"name":"avcodec","shortcuts":["ffmpeghw"],"score":10001}; raw 06-vlc-list-verbose.txt:190-192 `avcodec … s ffmpeghw … c video decoder (10001)`. |
| W-09 | "Linux census instead has xcb_apps services_discovery 'Screen capture'" | :105 | REAL | 06-vlc-list-verbose.txt:613 `xcb_apps               Screen capture`; modules-linux.json services_discovery xcb_apps (score 0) + services probe (100). |
| W-10 | expected_absent_vs_linux_census lists (alsa, pulse, v4l2, dvb, dtv, xcb_*, vaapi*, vdpau*, udev, avahi, fb, aa/caca, flaschen) | :117-122 | REAL | All named modules verified present in the Linux census module-name set (python3 set-membership: v4l2/dvb/dtv/xcb_x11/xcb_xv/vaapi_filters/vdpau_avcodec/udev/avahi/fb/flaschen/aa/caca all True; alsa score 150, pulse 160). |
| W-11 | "qt interface modules (Linux census lacks them only due to the headless scope limitation — vlc-plugin-qt excluded)" | :125-126 | REAL | modules-linux.json excluded_debs_headless includes "vlc-plugin-qt" and "vlc-plugin-skins2"; plugin_packages = base + video-output only. |
| W-12 | The 11 expected_platform_specific_modules with wiki/release-notes sources ("accessed 2026-10-19") | :9-115 | UNVERIFIED | External web sources not reachable/verifiable from this sandbox; no network fetch performed. The file self-flags each entry "verify": "blocked-on-hardware" — consistent. |
| W-13 | "skins2 interface [VERIFY still shipped in 3.0.23 win64]" | :127 | UNVERIFIED | Explicitly flagged by the doc itself; no in-repo evidence either way. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| (none found) | — | The file's self-declared non-observational status matches its content; no other doc contradicts it. The feature-matrix windows columns (rows 105-116 claiming windows implemented/partial) draw on NO windows evidence — this skeleton is the honest counterweight that the matrix chunk's windows statuses ignore. |

## 4. Fabrication / inflation indicators
None. The file fabricates nothing, measures nothing, and says so in the
first three fields. Its value as evidence is exactly what it claims: a
hypothesis list pending a hardware census. Note the systemic implication:
the repo's Windows parity rows (see
audit/docs/feature-matrix.toml.rows-105-117.audit.md) claim
implemented/partial statuses on windows with this file as the only
Windows-side inventory — a skeleton that forbids being cited as observed
behavior.

## 5. Matrix rows
Not a matrix chunk — n/a.

## 6. Commands run
- `uname -s` -> Linux
- ls: docs/baselines/{vlc-3.0.23.md, hardware-template.md},
  docs/baselines/raw/windows/ (acquisition-log.txt, official-sources.txt),
  docs/inventory/raw/modules/parse-vlc-list.py
- python3: modules-linux.json module-name set membership (13 names), ffmpeghw
  score lookup, excluded_debs_headless list
- sed -n: 06-vlc-list-verbose.txt:188-192, :613; 07-vlc-list-plugins-attempt.txt:1-8
- rg -c smb 05-vlc-list.txt -> 0
