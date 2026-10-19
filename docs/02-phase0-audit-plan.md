# Phase 0 — Audit Plan (baseline pinning, inventory, feasibility, ADRs)

| | |
|---|---|
| **Status** | Draft for Lead review (this is the executable plan for Phase 0) |
| **Task / agent** | 1.A2 (investigation agent) |
| **Implements** | MASTER-PROMPT §16 Phase 0; feeds §1 (parity definition), §5 (feature-matrix.toml), §14 (known limits) |
| **Baseline** | VLC 3.0.23 (desktop, released 2026-01-08 per MASTER-PROMPT §1) |
| **Not code** | Phase 0 produces documents and data only. No crates, no implementation. |

---

## 0. Purpose, scope, traceability

Phase 0 answers four questions before any engine code exists: (1) what exactly is "VLC 3.0.23" — a pinned, hashed, reproducibly observable oracle (§2); (2) what does it do — a complete, machine-readable inventory of modules, options, preferences, menus, dialogs, hotkeys and formats (§3); (3) what can a Rust-only engine honestly deliver — the feasibility report, with gaps recorded as blocked/partial, never hidden (§5, MP §14); (4) what are the three hard architectural choices — ADR-001 hardware decoding, ADR-002 WASM extension runtime, ADR-003 video-in-webview presentation (§6).

Phase 0 also seeds `docs/feature-matrix.toml` (§4), from then on the single source of parity truth: *"Parity is the share of inventory features in docs/feature-matrix.toml that are verified by automated evidence"* (MP §1).

**Out of scope:** writing any Rust/TS code; dependency *approvals* (Phase 0 records audit results and the allowlist format only, MP §9); performance benchmarking (Phase 8 executes it — Phase 0 fixes only methodology and the hardware-recording format, MP §11); VLC 4.0 delta analysis (MP §1); the program-wide wave plan (doc 13 owns that; §7 defines only Phase 0's agents). Full traceability in Appendix A.

---

## 1. Phase 0 objectives and exit gate

### 1.1 Objectives

| # | Objective | MASTER-PROMPT |
|---|---|---|
| O1 | Pin the VLC 3.0.23 oracle on Windows, Linux and macOS with hashes and signature checks | §16, §2 |
| O2 | Build the complete feature inventory from plugin/module categories, options/preferences, menus/dialogs, hotkeys and formats | §16 |
| O3 | Write the feasibility report: Rust-only parity vs reality | §16, §9 |
| O4 | Decide ADR-001 (hardware decoding) and ADR-002 (WASM runtime); decide or conditionally decide ADR-003 (video-in-webview) | §16, §2, §6, §7 |
| O5 | Freeze the `feature-matrix.toml` schema and seed it from the inventory | §1, §5 |
| O6 | Draft the test-corpus plan (fixtures + expectations + malformed generators) | §13 |
| O7 | Define and run the Phase 0 agent program (§7 of this doc) | §15 |

### 1.2 Exit gate — evidence checklist

Phase 0 is done **only** when every row below is satisfied by a committed artifact, not by a claim (MP §16: *"A phase is done only when its gate passes with recorded evidence"*). The Lead writes `docs/phase0-exit-report.md` ticking each row with links.

| # | Exit item | Artifact (repo path) | Passing condition | Owner |
|---|---|---|---|---|
| G1 | Baseline pinned and hashed | `docs/baseline/vlc-3.0.23.md` + `docs/baseline/vlc-3.0.23.json` | Per-platform: artifact URL, computed SHA-256 cross-checked against the official sidecar, signature-check result, `vlc --version` line, OS version + arch, date | 0.A1 |
| G2 | Raw inventory dumps | `docs/inventory/raw/` + `docs/inventory/raw/manifest.json` | Every dump has: verbatim command line, platform, environment, date; every command re-runnable | all inventory agents |
| G3 | Module/plugin census | `docs/inventory/modules-{windows,linux,macos}.json` + `modules-summary.md` | Name + capability + description per module; per-capability counts; cross-platform diffs annotated | 0.A3 |
| G4 | Options/preferences inventory | `docs/inventory/options-{windows,linux,macos}.json` + `options-summary.md` | Normalized option records; total counts; three-way reconciliation (--full-help vs generated vlcrc vs UI tree), discrepancies annotated | 0.A2 |
| G5 | Formats inventory | `docs/inventory/formats.json` + `formats.md` | Containers, video codecs, audio codecs, subtitle formats, access/protocols: each mapped to observed VLC module name(s) with probe evidence | 0.A4 |
| G6 | UI, menus, dialogs, hotkeys | `docs/inventory/ui-inventory.md` + `docs/inventory/hotkeys.toml` | All Qt menus/dialogs/context menus of 3.0.23 transcribed as text; every default hotkey with per-platform key sequence and VLC action name | 0.A5 |
| G7 | Feasibility report | `docs/feasibility-report.md` | Follows §5 outline; every P0/P1 codec, container and subsystem has a verdict; every gap is blocked/partial with matrix linkage | 0.A6–0.A9, consolidated by Lead |
| G8 | ADR-001 decided | `docs/adr/ADR-001-hardware-decoding.md` | Status accepted/rejected; enumerated exact OS APIs if accepted; audit-evidence links; safety-review requirements | Lead (input: 0.A6, 0.A10) |
| G9 | ADR-002 decided | `docs/adr/ADR-002-wasm-extension-runtime.md` | Runtime chosen only if it passes the dependency audit with no hidden native code; resource-limit and capability model specified | Lead (input: 0.A9) |
| G10 | ADR-003 decided or conditionally decided | `docs/adr/ADR-003-video-presentation.md` | Decided per-platform strategy, or default per platform **plus** the measurable Phase 2 prototype gate that confirms it [PROPOSAL] (MP §7 puts the prototype in Phase 2) | Lead (input: 0.A10) |
| G11 | Matrix schema frozen + seeded | `docs/feature-matrix.toml` | Parses as TOML; §4.6 rules R1–R10 pass (manual checklist until `tools/feature-audit` exists, Phase 1); every census category has rows or explicit wont-do rows; all P0 features identified | Lead |
| G12 | Corpus plan drafted | `docs/inventory/corpus-plan.md` | MP §13: legally distributable fixtures with recorded expectations; malformed/truncated/multi-track/discontinuous generators; coverage map vs matrix rows | 0.A12 |
| G13 | Exit report | `docs/phase0-exit-report.md` | Lead's gate review: each G-row ticked with artifact link; MP §17 report format (Implemented/Verified/Compatibility/Dependency/Known gaps/Risks/Next action) | Lead |

Artifacts are written by agents; **the Lead commits them** (AGENTS.md §3.8 — no agent commits during planning).

### 1.3 Non-goals (explicit)

- No source code, no Cargo workspace, no UI code; no vendoring of anything VLC (§2.1).
- No dependency approvals — only audit results and the allowlist format; use requires Phase 1+ Lead approval (MP §9).
- No performance measurements (only methodology + hardware template, MP §11).
- No Phase 1+ planning content (docs 03–15 own that).

---

## 2. Pinning VLC 3.0.23 — the test oracle

### 2.1 Clean-room ground rules (MP §2, §9, §13)

VLC is GPL. The fixed decision (MP §2) is: *"running it as an external reference oracle in tests is allowed, vendoring or copying it is not."* Phase 0 therefore:

- **Never** downloads VLC source, reads VLC source code, or copies tables/strings/option lists from it; the source tarballs on the download server are deliberately ignored.
- **Does** observe the official binaries: `--help` dumps, module lists, runtime behavior on test inputs — clean-room-safe evidence.
- **Does** use public documentation (wiki, release notes) as *secondary* specification input (MP §2); where wiki and observed behavior disagree, **observed behavior wins** and the discrepancy is annotated.
- **Never** stores VLC UI screenshots — logos, icons and window captures are VLC assets (MP §2 "no assets"); UI inventory is text transcription (§3.4). [PROPOSAL: strict reading of "no assets"; confirm with Lead.]

### 2.2 Acquisition procedure

All artifacts come from the official `get.videolan.org` tree, which publishes per-binary `.md5`, `.sha1`, `.sha256` sidecars and GPG `.asc` signatures alongside each file (verified 2026-10-19 by direct listing of `https://get.videolan.org/vlc/3.0.23/`):

| Platform | Primary artifact | URL | Notes |
|---|---|---|---|
| Windows x64 | `vlc-3.0.23-win64.exe` (installer) and `vlc-3.0.23-win64.7z` (no-install) | `https://get.videolan.org/vlc/3.0.23/win64/` | `.msi`, `.zip`, debug symbols also published; 3.0.23 also ships `winarm64/` (ARM64 support is new in this release per the release announcement [verified: videolan.org news]) |
| macOS | `vlc-3.0.23-universal.dmg` (also `-arm64.dmg`, `-intel64.dmg`) | `https://get.videolan.org/vlc/3.0.23/macosx/` | Use the universal image; record which arch actually executes on the test Mac |
| Linux | Distro package, exact version pinned | e.g. Ubuntu archive or VideoLAN apt repo | get.videolan.org publishes **no Linux binaries** [verified by directory listing]; choose one pinned distro (default: Ubuntu 24.04 LTS package) and additionally record the Flathub `org.videolan.VLC` build as secondary [VERIFY: exact package version + repo layout at execution time] |

Steps (executed by agent 0.A1 on one machine per platform):

1. Provision the reference machine; record OS name, version, arch, display server (Linux: X11/Wayland), CPU into `docs/baseline/hardware-template.md` so later measurements cite comparable hardware [PROPOSAL].
2. Download over HTTPS from `get.videolan.org` (record which mirror the redirect served); compute **our own** SHA-256 (`sha256sum`/`Get-FileHash`/`shasum -a 256`).
3. Cross-check against the official sidecar hash for that exact file.
4. Verify authenticity — Windows: Authenticode (`Get-AuthenticodeSignature` or `osslsigncode verify`); macOS: `codesign --verify --deep` + `spctl -a -t exec` on the bundle; Linux: apt GPG verification of the pinned repo/archive [VERIFY: record the exact commands and their output].
5. Install (or extract the 7z/zip to a fixed path); record `vlc --version`'s first line + build identification.
6. Write the record (§2.3).
7. Smoke-test the three observation channels used later: `vlc --list-verbose`, `vlc --full-help`, one headless open of a tiny sample file with `-I dummy` [VERIFY exact flags from `vlc --help` on the pinned binary; record the confirmed flags in the raw-dump manifest].
8. Repeat on the other two platforms.

Pre-verified data points (fetched 2026-10-19 from official sidecars; the pinning agent **must re-verify** by computing the hash of its own download):

- `vlc-3.0.23-win64.exe` — SHA-256 `20ad191348684b470ddc4e05204316f3d8e39655f412b3e392a0eef97639daaf`
- `vlc-3.0.23-universal.dmg` — SHA-256 `56ee657c3aaf5c71b4ab7d6e4f4a77f6eca54633e0bf42a93b8116eb1d1f6ec9`
- Release announcement: `https://www.videolan.org/vlc/releases/3.0.23.html` (page exists [verified]; the 3.0.x branch page `https://www.videolan.org/vlc/releases/` indexes it)

### 2.3 The pin record

`docs/baseline/vlc-3.0.23.md` (human) and `vlc-3.0.23.json` (machine, consumed later by `tools/parity-report` / `tools/bench-vs-vlc`) contain, per platform:

```json
{
  "platform": "windows",
  "os": "Windows 11 23H2 x64",          // as recorded on the machine
  "artifact_url": "https://get.videolan.org/vlc/3.0.23/win64/vlc-3.0.23-win64.exe",
  "mirror_served": "…",                  // if redirected
  "sha256_computed": "…",                // computed by us
  "sha256_official_sidecar": "…",        // from get.videolan.org .sha256
  "signature_check": {"tool": "Get-AuthenticodeSignature", "result": "Valid", "signer": "…"},
  "version_string": "VLC media player 3.0.23 …",   // verbatim first line of vlc --version
  "recorded": "2026-XX-XX", "recorded_by": "0.A1",
  "machine": {"cpu": "…", "display": "n/a", "notes": "…"}
}
```

Plus a `sources` section listing every public documentation page Phase 0 relies on (wiki URLs), and a `provenance` statement: binaries are never committed; they live only on reference/CI machines and are re-downloadable from this record.

### 2.4 Oracle invocation contract (differential testing, MP §13)

Later phases (Phase 2+ differential tests, Phase 8 benchmarks) invoke the pinned binary only through these recorded modes, so the harness (`tools/bench-vs-vlc`, `tools/parity-report`) has a stable contract from day one:

- **M1 — headless metadata & stream info.** Non-interactive run, log parsed: `vlc -I dummy --verbose 2 --play-and-exit --no-video --no-audio <input>`; the log's input/demux/decoder lines are the observed source for track lists, durations and chosen modules. [VERIFY: exact log grammar and flags on 3.0.23; alternative: the rc interface (`vlc -I rc`; `info`, `status`, `get_time`, `length`, `is_playing`) — confirm which is stable on all three platforms and pin the choice in the harness.]
- **M2 — deterministic decoded output.** Produce a comparable decoded artifact via VLC's stream-output chain (transcode to fixed, lossless-tolerant parameters into a stable container); compare Lumen's decode with tolerance metrics (per-frame PSNR/SSIM for video; sample-wise RMS for audio). Byte-equality is never claimed. [VERIFY: exact `--sout` chain + determinism on 3.0.23 — measure in Phase 2 before parity tests depend on it.]
- **M3 — error behavior on malformed inputs.** Run with `--play-and-exit`; record exit code + user-visible error for every malformed corpus case; feeds the per-format corrupt-input records (MP §8).
- **M4 — timing.** Repeated runs, same files and hardware (startup, probe latency, time to first frame/sound, seek latency), median + p95, recorded against `docs/baseline/hardware-template.md`. Phase 8 executes; the protocol is fixed now (MP §11).
- **Guard.** Every invocation first asserts the version string matches the pin record (hash where we control the artifact, e.g. the extracted 7z/zip); drift aborts the run — a drifted oracle invalidates evidence.
- **Sandboxing.** The oracle runs untrusted media with a large native attack surface: disposable VM/container per corpus run, no network egress except test-local servers, no credentials (0.A11). [VERIFY: isolation approach for the CI fleet.]

### 2.5 Lifecycle and drift

- VLC 3.0.24 has since been published (a security update; per videolan.org news it upgrades FFmpeg 4.4 → 8.1.2 and updates ~49 bundled libraries) [verified: `https://www.videolan.org/news.html`]. **The baseline stays 3.0.23** — MP §1 pins it; re-pinning is a Lead decision requiring a new pin record. 3.0.24's existence is noted as a mirror-drift risk.
- Mirrors may rotate: the record's `artifact_url` plus both hashes make any future re-download verifiable.
- Re-check the pin at each phase gate (recompute hash, re-run `vlc --version`) and append the result. If an artifact disappears from all mirrors, the Lead decides the response; vendoring into the repo remains forbidden regardless.
- The oracle is installed on CI/reference machines only, never committed, never linked against.

---

## 3. VLC feature inventory methodology

### 3.1 Evidence sources, ranked by clean-room safety

| Rank | Source | Use | Caveat |
|---|---|---|---|
| S1 | Observed behavior of the pinned binary (help dumps, module lists, probing test files, log output) | Primary for everything | Always recorded verbatim under `docs/inventory/raw/` with a command manifest |
| S2 | Public specifications: VideoLAN wiki and release notes (e.g. `wiki.videolan.org/VLC_Features_Formats/`, `wiki.videolan.org/QtHotkeys/`, `wiki.videolan.org/Preferences/`, `wiki.videolan.org/Documentation:Modules/` — all confirmed live) | Secondary; cross-check and naming | Community-maintained; may lag 3.0.23 — discrepancies annotated, S1 wins |
| S3 | Our own design package (`design/figma/`) | Target-side mapping only (what *we* will build), never a claim about VLC | — |
| ✗ | VLC source code, forum posts quoting source, decompilation | **Forbidden** | Clean-room rule, MP §2 |

**Reproducibility rule:** every artifact is regenerable: each dump in `docs/inventory/raw/` has a `raw/manifest.json` entry (platform, exact command, environment/`vlc --version` line, date, agent). A re-run must yield equivalent output; textual diffs must be platform-explainable.

### 3.2 Module/plugin census (MP §16: "plugin categories")

VLC functionality ships as modules, each declaring a **capability** (the plugin category), enumerated from the binary, never from source:

1. Run `vlc --list-verbose` on each pinned platform [VERIFY: exact flag names on 3.0.23 — confirm via `vlc --help` on the pinned binary; `vlc --list` is the short form]. This prints each module with its capability and a one-line description.
2. Parse into `modules-{platform}.json`: `{name, capability, description, platforms: [...], options: <count, filled later>}`.
3. Group and count by capability. Expected capability set (indicative only — **the dump is the authority**; [VERIFY] each against the census): `access`, `access_demux`, `demux`, `decoder`, `packetizer`, `encoder`, `audio output`, `audio filter`, `audio mixer`, `audio resampler`, `video output`, `video filter`, `video splitter`, `video chroma`, `sub filter`/`subsource`, `text renderer`, `stream_filter`, `stream output`, `services_discovery`, `meta fetcher`/`meta art finder`, `interface`, `playlist export`, plus misc host services (inhibit, wallpaper, vout window, …).
4. Per-module option dump: for each module, `vlc -p <module> --advanced --help-verbose` [VERIFY flag spelling], stored under `raw/module-options/<name>.txt`. This feeds §3.3 and the per-format records (MP §8).
5. Annotate platform-only modules (e.g. `directsound`/`wasapi` vs `coreaudio` vs `alsa`/`pulse`; `dshow` vs `avfoundation` vs `v4l2`; `direct3d11` vs `macosx`/`opengl` vouts) — they become platform-scoped matrix rows.

Expected volume: several hundred modules per platform [VERIFY exact counts]. `modules-summary.md` states measured counts per platform and per capability.

### 3.3 Options and preferences tree (MP §5: "thousands of options")

Three independent extractions, reconciled:

1. **Full help dump.** `vlc --full-help` / `vlc --longhelp` with `--advanced`, per platform [VERIFY flags on 3.0.23]. Parse into normalized records: `{name, short, type, default, range/choices, description, advanced?, module}`.
2. **Preferences file walk.** Run VLC once headless to generate the preferences file (`~/.config/vlc/vlcrc` Linux, `%APPDATA%\vlc\vlcrc` Windows, `~/Library/Preferences/org.videolan.vlc/vlcrc` macOS [VERIFY paths]); it enumerates all options with section headers, including ones hidden from the simple UI. Parse and store under `raw/`.
3. **Preferences UI tree.** Transcribe the Qt advanced-preferences tree (top-level groups ≈ Audio, Video, Input/Codecs, Subtitles/OSD, Interface, Playlist, Hotkeys [VERIFY exact 3.0.23 structure]) as a text tree, cross-checked against `wiki.videolan.org/Preferences/` (live page). Nodes map to option sets and feed the settings-schema design (doc 07 consumes this).

Reconciliation (`options-summary.md`): per-platform totals; set-differences across the three extractions, each annotated (e.g. "advanced-only, absent from simple UI"); unexplained differences are Phase 0 defects.

**Matrix scoping rule** [PROPOSAL]: individual options are *not* matrix rows (thousands; they belong to the Phase 1 settings schema, inventoried here). The matrix tracks user-visible capabilities — formats, protocols, outputs, filters, dialogs/screens, hotkeys, device classes, plugin kinds; a preference panel is one `ui` row.

### 3.4 Menus and dialogs

Method (agent 0.A5, on each platform, in the reference VM):

1. Transcribe the Qt menu bar and all submenus as text (3.0.x ≈ Media, Playback, Audio, Video, Subtitle, Tools, View, Help [VERIFY against the pinned build; record exact labels]).
2. Enumerate every reachable dialog (open file/disc/network/capture, media info, codec info, messages, effects and filters, track synchronization, bookmarks, preferences, customize interface, plugins/extensions, …): menu path, shortcut, purpose, contained controls (transcribed), implied engine capabilities. [VERIFY each label and shortcut against the binary; do not rely on memory.]
3. Transcribe context menus (player, playlist, systray) and modes (fullscreen controller, minimal view, Skins2 — recorded as an `interface` capability, expected `unsupported` for us by design).
4. Map each dialog to the nearest `design/figma/` screen and a matrix `ui` row — the VLC↔Lumen crosswalk used by doc 08.
5. No screenshots (§2.1). Text, tables and our own annotations only.

### 3.5 Hotkeys and default keybindings

1. Extract the hotkeys section printed by `vlc --full-help` on each platform [VERIFY it is included there on 3.0.23]: VLC action name + default key sequence.
2. Cross-check with the Qt preferences hotkeys pane (transcribed) and `wiki.videolan.org/QtHotkeys/` (live page). Note macOS Cmd/Option variants [VERIFY per platform].
3. Emit `docs/inventory/hotkeys.toml`: `{action, vlc_action_name, default_keys per platform, lumen_command_id (doc 05 fills)}`. Safe examples (all [VERIFY] on the pinned build): Space play/pause, F fullscreen, Esc exit fullscreen, M mute, S stop, N/P next/previous, arrows volume/seek, modifier+arrows longer seeks, b/v cycle audio/subtitle track.

### 3.6 Formats: containers, codecs, subtitles, protocols, devices

The formats list is **derived, not assumed**:

1. Start from the census: demux-capability modules ⇒ container candidates; decoder-capability modules ⇒ codec candidates; text-render/sub/spu modules ⇒ subtitle pipeline; access/access_demux modules ⇒ protocols and device classes (http(s), ftp, mms, rtsp/rtp/udp, smb, sftp, nfs, cdda, dvdnav/dvdread, bluray, dshow/avfoundation/v4l2, screen, …) [VERIFY each against census].
2. **Probe-validate**: open a small reference file with the oracle (M1) and confirm from the log which demuxer/decoder engaged. No test artifact ⇒ listed "claimed by module, not probe-verified" — never silently asserted.
3. Reconcile against `wiki.videolan.org/VLC_Features_Formats/` (live page); annotate discrepancies.
4. Emit `formats.json` (machine-readable; feeds matrix seeding): `{kind, name, vlc_modules, probe_evidence, notes}`. The MP §8 per-format records (parse/decode/seek/limits/corrupt behavior) start here and grow per phase.

### 3.7 Cross-platform reconciliation

Censuses differ per platform (platform modules; possibly version-skewed Linux distro builds). Every difference is either a known platform-specific module (annotated; platform-scoped matrix row) or unexplained — a Phase 0 defect. Reconciliation tables live in each `*-summary.md`; arch nuances recorded too (macOS universal binary; Windows ARM64 exists for 3.0.23 [verified via directory listing]).

### 3.8 Expected volume and effort

Planning figures, replaced by measured counts at exit [VERIFY all]: hundreds of modules; over a thousand options; dozens of hotkey actions and dialogs; ~100 format families. Hence the parallelized agents (§7) and normalized JSON before prose summaries.

---

## 4. `feature-matrix.toml` schema

### 4.1 Role

`docs/feature-matrix.toml` is the parity ledger (MP §1). CI validates it via `tools/feature-audit` (built in Phase 1; §4.6 is a manual checklist until then). It couples to config (MP §5): **feature exposure lists only implementations that exist; config can never enable a missing codec, protocol or hardware capability** — enforced jointly with `tools/config-validator`.

### 4.2 File layout and field reference

```toml
[meta]
schema_version = 1
baseline = "vlc-3.0.23"
last_updated = "2026-XX-XX"

[[feature]]
id = "container-matroska"
category = "container"
vlc_name = "Matroska / WebM"
description = "Demux Matroska and WebM containers (video/audio/subtitle tracks, chapters, attachments, seeking)"
priority = "P0"
phase_target = 5
vlc_modules = ["avformat"]          # observed module names from the census [VERIFY]
inventory_ref = "inventory/formats.json#matroska"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }
evidence = ""                       # repo path or CI-run URL; required iff any status is "verified"
blocked_reason = ""                 # required iff any status is "blocked" or "unsupported"
notes = ""                          # required iff any status is "partial"
```

| Field | Type | Required | Rules |
|---|---|---|---|
| `id` | string | always | Unique; `^[a-z0-9]+(-[a-z0-9]+)+$`; prefix must match category (R1) |
| `category` | enum | always | `container`, `video-codec`, `audio-codec`, `subtitle-codec`, `protocol`, `access`, `device`, `audio-output`, `video-output`, `video-filter`, `audio-filter`, `text-render`, `services-discovery`, `playlist-format`, `ui`, `hotkey`, `extension` |
| `vlc_name` | string | for VLC-derived rows | Observed user-facing VLC feature name; empty for Lumen-only rows |
| `description` | string | always | One line, factual |
| `priority` | `P0..P3` | always | Usage-share rubric (§4.4) |
| `phase_target` | int | always | 2–8, consistent with MP §16 roadmap; P0 features must target ≤ 6 [PROPOSAL] |
| `vlc_modules` | [string] | optional | Observed implementing module names (census); provisional values need census confirmation |
| `inventory_ref` | string | for VLC-derived rows | Pointer to the backing `docs/inventory/` artifact (anti-hallucination, R10) |
| `status` | inline table | always | Exactly the keys `windows`, `linux`, `macos`, `web` (dedicated web column, MP §10); values per §4.3 |
| `evidence` | string | iff any status = `verified` | Existing artifact recording commands + results, or our-CI run URL [PROPOSAL] |
| `blocked_reason` | string | iff any status = `blocked`/`unsupported` | Cites a feasibility-report §, an ADR, or a risk-register entry |
| `notes` | string | iff any status = `partial` | What subset works and what does not |

Category→id-prefix map (R1): `container-`, `codec-video-`, `codec-audio-`, `subtitle-`, `protocol-`, `access-`, `device-`, `aout-`, `vout-`, `vfilter-`, `afilter-`, `textrender-`, `sd-`, `plfmt-`, `ui-`, `hotkey-`, `ext-`.

### 4.3 Status model (MP §17 + §10)

Per-platform values: `not_started`, `in_progress`, `implemented`, `verified`, `partial`, `unsupported`, `blocked`.

- `verified` — MP §17: specified, complete incl. error paths, tested on each **claimed** platform (all except `unsupported` ones), documented limits, non-regressing. Requires `evidence`.
- `implemented` — code complete on that platform, evidence not yet complete.
- `partial` — a documented subset works (e.g. decode without encode; some profiles). Requires `notes`; P0 rows may not stay partial past their target phase without Lead sign-off [PROPOSAL].
- `unsupported` — intentionally out of scope on that platform (browser limits, by-design replacements). Requires `blocked_reason`.
- `blocked` — external blocker (dependency, hardware, legal). `blocked_reason` must cite investigation, alternatives considered, next step (MP §17).

**Rollup** (computed by `tools/feature-audit`, never stored): `verified` overall iff verified on all claimed platforms, else the weakest claimed status; `blocked`/`unsupported` counted explicitly; parity reported raw and P0-only [PROPOSAL]. **Transitions are audited**: `not_started → verified` jumps are rejected (anti-fabrication, MP §15 "agents never mark their own work verified" — the tool decides from evidence).

### 4.4 Priority rubric [PROPOSAL]

| Priority | Definition | Examples |
|---|---|---|
| P0 | ≥ ~5% of expected real-world usage; breaks the "usable player" claim if missing | mp4, matroska, h.264, aac, mp3, srt, http(s), basic playback UI + core hotkeys |
| P1 | ~0.5–5% of usage; commonly expected | flac, opus, vorbis, hls, ass subtitles, deinterlace, media-info dialog |
| P2 | < ~0.5%; requested but rare | av1, rtsp, upnp discovery, legacy containers |
| P3 | Legacy, deprecated, exotic, or by-design divergent | dvd navigation, skins2, Lua extensions (replaced by WASM) |

VLC publishes no per-format usage telemetry [VERIFY]; priorities therefore derive from public format-popularity studies, corpus design and forum/wiki signal — judgment calls recorded per row. Re-rating is a Lead edit with justification; the thresholds are a [PROPOSAL] pending Lead confirmation.

### 4.5 Seed rows (examples; statuses are the expected Phase 0-exit initial state)

```toml
# Containers
[[feature]]
id = "container-matroska"
category = "container"
vlc_name = "Matroska / WebM"
description = "Demux Matroska/WebM: tracks, chapters, attachments, cues, seeking"
priority = "P0"
phase_target = 5
vlc_modules = ["avformat"]                    # [VERIFY vs census]
inventory_ref = "inventory/formats.json#matroska"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

[[feature]]
id = "container-mp4"
category = "container"
vlc_name = "MP4 / MOV / 3GP"
description = "Demux ISO-BMFF family incl. fragmented mp4 and quicktime variants"
priority = "P0"
phase_target = 5
vlc_modules = ["mp4", "avformat"]             # [VERIFY priority order vs census]
inventory_ref = "inventory/formats.json#mp4"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

[[feature]]
id = "container-mpeg-ts"
category = "container"
vlc_name = "MPEG transport stream"
description = "Demux MPEG-TS (program/map tables, multiple programs, teletext cues)"
priority = "P1"
phase_target = 5
vlc_modules = ["ts"]                           # [VERIFY]
inventory_ref = "inventory/formats.json#mpeg-ts"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

# Codecs — video
[[feature]]
id = "codec-video-h264"
category = "video-codec"
vlc_name = "H.264/AVC"
description = "Decode H.264 (AVC1) incl. baseline/main/high profiles; feeds ADR-001 (software vs OS decoders)"
priority = "P0"
phase_target = 5
vlc_modules = ["avcodec"]                      # [VERIFY]
inventory_ref = "inventory/formats.json#h264"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }
notes = "Pure-Rust production decoder availability unproven — see feasibility report; outcome depends on ADR-001"

[[feature]]
id = "codec-video-hevc"
category = "video-codec"
vlc_name = "HEVC/H.265"
description = "Decode HEVC incl. 10-bit; hardware-decode candidate (ADR-001)"
priority = "P0"
phase_target = 5
vlc_modules = ["avcodec"]                      # [VERIFY]
inventory_ref = "inventory/formats.json#hevc"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }
notes = "Same caveat as h264"

[[feature]]
id = "codec-video-av1"
category = "video-codec"
vlc_name = "AV1"
description = "Decode AV1 (VLC 3.0.x ships dav1d [VERIFY])"
priority = "P2"
phase_target = 7
vlc_modules = ["dav1d"]                        # [VERIFY]
inventory_ref = "inventory/formats.json#av1"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

# Codecs — audio
[[feature]]
id = "codec-audio-aac"
category = "audio-codec"
vlc_name = "AAC (ADTS/MP4)"
description = "Decode AAC LC/HE incl. in-container and raw ADTS"
priority = "P0"
phase_target = 5
vlc_modules = ["avcodec"]                      # [VERIFY: VLC 3.0 AAC decoder module set]
inventory_ref = "inventory/formats.json#aac"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

[[feature]]
id = "codec-audio-mp3"
category = "audio-codec"
vlc_name = "MP3"
description = "Decode MPEG layer I/II/III audio"
priority = "P0"
phase_target = 5
vlc_modules = ["mpg123", "avcodec"]            # [VERIFY]
inventory_ref = "inventory/formats.json#mp3"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

[[feature]]
id = "codec-audio-flac"
category = "audio-codec"
vlc_name = "FLAC"
description = "Decode FLAC lossless audio"
priority = "P1"
phase_target = 5
vlc_modules = ["flac", "avcodec"]              # [VERIFY]
inventory_ref = "inventory/formats.json#flac"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

[[feature]]
id = "codec-audio-opus"
category = "audio-codec"
vlc_name = "Opus"
description = "Decode Opus (incl. ogg/matroska/webm mappings)"
priority = "P1"
phase_target = 5
vlc_modules = ["opus"]                         # [VERIFY]
inventory_ref = "inventory/formats.json#opus"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

# Subtitles
[[feature]]
id = "subtitle-srt"
category = "subtitle-codec"
vlc_name = "SubRip (.srt)"
description = "Parse and render SubRip subtitles incl. basic styling tags"
priority = "P0"
phase_target = 5
vlc_modules = ["subtitle", "subsdec"]          # [VERIFY]
inventory_ref = "inventory/formats.json#srt"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

[[feature]]
id = "subtitle-ass"
category = "subtitle-codec"
vlc_name = "ASS/SSA"
description = "Parse and typeset ASS/SSA (positioning, karaoke, fonts); pure-Rust renderer unproven — libass is C"
priority = "P1"
phase_target = 5
vlc_modules = ["ass"]                          # [VERIFY]
inventory_ref = "inventory/formats.json#ass"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }
notes = "Parse in Rust is feasible; full ASS typesetting parity is a known-gap candidate (feasibility report)"

# Network / access
[[feature]]
id = "protocol-http-https"
category = "protocol"
vlc_name = "HTTP/HTTPS progressive streams"
description = "Open http(s) URLs: range requests, redirects, cookies, TLS verification"
priority = "P0"
phase_target = 6
vlc_modules = ["http", "https"]                # [VERIFY]
inventory_ref = "inventory/formats.json#http"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

[[feature]]
id = "protocol-hls"
category = "protocol"
vlc_name = "HTTP Live Streaming"
description = "HLS playback: playlists, segment buffering, variant selection, live edges"
priority = "P1"
phase_target = 6
vlc_modules = ["adaptive"]                     # [VERIFY]
inventory_ref = "inventory/formats.json#hls"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

[[feature]]
id = "protocol-rtsp"
category = "protocol"
vlc_name = "RTSP/RTP streaming"
description = "RTSP session control with RTP/RTCP media transport"
priority = "P2"
phase_target = 6
vlc_modules = []                               # [VERIFY vs census: realrtsp/live555 module set]
inventory_ref = "inventory/formats.json#rtsp"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "unsupported" }
blocked_reason = "web: browsers cannot open raw RTSP/RTP sockets (MP §10)"

[[feature]]
id = "protocol-udp-rtp"
category = "protocol"
vlc_name = "UDP/RTP unicast and multicast"
description = "Raw UDP/RTP media reception"
priority = "P2"
phase_target = 6
vlc_modules = ["udp", "rtp"]                   # [VERIFY]
inventory_ref = "inventory/formats.json#udp"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "unsupported" }
blocked_reason = "web: no raw UDP sockets in browsers (MP §10)"

# UI / hotkeys / filters
[[feature]]
id = "ui-playlist"
category = "ui"
vlc_name = "Playlist window/panel"
description = "Queue with reorder, sorting, search, save/load, drag-and-drop; maps to design/figma playlist screens"
priority = "P0"
phase_target = 4
inventory_ref = "inventory/ui-inventory.md#playlist"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

[[feature]]
id = "hotkey-space-play-pause"
category = "hotkey"
vlc_name = "Play/Pause (Space)"
description = "Default keybinding: Space toggles play/pause"
priority = "P0"
phase_target = 4
inventory_ref = "inventory/hotkeys.toml#play-pause"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

[[feature]]
id = "vfilter-deinterlace"
category = "video-filter"
vlc_name = "Deinterlacing"
description = "Deinterlace modes (blend, bob, yadif-class, etc.) selectable at runtime"
priority = "P1"
phase_target = 5
vlc_modules = ["deinterlace"]                  # [VERIFY]
inventory_ref = "inventory/formats.json#deinterlace"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }

# Platform-specific output example
[[feature]]
id = "aout-directsound"
category = "audio-output"
vlc_name = "DirectSound audio output"
description = "Windows audio output path (OS-service binding; exclusive/shared modes)"
priority = "P0"
phase_target = 7
vlc_modules = ["directsound"]                  # [VERIFY]
inventory_ref = "inventory/modules-windows.json#directsound"
status = { windows = "not_started", linux = "unsupported", macos = "unsupported", web = "unsupported" }
blocked_reason = "linux/macos/web: DirectSound is Windows-only; each platform has its own aout row"

# Devices / extensions — gap examples recorded, not hidden (MP §14)
[[feature]]
id = "device-dvd"
category = "device"
vlc_name = "DVD playback (menus)"
description = "DVD-Video navigation: menus, angles, titles; depends on nav library + legal review"
priority = "P3"
phase_target = 7
vlc_modules = ["dvdnav", "dvdread"]            # [VERIFY]
inventory_ref = "inventory/formats.json#dvd"
status = { windows = "blocked", linux = "blocked", macos = "blocked", web = "unsupported" }
blocked_reason = "CSS/menu navigation needs C libraries (libdvdnav/libdvdcss) + legal questions — MP §14; decide by ADR (feasibility report §7)"

[[feature]]
id = "ext-lua-extensions"
category = "extension"
vlc_name = "Lua extensions / playlist scripts"
description = "VLC Lua scripting; replaced by sandboxed WASM extension plugins per MP §14"
priority = "P3"
phase_target = 7
vlc_modules = ["lua"]                          # [VERIFY]
inventory_ref = "inventory/modules-linux.json#lua"
status = { windows = "unsupported", linux = "unsupported", macos = "unsupported", web = "unsupported" }
blocked_reason = "by design: WASM plugin model replaces Lua (MP §14); existing scripts will not run"
```

These 23 rows are **seed examples**: their `vlc_modules` values are provisional [VERIFY] and must be confirmed or corrected from the census before G11 passes; statuses will be updated only through the §4.3 transition rules as phases progress.

### 4.6 Validation rules (enforced by `tools/feature-audit`; manual checklist in Phase 0)

- **R1** `id` unique, regex-valid, prefix matches category.
- **R2** `category`/`priority`/`phase_target` in range; P0 ⇒ `phase_target ≤ 6` [PROPOSAL].
- **R3** `status` has exactly `windows`,`linux`,`macos`,`web`; values valid.
- **R4** Any `verified` ⇒ `evidence` resolves to an existing artifact recording commands + actual results (or our-CI URL [PROPOSAL]); mock evidence must be labeled mock (MP §13).
- **R5** Any `blocked`/`unsupported` ⇒ `blocked_reason` non-empty and referencing an existing doc section/ADR/risk entry.
- **R6** Any `partial` ⇒ `notes` non-empty describing the working subset.
- **R7** Config cross-check (with `tools/config-validator`): product feature-exposure lists, profiles, keymaps and layouts reference only existing matrix `id`s with a non-`not_started` implementation on that platform (MP §5).
- **R8** Inventory completeness: everything discovered in §3 has a matrix row — including explicit wont-do rows; per-category counts reconcile with the inventory; no silent omissions.
- **R9** File parses as TOML; `meta.schema_version` present; CI gate merges blocked on failure (MP §13).
- **R10** Anti-hallucination: every VLC-derived row carries `inventory_ref` + `vlc_name` tracing to inventory; no invented VLC features.

### 4.7 Change control and freezing

At Phase 0 exit the schema is **frozen**: later changes need Lead approval, a `schema_version` bump and a migration note. Rows are append-mostly; status changes require new evidence (R4) and are visible in git history. Parity percentages are computed, never stored.

---

## 5. Feasibility report structure (`docs/feasibility-report.md`)

Answers one question honestly: **what can a 100%-Rust engine deliver, and where must parity be blocked, partial or exception-based?** Consolidates 0.A6–0.A9 (§7); crate-level detail lives in doc 04. Required outline:

1. **Executive summary** — features inventoried; share of P0 with a compliant pure-Rust path today; blocked/partial counts with one-line reasons; the explicit statement that full VLC parity is **not** achievable within the fixed decisions, with the response to each gap (ADR-001 exception, WASM-for-Lua, blocked register). No claims beyond evidence (MP §1).
2. **Method** — the crate-investigation protocol for every verdict: transitive graph (`cargo tree`), build-script/native-code inspection, cargo-deny/vet-style checks, license, maintenance, security record, provenance; every claim labeled "audited" or "asserted" (asserted ⇒ [VERIFY] + owner). A Rust API proves nothing about purity (MP §9).
3. **Video codecs** — per codec: pure-Rust decoder availability (candidate, completeness, conformance evidence, license), verdict `available|partial|absent`, consequence (Rust path / ADR-001 exception / blocked). Headline gaps stated plainly: **H.264/HEVC software decode** — no production-grade pure-Rust decoder known [VERIFY in doc 04]; dav1d (AV1) is C [VERIFY]; VLC leans on bundled libavcodec for most codecs [VERIFY census].
4. **Audio codecs** — same shape (mp3, aac, flac, vorbis, opus, ac3/dts, …). Several solid pure-Rust paths expected (Symphonia-family [VERIFY per-codec maturity]); completeness varies; AAC is a watch item [VERIFY].
5. **Subtitles/text** — parsing mostly feasible in Rust; full ASS/SSA typesetting parity is a gap candidate (libass is C [VERIFY]); pure-Rust font-stack options and limits.
6. **Subsystems** — demuxers (mkv/mp4/ts/ogg/wav/…), network/streaming (TLS purity, HLS/DASH parts), per-OS audio outputs (cpal-style OS-ABI bindings [VERIFY purity]), renderer (wgpu), filters, playlists, services discovery (UPnP [VERIFY]), transcode/stream-out, devices. Each: availability, risk, recommended phase.
7. **Known-gap register (MP §14 — recorded, not hidden)** — per limit, mapped to matrix statuses: (a) Lua extensions/playlist scripts — unsupported by design, WASM replaces them, existing scripts will not run; (b) DVD/Blu-ray — CSS/menus need C libraries + legal review, ADR decides; (c) DVB/capture — unverifiable without hardware, blocked with a named requirement; (d) codec patents — separate from source licenses (doc 11); (e) H.264/HEVC — pending audit + ADR-001. Each entry: reason, investigation, alternatives, next step (MP §17).
8. **Web/WASM reality** — what the web build cannot do (raw UDP/RTSP, discs, capture; threaded WASM needs COOP/COEP — MP §10) and browser-API replacements (WebCodecs as web-side hardware decode [PROPOSAL], pending ADR-001 scope).
9. **ADR inputs** — the facts ADR-001/002/003 need, with pointers to the producing sections.
10. **Confidence and [VERIFY] ledger** — everything unconfirmed, with owners and deadlines; nothing silently asserted.

**Acceptance:** every P0/P1 codec, container and subsystem has a verdict row; every `absent` names ≥2 checked sources; the gap register reconciles 1:1 with `blocked`/`partial` matrix rows (R8).

---

## 6. Phase 0 ADRs — decision briefs

Briefs, not decisions: each ADR file (`docs/adr/ADR-00X-….md`) records Context, Options, Decision, Consequences, Evidence, entering Phase 0 as `proposed`. Exit requires decided ADR-001/002; ADR-003 may end "conditionally decided" with a pre-registered Phase 2 gate [PROPOSAL].

### 6.1 ADR-001 — Hardware decoding (MP §2 fixed decision, §9 exceptions, §11 performance, §14)

**Context.** MP §2: software codecs in Rust; OS decoders (VideoToolbox, Media Foundation, VA-API, …) only after audit, as a documented OS-ABI exception, with truthful reporting of when active. The web build has no OS decoders — whether WebCodecs is its analog in v1 is part of this ADR [PROPOSAL].

**Question.** Adopt OS hardware decoders behind the `codec-api` trait — which codecs, which platforms, what fallback and reporting — or stay pure-software and accept blocked/slow H.264/HEVC-class outcomes?

**For:** (1) the audit is expected to find no production-grade pure-Rust H.264/HEVC decoders [VERIFY — the deciding input]; without OS decoders the most common P0 codecs are blocked, gutting parity; (2) MP §9 enumerates hardware decoders as an OS-service exception category; (3) MP §11 admits pure-Rust decoders may start slower and wants hardware decode at high resolutions; (4) OS decoders shift patent exposure to the OS vendor [VERIFY — legal, doc 11].

**Against:** (1) bigger audit surface — each API needs an exact ADR-listed set + safety review; (2) platform divergence vs one Rust path, plus a fourth web behavior; (3) VA-API is reached through **libva, a system C library**, not a kernel ABI — the ADR must decide whether "OS-service binding" covers that, or Linux hardware decode stays blocked (VideoToolbox/Media Foundation are direct framework calls) [VERIFY]; (4) "all-Rust" misdescription risk — forbidden (MP §2); truthful reporting enforced in UI, logs, matrix evidence.

**Recommendation framing [PROPOSAL].** Accept narrowly: only behind `codec-api`; Rust software path preferred where one exists, sole path where none does; only ADR-enumerated APIs (expected: VideoToolbox `VTDecompressionSession`; Media Foundation transforms / D3D11 video; VA-API if the libva question resolves [VERIFY each]); default-off until verified per platform, then default-on only for codecs without a compliant software path; UI/logs/reporting always name the active decoder.

**Audit must verify before deciding:** per-OS API availability across supported OS versions; Rust binding options (`-sys` crates vs our own minimal ABI declarations) + unsafe surface; frame-format/HDR negotiation; no-hardware behavior (truthful fallback or error, never silent failure); oracle behavior notes (M1); legal input (doc 11); the web story (WebCodecs in/out of v1).

### 6.2 ADR-002 — WASM runtime for extension plugins (MP §6)

**Context.** MP §6: built-in compile-time plugins plus sandboxed WASM extension plugins in v1; the runtime must pass the dependency audit with **no hidden native code** (MP §6, §9).

**Candidates:** `wasmtime` (Rust compiler runtime; fuel metering, epoch interruption, store limiters [VERIFY names + purity via full `cargo tree`]); `wasmi` (pure-Rust interpreter; smaller/slower, likely fine for UI panels/metadata); layered frameworks (e.g. Extism [VERIFY]). Disqualified outright: `wasm3` (C), V8/JS engines (C++), anything with a C core.

**Hard requirements (pass/fail):** clean dependency audit (no C/C++ sources, no C-compiling build scripts, no hidden transitive native code); `unsafe` only in approved adapters with documented invariants; **resource limits** — per-instance memory cap, CPU metering (fuel/epoch deadline) + wall-clock deadlines, bounded I/O, no ambient filesystem/network; **capability model** — deny-by-default host functions granted per plugin-manifest permission (e.g. `ui-panel`, `playlist-read`, `metadata-read`, `net-fetch:<origin>`) [PROPOSAL — final names in doc 06]; crash isolation (a trapping plugin becomes an error event, playback continues); API versioning and compatibility contract; install/remove with provenance and trust checks (hash pinning minimum; signatures if feasible [PROPOSAL]).

**Decision criteria:** audit pass; embed size; per-instance memory; throughput on representative workloads (UI panel, playlist handler, light filter); wasm32 host-target support; maintenance/license (permissive expected [VERIFY]); determinism for tests; conformance-kit cost.

**Open sub-questions:** can the web build host WASM plugins in v1 (WASM-in-WASM, browser restrictions) [PROPOSAL: defer to doc 09]; default vs fallback runtime.

### 6.3 ADR-003 — Video-in-webview presentation (MP §7)

**Context.** The desktop UI is React in the OS webview (Tauri); video must reach the screen. MP §7 fixes the two candidates and the risk ranking: Linux webviews are the highest risk; WebView2, WKWebView and WebKitGTK must be tested **separately**; the prototype belongs to Phase 2.

**Option A — native wgpu surface behind a transparent webview.** Engine renders via wgpu to a native surface; the webview overlays with transparency for controls. Pros: no per-frame copies, best latency/perf path, full GPU control. Cons: per-platform transparency/compositing quirks, z-order and input routing, fullscreen/multi-monitor/DPI edge cases; window geometry must track the webview exactly.

**Option B — frame streaming to a canvas.** Engine ships frames into the webview (IPC or shared-memory paths); the UI paints them to canvas/WebGL/WebGPU. Pros: one rendering path much closer to the web build; simpler layering; controls are ordinary DOM. Cons: copy/IPC bandwidth and frame pacing, CPU/GPU cost at 4K60, potential tearing/color-management limits.

**Option C [PROPOSAL] — per-platform default behind one renderer abstraction.** The renderer interface hides the strategy; each platform's default comes from measurements and is config-switchable; divergence is documented, not hidden.

**Pre-registered test matrix (decision evidence, executed in Phase 2):** {WebView2/Windows, WKWebView/macOS, WebKitGTK/Linux-X11, WebKitGTK/Linux-Wayland} × {1080p60, 4K60} × {windowed resize, fullscreen, DPI scaling, Linux compositor variation [VERIFY: GNOME/KDE Wayland]}. Metrics: sustained fps, dropped-frame %, latency proxy, CPU/GPU/power, visual artifacts.

**Audit must verify before the prototype:** current Tauri's transparent-window and raw-window-handle support per platform [VERIFY]; wgpu surface creation from the webview's native window per platform [VERIFY]; known WebKitGTK transparency/Wayland compositing issues [VERIFY current state]. **Allowed exit status:** "conditionally decided — default strategy per platform + measurable gate the Phase 2 prototype must pass" [PROPOSAL].

---

## 7. Phase 0 agent breakdown

Twelve self-contained agent tasks (one deliverable, one owner each). Agents write files, never commit, append worklog entries, mark [PROPOSAL]/[VERIFY] (AGENTS.md). The program-wide wave plan is doc 13's; this table is Phase 0 only.

| ID | Agent | Scope | Deliverable | Acceptance |
|---|---|---|---|---|
| 0.A1 | Baseline pinner | Execute §2 end-to-end on Windows, Linux, macOS reference machines | `docs/baseline/vlc-3.0.23.{md,json}` + `hardware-template.md` | G1 passes: all fields present, hashes computed + cross-checked, signature results recorded, dump-channel smoke test done |
| 0.A2 | Options & preferences inventory | §3.3 on all three platforms | `docs/inventory/options-{windows,linux,macos}.json`, `options-summary.md`, raw dumps + manifest | G4 passes: normalized records, three-way reconciliation with all diffs annotated |
| 0.A3 | Module census | §3.2 on all three platforms | `docs/inventory/modules-{windows,linux,macos}.json`, `modules-summary.md`, `raw/module-options/` | G3 passes: per-capability counts, platform diffs annotated, unexplained diffs = open defects |
| 0.A4 | Formats & access inventory | §3.6, using 0.A3's census + oracle probing | `docs/inventory/formats.{json,md}`, `raw/probe/` | G5 passes: every format mapped to observed modules with probe evidence or explicit "not probe-verified" |
| 0.A5 | UI, menus & hotkeys inventory | §3.4–3.5, Qt UI observation per platform | `docs/inventory/ui-inventory.md`, `hotkeys.toml`, design-package crosswalk | G6 passes: complete text transcription, no screenshots, per-platform key sequences |
| 0.A6 | Video codec coverage auditor | Pure-Rust availability for every inventoried video codec (feasibility §3); ADR-001 inputs | Video codec section of the feasibility report + evidence appendix | Every codec has a verdict with ≥2 sources; H.264/HEVC conclusion explicit |
| 0.A7 | Audio & subtitle codec coverage auditor | Same for audio codecs + subtitle parsing/rendering (feasibility §4–5) | Audio/subtitle sections of the feasibility report | Same acceptance shape; ASS-rendering gap quantified |
| 0.A8 | Subsystem feasibility auditor | Demuxers, network/streaming, audio/video outputs, filters, playlists/services-discovery, transcode (feasibility §6) | Subsystem sections of the feasibility report | Every subsystem: availability summary, risk, recommended phase |
| 0.A9 | WASM runtime & dependency/license auditor | ADR-002 candidates; dependency-audit method (MP §9); initial allowlist *format*; license policy inputs (doc 11) | ADR-002 evidence pack + audit-method spec | Candidate table complete; pass/fail per hard requirement; method reproducible |
| 0.A10 | Platform capabilities auditor | Per-OS reports: webview capabilities (Tauri transparency, raw window handles), hw-decode APIs, audio/video output OS bindings (ADR-001/003 inputs) | `docs/inventory/platform-{windows,macos,linux}.md` | Each ADR open question answered or explicitly [VERIFY]-flagged with follow-up |
| 0.A11 | Security reviewer | Threat-model draft for the whole program (MP §12); oracle sandboxing spec (§2.4); corpus safety rules; evidence-integrity rules for the matrix | `docs/inventory/security-phase0.md` | Threat model covers parsers, plugins, control server, oracle runs; matrix R4 evidence rules reviewed |
| 0.A12 | Corpus plan drafter | MP §13 corpus requirements mapped to matrix rows | `docs/inventory/corpus-plan.md` | G12 passes: per-P0/P1-feature fixture mapping, provenance/license column, malformed-generator spec, expectation-recording format |
| Lead | Synthesis & gates | Consolidate feasibility report; write the three ADRs; seed and freeze `feature-matrix.toml`; gate review | G7–G13 artifacts | Exit report ticks every row with links |

**Cross-cutting rules:** every command recorded; no VLC source access; wiki cited by URL; observed behavior beats documentation; append-only worklog; no commits (Lead commits); ask the Lead rather than inventing interface/policy decisions (AGENTS.md §3.6).

### Sequencing

```
Stage 1  0.A1  (pin; everything depends on the oracle)
Stage 2  0.A2, 0.A3, 0.A5            (parallel; need pinned binary; 0.A5 needs reference VMs)
Stage 3  0.A4 (after 0.A3 census) ∥ 0.A6, 0.A7, 0.A8, 0.A9, 0.A10 (can start on public
         docs/crates mid-Stage-2, finalize against inventory)
Stage 4  0.A11, 0.A12                 (need inventory + feasibility drafts)
Stage 5  Lead: feasibility consolidation → ADR-001/002/003 → matrix seeding + freeze →
         phase0-exit-report.md → Phase 0 gate
```

Rework loop: audits contradicting inventory (census vs wiki) correct the inventory — observed dumps are the truth.

---

## 8. Risks specific to Phase 0 (full register: doc 14)

| Risk | Mitigation |
|---|---|
| Oracle drift (mirrors rotate; 3.0.24 superseded 3.0.23) | Hash-pinned record + official sidecar cross-check; re-pin is Lead-only; Lead consciously confirms the 3.0.23 pin |
| Inventory volume underestimated (thousands of options × 3 platforms) | Normalized JSON first, prose second; parallel agents; resumable manifests |
| Clean-room contamination (agent consults VLC source or source-quoting forums) | §3.1 source ranking; Lead citation review; wiki/docs URLs only |
| Wiki inaccuracy vs 3.0.23 (community docs lag) | Observed behavior wins; discrepancies annotated in `*-summary.md` |
| ADR-001 deadlock (libva/OS-ABI question; patent review) | Conditional decision allowed (software-first, per-API escalation); legal inputs via doc 11 |
| GUI observation capacity (3 OSes) | 0.A5 scoped to reference VMs; Lead confirms availability (open question 3) |

---

## 9. Open questions for the Lead

1. **Project license** for Lumen itself — needed before 0.A9's license-policy work and any dependency approval (feeds doc 11). Not decided anywhere in the brief.
2. Confirm the **strict no-screenshot rule** for VLC UI observation (§2.1) and the text-transcription alternative.
3. Availability of **interactive reference VMs/CI runners** on all three OSes for 0.A5 and later UI-fidelity work.
4. **WebCodecs** in ADR-001 scope for the web build's "hardware decode" analog — in v1 or deferred? [PROPOSAL: defer decision to doc 09 + ADR-001.]
5. **WASM-in-WASM** (web build hosting extension plugins) — in v1 scope? [PROPOSAL: out of v1.]
6. Confirm the **artifact paths** fixed here (`docs/baseline/`, `docs/inventory/`, `docs/adr/`) so docs 13 (wave program) and 14 (risk register) reference them consistently.
7. Whether to also record (not adopt) a **VLC 4.0 watch-list** now, given MP §1 tracks 4.0 separately. [PROPOSAL: defer to Phase 5+.]

---

## Appendix A — Traceability to MASTER-PROMPT

| MP § | Requirement | This doc |
|---|---|---|
| §1 | Parity = share of matrix features verified by evidence; pinned VLC 3.0.23 | §0, §2, §4.1, §4.3 |
| §2 | Fixed decisions: clean-room; GPL oracle allowed, vendoring not; hw decode default + ADR | §2.1, §6.1 |
| §5 | Config-driven; feature exposure lists only existing implementations | §3.3 scoping rule, §4.6 R7 |
| §6 | Plugin architecture; WASM runtime must pass dependency audit | §6.2 |
| §7 | UI; video-in-webview candidates; Linux webview risk; prototype Phase 2 | §3.4, §6.3 |
| §8 | Per-format records | §3.6 |
| §9 | Rust-only policy; documented OS-service exceptions; crate audit fields | §2.1, §5.2, §6.1, §6.2 |
| §10 | Web column; blocked/unsupported with reason | §4.2, §4.5, §5.8 |
| §11 | Measure vs VLC on same files/hardware | §2.4 M4, §2.3 hardware template |
| §12 | Security; untrusted media | §2.4 sandboxing, 0.A11 |
| §13 | Differential testing vs external oracle; corpus; CI gates | §2.4, §4.6, 0.A12 |
| §14 | Known limits recorded, not hidden | §4.5 gap rows, §5.7 |
| §15 | Multi-agent protocol | §7 |
| §16 | Phase 0 definition; gates need recorded evidence | §1 (whole gate) |
| §17 | Statuses; blocked-item definition; reporting format | §4.3, §1.2 G13 |

## Appendix B — [VERIFY] ledger (Phase 0 follow-ups)

| # | Item | Follow-up owner |
|---|---|---|
| V1 | Exact 3.0.23 CLI flags (`--list-verbose`, `--full-help`, `--longhelp`, `-p <module>`) and hotkeys-in-help | 0.A1–0.A3 |
| V2 | Official sidecar hashes for all pinned artifacts; signature verification commands + results | 0.A1 |
| V3 | Linux pinning source (distro package vs VideoLAN apt repo vs Flathub) and exact versions | 0.A1 |
| V4 | Capability set + module counts per platform (census is authority) | 0.A3 |
| V5 | Option totals and three-way reconciliation counts | 0.A2 |
| V6 | vlcrc paths; preferences tree structure; dialog labels/shortcuts; hotkey bindings incl. macOS variants | 0.A2, 0.A5 |
| V7 | Per-format VLC module mappings used in seed rows (avformat/mp4/ts/mpg123/adaptive/ass/…) | 0.A4 |
| V8 | H.264/HEVC/AV1 pure-Rust decoder availability; Symphonia-family per-codec coverage; cpal/wgpu purity | 0.A6–0.A8, doc 04 |
| V9 | wasmtime/wasmi purity + resource-limit feature names; Extism status; permissive licenses | 0.A9 |
| V10 | VA-API/libva linking model; VideoToolbox/Media Foundation exact API names; WebCodecs scope | 0.A10, ADR-001 |
| V11 | Tauri transparency + raw-window-handle support; WebKitGTK/Wayland state; WebView2/WKWebView quirks | 0.A10, ADR-003 |
| V12 | Oracle log grammar / rc-interface command set; `--sout` determinism for M2 | Phase 2 harness owner |
| V13 | VLC telemetry publication status (priority-rubric input) | Lead |

---

*End of Phase 0 audit plan. Phase 0 begins when the Lead assigns 0.A1.*


