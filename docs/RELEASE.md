# RELEASE.md — Lumen release process (card 7.w22g, issue #56 release half)

How a Lumen release is cut, what the pipeline does, and — honestly —
what it does not do. This file is the template authority for
`scripts/release/release-notes.sh` (edit the template here, then mirror
the prose there).

## What ships

Two binaries, packaged per OS (identity from `product.toml`, never
hard-coded — see "Config-driven identity" below):

| OS | Job | Artifacts |
|----|-----|-----------|
| Windows x64 | `windows` | `<slug>-<ver>-x86_64-windows.zip`, `.msi` (WiX v5) |
| macOS arm64 | `macos-arm64` | `<slug>-<ver>-aarch64-macos.dmg`, `.zip` (of the `.app`) |
| macOS x64 | `macos-x64` | `<slug>-<ver>-x86_64-macos.dmg`, `.zip` |
| Linux x64 | `linux-x64` | `<slug>-<ver>-x86_64-linux.tar.gz`, `.deb`, `.AppImage` |
| Linux arm64 | `linux-arm64` (best-effort) | `<slug>-<ver>-aarch64-linux.tar.gz`, `.deb`, `.AppImage` |
| all | `release` | `SHA256SUMS.txt`, generated notes, `gh release create` |

Every archive/installer contains **both** binaries — `lumen` (the CLI:
the real engine surface) and `lumen-desktop` (the ADR-004 std-only
composition-root skeleton) — plus `LICENSE` and `README.md`.

## Cutting a release (Lead-only: tags and releases are Lead actions)

1. **Version**: set `identity.display_version` (and bump
   `identity.version_code`) in `product.toml`. That file is the single
   source of truth — the workflow and every packaging script read name,
   version, publisher, description, per-OS identifiers and the MSI
   UpgradeCode from there (`scripts/release/product-env.py`). Nothing
   else needs a version literal.
2. **Green gates**: local `scripts/full_gate.sh` green (CI may be
   billing-blocked — see known limits), `Cargo.lock` committed fresh
   (the workflow builds `--locked`).
3. **Tag**: `git tag -a vX.Y.Z && git push origin vX.Y.Z` — tag format
   `v` + numeric semver. The release workflow triggers on `v*` tags
   only. `workflow_dispatch` runs are dry-runs: artifacts are built,
   checksummed and uploaded as workflow artifacts, but **no GitHub
   release is created** (the `gh release create` step is tag-ref-gated).
4. **Watch the run**: the `release` job fails loudly if any blocking
   build job failed; arm64 is `continue-on-error` (see below) and its
   absence shows as a `::warning::` in the artifact-set check, not a
   failure.
5. **Post-release**: sanity-install one artifact per OS from the
   release page, verify `sha256sum --check SHA256SUMS.txt`, and run the
   CLI once (`lumen --help`).

### RC checklist

The full executable version — per-row commands, recorded results,
blocked rows with owners — is `docs/release-checklist.md` (card 8.G2,
issue #58); `scripts/rc-smoke.sh` is its local smoke harness. This
section is the short form:

- [ ] `product.toml` version bumped (+ the two crate versions — the
      binary `--version` reads `apps/cli/Cargo.toml`, not product.toml;
      see release-checklist §0) and SBOM regenerated
      (`cargo run -p xtask --locked -- sbom`).
- [ ] `cargo run -p xtask --locked -- config-validator`: currently
      RED on `main` — 4 violations, all the missing `assets/brand/*`
      files (known-limit 5 below). Land the assets or record the
      waiver before tagging. (Note: `scripts/full_gate.sh` does not
      run this validator — do not rely on the gate for it.)
- [ ] Tag name matches `identity.display_version` (the release job
      cross-checks and warns on mismatch — it is a warning, not a gate:
      pre-release suffixes like `-dev` do not survive MSI
      sanitization, so `v0.1.0` ↔ `0.1.0-dev` is the expected pair).
- [ ] `Cargo.lock` fresh; build is `--locked`.
- [ ] Local gate green; `scripts/rc-smoke.sh` all 8 checks PASS on the
      RC commit; no red lanes in the wave ledger.
- [ ] After the run: checksum file spot-checked, one artifact per OS
      installed and executed.

## What the workflow does (by construction)

`/.github/workflows/release.yml`, validated with actionlint 1.7.12,
`bash -n`/`sh -n` on every script and a YAML parse — see known limits
for why there is no live-run evidence:

1. **Build**: every OS job checks out (SHA-pinned `actions/checkout`),
   installs the pinned toolchain (dtolnay action @ the 1.99.1 branch,
   with the ci.yml-style three-way assert against
   `rust-toolchain.toml`), restores the cargo cache, then
   `cargo build --release --locked -p lumen-cli -p desktop`.
2. **Config injection**: `scripts/release/product-env.py` parses
   `product.toml` + the default locale catalog + `cargo metadata` and
   writes `PRODUCT_*`, `MSI_VERSION`, `BIN_CLI`, `BIN_DESKTOP` into the
   job env. The workflow YAML contains **no product-name or version
   literals** (the only sanctioned exceptions are the two Cargo package
   ids `lumen-cli`/`desktop` needed to address the build).
3. **Package** (all POSIX sh under `scripts/release/`):
   - Windows: `package-windows.sh` — zip via stdlib `python -m zipfile`
     (same interpreter story on all three images), MSI via WiX v5.0.2
     (`wix.exe` extracted from the SHA-256-verified
     `wix.5.0.2.nupkg` pinned at the nuget.org flat-container URL) with
     a `.wxs` generated by `gen-wxs.sh` from the product env (name,
     manufacturer, sanitized numeric version, frozen `UpgradeCode` from
     `product.toml` → upgrades replace, never co-install).
   - macOS: `make-app.sh` builds the `.app` (Info.plist fields
     config-driven), `make-dmg.sh` packs it with native `hdiutil`, the
     workflow zips the bundle with native `ditto`.
   - Linux: tar.gz staged by the workflow, `.deb` by `package-deb.sh`
     (native `dpkg-deb`, `DEBIAN/control` fields config-driven, no
     maintainer scripts — the package ships files only), `.AppImage` by
     `make-appimage.sh` (pinned `appimagetool` 1.9.1 + pinned type2
     runtime 20251108, `--runtime-file` so **no rolling runtime is
     fetched at build time**).
4. **Release job**: downloads all `dist-*` artifacts (SHA-pinned
   `actions/download-artifact`), writes `SHA256SUMS.txt`, generates
   notes with `release-notes.sh` (git log since the previous tag, the
   template below), then `gh release create` with the preinstalled `gh`
   CLI + `GITHUB_TOKEN` (no third-party release action). Tag refs
   only; dispatch runs print a notes preview to the step summary.

## Release notes template

`release-notes.sh` emits, in order: `# <name> <version>`, the release
tag, `## Changes since <previous tag>` (bullet-per-commit, `git log
--oneline` range; first tagged release lists the last 50 commits),
`## Install` (points at the README Install section), `## Checksums`
(`sha256sum --check SHA256SUMS.txt`), `## Known limits (honest)` (the
three bullets below), and a generation footer pointing here.

## Known limits (honest register)

1. **Binaries are unsigned.** No Windows code-signing certificate, no
   Apple Developer account, no notarization — external procurement
   blockers tracked in issue #56. Consequences users see: Windows
   SmartScreen warnings; macOS Gatekeeper requires right-click → Open
   (or `xattr -dr com.apple.quarantine`). The macOS arm64 binary
   carries only the linker's default ad-hoc signature, which is not a
   Developer ID.
2. **The workflow has never executed.** CI billing is blocked on this
   private repository (every Actions run dies at 0 steps; worklog
   7.CLOSEOUT). Correctness is by construction: ci.yml-pattern mirroring
   (SHA-pinned actions, toolchain assert, SHA-verified tool downloads),
   actionlint 1.7.12 clean, `bash -n`/`sh -n` on all scripts, YAML
   parse, and **real local dry-runs on Linux x86_64**: a genuine
   `dpkg-deb`-built `.deb` (validated with `dpkg-deb --info/--contents`
   + extraction) and a genuine `appimagetool`-built `.AppImage` that
   was executed end-to-end. Windows/macOS packaging paths are
   [VERIFY] at first live run — start with a `workflow_dispatch`
   dry-run when billing is unblocked.
3. **Linux arm64 is best-effort.** `ubuntu-24.04-arm` hosted-runner
   availability for private repos has varied; the job is
   `continue-on-error` and the release ships without arm64 artifacts
   rather than blocking. Do not "fix" a red-but-ignored arm64 job
   without first separating infra failure from real failure.
4. **Runner-label deviation**: the release-engineering card suggested
   `macos-13` for the x64 job, but actionlint 1.7.12's label database
   no longer knows that label; `macos-15-intel` is used [VERIFY at
   first live run].
5. **Placeholder icons**: `assets/brand/icon.*` referenced by
   `product.toml` do not exist yet, so Linux packaging ships a
   deterministic 512×512 PNG filled with `brand.accent_color`
   (`scripts/release/gen-icon.py`). No borrowed third-party artwork
   (clean-room rule). The MSI ships no Start-Menu shortcut — the
   desktop binary is a skeleton that opens no window.
6. **MSI version sanitization**: `identity.display_version` pre-release
   suffixes (`-dev`) are stripped for the MSI (WiX rejects non-numeric
   `ProductVersion`); the zip/deb/dmg/AppImage names keep the full
   string. `wix.exe` targets .NET 6 with `rollForward=Major`, satisfied
   by the .NET 8/9 runners on `windows-latest` [VERIFY].
7. **Budget**: the pipeline runs only on `v*` tags (never per-PR) —
   ~5 jobs × 15–25 raw minutes at private-repo multipliers
   (Windows 2×, macOS 10×) per release.

## First real run (v0.1.0, 2026-11-13) — the record

The pipeline's first live tag run, with every [VERIFY] cleared against
real runner behavior:

- **Release**: https://github.com/codzxe/rust-vlc/releases/tag/v0.1.0 —
  published 17:00:58Z from run 43466814633 (tag `v0.1.0` @ `a64b7030`).
- **Iterations (2)**: the first tag run (43464162696) died on the LAST
  step — `gh release create` HTTP 422 "body is too long (maximum is
  125000 characters)"; the notes carry the full git-log range since the
  previous tag (`backup-2026-10-20..v0.1.0`, three weeks of history).
  Fix: the gh step bounds the body at 120,000 characters with an
  honest truncation footer (release.yml, round 18); tag re-cut on the
  fixed commit (no draft was left behind — releases API verified).
- **Asset inventory (40)**: per-OS/arch — Linux x64+arm64
  (tar.gz/deb/rpm/AppImage + per-set SHA256SUMS + .spec), macOS x64
  +arm64 (dmg/zip), Windows x64 (zip/msi + .wxs/.wixpdb audit source);
  per-artifact SBOMs (`.sbom.cdx.json`, CycloneDX 1.5); the global
  `SHA256SUMS.txt`. Both `lumen` and `lumen-desktop` binaries in every
  package. `gh` preinstalled: VERIFIED (the tool ran; the payload was
  the first failure).
- **MSI upgrade-path test**: green twice (dry-run 43461062527 + tag
  run 43466814633) — install → REPLACE (new ProductCode
  `FF3F9614…` → `30E6F8AD…`, replaced not co-installed) → uninstall
  clean (0 codes, dir removed). The canonical squished-GUID probe
  (`25a6f1c7e3d9a8b4…`) found the registration first try after rounds
  10–11 chased byte-order-reversal variants that never match a key
  Windows writes.
- **Signing**: honestly labeled-unsigned — `*.UNSIGNED.txt` sidecars
  ship beside the dmg/msi; the signing-credentials step warned (the
  eight configured names are all absent; blockers table owners: user).
- **Shipped-open items (the honest list)**:
  - ADR-010 (codec patent / redistribution posture) is still
    `proposed` — its deciders are user + counsel and its hard trigger
    is "before any public binary distribution". This release shipped
    publicly on the user's explicit standing instruction (issue #66);
    the ADR-010 sign-off remains owed and is recorded as such.
  - `assets/brand/*` still placeholder-driven (known-limit 5 above);
    the Linux/MSI packages ship the deterministic accent-color icon.
  - macOS/Windows launch verdicts stay BEST-EFFORT-PENDING per
    ADR-004's own rule (a runner session is not a real desktop);
    the human/GUI confirmation half is owed on real hardware.
