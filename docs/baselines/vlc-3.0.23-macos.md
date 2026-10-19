# VLC 3.0.23 Pin Record — macOS

Task ID: **0.A3** · Wave: 0.A (baseline pinning) · Recorded: **2026-10-19 (UTC)** · Recorder: sub-agent 0.A3
Feeds gate: **G1** (doc 02 §1.2) for the macOS row of the cross-platform pin.
Path note: per Lead decision D-007 this lives in `docs/baselines/` (doc 13 §3's `docs/baseline/` is superseded).
Raw command/output evidence: `docs/baselines/raw/macos/01..06` (text only; every command + actual result).

---

## 1. Version

| Field | Value | Evidence |
|---|---|---|
| Product | VLC media player ("Vetinari" branch) | release page title: "VLC 3.0.23 Vetinari - VideoLAN" |
| Version | **3.0.23** | official archive tree + release page |
| Release date | **2026-01-08** | videolan.org news index ("VLC 3.0.23 / 2026-01-08"); matches MASTER-PROMPT §1 | 
| Announcement | `https://www.videolan.org/vlc/releases/3.0.23.html` | raw 06, SOURCE 1 |
| Successor (drift) | 3.0.24 published 2026-09-22 — **not** the baseline (MP §1; doc 02 §2.5) | raw 06, SOURCE 2 |

## 2. Arch coverage (what exists officially)

All three macOS artifacts exist for 3.0.23 in the official archive trees
(`https://get.videolan.org/vlc/3.0.23/macosx/` and the identical
`https://download.videolan.org/pub/videolan/vlc/3.0.23/macosx/`; full verbatim listing in raw 01):

| Artifact | Bytes | URL (get.videolan.org primary) |
|---|---|---|
| `vlc-3.0.23-arm64.dmg` | 51,273,389 | `https://get.videolan.org/vlc/3.0.23/macosx/vlc-3.0.23-arm64.dmg` |
| `vlc-3.0.23-intel64.dmg` | 57,544,217 | `https://get.videolan.org/vlc/3.0.23/macosx/vlc-3.0.23-intel64.dmg` |
| `vlc-3.0.23-universal.dmg` | 89,124,160 | `https://get.videolan.org/vlc/3.0.23/macosx/vlc-3.0.23-universal.dmg` |

Per doc 02 §2.2, the **universal** image is the primary pin; arch-specific images are recorded as the
arch coverage. Sidecars published per dmg: `.asc` (GPG), `.md5`, `.sha1`, `.sha256`. No `.sha512`
sidecar, no `.pkg`/`.zip` variants for macOS in this tree. Server file dates: dmg 18-Dec-2025,
sidecars 31-Dec-2025. System requirement (official download page): "VLC media player requires
Mac OS X 10.7.5 or later. It runs on any Mac with a 64-bit Intel processor or an Apple Silicon chip."

## 3. Acquisition method + date

- **Downloaded into gitignored scratch** `/home/z/my-project/tmp/pinning/` on **2026-10-19 (UTC)** from
  the official trees above (arm64 + intel64 from download.videolan.org, universal from get.videolan.org).
  All three first-attempt successes within `--max-time 240`; **no retries needed** (raw 03 has timings:
  31.0 s / 34.7 s / 2.3 s).
- Environment: Linux x86_64 sandbox (kernel 5.10.134), curl 8.14.1, GNU coreutils 9.7, gpg 2.4.7.
  **macOS binaries cannot execute here** — download, hashing and GPG verification only (see §8).
- `file` identifies all three as bzip2-compressed Apple UDIF images; each file's trailing 512 bytes
  carry the `koly` UDIF magic (raw 03) — structural sanity beyond hashes.

## 4. File hashes — computed AND cross-checked

Computed by us (Linux coreutils; raw 04 has verbatim outputs) vs official sidecars (raw 02):

| Artifact | SHA-256 (computed) | Sidecar match | SHA-512 (computed; no official sidecar exists) |
|---|---|---|---|
| arm64 | `fc6fac08d87f538517d44aca0c5e7a244b67c8c4cb589bf478363a7315fd5e0d` | ✅ sha256+sha1+md5 | `8ba24bd8bd52a79433a0daea65e64930b07c297004e2e17a2be5b0b2aa345a4adb8f600f5c957e2ee5193d5c0cb7dab8d8c0f8dfa2967e208e7f20b7ab58de58` |
| intel64 | `ec01530ce69d849dd057fba8876e68ac39bf279dc28de4e9c04e4aec11fc98db` | ✅ sha256+sha1+md5 | `8e964c54132dfb4a2dc67396f6ef11a56b6abb2d57c98125158cc4e04c6650d119d78ec8fd71c707a89f64ba7767f7d48e538caec2d8fdca2b1a1f020e46f608` |
| universal | `56ee657c3aaf5c71b4ab7d6e4f4a77f6eca54633e0bf42a93b8116eb1d1f6ec9` | ✅ sha256+sha1+md5 | `fcdfa8d82ee75522cb0b95f060baa1dab214b760410a1b27027e686df3bcf540a2496243e7f17e7e97aaf79c58af3d2478984116e671258008e053ea67b66fb4` |

- **9/9 hash cross-checks PASS** (3 algorithms × 3 artifacts) — the downloads are byte-identical to
  what VideoLAN's sidecars describe.
- The universal SHA-256 also matches doc 02 §2.2's pre-verified value → doc 02's data point re-verified.
- SHA-1/MD5 sidecars additionally match (raw 04). SHA-512 values are ours alone (recorded for future
  self-re-verification; VideoLAN publishes none).

## 5. Signature verification

**GPG detached signature — VERIFIED in-sandbox (2026-10-19):**
`gpg --verify <dmg>.asc <dmg>` → **Good signature** for all three dmgs from key
`65F7 C6B4 206B D057 A7EB 7378 7180 713B E58D 1ADC` ("VideoLAN Release Signing Key (2018)",
fetched from keys.openpgp.org). Signatures created 2025-12-31 (DSA, SHA-1 digest — legacy).
Honest caveats: gpg reports the signing key as **EXPIRED**; DSA/SHA-1 is legacy crypto. Neither
invalidates the byte-level check, and the sha256 sidecar match (§4) independently corroborates
integrity. Full verbatim output: raw 05.

**Apple codesign / Gatekeeper — [VERIFY-blocked-on-hardware]** (cannot run on Linux). Runbook:

```bash
# on the reference Mac, in the dir holding the downloaded dmg:
shasum -a 256 vlc-3.0.23-universal.dmg          # must equal §4 universal SHA-256
hdiutil attach vlc-3.0.23-universal.dmg          # note the mounted volume path
codesign --verify --deep --strict --verbose=2 "/Volumes/<VOL>/VLC.app"   # expect: valid on disk, satisfies Designated Requirement
spctl -a -t exec -vv "/Volumes/<VOL>/VLC.app"    # expect: accepted, source=Notarized Developer ID
codesign -dv --verbose=4 "/Volumes/<VOL>/VLC.app" 2>&1 | tee codesign-details.txt   # record Authority=Developer ID Application: VideoLAN..., TeamIdentifier
hdiutil detach "/Volumes/<VOL>"
```
Record actual outputs verbatim into `docs/baselines/raw/macos/` when run; paste the Team ID into this
record's §5. Expected authority string is NOT asserted here [VERIFY] — record what the hardware run shows.

## 6. Machine-readable pin (doc 02 §2.3 JSON shape, macOS row)

```json
{
  "platform": "macos",
  "os": "NOT INSTALLED — Linux x86_64 sandbox at pin time; hardware run pending",
  "artifact_url": "https://get.videolan.org/vlc/3.0.23/macosx/vlc-3.0.23-universal.dmg",
  "artifact_url_arch": {
    "arm64":   "https://get.videolan.org/vlc/3.0.23/macosx/vlc-3.0.23-arm64.dmg",
    "intel64": "https://get.videolan.org/vlc/3.0.23/macosx/vlc-3.0.23-intel64.dmg",
    "universal": "https://get.videolan.org/vlc/3.0.23/macosx/vlc-3.0.23-universal.dmg"
  },
  "mirror_served": "download.videolan.org (arm64, intel64) and get.videolan.org (universal); no redirect chain observed",
  "sha256_computed": "56ee657c3aaf5c71b4ab7d6e4f4a77f6eca54633e0bf42a93b8116eb1d1f6ec9",
  "sha256_official_sidecar": "56ee657c3aaf5c71b4ab7d6e4f4a77f6eca54633e0bf42a93b8116eb1d1f6ec9",
  "signature_check": {
    "gpg": {"tool": "gpg 2.4.7", "result": "Good signature (key expired; DSA/SHA-1)", "key": "65F7C6B4206BD057A7EB73787180713BE58D1ADC"},
    "codesign_spctl": {"status": "VERIFY-blocked-on-hardware", "runbook": "docs/baselines/vlc-3.0.23-macos.md §5"}
  },
  "version_string": null,
  "version_string_note": "vlc --version cannot run in this sandbox — [VERIFY-blocked-on-hardware]",
  "recorded": "2026-10-19", "recorded_by": "0.A3",
  "machine": {"cpu": "sandbox x86_64 (download/hash host, not the reference Mac)", "display": "n/a",
              "notes": "reference Mac per docs/baselines/hardware-template.md TBD by hardware run"}
}
```

## 7. Install record

**NOT INSTALLED.** macOS binaries cannot run in this Linux sandbox. Hardware procedure (runbook):

```bash
# 1. download (re-verify hash first, §4)          2. attach + install to /Applications
hdiutil attach vlc-3.0.23-universal.dmg
cp -R "/Volumes/<VOL>/VLC.app" /Applications/VLC.app        # replace any prior VLC.app first
hdiutil detach "/Volumes/<VOL>"
# 3. confirm version (first line is the pin string for doc 02 §2.3)
/Applications/VLC.app/Contents/MacOS/VLC --version
```
On Apple Silicon the universal image's arm64 slice executes; record which arch actually runs
(doc 02 §2.2 note). Install happens only on reference/CI Macs — never into this repository.

## 8. Smoke-dump runbook — [VERIFY-blocked-on-hardware]

Cannot be executed here (Linux sandbox; macOS binary). Exact commands for the hardware run —
the three doc 02 §2.2 observation channels plus the card-requested dumps; flag existence for
`--list-plugins` is unconfirmed on 3.0.23 [VERIFY: run `--help` and record which of these exist]:

```bash
VLC=/Applications/VLC.app/Contents/MacOS/VLC
"$VLC" --version            > version.txt 2>&1     # first line = version_string for the pin json
"$VLC" --list               > list.txt 2>&1        # module list (short)
"$VLC" --list-verbose       > list-verbose.txt 2>&1 # module list w/ descriptions (doc 02 §2.2 ch.1)
"$VLC" --full-help          > full-help.txt 2>&1   # options inventory source (doc 02 §2.2 ch.2)
"$VLC" --list-plugins       > list-plugins.txt 2>&1 # [VERIFY: flag may not exist in 3.0.x — record]
# channel 3: one headless open (flags per doc 02 §2.2 step 7; confirm against --help) [VERIFY]
"$VLC" -I dummy --verbose 2 --play-and-exit --no-video --no-audio <tiny-sample-file> > headless-open.txt 2>&1
```
Capture each command + verbatim output + `sw_vers` + `uname -a` into `docs/baselines/raw/macos/`.
These dumps seed G3/G4/G6 macOS rows and pin the doc 02 §2.4 M1 invocation contract.

## 9. Clean-room statement

The three dmgs were downloaded **solely for hash and signature verification of the pinned oracle**.
They were **never executed** (impossible in this sandbox, and not attempted), **never committed**
(they live only in gitignored `tmp/pinning/`; `.gitignore` line 28 `tmp/`), and **no VLC source code,
tables, strings or assets entered the repository**. Raw evidence files under
`docs/baselines/raw/macos/` contain commands and text outputs only — no binaries, no screenshots,
no VLC-copyrighted content beyond quoted official hash/signature metadata needed for verification.
The GPL oracle is used only as an externally-run reference per MP §2 / doc 02 §2.1.

## 10. Discrepancies & notes

1. **Card URL wrong**: task card 0.A3 (doc 13 §3) writes `.../3.0.23/macos/vlc-3.0.23-arm64.dmg`;
   that path 404s (nginx, verbatim body in raw 01). Official segment is **`macosx/`**. Doc 02 §2.2's
   `get.videolan.org/vlc/3.0.23/macosx/` is correct and was used.
2. **Doc 02 path note**: doc 02 §1.2 G1 names `docs/baseline/…`; D-007 makes it `docs/baselines/…`
   (this file's location). Doc 02 §2.3's `vlc-3.0.23.json` is a cross-platform file owned by the
   G1 assembler (0.A1/Lead); the macOS JSON row is embedded here (§6) for that assembly.
3. **Front-door drift**: videolan.org's macOS download page now serves 3.0.24 (2026-09-22). 3.0.23
   remains on the archive trees used here. Re-pinning stays a Lead decision (doc 02 §2.5).
4. **Signing key expired** (VideoLAN Release Signing Key (2018)) and uses DSA/SHA-1; signatures still
   verify Good; hash sidecars corroborate. If the Lead wants stronger assurance, the hardware run
   should additionally record Apple notarization/codesign details (§5 runbook).
5. **No `.sha512` sidecars** exist for macOS artifacts; SHA-512 values in §4 are computed by us only.
6. **Universal dmg is the pin-of-record** per doc 02 §2.2; arch-specific hashes recorded so either
   slice can be pinned later if the Lead prefers per-arch oracles.
7. Sandbox date is 2026-10-19; server timestamps (dmg 18-Dec-2025, sidecar/signatures 31-Dec-2025)
   pre-date the 2026-01-08 announcement — consistent build→sign→publish ordering, noted for audit trail.
