# w26.m.t2 — Linux payload lanes gain the tauri binary (task W26-M-T2, issue #67 admission crate)

Local, reproducible proof for the Linux packaging payload picking up the
THIRD shipped binary: every Linux lane that stages/installs
`BIN_CLI` + `BIN_DESKTOP` now stages/installs `BIN_TAURI` (the
workspace-EXCLUDED tauri admission shell, `apps/desktop-tauri`, bin
`lumen-tauri`), and the deb/rpm declare its webview runtime deps.

| | |
|---|---|
| Date | 2026-11-13 |
| Host | Linux x86_64, this build sandbox (Debian-family: dpkg-deb present, rpmbuild absent) |
| Base commit | `aebd6ba3` (= `main` at branch time), branch `w26.m/t2-linux-payload` |
| Change | 7 scripts under `scripts/packaging/` ONLY (no config files, no cross-lane files) |
| Builds | none — zero cargo builds/tests (Lead's job); stand-in zero-byte binaries for the staging proofs, labeled as such |
| Env bridge | `packaging-env.py` on this base does not yet export `BIN_TAURI` (the w26.m.t1 commit is a parallel lane) — see Proof 8 for the post-merge rehearsal |

Per-script changes (all `bash -n` + `sh -n` clean, gate step 2 green):

| script | change |
|---|---|
| `stage-linux-payload.sh` | `BIN_TAURI` required-env guard; `tauri_bin="$TARGET_DIR/release/$BIN_TAURI"` existence check (same "missing packaging input" hard error); third `install -m 0755` row; layout comment gains the `usr/bin` row |
| `build-targz.sh` | `BIN_TAURI` guard + existence check + third 0755 install + third tar member (`set -- "$BIN_CLI" "$BIN_DESKTOP" "$BIN_TAURI"`) |
| `build-deb.sh` | **`Depends: libgtk-3-0, libwebkit2gtk-4.1-0`** (the package's FIRST runtime-dep rows) + Description prose "Both shipped binaries" → "The shipped binaries (… and the tauri admission shell)" |
| `build-appimage.sh` | `BIN_TAURI` guard + existence check + second AppDir install (`usr/bin/$BIN_TAURI`); AppRun stays the desktop composition root; HONEST RUNTIME-DEP BOUNDARY comment (see below) |
| `build-rpm.sh` | **`Requires: gtk3` + `Requires: webkit2gtk4.1`** in the generated spec; `/usr/bin/$BIN_TAURI` in %files; `validate_spec` asserts BOTH Requires rows; %description prose updated |
| `verify-linux-artifacts.sh` | `BIN_TAURI` guard + 7 new assertion rows (counted in Proof 6 below) |
| `gate.sh` | bridge-pair probe + pair passed at all 3 `packaging-env.py` call sites + `BIN_TAURI` export label (step 1) + refusal contract extended (step 5a) |
| `gen-desktop-entry.sh` | UNTOUCHED (deliberate): it lists no payload binaries — `Exec=${BIN_DESKTOP}` stays, the entry launches the desktop composition root; the admission shell gets no desktop entry (do-not-invent) |
| `packaging.toml` / `packaging.schema.json` | UNTOUCHED (deliberate): binaries are NOT config rows ([shipped] docs only, by design); the webkit dep rows are script-local — see Honest notes |

## Proof 1 — syntax: every owned script parses under BOTH parsers

```
for f in scripts/packaging/{stage-linux-payload.sh,build-targz.sh,build-deb.sh,\
build-appimage.sh,build-rpm.sh,gen-desktop-entry.sh,verify-linux-artifacts.sh,gate.sh}; do
    bash -n "$f" && sh -n "$f" && echo "PARSE OK (bash -n + sh -n): $f"
done
```

Output: `PARSE OK` ×8, exit 0 (gate step 2 independently re-proves all 14
`scripts/packaging/*.sh`).

## Proof 2 — gate.sh, post-edit, pre-t1-merge base (this branch alone)

`sh scripts/packaging/gate.sh` → **PASS**, exit 0. The step-1 label is the
honest pre-merge one:

```
gate.sh: 1. packaging.toml <-> packaging.schema.json + product bridge
gate.sh:    packaging-env.py: 51 variables resolved (schema + semantic pass green)
gate.sh:    bridge does NOT export BIN_TAURI yet (w26.m.t1's --tauri pair lands it; until then the stager's required-env guard is BIN_TAURI's loud refusal) [VERIFY at first live run]
...
gate.sh:    stager refuses loudly without built binaries (as designed)
gate.sh: PASS (packaging config + scripts + self-tests + local-provable halves)
```

The gate probe checks `packaging-env.py --help` for `--tauri-package`
(the bridge's own declared interface); pre-merge it is false, post-merge
it is trivially true and the gate passes the pair at steps 1/5/6 — no
gate edit needed when the t1 commit lands (flattening the probe to a
direct pass is optional; noted for the Lead).

## Proof 3 — stager stages the three binaries FOR REAL (stand-in binaries)

Zero-byte stand-ins (proves LAYOUT + modes, not the real artifacts —
the real binaries are the release lane's job). `BIN_TAURI` is set BY
HAND to the exact value T1's bridge + release.yml export; the bridge
commit is not on this branch's base, so the export cannot come from it
here — stated, not hidden:

```
eval "$(python3 scripts/packaging/packaging-env.py --cli-package lumen-cli --desktop-package desktop --shell)"
export BIN_TAURI=lumen-tauri        # what the bridge exports post-merge
mkdir -p "$proof/target/release"    # : > the three stand-in binaries
OS_ID=linux ARCH_ID=x86_64 TARGET_DIR="$proof/target" \
    sh scripts/packaging/stage-linux-payload.sh "$proof/payload"
```

Staged tree (modes exact):

```
644 usr/share/applications/dev.lumen.Lumen.desktop
644 usr/share/doc/lumen/LICENSE
644 usr/share/doc/lumen/README.md
644 usr/share/icons/hicolor/512x512/apps/lumen.png
755 usr/bin/lumen
755 usr/bin/lumen-desktop
755 usr/bin/lumen-tauri      <-- the new row
```

## Proof 4 — negative: a missing tauri binary is the SAME loud refusal

```
rm "$proof/target/release/lumen-tauri"   # rerun the stager
stage-linux-payload.sh: error: missing packaging input: .../target/release/lumen-tauri (the tauri admission shell builds separately: --manifest-path apps/desktop-tauri/Cargo.toml, same CARGO_TARGET_DIR)
```

exit 1 (want 1). Required-binary posture is byte-for-byte the
`BIN_CLI`/`BIN_DESKTOP` contract.

## Proof 5 — tar.gz + deb with the three members (real archives, stand-in bodies)

```
DIST_DIR="$proof/dist" TARGET_DIR="$proof/target" OS_ID=linux ARCH_ID=x86_64 sh scripts/packaging/build-targz.sh
tar -tzf dist/lumen-0.1.0-dev-x86_64-linux.tar.gz
```
```
lumen
lumen-desktop
lumen-tauri      <-- third member
LICENSE
README.md
```

```
DIST_DIR="$proof/dist" TARGET_DIR="$proof/target" OS_ID=linux ARCH_ID=x86_64 sh scripts/packaging/build-deb.sh
dpkg-deb -f dist/lumen-0.1.0-dev-x86_64-linux.deb Package Version Architecture Depends
```
```
Package: lumen
Version: 0.1.0-dev
Architecture: amd64
Depends: libgtk-3-0, libwebkit2gtk-4.1-0      <-- the new webview rows
```
`dpkg-deb -x` of the same .deb lists `usr/bin/{lumen,lumen-desktop,lumen-tauri}`
all 0755.

## Proof 6 — build-rpm.sh --spec-only (the script's own dry-run mode) + verify rows

```
OS_ID=linux ARCH_ID=x86_64 TARGET_DIR="$proof/target" \
    sh scripts/packaging/build-rpm.sh --spec-only "$proof/spec/lumen.spec"
```
```
build-rpm.sh: wrote + validated /.../spec/lumen.spec (rpm_version=0.1.0~dev)
build-rpm.sh: spec-only mode — binary build needs rpmbuild (CI lane; local proof covers generation + validation only)
```

Spec carries (grep excerpt):

```
Requires:       gtk3                 <-- new
Requires:       webkit2gtk4.1        <-- new
/usr/bin/lumen
/usr/bin/lumen-desktop
/usr/bin/lumen-tauri                 <-- new %files row (validated against the staged tree)
```

`validate_spec` now asserts BOTH Requires rows; negative sanity that the
assertions bite — the greps reject a spec with the `Requires:` lines
sed-stripped (exit 1) and accept the real spec (exit 0).

## Proof 7 — verify-linux-artifacts.sh: the 7 new assertion rows (counted)

The script itself cannot run locally (it needs the real release
binaries + the pinned AppImage tools), so its new rows are counted here
and exercised at the first CI run:

1. `BIN_TAURI` required-env guard (mirrors `BIN_CLI`/`BIN_DESKTOP`).
2. tar.gz member check includes `$BIN_TAURI` (step 7 loop).
3. deb payload presence: `usr/bin/$BIN_TAURI` exec-bit check after `dpkg-deb -x` (presence only, NO launch — the admission shell needs the host's WebKitGTK; launching it in a sandbox would test the display stack, not the package; the boundary is stated in the script).
4. deb `Depends` contains `libgtk-3-0`.
5. deb `Depends` contains `libwebkit2gtk-4.1-0`.
6. rpm full build (CI path): `rpm -qlp` lists `/usr/bin/$BIN_TAURI` (loop over all three bins).
7. rpm spec-only (sandbox path): the generated spec's %files carries the `/usr/bin/$BIN_TAURI` row (`grep -Fxq`, loop over all three bins).

## Proof 8 — integration rehearsal: my scripts on top of `w26.m/t1-bin-tauri-env`

To prove the post-merge state (this is exactly what the Lead's merge
produces for the packaging lane), the seven edited scripts were copied
into a THROWAWAY detached checkout of T1's pushed branch
(`origin/w26.m/t1-bin-tauri-env` @ `9a87730a`), nothing committed there,
worktree removed after:

- `sh scripts/packaging/gate.sh` → **PASS**; `packaging-env.py: 52
  variables resolved`; step-1 label flips to the wired one:
  `bridge exports BIN_TAURI (third shipped bin wired end to end)`.
- With the bridge's own export (NO hand-set `BIN_TAURI`):
  `packaging-env.py --tauri-package lumen-desktop-tauri --tauri-manifest
  apps/desktop-tauri/Cargo.toml --shell` → `BIN_TAURI=lumen-tauri`, and
  the stager stages all three `usr/bin` binaries 0755.
- The refusal without binaries becomes the classic
  `missing packaging input: .../release/lumen` — the gate's 5a
  contract holds on BOTH sides of the merge (both accepted texts).

## Honest notes — what needs the first CI run

- Real binaries: every proof above used zero-byte stand-ins; sizes,
  real ELF content and the launch tests (`--version`, decode-bench,
  desktop skeleton exit 86) are the release lane's proof
  (verify-linux-artifacts.sh, first CI run).
- AppImage: `APPIMAGETOOL`/`RUNTIME_FILE` are pinned release-lane
  downloads — absent here, so neither the image build nor the new
  `--appimage-extract` payload row (verify step 6b) ran locally
  [VERIFY at first live run]. The check mechanism itself (type2
  runtime's own extraction, no FUSE) is the same one the build lane
  already relies on via `APPIMAGE_EXTRACT_AND_RUN`.
- rpmbuild: absent in this sandbox (as documented since W24-C); the rpm
  binary build + `rpm -qlp` payload row are CI-only [VERIFY].
- The webkit runtime caveat for the AppImage is a REAL boundary, not a
  TODO: the lane bundles no libraries (no linuxdeploy), so the tauri
  binary inside the AppImage needs the host's WebKitGTK 4.1 + GTK3 —
  stated in build-appimage.sh's header; bundling stays a non-goal.

## Cross-lane notes for the Lead

1. **gate.sh probe**: on pre-t1-merge main the probe (`packaging-env.py
   --help | grep -- --tauri-package`) is false and the gate passes with
   an honest `[VERIFY at first live run]` label; post-merge it passes
   the pair everywhere (Proof 8). Optionally flatten the probe to a
   direct pass once the t1 commit is on main.
2. **WebKit dep rows are script-local literals** (`libgtk-3-0` +
   `libwebkit2gtk-4.1-0` in build-deb.sh; `gtk3` + `webkit2gtk4.1` in
   build-rpm.sh): packaging.toml has no `deb_depends`/`rpm_requires`
   keys and build-deb/build-rpm build no Depends from config today, so
   per the task's "if the script builds Depends from config" clause the
   rows went into the scripts, honestly commented. If config-driving
   them is wanted, it needs packaging.toml + schema + packaging-env.py
   (T1's file) in ONE change — not done unilaterally.
3. **Ubuntu package name check**: `libwebkit2gtk-4.1-0` is the
   ubuntu-22.04+ webkit2gtk-4.1 ABI package (jammy/noble both carry it);
   `libgtk-3-0` arrives transitively via webkit's own Depends but is
   declared explicitly. Fedora name per the task: `webkit2gtk4.1`
   (f38+); `gtk3` declared for symmetry. The deb/rpm rows pair with the
   sandbox's webkit absence: the tauri binary is NEVER launch-tested
   locally — presence + declared deps only.
4. gen-desktop-entry.sh, packaging.toml, packaging.schema.json,
   packaging-env.py, product-env.py, .github/**, and all
   Windows/macOS lanes: untouched (see the table above for the two
   deliberate untouched decisions).
