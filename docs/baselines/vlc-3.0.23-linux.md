# VLC 3.0.23 — Linux Pin Record

| | |
|---|---|
| **Status** | COMPLETE — VLC 3.0.23 ran in the Linux sandbox (baseline-quality, with environment deltas noted below) |
| **Task / agent** | 0.A2 (wave 0.A, doc 13 §3), per doc 02 §2 |
| **Platform** | Linux x86_64 |
| **Pinned version** | VLC **3.0.23 Vetinari**, Debian package **`3.0.23-0+deb13u1`** |
| **Recorded** | 2026-10-19, by agent 0.A2 |
| **Raw evidence** | `docs/baselines/raw/linux/` (dumps 01–11, text only) |
| **Feeds** | Gate G1 (doc 02 §1.2); consolidated JSON `docs/baselines/vlc-3.0.23.json` by Lead/0.A1 |

---

## 1. Executive summary

- **A real VLC 3.0.23 ran headless in this sandbox.** Exact first line of `vlc --version`
  (raw dump 04): `VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)`.
- Pin = **Debian 13 "trixie" main-archive packages** at exactly `3.0.23-0+deb13u1`, acquired
  via `apt-get download` (no root required), every downloaded `.deb` SHA-256 cross-checked
  **MATCH** against Debian's GPG-signed package index (raw dump 02). All 372 shipped plugin
  `.so` files load in the extracted tree (raw dump 03).
- **No "shakeout-only" caveat needed**: the sandbox version *is* 3.0.23. The dumps in
  `raw/linux/` are labeled **baseline** — with one honest environment delta: the Qt GUI
  plugin package (`vlc-plugin-qt`) and 8 other optional plugin packages were not downloaded
  (headless container, no Qt/X); §6.2 quantifies the delta.

## 2. Version and release identification

| Item | Value | Source |
|---|---|---|
| Product | VLC media player | — |
| Version / codename | 3.0.23 "Vetinari" | `vlc --version` (dump 04); release page title |
| Upstream release date | 2026-01-08 | MASTER-PROMPT §1 (baseline premise); release page `https://www.videolan.org/vlc/releases/3.0.23.html` exists [HTTP 200 verified 2026-10-19] |
| Upstream source tarball | `vlc-3.0.23.tar.xz`, 26,486,988 bytes, server mtime 2025-12-23 | `https://get.videolan.org/vlc/3.0.23/` listing (dump 09) |
| Publisher tarball SHA-256 (sidecar) | `e891cae6aa3ccda69bf94173d5105cbc55c7a7d9b1d21b9b21666e69eff3e7e0` | `.tar.xz.sha256` sidecar (dump 10); **tarball NOT downloaded** (clean-room, doc 02 §2.1) |
| Debian package version | `3.0.23-0+deb13u1` (source package `vlc`, Debian Multimedia Maintainers) | signed Packages index via `apt-cache show` (dump 02) |
| Debian build identification | `Compiled by Debian buildd for trixie-security on Debian amd64 buildd (Jan 7 2026 20:01:08)`, `gcc 14.2.0 (Debian 14.2.0-19)` | `vlc --version` (dump 04), verbatim |
| Full version string | `VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)` | dump 04, verbatim |

## 3. Distro choice (decision, evidence-based)

Doc 02 §2.2 defaulted the Linux pin to "Ubuntu 24.04 LTS package" with a [VERIFY] on the
exact version. **That default is unusable — verified via the Launchpad publishing API
(raw dump 11):**

| Ubuntu series | published `vlc` version |
|---|---|
| noble (24.04 LTS) | `3.0.20-3build6` |
| plucky (25.04) | `3.0.21-9build1` |
| questing (25.10) | `3.0.21-11` |

No current Ubuntu stable series ships 3.0.23. **Decision (recorded here, Lead to ratify at
G1): pin Debian 13 (trixie) `main`**, which carries exactly `3.0.23-0+deb13u1`
(`apt-cache policy`: trixie/main 500; trixie-security carries only the successor
`3.0.24-0+deb13u1` — mirror-drift risk noted in §9).

Secondary pin candidate (doc 02 §2.2): Flathub `org.videolan.VLC`, stable = **3.0.23**
(release timestamp 1767225600 = 2026-01-01) — verified via Flathub API v2 (dump 11); not
installed/run here (no flatpak, no root).

## 4. Pinning environment (sandbox)

| Item | Value |
|---|---|
| OS | Debian GNU/Linux 13 (trixie), `VERSION_ID=13`, `DEBIAN_VERSION_FULL=13.4` (dump 01) |
| Kernel / arch | `5.10.134-013.15.kangaroo.al8.x86_64`, x86_64 |
| Display server | none (headless container; no X, no Wayland, no PulseAudio, no D-Bus session) |
| Privileges | unprivileged user `uid=1001(z)`; **no root** (system `apt-get install` not attempted beyond download — see §5) |
| Pre-existing VLC | none (`which vlc` → not found; no vlc packages installed; host has no Qt) |
| Network | egress to `deb.debian.org`, `download.videolan.org`, `get.videolan.org`, `flathub.org`, `api.launchpad.net` verified working; `snapshot.debian.org` **blocked** (HTTP 509 admin policy, dump 11) |

## 5. Acquisition method (2026-10-19, all commands in raw dumps 01–03)

1. **Discovery** (dump 01): `which vlc`, `vlc --version`, `cat /etc/os-release`, `uname -a`,
   `id`, `apt-cache policy vlc`, `apt list --installed | grep -i vlc` (empty),
   `dpkg -l | grep -ci qt` (0).
2. **Download without root** (dump 02): `apt-get download` at exact pinned versions from
   `http://deb.debian.org/debian trixie/main` — 8 packages (pool path
   `pool/main/v/vlc/`): `vlc`, `vlc-bin`, `vlc-data`, `libvlc-bin`, `libvlc5`, `libvlccore9`,
   `vlc-plugin-base`, `vlc-plugin-video-output`, all `=3.0.23-0+deb13u1`. 4,749 kB total.
   `apt-get install` was **not** attempted (no root; download-only mode is the root-free path).
3. **Extract, don't install**: `dpkg-deb -x <deb> tmp/pinning/extract/` for every package;
   run as `LD_LIBRARY_PATH=<extract>/usr/lib/x86_64-linux-gnu:<extract>/usr/lib/x86_64-linux-gnu/vlc <extract>/usr/bin/vlc …`.
   VLC locates its plugin cache relative to `libvlccore.so.9`, so no `VLC_PLUGIN_PATH` was needed.
4. **Support libraries** (dump 03): the minimal container lacks `vlc-plugin-base`'s optional
   library deps (43 of 372 plugin `.so` files had unmet `DT_NEEDED` entries). Downloaded 32
   additional trixie library `.deb`s (soname→package resolved via the official
   `Contents-amd64.gz` index; round-1 name guesses failed — recorded — round 2/3 succeeded)
   and extracted them into the same tree. Result: **372/372 plugins load** (ldd-verified,
   transitive check clean). These 32 libs are *not* part of the pin definition; a root-installed
   reference machine gets them automatically via apt Depends/Recommends.
5. **Reproducibility anchor**: permanent archive fallback
   `https://snapshot.debian.org/package/vlc/3.0.23-0+deb13u1/` — recorded but unreachable
   from this sandbox (HTTP 509) → [VERIFY on next acquisition].

## 6. Pinned artifacts, sizes, hashes

### 6.1 The eight pinned packages (the Linux oracle definition)

| Package | Size (bytes) | SHA-256 (computed = published, MATCH) |
|---|---|---|
| `vlc_3.0.23-0+deb13u1_amd64.deb` | 112,404 | `68087c1c26cb640440e1d5d327acc1f17849689d3fa8b658797225fd225986c6` |
| `vlc-bin_3.0.23-0+deb13u1_amd64.deb` | 130,020 | `116a6af7bb48414475ae4ec9c852fd440370bfdb4604ad442df5697adf010ffe` |
| `vlc-data_3.0.23-0+deb13u1_all.deb` | 423,200 | `4a05a94246c57a04bf713dd4bfcad827d83de01cc38383f057d6e06a861f8e3a` |
| `libvlc-bin_3.0.23-0+deb13u1_amd64.deb` | 94,240 | `bc6857db7144bfd7447a77757689e43b8097e4b1b7cd6433b93b9451e83af8d6` |
| `libvlc5_3.0.23-0+deb13u1_amd64.deb` | 142,172 | `73a7789c1aa31fba279544c48088416e0658702cb7aa4ec4385995dc418a9a73` |
| `libvlccore9_3.0.23-0+deb13u1_amd64.deb` | 515,116 | `df7767917a140e331c8d4ed9c71974446c72e6cd9df67c910e391f3ba3d57770` |
| `vlc-plugin-base_3.0.23-0+deb13u1_amd64.deb` | 3,096,972 | `2a546b92264e1f8f2fbfb8c22963ce2f14854e33bb8d8ed1e41b80c022a476f9` |
| `vlc-plugin-video-output_3.0.23-0+deb13u1_amd64.deb` | 234,636 | `66989412853e506bc683398194a5a1de36d19d09fe10f8cd73c3ef67b212dd91` |

Publisher hash source: `apt-cache show <pkg>=3.0.23-0+deb13u1` `SHA256:` field — served
from the apt package index whose authenticity is anchored by the **GPG-signed
`dists/trixie/InRelease`**: verified with
`gpg --no-default-keyring --keyring /usr/share/keyrings/debian-archive-keyring.gpg --verify InRelease`
→ **Good signature** from *Debian Archive Automatic Signing Key (12/bookworm)* and
*(13/trixie)* (dump 02, verbatim). Canonical URLs:
`http://deb.debian.org/debian/pool/main/v/vlc/<file>` (HTTPS also served).

### 6.2 Environment delta (what the sandbox run does NOT include)

Not downloaded (headless container): `vlc-plugin-qt` (Qt GUI — needs Qt5 libs absent on
host), `vlc-plugin-skins2`, `vlc-plugin-notify`, `vlc-plugin-samba`, `vlc-plugin-jack`,
`vlc-plugin-svg`, `vlc-plugin-fluidsynth`, `vlc-plugin-visualization`,
`vlc-plugin-video-splitter`, `vlc-plugin-access-extra`. All of these also exist at
`3.0.23-0+deb13u1` in trixie/main (verified via `apt-cache policy`, dump 03) — a reference
machine with a desktop stack should install them for the full module census; inventory
agents must not treat the 502-module dump as the complete Debian build. (`vlc-plugin-pipewire`
and `vlc-plugin-bittorrent` exist **only** in the 3.0.24 security upload — not part of 3.0.23.)

## 7. Run status in sandbox (what actually ran)

| Check | Result | Raw dump |
|---|---|---|
| `vlc --version` | exit 0; first line `VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)` | 04 |
| `vlc --list` (module list) | exit 0; **502 modules**, one line each | 05 |
| `vlc --list-verbose` | exit 0; 502 modules with shortcuts + capability + score | 06 |
| `vlc --list-plugins` | **flag does not exist on 3.0.x** (VLC 4.x flag) → usage error, exit 1; honest failure record | 07 |
| `vlc -I dummy --play-and-exit vlc://quit` | exit 0; dummy interface works; expected headless errors (no PulseAudio server, no D-Bus) then clean exit | 08 |
| Plugin `.so` loadability | 372/372 pass `ldd` with zero missing deps | 03 |

Plugin capability directories present: access, access_output, audio_filter, audio_mixer,
audio_output, codec, control, demux, gui (dummy/ncurses/rc/hotkeys — **not** Qt), keystore,
logger, lua, meta_engine, misc, mux, packetizer, services_discovery, spu, stream_extractor,
stream_filter, stream_out, text_renderer, vaapi, vdpau, video_chroma, video_filter,
video_output. Binaries in `usr/bin`: `vlc`, `cvlc`, `nvlc`, `rvlc`, `vlc-wrapper`.

**Label: BASELINE** (not "methodology shakeout"): the executed binary *is* the pinned
3.0.23 Debian build, hash-verified before running. Caveat repeated: module set = base +
video-output packages only (§6.2).

## 8. Clean-room statement

No VLC source code was downloaded, read or stored: the upstream `vlc-3.0.23.tar.xz` was
deliberately **not** fetched (doc 02 §2.1); only its `.sha256` **sidecar text file** was
saved as release-provenance cross-evidence (dump 10). No VLC binaries, `.deb` payloads or
extracted trees are committed to the repository: all downloads live under
`/home/z/my-project/tmp/pinning/` (git-ignored scratch, never committed); the repo contains
only text records — commands, outputs, hashes, URLs. Observation was limited to running the
oracle's own CLI (`--version`, `--list`, `--list-verbose`, `-I dummy` smoke). Running VLC
externally as reference oracle is explicitly allowed (MASTER-PROMPT §2).

## 9. Discrepancies, risks and notes

1. **`--list-plugins` (task-card assumption) does not exist on VLC 3.0.x** — it is a VLC 4.x
   flag. 3.0.x equivalents `--list` / `--list-verbose` captured instead (dumps 05–07). Later
   inventory cards must use the 3.0.x flags.
2. **No `.sha512` sidecar exists for the 3.0.23 tarball** (task-card assumption wrong):
   `get.videolan.org` → HTTP 500, `download.videolan.org` → HTTP 404. Published sidecars:
   `.asc` (GPG), `.md5`, `.sha1`, `.sha256` (dump 09). The `.sha256` sidecar was recorded (dump 10).
3. **`get.videolan.org` serves an HTML mirror-redirect page**, not the raw sidecar, for
   direct file GETs; final mirror observed: `mirror-hk.koddos.net`; content identical to the
   `download.videolan.org` origin (dump 10). Record `mirror_served` accordingly.
4. **Doc 02 §2.2 default distro (Ubuntu 24.04) impossible** for 3.0.23 (§3) — pinning
   decision deviates to Debian trixie; Lead to ratify in G1/exit report.
5. **Version drift risk**: trixie-security already offers `3.0.24-0+deb13u1` as apt
   Candidate; any unversioned `apt-get install/download` now yields 3.0.24. All future
   acquisitions must pin `=3.0.23-0+deb13u1` explicitly and re-run the §6.1 hash check
   (doc 02 §2.5). Upstream has also published 3.0.24 (doc 02 §2.5) — baseline stays 3.0.23.
6. **Build-string nuance**: the trixie/main `.deb` reports "Compiled by Debian buildd for
   trixie-security" — Debian's security buildds built this binary; the artifact itself comes
   from the `main` pool (Filename: `pool/main/v/vlc/…` in the signed index, dump 02). Recorded
   verbatim, no interpretation beyond that.
7. **Headless-only evidence**: no GUI/display/audio-stack behavior is covered by this run;
   Qt-interface-dependent inventory (menus, dialogs, hotkeys) needs a reference machine or
   container with a display stack (§10 runbook) — relevant to 0.A4/0.A5-style cards on Linux.
8. `snapshot.debian.org` unreachable from this sandbox (HTTP 509) — permanent-archive
   fallback unverified from here [VERIFY].
9. Upstream release page exists and titles "VLC 3.0.23 Vetinari"; the page carries no
   machine-readable date in our fetch — release date rests on MASTER-PROMPT §1 (2026-01-08)
   [VERIFY if the exact day ever matters].

## 10. Runbook — reference-machine install (for later hardware runs / CI)

```bash
# 0) Preconditions: Debian 13 (trixie) amd64; record /etc/os-release, uname -a, CPU, display server.
# 1) Pin the exact version (never rely on the Candidate — it may be 3.0.24 from -security):
apt-get install --no-install-recommends -y \
  vlc=3.0.23-0+deb13u1 vlc-bin=3.0.23-0+deb13u1 vlc-data=3.0.23-0+deb13u1 \
  libvlc-bin=3.0.23-0+deb13u1 libvlc5=3.0.23-0+deb13u1 libvlccore9=3.0.23-0+deb13u1 \
  vlc-plugin-base=3.0.23-0+deb13u1 vlc-plugin-video-output=3.0.23-0+deb13u1 \
  vlc-plugin-qt=3.0.23-0+deb13u1 vlc-plugin-skins2=3.0.23-0+deb13u1 \
  vlc-plugin-notify=3.0.23-0+deb13u1 vlc-plugin-samba=3.0.23-0+deb13u1 \
  vlc-plugin-svg=3.0.23-0+deb13u1 vlc-plugin-visualization=3.0.23-0+deb13u1 \
  vlc-plugin-video-splitter=3.0.23-0+deb13u1 vlc-plugin-access-extra=3.0.23-0+deb13u1
#    (add vlc-plugin-jack / vlc-plugin-fluidsynth if the audio stack is present)
# 2) Verify: vlc --version | head -1   ->  "VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)"
# 3) Re-verify artifact hashes (installed-from-cache or re-download path) per §6.1 and
#    re-check the InRelease signature; record both outputs.
# 4) Headless channels: vlc --list ; vlc --list-verbose ; vlc -I dummy --play-and-exit <input>
#    (NOTE: --list-plugins does NOT exist on 3.0.x — see §9.1)
# 5) Root-free sandbox reproduction (this record's exact path): see raw dumps 02–03
#    (apt-get download + dpkg-deb -x + LD_LIBRARY_PATH; support libs listed in dump 03).
```

## 11. Machine-readable pin (for Lead consolidation into `docs/baselines/vlc-3.0.23.json`)

```json
{
  "platform": "linux",
  "os": "Debian GNU/Linux 13 (trixie), kernel 5.10.134, x86_64, headless container",
  "artifact_url": "http://deb.debian.org/debian/pool/main/v/vlc/",
  "packages": [
    {"name": "vlc", "version": "3.0.23-0+deb13u1", "size": 112404, "sha256": "68087c1c26cb640440e1d5d327acc1f17849689d3fa8b658797225fd225986c6"},
    {"name": "vlc-bin", "version": "3.0.23-0+deb13u1", "size": 130020, "sha256": "116a6af7bb48414475ae4ec9c852fd440370bfdb4604ad442df5697adf010ffe"},
    {"name": "vlc-data", "version": "3.0.23-0+deb13u1", "size": 423200, "sha256": "4a05a94246c57a04bf713dd4bfcad827d83de01cc38383f057d6e06a861f8e3a"},
    {"name": "libvlc-bin", "version": "3.0.23-0+deb13u1", "size": 94240, "sha256": "bc6857db7144bfd7447a77757689e43b8097e4b1b7cd6433b93b9451e83af8d6"},
    {"name": "libvlc5", "version": "3.0.23-0+deb13u1", "size": 142172, "sha256": "73a7789c1aa31fba279544c48088416e0658702cb7aa4ec4385995dc418a9a73"},
    {"name": "libvlccore9", "version": "3.0.23-0+deb13u1", "size": 515116, "sha256": "df7767917a140e331c8d4ed9c71974446c72e6cd9df67c910e391f3ba3d57770"},
    {"name": "vlc-plugin-base", "version": "3.0.23-0+deb13u1", "size": 3096972, "sha256": "2a546b92264e1f8f2fbfb8c22963ce2f14854e33bb8d8ed1e41b80c022a476f9"},
    {"name": "vlc-plugin-video-output", "version": "3.0.23-0+deb13u1", "size": 234636, "sha256": "66989412853e506bc683398194a5a1de36d19d09fe10f8cd73c3ef67b212dd91"}
  ],
  "sha256_source": "apt signed Packages index (apt-cache show), anchored by GPG-signed dists/trixie/InRelease",
  "hash_crosscheck": "8/8 MATCH (computed vs published)",
  "signature_check": {"tool": "gpg --keyring /usr/share/keyrings/debian-archive-keyring.gpg --verify InRelease", "result": "Good signature x2 (Debian Archive Automatic Signing Key 12/bookworm + 13/trixie)"},
  "mirror_served": "deb.debian.org (fastly CDN); videolan sidecar mirror: mirror-hk.koddos.net",
  "version_string": "VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)",
  "run_mode": "extracted .deb tree, LD_LIBRARY_PATH shim, headless (-I dummy); 502 modules load (base+video-output only; qt/skins2/notify/samba/jack/svg/fluidsynth/visualization/video-splitter/access-extra not included)",
  "upstream_tarball_sha256_sidecar": "e891cae6aa3ccda69bf94173d5105cbc55c7a7d9b1d21b9b21666e69eff3e7e0 (vlc-3.0.23.tar.xz, NOT downloaded)",
  "recorded": "2026-10-19", "recorded_by": "0.A2",
  "machine": {"cpu": "container vCPU (model not recorded) [VERIFY on hardware]", "display": "none (headless)", "notes": "no root; no X/Wayland/PulseAudio/D-Bus; snapshot.debian.org blocked (HTTP 509)"}
}
```

## 12. [VERIFY] ledger for this record

| # | Item | Status |
|---|---|---|
| V1 | `snapshot.debian.org` permanent archive of `3.0.23-0+deb13u1` | blocked from sandbox (509) — verify on next machine with egress |
| V2 | Flathub `org.videolan.VLC` build commit/runtime equality with 3.0.23 | API shows 3.0.23 stable; not executed here |
| V3 | Full-plugin-set module census (Qt, skins2, …) on a desktop-stack machine | runbook §10, for inventory cards |
| V4 | Exact upstream release day (2026-01-08 per MASTER-PROMPT §1) | release page exists, no machine-readable date in fetch |
| V5 | CPU model of sandbox container | not recorded (hardware-template.md is for reference machines) |

## 13. Sources

- Debian archive: `http://deb.debian.org/debian/dists/trixie/InRelease` (GPG-signed);
  `…/dists/trixie/main/Contents-amd64.gz`; pool `…/pool/main/v/vlc/`.
- VideoLAN: `https://get.videolan.org/vlc/3.0.23/` (directory listing);
  `https://download.videolan.org/vlc/3.0.23/vlc-3.0.23.tar.xz.sha256`;
  `https://www.videolan.org/vlc/releases/3.0.23.html`.
- Flathub API v2: `https://flathub.org/api/v2/appstream/org.videolan.VLC`.
- Launchpad API: `https://api.launchpad.net/1.0/ubuntu/+archive/primary?ws.op=getPublishedBinaries&binary_name=vlc…`.
- Internal: doc 02 §2 (procedure), MASTER-PROMPT §1/§2, decision D-007 (`docs/baselines/`).
