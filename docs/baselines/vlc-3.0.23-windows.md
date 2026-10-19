# Pin record — VLC 3.0.23 on Windows

| | |
|---|---|
| **Task / agent** | 0.A1 (investigation), wave 0.A |
| **Status** | Pin VERIFIED from official sources (hashes + GPG cross-checked). Install + run-based smoke dumps **[VERIFY-blocked-on-hardware]** — see §6, §7. |
| **Implements** | doc 02 §2 (pinning procedure); feeds gate G1 (doc 02 §1.2) via 0.A4 consolidation |
| **Path note** | Doc 13 §3 writes `docs/baseline/`; D-007 supersedes → `docs/baselines/` (this file). |
| **Recorded** | 2026-10-19, sandbox `Linux x86_64 5.10.134` (cannot execute Windows binaries — see §1) |

---

## 1. Version and provenance

| Field | Value |
|---|---|
| Product | VLC media player |
| Version | 3.0.23 |
| Codename | Vetinari (24th update of the 3.0 branch) |
| Release date | **2026-01-08** — verified: official news page anchor `news-2026-01-08` (`https://www.videolan.org/news.html`), matching MASTER-PROMPT §1. Download-server upload timestamps are 31-Dec-2025 (staging before announcement). |
| Release page | `https://www.videolan.org/vlc/releases/3.0.23.html` (HTTP 200, title "VLC 3.0.23 Vetinari - VideoLAN") |
| Primary artifact | `vlc-3.0.23-win64.exe` (installer) |
| Secondary artifact | `vlc-3.0.23-win64.7z` (no-install archive — future controlled-oracle extract per doc 02 §2.4) |
| Origin URL | `https://download.videolan.org/pub/videolan/vlc/3.0.23/win64/vlc-3.0.23-win64.exe` |
| Mirror URL family | `https://get.videolan.org/vlc/3.0.23/win64/vlc-3.0.23-win64.exe` (identical file set + hashes verified) |
| Mirror served (redirect) | `https://mirror-hk.koddos.net/videolan/vlc/3.0.23/win64/vlc-3.0.23-win64.exe` |
| Acquisition method | `curl -L` over HTTPS (TLS: cert CN=www.videolan.org, Let's Encrypt, default verification) into gitignored `tmp/pinning/` |
| Acquisition date | 2026-10-19 (sandbox) |

Other artifacts officially published for Windows in `3.0.23/` (sidecar SHA-256 values fetched as text; binaries not downloaded here):

| File | Official SHA-256 (sidecar) | Size (listing) |
|---|---|---|
| `vlc-3.0.23-win64.msi` | `bc4b902a480b98a4a5479327a7f210f06369a59bf727649e320faba5b4ef1f5e` | 60,608,512 |
| `vlc-3.0.23-win64.zip` | `992d19dbd0b8a7cde9167d2f7780b1ef6f92acc8a71acfa736101a21f35181e1` | — |
| `vlc-3.0.23-win32/*`, `vlc-3.0.23-winarm64/*` | directories populated (see raw/official-sources.txt §6) | — |

## 2. Primary artifact — file facts and hashes

`vlc-3.0.23-win64.exe`:

| Field | Value | Cross-check |
|---|---|---|
| Size | **45,948,080 bytes** | matches official directory listing (45948080) and final `Content-Length` on HEAD re-check |
| SHA-256 (computed by us) | `20ad191348684b470ddc4e05204316f3d8e39655f412b3e392a0eef97639daaf` | **MATCH** official sidecar on `download.videolan.org` AND `get.videolan.org` (verbatim sidecar text in raw/official-sources.txt §3); **MATCH** doc 02 §2.2 pre-verified value; `sha256sum -c` → `OK` (exit 0) |
| SHA-512 (computed by us) | `fe7aa76985fe90e13a47ddd255bcaafa2bfbc6ece4b998a8031a8e0b6042e3d904b9db03f669cd9580914f050b558bc6541fcaaf1be176993eb847cf739c6f45` | no official `.sha512` sidecar exists in the tree — value is ours, for future re-download checks |
| SHA-1 (computed / sidecar) | `ee04a6c1317bba9c60369a586379ff245abb286c` | `sha1sum -c` → `OK` |
| MD5 (computed / sidecar) | `f711b3aee7c805d52ab28fe53440e35f` | `md5sum -c` → `OK` |
| File type | `PE32 executable for MS Windows 4.00 (GUI), Intel i386 (stripped to external PDB), Nullsoft Installer self-extracting archive, 7 sections` | see note D3 (§9) — NSIS stub is 32-bit; installed payload expected 64-bit **[VERIFY on hardware]** |
| Server Last-Modified | Wed, 31 Dec 2025 12:48:04 GMT | matches listing timestamp |

`vlc-3.0.23-win64.7z` (secondary, downloaded + verified same session):

| Field | Value |
|---|---|
| Size | 39,583,707 bytes |
| SHA-256 (computed) | `eb4fd8a28291da73608c733786a09610fea865fbe94113bcb60b91c1ebb8404a` — `sha256sum -c` vs official sidecar → **OK** |
| SHA-512 (computed) | `97014130095f9af4b3808c4d17fad3c49a21f41c02a2d8d57e656c8635af238973eeec425b3e9a68da8d4ce6dd5bc64294bc0ad3b3bce818dcc301077680af82` |
| File type | 7-zip archive data, version 0.4 |

## 3. Signature verification result

**What VideoLAN publishes (observed 2026-10-19):**

- Per-binary sidecars: `.md5`, `.sha1`, `.sha256` and a detached GPG `.asc` for every Windows artifact. No `.sha512` sidecars.
- No VideoLAN-hosted copy of the signing key was found (`/keys/` → 404, `/pub/videolan/KEYS` → 404).

**GPG result (executed in sandbox, gpg 2.4.7):**

```
$ gpg --verify vlc-3.0.23-win64.exe.asc vlc-3.0.23-win64.exe
gpg: Signature made Wed Dec 31 14:49:27 2025 UTC
gpg:                using DSA key 65F7C6B4206BD057A7EB73787180713BE58D1ADC
gpg: Good signature from "VideoLAN Release Signing Key (2018)" [expired]
gpg: Note: This key has expired!
Primary key fingerprint: 65F7 C6B4 206B D057 A7EB  7378 7180 713B E58D 1ADC
```

- Result: **Good signature** (exit 0). Signature created 2025-12-31 14:49:27 UTC; key (DSA-1024, uid "VideoLAN Release Signing Key (2018)", self-sigs 2013→2025) **expired 2026-08-01**, i.e. after this signature was made.
- Key obtained from `keyserver.ubuntu.com` — a third-party anchor, not a VideoLAN-controlled HTTPS page. **[VERIFY: strengthen anchor — confirm fingerprint `65F7C6B4206BD057A7EB73787180713BE58D1ADC` against a VideoLAN-published source or known-good out-of-band copy.]**
- **Authenticode (Windows-native signature of the .exe): NOT CHECKED — [VERIFY-blocked-on-hardware].** `osslsigncode` is not installed in this sandbox and there is no Windows host. Runbook command in §6 step 2.

## 4. Machine-readable record (feeds 0.A4's `vlc-3.0.23.json`)

```json
{
  "platform": "windows",
  "os": "NOT INSTALLED (Linux sandbox acquisition) — install/run on hardware [VERIFY-blocked-on-hardware]",
  "artifact_url": "https://download.videolan.org/pub/videolan/vlc/3.0.23/win64/vlc-3.0.23-win64.exe",
  "mirror_url_family": "https://get.videolan.org/vlc/3.0.23/win64/vlc-3.0.23-win64.exe",
  "mirror_served": "https://mirror-hk.koddos.net/videolan/vlc/3.0.23/win64/vlc-3.0.23-win64.exe",
  "file_name": "vlc-3.0.23-win64.exe",
  "size_bytes": 45948080,
  "sha256_computed": "20ad191348684b470ddc4e05204316f3d8e39655f412b3e392a0eef97639daaf",
  "sha256_official_sidecar": "20ad191348684b470ddc4e05204316f3d8e39655f412b3e392a0eef97639daaf",
  "sha512_computed": "fe7aa76985fe90e13a47ddd255bcaafa2bfbc6ece4b998a8031a8e0b6042e3d904b9db03f669cd9580914f050b558bc6541fcaaf1be176993eb847cf739c6f45",
  "sha1_computed": "ee04a6c1317bba9c60369a586379ff245abb286c",
  "md5_computed": "f711b3aee7c805d52ab28fe53440e35f",
  "signature_check": {
    "gpg_sidecar": {
      "tool": "gpg 2.4.7 (Linux sandbox)",
      "result": "Good signature (key expired 2026-08-01, after sig creation 2025-12-31)",
      "key_uid": "VideoLAN Release Signing Key (2018)",
      "key_fpr": "65F7C6B4206BD057A7EB73787180713BE58D1ADC",
      "key_source": "keyserver.ubuntu.com [VERIFY: anchor]",
      "sidecar": "https://download.videolan.org/pub/videolan/vlc/3.0.23/win64/vlc-3.0.23-win64.exe.asc"
    },
    "authenticode": {
      "tool": "Get-AuthenticodeSignature (planned)",
      "result": "[VERIFY-blocked-on-hardware] not run: no Windows host / osslsigncode in sandbox"
    }
  },
  "version_string": "[VERIFY-blocked-on-hardware] expected shape: 'VLC media player 3.0.23 Vetinari (revision ...)' — capture verbatim on install",
  "release_date": "2026-01-08",
  "recorded": "2026-10-19",
  "recorded_by": "0.A1",
  "machine": {
    "acquisition_sandbox": "Linux x86_64, kernel 5.10.134 (download + hash + GPG only; no execution)",
    "install_target": "Windows 11 x64 reference machine [VERIFY-blocked-on-hardware]"
  },
  "secondary_artifact": {
    "file_name": "vlc-3.0.23-win64.7z",
    "size_bytes": 39583707,
    "sha256_computed": "eb4fd8a28291da73608c733786a09610fea865fbe94113bcb60b91c1ebb8404a",
    "sha512_computed": "97014130095f9af4b3808c4d17fad3c49a21f41c02a2d8d57e656c8635af238973eeec425b3e9a68da8d4ce6dd5bc64294bc0ad3b3bce818dcc301077680af82"
  }
}
```

## 5. Clean-room statement

- The `vlc-3.0.23-win64.exe` and `.7z` binaries were downloaded **for hash and signature verification only**. They were **never executed** in this sandbox (a Linux box cannot run them; no attempt was made via wine or similar).
- Both binaries live **only** in `/home/z/my-project/tmp/pinning/`, which is gitignored (verified: `git check-ignore` matches `.gitignore:28 tmp/`; `*.exe` is additionally ignored as a build artifact). They are **never committed** and will never be linked against, vendored, or shipped.
- The VLC **source tarball** (`vlc-3.0.23.tar.xz`, present in the official tree) was **deliberately not downloaded** — doc 02 §2.1 forbids acquiring VLC source. No VLC source, code, comments, tables, assets, logos or branding are in this record or the repo.
- Evidence used: official directory listings, official hash/signature sidecars, PE container metadata (`file`), and public videolan.org pages — all observable, non-source facts.

## 6. Install record — NOT INSTALLED (sandbox) + hardware runbook

**Status: NOT INSTALLED.** This Linux sandbox cannot run Windows binaries. The following is the documented procedure for the Windows reference-machine run; execute verbatim and capture all output into `docs/baselines/raw/windows/` on that machine.

```powershell
# 0. Environment record (mandatory context for every dump)
Get-ComputerInfo | Select-Object WindowsProductName, WindowsVersion, OsArchitecture, CsProcessors | Format-List
winver                            # or: [System.Environment]::OSVersion.VersionString

# 1. Download + hash-verify the pinned installer (PowerShell, run from a scratch dir)
$url = "https://download.videolan.org/pub/videolan/vlc/3.0.23/win64/vlc-3.0.23-win64.exe"
Invoke-WebRequest -Uri $url -OutFile .\vlc-3.0.23-win64.exe
Get-FileHash .\vlc-3.0.23-win64.exe -Algorithm SHA256
#   EXPECT: 20ad191348684b470DDC4E05204316F3D8E39655F412B3E392A0EEF97639DAAF
Get-FileHash .\vlc-3.0.23-win64.exe -Algorithm SHA512
#   EXPECT: FE7AA76985FE90E13A47DDD255BCAAFA2BFBC6ECE4B998A8031A8E0B6042E3D9
#           04B9DB03F669CD9580914F050B558BC6541FCAAF1BE176993EB847CF739C6F45
#   Abort on mismatch (doc 02 §2.4 drift guard).

# 2. Authenticode verification (closes the [VERIFY-blocked-on-hardware] item in §3)
Get-AuthenticodeSignature .\vlc-3.0.23-win64.exe | Format-List Status, StatusMessage, SignerCertificate
#   EXPECT: Status Valid; signer certificate issued to VideoLAN. Record signer CN + thumbprint.

# 3. Install (NSIS silent install; default per-machine path)
.\vlc-3.0.23-win64.exe /S
#   [VERIFY: /S silent flag actually honored by this NSIS installer — else run GUI install
#    and record every clicked option.]
#   Default install dir (NSIS): C:\Program Files\VideoLAN\VLC

# 4. Version check (first dump channel)
& "C:\Program Files\VideoLAN\VLC\vlc.exe" --version | Tee-Object -FilePath vlc-3.0.23-win64--version.txt
#   EXPECT first line shape: "VLC media player 3.0.23 Vetinari (revision <hash>)"
#   followed by compiler/toolchain lines. Capture verbatim into the pin record's
#   version_string field (replaces the [VERIFY] placeholder in §4).

# 5. Optional: extract the no-install 7z instead of/alongside the installer
#    (artifact for the controlled oracle, doc 02 §2.4) — verify hashes first as in step 1
#    using the .7z values in §2.
```

## 7. Smoke-dump runbook — [VERIFY-blocked-on-hardware]

The card's three dump channels cannot be executed here (no Windows host). Exact commands for the hardware run; expected output shapes are from public documentation and **must be replaced by verbatim captures**:

| # | Channel | Command (PowerShell, install dir on PATH or absolute) | Expected shape |
|---|---|---|---|
| 1 | Version | `vlc.exe --version > vlc-3.0.23-win64--version.txt 2>&1` | First line `VLC media player 3.0.23 Vetinari (revision …)`; then build/compiler lines |
| 2 | Module list | `vlc.exe --list > vlc-3.0.23-win64--list.txt 2>&1` | Plugins grouped under category headers (Access, Audio filter, …), plugin names listed per category |
| 3 | Verbose module list | `vlc.exe --list-verbose > vlc-3.0.23-win64--list-verbose.txt 2>&1` | Same categories plus per-module description + capability line |
| 4 | "list-plugins" (card wording) | `vlc.exe --list-plugins > vlc-3.0.23-win64--list-plugins.txt 2>&1` | **[VERIFY: `--list-plugins` is not a documented VLC 3.0.x flag**; if the binary rejects it, channels 2–3 are the module-dump equivalents. Doc 02 §2.2 step 7 requires confirming exact flags from `vlc --help` on the pinned binary — do that first and record the confirmed flag set.] |

Additional smoke per doc 02 §2.2 step 7 (for 0.A4 to validate the oracle invocation contract):

```powershell
vlc.exe -I dummy --verbose 2 --play-and-exit --no-video --no-audio <tiny-sample-file> `
  > vlc-3.0.23-win64--smoke-m1.txt 2>&1
# [VERIFY exact flags from vlc --help on the pinned binary — doc 02 §2.4 M1]
```

Every dump must record: verbatim command line, OS version, arch, date, and the `vlc --version` first line (oracle-identity guard, doc 02 §2.4).

## 8. Lifecycle / drift

- **3.0.24 exists** on official mirrors (directory listing verified); `www.videolan.org/vlc/download-windows.html` already links 3.0.24 as current. **The baseline stays 3.0.23** (MP §1); the pinned URLs above remain valid and hash-verifiable.
- Re-check procedure at each phase gate (doc 02 §2.5): re-download (or re-hash the retained copy), recompute SHA-256/SHA-512, compare against §2, re-run `vlc --version` on hardware; append result to this record.
- If the artifact disappears from all mirrors, the Lead decides the response; vendoring into the repo remains forbidden.

## 9. Discrepancies and notes (honest ledger)

- **D1 — Card vs reality (smoke channels):** doc 13 §3 card names `--list-plugins`; that flag is not a known VLC 3.0.x option (`--list` / `--list-verbose` are). Marked [VERIFY] in §7 rather than silently "corrected".
- **D2 — Path correction honored:** doc 13 writes `docs/baseline/`; D-007 supersedes with `docs/baselines/` (this file). Additionally, the task brief said `docs/baselines/raw/windows/` "already created" — it was **not** (only `docs/baselines/raw/` existed); this task created `raw/windows/`.
- **D3 — Installer stub arch:** `file` reports the .exe as PE32 i386 (NSIS self-extracting stub). The "win64" designation refers to the installed payload; [VERIFY on hardware] that the installed binaries under `C:\Program Files\VideoLAN\VLC` are x64 PE32+.
- **D4 — GPG key expired:** the release key expired 2026-08-01 (signature itself predates expiry). GPG verification succeeds with an expiry warning. Authenticode is the primary Windows authenticity channel and remains unchecked — [VERIFY-blocked-on-hardware] (§3, §6 step 2).
- **D5 — No official SHA-512 sidecars:** only .md5/.sha1/.sha256/.asc are published. SHA-512 values are our own computations.
- **D6 — Release-date sourcing:** MASTER-PROMPT §1's 2026-01-08 is confirmed by the news-page anchor `news-2026-01-08`; the release page itself carries no greppable date; download-server timestamps are 31-Dec-2025 (staging).
- **D7 — Sandbox tooling:** no WebSearch/WebFetch tools exist in this sandbox; all web access was via `curl` (recorded). `osslsigncode` absent. Concurrent wave-0.A agents share `tmp/pinning/` (macOS task's `.dmg` files are visible there; they are not part of this Windows pin — see acquisition-log.txt scope note).
- **D8 — Untracked file at repo root:** `macosx-listing-download.html` (untracked, produced by a concurrent agent, not this task) sits at the repo root at the time of writing; flagged for the Lead — not touched per scope discipline.

## 10. Evidence files

- `docs/baselines/raw/windows/acquisition-log.txt` — every command executed in this sandbox with its actual output (downloads, hash computations + cross-checks, GPG analysis, mirror checks, negative results).
- `docs/baselines/raw/windows/official-sources.txt` — official source citations: URLs, verbatim sidecar text, signature, key facts, negative results.

No VLC binaries, source, or assets are committed anywhere in this repository.
