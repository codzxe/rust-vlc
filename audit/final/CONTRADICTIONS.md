# CONTRADICTIONS — The Register (§8.1, W20-A08 retry)

| Field | Value |
|---|---|
| Auditor | W20-A08 (contradiction register, retry run) |
| Date | 2026-11-24 |
| Method | Consolidated from audit/waves/wave-{01..19}/SUMMARY.md, audit/verify/W20-A0{1,2,3}-reverification.md, the doc claim ledgers under audit/docs/**.audit.md, and the prior-run register. The TOP 12 contradictions (marked **[VERIFIED this run]**) carry two fresh verbatim quotes each, re-verified with sed/rg against the read-only tree (commands in audit/runs/W20-A08.log.md). All other entries are listed from the corpus with pointers to the proving audit files (quotes as recorded there; no re-verification per orders). |
| Entries | 46 — S0: 15 · S1: 0 · S2: 26 · S3: 5 |
| Layout | Tier 1 = user-facing lies (6). Tier 2 = status-layer drift (15). Tier 3 = process/paper contradictions (25). |

---

## TIER 1 — User-facing lies (doc denies or misstates what the shipped binary does)

### C-001 [S0] **[VERIFIED this run]** video-h264.md denies software H.264 decode; the shipped release binary embeds and dispatches rust_h264
| Side | Citation | Verbatim quote |
|---|---|---|
| A (doc) | `docs/formats/video-h264.md:8-12` | "**There is no software H.264 decode, and none is scheduled before ADR-001's conformance-gated software track produces a validated decoder** — doc 04 §4.2: \"no production-grade pure-Rust H.264 decoder exists today\"" |
| B (code) | `Cargo.lock:769-771` + `apps/cli/src/pipeline_host.rs:346` | `name = "rust_h264"` / `version = "0.6.1"` … `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),` |

**Truth:** the CODE. The release binary embeds rust_h264-0.6.1 and dispatches it on the MP4 open path. Proof: W20-A02 F-3 (executed: `strings target/release/lumen | rg rust_h264` shows the full decoder; `lumen open sample.mp4` walks to end-of-media selecting the h264+aac lanes). Ledger: W19-S2 (audit/docs/docs/formats/video-h264.md.audit.md).

### C-002 [S0] **[VERIFIED this run]** Four decoder registries inside one shipped binary give four different inventories
| Side | Citation | Verbatim quote |
|---|---|---|
| A1 (engine-info) | `apps/cli/src/host.rs:1643-1658` | `decoders: vec![ DecoderInfo { name: "mjpeg".to_string(), … }, DecoderInfo { name: "pcm_s16le".to_string(), … } ]` (exactly 2 entries) |
| B (dispatch) | `apps/cli/src/pipeline_host.rs:242-347` + refusal text `:374-378` | `"mjpeg" => { let mut decoder = codecs::video::mjpeg::MjpegDecoder::new(); …` … `"h264" \| "avc1" => self.seeded(track, config, H264SwDecoder::new),` … `"no decoder for \`{other}\` in this build (landed: mjpeg, pcm, flac, mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264)"` |

**Truth:** the dispatch (16 match arms / 14 codec families incl. vp9 `:313` and dnxhd `:326`, which even the refusal text's 12-name list omits). Fourth surface: `apps/cli/src/bench.rs:334` `"no decoder in this build (first vertical: mjpeg + pcm)"`. Proof: W20-A02 F-3 (executed: `engine-info` prints 2 decoders; `decode-bench` prints v_frames=0 on a file the same binary decodes; HEVC refusal lists 12). Wave-0's "zero decode" litmus was bench's instrument artifact (W03 correction).

### C-003 [S0] The seek triple-lie: matrix "implemented" + MediaInfo `seekable: true` + CLI `seekable: yes` vs every in-session MP4 seek failing
| Side | Citation | Verbatim quote (as recorded in the proving audit files) |
|---|---|---|
| A (docs/status/CLI) | `docs/feature-matrix.toml:1510-1519` (row 103 playback-seek) + `apps/cli/src/host.rs:1307-1308` | `status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }` … `seekable: true,` / `is_live: false,` (hardcoded constants in `media_info()`) |
| B (code, executed) | `crates/plugin-api/src/demux.rs:113-120` (default `reanchor` every non-overriding demuxer gets; `impl Demuxer for Mp4Demuxer` at `crates/demux/src/mp4/mod.rs:2008-2084` implements NO `reanchor`) | `fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> { let _ = target; Err(EngineError::new(… "plugin.demux.reanchor.unsupported")) }` |

**Truth:** the engine's refusal. Executed: `lumen open sample.mp4` + `seek 1s` → `TimingSeekFailed { … "[demux] unsupported (plugin.demux.reanchor.unsupported)" }` while the same session's `MediaInfoReady` says `seekable: true` (both MP4 samples; WAV contrast completes). One user action, three simultaneous lies. Proof: W20-A02 F-1 (executed) + F-2 (executed FLAC-in-WAV divergence: engine `seekable: false` vs CLI `seekable: yes` in one session); W05-A01, W13-A02.

### C-004 [S0] **[VERIFIED this run]** network-http.md: "CLI does not dispatch yet … every `SourceSpec::Url` still refuses" vs the shipped http arm
| Side | Citation | Verbatim quote |
|---|---|---|
| A (doc) | `docs/formats/network-http.md:7-11` | "**The CLI open path does not dispatch through the registry yet** … today every `SourceSpec::Url` still refuses at `apps/cli/src/demux_host.rs` (`UnsupportedForHost`, \"needs the streaming crate\")." |
| B (code) | `apps/cli/src/demux_host.rs:427` | `"http" \| "https" => open_http_source(url, fetch)?,` |

**Truth:** the CODE. `lumen open http://…` rides the production ureq transport + RangedResource. The doc is stale by one landing (w31.3; matrix row 096 `protocol-http-https` = implemented with that evidence). Proof: W18-A10 (executed loopback e2e exit 0), W13 trace.

### C-005 [S0] **[VERIFIED this run]** PARITY-REPORT: "CI is billing-blocked … every Actions run dies at 0 steps" vs RELEASE's unblock and six live run IDs
| Side | Citation | Verbatim quote |
|---|---|---|
| A (doc) | `docs/PARITY-REPORT.md:122-125` | "because CI is billing-blocked on this private repository: every GitHub Actions run dies at 0 steps regardless of code (`worklog.md` 7.CLOSEOUT; risk R-16/R-20 refresh; `docs/RELEASE.md` known-limit #2)." |
| B (doc) | `docs/RELEASE.md:145-148` + `worklog.md:1625/1968/2065` | "2. **~~The workflow has never executed.~~ The workflow has executed (v0.1.0–v0.1.2).** Billing was unblocked 2026-11-12" … "CI run 43378741271/43378741286/43378757248" / "The v0.1.0 tag run (43464162696)" / "MSI upgrade test exit 1603 (run 43729218652, windows job)" |

**Truth:** B. Six in-tree live-run IDs side with RELEASE.md. Proof: W14-A04; W17; W19-S2 ledger.

### C-006 [S0] **[VERIFIED this run]** INFRA-BLOCKED: "the crate compiles in its dedicated CI lane" vs app.rs:301 calling a function that does not exist (E0425)
| Side | Citation | Verbatim quote |
|---|---|---|
| A (doc) | `apps/desktop-tauri/INFRA-BLOCKED.md:3-6` | "> **Status: ADMITTED (code half) — 2026-11-13.** The wrapper exists, the anti-bypass audit exists, the crate compiles in its dedicated CI lane (`tauri-check`, Linux + webkit2gtk)…" |
| B (code) | `apps/desktop-tauri/src/app.rs:300-303` | `apply_window_spec(app.handle())?; spawn_session_pump(app.handle().clone()); open_argv_files(app.handle()); Ok(())` — repo-wide rg: `spawn_session_pump` has exactly ONE hit (this call); the real name is `fn spawn_event_pump` at app.rs:190 |

**Truth:** the CODE. The crate cannot compile on any host (E0425; 4 independent confirmations — the sandbox gdk-sys failure masks it). ci.yml tauri-check + tauri-launch.yml + release.yml:219 all build it → all red on this tree. Proof: W20-A02 F-4 (CONFIRMED, static); W13-A10, W14-A05.

---

## TIER 2 — Status-layer drift (matrix rows, registries, status docs)

Format: A-says (quote+loc) | B-says (quote+loc) | truth + proof pointer. Quotes marked **[VERIFIED this run]** were sed/rg-verified today; others are quoted as recorded in the proving audit file.

### C-007 [S2] **[VERIFIED this run]** KNOWN-LIMITS "no `<canvas>` exists in `ui/src`" vs the shipped canvas presenter
| Side | Citation | Verbatim quote |
|---|---|---|
| A (doc) | `docs/KNOWN-LIMITS.md:52` | "\| **GUI video display: not wired** \| The desktop UI mounts an honest empty media surface — no `<canvas>` exists in `ui/src` (pinned by test: the surface is a labelled region, never a fake video)…" |
| B (code) | `ui/src/components/player/MediaSurface.tsx:172-177` and `:224-229` | `return ( <canvas ref={canvasRef} className="lumen-player__media" data-media-surface="frames"…` … `const image = ctx.createImageData(header.width, header.height); image.data.set(pixels); ctx.putImageData(image, 0, 0);` |

**Truth:** the CODE (inside the desktop shell behind `__TAURI_INTERNALS__`); the browser build is genuinely canvas-less (W15-A01, pinned by PlayerScreen.test). Proof: W15 summary finding 1; audit/docs/docs/KNOWN-LIMITS-family ledger.

### C-008 [S0] **[VERIFIED this run]** Matrix rows 095/099/100/101: `not_started`, `evidence = ""` vs ~4,620–5,350 LOC of landed, tested, executed code
| Side | Citation | Verbatim quote |
|---|---|---|
| A (matrix) | `docs/feature-matrix.toml:1405-1414` (row 095; identical at `:1457/:1470/:1481` rows 099/100/101) | `id = "network-http-client"` … `status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }` / `evidence = ""` (same four rows: network-reconnect-buffering, network-auth, network-remote-playlists) |
| B (code) | `crates/streaming/src/client.rs:1-2` (+ reconnect.rs 1560 LOC, auth.rs 1562, remote_playlist.rs 500) | `//! ureq adapter — the real HTTP(S) transport behind [`HttpFetch`]` / `//! (card 6.A1b admission; hardened by 6.A1c).` |

**Truth:** the CODE. Proof: W20-A02 F-10 (executed: `cargo test -p streaming --lib` → **212 passed; 0 failed**; 8 auth_e2e + 3 net_interrupted tests passing over client/reconnect/auth/remote_playlist). W18-A10. The matrix understates in the same direction it overstated asf (C-009).

### C-009 [S0] **[VERIFIED this run]** Matrix row 007 `container-asf-wmv`: "implemented" (windows/macos) vs an open path that refuses ASF on every platform
| Side | Citation | Verbatim quote |
|---|---|---|
| A (matrix) | `docs/feature-matrix.toml:207-215` | `id = "container-asf-wmv"` … `status = { windows = "implemented", linux = "partial", macos = "implemented", web = "not_started" }` |
| B (code) | `apps/cli/src/containers/mod.rs:52-74` | the dispatch arms are `adts/aiff/avi/ivf/matroska/flac/hevc/h264/mp2/mp3/mp4/ogg/mpegps/mpegts/wav` then `other => Err(unwired_refusal(other))` — **no `asf` arm**; `crates/media-probe/src/signature/mod.rs` registers no ASF signature (rg "asf" = 0 hits) |

**Truth:** the CODE. The demuxer itself is real (`crates/demux/src/asf/*`) but unreachable from every shipped lane; the compat-matrix run proved the open path REFUSES corpus asf files (`probe.no-demuxer`, docs/corpus/compat-matrix-20261110.json). Proof: W04-A04/A08/A10; W18 row verdict FALSE.

### C-010 [S2] **[VERIFIED this run]** The 2026-11-10 compat-matrix downgrades (applied) vs the rows' current statuses
| Side | Citation | Verbatim quote |
|---|---|---|
| A (transitions log) | `docs/corpus/compat-matrix-transitions-applied.jsonl` (row 1 of 6, applied_at 2026-11-10T20:07:51Z) | `{"schema":"compat-matrix/transitions-applied/1", … "row":"container-asf-wmv","platform":"linux","from":"implemented","to":"partial","reason":"compat-matrix run (linux-native lane): the open path REFUSES the corpus files of probe format \`asf\` — unsupported…` |
| B (current matrix) | `docs/feature-matrix.toml:215` (asf) and `:673/:720/:480/:634` (aac-lc/alac/vp9/pcm) | asf: `status = { windows = "implemented", linux = "partial", macos = "implemented", web = "not_started" }`; the four codec rows: all `linux = "partial"` but `windows = "implemented", macos = "implemented"` |

**Truth:** neither row state is fully true. The asf refusal is platform-independent (no asf arm at all — C-009) yet only the linux cell was downgraded; the codec transitions' own reasons say "1 corpus tracks of codec `aac-lc`/`alac`/`vp9` REFUSED (no decoder in this build)" while the shipped dispatch carries `"aac"|"aac-lc"` (`pipeline_host.rs:288`), `"alac"` (`:293`), `"vp9"` (`:313`) arms. The 6 evidence-driven transitions were applied; the matrix kept drifting. Proof: W19-S2 ledger ("then the matrix drifted again"); audit/docs/docs/corpus/compat-matrix* ledgers.

### C-011 [S2] **[VERIFIED this run]** AGENTS.md: "No unsafe code in application or core crates" vs `allow(unsafe_code)` in application/core crates
| Side | Citation | Verbatim quote |
|---|---|---|
| A (doc) | `AGENTS.md:33` | "3. **No unsafe code** in application or core crates. Platform adapters need an approved ADR + documented invariants." |
| B (code) | `crates/codecs/src/video/av1/decoder.rs:93`, `crates/codecs/src/video/mjpeg/idct_simd.rs:78`, `apps/web/src/abi.rs:80` (+ 22 more tree-wide) | `#![allow(unsafe_code)]` (rg: 25 files tree-wide carry it; 11 in application/core per W1) |

**Truth:** the CODE. Core (crates/codecs) and application (apps/web) files violate the stated rule with no ADR of record covering them. Proof: W1 finding (11 files) — still true at HEAD.

### C-012 [S0] Matrix row 084 `protocol-file`: not_started ×4 vs the shipped file-open path (corpus)
| A | `docs/feature-matrix.toml:1253-1262`: `status = { windows = "not_started", … }` / `evidence = ""` (as recorded in the prior register) |
| B | `apps/cli/src/demux_host.rs:421-424`: the first thing the shipped CLI does (`file_uri_from_canonical` → `FileAccess::open`; the whole baseline corpus smoke ran on it) |
| Truth + proof | the CODE (shipped and executed since the first vertical). W18-A06 verdict: "084 file (UNDERSTATED: shipped+executed but row says not_started)". |

### C-013 [S2] Matrix row container-mp4: notes begin "No MP4 demuxer code exists" inside an implemented row that also claims proven playback (corpus)
| A | `docs/feature-matrix.toml:142-154` (intra-row): notes: `No MP4 demuxer code exists; the MP4 box walker in crates/codecs/tests/common/mod.rs is TEST-SIDE fixture-extraction tooling…` |
| B | same row: `status = { windows = "implemented", … }`, evidence `crates/demux/src/mp4/**`, and notes: `2026-11-15: end-to-end VIDEO PLAYBACK proven on the h264+aac lane — mp4_h264_aac.mp4 plays to Ended (exit 0…)` |
| Truth + proof | the demuxer exists and MP4 open+decode+play works. W13 executed trace; baseline mp4-smoke exit 0; W19 rows ledger. |

### C-014 [S2] Matrix rows 148-150: config areas "partial … NOT registered" vs registered, shipped areas (corpus)
| A | rows at `docs/feature-matrix.toml:2099+`: `evidence = "video.tsx (+ video.test.tsx) — component exists but is NOT registered in SETTINGS_AREA_IDS/SETTINGS_SCREENS (ui/src/screens/settings/areas/index.tsx)"` (as recorded) |
| B | `ui/src/screens/settings/areas/index.tsx:189-203`: `"controls", … "recording"` ∈ `SETTINGS_AREA_IDS`; `video: VideoAreaScreen` registered (:243) |
| Truth + proof | the CODE. W19 rows-144-156 audit: 6 of 13 rows FALSE in that chunk (5 stale-understated, 1 inflated). |

### C-015 [S0] Matrix rows 151/156: config-area-controls and config-area-recording "not_started", `evidence = ""` vs registered + tested areas (corpus)
| A | `docs/feature-matrix.toml:2135-2143`: `id = "config-area-controls"` … `status = { … "not_started" ×4 }` / `evidence = ""` (as recorded) |
| B | `ui/src/screens/settings/areas/index.tsx:193` (`"controls",`); `controls.tsx` 244 LOC + `controls.test.tsx`; `recording.tsx` 159 LOC + test; `recording: RecordingArea` registered :246 |
| Truth + proof | the CODE. W19-A03 rows ledger ("controls and recording claim not_started with evidence = \"\" while ControlsArea and RecordingArea are registered with real schema-generated rows and tests"). |

### C-016 [S2] Matrix menu rows cite "AppShell.tsx File/Playback/Tools/View/Help menu" vs AppShell: "The Solstice menubar is GONE" (corpus)
| A | `docs/feature-matrix.toml:3011` (also :3023/:3131/:3143/:3155): `evidence = "ui/src/components/appshell/AppShell.tsx File menu (menu.file.open-media → …) over ui/src/components/menu/MenuBar.tsx"` (as recorded) |
| B | `ui/src/components/appshell/AppShell.tsx:35`: `// The Solstice menubar is GONE (no W25 frame carries one): navigation lives in the rail + breadcrumb…` |
| Truth + proof | the CODE — the evidence anchors of ≥5 menu rows cite UI that no longer exists. W19-A03 (3 SHALLOW + 8 FALSE-understated rows in the menus band). |

### C-017 [S2] network-http.md's own status vs matrix row 095: two docs, opposite statuses for the same capability (corpus)
| A | `docs/formats/network-http.md:3-4`: `Status: **implemented (library stack)** — the fetch port (6.A1a), the admitted ureq client (6.A1b/6.A1c)…` |
| B | `docs/feature-matrix.toml:1409`: `id = "network-http-client"` … `status = { windows = "not_started", … }` / `evidence = ""` |
| Truth + proof | neither — truth is row 096/protocol-http-https "implemented" + C-004's http arm (shipped, executed). The status layer disagrees with itself in both directions around the truth. W18-A10: "Row 095 is directly contradicted INSIDE this same document by row 096." |

### C-018 [S2] Matrix row 142 autostart: not_started vs `--start-paused` shipped + tested (corpus)
| A | row 142 `autostart` in `docs/feature-matrix.toml` (rows 131-143 band): not_started (as recorded in wave-19 summary) |
| B | the shipped `--start-paused` CLI flag with tests (as recorded) |
| Truth + proof | the CODE — wave-19 rows-131-143 audit: "1 FALSE-understated (142 autostart: --start-paused shipped+tested)". |

### C-019 [S3] Matrix rows 041/044 notes: machine-written triage cites the canned phrase "no decoder in this build" for codec-gate refusals (corpus)
| A | rows 041/044 notes in `docs/feature-matrix.toml` citing "no decoder in this build" (canned phrase from `apps/cli/src/report.rs:1199`) |
| B | the actual refusals are codec-configuration gates (aac.channel_count, alac.config.missing), not decoder absence — the dispatch has both arms (`pipeline_host.rs:288/:293`) |
| Truth + proof | wave-18 summary finding 3: "Notes' machine-written triage misdiagnoses (rows 041/044) cite the canned phrase … which collapses codec-gate refusals." |

### C-020 [S2] PARITY-REPORT's own counts are stale vs the live matrix it describes (corpus)
| A | `docs/PARITY-REPORT.md:120-122`: "**Lumen's verified parity is 0/252, honestly.** 64 of 252 inventory features (25.4%) are `implemented` on each desktop platform (windows, linux, macos identically…)" |
| B | `audit/runs/baseline/xtask-parity-report.txt:61` (tool re-run on the audited tree): `parity: rows=252 windows=66/252 linux=61/252 macos=66/252 web=0/252` |
| Truth + proof | the live matrix (three DIFFERENT desktop numbers; none is 64). Recount debt self-confessed in worklog.md:2221. W19 doc audit. |

### C-021 [S3] Opus matrix row's channel order contradicts the RFC (corpus)
| A | opus row in `docs/feature-matrix.toml` claiming the 4-channel order (as recorded in wave-07 summary) |
| B | RFC 6716 4-channel order (the ruopus 0.1.2 decoder's own tables) |
| Truth + proof | the spec — wave-07 summary: "Opus: ruopus 0.1.2 (4-ch order row contradicts RFC)". Proof file: audit/waves/wave-07/SUMMARY.md. |

---

## TIER 3 — Process/paper contradictions (gates, evidence logs, signatures, policy docs)

### C-022 [S2] **[VERIFIED this run]** Risk register: "implemented (awaiting Lead review)" + "tools/feature-audit still unbuilt" vs PENDING signatures and a running tool
| Side | Citation | Verbatim quote |
|---|---|---|
| A (doc) | `docs/14-risk-register.md:6` and `:46` | `\| Status \| implemented (awaiting Lead review) \|` … "`tools/feature-audit` still unbuilt (fuzz-target analog `scripts/fuzz_registry_check.sh` is live); `docs/gates/phase3-exit.md`/`phase4-exit.md` verdict lines still read PENDING LEAD SIGNATURE though Phases 4–6 proceeded on their evidence" |
| B (docs/baseline) | `docs/gates/phase3-exit.md:3-5` + `audit/runs/baseline/xtask-feature-audit.txt:1,4,10-12` | "- **Verdict: PENDING LEAD SIGNATURE** — this is the evidence record produced by gate agent 7-GATE…" / "# cmd: cargo run -p xtask -- feature-audit --locked-independent (xtask tool)" / "=== LUMEN FEATURE-AUDIT REPORT ===" … "result: FAILURE (2 violation(s))" |

**Truth:** B. The register (a) claims a review status its own §0.1 says never completed, and (b) says the feature-audit tool is unbuilt while the baseline executed it. Proof: W19-S2 ledger; wave-00 baseline.

### C-023 [S2] **[VERIFIED this run]** Soak report: "CI, all jobs — billing-blocked repo-wide (every Actions run dies at 0 steps)" vs the same run IDs
| Side | Citation | Verbatim quote |
|---|---|---|
| A (doc) | `docs/soak/soak-report-20261110.md:222-225` | "- **CI, all jobs** — billing-blocked repo-wide (every Actions run dies at 0 steps; docs/KNOWN-LIMITS.md R-16). The 4 h nightly profile, the LSAN run, and every workflow step above are wired-but-never-run `[VERIFY on first run]`." |
| B (doc) | `docs/RELEASE.md:145-148` + `worklog.md:1625/1968/2065` | "The workflow has executed (v0.1.0–v0.1.2).** Billing was unblocked 2026-11-12" + run IDs 43378741271/43378741286/43378757248, 43464162696, 43729218652 |

**Truth:** B. Proof: W17 run IDs; W19-S2 ledger ("nightly tier billing-blocked" FALSE, header stale).

### C-024 [S0] release.yml's own header: "THIS WORKFLOW HAS NEVER EXECUTED" vs the same file's live-run post-mortems (intra-file) (corpus)
| A | `.github/workflows/release.yml:25-27`: `# HONESTY NOTE — THIS WORKFLOW HAS NEVER EXECUTED (CI billing blocked): every Actions run in this repository currently dies at 0 steps…` |
| B | same file `:1139-1144` (+:278, :1140): `# GitHub caps release bodies at 125,000 characters (HTTP 422 "body is too long" -- first live tag run 43464162696, step died on it).` |
| Truth + proof | the body — the workflow's own inline post-mortems of three distinct live runs (43378757248, 43464162696, 43729218652) falsify its own header. W17, W19. |

### C-025 [S2] KNOWN-LIMITS: "CI billing blocked" row vs the same file's §9 "billing unblocked 2026-11-12" (intra-doc) (corpus)
| A | `docs/KNOWN-LIMITS.md:86`: `\| **CI billing blocked** \| Full matrix, RC builds, 3-OS runners all blocked (~670 h/month budget line); every lane re-verified locally instead \|` |
| B | same file `:104-106`: "the executed workflow's own record (billing unblocked 2026-11-12; v0.1.0–v0.1.2 shipped — first-real-run evidence there)" |
| Truth + proof | B — the register carries both the stale row and the correction without reconciling them (README.md:135 admits "some older docs still carry the pre-unblock 'billing-blocked' wording"). |

### C-026 [S2] phase3-exit signature line "Signed: 2026-10-24, Lead" vs its own square-up saying it stood PENDING until 2026-11-13 (corpus)
| A | `docs/gates/phase3-exit.md:218`: `- Signed: 2026-10-24, Lead, on \`main\` post-merge of all wave-7 branches` |
| B | same file `:255-258`: "## Lead sign-off (the owed square-up, 2026-11-13)" / "**Verdict: PASS (Lead-signed 2026-11-13).** The signature line above stood PENDING from 2026-10-24 while the program proceeded on exactly this evidence — Phase 4 launched on it the same day…" |
| Truth + proof | B — the date is backdated; phases 4–6 proceeded on unsigned evidence for ~3 weeks. W17-A03/A06. |

### C-027 [S2] phase3-exit: 4h-soak debt "subsequently discharged … soak reports now live under docs/perf/" vs the cited report: "has NOT been taken" (corpus)
| A | `docs/gates/phase3-exit.md:265-267`: "The gate's own named debts (oracle PENDING-VLC, 4 h soak, …) were subsequently discharged by the phases that own them — soak reports now live under `docs/perf/` (soak-nightly-20261109)…" |
| B | `docs/perf/soak-nightly-20261109.md:3-5`: "Status: **planning + workflow are real and committed; the 4 h measurement itself has NOT been taken** — it is the nightly job's job…" |
| Truth + proof | B — the debt was not discharged by the named artifact; the 4h tier is still `PENDING first scheduled run` (:19) and billing-blocked per C-023. W17-A03/A06. |

### C-028 [S0] 99_run_all.sh: "JG … its exit code fails the tier (and therefore the CI job)" vs a registered gate that structurally cannot fail (corpus)
| A | `scripts/quality/99_run_all.sh:16-18`: `#   JG = run a GATE: its exit code fails the tier (and therefore the CI job).` / `# Without this, every tier exited 0 no matter what a script reported - a CI that cannot fail is theatre.` |
| B | `scripts/quality/23_alloc_budget.sh:131-135`: `sys.exit(1)` followed by `say ""` / `say "written: …"` (script runs `set -uo pipefail`, no `-e`; last command wins) |
| Truth + proof | B. Executed repro (W20-A02 F-6): `ALLOC_RATIO_MAX=0.01` → "OVER THRESHOLD: 7 over threshold" printed, `SCRIPT_EXIT=0`. The JG contract is a lie for this gate. |

### C-029 [S0] Doc 10's gate table: clippy "-D warnings … any warning fails" vs the CI clippy lane that cannot fail (corpus)
| A | `docs/10-testing-ci-quality.md:79`: `\| 3 \| cargo clippy --workspace --all-targets --all-features -- -D warnings \| the lint policy of §3 …, any warning fails \|` |
| B | `.github/workflows/rust-clippy.yml:44-49`: `run: cargo clippy --all-features --message-format=json \| clippy-sarif \| tee … \| sarif-fmt` with `continue-on-error: true` |
| Truth + proof | the lane — no `-D warnings`, `continue-on-error: true`: the CI clippy job can NEVER fail on lint findings. W17-A04 (S0-class). |

### C-030 [S2] Committed phase-exit fmt evidence "FMT_EXIT=0" vs the tree's fmt failure today (corpus)
| A | `docs/gates/logs/phase7-exit-fmt.log:5` (committed evidence; phase7-exit.md:44 cites it as PASS): `FMT_EXIT=0` |
| B | `audit/runs/baseline/cargo-fmt.txt`: `# cmd: cargo fmt --check (workspace)` + `Diff in /home/z/my-project/audit-workspace/crates/plugin-api/tests/testkit_wiring.rs:40:` (exit 1; 2 files drifted) |
| Truth + proof | B — the tree regressed after the gates were cut; committed "exit" evidence is stale-green. W17-A04/A06. |

### C-031 [S2] Committed wave-3 merge log & phase-3 trajectory "0 failed" vs today's 2 failed (corpus)
| A | `docs/gates/phase3-exit.md:83-84`: `- Trajectory: 1329 (Phase-2 exit) → 1718/0/20 (wave-3 merge log) → 1919/0 (worklog 7.LEAD-RESET @ 9393b53) → **1939/0/36 (this gate run)**.` |
| B | `audit/runs/baseline/cargo-test-workspace.txt:2791`: `test result: FAILED. 1 passed; 2 failed; 1 ignored; …` |
| Truth + proof | B (committed_transcripts_match_fresh_runs + final_frame_is_current fail at HEAD). The all-green trajectory describes a tree that no longer exists. W17-A04/A06. |

### C-032 [S2] "4744 passed / 2 failed" headline vs the first-vertical package abort that hides 3 more test binaries (corpus)
| A | `audit/waves/wave-00/AGENT-SPEC.md:202-203` (baseline headline echoed in wave summaries): "cargo test per-package: 4744 passed / 2 failed (first-vertical: committed_transcripts_match_fresh_runs, final_frame_is_current) / 51 ignored." |
| B | `audit/runs/baseline/cargo-test-workspace.txt:2794-2795`: `### PACKAGE first-vertical exit_code=101` / `### PACKAGE first-vertical FINISHED (real failure ec=101)` |
| Truth + proof | the headline UNDERCOUNTS. `audit/runs/INCIDENT-LOG.md:16-20` preserves the lost-run reports: recovery_matrix 22/1 FAIL, journey 6/1 FAIL — "4744 passed / 2 failed" UNDERCOUNTS failures. W16-A06. |

### C-033 [S2] worklog: "main should be fully green again" vs the baseline's 2 standing failures (corpus)
| A | `worklog.md:2090-2091`: "Stage Summary:" / "- main should be fully green again (gate + Rust lanes rode the single pre-existing evidence drift…)." |
| B | `audit/runs/baseline/cargo-test-workspace.txt:2791`: `test result: FAILED. … 2 failed` (the exact pair the worklog claims resolved) |
| Truth + proof | B — on the audited tree the claim is FALSE. W20 doc-audit ledger (worklog claim 4). |

### C-034 [S2] Two repo panic policies give opposite verdicts on the same demux panic sites (corpus)
| A | `tools/xtask/src/cmd/check_panics.rs` (real scanner; rg for `check-panics` in .github/+scripts/ = 0 hits — wired nowhere; its allowlists cover only input/media-probe contract.rs) |
| B | `docs/audit/panic-baseline.json:34-40` (committed ratchet consumed by 24_panic_budget.sh, a JG gate): `"crates/demux": { "unreachable": 2, "panic": 0, "unwrap": 0, … }` |
| Truth + proof | both tools are real but disagree: check_panics FAILS its own tree (2 demux `unreachable!` sites) while the ratchet freezes the same sites as acceptable baseline. One gate dead, one contradictory. W01-A10. |

### C-035 [S3] CORPUS.md: "Every file … authored in-tree … no external encoder output" vs the unaccounted families on disk (corpus)
| A | `tests/fixtures/CORPUS.md:10-16`: "Every file under `generated/` and `expectations/` is authored in-tree by the deterministic generator … no external encoder output — ffmpeg was used only as a local cross-check oracle…" |
| B | `tests/fixtures/generated/MANIFEST.toml`: 28 sha256-pinned rows vs ~91-116 files on disk (ogg + HLS families unpinned/unaccounted) |
| Truth + proof | the manifest/disk. W16-A02/A03/A06: "CORPUS.md 'authored in-tree / no external encoder output' is FALSE for the ogg and HLS families." |

### C-036 [S2] fuzz/README: "Status: SCAFFOLDED, NOT WIRED — one Lead decision outstanding" vs the wired nightly fuzz job (corpus)
| A | `fuzz/README.md:17-19`: `## Status: SCAFFOLDED, NOT WIRED — one Lead decision outstanding` |
| B | `.github/workflows/audit.yml:325-352`: `fuzz:` job, `if: github.event.schedule == '0 2 * * *'`, 5-shard matrix, `tool: cargo-fuzz`, `bash scripts/quality/25_fuzz_campaign.sh 180 …` |
| Truth + proof | the workflow — the nightly fuzz job is fully wired (5 shards × 180s). Whether it ever ran is the billing question (C-005), but "NOT WIRED" is false as stated. W16-A04. |

### C-037 [S3] ADR-004 pins tauri 2.12.4 / wry 0.57.0; the shipped closure is tauri 2.12.5 / wry 0.57.2 (corpus)
| A | `docs/adr/ADR-004-os-webview.md:47-48`: "pinned the candidates: **tauri 2.12.4** and **wry 0.57.0**…" |
| B | `apps/desktop-tauri/Cargo.lock` (extracted): `wry 0.57.2` / `tauri 2.12.5` |
| Truth + proof | the lock. W14-A01/A06 (version claims verified in lock; ADR still says 0.57.0 everywhere). |

### C-038 [S3] Window size: tauri.conf.json 1280x800 vs product.toml 1280x720 (corpus)
| A | `apps/desktop-tauri/tauri.conf.json:15-16`: `"width": 1280,` / `"height": 800,` |
| B | `product.toml:64-65`: `width = 1280 … height = 720` (:58 "The default window is 1280x720 — same 16:9 shape…") |
| Truth + proof | they simply disagree — 80px of first-paint flash every launch; `remember_geometry` parsed but dropped (no persistence exists, C-040). W14-A08. |

### C-039 [S2] Doc 11 SEC-07: "Dev-only override via LUMEN_DEV_INSECURE_TLS=1" vs a tls_config that reads no env (corpus)
| A | `docs/11-dependency-security-compliance.md:494`: "Dev-only override via `LUMEN_DEV_INSECURE_TLS=1`, refused in release builds (`debug_assertions`-gated compile-out) `[PROPOSAL]`" |
| B | `crates/streaming/src/client.rs:91-96`: `pub(crate) fn tls_config() -> TlsConfig { TlsConfig::builder().provider(TlsProvider::Rustls).root_certs(RootCerts::WebPki).build() }` |
| Truth + proof | the CODE — no env read, no disable path in any build profile; the documented override is PAPER-ONLY (direction safe: verification is MORE always-on than documented). W01-A04 → W02-A05/W02-A09. |

### C-040 [S2] Doc 07: "persisted settings written by `config.set-setting`" vs a session-local BTreeMap (corpus)
| A | `docs/07-config-driven-system.md:466`: `\| 4 \| **User preferences** \| persisted settings written by \`config.set-setting\` \| the user \|` |
| B | `apps/cli/src/host.rs:1404` (whole write path; `self.settings: BTreeMap<String, SettingValue>` at :181): `self.settings.insert(full, value.clone());` |
| Truth + proof | the CODE — NO persistence exists anywhere in the tree; the UI's persistence is a localStorage mock nothing reads back. W11-A05/A07. |

### C-041 [S0] settings-help.txt advertises 121 `--<area>-<key>` CLI flags; the parser knows none of them (corpus)
| A | `schemas/generated/settings-help.txt:7-8` (122 `^  --` lines in the file): "Settings surface as generated long flags (--<area>-<key> <value>)." / "`lumen --help` lists areas with counts; `lumen --help <area>` lists that" |
| B | `apps/cli/src/parse.rs:156-159` (the complete global flag set; no settings-flag arm exists anywhere): `"--json" => invocation.json = true,` / `"--control-stdio" => invocation.control_stdio = true,` / `"--version" \| "-v" => invocation.version = true,` / `"-h" \| "--help" => {` |
| Truth + proof | the parser. Executed W11-A04: `lumen --video-hue 50` → exit 64 "unknown command". The generated help describes a CLI surface that does not exist. |

### C-042 [S0] In-tree test asserts "an mp4 must refuse in this build" vs the shipped CLI that opens MP4 (corpus)
| A | `crates/playback-engine/tests/scenarios_b/scenario_01_open.rs:117` (harness-local AVI-only sniff): `Ok(_) => unreachable!("an mp4 must refuse in this build"),` |
| B | `audit/runs/baseline/mp4-smoke.md:9-20` (shipped release binary): `event: state-changed Idle → Opening … Buffering → Playing … Stopped` / `exit_code=0` |
| Truth + proof | the product — the test proves the opposite of the shipped behavior and still passes because it drives a host whose probe registry was seeded AVI-only. W03-A02 (executed). |

### C-043 [S2] TS probe comments: "a whole 188-byte file is also accepted" vs the executed panic (corpus)
| A | `crates/demux/src/ts/mod.rs:315-322` comment: "a whole 188-byte file is also accepted — the walk will tell" (twin comment in `crates/media-probe/src/signature/phase5.rs`) |
| B | executed (W20-A01 F-1): `./target/release/lumen open ts188.bin` → `exit=101 … panicked at crates/media-probe/src/signature/phase5.rs:161:16: index out of bounds: the len is 188 but the index is 188` |
| Truth + proof | the code's behavior — the process aborts; "accepted" is false. W20-A01 F-1 (CONFIRMED, executed this audit). |

### C-044 [S2] mkv/cues.rs doc-comment: "degrades to SeekSupport::SlowLinear" vs the code that refuses to open (corpus)
| A | `crates/demux/src/mkv/cues.rs:11-14` (comment): "more than `MAX_CUE_ENTRIES` entries is the typed `cue_table_too_large` refusal and the demuxer degrades to `SeekSupport::SlowLinear` — never an unbounded allocation." |
| B | `crates/demux/src/mkv/cues.rs:91-95` (code; `rg cue_table_too_large` in mkv/mod.rs = no catch site): `if index.points.len() >= MAX_CUE_ENTRIES { return Err(error::cue_table_too_large(…)); }` |
| Truth + proof | B — the file refuses to open; there is no degrade path. W05-A09/A10. |

### C-045 [S2] XSPF doc-comment: "extension content … ignored" vs the hijack it performs (corpus)
| A | `crates/playlists/src/xspf.rs:18-21` (comment): "Unknown elements anywhere (extension content, foreign namespaces) are ignored, as the XSPF spec instructs consumers to do." |
| B | executed (W10-A08 probe proof, wave-10 summary): "`<app:location>` in `<extension>` hijacks entry location; 'extension content ignored' doc FALSE" — the parser reads locations out of extension blocks, changing the entry's target URL |
| Truth + proof | B — probe-proven. Proof: audit/waves/wave-10/SUMMARY.md + W10-A08 log. |

### C-046 [S2] Doc 01 §2.5 "no secrets in logs/errors" vs credentials printed verbatim (corpus)
| A | doc 01 §2.5 policy (as recorded in wave-02 summary): no secrets in logs/errors |
| B | wave-02 summary: "credentials print verbatim through Display/Debug, contradicting the [no-secrets] policy" |
| Truth + proof | the CODE — Display/Debug impls render credential material. Proof: audit/waves/wave-02/SUMMARY.md finding + W02 ledgers. |

---

## The three worst

1. **C-002** — one shipped binary whose four self-description surfaces (engine-info, decode-bench, refusal text, dispatch) give four different decoder inventories; the instrument lie contaminated the audit's own Wave-0 litmus ("zero decode" was bench's artifact).
2. **C-003** — the seek story: matrix says implemented, MediaInfo says `seekable: true`, CLI info says `seekable: yes`, and every MP4 in-session seek fails at `plugin.demux.reanchor.unsupported` — one user action, three simultaneous lies, executed proof on both MP4 samples.
3. **C-008 (+ C-009, C-015, C-012)** — the status layer drifts in BOTH directions around the truth: ~4,620-5,350 lines of landed, tested, executed streaming code marked `not_started`, registered tested UI areas marked `not_started`, the shipped file protocol marked `not_started` — while an unreachable ASF demuxer is marked `implemented` on two platforms.

## Provenance / method note
- Sources consolidated: audit/waves/wave-{01..19}/SUMMARY.md, audit/verify/W20-A0{1,2,3}-reverification.md, audit/docs/**.audit.md claim ledgers, and the prior-run register (its quotes were sed-verified by that run; the TOP 12 were re-verified fresh this run — commands in audit/runs/W20-A08.log.md).
- REFUTED during consolidation (excluded): "apps/web/src/io/mod.rs cites a nonexistent fixture" (W12-A06's S0) — W20-A02 F-9 executed the cited proof (`fetch_lane_serves_the_real_fixture_to_the_demuxer` … 1 passed) and the fixture exists (425,516 B, MANIFEST-declared). Not a repo contradiction.
- Severity tally: S0=15, S1=0, S2=26, S3=5 (46 entries). Tiers: 1 user-facing lies (C-001..C-006), 2 status-layer drift (C-007..C-021), 3 process/paper (C-022..C-046).
