# W24-C packaging lane — review findings (audit W24-R1)

- **Auditor**: W24-R1 (wave 24 review scout 1), read-only audit of `origin/w24/c` (issue #56, cards 8.E1–8.E3).
- **Audited diff**: `git diff origin/main...origin/w24/c` — 28 files: `scripts/packaging/**` (14 .sh + 2 .py + 2 .ps1), `packaging/packaging.toml` + `packaging.schema.json`, `.github/workflows/release.yml`, `docs/packaging/**`, `scripts/full_gate.sh` step 11, `.gitignore`, `worklog.md`.
- **Verdict**: **no P0 (no fakes/stubs/mock evidence found)** — the signing drivers are real, the config plane is real and validated, the local evidence reproduces. But **2 P1 contract violations** (one in code, one in workflow wiring) plus 4 P2 and 4 P3 findings. Fix P1s before merge; P2s before or immediately after.

**Severity counts: P0=0, P1=2, P2=4, P3=4 (total 10).**

---

## P1 — fix before merge

### P1-1. `scripts/packaging/sign-macos.sh:175` — `|| true` demotes every hard-fail in identity resolution to the unsigned fallback (contract violation, misattributed sidecar)

```sh
signing_identity=$(resolve_identity || true)
```

`resolve_identity` runs in a command-substitution subshell; every `die()` inside it (lines 135–139 `codesign not found — needs macOS when a certificate is present`, 138–139 `p12 secret is set but its passphrase secret is empty — refusing to half-sign`, 146 empty base64 decode, 155/157/160/163/166 keychain create/unlock/import/set-partition-list/search-list failures, 171 `no codesigning identity found in the imported p12`) only exits the **subshell**; `|| true` swallows the status, `signing_identity` ends up empty, and the script takes the unsigned path (lines 178–183): **exit 0** plus an `.UNSIGNED.txt` sidecar whose stated reason — "no Apple Developer account is provisioned — risk register R-35" — is **false** in exactly those failure cases (broken passphrase, failed import, bad p12).

This contradicts the script's own header contract (lines 26–28: *"any credential present but a signing/notarization STEP FAILS -> HARD exit. Never silently pretend, never ship half-done"*) and the lane's central "secrets-driven fail-loud" claim. It is the one place in the lane where a real failure is converted into success-with-wrong-evidence.

**Fix**: distinguish "no credentials" (legit return 1) from "credentials present but broken" (must propagate). E.g. have `resolve_identity` `return 2` for the no-credential case and `die` otherwise, then:

```sh
signing_identity=$(resolve_identity) && rc=0 || rc=$?
[ "$rc" -eq 0 ] || [ "$rc" -eq 2 ] || exit 1   # die already printed the reason
```

(or resolve without the substitution masking — e.g. write the identity to a temp file so `die` exits the script directly). The gate's step 6 only proves the credentials-absent path, so this bug is invisible to it; a negative test (p12 set + password empty → must exit 1) would close that gap.

### P1-2. `.github/workflows/release.yml` — zero `secrets.*` mappings: the signing lanes can never see credentials in CI; the documented user next-step is unactionable

Verified: `rg 'secrets\.' .github/workflows/` → **no matches in any workflow**. The signing scripts read env vars *named* by `packaging.toml [signing.*]` (`WINDOWS_SIGNING_PFX_B64`, `WINDOWS_SIGNING_PFX_PASSWORD`, `MACOS_SIGNING_IDENTITY`, `MACOS_SIGNING_CERT_P12_B64`, `MACOS_SIGNING_CERT_PASSWORD`, `APPLE_API_KEY_ID`, `APPLE_API_KEY_B64`, `APPLE_API_ISSUER`) — but GitHub **does not inject repo secrets as env vars automatically**. No step in `release.yml` maps `secrets.<NAME>` → `env.<NAME>` for the windows/macos jobs.

Consequences:
- The runbook's blocker table (`docs/packaging/README.md:181–182`) and both sidecar writers (`sign-windows.sh:87–88`, `sign-macos.sh:94–97`) tell the user to *"set the repository secrets this pipeline reads"* — following that instruction to the letter **changes nothing**: releases stay unsigned forever, silently, exactly the outcome the lane claims to prevent.
- The lane's claim "with the secret env vars set they sign" is true only for manual invocation, not for the release pipeline this lane shipped.

**Fix** (a few lines, no secrets inlined — names come from config at runtime, values from the secrets context): in the `windows` job's MSI step add

```yaml
        env:
          WINDOWS_SIGNING_PFX_B64: ${{ secrets.WINDOWS_SIGNING_PFX_B64 }}
          WINDOWS_SIGNING_PFX_PASSWORD: ${{ secrets.WINDOWS_SIGNING_PFX_PASSWORD }}
```

and equivalently for the two macOS jobs (all six `[signing.macos]` names, on the `build-dmg.sh` step). Absent secrets resolve to empty strings → the existing honest-unsigned path is unchanged. Optional hardening: a release-job `::warning::` listing which signing env names were unset, so "provisioned but not wired" can never recur silently.

---

## P2 — real, bounded

### P2-1. `scripts/packaging/build-msi.ps1:153–156` — .wxs hardcodes the docs list while the config claims every package ships `[shipped]` docs

The embedded WiX template pins `<File Source="LICENSE">` / `<File Source="README.md">`, while the same script (lines 170–174) stages the **config-driven** `PACKAGING_DOCS` list into the build dir — extra docs are silently ignored by `wix build`, removed docs break the build. `packaging/packaging.toml:41–47` says `[shipped] docs` are "Repo-relative files that ride in **EVERY** package". The bash twin inherits the same hardcode from wave-22 `scripts/release/gen-wxs.sh:78–81`, and the macOS `.app` (wave-22 `make-app.sh:51,57`) likewise hardcodes `LICENSE README.md` into `Contents/Resources/`. A future docs-list change in packaging.toml reaches tar.gz/zip/deb/rpm/AppImage but **silently not the MSI** — the exact config-drift class this lane exists to eliminate.
**Fix**: generate the `<Component Id="Docs">` file rows from `$env:PACKAGING_DOCS` in `build-msi.ps1` (and extend `gen-wxs.sh` the same way, or route both through one writer); until then, hard-assert the hardcoded pair equals the configured list.

### P2-2. `release.yml:164` → `test-msi-upgrade.ps1:150` → `build-msi.ps1:43,93` — WIX_EXE is a POSIX path when consumed by PowerShell; the upgrade test breaks at first live run

The WiX install step (bash/git-bash) exports `WIX_EXE=${PWD}/.tools/wix/tools/net6.0/any/wix.exe` — in git-bash `$PWD` is POSIX-form (`/d/a/<repo>/<repo>/...`). The later `pwsh scripts/packaging/test-msi-upgrade.ps1` step calls `build-msi.ps1` without `-WixExe`, so it defaults to `$env:WIX_EXE` (line 43) and `Test-Path` (line 93) resolves a leading `/` against the **current drive** (`D:\d\a\...`), which does not exist → `Fail "WixExe not found"` → the config-gated upgrade-path test (`windows.upgrade_test = true`) fails on the very first live Windows run. Bash consumers (`build-msi.sh:69`) are unaffected — this is purely the bash→pwsh env handoff.
**Fix**: export the Windows form (`WIX_EXE="$(cygpath -w "${PWD}/.tools/wix/tools/net6.0/any/wix.exe")"`), or convert in `build-msi.ps1` when the value starts with `/`.

### P2-3. `docs/packaging/evidence/README.md:60–61` — artifact table digests contradict the log it summarizes

The r1 evidence table lists `.deb` → `b3735fdc84d2…` and `.AppImage` → `5bd0ff0dd309…`, but `linux-lane-20261110.log:92–93` (the same run, per the tar.gz/spec/SBOM rows which **do** match) records `a39dafa88039…` and `c0b86e2984b1…`. Looks like a stale copy from an intermediate run; as committed it undermines the "the logs are the record" contract (a reviewer cross-checking the table against the log gets a mismatch on 2 of 5 rows).
**Fix**: regenerate the table from the committed log (or stamp both from one run).

### P2-4. `docs/packaging/README.md:32–35` — "Every artifact packages BOTH shipped binaries … plus the `[shipped]` docs list" is false for the AppImage and the macOS `.app`

The new AppImage lane installs **only** `$BIN_DESKTOP` into the AppDir (`build-appimage.sh:64–72` — no `BIN_CLI`, no docs; confirmed by the evidence log's squashfs census: 4 files = AppRun + entry + icon + one binary), and the macOS `.app` ships the desktop binary + hardcoded LICENSE/README (`make-app.sh:56–57`). The matrix section that makes the "every artifact" claim lists both rows (the inherited workflow header `release.yml:8–10` says it too). Shipping a desktop-only AppImage/`.app` is a defensible design — but the claim as written is wrong, and it is the lane's own summary document.
**Fix**: scope the claim to the artifacts that honor it (tar.gz/zip/deb/rpm/msi) and state the AppImage/.app payload explicitly; or add the CLI + docs to the AppDir if the "every artifact" contract is the intent.

---

## P3 — nits / hardening

### P3-1. Product-name literals in scratch paths contradict the "ZERO IDENTITY LITERALS" banner
`build-msi.ps1:116` (`"lumen-msi-" + …`) and `test-msi-upgrade.ps1:146` (`"lumen-upgrade-test-" + …`) hardcode the product slug in temp-dir prefixes while both file headers advertise "CONFIG-DRIVEN, ZERO IDENTITY LITERALS". Nothing shipped, nothing breaks on rebrand — but a rebrand sweep won't find them. **Fix**: `"$($env:PRODUCT_LOWER)-msi-"` (env is guaranteed by the assertion block above).

### P3-2. `sign-windows.sh:63–76` — signtool locator probes only the lexically-newest Windows SDK dir
`ls | sort -V | tail -1` picks one directory; if that dir lacks `signtool.exe` (SDK layouts can carry non-version dirs like `x86`/`uap10.0.*` that sort newest), the search gives up without trying older version dirs. The `SIGNTOOL` env override is the escape hatch. **Fix**: iterate all version dirs newest-first until `signtool.exe` is found.

### P3-3. `docs/packaging/README.md:66` — runbook hardcodes the sandbox absolute path
`source /home/z/my-project/scripts/cargo-env.sh` is a sandbox-specific path in a repo-level runbook (the rest of the runbook is repo-relative). **Fix**: repo-relative form or an explicit "this sandbox's convention" placeholder.

### P3-4. macOS `.app`-zip artifact ships without any unsigned sidecar
`sign-macos.sh --app` writes `bundle/<Name>.app.UNSIGNED.txt` (beside the bundle), but the workflow uploads only `dist/**` (`release.yml:262–267`) — the ditto'd `dist/<base>.zip` carries no `.UNSIGNED.txt`, so the app-zip is the one release asset whose unsigned status is not self-describing (the dmg sidecar does ship). **Fix**: copy the app sidecar into `dist/` in the workflow's zip step (e.g. `dist/<base>.zip.UNSIGNED.txt`), or fold the notice into the zip.

---

## What was checked and verified GREEN

| Check | Method | Result |
|---|---|---|
| Workflow YAML validity | `python3 -c "import yaml,sys; yaml.safe_load(...)"` | OK — 6 jobs (`windows`, `macos-arm64`, `macos-x64`, `linux-x64`, `linux-arm64`, `release`), 63 steps, `release.needs` = all five build jobs |
| Run-block syntax | `bash -n` over all 41 `run:` blocks with `${{ }}` neutralized | 41/41 clean (reproduces the r3 evidence claim) |
| Script references | extracted every `scripts/...` path from run blocks, checked existence | all 8 exist |
| Shell syntax | `bash -n` + `sh -n` over all 14 `scripts/packaging/*.sh` | clean (also re-proven by gate step 2) |
| shellcheck | not available in this sandbox | n/a (noted; `bash -n`/`sh -n` + gate ran instead) |
| Packaging gate | executed `bash scripts/packaging/gate.sh` in this worktree | **PASS** (6/6: config+bridge 51 vars, 14 scripts, python ast, ps1 sanity, stagers, unsigned-fallback contracts) |
| Literal lint | `cargo run -p xtask --locked -- lint-literals` on this branch | **PASS — 0 violations, 452 allowlisted** |
| Config-value duplication grep | `rg` for versions, `usr/share`, `SHA256SUMS`, digicert, `Program Files`, product name in `scripts/packaging/**` | only legitimate spec constants (`usr/bin` asserted against config in `build-appimage.sh:58`; macOS/git-bash system paths in the signing drivers); the sanctioned `lumen-cli`/`desktop` package ids; exceptions detailed in P3-1 |
| Schema validator is real | negative test: tampered `algorithm = "md5"` → `packaging-env.py` exit 1 with an actionable message; restored → exit 0, 49 vars (51 with bin names) | works as claimed — no silent fallback |
| Tool pins are real | downloaded all three x86_64 pins live and hashed | appimagetool 1.9.1 `ed4ce84f…84eb0` ✓, type2 runtime 20251108 `2fca8b44…260d` ✓, WiX 5.0.2 nupkg `f30ef0c7…e173` ✓; the nupkg really contains `tools/net6.0/any/wix.exe` and `wix.runtimeconfig.json` with `rollForward: Major`, as the workflow comment claims |
| Stubs/mocks | `rg` for `todo!`/`unimplemented!`/placeholder/fake/dummy across the diff | none in lane code; all "placeholder/stand-in" hits are documented honest fallbacks (accent-colored icon generator; the gate's stand-in files exist only to test the sidecar contract, clearly labeled) |
| Signing reality | read `sign-windows.sh` / `sign-macos.sh` line by line | real `signtool sign /fd SHA256 /tr … /td SHA256` + `signtool verify /pa /all`; real `codesign --force --timestamp --options runtime`, `security` scratch-keychain dance, `notarytool submit --wait`, `stapler staple`, `notarytool validate`. **No fake signing** — except the P1-1 masking bug and the P1-2 missing workflow wiring |
| Checksum honesty | manifest writers hash artifacts before the manifest exists; evidence digests full-length and internally consistent (r3 log step-5 dry-run digests match its step-1 manifest and `ls` sizes) | no dummy/fake checksums found |
| Evidence plausibility | logs vs sandbox reality | launch-test lines match the real CLI contract (`--version` → `lumen 0.1.0`, `BENCH … a_decoded=1`, desktop exit 86); the aarch64-appimagetool attempt is honestly narrated as impossible on the x86_64 host; the rpm spec-only path is honestly labeled; **no "verified on Windows/macOS" claims anywhere** — every CI-only step carries `[VERIFY at first live run]` |
| Cross-platform | read the .ps1 pair + Windows-path handling end to end | genuine Windows PowerShell (registry `UpgradeCodes` squished-GUID lookup, `msiexec /qn` with 3010 tolerance, elevated-shell assert, `ProgramFiles` join, backslash paths); zip lane uses `python -m zipfile` (no `zip` CLI on git-bash); AppImage lanes use `APPIMAGE_EXTRACT_AND_RUN` (no FUSE). One real cross-shell bug found (P2-2) |
| Repo hygiene | `git ls-files` | no binaries/.pyc/pinned tools committed; `.gitignore` negations effective (evidence logs tracked) |
| `set -euo pipefail` | per-script | all lane scripts are `#!/bin/sh` with `set -eu` (pipefail is bash-only — correct for POSIX sh); `gate.sh` uses `set -u` deliberately to collect all failures; EXIT traps guard temp cleanup (incl. the documented EXIT-trap-status fix in `sign-macos.sh:110–122`) |
| full_gate step 11 | read diff + executed the gate body | wired correctly (step 11 → `bash scripts/packaging/gate.sh`, failures aggregate into `fail`) |
| Blockers honesty | runbook blockers table | every blocker has owner + next step + tracking (R-35, Authenticode cert, CI billing, AppStream metadata, arm64 runners) — but see P1-2: the "set repo secrets" next-step currently has no mechanism behind it |

## Suggested merge posture

Merge **after** P1-1 and P1-2 (both small, both in the lane's core claim area: fail-loud signing + secrets-driven wiring). P2-1..P2-4 are follow-up-commit sized; P3s at leisure. Nothing found invalidates the local Linux-lane evidence or the config-plane design — both held up under adversarial re-execution.
