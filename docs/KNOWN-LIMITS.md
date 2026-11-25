# KNOWN-LIMITS — the consolidated known-limits register (issue #57, card 8.F2)

| Field | Value |
|---|---|
| Purpose | MP §14: "Known limits (record in the matrix, do not hide)" — one consolidated register of every known limit recorded across the repo, each row traceable to its source (doc §, ADR, risk row, or measured record) |
| Card / issue | 8.F2 / GitHub issue #57 (finalization lane `7.w22h`) |
| Compiled | 2026-11-10 |
| Refreshed | 2026-11-15 (wave `w30.10`, the docs-truth lane): stale rows struck with closing evidence (w27 h264-sw, w28 tauri shell, w30.1–w30.2 the Windows open path — audit findings S-1/S-2/S-3); current limits added (§4 GUI video display, §2 URL sources, release register MSI post-install); 2026-11-25 (Devil-Fix wave FX3, agent FX3-A08): §4's "Audio output: null backend only" row struck — cpal admitted behind `backend-cpal` (ADR-005 amended), default builds keep the instrumented null backend, audible output [VERIFY] pending a Windows run |
| Scope rule | This file **consolidates and links; it is not a second source of truth.** Status truth lives in `docs/feature-matrix.toml` (252 rows; audited — see `docs/PARITY-REPORT.md`); risk scoring lives in `docs/14-risk-register.md`; **release-specific limits (unsigned binaries, workflow-execution record, arm64 best-effort, placeholder icons, MSI sanitization, MSI post-install behavior) live in `docs/RELEASE.md` §"Known limits (honest register)" (wave `7.w22g`, refreshed `w30.10`) and are only linked here** |

---

## 1. Program-level limits (honesty frame)

| Limit | Detail | Source |
|---|---|---|
| **Verified parity is 0/252, pending the evidence flip** | Billing was unblocked 2026-11-12 and the release workflow has since executed (v0.1.0–v0.1.2 shipped; `docs/RELEASE.md` "First real run"), but the evidence-outbox flip pass that would move rows to `verified` has not run — all landed evidence remains local-run, `hardware_class = mock`; 64/252 rows `implemented` per desktop platform at the 2026-11-10 recompute (w27's h264-sw flip makes it 65; the PARITY-REPORT refresh is owed by its owner), 0 on web | `docs/PARITY-REPORT.md` §5; `docs/release-checklist.md` §9; doc 10 §7.1, §10.4 |
| **No wall-clock timing vs VLC** | All recorded timing is vs ffmpeg 7.1.5 on a shared 2-core sandbox (doc 12 §1.3 fallback); the 3-OS reference-hardware tier is externally blocked | `docs/PERF-VS-VLC.md` §1, §7; `docs/plans/phase8-cards.md` §1 |
| **Single-maintainer Lead** | The only committer/reviewer/dependency-approver — review latency is the program's throughput ceiling | R-21 |
| **Clean-room constraints** | No VLC source consultation (GPL contamination, R-17); MD5/comparison digests implemented in-tree from public specs (RFC 1321); external oracle binaries run unvendored, version-pinned | MP §2; doc 10 §6.6; `tools/bench-vs-vlc/README.md` |

## 2. Codec & format limits

| Limit | Detail | Source |
|---|---|---|
| ~~**H.264 software decode: none**~~ **H.264 software decode: landed (w27)** | ~~Parsers only; no production-grade pure-Rust decoder exists. Hardware-first per ADR-001 (accepted); admission audits recommend candidates, decision Lead-gated~~ **Struck 2026-11-15 (`w30.10`, audit S-1):** w27 admitted `rust_h264` 0.6.1 behind `codec-h264-sw` — MP4/MKV/TS/ES H.264 decode + play end to end, bit-exact 17/17 vs the ffmpeg 7.1.5 oracle; matrix row `codec-video-h264-software` flipped `blocked`→`implemented`. Open tails: 4:2:2/10-bit output vocabulary (the PixelFormat-extension card); hw-lane runtime activation stays ADR-013-gated | R-01; ADR-001 (revisit addendum — the trigger fired); `docs/dependencies/records/rust_h264-0.6.1.md`; worklog `w27.h264-sw-landing` |
| **HEVC software decode: none** | Parsers only; heavier patent picture; ADR-013 (VA-API lane) is a **draft — decision pending** | R-02; ADR-013 (proposed) |
| **Hardware decode lanes: not landed** | ADR-001 accepted hw-first for H.264/HEVC but no OS-decoder adapter exists; VA-API draft ADR-013 not decided; Media Foundation breadth (R-33) and VideoToolbox profile gaps (R-34) unproven; VA-API driver fragmentation (R-15) | ADR-001, ADR-013; R-15/R-33/R-34 |
| **Codec patent & redistribution** | ADR-010 is **proposed, pending counsel sign-off**; distributing binaries with H.264/HEVC/AC-3/DTS decoders may be legally blocked | R-03; ADR-010 (proposed) |
| **Opus: no pure-Rust decoder** | `symphonia-codec-opus` unpublished — WebM/YouTube-class audio unplayable | R-06 |
| **DTS/WMA: no decoder; TrueHD young-crate; AC-3 self-build** | Movie-file surround audio partial/missing through mid-program | R-10 |
| **rav1d runs asm-OFF** | Dependency-purity policy (MP §9) costs ~0.29× vs libdav1d full-asm — a policy gap, not an engineering one; Phase-7 admission reviews it | R-08; `docs/perf/breadth.md` §3; `docs/dependencies/records/rav1d-1.1.0.md` |
| **AV1/MJPEG/self-built lossy decoders slower than ffmpeg** | MJPEG 0.47–0.69× ffmpeg wall; lossy audio 2.3–4.6× ffmpeg CPU; honestly published next to wins | R-23; `docs/PERF-VS-VLC.md` §4 |
| **Differential comparable set is v1-narrow** | PCM-in-AVI only (35/500 corpus rows); pcm_s24le/s32le deferred on VLC `afile` format gaps; f32 float-chain deviation unattributed (6 tolerance-FAIL rows) | `docs/PERF-VS-VLC.md` §2–§3, §7 |
| **Long-tail container quirks** | RealMedia/ASF/legacy-hack divergence risk keeps rows `partial` | R-32 |
| ~~**First-vertical open path: only AVI**~~ **Open path: local files, every form** | ~~Only AVI opens through the command layer (D-018)~~ **Struck in two steps (audit S-2, then the w30 Windows fix):** w27 wired MP4/MKV/TS/ES-H.264 opens; w30.1–w30.2 made every local form open on every OS — Windows canonical paths (`\\?\` verbatim stripped, `/C:/`→`C:/` drive forms normalized, UNC typed-rejected both sides) and `file:` URLs routed through the local-file path (`C:\x` == `file:///C:/x` == `file://localhost/C:/x`); the v0.1.2 Windows open blocker (no local file opened by any path) is gone. Still true: raw `flac` opens refuse typed (no raw-FLAC demuxer), and in-session seek is not implemented (reanchor pending) on MP4 — every in-session MP4 seek returns `plugin.demux.reanchor.unsupported` (executed: W20-A05 F-001; audit/final/CONTRADICTIONS.md C-003) — while WAV seek completes through the CLI (the executed reference — `TimingSeekCompleted`; W20-A05); MP4/other containers pending the real reanchor (FIX-PLAN F1.3) | worklog w27 + `w30.1`/`w30.2`; `docs/plans/open-path-wiring.md`; `crates/media-types` `SourceUri::from_file_path`; `audit/final/CONTRADICTIONS.md` C-003; `audit/runs/W20-A05.log.md` F-001 |
| **HLS/DASH/RTSP URL sources: typed-refused at the open path** | http(s) progressive/ranged OPENS since w31.3 (the streaming crate's `RangedResource` serves the same SourceStream contract as file access — loopback-proven through the real binary with the production ureq transport); HLS/DASH/RTSP keep the lane-named typed refusal (blocked v1 per the feature matrix); `product.toml` exposes `streaming-http` = true, `streaming-hls`/`streaming-dash` = false | `apps/cli/src/demux_host.rs` (open_source, the http arm + the blocked-lane refusal); `product.toml` features; worklog `w31.3` |

## 3. Subtitle & rendering limits

| Limit | Detail | Source |
|---|---|---|
| **libass forbidden (C)** | ASS/SSA render without full styling (positioning, karaoke, fonts) — a visible parity gap on anime/fansub content | R-09; doc 04 §7 gap 5 |
| **Bitmap subtitles: renderer overlay contract missing** | PGS/DVB captions stay missing until compositing is proven | R-27; ADR-012 (accepted — the contract direction; landing tracked there) |

## 4. Platform & output limits

| Limit | Detail | Source |
|---|---|---|
| ~~**OS-webview shell: seams only, INFRA-BLOCKED**~~ **OS-webview shell: real and shipped; deep verdicts open** | ~~no tauri binary has ever been built or launched on any OS~~ **Struck 2026-11-15 (`w30.10`, audit S-3):** w28 built and shipped `lumen-tauri.exe` — GUI subsystem, frameless shell, the real engine host behind `lumen_dispatch` (v0.1.1/v0.1.2 artifacts); w30.6 gave it a visible crash face (panic hook + crash log under `<tmp>/lumen/` + a Windows no-console MessageBox with the WebView2 hint) and w30.7 the ~30 ms pacer (`ShellRuntime::pump_once`) so GUI playback actually progresses; w30.8 added the Windows launch smokes. Still open: the tauri-launch.yml deep verdicts (window opens / file opens / playback + settings round-trip per OS) remain manual, and the per-OS human confirmation on real hardware is owed per ADR-004's own rule | ADR-004; `apps/desktop-tauri/INFRA-BLOCKED.md` §B; worklog w28/w29/w30 |
| ~~**Audio output: null backend only**~~ **Audio output: cpal admitted behind `backend-cpal`; default builds null; audible output [VERIFY]** | ~~ADR-005 accepted the adapter seam; **cpal is NOT admitted** (Lead-gated conditional-E2; sandbox build fails exit 101); every platform refuses real output through a typed marker; no real-device evidence exists — engine-wide this means **no sound on any OS yet** (the most user-visible gap in the shipped product)~~ **Struck 2026-11-25 (Devil-Fix wave FX3, FIX-PLAN §3 F1.1 Route A):** ADR-005 is amended (FX3-A02) — cpal 0.18.2 is admitted behind the optional `backend-cpal` cargo feature of `crates/audio-output` (the adapter is `crates/audio-output/src/cpal.rs`, FX3-A01); the windows-latest CI lane compiles `audio-output` with that feature (FX3-A07) — the committed, regenerable command for the compile claim (`cargo build -p audio-output --features backend-cpal` needs an OS audio stack: this sandbox's ALSA dev package is absent, which is why the lane lives on windows-latest). **Default builds (no feature) still fall back to the null backend, now instrumented with sample/frame counters** (FX3-A05) — the null path's progress/underrun evidence is real counter evidence, not a mock label on silence. **The shipped hosts still open the null backend today** — host wiring of backend selection (the wave-08 S0 finding "every shipped host renders into NullBackend") **lands in wave FX4**; FX3 admitted the seam. Volume/mute/balance/EQ are real engine controls (`audio-engine` DSP chain, FX3-A03) and resampler rate conversion is real at engine level (FX3-A04) — **wiring them into the shipped hosts also lands in wave FX4; not claimed today**. **Audibility on real hardware remains [VERIFY] pending a Windows run** (FIX-PLAN §3 F1.1 acceptance: audible AAC via `lumen open sample.mp4`) | ADR-005 (amended); `audit/fix/waves/FX3/FX3-A0{1,2,5,7}.md`; `audit/fix/FIX-PLAN.md` §3 F1.1; `audit/waves/wave-08/SUMMARY.md` (S0); R-29 (re-scored 2026-11-25) |
| **GUI video display: not wired** | The desktop UI mounts an honest empty media surface — no `<canvas>` exists in `ui/src` (pinned by test: the surface is a labelled region, never a fake video); the LUF1 wire + `CanvasBridge`/canvas-presenter reference (2.D3) is complete but unported, so the CLI renders frames headless while the GUI displays none | doc 08; `apps/desktop/src/bridge.rs` (the unported reference); `ui/src/screens/player/PlayerScreen.test.tsx` (the no-fake-canvas pin) |
| **TLS: seam active, real handshake unproven in-tree** | ADR-011 accepted ureq+rustls (ring = ratified E2-class exception); https routes/activates with verification always-on; a real TLS round-trip needs a networked lane (sandbox has no egress) — out-of-band measurement recorded in the ureq record | ADR-011; matrix row `network-tls-policy`; `docs/dependencies/records/ureq-3.5.0.md` R10 |
| **macOS notarization/signing** | Requires an Apple Developer account + CI key custody (external, user-owned) — cascades to packaging/SBOM/RC cards | R-35; `docs/plans/phase8-cards.md` §1 |
| **Linux distro variance** | WebKitGTK versions, deps, deb/rpm/flatpak per-distro breakage risk | R-36 |
| **Windows code-signing** | Certificate availability external `[VERIFY]` | `docs/plans/phase8-cards.md` §1; `docs/RELEASE.md` known-limit #1 |

## 5. Web (wasm) limits — doc 09 §10 view

| Capability | web status | Reason |
|---|---|---|
| Raw UDP, RTSP/RTP | `unsupported` | No raw sockets in browsers (MP §10) |
| Optical discs (DVD/BD/CD) | `unsupported` | No disc APIs; ADR-gated on desktop too (MP §14) |
| DVB / capture hardware | `blocked` | No hardware access; browser-API substitutes are the next step |
| `getUserMedia`/`getDisplayMedia` capture | `not_started` | Phase 7+; Chromium-first `MediaStreamTrackProcessor` `[VERIFY]` |
| Hardware decode | `partial` [PROPOSAL] | WebCodecs ADR not yet opened — until it passes, H.264/HEVC stay `blocked` on web |
| Threaded decode perf | `partial` | Conditional on COOP/COEP headers, which break third-party embedding (R-13) |
| Local filesystem browsing | `unsupported` | By design; File System Access API may lift on Chromium later |
| Renderer | `partial` | WebGPU fragmented (Firefox 141 Windows-only, Firefox-Android none — R-12); WebGL2 fallback |
| Web target overall | **0/252 implemented** | wasm target is Phase 7 work (`docs/PARITY-REPORT.md` §4) |

Source: `docs/09-web-wasm-target.md` §10 (the table is a *view* of the matrix, never a second source of truth).

## 6. Extension/plugin limits

| Limit | Detail | Source |
|---|---|---|
| **VLC Lua extensions will not run** | Replaced by the WASM plugin model (ADR-002: wasmi accepted); existing Lua scripts are incompatible | MP §14; ADR-002 |
| **WASM runtime resource limits** | Fuel/epoch + memory-cap enforcement must be re-audited on web; plugin sandboxing claims depend on it | R-30; doc 09 §9.4 |
| **WASM binary size budgets 3/6 MB are proposals** | Pinned after the first real build | R-31; doc 09 §2.4 |

## 7. Process & evidence limits

| Limit | Detail | Source |
|---|---|---|
| **CI billing blocked** | Full matrix, RC builds, 3-OS runners all blocked (~670 h/month budget line); every lane re-verified locally instead | R-16; doc 10 §10.4; `docs/plans/phase8-cards.md` §1 |
| **Soak 4 h measurement pending** | Nightly workflow is wired; only a 2-min CI-short profile exists; the 5 MB/h ceiling is `[PROPOSAL until first real measurement]` | `docs/perf/soak-nightly-20261109.md` |
| **TTF per-frame events absent** | Doc 05 v1 catalog lacks `video.first-frame-presented`/`audio.first-sound`; latency metrics stay host-measured | doc 12 §2.4, §1.5 |
| **uhd4k warm-seek discrepancy unexplained** | 404 → 705 ms warm-seek on the 4 K lane, flagged to the engine owner | `docs/perf/breadth.md` §5 |
| **Parity tooling gaps** | R7 config cross-check (owned elsewhere), R8 inventory completeness, R10 beyond path existence, `not_started → verified` git-history jump rejection (needs `--from`) | `tools/xtask/src/cmd/feature_audit.rs` header; `docs/PARITY-REPORT.md` §2 |
| **Design-asset license** | Figma "VLC Redesign" package terms unverified; title/name borrows VLC's mark — shipped-UI risk | R-07; `design/README.md` |
| **19/35 differential inputs re-based** | Original 2026-10-23 corpus inputs with runtime-random sources are unrecoverable; verdicts are same-input on fresh realizations | `docs/corpus/vlc-oracle-run-20261109.json` `inputs` |

## 8. Out-of-scope by policy (never claimed)

- VLC 4.x / non-inventory features (scope pinned to the VLC 3.0.23 inventory — R-22).
- Corrupt-input *comparison* vs VLC (the differential is clean-file only v1; corrupt inputs follow the error-class rule, doc 10 §6.6).
- Byte-equality of lossy decode vs VLC (never claimed — different conforming implementations; doc 10 §6.6).
- "All formats supported" claims of any kind (MP §1; README-audit rule, R-26).

## 9. Release-specific limits

Linked, not duplicated: **`docs/RELEASE.md` §"Known limits (honest register)"**
(wave `7.w22g`, refreshed `w30.10`): unsigned binaries (SmartScreen/Gatekeeper
consequences), the executed workflow's own record (billing unblocked
2026-11-12; v0.1.0–v0.1.2 shipped — first-real-run evidence there), Linux
arm64 best-effort, runner-label deviation, placeholder icons, MSI version
sanitization, **MSI does not auto-launch after install (Start-Menu + Desktop
shortcuts are the entry points; the w30.9 post-install decision)**, tag-only
budget.

## 10. Maintenance rule

Every row above must stay traceable to a living source. When a limit
resolves (e.g. ADR-013 decided, CI billing restored, cpal admitted), update
the source (ADR status / matrix row / risk re-score) and then this register
in the same wave — never delete a row silently; strike it with the closing
commit reference, per the append-mostly discipline of the matrix
(doc 02 §4.7) and the risk register's re-score log.
