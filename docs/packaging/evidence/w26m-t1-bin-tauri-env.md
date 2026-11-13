# w26.m.t1 — BIN_TAURI env bridge (task W26-M-T1, issue #67 admission crate)

Local, reproducible proof for the config-driven env bridge picking up
the THIRD shipped bin: the workspace-EXCLUDED tauri admission crate
(`apps/desktop-tauri`, package `lumen-desktop-tauri`, bin
`lumen-tauri`).

| | |
|---|---|
| Date | 2026-11-13 |
| Host | Linux x86_64, this build sandbox |
| Base commit | `aebd6ba3` (branch `w26.m/t1-bin-tauri-env`) |
| Change | `scripts/release/product-env.py` + `scripts/packaging/packaging-env.py` only |
| Cargo | `cargo metadata` ONLY — a metadata-only read, nothing compiled; run with `CARGO_INCREMENTAL=0` |

What the change does, in one paragraph: root `cargo metadata` cannot
see `lumen-desktop-tauri` (root Cargo.toml `exclude` — the ADR-004
admission crate keeps its own lockfile so `--all-features` lanes never
drag the tauri graph into the main build). When the new
`--tauri-package`/`--tauri-manifest` pair is passed, product-env.py
runs a SECOND `cargo metadata --no-deps --format-version 1 --locked
--manifest-path <subcrate Cargo.toml>` — same flags and error posture
as the root call — and resolves `BIN_TAURI` through the same
`cargo_bin_name()` logic (exactly one `[[bin]]` target). Omit the pair
and `BIN_TAURI` is simply not emitted, same as the BIN_* behavior the
toolchain-less release-notes job relies on. Half a pair is a hard
error, not a silent omission (no-fallback posture).

## Proof 1 — WITH the tauri pair: `BIN_TAURI` joins `BIN_CLI`/`BIN_DESKTOP`

Command (from the repo root):

```
python3 scripts/packaging/packaging-env.py \
    --cli-package lumen-cli --desktop-package desktop \
    --tauri-package lumen-desktop-tauri \
    --tauri-manifest apps/desktop-tauri/Cargo.toml --print
```

Output (exit 0, full):

```
PRODUCT_NAME=Lumen
PRODUCT_SHORT_NAME=Lumen
PRODUCT_PUBLISHER=Lumen Contributors
PRODUCT_VERSION=0.1.0-dev
PRODUCT_VERSION_CODE=1
PRODUCT_DESCRIPTION=A clean-room, Rust-native media player.
PRODUCT_LOWER=lumen
PRODUCT_BUNDLE_ID=dev.lumen.Lumen
PRODUCT_APPSTREAM_ID=dev.lumen.Lumen
PRODUCT_MSI_UPGRADE_CODE=7c1f6a52-9d3e-4b8a-a001-3f9c2d64e701
PRODUCT_AUMID=dev.lumen.Lumen
PRODUCT_ACCENT_COLOR=#4f8cff
MSI_VERSION=0.1.0
BIN_CLI=lumen
BIN_DESKTOP=lumen-desktop
BIN_TAURI=lumen-tauri
PRODUCT_ICON_ICO=assets/brand/icon.ico
PRODUCT_ICON_ICNS=assets/brand/icon.icns
PRODUCT_ICON_PNG_512=assets/brand/icon-512.png
PRODUCT_LOGO_SVG=assets/brand/logo.svg
PRODUCT_URLS_HELP=https://lumen-player.example.org/docs
PACKAGING_CHECKSUM_ALGORITHM=sha256
PACKAGING_CHECKSUM_MANIFEST=SHA256SUMS.txt
PACKAGING_SBOM_SPEC_VERSION=1.5
PACKAGING_SBOM_ARTIFACT_SUFFIX=sbom.cdx.json
PACKAGING_DOCS=LICENSE README.md
PACKAGING_DESKTOP_CATEGORIES=AudioVideo;Video;Player;
PACKAGING_DESKTOP_TERMINAL=false
PACKAGING_LINUX_BINDIR=usr/bin
PACKAGING_LINUX_DOCS_DIR=usr/share/doc
PACKAGING_LINUX_APPLICATIONS_DIR=usr/share/applications
PACKAGING_LINUX_ICONS_DIR=usr/share/icons/hicolor/512x512/apps
PACKAGING_DEB_SECTION=video
PACKAGING_RPM_RELEASE=1
PACKAGING_RPM_LICENSE=MIT
PACKAGING_MACOS_UNIVERSAL=false
PACKAGING_MACOS_TARGETS=aarch64-apple-darwin x86_64-apple-darwin
PACKAGING_WINDOWS_BINDIR=bin
PACKAGING_WINDOWS_UPGRADE_TEST=true
PACKAGING_LAUNCH_VERSION_FLAG=--version
PACKAGING_LAUNCH_SMOKE_VERB=decode-bench
PACKAGING_LAUNCH_SMOKE_FIXTURE=tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav
PACKAGING_LAUNCH_DESKTOP_PENDING_EXIT=86
PACKAGING_SIGNING_WINDOWS_TIMESTAMP_URL=http://timestamp.digicert.com
PACKAGING_SIGNING_WINDOWS_PFX_B64_ENV=WINDOWS_SIGNING_PFX_B64
PACKAGING_SIGNING_WINDOWS_PFX_PASSWORD_ENV=WINDOWS_SIGNING_PFX_PASSWORD
PACKAGING_SIGNING_MACOS_IDENTITY_ENV=MACOS_SIGNING_IDENTITY
PACKAGING_SIGNING_MACOS_P12_B64_ENV=MACOS_SIGNING_CERT_P12_B64
PACKAGING_SIGNING_MACOS_P12_PASSWORD_ENV=MACOS_SIGNING_CERT_PASSWORD
PACKAGING_SIGNING_MACOS_NOTARY_KEY_ID_ENV=APPLE_API_KEY_ID
PACKAGING_SIGNING_MACOS_NOTARY_KEY_B64_ENV=APPLE_API_KEY_B64
PACKAGING_SIGNING_MACOS_NOTARY_ISSUER_ENV=APPLE_API_ISSUER
```

## Proof 2 — WITHOUT the tauri pair: `BIN_TAURI` absent, nothing else moves

Command:

```
python3 scripts/packaging/packaging-env.py \
    --cli-package lumen-cli --desktop-package desktop --print
```

Output (exit 0): identical to Proof 1 minus `BIN_TAURI` — asserted by
`diff` of the two runs, whose ENTIRE output is:

```
15a16
> BIN_TAURI=lumen-tauri
```

(one added line; every identity/packaging value byte-identical, so
existing callers — the release-notes job, all packaging lanes — see no
behavior change when they don't pass the pair).

## Negative tests (hard errors, exit 1, actionable messages)

1. Half a pair (`--tauri-package` without `--tauri-manifest`):
   `product-env: error: --tauri-package and --tauri-manifest are a pair — pass both or neither (the workspace-excluded crate resolves only from its own manifest)`
2. Wrong manifest path:
   `product-env: error: --tauri-manifest not found at /home/z/my-project/rust-vlc/apps/nope/Cargo.toml (pass the workspace-excluded crate's Cargo.toml, e.g. apps/desktop-tauri/Cargo.toml)`

## Honest notes

- The sandbox's default `PATH` does not carry cargo (it lives in
  `~/.cargo/bin`); the proof runs exported
  `PATH="$HOME/.cargo/bin:$PATH"` and `CARGO_INCREMENTAL=0`. CI runners
  with the Rust toolchain already have cargo on `PATH`.
- The subcrate's `--locked` metadata resolves cleanly against its own
  `apps/desktop-tauri/Cargo.lock` (verified before the change was
  written); a stale subcrate lockfile would fail the same way the root
  call does today — via the shared `cargo metadata failed:` handler.
- No packaging `.sh` script was touched: no bin literal is introduced
  anywhere; `lumen-tauri` appears only in this evidence file and in the
  error-message example, both non-executing.
