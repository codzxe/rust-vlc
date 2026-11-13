# w26.m.t4 — release.yml gains the tauri admission build (task W26-M-T4, issue #67)

Static-only proof for the release-lane integration half of
apps/desktop-tauri/INFRA-BLOCKED.md §B.1/§C: the workspace-EXCLUDED
webview crate (`apps/desktop-tauri`, package `lumen-desktop-tauri`, bin
`lumen-tauri`) now builds on EVERY lane of
`.github/workflows/release.yml` that ships binaries.

| | |
|---|---|
| Date | 2026-11-13 |
| Host | Linux x86_64, this build sandbox (static validation ONLY — no cargo, per the lane contract) |
| Base commit | `aebd6ba3` (branch `w26.m/t4-release-tauri`, cut from `main`) |
| Change | `.github/workflows/release.yml` + this evidence file ONLY (files this lane owns) |
| Cargo/npm | NOT run — the workflow change is validated statically (see Proofs); builds happen on Actions |

## What changed (per lane)

Every lane that ships binaries gained the same three-step tauri block
(plus the env-passthrough), inserted AFTER the existing workspace
`cargo build` and BEFORE the packaging steps — so the release binary
`target/release/lumen-tauri` exists by the time any packaging script
stages from the shared `CARGO_TARGET_DIR=target` (workflow env;
overrides the sandbox-path `.cargo/config.toml` setting, per the
existing header note).

1. **Env passthrough** — in the existing
   `Parse product.toml + packaging.toml -> job env` step, the
   `packaging-env.py` invocation gains
   `--tauri-package lumen-desktop-tauri --tauri-manifest apps/desktop-tauri/Cargo.toml`
   (both args, never half — half a pair is a hard error per T1's
   bridge). Comment notes these export `BIN_TAURI` from the SUBCRATE'S
   OWN lockfile because the crate is workspace-EXCLUDED (root
   `Cargo.toml` `exclude`) — root cargo metadata cannot see it, the
   manifest path addresses it directly. Exported to `$GITHUB_ENV`
   together with `BIN_CLI`/`BIN_DESKTOP`, so the packaging scripts
   (T2/T3 payload extension) read it like any other bin name.

2. **`Build the embedded UI (ui/dist for tauri-codegen — must precede
   the tauri build)`** — NEW step:
   `npm --prefix ui ci --no-audit --no-fund && npm --prefix ui run build`
   + a `test -f ui/dist/index.html` guard. ORDER IS LOAD-BEARING:
   `tauri.conf.json` `frontendDist` is `../../ui/dist` and
   tauri-codegen embeds it AT COMPILE TIME, so this step MUST land
   before the cargo step below. Mirrors the proven ci.yml `tauri-check`
   lane's invocation exactly (same flags). `node`/`npm` added to the
   header's preinstalled-tools list [VERIFY at first live run].

3. **`Build the tauri webview app (workspace-excluded subcrate —
   release, locked)`** — NEW step: the §C recipe's exact form
   `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml`.
   NOT a workspace feature; the subcrate's own `Cargo.lock`
   (~468 packages, the tauri closure) pins the build. Honest per-OS
   comments inside the step:
   - **windows**: NO system webview dev packages — the WebView2
     runtime ships with Windows 10/11 (Evergreen) and the loader rides
     the wry/tauri dependency closure [VERIFY at first live run].
   - **macos-arm64 / macos-x64**: NO system packages — WKWebView/AppKit
     are OS-provided frameworks; the build only links them.
   - **linux-x64 / linux-arm64**: webkit2gtk needed BEFORE the build —
     see 4.

4. **Linux lanes only** — NEW step
   `Install webkit2gtk + GTK dev packages (the tauri build deps — ci.yml
   tauri-check's proven set)` placed BEFORE the ui/tauri steps, with the
   EXACT package list the ci.yml `tauri-check` lane installed green
   (INFRA-BLOCKED.md §B.1 dep set):
   `libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libgtk-3-dev
   libsoup-3.0-dev librsvg2-dev libxdo-dev libssl-dev build-essential`,
   plus a fail-loud `pkg-config --exists webkit2gtk-4.1` assertion
   mirroring ci.yml. Both `ubuntu-latest` (x64) and `ubuntu-24.04-arm`
   (arm64) carry the same package names.

Lane-by-lane step lists (new steps in **bold**):

- **windows** (zip + msi):
  Parse env (**+ tauri args**) → Build (renamed: "the two workspace
  binaries") → **Build the embedded UI** → **Build the tauri webview
  app** → Install WiX → Package zip → Package MSI → upgrade test →
  SBOM → upload.
- **macos-arm64** (dmg + app zip):
  Parse env (**+ tauri args**) → Build (renamed) → **Build the embedded
  UI** → **Build the tauri webview app** → macOS lane (build-dmg.sh) →
  ditto zip → SBOM → upload.
- **macos-x64**: identical shape to macos-arm64 (macos-15-intel).
- **linux-x64** (tar.gz + deb + AppImage + rpm):
  Parse env (**+ tauri args**) → Build (renamed) → **Install webkit2gtk
  + GTK dev packages** → **Build the embedded UI** → **Build the tauri
  webview app** → appimagetool install → rpmbuild install →
  verify-linux-artifacts.sh → manifest scoping → SBOM-in-harness →
  upload.
- **linux-arm64** (best-effort): identical shape to linux-x64. The
  job-level `continue-on-error: true` is UNTOUCHED — the tauri steps
  are ordinary steps of that job and ride the job-level flag exactly
  like everything else; NO step-level continue-on-error was added (a
  lane-header comment and a comment inside the tauri build step say
  this out loud).

## The `release` job — checked, deliberately NOT changed

Read the whole collection path: per-lane uploads are `path: dist/**`
into the existing `dist-<os>-<arch>` artifact names; the release job
downloads `pattern: dist-*` with `merge-multiple: true`, checks
artifact KINDS (windows.zip/windows.msi/macos.dmg/macos.zip/
linux.tar.gz/linux.deb/linux.AppImage/linux.rpm), writes the global
checksum manifest over `dist/*`, and `gh release create ... dist/*`.
The tauri binary rides INSIDE the existing installer payloads (zip/
msi/tar.gz/deb/AppImage/dmg/app-zip — the payload lists are T2/T3's
packaging-script/config job), so no new artifact name, no new kind,
no change to the verify/checksum/create steps. A header comment
("W26-M RECONCILIATION") documents this zero-delta decision.

## workflow_dispatch dry-run — verified by reading

The trigger block (`on: push: tags: ['v*']` + `workflow_dispatch`)
and all job steps are event-agnostic EXCEPT the two release-cut steps
(`Cross-check tag vs product.toml version` and `Create the GitHub
release`, both `if: github.ref_type == 'tag'`; the dry-run summary
step handles the non-tag case). The tauri steps live inside the build
lanes with no ref gating, so a manual dispatch builds + uploads the
tauri-enabled installers without cutting a release — automatic.

## Proof 1 — YAML parses

```
$ python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml')); print('yaml ok')"
yaml ok
```

## Proof 2 — structural checks (python over the parsed YAML)

```
job ids: ['windows', 'macos-arm64', 'macos-x64', 'linux-x64', 'linux-arm64', 'release']
windows: 14 steps, 11 named, dupes=[], ids=['toolchain']
macos-arm64: 12 steps, 9 named, dupes=[], ids=['toolchain']
macos-x64: 12 steps, 9 named, dupes=[], ids=['toolchain']
linux-x64: 14 steps, 11 named, dupes=[], ids=['toolchain']
linux-arm64: 14 steps, 11 named, dupes=[], ids=['toolchain']
release: 10 steps, 9 named, dupes=[], ids=[]
needs release: ['windows', 'macos-arm64', 'macos-x64', 'linux-x64', 'linux-arm64'] -> missing=[]
```

- every job id is unique (dict keys), no duplicate step NAME within any
  job, the only step `id:` (`toolchain`) is unique per job, and every
  `needs:` resolves to an existing job (the five build jobs have no
  needs; `release` needs all five — unchanged).
- per-lane tauri step counts: windows/macos×2 = 2 new steps each; the
  two linux lanes = 3 new steps each (webkit install + ui + tauri).
- grep counts over the file: 5 × the exact recipe command
  `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml`,
  5 × `--tauri-package lumen-desktop-tauri`, 5 ×
  `--tauri-manifest apps/desktop-tauri/Cargo.toml`.
- action pins: zero new `uses:` entries — every existing SHA-pinned
  action ref is byte-identical (checkout / rust-toolchain / rust-cache
  / upload-artifact in all lanes, download-artifact in release).

## Cross-lane reconciliation notes for the Lead

1. **Exact arg names wired here** (must match T1's implementation —
   they DO: T1's evidence `w26m-t1-bin-tauri-env.md` proves
   `--tauri-package lumen-desktop-tauri --tauri-manifest
   apps/desktop-tauri/Cargo.toml` exports `BIN_TAURI=lumen-tauri`,
   omit-pair emits nothing, half-pair is a hard error):
   - `--tauri-package lumen-desktop-tauri`
   - `--tauri-manifest apps/desktop-tauri/Cargo.toml`
2. **Merge order dependency (fail-loud by design)**: on `main` as of
   this branch's base, `packaging-env.py` does NOT yet accept the
   `--tauri-*` args (argparse exits 2, "unrecognized arguments"). This
   lane's parse steps therefore hard-fail at zero packaging work until
   T1's `w26.m/t1-bin-tauri-env` branch lands — intentional, honest
   failure (no silent no-op), same posture as every other wiring guard
   in this file.
3. **BIN_TAURI consumers**: T2 (linux payload) / T3 (win/mac payload)
   own the packaging scripts' payload lists + `:?` env assertions.
   This lane only guarantees BIN_TAURI is in `$GITHUB_ENV` and that
   `target/release/lumen-tauri` is built before any packaging step
   runs. If a script needs a DIFFERENT variable name, that is a
   scripts-side change — the workflow wiring above is fixed to the
   names T1 shipped.
4. **No scripts/** touched** by this lane (verified: the only changed
   files in the commit are release.yml + this evidence file). The
   shared sandbox carried uncommitted scripts/** edits from the T2/T3
   lanes in the working tree — left strictly alone.

## Honest notes / limitations

- This workflow has NEVER executed (CI billing blocked — the header's
  standing honesty note). Everything here is static validation; the
  per-OS webkit/npm/cargo behavior carries inline [VERIFY at first
  live run] markers where this lane could not prove it.
- `node`/`npm` are claimed preinstalled on the runner images (they are
  on today's images) — added to the existing preinstalled-tools
  [VERIFY] list rather than adding a setup-node action pin, keeping
  the pin table unchanged.
- No `Swatinem/rust-cache` workspaces retarget was added for the
  subcrate: the shared `CARGO_TARGET_DIR=target` keeps the tauri build
  in the same cache dir; cache-key integration for the second
  lockfile is a possible follow-up if build minutes demand it.
- The runner-budget header estimate is amended honestly: each lane
  also compiles the ~470-package tauri closure + the npm ui build —
  expect the upper end (macOS 10× multiplier bites hardest).
