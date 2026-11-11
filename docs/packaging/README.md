# Lumen packaging — runbook (issue #56, cards 8.E1–8.E3, doc 11 §9.3)

The owner's manual for every release artifact: what ships, where each
decision lives, how each lane is built, how it is verified, and what is
honestly blocked on whom. Companion to
[docs/RELEASE.md](../RELEASE.md) (the wave-22 release process this lane
plugs into) and [evidence/](evidence/) (local proof logs).

## The two config planes

Nothing in `scripts/packaging/**` carries a product or packaging
literal. Every value flows from two TOML files through one bridge:

| Plane | File | Owns | Validated by |
|---|---|---|---|
| Product identity | [`product.toml`](../../product.toml) | name, version, publisher, per-OS identifiers, MSI UpgradeCode, brand accent/icon paths | `product.schema.json` via `cargo xtask config-validator` |
| Packaging decisions | [`packaging/packaging.toml`](../../packaging/packaging.toml) | install dirs, shipped docs, desktop-entry categories, checksum algorithm + manifest name, SBOM suffix + spec version, signing secret env NAMES, upgrade-test gate, launch-test verbs | `packaging/packaging.schema.json` via `scripts/packaging/packaging-env.py` |

The bridge — **`scripts/packaging/packaging-env.py`** — runs
`scripts/release/product-env.py` (identity + cargo-metadata bin names,
the wave-22 tool, re-used not duplicated), overlays the validated
packaging plane, and emits one merged environment
(`PRODUCT_*` + `PACKAGING_*`, 51 variables at the time of writing).
Every packaging script hard-fails on a missing variable (`:?` asserts);
there is no silent fallback anywhere in the lane.

A rebrand (`cargo xtask rebrand`), a version bump, a docs-list change,
or a checksum-algorithm change touches TOML only — no script edits.

One deliberate constraint on the docs list (W24-R6 P4-4): doc paths in
`[shipped]` must be **whitespace-free** (schema pattern,
`packaging.schema.json` shipped.docs). The bridge exports the list as
the space-separated `PACKAGING_DOCS` env var, so a path containing a
space could not be represented — it would word-split into bogus paths
in staging and the `.wxs` File rows. `packaging-env.py` therefore
refuses it at validation time (the first step of every job), with an
error naming the offending config entry — not by a confusing
downstream staging failure. A richer list format is a deliberate
non-goal; rename the file instead.

## Artifact matrix

Naming: `<slug>-<version>-<arch>-<os>.<ext>` (all from config).

**Payload honesty** (W24-R1 P2-4 — the earlier "every artifact
packages BOTH binaries + docs" claim was false for two artifact
families and is replaced by this table):

| Payload | Artifacts |
|---|---|
| BOTH shipped binaries (`lumen` CLI — the real engine surface; `lumen-desktop` — the std-only ADR-004 skeleton that exits 86 `WEBVIEW_PENDING`) **plus** the `packaging.toml [shipped]` docs list | `.tar.gz`, `.zip`, `.deb`, `.rpm`, `.msi` |
| Desktop binary ONLY — the GUI-shell distribution channel (AppRun + desktop entry + icon; no CLI, no docs) | `.AppImage` (`build-appimage.sh`) |
| Desktop binary + LICENSE/README in `Contents/Resources` (no CLI — GUI-shell bundle; the doc pair is hardcoded by the wave-22 `make-app.sh`, flagged as drift vs the `[shipped]` list) | macOS `.app`, `.dmg`, app-`.zip` (`make-app.sh` → `build-dmg.sh`) |

The full matrix:

| OS / arch | Artifact | Builder | Where it runs | Local proof |
|---|---|---|---|---|
| Linux x86_64 + aarch64 | `.tar.gz` flat archive | `build-targz.sh` | CI Linux jobs; any host | **PROVEN** — [evidence](evidence/linux-lane-20261110.log) |
| Linux x86_64 + aarch64 | `.deb` (dpkg-deb) | `build-deb.sh` via `stage-linux-payload.sh` | CI Linux jobs; Debian-family host | **PROVEN** — same log |
| Linux x86_64 + aarch64 | `.AppImage` (pinned appimagetool 1.9.1 + type2 runtime 20251108) | `build-appimage.sh` | CI Linux jobs | **PROVEN** — same log |
| Linux x86_64 + aarch64 | `.rpm` (rpmbuild) | `build-rpm.sh` | CI Linux jobs (`apt-get install rpm`) | spec generated + validated locally; binary build is CI-only |
| Windows x86_64 | `.zip` flat archive | `build-zip.sh` (`python -m zipfile`) | CI Windows job | parse-checked (`bash -n` + `sh -n` + gate); execution is CI-only |
| Windows x86_64 | `.msi` (WiX v5.0.2, pinned nupkg) | `build-msi.sh` → `build-msi.ps1` + `scripts/release/gen-wxs.sh` | CI Windows job | `.wxs` generation proven by BOTH twins' self-tests (`gen-wxs.sh --self-test` + `build-msi.ps1 -SelfTest`: hostile-fixture XML parse, round-trip, raw-text escape-set pins); `wix build` is CI-only |
| macOS aarch64 + x86_64 | `.dmg` (hdiutil UDZO) | `build-dmg.sh` (bundle → codesign → dmg → notarize → staple) | CI macOS jobs | staging steps are pure file ops; codesign/notarytool/hdiutil are CI-only |
| macOS aarch64 + x86_64 | `.zip` of the `.app` (ditto) | inline in `.github/workflows/release.yml` | CI macOS jobs | CI-only |
| every artifact | `.sbom.cdx.json` (CycloneDX 1.5) | `gen-sboms.sh` → `cargo run -p xtask --locked -- sbom` | CI every job; local | **PROVEN** — [r3 evidence](evidence/w24c-r3.log) |
| every release | `SHA256SUMS.txt` global manifest | release job (algorithm + name from config) | CI release job | **dry-run for real** on the local artifact set — [r3 evidence](evidence/w24c-r3.log) |

Plus per-artifact audit sidecars that ride along: the generated `.wxs`
beside each MSI, `<artifact>.UNSIGNED.txt` / `.NOT-NOTARIZED.txt` when
signing credentials are absent (see below), and one per-Linux-set
checksum manifest (`<base>.SHA256SUMS.txt`) alongside the global one.

## Per-platform runbook

### Linux (the sandbox-provable lane — run it yourself)

```sh
# one-time: pinned tools (SHA-256s recorded in release.yml)
curl -sSfL -o appimagetool.AppImage \
  https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage
curl -sSfL -o type2-runtime \
  https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64

# (build-sandbox-only helper — NOT a repo file: pins this sandbox's
# shared CARGO_TARGET_DIR for the parallel agent worktrees. On any
# other host skip this line; a plain build then uses the repo-relative
# target/, which is the correct default everywhere else.)
source /home/z/my-project/scripts/cargo-env.sh
cargo build --release --locked -p lumen-cli -p desktop

eval "$(python3 scripts/packaging/packaging-env.py \
        --cli-package lumen-cli --desktop-package desktop --shell)"
export APPIMAGETOOL="$PWD/appimagetool.AppImage" RUNTIME_FILE="$PWD/type2-runtime"

scripts/packaging/verify-linux-artifacts.sh    # builds + launch-tests + SBOMs + manifest
```

`verify-linux-artifacts.sh` is the WHOLE lane in one honest script:
builds the tar.gz/deb/AppImage (+ binary .rpm when `rpmbuild` exists —
otherwise the validated spec, loudly labeled), then **launch-tests the
packaged binaries** (`--version`, `decode-bench` on a real fixture, the
documented desktop skeleton exit code from
`packaging.toml [launch_test]`), asserts deb/rpm header fields equal
the config, writes per-artifact SBOMs and the per-set checksum
manifest. Exit 0 means every claim above held.

### Windows / macOS (CI lanes — locally provable halves only)

The same env bridge, then:

- Windows: `build-zip.sh` → `build-msi.sh` (generates the `.wxs` from
  product.toml, runs pinned WiX, hands the MSI to `sign-windows.sh`) →
  `test-msi-upgrade.ps1` (config-gated by
  `packaging.toml [windows] upgrade_test`) — installs a predecessor MSI
  built from the same tree + frozen UpgradeCode and proves the current
  one REPLACES it (MajorUpgrade semantics).
- macOS: `build-dmg.sh` — `.app` bundle (wave-22 `make-app.sh`), then
  `sign-macos.sh --app`, then `hdiutil` dmg (`make-dmg.sh`), then
  `sign-macos.sh --dmg` (codesign + notarytool + stapler).

**Universal-binary decision** (card 8.E2 deliverable): we ship TWO
per-arch dmg/app sets (aarch64 + x86_64), not one `lipo`'d universal
`.app`. Rationale: a universal build doubles the build time on the
most expensive (10x-multiplier) runners; every artifact stays
self-contained (own SBOM, own checksum line, own future signature — a
universal `.app` would additionally need every slice signed per
`docs/inventory/platform-macos.md`); the arch segment in the artifact
name already self-selects the right download; and the choice is
reversible purely in `release.yml` (dual `--target` + a lipo step — no
script or config change). Revisit when R-35 resolves and a user
preference for a single universal download is recorded. Full text:
the `macos-arm64` job header comment in
[`.github/workflows/release.yml`](../../.github/workflows/release.yml).

What runs WITHOUT the native platform: env assertions, staging, the
unsigned-fallback sidecars, and (since W24-Y, audit R13 F-U3) BOTH
`.wxs`-generator self-tests — all exercised in
`scripts/packaging/gate.sh` (below; each self-test also runs
standalone): `scripts/release/gen-wxs.sh --self-test` (W24-R6 P4-2;
needs only python3) and `pwsh scripts/packaging/build-msi.ps1
-SelfTest` (added in W24-U; needs pwsh — the gate runs it wherever
pwsh is on PATH and skips it with an honest label where not; this
sandbox has no pwsh, so step 4's structural check stays that twin's
local floor). Both run the REAL generator (`generate_wxs` /
`New-MsiWxs` — the same code path the build invokes) with the same
hostile identity values and hostile doc basenames
(`a&b<c>"d".md` …) and prove with python's XML parser that the
output parses and the values round-trip — plus, since W24-Y
(audit R13 F-U1/F-U2), a raw-text pin asserting the escaped forms
themselves (`Source="it&apos;s.md"` …): a reverted `&apos;` parses
fine, so parse + round-trip alone cannot catch it. What cannot:
`wix build`, `signtool`, `codesign`, `notarytool`, `hdiutil`,
`msiexec` — each is marked `[VERIFY at first live run]` at its call
site.

### The gate

`scripts/packaging/gate.sh` — wired as **step 11 of
`scripts/full_gate.sh`** — runs on every commit and proves the cheap
half: packaging.toml↔schema + product bridge (51 vars), every
`scripts/packaging/*.sh` parses under BOTH `bash -n` and `sh -n`,
python tools parse (ast, no bytecode), `.ps1` structural sanity
(`ps1-sanity.py`), payload stagers + desktop entry executed against a
throwaway tree, the `.UNSIGNED.txt` fallback contracts of both
signing drivers executed for real, and (W24-Y, audit R13 F-U3) step
7: BOTH `.wxs`-generator self-tests — `gen-wxs.sh --self-test`
always, `build-msi.ps1 -SelfTest` wherever pwsh is on PATH (honest
skip label, never a fake PASS, where it is not).

## The release workflow (`.github/workflows/release.yml`)

Tag push `v*` (or manual `workflow_dispatch` dry-run) → six jobs:

1. **windows** (windows-latest): zip + MSI + upgrade test + SBOMs.
2. **macos-arm64** (macos-latest): dmg + app-zip + SBOMs.
3. **macos-x64** (macos-15-intel): same set.
4. **linux-x64** (ubuntu-latest): the FULL `verify-linux-artifacts.sh`
   harness (tar.gz + deb + AppImage + binary rpm + launch tests +
   SBOMs + per-set manifest).
5. **linux-arm64** (ubuntu-24.04-arm, `continue-on-error`): same.
6. **release** (after all): flat-merges every `dist-*` artifact,
   verifies expected kinds, writes the GLOBAL `SHA256SUMS.txt`
   (algorithm + name from `packaging.toml [checksum]`; hashes the
   per-set manifests too, never itself), generates notes, and — on tag
   refs only — `gh release create` with every asset.

Every job: pinned toolchain assertion, `rust-cache`, the
`packaging-env.py` bridge, `cargo build --release --locked`. Tools are
SHA-256-pinned (WiX nupkg, appimagetool, type2 runtime — no floating
installs). Full pin table and runner-budget honesty live in the
workflow header.

**The workflow has never executed** (CI billing blocked — see
blockers). Evidence in lieu: the Linux lane is the SAME script proven
locally; every run-block parses (`bash -n` over all 42 run-blocks,
re-counted in W24-Q — 41 at r3 plus the W24-H secrets-visibility
step); YAML validates (`yaml.safe_load`); the release job's
kind-check + manifest steps were dry-run for real against the local
artifact set ([r3 evidence](evidence/w24c-r3.log)).

## Signing posture — honest, never faked

Both signing lanes are **externally blocked on user-owned credentials**.
The drivers are real (`signtool` / `codesign` + `notarytool` +
`stapler`); with secrets set they sign and any failure is HARD; without
them they write an explicit sidecar and a `::warning::` naming the
blocker, and the release ships **labeled-unsigned + checksums** (doc 11
§9.3 item 6):

| Artifact | Absent credential → | Files |
|---|---|---|
| `.msi` | `<base>.msi.UNSIGNED.txt` (why + how to verify via SHA256SUMS instead) | `sign-windows.sh` |
| `.app` / `.dmg` | `<artifact>.UNSIGNED.txt` (no identity at all) or `<artifact>.NOT-NOTARIZED.txt` (signed, but Gatekeeper needs right-click > Open) | `sign-macos.sh` |

Secret env-var NAMES are config (`packaging.toml [signing.*]`), so no
script carries a secret-name literal either.

## Blockers (owner + next step)

| # | Blocker | Owner | Next step | Tracking |
|---|---|---|---|---|
| 1 | **Apple Developer account absent** — no Developer ID certificate, no App Store Connect API key (risk **R-35**, open, User-decision) | user | provision the account + Developer ID Application cert + ASC API key; set repo secrets named in `packaging.toml [signing.macos]` | risk register R-35; issue #56 |
| 2 | **Authenticode certificate not budgeted** (Windows) | user | purchase the cert; set the two repo secrets named in `packaging.toml [signing.windows]` | phase8-cards §1 row 2; issue #56 |
| 3 | **CI billing blocked** — every Actions run dies at 0 steps | user | lift the spending limit; then run a `workflow_dispatch` dry-run of `release.yml` FIRST (builds + uploads, cuts no release) | issues #53 / #56; worklog 7.CLOSEOUT |
| 4 | AppStream upstream metadata missing (`usr/share/metainfo/*.appdata.xml` — appimagetool warns) | repo (follow-up card) | author appdata from product.toml + locale catalog when a card owns it | noted here; not silenced |
| 5 | arm64 hosted-runner availability for private repos varies | infra | none — job is deliberately `continue-on-error`; do not "fix" a red arm64 without checking infra first | release.yml job comment |

## Evidence

- [linux-lane-20261110.log](evidence/linux-lane-20261110.log) — the
  full 8.E3 lane executed for real (annotated:
  [evidence/README.md](evidence/README.md)).
- [w24c-r3.log](evidence/w24c-r3.log) — r3: `gen-sboms.sh` real run +
  specVersion drift negative test, gate.sh full pass, release.yml
  YAML + run-block validation, release-job dry-run on real artifacts
  (which caught and fixed the wave-22 kind-glob no-op).
