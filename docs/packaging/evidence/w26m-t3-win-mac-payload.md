W26-M T3 evidence — Windows/macOS payload lanes gain `lumen-tauri`
===================================================================
Branch `w26.m/t3-win-mac-payload` (base: main `aebd6ba3`), 2026-11-13.
Task: the third shipped binary (`BIN_TAURI`, the workspace-excluded
`apps/desktop-tauri` admission crate's bin) rides the Windows zip, the
MSI (new `BinTauri` Component/ComponentRef) and the macOS `.app`/`.dmg`
payload exactly like `BIN_CLI`/`BIN_DESKTOP`.

Owned files changed (nothing else touched):
`scripts/packaging/build-zip.sh`, `scripts/packaging/build-msi.sh`,
`scripts/packaging/build-msi.ps1`, `scripts/release/gen-wxs.sh`,
`scripts/packaging/build-dmg.sh`, `scripts/release/make-app.sh`,
`scripts/release/package-windows.sh`.

1. Syntax proof (`bash -n` AND `sh -n` per script — the gate.sh step-2
   contract; POSIX-sh scripts must parse under both)
-------------------------------------------------------------------
    for f in scripts/packaging/build-zip.sh scripts/packaging/build-msi.sh \
             scripts/packaging/build-dmg.sh scripts/release/gen-wxs.sh \
             scripts/release/make-app.sh scripts/release/package-windows.sh; do
        bash -n "$f" && sh -n "$f" && echo "OK: $f"
    done
    -> OK: all six (exit 0 on both parsers, every file)

2. PowerShell structural sanity (no pwsh in this sandbox — gate.sh
   step-4 floor: ASCII-only, delimiters balanced, param-first)
-------------------------------------------------------------------
    python3 scripts/packaging/ps1-sanity.py scripts/packaging/build-msi.ps1
    -> ps1-sanity: PASS (ASCII-only, delimiters balanced, param first)
   The new BinTauri rows in the .ps1 are pure ASCII; the generated XML
   comment avoids the `--` sequence (see 3b) so it is legal XML and
   byte-identical between the two generators.

3. The ONE .wxs generator: `gen-wxs.sh --self-test` (hostile fixtures)
-------------------------------------------------------------------
3a. PASS line after the change:

    sh scripts/release/gen-wxs.sh --self-test
    -> gen-wxs.sh: --self-test PASS (5 File rows parsed; hostile
       basenames + identity round-trip; escape set pinned on raw text;
       ids sanitized; exe rows KeyPath; Feature refs cover every
       component by Id=)

   Self-test extensions (the BIN_DESKTOP-assertion pattern extended for
   BIN_TAURI, per the task): the hostile fixture set now carries
   `BIN_TAURI=selftest-tauri`; the python proof asserts
   - all three exe rows present (`selftest-cli.exe`,
     `selftest-desktop.exe`, `selftest-tauri.exe`), each KeyPath=yes;
   - the Feature's ComponentRef set covers EVERY Component id exactly
     (BinCli, BinDesktop, BinTauri, Docs), each ref carrying `Id=` and
     never a `RefId=` attribute (pins the just-fixed RefId typo class).

3b. The self-test caught a real bug in this very change: the first
   draft of the generated WebView2 XML comment contained `--`, which
   is ILLEGAL inside an XML comment (`ParseError: not well-formed,
   line 24`). Fixed before landing: the comment now reads
   "(preinstalled on Win11; distributed by the Evergreen setup on
   Win10). No runtime is bundled or bootstrapped here." — no double
   hyphens. The hostile-fixture harness did exactly its job.

4. Generated-shape proof (plain fixtures, real generation)
-------------------------------------------------------------------
    PRODUCT_NAME=Lumen … BIN_CLI=lumen BIN_DESKTOP=lumen-desktop \
      BIN_TAURI=lumen-tauri PACKAGING_DOCS="LICENSE README.md" \
      sh scripts/release/gen-wxs.sh <out>.wxs
    python3 -c "ET.parse('<out>.wxs')" -> parses OK

    Component/File/ComponentRef rows in the generated .wxs:
        <Component Id="BinCli" ...>      <File Id="BinCliExe" Source="lumen.exe" KeyPath="yes" />
        <Component Id="BinDesktop" ...>  <File Id="BinDesktopExe" Source="lumen-desktop.exe" KeyPath="yes" />
        <!-- WebView2 assumption comment -->
        <Component Id="BinTauri" ...>    <File Id="BinTauriExe" Source="lumen-tauri.exe" KeyPath="yes" />
        <Component Id="Docs" ...>        (one File row per [shipped] doc)
    Feature "Main": ComponentRef Id= BinCli, BinDesktop, BinTauri, Docs
    (asserted programmatically: comps == refs == those four, in order;
    every ref uses `Id=`; 5 File rows total.)

5. Windows zip lanes, executed FOR REAL (stub exes; python -m zipfile
   works on this host — same tool the runner uses)
-------------------------------------------------------------------
5a. Negative test — the guard refuses loudly without the third binary:

    env …-full-minus-BIN_TAURI sh scripts/packaging/build-zip.sh
    -> scripts/packaging/build-zip.sh: 30: BIN_TAURI: build-zip.sh:
       BIN_TAURI not set (run packaging-env.py first)
    -> exit 2

5b. Full run with all three stub exes:

    env PRODUCT_LOWER=lumen PRODUCT_VERSION=0.1.0-dev BIN_CLI=lumen \
        BIN_DESKTOP=lumen-desktop BIN_TAURI=lumen-tauri OS_ID=windows \
        ARCH_ID=x86_64 PACKAGING_DOCS="LICENSE README.md" \
        TARGET_DIR=<stub>/target DIST_DIR=<stub>/dist \
        sh scripts/packaging/build-zip.sh
    -> wrote lumen-0.1.0-dev-x86_64-windows.zip; member list:
       lumen.exe, lumen-desktop.exe, lumen-tauri.exe, LICENSE, README.md
       (flat root — BIN_TAURI staged exactly like BIN_DESKTOP, docs
       payload boundary unchanged)

5c. Superseded wave-22 flow kept consistent:

    env … BIN_TAURI=lumen-tauri … sh scripts/release/package-windows.sh
    (WIX_EXE unset) -> zip member list identical to 5b; "MSI skipped"
    path intact. (Its gen-wxs.sh call REQUIRES BIN_TAURI now — staging
    the exe keeps that flow working rather than dying mid-lane.)

6. MSI lane up to the CI-only boundary (WIX_EXE=/bin/false — a real
   executable that cannot build: proves staging + generation + the
   honest stop, fakes nothing)
-------------------------------------------------------------------
    env PRODUCT_* MSI_VERSION=0.1.0 PRODUCT_MSI_UPGRADE_CODE=… \
        BIN_CLI=lumen BIN_DESKTOP=lumen-desktop BIN_TAURI=lumen-tauri \
        OS_ID=windows ARCH_ID=x86_64 PACKAGING_WINDOWS_BINDIR=bin \
        PACKAGING_DOCS="LICENSE README.md" WIX_EXE=/bin/false \
        sh scripts/packaging/build-msi.sh
    -> gen-wxs.sh: wrote <stage>/installer.wxs (name=Lumen version=0.1.0
       upgrade-code=…)
    -> build-msi.sh: error: wix build failed        (exit 1 — expected,
       `wix build` needs Windows + .NET [VERIFY at first live run])
    -> dist/lumen-0.1.0-dev-x86_64-windows.wxs KEPT (provenance copy);
       python ET.parse + row assertions on THAT file: 5 File rows
       (lumen.exe, lumen-desktop.exe, lumen-tauri.exe, LICENSE,
       README.md), components = refs = BinCli/BinDesktop/BinTauri/Docs,
       every ComponentRef uses Id=.

7. macOS lanes, executed to the macOS-only boundary
-------------------------------------------------------------------
    env PRODUCT_NAME=Lumen … BIN_DESKTOP=lumen-desktop \
        BIN_TAURI=lumen-tauri TARGET_DIR=<stub> APP_DIR=<stub>/bundle \
        sh scripts/release/make-app.sh
    -> wrote Lumen.app (…; MacOS/: lumen-desktop + lumen-tauri)
    -> tree: Contents/MacOS/{lumen-desktop, lumen-tauri},
       Contents/Resources/{LICENSE, README.md},
       Info.plist CFBundleExecutable=lumen-desktop (the tauri exe is an
       AUXILIARY executable — CFBundleExecutable unchanged)

    Full orchestrator (signing env names set; credentials absent by
    construction):
    env … PACKAGING_SIGNING_MACOS_*_ENV=… sh scripts/packaging/build-dmg.sh
    -> make-app.sh: wrote Lumen.app (both exes in MacOS/)
    -> sign-macos.sh: ::warning:: UNSIGNED (R-35) + .UNSIGNED.txt
       sidecar (the honest-unsigned contract, holds)
    -> make-dmg.sh: error: hdiutil not found — must run on macOS
       (exit 2 at the exact macOS boundary; hdiutil is CI-only
       [VERIFY at first live run])

8. .ps1 self-test proof body (no pwsh here — the body is what
   build-msi.ps1 -SelfTest feeds python on the runner)
-------------------------------------------------------------------
    Extracted the `$proof = @' … '@` python heredoc from
    scripts/packaging/build-msi.ps1 and executed it against the bash
    twin's hostile-fixture output (the two generators emit the same
    shape):
    -> build-msi.ps1: -SelfTest PASS (5 File rows parsed; hostile
       basenames + identity round-trip; escape set pinned on raw text;
       ids sanitized; exe rows KeyPath; Feature refs cover every
       component by Id=)
    The real pwsh execution remains runner-only [VERIFY at first live
    run].

9. Count assertions updated (requirements 3/6 sweep)
-------------------------------------------------------------------
- build-msi.ps1 -SelfTest python proof: `len(files) == 4` -> `== 5`
  ("expected 5 File rows (3 exes + 2 docs)") — the only numeric
  File-row count in the repo (grep-verified).
- gen-wxs.sh --self-test: no numeric count existed; the structural
  coverage assertions in 3a were ADDED (they imply the count).
- build-msi.sh does NOT run the MSI upgrade test — verified by grep:
  the config gate lives in release.yml (`PACKAGING_WINDOWS_UPGRADE_TEST`,
  step "MSI upgrade-path test") and runs
  scripts/packaging/test-msi-upgrade.ps1 separately. test-msi-upgrade.ps1
  asserts ProductCode counts (1/1/0) and per-file presence — NOT
  component counts — so nothing in it breaks with a fourth component.
  It is NOT in this lane's owned-file set: see cross-lane note N1.

10. What needs the first CI run (honest boundary)
-------------------------------------------------------------------
- `wix build` of the 5-row .wxs (Windows + .NET) — the BinTauri row is
  XML-parse-proven locally; the MSI itself is not.
- `build-msi.ps1` under real pwsh (runner parse + -SelfTest).
- `hdiutil` dmg + codesign/notarytool (the whole post-.app macOS half).
- The WebView2 runtime assumption itself (system component on
  Win10/11): not testable from this repo's lane — noted in the
  generated XML, revisit only if a user hits a missing-runtime box.

Cross-lane notes (for the Lead; NOT edited by this lane)
-------------------------------------------------------------------
N1. scripts/packaging/test-msi-upgrade.ps1 `Assert-InstalledFiles`
    iterates @("$($env:BIN_CLI).exe", "$($env:BIN_DESKTOP).exe").
    Owner should append "$($env:BIN_TAURI).exe" so the upgrade test
    proves the third exe installs/replaces/uninstalls too. Nothing
    breaks before that fix (the loop checks the two it knows).
N2. T1 (branch w26.m/t1-bin-tauri-env, commit 9a87730a) exports
    BIN_TAURI from the bridge; until it merges, every lane script dies
    at its BIN_TAURI guard — correct fail-loud behavior, integration
    order matters (T1 first, or same-wave merge).
N3. docs/packaging/README.md payload table (line ~51 "BOTH shipped
    binaries", and the macOS row) predates the third binary; a docs
    refresh should follow the wave merge.
N4. T4 owns release.yml: build steps must build the subcrate
    (`cargo build --release --locked` for the lumen-desktop-tauri
    package, e.g. via --manifest-path apps/desktop-tauri/Cargo.toml)
    before packaging; BIN_TAURI itself needs no workflow-side literal —
    packaging-env.py exports it once T1 merges.
