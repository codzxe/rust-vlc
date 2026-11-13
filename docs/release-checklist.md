# Release checklist — card 8.G2 (issue #58)

The executable end-to-end checklist for cutting a release candidate.
Every row names the exact command and/or artifact; rows marked
**[verified locally]** were run on this branch with recorded output
(`docs/audit/docs-sweep-20261110.md` §7 has the outputs); rows marked
**[blocked: …]** carry the blocker, its owner, and the next step
(MP §17 blocked definition — never silently descoped).

Honest top line (2026-11-10, branch `w24/e`, base `c339ee8`): the local
half of this checklist is executable and its green rows are recorded;
the CI half (build matrix, packaging on 3 OS, GitHub Release) has
**never executed** — CI billing is blocked on this private repository
(worklog 7.CLOSEOUT; RELEASE.md known-limit 2) — and two web-UI defects
on `main` (F-1/F-2, lane A's fixes unmerged) keep the web smoke red
until the wave-24 merge. This file is the RC-day runbook: execute it in
order, top to bottom.

## 0. Version stamping (the ACTUAL mechanism, verified — not the assumed one)

`product.toml` `[identity]` is the single source of truth for the
**product** version — but the shipped **binary**'s `--version` does not
read it. Two tracks, both verified on this tree:

| Surface | Reads from | Mechanism (verified) |
|---|---|---|
| Artifact + installer names, installer metadata, release-notes header, SBOM subject | `product.toml` → `display_version` (`0.1.0-dev`) + `version_code` (1) | `scripts/release/product-env.py` parses product.toml (+ default locale + `cargo metadata` for bin names) and exports `PRODUCT_*`, `MSI_VERSION`, `BIN_CLI`, `BIN_DESKTOP`; the workflow and every `scripts/release/*.sh` read those, never literals. **[verified locally]** `product-env.py --print` → `PRODUCT_VERSION=0.1.0-dev`, `PRODUCT_VERSION_CODE=1`, `MSI_VERSION=0.1.0`, `BIN_CLI=lumen`, `BIN_DESKTOP=lumen-desktop`, exit 0 |
| MSI `ProductVersion` | `display_version`, sanitized | Pre-release suffixes (`-dev`) are stripped (`MSI_VERSION=0.1.0` above) because WiX rejects non-numeric versions (RELEASE.md known-limit 6) |
| SBOM subject | `product.toml` `[identity]` | `xtask sbom` reads name/display_version/publisher as the BOM subject — drift-gated. **[verified locally]** `sbom --check` → no drift (157 components, subject `Lumen 0.1.0-dev`) |
| `lumen --version` | `apps/cli/Cargo.toml` `version = "0.1.0"` (`env!("CARGO_PKG_VERSION")`) | **NOT wired to product.toml** — recorded as `[PROPOSAL]` in `apps/cli/src/help.rs` (identity wiring pending). The two versions can drift; today they do (`0.1.0` vs `0.1.0-dev`) |

**Procedure (both tracks, one commit):**

1. Set `identity.display_version` and bump `identity.version_code` in
   `product.toml`.
2. Set the same version (semver numeric) in `apps/cli/Cargo.toml` and
   `apps/desktop/Cargo.toml` (`version = "…"`) so the binaries'
   `--version` matches the artifacts they are shipped in.
3. Regenerate the SBOM: `cargo run -p xtask --locked -- sbom` and commit
   `docs/security/sbom/cyclonedx.json` (the drift gate fails otherwise).
4. Tag = `v` + numeric semver of the release (`v0.1.0` for
   `0.1.0-dev` — the pre-release suffix does not survive MSI
   sanitization; the release job's tag↔version cross-check is a
   **warning**, not a gate, by design — RELEASE.md RC checklist).

## 1. Pre-flight (local, before any tag)

| # | Row | Command / evidence | Status |
|---|---|---|---|
| 1.1 | Tree clean, on the release commit | `git status` → clean; note the SHA in the RC report | [verified locally] |
| 1.2 | Syntax: all release scripts | `for f in scripts/rc-smoke.sh scripts/release/*.sh; do bash -n "$f" \|\| exit 1; done` — a loop is **required** (the `\|\|` is GFM-escaped: GitHub splits table cells on raw pipes even inside code spans — W24-R7 finding NEW-1): `bash -n` syntax-checks only its first file argument, the rest become positional parameters and are silently unchecked (W24-R2 finding P2-3); rc-smoke.sh + the 7 `scripts/release/*.sh` = 8 sh. `node --check scripts/rc-smoke-web.mjs`; `python3 -m py_compile scripts/release/product-env.py scripts/release/gen-icon.py` | **[verified locally]** all OK (8 sh + 1 mjs + 2 py — loop re-run post-review, per-script output in `docs/audit/docs-sweep-20261110.md` §7) |
| 1.3 | Workflow YAML parses | `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"` | **[verified locally]** jobs: windows, macos-arm64, macos-x64, linux-x64, linux-arm64, release |
| 1.4 | Config-driven identity gates | `cargo run -p xtask --locked -- lint-literals` (0 violations, 452 allowlisted) and `check-identity` (0 violations, 453 allowlisted) | **[verified locally]** both PASS |
| 1.5 | SBOM in sync | `cargo run -p xtask --locked -- sbom --check` | **[verified locally]** no drift (157 components) |
| 1.6 | product.toml validity | `cargo run -p xtask --locked -- config-validator` | **RED today — 4 violations, all the missing `assets/brand/*` files** (icon.ico, icon.icns, icon-512.png, logo.svg; RELEASE.md known-limit 5: packaging ships the deterministic generated PNG instead). **Before a public release**: either land real brand assets or record the waived state in the release notes' known-limits section. Note: `scripts/full_gate.sh` does not run this validator (its 9 steps are fmt/clippy/test/check-architecture/fixtures/fuzz-registry/lint-literals/check-identity/sbom) — do not rely on the gate to catch it. |
| 1.7 | Full local gate | `scripts/full_gate.sh` (steps 1–9 above) | Run at RC time. **[blocked: disk]** the sandbox was at 100% disk during this checklist's authoring — the gate's own logs from the wave-23/24 lanes are the standing evidence; re-run when the RC commit exists. |
| 1.8 | `Cargo.lock` fresh | `git diff --stat -- Cargo.lock` empty after `cargo build --locked` (workflow builds `--locked`) | check at RC time |

## 2. RC smoke (local — `scripts/rc-smoke.sh`, card 8.G2's harness)

One command, exit 0 = all eight checks pass:

```sh
source /home/z/my-project/scripts/cargo-env.sh   # sandbox only: shared CARGO_TARGET_DIR
scripts/rc-smoke.sh                               # add RC_SMOKE_SKIP_WEB=1 for engine-only
```

| Check | What it proves | Last recorded result |
|---|---|---|
| 1 engine-build | `cargo build --release --locked -p lumen-cli -p desktop` — both shipped binaries | green (commit `ff001e5` run) |
| 2 engine-version | `lumen --version` prints `lumen <semver>` | green |
| 3 engine-info | `lumen engine-info` self-describes (version + decoders) | green |
| 4 headless-decode | `decode-bench` decodes every frame of the default fixture (`tests/fixtures/generated/valid/avi_mjpeg_pcm.avi`) | green |
| 5 command-layer-play | `lumen open` reaches Playing → Stopped through the catalog | green |
| 6 wasm-web-build | `lumen-web` builds for wasm32; `\0asm` magic; > 1 MiB | green |
| 7 web-ui-build | `npm run build` in `ui/` (tsc + vite) | **red on `main`** — F-1 (`ui/src/App.tsx` committed merge markers) + F-2 (stale generated settings screen); both fixed on lane A's `w24/a` (commits `70ed69c`, `771259c`; **unmerged at authoring — `origin/w24/a` not pushed**). rc-smoke's own failure note prints the F-1/F-2 pointers |
| 8 web-ui-browser-load | Playwright chromium loads the built app, boots to the player surface, zero page errors | blocked-by-7 (harness treats it as FAIL — an RC with a red web build is not shippable) |

**RC-day rule:** checks 7–8 must be green on the RC commit. They cannot
be green until the wave-24 merge lands lane A's fixes; the re-run is a
blocking row in §6. A skipped-web run (`RC_SMOKE_SKIP_WEB=1`) is an
engine-only smoke and the script says so — it is **not** a passing RC
smoke.

Chromium discovery (sandbox/CI drift): rc-smoke never downloads a
browser; it takes `RC_SMOKE_CHROMIUM` or the newest
`$PLAYWRIGHT_BROWSERS_PATH` chromium — the documented pin-mismatch
workaround (`docs/guides/troubleshooting.md`).

## 3. Build matrix + per-OS packaging

### 3.1 The tag-triggered pipeline (`origin/main` state, validated by construction)

`.github/workflows/release.yml` — actionlint 1.7.12 clean, `bash -n`/
`sh -n` on every script, YAML parse **[verified locally]** — but
**never executed** (CI billing). Jobs and artifacts (full detail:
`docs/RELEASE.md` "What ships"):

| Job | Runner | Artifacts (names from `product-env.py`: `lumen-<version>-<arch>-<os>.<ext>`) | Packaging drivers |
|---|---|---|---|
| windows | windows-latest | `.zip` + `.msi` (WiX v5.0.2, SHA-verified nupkg; `UpgradeCode` frozen from product.toml → upgrades replace, never co-install) | `scripts/release/package-windows.sh` + `gen-wxs.sh` |
| macos-arm64 | macos-latest | `.dmg` + `.zip` (hdiutil/ditto) | `scripts/release/make-app.sh` + `make-dmg.sh` |
| macos-x64 | macos-15-intel | same, x86_64 | same |
| linux-x64 | ubuntu-latest | `.tar.gz` + `.deb` + `.AppImage` (dpkg-deb; appimagetool 1.9.1 + pinned type2 runtime — no rolling download) | `scripts/release/package-deb.sh` + `make-appimage.sh` |
| linux-arm64 | ubuntu-24.04-arm (**continue-on-error**) | same, aarch64 | same |
| release | ubuntu-latest, needs all | `SHA256SUMS.txt` + notes via `release-notes.sh` + `gh release create` (tag refs only; dispatch = dry-run) | `scripts/release/release-notes.sh` |

**Every archive/installer contains both binaries** — `lumen` (CLI, the
real engine surface) and `lumen-desktop` (ADR-004 std-only composition
root) — plus LICENSE and README.md.

### 3.2 Lane C's wave-24 packaging drivers (cards 8.E1–8.E3, issue #56)

Wave 24 lane C built the per-OS driver set on branch `w24/c`
(`packaging.toml` + schema, `scripts/packaging/*`: build-deb /
build-appimage / build-rpm / build-msi(.sh/.ps1) / build-dmg /
build-targz / build-zip, sign-windows / sign-macos, stage-linux-payload,
verify-linux-artifacts, test-msi-upgrade.ps1, gate wiring; evidence in
`docs/packaging/evidence/linux-lane-20261110.log` — real .deb + real
executable AppImage, packaged-binary launch tests, SBOMs, checksum
manifest). ~~**`origin/w24/c` was not pushed at authoring time**~~ —
**since pushed**: `git ls-remote origin w24/c` → `fb97822`, the
evidence log and every named `scripts/packaging/*` driver verified on
the remote branch (strike per §8, 2026-11-10 post-review, W24-R2
P3-8; the branch remains **unmerged into `main`**) → these rows still
**execute at wave merge**:

| Row (at wave merge) | Command (post-merge paths) |
|---|---|
| Linux lane (proven on w24/c) | `scripts/packaging/build-deb.sh`, `build-appimage.sh`, `build-rpm.sh`; verify: `scripts/packaging/verify-linux-artifacts.sh` |
| Archive lanes (tar.gz + zip) | `scripts/packaging/build-targz.sh`, `build-zip.sh` |
| Windows MSI + upgrade path | `scripts/packaging/build-msi.sh` → `build-msi.ps1`; upgrade test: `test-msi-upgrade.ps1` (PowerShell — Windows machine required) |
| macOS dmg (+ signing if R-35 unblocks) | `scripts/packaging/build-dmg.sh`; `sign-macos.sh` **[blocked: Apple Developer account R-35, user]** |
| Windows signing | `scripts/packaging/sign-windows.sh` **[blocked: code-signing certificate, user — escalate per card 8.E1]** |

### 3.3 Launch / install / uninstall tests (per OS — the RC's user-visible proof)

| OS | Install + launch | Uninstall | Status |
|---|---|---|---|
| Linux (local lane proven) | `dpkg -i lumen-*.deb && lumen --version` (in-container launch tests recorded on w24/c's evidence log) | `dpkg -r lumen` (or apt remove) | proven on w24/c; re-run post-merge |
| Windows | install `.msi`, launch `lumen --help` from PATH; upgrade: install older → newer, assert replace (UpgradeCode) | `msiexec /x` (or Settings→Apps) | **[blocked: Windows machine, user]** |
| macOS | mount `.dmg`, copy `.app`, launch; Gatekeeper: right-click → Open (unsigned — known-limit 1) | delete `.app` | **[blocked: macOS machine + Apple account R-35, user]** |
| Web | serve `apps/web` build (or the packaged static bundle), open in chromium — rc-smoke checks 7–8 are the automated form | n/a (no install) | red on main (F-1/F-2) → wave-merge re-run |

### 3.4 Update-URL wiring (card text) — the honest reality

**No update mechanism exists in v0.1.0 and none is claimed.** Verified:
`apps/desktop-tauri/tauri.conf.json` contains no updater configuration
(no `updater`/`updateUrl` keys; the tauri shell itself is INFRA-BLOCKED,
ADR-004); the web `app_id` in product.toml is PWA identity only; there
is no appcast/feed URL anywhere in the tree. What exists and is wired:
the MSI **UpgradeCode** (frozen in `product.toml`) makes a newer MSI
replace an older install rather than co-install — that plus the release
page is the whole upgrade story. In-app update checks are future work
(they would read a URL from product.toml per MP §5 when they exist).
This row is intentionally **not** green: the card's "update-URL wiring"
reduces today to "no dead update surface ships" — which is true.

## 4. SBOM + checksums

| Row | Command / artifact | Status |
|---|---|---|
| 4.1 Rust-graph SBOM (CycloneDX 1.5, std-only generator) | `cargo run -p xtask --locked -- sbom` → commit `docs/security/sbom/cyclonedx.json`; drift gate `sbom --check` in CI (cargo-deny job) + `full_gate.sh` step 9 | **[verified locally]** no drift; regenerate on every version bump (§0.3) |
| 4.2 Checksum manifest | the `release` job writes `SHA256SUMS.txt` over every artifact (SHA-pinned `actions/download-artifact` aggregation) | CI-side; verify post-run (§6.4) |
| 4.3 Per-platform SBOM finalization + cargo-auditable embedding | card 8.E3 scope (lane C; SBOMs attached to artifacts in their evidence log) | execute at wave merge |
| 4.4 User verification instructions | `sha256sum --check SHA256SUMS.txt` (Linux/git-bash) / `shasum -a 256 --check` (macOS) — already in README Install + release-notes template | in place |

## 5. Release notes

- Generator: `scripts/release/release-notes.sh` — product name+version
  from `product_*` env (product-env.py), git-log-derived changes,
  Install/Checksums/Known-limits sections; template authority is
  `docs/RELEASE.md` (edit there, mirror here).
- **[verified locally]** dispatch-mode dry run on this branch:
  `PRODUCT_NAME=Lumen PRODUCT_VERSION=0.1.0-dev sh scripts/release/release-notes.sh /tmp/rc-notes-preview.md`
  → wrote the notes (20-commit range, exit 0).
- The curated first-release skeleton — the honest scope statement with
  known-limits references — is committed at
  `docs/release-notes/skeleton-v0.1.0.md` (authored with this
  checklist, issue #58). Use it as the human-edited base; the script's
  commit list appends under "Changes".

## 6. The RC sequence (execute top to bottom on RC day)

1. **Wave-24 merge complete** — specifically lane A's F-1/F-2 fixes
   (`w24/a` `70ed69c` + `771259c`) and lane C's packaging drivers
   (`w24/c`) are on the release branch. Blocking.
2. **§0 version stamping** — product.toml + both crate versions + SBOM
   regenerated, one commit.
3. **§1 pre-flight** — all green (or the recorded-waiver state for 1.6,
   written into the release notes).
4. **§2 rc-smoke full run** — all 8 checks PASS on the RC commit
   (record the output in the RC report; the harness exit code is the
   gate). **[blocked until step 1]**.
5. **Tag + push**: `git tag -a v0.1.0 && git push origin v0.1.0`
   (Lead-only: tags and releases are Lead actions — RELEASE.md §"Cutting
   a release"). Tag format `v` + numeric semver (§0.4).
6. **Watch the run**: release.yml fails loudly if any blocking job
   failed; arm64 is continue-on-error (absence = warning, not failure).
   **[blocked: CI billing — user]**. First live run should be a
   `workflow_dispatch` dry-run (artifacts + checksums, no release).
7. **Post-release sanity**: download one artifact per OS from the
   release page; `sha256sum --check SHA256SUMS.txt`; install + launch +
   uninstall per §3.3; run `lumen --help` and `lumen --version` — the
   printed version must equal the artifact version (§0's two-track
   check, now human-verified end-to-end).
8. **Record**: RC report (this file's rows with results), worklog
   entry, and — for the first release — close the README Install
   section's "no release published yet" paragraph by listing the actual
   artifacts.

## 7. Blocked rows + owners (summary — MP §17 format)

| Blocker | Rows affected | Owner | Next step |
|---|---|---|---|
| CI billing (private repo; every Actions run dies at 0 steps) | §3.1 pipeline execution, §6.5–6.6 tag run, §4.2 checksum manifest generation | user | unblock billing → `workflow_dispatch` dry-run first |
| 3-OS runner machines | §3.3 Windows/macOS install/launch/uninstall, MSI upgrade test, matrix evidence | user | user-owned machines (doc 10 §10.4); Linux lane already proven locally |
| Apple Developer account (R-35) | macOS codesign + notarization | user | procurement; then `scripts/packaging/sign-macos.sh` |
| Windows code-signing certificate | §3.2 signing row | user | escalate per card 8.E1 (`[VERIFY]` — escalate if absent) |
| ADR-010 codec-patent decision (still `proposed`) | **any binary distribution** | user + counsel | sign-off before the first public release (ADR register row; hard trigger = 8.E1–8.E3) |
| Missing brand assets (`assets/brand/*`) | §1.6 config-validator red | Lead/design | land assets or record the waiver (RELEASE.md known-limit 5) |
| Wave-24 merge (lane A fixes unmerged, `origin/w24/a` absent; lane C drivers pushed as `origin/w24/c` @ `fb97822` but unmerged) | §2 checks 7–8, §3.2 rows, §6.1 | Lead | merge wave 24; then re-run rc-smoke (the harness's failure note already points at F-1/F-2) |
| Sandbox disk (100% during authoring) | §1.7 full local gate re-run | this lane, re-run | re-run `scripts/full_gate.sh` on the RC commit |

## 8. Ownership + maintenance

Authored by wave-24 lane E (issue #58, card 8.G2). Files this checklist
owns with it: `scripts/rc-smoke.sh`, `scripts/rc-smoke-web.mjs`,
`docs/release-notes/skeleton-v0.1.0.md`. The process narrative it
compresses is `docs/RELEASE.md` (template authority for the notes);
known-limits cross-references live in `docs/KNOWN-LIMITS.md`. When a
row's reality changes (billing unblocked, R-35 resolved, F-1/F-2
merged), edit the row and strike the blocker — never delete history
silently; the RC report is the record of what was true at each RC.

## 9. RC 1 record (v0.1.0, 2026-11-13) — the first real run

Executed after the §7 blockers moved: billing unblocked 2026-11-12
(also unblocked: the 3-OS hosted runners, the workflow_dispatch dry-run
path, the tag path, the checksum manifest generation — rows below
STRUCK in place per §8's rule; history preserved above).

| Row | Was | Now (evidence) |
|---|---|---|
| §3.1 pipeline execution | never executed (billing) | **EXECUTED**: dry-runs + two tag runs; final green = 43466814633 (@a64b7030) — all 5 OS jobs + release job; the windows upgrade test green twice |
| §4.2 checksum manifest | blocked (billing) | **EXECUTED** — global `SHA256SUMS.txt` + per-set manifests in the published release |
| §6.5 dry-run first | — | done (43461062527, green end to end) before the tag |
| §6.6 tag run | blocked (billing) | **DONE** — v0.1.0 published 17:00:58Z, 40 assets, body bounded at 120k chars (422 fix, round 18) |
| §3.3 install/launch/uninstall | 3-OS runners owed | CI half EXECUTED (MSI upgrade test green ×2; Linux verify+launch harness green in every payload job); the HUMAN post-release sanity (§6.7: download from the release page, `sha256sum --check`, install+launch+uninstall per OS) remains OWED by the user |
| §3.4 update-URL wiring | — | unchanged (no update server yet; row stands) |
| ADR-010 sign-off (§7) | proposed | **STILL PROPOSED** — shipped on the user's explicit instruction (issue #66); sign-off by user + counsel remains owed and is the top open governance item |
| macOS codesign / Windows signing (§7) | credentials absent | still absent — the release ships honestly labeled-unsigned (`*.UNSIGNED.txt` sidecars; the workflow warns and names the owners) |
| §1.6 brand assets (§7) | missing | still missing — deterministic generated icon ships (RELEASE.md known-limit 5); row stands |

Post-release TODO (user): §6.7 human sanity per OS; ADR-010 decision;
brand assets or recorded waiver; signing credentials (blockers table
owners). The RC report above is the record of what was true at RC 1.
