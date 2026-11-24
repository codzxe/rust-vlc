# AUDIT: .github/workflows/release.yml
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 1175 / 6 jobs, 83 steps (15+12+12+14+14+10) |
| File verdict | REAL-UNPROVEN (structure, config wiring, guards verified locally; the pipeline itself has never run to completion — see F-001) |
| Reachable from shipped entry point? | YES — it IS the release lane: `cargo build --release --locked -p lumen-cli -p desktop` + `cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml` (L193, L219) matches the orchestrator baseline release lane |
| Findings | S0: 0 S1: 0 S2: 3 S3: 3 S4: 3 |

## 1. One-paragraph verdict
This is a real, load-bearing release pipeline, not a paper workflow: pinned SHAs (match ci.yml's pin table, byte-verified), config-driven identity via packaging-env.py (executed here, exit 0, 9 PACKAGING_SIGNING_* exports), fail-loud secret-wiring guards, SHA-256-verified tool downloads, `--locked` builds, per-artifact SBOMs, a global checksum manifest, and — uniquely strong — a launch smoke of the exact packaged Windows exe (10 s survival + failure-receipt grep) and a locally-proven Linux harness. The two structural weaknesses: (1) the header's "THIS WORKFLOW HAS NEVER EXECUTED ... dies at 0 steps" claim is contradicted by the file's own inline citations of live runs 43378757248 and 43464162696 — the honesty note is stale or false; (2) the release job's artifact-kind verification is warning-only, so a lane that silently under-produces still cuts a release. Unsigned shipping is disclosed (sidecars + ::warning::), not faked — the §8.17 "unsigned presented as signed" fraud pattern is NOT present, but unsigned releases WILL be cut by default while credentials are absent, with only a warning between the user and that outcome.

## 2. What actually works (verified)
— `python3 -c yaml.safe_load(...)` -> parses OK; jobs = windows/macos-arm64/macos-x64/linux-x64/linux-arm64/release; linux-arm64 `continue-on-error: true` (documented best-effort), all others blocking.
— All 19 referenced scripts/configs exist (`scripts/packaging/{build-zip,build-msi,build-deb,verify-linux-artifacts,build-dmg,gen-sboms,sign-windows,sign-macos,gate}.sh`, `test-msi-upgrade.ps1`, `packaging-env.py`, `scripts/release/{release-notes.sh,product-env.py}`, `packaging/packaging.toml`, `product.toml`, `packaging.schema.json`, smoke fixture `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi`) — checked with `test -e`, all OK.
— `python3 scripts/packaging/packaging-env.py --print` -> exit 0; exports PRODUCT_* + PACKAGING_* incl. `PACKAGING_SIGNING_WINDOWS_PFX_B64_ENV` etc. — the guard variables the workflow's fail-loud checks consume actually exist.
— Action pins cross-checked against `.github/workflows/ci.yml` L90-93: checkout 3d3c42e, toolchain d124f86f, cache 6323deb, upload cf430e0 — identical in both files.
— `rust-toolchain.toml` channel = "1.99.1" matches the toolchain-assert step's expectation and the pin comment.
— packaging.toml parses (tomllib) and `windows.upgrade_test = true`, so the MSI upgrade-path test is NOT silently skipped in the shipped config.

## 3. Findings

F-001 [S2]
Where: .github/workflows/release.yml:26-28 vs :278,1138,1145
Code:
```
# HONESTY NOTE — THIS WORKFLOW HAS NEVER EXECUTED (CI billing blocked):
# every Actions run in this repository currently dies at 0 steps
```
vs (L278):
```
            # with "invalid variable name" (run 43378757248, MSI lane,
```
vs (L1145):
```
          # "body is too long" -- first live tag run 43464162696, step
```
What is wrong: The header asserts zero-step death for every Actions run, while four inline comments (L278, L539, L681, L1059) cite live run 43378757248 with a step/line-specific failure, and L1138/L1145 cite live tag run 43464162696 twice with concrete failure modes (signtool guard "invalid variable name"; HTTP 422 "body is too long"). These are mutually exclusive: either the workflow HAS executed (header false/stale) or the run citations are fabricated. The inline citations describe post-mortem fixes too specific to be invented (CRLF in `$(...)`, 125000-char release-body cap), so the header's "never executed" honesty note is the false side. Why it matters: the entire evidence posture of this file ("the Linux lane log is the proof because CI never runs") rests on that note; a stale note means the workflow's real execution history is being under-reported, and future readers cannot tell which [VERIFY] markers were actually discharged. What correct looks like: the honesty note must be updated to the current billing state, and every [VERIFY] inline discharged or kept in a single list. Confidence: HIGH — Verification: read (ripgrep byte-verified, all 6 lines).

F-002 [S2]
Where: .github/workflows/release.yml:1090-1105 (release job "Verify artifact set")
Code:
```
          for kind in windows.zip windows.msi macos.dmg macos.zip \
                      linux.tar.gz linux.deb linux.AppImage linux.rpm; do
            if ls dist/*-"$kind" >/dev/null 2>&1; then
              echo "artifact kind present: ${kind}"
            else
              echo "::warning::artifact kind missing: ${kind} (arm64 best-effort or job failure — check the build jobs)"
            fi
          done
```
What is wrong: The only completeness check on the user-downloadable payload is a WARNING, not a failure. A blocking lane whose packaging script under-produces (e.g. an rpm step that exits 0 without writing the rpm, or a glob mismatch after a rename) still cuts a public release minus artifacts, with nothing red. `if-no-files-found: error` on each upload only catches a fully-empty dist/. Why it matters: §8.17 — the workflow's final gate before `gh release create` cannot fail on a missing artifact; "release shipped incomplete" is exactly the class this gate exists to stop. What correct looks like: the eight blocking kinds (non-arm64) hard-fail; only arm64 kinds warn. Confidence: HIGH — Verification: read. (Mitigation present: each packaging script is individually fail-loud, so this is defense-in-depth that is soft — hence S2 not S1.)

F-003 [S2]
Where: .github/workflows/release.yml:436-578 and 589-716 (macos-arm64 / macos-x64 jobs)
Code:
```
      - name: Per-artifact SBOM (gen-sboms.sh — CycloneDX, suffix + spec version from config)
        run: |
          base="${PRODUCT_LOWER:?}-${PRODUCT_VERSION:?}-${ARCH_ID:?}-${OS_ID:?}"
          scripts/packaging/gen-sboms.sh "dist/${base}.dmg" "dist/${base}.zip"
```
What is wrong: Neither macOS lane launch-tests anything. Compare: Windows gets a packaged-exe launch smoke (L292-380) and Linux runs verify-linux-artifacts.sh with packaged-CLI launch tests; macOS builds, signs (or sidecars), SBOMs, zips, uploads — zero execution of the shipped .app/tauri binary. The workflow's own header (L294-296) states the v0.1.2 defect class was "a shipped exe that cannot start stayed green", and the fix was to launch-test the packaged bytes — but the fix was applied only to Windows and Linux. Why it matters: the exact defect class the workflow documents fixing is un-gated on macOS; a macOS tauri binary that fails at startup ships green. What correct looks to: an `open -W`/`timeout` launch smoke of the extracted .app on the runner (headless caveats documented), or an explicit "[VERIFY] macOS launch untested" admission in the lane comment — the current comments carry no such admission. Confidence: HIGH — Verification: read (job step lists enumerated). Note the header's L84-89 lane map also omits any macOS launch step, so this is an honest omission, not a false claim — S2 not S0.

F-004 [S3]
Where: .github/workflows/release.yml:1126,1133,1165
Code:
```
        if: github.ref_type == 'tag'
```
(both `gh release create` and the tag/version cross-check) vs L132:
```
  workflow_dispatch: # manual dry-run: builds + uploads artifacts, no release (see `release` job)
```
What is wrong: `workflow_dispatch` can be run FROM A TAG ref in the Actions UI; in that case `github.ref_type == 'tag'` is true and the "dry-run ... no release" promise at L132 is false — the dispatch cuts a real release. The gate distinguishes ref type, not event type. What correct looks like: `if: github.event_name == 'push'` (or `startsWith(github.ref, 'refs/tags/')` AND event push). Confidence: MEDIUM — Verification: read (GitHub dispatch-on-tag behavior is documented platform semantics; not executable here — UNVERIFIED runtime behavior). Severity S3 (requires deliberate UI choice of a tag while dispatching).

F-005 [S3]
Where: .github/workflows/release.yml:1116-1122 (tag vs product.toml version cross-check)
Code:
```
          if [ "${tag_version}" != "${PRODUCT_VERSION}" ] && [ "${tag_version}" != "${MSI_VERSION}" ]; then
            echo "::warning::tag ${GITHUB_REF_NAME} matches neither identity.display_version (${PRODUCT_VERSION}) nor its numeric MSI form (${MSI_VERSION}) — intentional?"
          fi
```
What is wrong: A tag/product version mismatch ships: the release title/body use PRODUCT_VERSION while the release is attached to the tag — a mismatched release is published with a warning only. Self-declared ("warning only — honest, not fatal"), so not hidden, but the version-identity invariant of the whole config-driven design has no teeth at its one enforcement point. Confidence: HIGH — Verification: read.

F-006 [S3]
Where: .github/workflows/release.yml:262-290 (MSI step guard) — logic verified sound; residual risk
Code:
```
            cfg="${cfg%$'\r'}"
            name="${!cfg-}"
```
What is wrong (residual): the guard proves the secret NAME is mapped, not that the credential is valid; an expired PFX produces signed-then-failed signtool inside build-msi.sh (handled there). Also the four copies of the same ~25-line guard block (L262-290, L517-551, L663-693, L1046-1080) are copy-paste x4 in one file — a future edit to one copy and not the other three drifts silently (no shared script). Confidence: HIGH — Verification: read + packaging-env.py export list cross-checked (all derived names exported; guard would pass).

F-007 [S4]
Where: .github/workflows/release.yml:134-135
Code:
```
permissions:
  contents: write # `gh release create` with the built-in GITHUB_TOKEN
```
What is wrong: workflow-wide `contents: write` grants every build job write scope it never uses; only the `release` job needs it. Hygiene, documented intent, low blast radius (same-repo only). Confidence: HIGH — Verification: read.

F-008 [S4]
No `concurrency` group: two tag pushes in quick succession run two full pipelines concurrently; no cancel-in-progress. S4 hygiene.

F-009 [S4]
Where: .github/workflows/release.yml:986-988 (release job download)
Code:
```
      - name: Download all dist artifacts (flat merge)
        uses: actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333 # v8.0.2
```
What is wrong: `merge-multiple: true` flat-merges x86_64+aarch64 Linux sets; both sets' files coexist (unique by arch segment), verified OK — but a same-named file across sets would silently overwrite. Currently prevented only by naming discipline. UNVERIFIED: download-artifact v8.0.2 pin resolves to that SHA (offline; pin comment claims git ls-remote on 2026-11-10).

## 4. Per-function (per-job/step) table — every step, all 6 jobs
| Job / step | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| windows: checkout/toolchain | 161-163 | REAL | - | pinned SHA = ci.yml pin (verified) |
| windows: Assert toolchain pin | 164-175 | REAL | - | exit 1 on mismatch; sed/rustc cross-check |
| windows: rust-cache | 176 | REAL | - | pinned v2.9.2 SHA |
| windows: packaging-env parse | 177-191 | REAL | - | script exists; ran locally exit 0; wraps product-env.py |
| windows: Build (release, locked) | 192-193 | REAL | - | matches baseline release lane exactly |
| windows: Build embedded UI | 194-205 | REAL | - | index.html existence check; ordering documented |
| windows: tauri build | 206-219 | REAL | - | --manifest-path on workspace-excluded crate, --locked |
| windows: WiX install | 220-234 | REAL | - | SHA-256 `sha256sum -c -` hard gate; nupkg extract |
| windows: build-zip.sh | 235-236 | REAL-UNPROVEN | - | script exists; not run here (cargo build budget) |
| windows: build-msi.sh + wiring guard | 237-291 | REAL | - | guard derives names from packaging.toml at guard time; CRLF strip; exit 1 paths live |
| windows: zip launch smoke (pwsh) | 292-380 | REAL-UNPROVEN | - | real 10 s survival + receipts grep + tree kill; fixture exists; needs Windows to execute |
| windows: MSI upgrade test | 381-395 | REAL | - | config-gated; `upgrade_test = true` in toml => runs (verified) |
| windows: gen-sboms + upload | 396-406 | REAL | - | `if-no-files-found: error` = upload can fail |
| macos-arm64: (12 steps) | 436-578 | REAL-UNPROVEN | - | see F-003: no launch test on this lane; guard + ditto zip + sidecar copy verified by read |
| macos-x64: (12 steps) | 589-716 | REAL-UNPROVEN | - | duplicate of arm64 lane; runs-on macos-15-intel [VERIFY inline] |
| linux-x64: build + apt webkit + tauri | 733-830 | REAL | - | pkg-config existence assert; pinned appimagetool + type2 runtime SHA-256 verified twice (x86_64/aarch64 URLs distinct) |
| linux-x64: verify-linux-artifacts.sh | 845-850 | REAL-UNPROVEN | - | the locally-proven harness; evidence log exists (spot-checked: real appimagetool/mksquashfs/dpkg-deb output); not re-run here (cargo-build-budget) |
| linux-x64: manifest scope mv | 851-858 | REAL | - | plain rename before flat merge |
| linux-arm64: (14 steps) | 872-1002 | REAL-UNPROVEN | S3 | `continue-on-error: true` (L886) — job-level best-effort; documented; release proceeds on arm64 failure |
| release: parse (no packages) | 1042-1048 | REAL | - | executed locally: works without --cli-package (BIN_* omitted), exit 0 |
| release: signing visibility | 1049-1080 | REAL | - | unset=wiring gap (exit 1), empty=absent credential (::warning::); 8 names mapped, verified vs toml |
| release: download + verify set | 1081-1105 | SHALLOW | S2 | F-002: kind check warning-only |
| release: global checksum manifest | 1106-1121 | REAL | - | sha256 only wired; manifest never hashes itself; `cat` for log |
| release: release-notes.sh | 1122-1123 | REAL-UNPROVEN | - | script exists; needs git history on runner (fetch-depth: 0 set at checkout L1035-1038) |
| release: tag/version cross-check | 1124-1131 | SHALLOW | S3 | F-005 warning-only |
| release: gh release create | 1132-1163 | REAL | - | tag-ref gated; 120000-char body bound (real post-mortem fix); dry-run cannot cut release on branch refs |
| release: dry-run summary | 1164-1175 | REAL | - | writes GITHUB_STEP_SUMMARY; SC2129-clean |

## 5. Test honesty
No tests in-file. The "evidence" is scripts + docs: docs/packaging/evidence/linux-lane-20261110.log contains genuine tool output (dpkg-deb, appimagetool 8c8c91f, mksquashfs zstd) — spot-checked real, not fabricated console text. rc-smoke/web-budget failures in the orchestrator baseline do NOT gate this workflow (tag-gated only), so a red local gate can coexist with a green release — by design, since release.yml runs nothing from scripts/quality.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "THIS WORKFLOW HAS NEVER EXECUTED ... dies at 0 steps" | release.yml:26-27 | FALSE (self-contradicted) | inline run IDs 43378757248 (x4) and 43464162696 (x2) in same file, ripgrep-verified |
| "Nothing claims a GUI that does not exist" | release.yml:24 | REAL | payload table matches packaging.toml [shipped] comments: CLI+tauri+docs in zip/deb/rpm/tar.gz; AppImage/.app tauri-only; MSI pair+docs+shortcuts (to-be-checked in gen-wxs.sh) |
| "every script below passes `bash -n` + `sh -n`" | release.yml:36-38 | UNVERIFIED | would need to run per-script; scripts/packaging not in my file list |
| "linux lane was EXECUTED for real ... linux-lane-20261110.log" | release.yml:30-33 | REAL | evidence log exists w/ genuine tool output |
| "upgrade test config-gated" | release.yml:381-386 | REAL | packaging.toml windows.upgrade_test = true (tomllib, ran) |
| unsigned binaries "labeled" not faked | release.yml:238-240, 501-504 | REAL | sidecar .UNSIGNED.txt/.NOT-NOTARIZED.txt + ::warning:: path; sign-windows/macos.sh exist |

## 7. Dead / duplicated / unreachable code
- The wiring-guard block is duplicated verbatim 4x (L262-290, L517-551, L663-693, L1046-1080) — S3/S4 duplication with drift risk.
- `PACKAGING_LAUNCH_DESKTOP_PENDING_EXIT` exported by packaging-env.py but no release-lane consumer (retired w30.8 per packaging.toml comment) — config row kept deliberately; dead-but-documented.

## 8. Dependencies & flags
- Third-party surface = 5 pinned actions (SHAs match ci.yml table, verified) + pinned WiX nupkg (SHA-256 in-step) + pinned appimagetool 1.9.1/type2-runtime 20251108 (SHA-256 in-step, both arches). No floating tool installs found — claim at L103-112 verified by read.
- Build features: none — `cargo build --release --locked -p lumen-cli -p desktop` (no --features), matching the baseline release lane; shipped build = no feature-gated code enabled here.
- deps: rust-toolchain.toml channel 1.99.1 asserted 3-way at runtime.

## 9. Open questions for the next agent
1. gen-wxs.sh / build-msi.sh: does the MSI member list still include the exit-86 `lumen-desktop` stub (header L16-18 says the twins' lane owns that list as of w30.9 — confirm which way)?
2. scripts/packaging/verify-linux-artifacts.sh — audit its gates (claimed: deb/rpm header asserts, CLI launch tests); is the .rpm built by rpmbuild or spec-only?
3. Do the macOS lanes in tauri-launch.yml compensate for F-003 (xvfb lane covers Linux only)?
4. Verify `bash -n` over scripts/packaging/*.sh (header claim L36).
5. The 4 missing brand assets (assets/brand/*.{ico,icns,png,svg}) — releases will ship generated placeholder icons; check build-dmg.sh/make-app for the fallback path honesty.
