# REMEDIATION MAP — Devil Audit §8.1 (W20-A09)

| Field | Value |
|---|---|
| Agent | W20-A09 |
| Date | 2026-11-26 |
| Question | What stands between this repo and "an MP4 plays with picture and sound on Windows"? |
| Sources | Wave summaries 00–19 + W20-A02/A03/A04 reverification reports + W20-A05 log + my own sed/rg/exec verification (marked **[A09-exec]** / **[A09-sed]**) |
| Rule | NOT a fix plan — a ranked map: blocker, evidence, size, dependency order |

My own first-hand runs this session (target/release/lumen, samples/):
- `seek 1s` on `samples/sample.mp4` → `TimingSeekFailed … "[demux] unsupported (plugin.demux.reanchor.unsupported)"`, session still walks Playing→Stopped, exit 0. **[A09-exec]**
- `engine-info` → `decoders: mjpeg, pcm_s16le`. **[A09-exec]**
- `decode-bench samples/sample.mp4` → `not benched (no decoder in this build (first vertical: mjpeg + pcm))`, `v_frames=0 v_decoded=0 a_frames=0 a_decoded=0`. **[A09-exec]**
- `decode-bench tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` → `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60 wall_s=0.008`. **[A09-exec]**
- `strings target/release/lumen` → `…/rust_h264-0.6.1/src/decode_cabac.rs` (+ simd/chroma/slice_context paths). **[A09-exec]**

---

## 1. THE BLOCKERS (ranked by user-visible impact)

### B1 — Windows display surface: the tauri app is compile-broken [S0]
The shipped GUI cannot compile on any host; every "picture" claim downstream of it is untestable.
Evidence (all **[A09-sed]** unless noted):
- `apps/desktop-tauri/src/app.rs:301`:
  ```rust
              spawn_session_pump(app.handle().clone());
  ```
  The symbol exists nowhere else in the crate (rg: exactly one hit — the call site). The defined sibling is `fn spawn_event_pump(app: AppHandle)` at `app.rs:190`. → E0425, one-line class. Static proof is decisive even though the sandbox cannot compile the crate (gdk-sys, AGENT-SPEC §13) — W20-A02 F4, W13-A10, W14 (4 independent confirmations).
- Workspace-excluded: root `Cargo.toml:41` `members = ["crates/*", "apps/*", …]` + `Cargo.toml:51`:
  ```toml
  exclude = ["apps/desktop-tauri"]
  ```
  → the green release lane (`cargo build --release --locked -p lumen-cli -p desktop`) never builds it.
- CI-unproven + claim false: `apps/desktop-tauri/INFRA-BLOCKED.md:4-5` claims "the crate compiles in its dedicated CI lane (`tauri-check`, Linux + webkit2gtk)" — but `.github/workflows/ci.yml:730-731` runs:
  ```yaml
        - name: Test the admission crate (incl. the §7.4 anti-bypass audit)
          run: cargo test --manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets
  ```
  which compiles the lib → red on this tree. (Also `single_command.rs` — the "live enforcement" test — is a text scan of a crate that cannot link; W14-A05.)
- The frame path once working (after the 1-line fix) is CPU memcpy → putImageData, no GPU:
  engine → FrameTap → `apps/desktop/src/mailbox.rs:56` (`capacity: TWO` — the only backpressure) → 10ms ticker (`app.rs:109` `const SESSION_TICK: Duration = Duration::from_millis(10);`) ships **only** `records.last()` (`app.rs:223-229`: `if let (Some(channel), Some(record)) = (&frame_channel, records.last())`) → tauri Channel (raw bytes; `frame_channel.rs` docs: "a 1280×720 RGBA frame is ~3.7 MB") → JS `ArrayBuffer` → `ui/src/components/player/MediaSurface.tsx:227`:
  ```ts
    ctx.putImageData(image, 0, 0);
  ```
  W14-A07 additionally found the older fetch-token queue UNBOUNDED (stalled webview = OOM-class ~370MB/s) — audit-corpus citation; the w31.1 Channel lane re-verified above is the current shape.
Size: the E0425 fix itself is **S (1 LOC, <1 h)** — rename to `spawn_event_pump` (or define the pump). Verifying the whole frame lane on a real Windows/webview host (mailbox drops, 10ms cadence, channel stalls) = **M**.

### B2 — Windows audio: no output path exists [S0]
"Sound" on Windows today is architecturally impossible: every shipped host renders into NullBackend.
Evidence:
- `apps/cli/src/pipeline_host.rs:1599` **[A09-sed]** (the CLI lane; `apps/desktop/src/dispatch.rs:67` reuses `CliHost`, and `apps/desktop-tauri/Cargo.toml:53` `desktop = { path = "../desktop" }` chains the GUI to the same construction):
  ```rust
          NullBackend::open_stream(&format, RING_FRAMES, NullConfig { capture: false })?;
  ```
- `crates/platform-windows/src/windows/wasapi.rs` (787 LOC): rg `IAudioClient|IAudioRenderClient` → **0 hits** **[A09-exec]**. The FFI surface is enumeration/notifications only (`list_endpoints`, `enumerate_with`, `describe_device`, `client_on_device_*` callbacks, `subscribe`/`poll`). No render activation, no buffer scheduling.
- platform-windows is an ORPHAN crate: zero workspace dependents (W08-A01 S0; W20-A02 F8 CONFIRMED by manifest scan). Its latent defects must be fixed *before* wiring: `com.rs` PropVariant pinned at 16 bytes vs 24 on x64 (W08-A10; W20-A03 F2 executed size_of=16); dxgi vtables missing the 4 IDXGIObject slots — `get_desc1` actually calls `EnumOutputs` (W08-A10; W20-A02 F7).
- `crates/audio-output/src/cpal.rs` (138 LOC, zero cpal code): `cpal.rs:43` `pub const AUDIO_ADR005_ADMITTED: bool = false;`, the compile-time assert at `:48-52`, and `:98-107`:
  ```rust
  pub fn cpal_backend() -> Result<Box<dyn AudioBackend>, BackendNotAdmitted> {
      if !AUDIO_ADR005_ADMITTED {
          return Err(BackendNotAdmitted::CPAL);
      }
      unreachable!("the ADR-005 admission change replaces this arm")
  ```
  Flip the const alone → build breaks (assert); flip + delete assert → `unreachable!()` panic. There is no cpal adapter to admit (W08-A04).
- Even the audio *experience* knobs are dead: `audio.set-volume` refused by every host, EQ real math wired to nothing (W07-A02/A06; W20-A03 F7).
Size: **L**. Two routes, both large: (a) finish WASAPI render in platform-windows — IAudioClient/IAudioRenderClient/Event-driven mode, format negotiation, the ~800–1,500 LOC of new COM FFI + AudioBackend adapter + pipeline_host substitution + ADR-005 machinery + Windows hardware testing (substrate: 3,093 LOC of real platform-windows FFI already exists); or (b) admit cpal per the file's own one-change discipline (const + adapter + ADR amendment + allowlist row) — smaller code (~200–400 LOC) but Lead-gated and dependency-policy-bound. Est. band 300–1,500+ LOC and multi-day-per-piece; the dominant cost of the whole critical path.

### B3 — MP4 in-session seek: reanchor unimplemented [S0]
Evidence:
- The engine's only seek entry: `crates/playback-engine/src/seek.rs:389-390` **[A09-sed]**:
  ```rust
      // Step 4 — the re-anchor (D4: the demuxer commits only on
      // success).
      match pipeline.demuxer_reanchor(target_span(target_ns)) {
  ```
  → `pipeline.rs:1498-1499` `self.demuxer.reanchor(target)` → trait default `crates/plugin-api/src/demux.rs:113-120` returns `plugin.demux.reanchor.unsupported`.
- `crates/demux/src/mp4/mod.rs:2008` `impl Demuxer for Mp4Demuxer` has `fn seek(&mut self, target: TimeSpan)` at `:2050` and `SeekSupport::Fast { indexed: true }` at `:2046` — but **no `fn reanchor` anywhere in the file** (rg, 0 hits) **[A09-exec]**. The fully-tested `seek()` is engine-unreachable; MediaInfo reports seekable:true (W05-A01 S0; W20-A02 F1).
- **[A09-exec]** my own run above: `TimingSeekFailed … plugin.demux.reanchor.unsupported`. Control: WAV seek completes (W20-A02 F1 executed: `TimingSeekCompleted(1000000)`).
Size: **M** — the index machinery (stbl/stco/stsc/stts, `seek()`) exists and is tested; reanchor = commit-position + pending-flush + event re-priming on top, ~150–400 LOC + tests. UX blocker, not a playback blocker (play-to-end works).

### B4 — The registry lies: three instruments disagree, all wrong [S0]
Evidence **[A09-sed]** + **[A09-exec]**:
- `apps/cli/src/host.rs:1643-1658`: hardcoded `decoders: vec![ … "mjpeg" … "pcm_s16le" … ]` → engine-info prints 2.
- `apps/cli/src/bench.rs:300` `if … fact.codec == "mjpeg"`, `:313` `fact.codec.starts_with("pcm_")`, `:334` `"no decoder in this build (first vertical: mjpeg + pcm)"` → decode-bench reports 0 frames **for a file the same binary plays** (my run above).
- `apps/cli/src/pipeline_host.rs:372-378`:
  ```rust
                  "no decoder for `{other}` in this build (landed: mjpeg, pcm, flac, \
                   mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264)"
  ```
  omits wired vp9 (`:313`) and dnxhd (`:326`); the dispatch table has 16 arms incl. `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)` at `:346` and `"aac" | "aac-lc"` at `:288-292`.
- Four registries, four answers: 2 vs 16 arms vs 2-with-stale-message vs 12 (W13/F9; W20-A02 F3 executed; W03-A02 — Wave-0's "MP4 decodes zero frames" litmus was this instrument's artifact).
Size: **S** (~50–150 LOC): make engine-info/bench/landed-string derive from the one true dispatch registry (pipeline_host) instead of three hand-maintained lists.

### B5 — Panic paths reachable from `open` [S1 cluster, small fixes each]
1. TS-188 probe panic (executed, exit 101): `crates/media-probe/src/signature/phase5.rs:159-161` **[A09-sed]**:
   ```rust
           if prefix.len() >= TS_PACKET_BYTES {
               if prefix[TS_PACKET_BYTES] != 0x47 {
   ```
   indexes `prefix[188]` on a 188-byte window. Twin: `crates/demux/src/ts/mod.rs:315-322` `header.len() < PACKET_BYTES || header[PACKET_BYTES] == 0x47` — whose own comment "(a whole 188-byte file is also accepted — the walk will tell)" is false. W20-A01 F1 re-executed (188B → exit 101; 187B/189B → typed 65).
2. MP4 stts/ctts unclamped loops (CPU DoS on `open`): `crates/demux/src/mp4/mod.rs:1253-1254` and `:1316-1317` **[A09-sed]**:
   ```rust
                   let count = u32be(body, 4).unwrap_or(0) as usize;
                   for i in 0..count {
   ```
   while every sibling clamps (stsz `:1231` `for i in 0..count.min(MAX_SAMPLES)`, stco `:1270` `let take = count.min(MAX_SAMPLES);`). ~4.3e9 iterations per 4-byte field (W05-A01; W20-A01 F3 re-derived).
3. MKV lacing inversion: `crates/demux/src/mkv/cluster.rs:258` **[A09-sed]** `let frame_count = body[0] as usize;` — spec: frames = byte+1, in Fixed/Xiph/EBML lanes; ffmpeg-legal blocks mis-decode; **3 in-tree tests pin the bug** (W05-A06; W16).
4. MKV window-slide livelock: `crates/demux/src/mkv/mod.rs:644-663` **[A09-sed]** — block size in (4MiB, 32MiB] re-seeks/re-reads the same 4MiB window forever (W05-A06 executed watchdog kill; W20-A01 F4 static re-derivation: "No exit condition changes: livelock").
5. ts.rs adts loop: `apps/cli/src/containers/ts.rs:354-376` **[A09-sed]** — `at = end` with `end = at + frame_length`; `AdtsHeader::parse` (adts/mod.rs:192-195) only checks the syncword ("field validation is the caller's policy"), so `frame_length == 0` never advances `at` → infinite loop + unbounded queue (W13-A08 replica-proof: 1e6 zero packets).
Size: **S each** (guards are 1–4 LOC; the lacing fix must also correct the 3 pinning tests; livelock exit-condition is the only subtle one). Total ≈ 20–60 LOC.

### B6 — Doc/status layer reconciliation [S0-class paperwork]
- Billing contradiction: `docs/PARITY-REPORT.md:122-125` "CI is billing-blocked … every GitHub Actions run dies at 0 steps" **[A09-sed]** vs `docs/RELEASE.md:145-148` "Billing was unblocked 2026-11-12 … The workflow has executed (v0.1.0–v0.1.2)" **[A09-sed]** (W14-A04; W17 proves 6 in-tree run IDs).
- INFRA-BLOCKED.md "compiles" vs E0425 (B1).
- Matrix understatement: `docs/feature-matrix.toml` `network-http-client` row **[A09-sed]**:
  ```toml
  status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }
  evidence = ""
  ```
  while `apps/cli/src/demux_host.rs:474-496` ships the real ureq/ranged HTTP lane **[A09-sed]**; `network-reconnect-buffering` likewise not_started for ~4,900 LOC of landed, tested code (W18 S0; W20-A02 F10 executed: streaming lib 212 passed). Rows 084/095/099/100/101 same family; ~10 UI rows not_started-but-implemented (W19-A03).
- not_started-understatement hides the fact that most of the MP4 problem is display+audio, not decode (B1/B2) — the docs make re-planning impossible without this audit.
Size: **S** (text edits, dozens of rows + one driver script); effort hours, value high (it is the planning layer).

---

## 2. DEPENDENCY ORDER (critical path to "MP4 plays with picture and sound on Windows")

1. **Fix E0425** (rename to `spawn_event_pump`) — the hard gate. Nothing Windows-GUI can compile, test, or CI-prove until this lands. One line; do it in the same change as re-pointing the INFRA-BLOCKED claim.
2. **Wire a Windows audio output** — the largest single gap and the "sound" half. Either finish platform-windows WASAPI render (fix PropVariant 16→24 and dxgi slots while there) or land the cpal admission change (const + adapter + ADR-005 amendment + allowlist row). Replaces `pipeline_host.rs:1599` NullBackend for cfg(windows). Start immediately — longest lead item.
3. **Verify the tauri frame channel on Windows** (needs 1): build the crate on a webview host, open the sample MP4, watch mailbox→10ms ticker→Channel→`MediaSurface.tsx:227` putImageData; fix what the hardware run exposes (records.last() frame drops, cap-2 mailbox, W14-A07's unbounded-queue class of stall). Path exists end-to-end — this is verification + backpressure hardening, not new architecture.
4. **MP4 reanchor** (needs nothing; UX): implement reanchor on `Mp4Demuxer` reusing the existing `seek()`/index machinery so in-session seek stops returning `plugin.demux.reanchor.unsupported`.
5. **Registry strings** (needs nothing): single-source engine-info/bench/landed from the dispatch table — stops the instrument from lying during all subsequent verification (my decode-bench run above would have shown h264 frames).
6. **Panic guards** (needs nothing; do before wide dogfooding): TS-188 bounds, stts/ctts clamp, MKV lacing+tests, MKV livelock exit, ts adts frame_length==0 guard.
7. **Doc/status reconciliation** (last): billing contradiction, understated matrix rows, INFRA-BLOCKED, risk-register — so the next planner reads reality.

(2)+(3) are the two halves of the goal and can run in parallel after 1; audio dominates effort. 4–7 do not gate first playback but gate trust; 6 gates any hostile-input exposure.

---

## 3. SIZE TABLE (justified from audited code)

| # | Blocker | Size | LOC est. | Justification |
|---|---|---|---|---|
| 1 | E0425 | S (trivial) | 1 | Single symbol rename; call site `app.rs:301`, definition `app.rs:190` |
| 1b | Windows frame-lane verify/harden | M | 50–300 + hw cycles | Lane exists (mailbox/ticker/channel/putImageData verified); work = backpressure + on-metal testing |
| 2 | Windows audio | L | 300–1,500+ | cpal route ~200–400 LOC + ADR machinery; WASAPI route +800–1,500 LOC COM FFI (3,093 LOC substrate exists, 0 render LOC); plus PropVariant/dxgi fixes, pipeline_host substitution, hw test |
| 3 | MP4 reanchor | M | 150–400 | Index + tested `seek()` exist at `mp4/mod.rs:2050`; reanchor = commit + flush + re-prime on top |
| 4 | Registry strings | S | 50–150 | Three hand lists → one source; my runs show the lie end-to-end |
| 5 | Panic cluster | S | 20–60 | Guards are 1–4 LOC each; lacing needs 3 tests corrected; livelock needs an exit condition |
| 6 | Docs/status | S | text | Rows + 2 contradictions; mechanical |
| — | **TOTAL (functional 1–5)** | — | **~500–2,400 LOC** | Plus mandatory Windows hardware cycles; audio dominates |

Effort band: **~1–2 days to a compiled GUI; ~2–6 engineer-weeks to picture+sound on Windows**, dominated by B2 (audio). A cpal-admission shortcut could compress B2 to days if the Lead accepts the dependency.

---

## 4. ALREADY DONE — DO NOT REDO (the understatement side)

- **H.264+AAC decode works in the shipped binary.** Dispatch arms `pipeline_host.rs:346` (h264|avc1 → H264SwDecoder) and `:288-292` (aac/aac-lc); `Cargo.lock:769` rust_h264 0.6.1, `:1015` symphonia-codec-aac; my `strings` on the release binary shows rust_h264-0.6.1 source paths; my open run walks sample.mp4 to Playing (exit 0). Wave-0's "zero decode" was the bench instrument's artifact (W03-A02).
- **MP4 progressive demux is REAL** (boxes/tracks/avcC/esds ride stbl; open+play-to-null executes end-to-end — W05/W13 trace). Seek is the gap, not open.
- **HTTP(S) client works**: `demux_host.rs:474-496` open_http_source → ureq 3.5.0 + rustls lane (W10 executed; W20-A02 F10: streaming lib 212 tests pass). Matrix row says not_started — false.
- **AVI/MJPEG/PCM vertical is genuinely solid**: my decode-bench 60/60 v, 60/60 a, 8 ms wall; rc-smoke headless-decode PASS (baseline §13); fixtures digested.
- **The playback engine is real**: 7-step in-pipeline seek machinery, clocks, queues (W03); the CLI's WAV seek completes (W20-A02 F1).
- **14-decoder dispatch registry** (16 arms) — the true registry (W13/F9).
- **~4,900 LOC of reconnect/auth/remote-playlist code landed and tested** (rows 099/100/101 marked not_started — false; W18 S0, W20-A02 F10 executed).
- **platform-windows WASAPI enumeration/notification FFI (787 LOC), COM (415), DXGI (264)** — real substrate; needs render + two ABI fixes, not a rewrite.
- **UI bridge + canvas presenter real**: MediaSurface.tsx:227 putImageData behind the (compile-broken) tauri shell; lumen_dispatch IPC verified (W15).
- **Release lane is real and disciplined** (W17: six run IDs, SHA-pinned tools, honest unsigned disclosures).
- **Inventory/census doc layer is bulletproof** (W18/W19 re-derivation byte-identical).

---

## 5. UNVERIFIED / blocked in this sandbox
- desktop-tauri compile execution: gdk-sys (AGENT-SPEC §13) — E0425 stands on static proof (rg: symbol defined nowhere).
- Windows/macOS runtime behavior of WASAPI/dxgi/coreaudio: no SDKs here; all platform-* claims are static-arithmetic (W20-A02 F7, W20-A03 F2/F3).
- W14-A07's unbounded fetch-token queue: audited against the pre-w31.1 lane; current Channel lane re-verified structurally only.
- W20-A05's full MP4 reality trace was never completed (log stops after the plan); this map subsumes its intended scope with W20-A02/A03 executions + my own runs.
