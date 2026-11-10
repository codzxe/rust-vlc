# Getting started

No prebuilt binaries exist yet (see the README Install section — the
release pipeline is built but no release has been published). Everything
below is verified on the repository's reference sandbox (x86_64 Linux,
Intel Xeon, 10 vCPU — `uname -m` = `x86_64`; 2026-11-10, tree at commit
`559b897`).

## 1. Build from source

Requirements: the Rust toolchain pinned by `rust-toolchain.toml`
(cargo 1.99.1), a Git checkout, and for the UI section Node.js with the
`ui/package-lock.json` dependencies installed (`npm ci` in `ui/`).

```sh
git clone <repository> && cd <repository>
cargo build --release --locked -p lumen-cli -p desktop
```

Two binaries come out: `lumen` (the CLI — the real engine surface) and
`lumen-desktop` (the ADR-004 composition-root skeleton; it exercises
packaging paths and opens no window — `apps/desktop/README.md`).

Check what you built:

```sh
$ lumen --version
lumen 0.1.0
```

(`--version` reports the CLI crate's own version, `apps/cli/Cargo.toml`
— the product identity wiring from `product.toml` is a recorded
[PROPOSAL] in `apps/cli/src/help.rs`; the product version
`0.1.0-dev` lives in `product.toml` and drives packaging.)

## 2. First play (CLI)

`lumen` is an interactive REPL when run without arguments (or one-shot
with a subcommand). Open a media file and drive it:

```sh
$ lumen open tests/fixtures/generated/valid/avi_mjpeg_pcm.avi
ok: accepted session=1 gen=0
event: state-changed Idle → Opening
...
event: state-changed Buffering → Playing
info
ok: media
  source: avi_mjpeg_pcm.avi
  container: avi
  duration: 2.000000s
  seekable: yes
  tracks: 2
  #0 video mjpeg 320x240 @30.000 (selected)
  #1 audio pcm_s16le 2ch 44100Hz (selected)
time
ok: time 0.000000s (0.0%), 1 buffered range(s)
stop
quit
```

The full verb set (from `lumen --help`, generated from the settings
schema): `open <path> [--start-time T] [--start-paused] [--no-video]
[--no-audio]`, `close`, `play`, `pause`, `toggle`, `stop`, `seek 90s |
+10s | -1m | 50% | chapter 3`, `info`, `time`, `duration`, `tracks`,
`engine-info`, `commands`, `setting get|set <area.key> <value>`,
`decode-bench <path>`, `advance [ticks]`, `help`, `quit`.

**What "playing" means today (honest):** the engine demuxes, decodes
and synchronizes for real — but the audio output is the typed null
device (ADR-005: no real audio backend admitted yet) and there is no
video window (ADR-004: the desktop shell is a skeleton). You will not
hear or see anything; `info`, `time` and the event stream are the
observable surface. The deterministic open→play→pause→seek→stop
session is pinned by `apps/cli/tests/session_script.rs`.

Quick decode check without the REPL:

```sh
$ lumen decode-bench tests/fixtures/generated/valid/avi_mjpeg_pcm.avi
BENCH file=tests/fixtures/generated/valid/avi_mjpeg_pcm.avi bytes=425516 \
v_frames=60 v_decoded=60 a_frames=60 a_decoded=60 wall_s=0.008 decode_s=0.008
```

## 3. What plays

The formats with claimed support (implemented or partial, desktop) are
listed in `docs/feature-matrix.toml` — 37 of 73 format rows as of
2026-11-10: the AVI/MP4/MKV/Ogg/MP3/WAV/MPEG-PS/TS/ADTS/AIFF/ASF/IVF/
Annex-B demuxers, the MJPEG/AV1/VP9/(partial) H.264/HEVC/ProRes video
decoders, the PCM/MP3/MP2/AAC-LC/FLAC/Vorbis/Opus/ALAC/ADPCM audio
decoders, and the SRT/VTT/ASS/PGS/DVB/VobSub/CC subtitle parsers
(plus subtitle encoding detection — the 37th claimed row). Every
row links its evidence; per-format details and known gaps live in
`docs/formats/<record>.md`. Anything not in that list produces a typed
error (see [troubleshooting](troubleshooting.md)) — never a hang or a
silent fake success.

## 4. The UI (development preview)

```sh
cd ui && npm ci        # first clone only
npm run dev            # vite dev server on http://localhost:5173
```

A fresh profile boots into the first-run onboarding wizard
("Welcome to Lumen"); after completing it you reach the player
surface. The settings screens (`/settings/*`), the shortcuts screen
(`/keyboard-shortcuts`) and the playlist/media-info routes are real
and driven by generated data from `schemas/settings.toml` and
`keymaps/default.toml`. Playing media through the UI is not wired to
the engine yet — see the index's honest-scope note.

## 5. Where to go next

- [Settings](settings.md) — every key and its default.
- [Shortcuts](shortcuts.md) — the default keymap.
- [Troubleshooting](troubleshooting.md) — errors and their causes.
- `docs/KNOWN-LIMITS.md` — the full honest-limits register.
