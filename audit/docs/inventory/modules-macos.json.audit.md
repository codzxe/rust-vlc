# CLAIM AUDIT: docs/inventory/modules-macos.json

| Field | Value |
|---|---|
| Auditor | W18-A10 |
| Date | 2026-11-24 |
| Claims extracted | 16 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 7 / 0 / 0 / 0 / 0 / 0 / 9 |
| Internal contradictions found | 0 |
| Doc length | 85 lines; Task 0.B2 skeleton, all evidence dated 2026-10-19 |

## 1. Blunt verdict

This file is the honest end of the inventory spectrum: a pure skeleton
that refuses to fabricate. Every claim is either a verified
cross-reference (the pinned dmg artifacts, the hardware runbook, the
Linux census it differentiates against) or an explicitly-fenced
expectation ("NO COUNTS IN THIS FILE ARE MEASURED", 7/7 module entries
individually marked `verify: blocked-on-hardware`). No downstream doc
cites it as observed behavior — modules-summary.md propagates it as
"expected-only" and no feature-matrix row traces to it. The only thing
the devil-audit can hold against it: it is and remains PAPER — the
macOS module census itself does not exist, so every macOS platform cell
in the feature matrix rests on Linux-only evidence by construction, and
this file is the recorded reason.

## 2. Claim ledger

| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "status": "blocked-on-hardware" — "No Apple hardware in the sandbox." | :4-5 | REAL | Sandbox is Debian 13 trixie (`/etc/os-release`); baseline notes desktop-tauri also fails on gdk — consistent environment limit, honestly recorded. |
| 2 | "the pinned dmg artifacts and hashes exist (docs/baselines/vlc-3.0.23.md, platforms.macos)" | :5 | REAL | docs/baselines/vlc-3.0.23.md:24: macOS row "All 3 dmgs downloaded (`universal` 89,124,160 B pin-of-record; …) | **MATCH 9/9** — SHA-256+SHA-1+MD5 … GPG `.asc`: **Good signature ×3**"; raw/macos/ contains 00-06 artifacts. |
| 3 | "the runbook for capturing this census on hardware is docs/baselines/hardware-template.md §4 (--list / --list-verbose …; --list-plugins probe expected to fail on 3.0.x)" | :5 | REAL | hardware-template.md §4 (:108-127): step 5 dumps `--list`/`--list-verbose`/`--full-help` + "`--list-plugins` … (expected: rejected — record; DR-05)"; step 3 "record which arch slice actually executes" = arch-slice aware. |
| 4 | "NO COUNTS IN THIS FILE ARE MEASURED. … Nothing here may be cited as observed behavior." | :6 | REAL (verified literally) | Numeric-token scan of the file: only version strings (3.0.23, 2.1.0, 2.0.0), dates (2026-10-19), macOS 10.15, "167 pages" — zero module/category counts present. The negative claim is true. |
| 5 | 7 expected platform-specific modules (auhal, macosx vout/interface, avcapture, qtsound, screen, videotoolbox, coreaudio/midisynth) | :7-62 | UNVERIFIED (external) | Sourced to VideoLAN wiki pages + 3.0.0 release notes (URLs + access date inline); not re-checkable offline; each entry carries `verify: blocked-on-hardware` (7/7 counted). |
| 6 | "QTKit/QuickTime is deprecated for years (32-bit removal in macOS 10.15+); presence in 3.0.23 [VERIFY]" | :37 | UNVERIFIED (external) | Honest doubt marker present on the exact item most likely to be stale. |
| 7 | "Counterpart of Linux alsa/pulse audio outputs" (auhal note) | :13 | REAL (census cross-ref) | Linux census contains alsa + pulse modules (checked in modules-linux.json: `alsa` True, `pulse` True). |
| 8 | "Counterpart of Linux vaapi/vaapi_drm/vdpau_avcodec (hw decoder capability)" (videotoolbox note) | :52 | REAL (census cross-ref) | docs/baselines/raw/linux/05-vlc-list.txt:81-82 (`vaapi`, `vaapi_drm`), :144 (`vdpau_avcodec`). |
| 9 | expected_absent list: "alsa, pulse, pulsesrc, access_alsa", "v4l2, dvb …", "xcb_* vouts … fb, aa/caca …", "vaapi*, vdpau*, glconv_*", "udev, dbus, pulselist, avahi" | :64-69 | REAL (vs Linux census) | All named families present in the Linux census (checked: xcb/alsa/pulse/vdpau/avahi/udev all True in modules-linux.json) — the differential list is consistent with the observed Linux side. |
| 10 | "Linux census lacks any MIDI decoder (Debian ships one in the excluded vlc-plugin-fluidsynth — packaging, not platform)" | :60 | REAL | modules-linux.json `excluded_debs_headless` contains `"vlc-plugin-fluidsynth"`; zero MIDI modules in the census (`midi` search: False). Both halves verified. |
| 11 | "hardware_run_replaces_this_file_with … same parser as Linux: docs/inventory/raw/modules/parse-vlc-list.py" | :80-82 | REAL (paths exist) | parse-vlc-list.py exists; hardware-template §4 names the same dump convention. |
| 12 | "acceptance: per-category counts; every macOS-vs-Linux diff … known platform-specific module (annotated) or an open Phase 0 defect (doc 02 §3.7)" | :83 | REAL (ref) | Criterion consistent with modules-summary.md:257-260 ("Unexplained diffs: none measured (single platform)"). |
| 13 | cross_platform_sources — wiki module index "167 pages", 3.0.0 release notes, features page (doc 02 §3.1 S2 source) | :75-79 | UNVERIFIED (external) | URLs with access dates; not re-checkable offline (no network). |
| 14 | "macosx (vout/interface) … exact module names/split in 3.0.23 [VERIFY]" | :20 | UNVERIFIED (external) | Doubt marker on the one claim the wiki genuinely can't pin. |
| 15 | "osx notification module [VERIFY name and presence in 3.0.23]" | :73 | UNVERIFIED (external) | Doubt marker present. |
| 16 | Downstream citation hygiene: nothing cites this file as observed behavior | — | REAL | Repo-wide citation scan: only modules-summary.md (:6 "blocked-on-hardware skeletons", :68 "zero fabricated counts") and the worklog cite it — both propagate the caveat; zero feature-matrix.toml `inventory_ref` values point at it (grep: none). R10 anti-hallucination chain intact. |

## 3. Contradictions

None. The file's expectations are consistent with the observed Linux
census it differentiates against (claims #7-#10), its blocked status is
consistent with the sandbox reality, and its honesty markers are
complete (7/7 entries).

## 4. Fabrication / inflation indicators

None found — this is the anti-pattern the rest of the docs family
should copy: zero counts, per-entry doubt markers, a replacement
contract (`hardware_run_replaces_this_file_with`), and downstream docs
that propagate the caveat instead of laundering it. The residual
limitation is structural: the file is 100% PAPER (expectations), and
the feature-matrix macOS cells (e.g. rows 040-052's "macos =
implemented" in my chunk-1 ledger) inherit exactly this emptiness —
no macOS observation backs any macOS status cell anywhere in the
matrix; the Linux-only-evidence convention is the recorded cause.

## 5. Matrix rows

Not a matrix chunk. R10 coupling check performed instead: no
feature-matrix row carries `inventory_ref` = modules-macos.json; the
macOS-side statuses in my audited chunks (rows 040-052, 092-104) rest
on Linux-only code evidence, which this skeleton's blocked status
explains and bounds.

## Commands run

`python3` JSON parses of modules-macos.json + modules-linux.json
(entry/verify counts, family presence, fluidsynth/MIDI checks);
numeric-token regex scan (zero counts confirmed); `sed -n` on
hardware-template.md:108-127 and vlc-3.0.23.md; `grep` citation scan for
`modules-macos` across docs/ + audit/; `/etc/os-release`.

## Open questions for the next agent

- The 3.0.0 release-note claims (VideoToolbox hw decode, MIDI decoder,
  Direct3D11) are the same external sources modules-windows.json leans
  on; a hardware run is the only closer — the runbook is ready
  (§4.1 steps 0-6, verbatim).
- If Wave 18's macOS-cell verdicts ever cite this file as support for
  "implemented", that is a laundering violation of the file's own
  warning field (:6) — flag it on sight.
