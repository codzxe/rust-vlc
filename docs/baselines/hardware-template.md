# Hardware run-record template — VLC 3.0.23 baseline completion

| | |
|---|---|
| **Task / agent** | 0.A4 (wave 0.A) — template; runs executed later by Lead-assigned owners |
| **Purpose** | (1) Close every `[VERIFY-blocked-on-hardware]` item from the three platform pins (doc 02 §2.2 steps 1, 4–7; gate G1 partial rows); (2) record reference-machine environments for doc 12 §3.5/§4.1 (machine spec, `machine_id`); (3) validate the oracle invocation contract (doc 02 §2.4) on real hardware. |
| **Scope** | Template + runbooks only. No performance numbers (Phase 8 per doc 02 §1.3, doc 12 §4.1); no inventory beyond the pinned smoke channels (0.B* cards own the census). |
| **Source of commands** | Verbatim from the platform pin records (`vlc-3.0.23-{windows,linux,macos}.md`); no new procedures invented here. |
| **Master pin** | `docs/baselines/vlc-3.0.23.md` + `vlc-3.0.23.json` (hashes to assert before every step below). |

---

## 1. Generic rules for every hardware run

1. **Drift guard first (doc 02 §2.4):** before any invocation, re-verify the
   artifact hash (values in the master pin §1 / JSON) and, once installed,
   assert the `vlc --version` first line matches the recorded pin. Abort the run
   on mismatch; a drifted oracle invalidates evidence.
2. **Every dump records its own context** (prepend to the file or record in the
   environment dump): verbatim command line, date/time UTC, OS version + arch,
   CPU model, and the `vlc --version` first line.
3. **All artifacts land under `docs/baselines/raw/<platform>/`** with the exact
   filenames listed per section below. Text only — no screenshots (doc 02 §2.1).
4. **Binaries never enter the repo**; install only on reference/CI machines.
5. **Honesty:** if a step fails or a flag is rejected, record the verbatim
   failure — never omit it (precedent: raw/linux/07, the `--list-plugins`
   failure record).

### 1.1 Environment fields (record per machine, once + on any change)

| Field | Windows | Linux | macOS |
|---|---|---|---|
| `machine_id` | stable short id, cited by every later run/baseline (doc 12 §3.5) | same | same |
| OS name + version + build | `Get-ComputerInfo` (WindowsProductName, WindowsVersion, OsBuildNumber) | `/etc/os-release`, `uname -a` | `sw_vers`, `uname -a` |
| Arch | `OsArchitecture` | `uname -m` | `uname -m` (+ `sysctl -n sysctl.proc_translated` for Rosetta note) |
| CPU model, cores, freq | `CsProcessors` | `lscpu` (model name, cores, MHz) | `sysctl -n machdep.cpu.brand_string`, `sysctl -n hw.ncpu` |
| RAM | `OsTotalVisibleMemorySize` | `free -h` | `sysctl -n hw.memsize` |
| GPU + driver | `Get-CimInstance Win32_VideoController` | `lspci -nn \| grep -i vga` + driver version (`glxinfo`/`vainfo` if present) | `system_profiler SPDisplaysDataType` |
| Display server / compositor | n/a (record DPI scaling) | X11 or Wayland + compositor (doc 02 §2.2 step 1) | n/a (record display scaling) |
| Audio device | default output device | `pactl info` (PulseAudio) / PipeWire presence | default output device |
| Locale | `Get-WinSystemLocale` | `locale` | `locale` |
| Date (UTC) + operator | free text | free text | free text |

Expected filename: `raw/<platform>/<platform>-hardware-00-environment.txt`.

### 1.2 Sign-off block (append to each completed section's record file)

```
Run sign-off — <platform> <section>
Operator: <name/handle>          Date (UTC): <YYYY-MM-DD HH:MM>
machine_id: <id>                 Artifacts present: <list — tick each>
[VERIFY] items closed by this run: <list — cite master-pin DR-xx / record §>
Deviations from runbook: <none | describe verbatim>
Result: PASS / FAIL / PARTIAL — notes
Lead review: <pending | reviewed-by-lead on <date>>   (Lead ticks doc 02 §1.2 G1 at 0.D4-V)
```

---

## 2. Oracle invocation contract — validation status (doc 02 §2.4)

Consolidated result after wave 0.A (identical table in the master pin §4; the
hardware runs below move rows from PENDING to PROVEN).

| Mode | Invocation (contract) | Windows | Linux | macOS |
|---|---|---|---|---|
| M1 — headless metadata/stream info | `vlc -I dummy --verbose 2 --play-and-exit --no-video --no-audio <input>` (log parsed; rc-interface alternative open) | PENDING (hardware — §3.1 step 6) | **PROVEN (subset)**: `-I dummy --play-and-exit` exit 0 (raw/linux/08). Full flags + `--verbose 2` log grammar on a real input: PENDING (corpus 0.A12 / Phase 2) | PENDING (hardware — §4.1 step 6) |
| M2 — deterministic decoded output | `--sout` transcode chain, fixed lossless-tolerant parameters | PENDING [VERIFY — Phase 2 measure] | PENDING [VERIFY] | PENDING [VERIFY] |
| M3 — malformed-input error behavior | `--play-and-exit`; exit code + error class | PENDING (corpus 0.A12) | PENDING (corpus 0.A12) | PENDING (corpus 0.A12) |
| M4 — timing (startup/TTF/seek, median+p95) | repeated runs vs this template's machine records | PENDING (Phase 8; protocol fixed) | PENDING (Phase 8) | PENDING (Phase 8) |
| Guard — version assert before every invocation | hash + `--version` first-line check; drift aborts | data pending (hardware) | **data PROVEN** (verbatim line recorded); harness enforcement is Phase 2 code | data pending (hardware) |
| Sandboxing | disposable VM/container, no egress, no credentials | PENDING (owner 0.A11) | PENDING (0.A11) | PENDING (0.A11) |

Flag finding that binds all runs: **`--list-plugins` does not exist on VLC
3.0.x** (4.x flag; exit 1 observed, raw/linux/07). Use `--list` /
`--list-verbose`; still *probe* `--list-plugins` once per platform and record
the rejection (it documents the 3.0.x CLI contract; DR-05).

---

## 3. Windows reference-machine run (closes Windows `[VERIFY-blocked-on-hardware]`)

Source runbook: `vlc-3.0.23-windows.md` §6 (install) + §7 (smoke dumps).
Preconditions: Windows 11 x64 reference machine; PowerShell; scratch dir;
network egress to `download.videolan.org`.

### 3.1 Steps (execute verbatim; capture every output)

| # | Step | Command (PowerShell) | Expected artifact (`raw/windows/`) |
|---|---|---|---|
| 0 | Environment record | `Get-ComputerInfo | Select-Object WindowsProductName, WindowsVersion, OsBuildNumber, OsArchitecture, CsProcessors, OsTotalVisibleMemorySize | Format-List`; `Get-CimInstance Win32_VideoController | Format-List Name, DriverVersion`; `Get-WinSystemLocale`; `[System.Environment]::OSVersion.VersionString` | `windows-hardware-00-environment.txt` |
| 1 | Download + hash-verify (drift guard) | `Invoke-WebRequest -Uri "https://download.videolan.org/pub/videolan/vlc/3.0.23/win64/vlc-3.0.23-win64.exe" -OutFile .\vlc-3.0.23-win64.exe`; `Get-FileHash .\vlc-3.0.23-win64.exe -Algorithm SHA256` → **EXPECT** `20ad191348684b470ddc4e05204316f3d8e39655f412b3e392a0eef97639daaf`; `… -Algorithm SHA512` → **EXPECT** `FE7AA76985FE90E13A47DDD255BCAAFA2BFBC6ECE4B998A8031A8E0B6042E3D904B9DB03F669CD9580914F050B558BC6541FCAAF1BE176993EB847CF739C6F45`. **Abort on mismatch** (doc 02 §2.4). Record which mirror served the redirect. | `windows-hardware-01-download-hashcheck.txt` |
| 2 | Authenticode verification (closes pin §3) | `Get-AuthenticodeSignature .\vlc-3.0.23-win64.exe | Format-List Status, StatusMessage, SignerCertificate` — **EXPECT** Status `Valid`; record signer CN + thumbprint verbatim | `windows-hardware-02-authenticode.txt` |
| 3 | Install (NSIS silent) | `.\vlc-3.0.23-win64.exe /S` — `[VERIFY: /S honored; else GUI install and record every clicked option]`. Default dir `C:\Program Files\VideoLAN\VLC` | `windows-hardware-03-install.txt` (record start/end time + exit; GUI path if taken) |
| 4 | Version dump | `& "C:\Program Files\VideoLAN\VLC\vlc.exe" --version | Tee-Object -FilePath vlc-3.0.23-win64--version.txt` — first line becomes the JSON `version_string` (replaces the [VERIFY] placeholder; **assert it contains `3.0.23 Vetinari`**) | `vlc-3.0.23-win64--version.txt` |
| 5 | Module dumps | `vlc.exe --list > vlc-3.0.23-win64--list.txt 2>&1`; `vlc.exe --list-verbose > vlc-3.0.23-win64--list-verbose.txt 2>&1`; probe: `vlc.exe --list-plugins > vlc-3.0.23-win64--list-plugins.txt 2>&1` (expected: rejected — record; DR-05) | `vlc-3.0.23-win64--list.txt`, `vlc-3.0.23-win64--list-verbose.txt`, `vlc-3.0.23-win64--list-plugins.txt` |
| 6 | M1 smoke (doc 02 §2.2 step 7) | `vlc.exe -I dummy --verbose 2 --play-and-exit --no-video --no-audio <tiny-sample-file> > vlc-3.0.23-win64--smoke-m1.txt 2>&1` — `[VERIFY exact flags from vlc --help first; record the confirmed flag set]` | `vlc-3.0.23-win64--smoke-m1.txt` |
| 7 | x64 payload check (closes DR-06) | `Get-ChildItem "C:\Program Files\VideoLAN\VLC" -Filter *.exe | ForEach-Object { $_.Name }`; verify `libvlc.dll` etc. are PE32+ x64 (e.g. `dumpbin /headers` or PowerShell PE parse; record method) | `windows-hardware-04-x64-payload-check.txt` |
| 8 | Optional controlled-oracle extract | extract `vlc-3.0.23-win64.7z` to a fixed path (hashes first, pin §2 values) for the doc 02 §2.4 controlled oracle | (record in `03-install.txt` or a dedicated file) |

### 3.2 Sign-off

Complete §1.2 block per step-group; the Authenticode result (step 2) and the
verbatim `--version` line (step 4) are the two items G1 needs from this run.

---

## 4. macOS reference-Mac run (closes macOS `[VERIFY-blocked-on-hardware]`)

Source runbook: `vlc-3.0.23-macos.md` §5 (codesign) + §7 (install) + §8 (smoke).
Preconditions: Mac with 64-bit Intel or Apple Silicon, macOS ≥ 10.7.5 (record
actual version); `shasum`, `hdiutil`, `codesign`, `spctl` present.

### 4.1 Steps (execute verbatim; capture every output)

| # | Step | Command (bash/zsh) | Expected artifact (`raw/macos/`) |
|---|---|---|---|
| 0 | Environment record | `sw_vers`; `uname -a`; `sysctl -n machdep.cpu.brand_string`; `sysctl -n hw.ncpu`; `sysctl -n hw.memsize`; `system_profiler SPDisplaysDataType`; `locale`; note Rosetta if applicable | `macos-hardware-00-environment.txt` |
| 1 | Download + hash-verify (drift guard) | `curl -LO https://get.videolan.org/vlc/3.0.23/macosx/vlc-3.0.23-universal.dmg`; `shasum -a 256 vlc-3.0.23-universal.dmg` → **EXPECT** `56ee657c3aaf5c71b4ab7d6e4f4a77f6eca54633e0bf42a93b8116eb1d1f6ec9`. **Abort on mismatch.** | `macos-hardware-01-download-hashcheck.txt` |
| 2 | Attach + codesign/Gatekeeper/notarization (closes pin §5) | `hdiutil attach vlc-3.0.23-universal.dmg` (note volume path); `codesign --verify --deep --strict --verbose=2 "/Volumes/<VOL>/VLC.app"` (expect: valid on disk, satisfies Designated Requirement); `spctl -a -t exec -vv "/Volumes/<VOL>/VLC.app"` (expect: accepted, source=Notarized Developer ID); `codesign -dv --verbose=4 "/Volumes/<VOL>/VLC.app" 2>&1 \| tee codesign-details.txt` (record Authority chain + TeamIdentifier); additionally record notarization details (e.g. `spctl` origin string / `stapler validate` if available). Expected authority string is **not** pre-asserted — record what the run shows [VERIFY]. | `macos-hardware-02-codesign-spctl.txt` + `macos-hardware-02-codesign-details.txt` |
| 3 | Install | `cp -R "/Volumes/<VOL>/VLC.app" /Applications/VLC.app` (replace prior VLC.app first); `hdiutil detach "/Volumes/<VOL>"`; **record which arch slice actually executes** (doc 02 §2.2 note): on Apple Silicon record whether the arm64 slice runs (universal image) | `macos-hardware-03-install.txt` |
| 4 | Version dump | `VLC=/Applications/VLC.app/Contents/MacOS/VLC; "$VLC" --version > vlc-3.0.23-macos--version.txt 2>&1` — first line becomes the JSON `version_string` (**assert it contains `3.0.23 Vetinari`**; do not assume the Linux rendering) | `vlc-3.0.23-macos--version.txt` |
| 5 | Module + options dumps | `"$VLC" --list > vlc-3.0.23-macos--list.txt 2>&1`; `"$VLC" --list-verbose > vlc-3.0.23-macos--list-verbose.txt 2>&1`; `"$VLC" --full-help > vlc-3.0.23-macos--full-help.txt 2>&1`; probe `"$VLC" --list-plugins > vlc-3.0.23-macos--list-plugins.txt 2>&1` (expected: rejected — record; DR-05) | the four files as named |
| 6 | M1 smoke (doc 02 §2.2 step 7) | `"$VLC" -I dummy --verbose 2 --play-and-exit --no-video --no-audio <tiny-sample-file> > vlc-3.0.23-macos--smoke-m1.txt 2>&1` — `[VERIFY exact flags from --help first; record the confirmed flag set]` | `vlc-3.0.23-macos--smoke-m1.txt` |

(Names above consolidate the macos record §8's provisional `version.txt`,
`list.txt`, … into the `vlc-3.0.23-macos--*` convention; the record's §5
`codesign-details.txt` becomes `macos-hardware-02-codesign-details.txt`.)

### 4.2 Sign-off

Complete §1.2 block; the codesign/spctl result (step 2) and the verbatim
`--version` line (step 4) are the two items G1 needs from this run.

---

## 5. Linux desktop-stack run (closes Linux `[VERIFY]` items V1, V3, V5)

Source runbook: `vlc-3.0.23-linux.md` §10. Preconditions: Debian 13 (trixie)
amd64 machine (or container) **with display stack** (X11 or Wayland — record
which), root or equivalent for `apt-get install`, egress to
`deb.debian.org` (+ ideally `snapshot.debian.org`).

### 5.1 Steps

| # | Step | Command (bash, root) | Expected artifact (`raw/linux/`) |
|---|---|---|---|
| 0 | Environment record (closes V5 + DR-15) | `cat /etc/os-release`; `uname -a`; `lscpu` (model name, cores, MHz — **CPU model recorded here**); `free -h`; `lspci -nn \| grep -i vga` + driver; display server + compositor (`echo $XDG_SESSION_TYPE`, `echo $XDG_CURRENT_DESKTOP`); `pactl info`; `locale` | `linux-hardware-00-environment.txt` |
| 1 | Install full plugin set at the exact pin (closes V3; never rely on the Candidate — it is 3.0.24, DR-01) | `apt-get install --no-install-recommends -y vlc=3.0.23-0+deb13u1 vlc-bin=3.0.23-0+deb13u1 vlc-data=3.0.23-0+deb13u1 libvlc-bin=3.0.23-0+deb13u1 libvlc5=3.0.23-0+deb13u1 libvlccore9=3.0.23-0+deb13u1 vlc-plugin-base=3.0.23-0+deb13u1 vlc-plugin-video-output=3.0.23-0+deb13u1 vlc-plugin-qt=3.0.23-0+deb13u1 vlc-plugin-skins2=3.0.23-0+deb13u1 vlc-plugin-notify=3.0.23-0+deb13u1 vlc-plugin-samba=3.0.23-0+deb13u1 vlc-plugin-svg=3.0.23-0+deb13u1 vlc-plugin-visualization=3.0.23-0+deb13u1 vlc-plugin-video-splitter=3.0.23-0+deb13u1 vlc-plugin-access-extra=3.0.23-0+deb13u1` (+ `vlc-plugin-jack` / `vlc-plugin-fluidsynth` if the audio stack is present). Then re-verify per-package SHA-256 against the signed index (pin §6.1 values) and re-check `dists/trixie/InRelease` GPG — record both outputs. | `linux-hardware-01-install-hashcheck.txt` |
| 2 | Full-census dumps (incl. Qt modules — the sandbox's 502-module base set grows here; DR-11) | `vlc --version > vlc-3.0.23-linux-desktop--version.txt 2>&1` (assert first line = `VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)`); `vlc --list > vlc-3.0.23-linux-desktop--list.txt 2>&1`; `vlc --list-verbose > vlc-3.0.23-linux-desktop--list-verbose.txt 2>&1`. Note: `--list-plugins` does not exist on 3.0.x (pin §9.1) — do not use. | the three files as named |
| 3 | snapshot.debian.org re-fetch (closes V1 / DR-12) | `curl -sSI https://snapshot.debian.org/package/vlc/3.0.23-0+deb13u1/` — record status; if reachable, fetch one `.deb` and re-verify its SHA-256 against pin §6.1 | `linux-hardware-02-snapshot-refetch.txt` |
| 4 | Flathub secondary check (optional, closes V2) | If flatpak is available: `flatpak remote-info flathub org.videolan.VLC` — record version/commit; compare reported version to 3.0.23 | `linux-hardware-03-flathub-check.txt` |

### 5.2 Sign-off

Complete §1.2 block; the full-census module count (step 2) and CPU model
(step 0) feed the inventory cards (0.B2) and doc 12's machine records.

---

## 6. After the runs

- Update `vlc-3.0.23.json`: replace `[VERIFY-blocked-on-hardware]`
  `version_string` placeholders with verbatim captures; flip `run_status` to
  `sandbox-verified + hardware-verified`; fill `dumps.available`.
- Update the platform records' `[VERIFY]` ledgers and the master pin §4/§6
  (G1 rows) with links to the new raw files.
- The Lead then ticks doc 02 §1.2 G1 at gate review (0.D4-V) — agents do not
  edit doc 02 (0.A4 explicitly did not).
- Re-check discipline (doc 02 §2.5): at each phase gate, recompute the artifact
  hash and re-run `vlc --version`; append the result to the master pin.
