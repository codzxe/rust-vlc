# packaging evidence — task W24-C (issue #56, cards 8.E1–8.E3)

Local, reproducible proof runs for the packaging lanes. CI-only steps
(`wix build`, `signtool`, `codesign`/`notarytool`, `rpmbuild`, `msiexec`)
are **not** represented here — each is marked `[VERIFY at first live
run]` in the scripts and tracked in the blockers table of
[../README.md](../README.md).

## `linux-lane-20261110.log` — the 8.E3 Linux lane, executed for real

| | |
|---|---|
| Date | 2026-11-10 |
| Host | Linux x86_64 (dpkg arch `amd64`), this build sandbox |
| Command | `scripts/packaging/verify-linux-artifacts.sh` (env from `scripts/packaging/packaging-env.py --shell`) |
| Binaries | `cargo build --release --locked -p lumen-cli -p desktop` from this worktree (branch `w24/c`, commit `44bd866`) |
| appimagetool | 1.9.1 x86_64 (`git version 8c8c91f`), SHA-256 `ed4ce84f…84eb0` — the pin recorded in `.github/workflows/release.yml` |
| type2 runtime | `20251108` x86_64, SHA-256 `2fca8b44…260d` — the same pin |
| Run mode | `APPIMAGE_EXTRACT_AND_RUN=1` (no FUSE on this host) |

What the log proves, end to end (exit 0):

1. **Real .deb** via native `dpkg-deb` from the config-driven payload
   (`stage-linux-payload.sh`); control fields asserted equal to config
   (`Package=lumen`, `Version=0.1.0-dev`, `Architecture=amd64`,
   `Section=video` from packaging.toml).
2. **Real .AppImage** via the pinned appimagetool + pinned runtime
   (`--runtime-file`, no rolling runtime fetch); shared desktop entry.
3. **rpm .spec** generated + structurally validated
   (`rpm_version=0.1.0~dev`: the `-dev` pre-release mapped to RPM's
   tilde). Binary `.rpm` build needs `rpmbuild` — CI lane, honestly
   labeled in the log itself.
4. **Launch tests on the PACKAGED binaries** (extracted from the deb
   with `dpkg-deb -x`, no root):
   - `usr/bin/lumen --version` → `lumen 0.1.0`, exit 0;
   - `usr/bin/lumen decode-bench tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav`
     → exit 0 with a real `BENCH` line (`a_frames=1 a_decoded=1` — the
     open path + decoder actually ran inside the packaged artifact);
   - `usr/bin/lumen-desktop` → exit 86, the documented
     `WEBVIEW_PENDING` skeleton code (packaging.toml
     `launch_test.desktop_pending_exit`);
   - the AppImage itself, `APPIMAGE_EXTRACT_AND_RUN=1`, → exit 86
     (same documented contract, through the AppRun launcher).
5. **Per-artifact-set SBOMs** — `xtask sbom --out` (CycloneDX 1.5,
   121,900 bytes each, byte-identical content: lockfile-derived, as
   designed) for the `.deb`, `.AppImage` and `.spec` sets.
6. **SHA256SUMS.txt** written over all six artifacts (the manifest
   itself excluded — hashed before it exists).

Artifacts produced (kept out of the repo on purpose — the log + this
table are the record):

| Artifact | Size | SHA-256 (prefix) |
|---|---|---|
| `lumen-0.1.0-dev-x86_64-linux.deb` | 2,070,916 B | `b3735fdc84d2…` |
| `lumen-0.1.0-dev-x86_64-linux.AppImage` | 1,276,408 B | `5bd0ff0dd309…` |
| `lumen-0.1.0-dev-x86_64-linux.spec` | 1,038 B | `17bd1068c818…` |
| 3 × `*.sbom.cdx.json` | 121,900 B each | `f0b22c1bc2a8…` (identical, by design) |

Full digests are in the log (step 7).

### Honest notes

- The lane was first attempted with the `aarch64` appimagetool binary
  that ships in this sandbox — the host is actually **x86_64** (`uname
  -m`), so that binary cannot execute here; the pinned x86_64 pair was
  downloaded and SHA-256-verified instead (same 1.9.1 release the
  workflow pins). No aarch64 AppImage was therefore built locally; the
  aarch64 lane runs in CI (`ubuntu-24.04-arm`, best-effort job).
- `appimagetool` warns about missing AppStream upstream metadata
  (`usr/share/metainfo/*.appdata.xml`) — a real gap, recorded as a
  follow-up in ../README.md, not silenced.
- The `.deb` payload ships the placeholder brand icon (accent-colored
  deterministic PNG) because `assets/brand/` does not exist yet; the
  stager prefers the real asset the day it lands (product.toml
  `[brand] icon_png_512`), no script change needed.

## Gate run (config + scripts + local-provable halves)

`scripts/packaging/gate.sh` (the body of full_gate step 11) passes all
six steps on this host; the run that matters is the one executed by
`scripts/full_gate.sh` step 11 (see the repository gate logs).
