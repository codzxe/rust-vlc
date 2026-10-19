# Master pin record — VLC 3.0.23 (cross-platform test oracle)

| | |
|---|---|
| **Task / agent** | 0.A4 (consolidation), wave 0.A |
| **Status** | Consolidation of the three platform pins (0.A1 Windows, 0.A2 Linux, 0.A3 macOS). Linux pin is **sandbox-verified end-to-end**; Windows and macOS pins are **verified to hash+GPG** with install/run items **[VERIFY-blocked-on-hardware]** (runbooks: `hardware-template.md`). |
| **Implements** | doc 02 §2.3 (pin record), doc 13 §3 card 0.A4; completes the evidence side of gate **G1** (doc 02 §1.2) |
| **Path note** | Doc 02 §1.2 and doc 13 §3 write `docs/baseline/` (singular); Lead decision **D-007** supersedes with `docs/baselines/` (this file's location). The Lead aligns doc 02/13 spelling at gate review (doc 12 §10 OQ-3). |
| **Sources** | Every fact below comes from the three platform records and their committed raw evidence; this file adds no new external facts. Consolidation-time re-verification: 7/7 SHA-256 spot re-checks of retained artifacts PASS (§5). |
| **Recorded** | 2026-10-19, by 0.A4 |

Companion machine-readable pin: `docs/baselines/vlc-3.0.23.json` (consumed by the later
differential harness per doc 10 §6.6 and doc 12 §3.3). Hardware-run template:
`docs/baselines/hardware-template.md`.

---

## 1. Executive pin summary

| Platform | Version pinned | Source (official) | Acquisition | Hash status | Signature status | Run status |
|---|---|---|---|---|---|---|
| **Windows x64** | 3.0.23 Vetinari | `download.videolan.org` / `get.videolan.org` `…/vlc/3.0.23/win64/` | `vlc-3.0.23-win64.exe` (installer, 45,948,080 B) + `vlc-3.0.23-win64.7z` (no-install, 39,583,707 B), `curl -L` via mirror `mirror-hk.koddos.net` | **MATCH** — SHA-256/SHA-1/MD5 computed by us match official sidecars on both official hosts; also matches doc 02 §2.2 pre-verified value | GPG `.asc`: **Good signature** (DSA key, expired 2026-08-01, sig made 2025-12-31 — predates expiry); Authenticode **[VERIFY-blocked-on-hardware]** | **blocked-on-hardware** — not installed, not executed (Linux sandbox cannot run PE binaries) |
| **Linux x86_64** | 3.0.23 Vetinari, Debian `3.0.23-0+deb13u1` | Debian 13 (trixie) `main` archive, `deb.debian.org` pool | `apt-get download` (root-free) of 8 packages + `dpkg-deb -x` extraction (no install) | **MATCH ×8** — SHA-256 of each `.deb` matches Debian's GPG-signed `Packages` index | apt chain: `dists/trixie/InRelease` GPG-verified — **Good signature ×2** (Debian Archive Automatic Signing Key 12/bookworm + 13/trixie) | **sandbox-verified** — ran headless: `--version`, `--list` (502 modules), `--list-verbose`, `-I dummy --play-and-exit` smoke (exit 0) |
| **macOS (universal pin-of-record)** | 3.0.23 Vetinari | `get.videolan.org` / `download.videolan.org` `…/vlc/3.0.23/macosx/` | All 3 dmgs downloaded (`universal` 89,124,160 B pin-of-record; `arm64` 51,273,389 B; `intel64` 57,544,217 B) | **MATCH 9/9** — SHA-256+SHA-1+MD5 of each dmg match official sidecars; universal SHA-256 also matches doc 02 §2.2 pre-verified value | GPG `.asc`: **Good signature ×3** (same key as Windows; expired-key + DSA/SHA-1 caveats); codesign/spctl/notarization **[VERIFY-blocked-on-hardware]** | **blocked-on-hardware** — not mounted, not installed, not executed (Linux sandbox cannot run macOS binaries) |

Consolidation-time re-verification (2026-10-19, 0.A4): the retained artifacts in
gitignored `tmp/pinning/` were re-hashed with `sha256sum` —
`win64.exe`, `win64.7z`, `universal.dmg`, `arm64.dmg`, `intel64.dmg`,
`libvlccore9_…deb`, `vlc-plugin-base_…deb` — **7/7 match** the values in the
platform records. No re-download was needed.

## 2. Canonical oracle identity

This is the identity the differential harness (doc 10 §6.6, doc 12 §3.3) asserts
before every oracle invocation (doc 02 §2.4 "Guard").

| Field | Value | Evidence |
|---|---|---|
| Product | VLC media player | release page title "VLC 3.0.23 Vetinari - VideoLAN" |
| Version | **3.0.23** | all three records |
| Codename | **Vetinari** (24th update of the 3.0.x branch) | release page + Linux `--version` |
| Git describe | **`3.0.23-2-0-g79128878dd`** — observed on the Linux (Debian) build only; Windows/macOS revision strings are expected to embed the upstream revision but are **[VERIFY-blocked-on-hardware]** (capture verbatim; do not assume equality) | raw/linux/04 |
| Release date | **2026-01-08** — videolan.org news index anchor `news-2026-01-08`, matching MASTER-PROMPT §1; the release page itself carries no explicit date; download-server upload timestamps are 31-Dec-2025 (staging) | raw/macos/06, raw/windows/* |
| Observed version line (Linux, verbatim) | `VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)` | raw/linux/04 |
| Expected version line (Windows) | shape `VLC media player 3.0.23 Vetinari (revision …)` — **[VERIFY-blocked-on-hardware]**, capture verbatim on install | windows record §7 |
| Expected version line (macOS) | not asserted — **[VERIFY-blocked-on-hardware]**, capture verbatim on install | macos record §8 |
| Debian build identification | `Compiled by Debian buildd for trixie-security on Debian amd64 buildd (Jan 7 2026 20:01:08)`, `gcc 14.2.0 (Debian 14.2.0-19)` (verbatim; artifact served from the `main` pool — see DR-09) | raw/linux/04, raw/linux/02 |
| Release page | `https://www.videolan.org/vlc/releases/3.0.23.html` (HTTP 200) | all three records |
| Upstream source tarball | `vlc-3.0.23.tar.xz`, 26,486,988 B, sidecar SHA-256 `e891cae6aa3ccda69bf94173d5105cbc55c7a7d9b1d21b9b21666e69eff3e7e0` — **deliberately NOT downloaded** (clean-room, doc 02 §2.1); sidecar text recorded as release-provenance cross-evidence only | raw/linux/09, raw/linux/10 |

### Official artifact URLs per platform

- **Windows x64** (primary + secondary):
  - `https://download.videolan.org/pub/videolan/vlc/3.0.23/win64/vlc-3.0.23-win64.exe` (mirror family `https://get.videolan.org/vlc/3.0.23/win64/…`; observed redirect: `mirror-hk.koddos.net`)
  - `https://download.videolan.org/pub/videolan/vlc/3.0.23/win64/vlc-3.0.23-win64.7z`
  - Also published (sidecar hashes only, not downloaded): `win64.msi` (SHA-256 `bc4b902a480b98a4a5479327a7f210f06369a59bf727649e320faba5b4ef1f5e`), `win64.zip` (SHA-256 `992d19dbd0b8a7cde9167d2f7780b1ef6f92acc8a71acfa736101a21f35181e1`), plus `win32/`, `winarm64/` trees.
- **Linux** (8-package Debian pin, all `=3.0.23-0+deb13u1`):
  - `http(s)://deb.debian.org/debian/pool/main/v/vlc/` — `vlc`, `vlc-bin`, `vlc-data`, `libvlc-bin`, `libvlc5`, `libvlccore9`, `vlc-plugin-base`, `vlc-plugin-video-output` (exact per-file URLs, sizes, SHA-256s: Linux record §6.1 / the JSON).
  - Permanent-archive fallback: `https://snapshot.debian.org/package/vlc/3.0.23-0+deb13u1/` — unreachable from the pin sandbox (HTTP 509) **[VERIFY on next acquisition]**.
  - Secondary (not executed): Flathub `org.videolan.VLC`, stable = 3.0.23 (API-verified, release 2026-01-01).
- **macOS** (pin-of-record + arch coverage):
  - `https://get.videolan.org/vlc/3.0.23/macosx/vlc-3.0.23-universal.dmg` (pin-of-record)
  - `…/macosx/vlc-3.0.23-arm64.dmg`, `…/macosx/vlc-3.0.23-intel64.dmg` (arch variants, hashes recorded)
  - Note the path segment is **`macosx/`** — the doc 13 card's `macos/` 404s (DR-03).

## 3. Per-platform pointers

| Platform | Pin record (facts, hashes, runbooks, [VERIFY] ledgers) | Raw evidence (committed, text) |
|---|---|---|
| Windows | `docs/baselines/vlc-3.0.23-windows.md` — §1 provenance, §2 hashes, §3 GPG, §6 install runbook, §7 smoke-dump runbook, §9 discrepancy ledger D1–D8 | `docs/baselines/raw/windows/acquisition-log.txt` (489 lines, every command + output incl. failures); `raw/windows/official-sources.txt` (URLs, verbatim sidecar/signature text, negative results) |
| Linux | `docs/baselines/vlc-3.0.23-linux.md` — §2 version ID, §3 distro decision, §5 acquisition, §6 hashes, §7 run status, §10 reference-machine runbook, §12 [VERIFY] ledger V1–V5 | `docs/baselines/raw/linux/01…11` — 01 env, 02 acquisition+hashcheck, 03 support libs, 04 `--version`, 05 `--list` (502 modules), 06 `--list-verbose`, 07 `--list-plugins` honest failure, 08 `-I dummy` smoke, 09 upstream dir listing, 10 tarball sidecar, 11 secondary sources (Launchpad/Flathub/snapshot) |
| macOS | `docs/baselines/vlc-3.0.23-macos.md` — §1 version, §2 arch coverage, §4 hashes, §5 GPG + codesign runbook, §7 install runbook, §8 smoke-dump runbook, §10 discrepancies | `docs/baselines/raw/macos/01…06` — 01 directory listings, 02 sidecars, 03 dmg downloads, 04 hash cross-check, 05 GPG, 06 web citations; plus `00-macosx-directory-listing.html` (verbatim listing page — the only non-.txt raw file, kept as fetched) |

Machine-readable pin covering all three platforms: `docs/baselines/vlc-3.0.23.json`.

## 4. Oracle invocation contract — validation status (doc 02 §2.4)

Consolidated result of the three pin waves; expanded commands and expected
artifacts live in `hardware-template.md` §2.

| Mode | Contract (doc 02 §2.4) | Windows | Linux | macOS |
|---|---|---|---|---|
| **M1** headless metadata & stream info | `vlc -I dummy --verbose 2 --play-and-exit --no-video --no-audio <input>`, log parsed | **PENDING** (hardware) | **PROVEN (subset)**: `-I dummy --play-and-exit` ran, exit 0, expected headless errors only (raw/linux/08). Full flag set incl. `--verbose 2` + real input + log grammar: **PENDING** (needs corpus, 0.A12/Phase 2). rc-interface alternative: untested [VERIFY] | **PENDING** (hardware) |
| **M2** deterministic decoded output (`--sout` transcode chain) | [VERIFY] exact chain + determinism | PENDING | PENDING | PENDING |
| **M3** error behavior on malformed inputs (`--play-and-exit`, exit code + error class) | feeds per-format corrupt-input records | PENDING | PENDING (no corpus yet — 0.A12) | PENDING |
| **M4** timing (startup, TTF, seek; median+p95) | Phase 8 executes; protocol fixed now | PENDING (by design) | PENDING (by design) | PENDING (by design) |
| **Guard** version-string assert before every invocation | drift aborts the run | pin data ready (expected shape [VERIFY]); enforcement starts with the harness | **PROVEN data**: verbatim `--version` line recorded; guard is harness code (Phase 2+) | pin data pending (hardware) |
| **Sandboxing** disposable VM/container, no egress, no credentials | owned by 0.A11 | PENDING | PENDING | PENDING |

Flag-existence finding that binds the harness: **`--list-plugins` does not exist
on VLC 3.0.x** (it is a 4.x flag) — observed: `unknown option`, exit 1
(raw/linux/07). The 3.0.x module-dump channels are `--list` and `--list-verbose`
(both captured, exit 0, raw/linux/05–06). Later cards and the harness must use
the 3.0.x flags.

## 5. Consolidation-time re-verification

On 2026-10-19, 0.A4 re-ran `sha256sum` over the artifacts still present in
gitignored `tmp/pinning/` (no network, no re-download):

| Artifact | Expected (platform record) | Re-computed | Result |
|---|---|---|---|
| `vlc-3.0.23-win64.exe` | `20ad1913…daaf` | identical | PASS |
| `vlc-3.0.23-win64.7z` | `eb4fd8a2…0404a` | identical | PASS |
| `vlc-3.0.23-universal.dmg` | `56ee657c…f6ec9` | identical | PASS |
| `vlc-3.0.23-arm64.dmg` | `fc6fac08…5e0d` | identical | PASS |
| `vlc-3.0.23-intel64.dmg` | `ec01530c…98db` | identical | PASS |
| `libvlccore9_3.0.23-0+deb13u1_amd64.deb` | `df776791…7770` | identical | PASS |
| `vlc-plugin-base_3.0.23-0+deb13u1_amd64.deb` | `2a546b92…76f9` | identical | PASS |

7/7 PASS. `tmp/` is gitignored (`.gitignore` `tmp/` line; `*.exe` additionally
ignored); these files are scratch only and never committed.

## 6. G1 evidence checklist (doc 02 §1.2 row G1)

Doc 02 §1.2 G1 passing condition, decomposed per platform. **The Lead ticks the
G1 row in doc 02 at gate review (0.D4-V); this section is the evidence input.**

| G1 requirement | Windows | Linux | macOS |
|---|---|---|---|
| Artifact URL | ✅ complete | ✅ complete (8 package URLs) | ✅ complete (universal + 2 arch) |
| Computed SHA-256 cross-checked vs official sidecar | ✅ complete (exe + 7z; both official hosts) | ✅ complete (8/8 vs signed Packages index) | ✅ complete (9/9 across 3 dmgs) |
| Signature-check result | ◐ partial — GPG sidecar Good; **Authenticode [VERIFY-blocked-on-hardware]** | ✅ complete — InRelease GPG Good ×2 | ◐ partial — GPG sidecar Good ×3; **codesign/spctl [VERIFY-blocked-on-hardware]** |
| `vlc --version` line | ◐ **[VERIFY-blocked-on-hardware]** — expected shape recorded | ✅ complete — verbatim in raw/linux/04 | ◐ **[VERIFY-blocked-on-hardware]** |
| OS version + arch (run environment) | ◐ blocked (acquisition sandbox recorded: Linux x86_64; install target TBD) | ✅ complete — Debian 13 trixie, kernel 5.10.134, x86_64, headless | ◐ blocked (same pattern) |
| Date | ✅ 2026-10-19 | ✅ 2026-10-19 | ✅ 2026-10-19 |

**Overall G1 status: evidence complete for everything executable in the pin
sandbox; Windows/macOS rows are `partial-pending-hardware` exactly on the
native-signature, `--version`-line and run-environment items.** Closing them
requires the hardware runs defined in `hardware-template.md` (owners: Lead —
see §9). The sandbox-executable G1 content (URLs, hashes, cross-checks, GPG
results, Linux version line, dates) is complete and committed.

## 7. Consolidated discrepancy register

Facts differ between planning docs and observed reality, or between records.
Registered here rather than silently resolved. IDs DR-01… so later docs can cite
them stably. ("Card" = doc 13 §3 wave-0.A cards; "doc 02" = phase-0 audit plan.)

| ID | Discrepancy | Detail / evidence | Disposition |
|---|---|---|---|
| DR-01 | **3.0.24 front-door drift** | 3.0.24 published 2026-09-22 (>130 security fixes, FFmpeg 4.4→8.1.2); videolan.org Windows/macOS download pages now serve 3.0.24; Debian trixie-security candidate is `3.0.24-0+deb13u1` (unversioned apt install now yields 3.0.24) | Baseline **stays 3.0.23** (MP §1; doc 02 §2.5). All pinned URLs remain valid + hash-verifiable. Future Linux acquisitions must pin `=3.0.23-0+deb13u1` explicitly. Re-pinning = Lead decision |
| DR-02 | **Expired VideoLAN signing key; signature predates expiry** | "VideoLAN Release Signing Key (2018)" DSA-1024/SHA-1, fingerprint `65F7C6B4206BD057A7EB73787180713BE58D1ADC`, expired 2026-08-01. All observed signatures created 2025-12-31 (before expiry) and verify **Good** with an expiry warning. Windows agent anchored the key via `keyserver.ubuntu.com`; macOS agent via `keys.openpgp.org` — same fingerprint, mutually corroborating, but neither is a VideoLAN-controlled anchor [VERIFY] | Recorded, not resolved: **Lead ratifies acceptance** (§9). Hash-sidecar matches independently corroborate integrity |
| DR-03 | **`macos/` vs `macosx/` URL segment** | doc 13 card 0.A3 wrote `…/3.0.23/macos/…` → nginx 404; official segment is `macosx/` (doc 02 §2.2 was already correct) | Corrected in all records; doc 13 spelling fixed by Lead at gate review |
| DR-04 | **No official `.sha512` sidecars anywhere** | Windows, macOS and the upstream tarball publish only `.md5/.sha1/.sha256/.asc`. Card assumptions of `.sha512` sidecars were wrong | All SHA-512 values in the records are **our own computations**, labeled as such, for future self-re-verification |
| DR-05 | **`--list-plugins` is a VLC 4.x flag** | On 3.0.23: `unknown option`, exit 1 (raw/linux/07). Cards 0.A1–0.A3 requested it | 3.0.x equivalents `--list`/`--list-verbose` captured instead; later inventory cards (0.B*) and the harness must use the 3.0.x flags |
| DR-06 | **PE32-i386 NSIS installer stub on the "win64" artifact** | `file`: PE32 i386 GUI, Nullsoft self-extracting — the stub is 32-bit; "win64" refers to the installed payload | [VERIFY on hardware] that installed binaries under `C:\Program Files\VideoLAN\VLC` are x64 PE32+ (hardware-template §3.1 step 7) |
| DR-07 | **Debian-over-Ubuntu Linux pin (pending Lead ratification)** | doc 02 §2.2 default "Ubuntu 24.04 LTS" is unusable: noble ships 3.0.20, plucky 3.0.21-9, questing 3.0.21-11 (Launchpad API). Debian trixie/main carries exactly `3.0.23-0+deb13u1` | Decision recorded by 0.A2, **Lead ratifies** (§9). Flathub 3.0.23 recorded as secondary |
| DR-08 | **`docs/baseline/` vs `docs/baselines/`** | doc 02 §1.2/§2.3 and doc 13 §3 write singular; D-007 decided plural; doc 12 §10 OQ-3 tracks the alignment | Records live in `docs/baselines/`; Lead aligns doc 02/13 text at gate review |
| DR-09 | **Debian build-string nuance** | trixie/main `.deb` reports "Compiled by Debian buildd for **trixie-security**" while the artifact comes from the `main` pool (signed-index `Filename:` field) | Recorded verbatim, no interpretation (Linux record §9.6) |
| DR-10 | **Release-date sourcing differs in confidence across records** | 0.A1 and 0.A3 treat news-index anchor `news-2026-01-08` as verification; 0.A2 kept a [VERIFY] ("release page has no machine-readable date"). Same underlying evidence (news index), independently fetched by two agents | Consolidated: date **2026-01-08** stands on the news index + MP §1 agreement; Linux record's V4 is considered closed by cross-record corroboration (noted, not silently dropped) |
| DR-11 | **Linux module census scope** | The 502-module dump covers only `vlc-plugin-base` + `vlc-plugin-video-output` (headless container); `vlc-plugin-qt` + 9 other optional packages not downloaded. `vlc-plugin-pipewire`/`-bittorrent` exist only in the 3.0.24 security upload (not 3.0.23) | Annotated; full-display-stack census = hardware run (hardware-template §5). Inventory agents (0.B2) must not treat 502 as the complete Debian build |
| DR-12 | **`snapshot.debian.org` unreachable (HTTP 509)** | Permanent-archive fallback for the Debian pin unverified from the pin sandbox | [VERIFY on next acquisition with egress] — hardware-template §5.3 |
| DR-13 | **`get.videolan.org` serves an HTML redirect page for direct GETs** | Final mirror observed `mirror-hk.koddos.net`; content identical to `download.videolan.org` origin | `mirror_served` recorded accordingly in all records/JSON |
| DR-14 | **Windows/macOS sandbox-execution impossible** | Binaries acquired, hashed, GPG-verified on Linux x86_64 only; never executed (no wine attempted, none possible for dmgs) | Runbooks + expected artifacts in `hardware-template.md`; owners in §9 |
| DR-15 | **Sandbox CPU model not recorded (Linux)** | Container vCPU model unknown at pin time | Recorded on hardware run (hardware-template §5.1); doc 02 §2.2 step 1 field |
| DR-16 | **macOS raw evidence contains one HTML file** | `raw/macos/00-macosx-directory-listing.html` — verbatim listing page, kept as fetched; all other raw files are .txt | Noted for the Lead; no content concern (server-generated listing) |

Superseded/local items already handled inside single records (not cross-platform):
raw/windows/ directory did not pre-exist (created by 0.A1 — D2 there); an untracked
`macosx-listing-download.html` briefly sat at repo root during the wave (0.A1 D8) and
is no longer present at consolidation time (verified: `git status` clean at tree level
for tracked content).

## 8. Clean-room compliance statement (baseline program)

Consolidated statement for the whole wave-0.A pinning program (per MP §2/§9/§13
and doc 02 §2.1), on behalf of 0.A1–0.A4:

1. **No VLC source** was downloaded, read or stored at any point. The upstream
   `vlc-3.0.23.tar.xz` was deliberately not fetched; only its `.sha256` sidecar
   *text* was saved as release-provenance cross-evidence. No VLC code, comments,
   tables, strings, assets, logos, shaders or branding entered the repository.
2. **Binaries never committed.** All downloads (exe, 7z, dmgs, debs, extracted
   trees, support libraries) live only under `tmp/pinning/`, which is gitignored
   (`git check-ignore` verified; `*.exe` additionally ignored). The repository
   contains **text evidence only**: commands, verbatim outputs, hashes, URLs,
   listings. Verified at consolidation time: `git status` shows no binary
   artifacts.
3. **Oracle-only usage.** The GPL binaries are used exclusively as externally
   executed reference oracles (MP §2 fixed decision): hash/signature
   verification, CLI observation (`--version`, `--list`, `--list-verbose`,
   `-I dummy` smoke). Never linked against, never vendored, never shipped.
   Windows/macOS binaries were never executed at all (impossible in the Linux
   pin sandbox; no wine).
4. **Observation channels are clean-room-safe**: CLI text dumps and public
   official pages (release page, news index, download pages, archive listings,
   sidecars). No screenshots of VLC UI were taken (doc 02 §2.1).
5. **Provenance**: every artifact is re-downloadable from the recorded official
   URLs and verifiable via the recorded SHA-256 (+ our SHA-512) values; the
   Debian pin is additionally anchored by the GPG-signed `InRelease` and
   re-anchored via `snapshot.debian.org` once reachable (DR-12).
6. **Honesty**: all unverifiable items carry `[VERIFY]` markers; the failed
   `--list-plugins` invocation is preserved as a raw dump rather than omitted.

## 9. Lead-ratification checklist (0.A4 prepares; Lead decides)

| # | Decision requested | Context | Default if ratified as-is |
|---|---|---|---|
| R-1 | **Ratify Debian 13 (trixie)/main as the Linux pin** in place of doc 02 §2.2's unusable Ubuntu default (DR-07) | Ubuntu stable series ship 3.0.20–3.0.21 only; Debian carries exact `3.0.23-0+deb13u1`; Flathub 3.0.23 recorded secondary | Linux pin = the 8-package Debian set; doc 02 §2.2 Linux row annotated |
| R-2 | **Accept the expired-key GPG result** as satisfying "signature-check result" for the Windows/macOS sidecar channel (DR-02) | Signatures Good, created 2025-12-31 before the 2026-08-01 expiry; DSA/SHA-1 legacy; hash sidecars corroborate; key anchored via two keyservers (same fingerprint), no VideoLAN-hosted key found | Accept with caveat recorded; Authenticode/codesign (hardware) remain the native channels to close G1 fully |
| R-3 | **Assign owners + target dates for the hardware runs** (Windows install/Authenticode/dumps; macOS codesign/spctl/notarization/dumps; Linux desktop-stack census + snapshot re-fetch + CPU model) | Runbooks + expected artifacts + sign-off fields ready in `hardware-template.md` | Lead owns or delegates; items stay `[VERIFY-blocked-on-hardware]` until then |
| R-4 | **Note DR-08 path spelling** and align doc 02 §1.2/§2.3 + doc 13 §3 to `docs/baselines/` (D-007) at gate review (0.D4-V) | Doc 12 §10 OQ-3 already tracks this | Singular→plural text edit, no artifact moves |
| R-5 | **Confirm the G1 gate posture**: tick G1 as satisfied-with-pending-hardware-items, or hold G1 open until the hardware runs close DR-06/DR-14 items | §6 of this record is the evidence input | Lead decision at 0.D4-V |

## 10. Files in this pin

- `docs/baselines/vlc-3.0.23.md` — this master record.
- `docs/baselines/vlc-3.0.23.json` — machine-readable pin (differential-harness input).
- `docs/baselines/hardware-template.md` — hardware run-record template + oracle-contract validation detail.
- Platform records: `vlc-3.0.23-windows.md`, `vlc-3.0.23-linux.md`, `vlc-3.0.23-macos.md` (+ `raw/{windows,linux,macos}/`).

No binaries, source, or assets are committed anywhere in this repository.
