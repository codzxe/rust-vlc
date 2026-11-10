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

1. **Real .tar.gz** flat archive — both binaries + the
   `packaging.toml [shipped]` docs list at the archive root (step 7
   cross-checks the member list against the same config).
2. **Real .deb** via native `dpkg-deb` from the config-driven payload
   (`stage-linux-payload.sh`); control fields asserted equal to config
   (`Package=lumen`, `Version=0.1.0-dev`, `Architecture=amd64`,
   `Section=video` from packaging.toml).
3. **Real .AppImage** via the pinned appimagetool + pinned runtime
   (`--runtime-file`, no rolling runtime fetch); shared desktop entry.
4. **rpm .spec** generated + structurally validated
   (`rpm_version=0.1.0~dev`: the `-dev` pre-release mapped to RPM's
   tilde). Binary `.rpm` build needs `rpmbuild` — CI lane, honestly
   labeled in the log itself.
5. **Launch tests on the PACKAGED binaries** (extracted from the deb
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
6. **Per-artifact-set SBOMs** — `xtask sbom --out` (CycloneDX 1.5,
   121,900 bytes each, byte-identical content: lockfile-derived, as
   designed) for the `.tar.gz`, `.deb`, `.AppImage` and `.spec` sets.
7. **SHA256SUMS.txt** written over all eight artifacts (the manifest
   itself excluded — hashed before it exists).

Artifacts produced (kept out of the repo on purpose — the log + this
table are the record; `/tmp` on the build host is volatile, the logs
are committed). Table regenerated from the committed log's step-9
manifest (W24-H, review W24-R1 P2-3: the r1 table carried two stale
digests from an intermediate run — `.deb`/`.AppImage` — that
contradicted the log it summarizes). The log records the digests and
the SBOM byte sizes, not the four package byte sizes — no size is
claimed for them here:

| Artifact | Size | SHA-256 (prefix) |
|---|---|---|
| `lumen-0.1.0-dev-x86_64-linux.tar.gz` | — | `3b5eae753466…` |
| `lumen-0.1.0-dev-x86_64-linux.deb` | — | `a39dafa88039…` |
| `lumen-0.1.0-dev-x86_64-linux.AppImage` | — | `c0b86e2984b1…` |
| `lumen-0.1.0-dev-x86_64-linux.spec` | — | `17bd1068c818…` |
| 4 × `*.sbom.cdx.json` | 121,900 B each | `f0b22c1bc2a8…` (identical, by design) |

Full digests are in the log (step 9 of the run).

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

## `w24c-r3.log` — the r3 landing battery (task W24-C r3)

Same host, same pinned tools, 2026-11-10. Covers the r3 changes:

1. **verify-linux-artifacts.sh FULL re-run** after the step-8 refactor
   onto `gen-sboms.sh` (the ONE SBOM writer) — fresh artifact set,
   launch tests green, exit 0. Binary provenance noted honestly in the
   log: the shared-target-dir release binaries (this branch's delta vs
   origin/main touches no Rust file).
2. **gen-sboms.sh specVersion drift negative test** — a mismatch
   between `packaging.toml [sbom] spec_version` and what the xtask
   generator emitted is a hard error (exit 1), proven by forcing one.
3. **gate.sh** — full pass (as above).
4. **release.yml validation** — `yaml.safe_load` (6 jobs / 63 steps) +
   `bash -n` over all 41 run blocks (with `${{ }}` expressions
   neutralized, as the runner resolves them before bash).
5. **Release-job dry-run on real artifacts** — the kind-check and
   global-manifest step bodies executed verbatim against the step-1
   artifact set. This dry-run CAUGHT A REAL WAVE-22 BUG: the kind glob
   `dist/*."$kind"` matched nothing (artifact names dash-join the OS
   segment), making the check a silent no-op — fixed to `dist/*-"$kind"`
   in the same change and re-verified in the log.
6. The fresh artifact table (tar.gz 3,054,325 B · deb 2,073,444 B ·
   AppImage 1,276,408 B · spec 1,038 B · 4 × SBOM 121,900 B; full
   digests in the log).
