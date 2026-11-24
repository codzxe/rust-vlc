# MP4 REALITY TRACE — CLI (W20-A05)

| Field | Value |
|---|---|
| Auditor | W20-A05 (Devil Audit, §8.1 end-to-end reality trace) |
| Date | 2026-11-24 |
| Binary | `target/release/lumen` (md5 c03f7887ec970935842296140ec47fb1, 11.6 MB, built 19:59 — newer than all samples) |
| Samples | `/home/z/my-project/samples/`: sample.mp4 (moov@86752, moov-at-end), sample-faststart.mp4 (moov@32), sample-hevc.mp4, sample-aac-only.mp4, sample.wav (box offsets: audit/runs/baseline/mp4-smoke.md:24) |
| Spec | audit/waves/wave-00/AGENT-SPEC.md |

Blunt verdict: **the open path is REAL and works end-to-end into NULL sinks** — open,
probe, demux (moov-at-end AND faststart), avcC/esds-ASC harvest, h264 decode (rust_h264),
aac decode (symphonia), I420→RGBA, scheduling, headless wgpu present, full-duration play
to `end-of-media` at 100.0%, exit 0. **What "user watches an MP4" lacks is the last
quarter-mile**: in-session seek always fails (MP4 reanchor unimplemented), audio renders
into a null sink, the renderer is headless with no window anywhere in the shipped build,
and HEVC content is refused. All four are typed-honest gaps, not silent frauds — except
the two capability reporters (`engine-info`, `decode-bench`) which lie about the same
binary that just played h264+aac to 100%.

---

## 1. CLI trace — per variant

All runs: `timeout 30 ./target/release/lumen open <sample> </dev/null` (stdin-closed),
`tail -f /dev/null |` (stdin-held), scripted stdin (`seek`, `time`, `advance`, `info`,
`tracks`, `engine-info`), `decode-bench <sample>`.

| Variant | open exit | State walk (stdin closed) | stdin-held (30 s) | seek 1s | advance-to-end | decode-bench | engine-info |
|---|---|---|---|---|---|---|---|
| sample.mp4 (moov-at-end) | 0 | Idle→Opening→Buffering→Playing→Stopped→Idle | killed at 30 s (`EXIT=124`, `user 0m0.014s`), stuck in Playing, time frozen 0.000000 s | FAIL `plugin.demux.reanchor.unsupported`, position stays 0 | 100.0% (3.041814 s), `end-of-media`, exit 0 | v_frames=0 `not benched (no decoder in this build (first vertical: mjpeg + pcm))` | `decoders: mjpeg, pcm_s16le` |
| sample-faststart.mp4 | 0 | identical walk | stalled in Playing, EXIT=124 (8 s check) — no pump between commands | FAIL (same) | 100.0% (3.041814 s), exit 0 | same refusal | same |
| sample-hevc.mp4 | **65** | Idle→Opening→**Error**→Idle | — (open itself fails) | — (no session) | — | v_frames=0 refusal | same |
| sample-aac-only.mp4 | 0 | Idle→Opening→Buffering→Playing→Stopped→Idle | stalled in Playing, EXIT=124 (8 s check) | FAIL (same) | 100.0% (3.041814 s), exit 0 | a_frames=0 refusal | same |
| sample.wav | 0 | identical walk | stalled in Playing, EXIT=124 (8 s check) | **WORKS**: `TimingSeekCompleted { position: Timestamp(1000000), seek_kind: Command }`, time-tick 1.000000 s (33.3%) | 100.0% (3.000000 s), exit 0 | `a_frames=5 a_decoded=5` (the one honest bench lane) | same |

Trimmed command outputs (all executed this run):

```
$ timeout 30 ./target/release/lumen open samples/sample.mp4 </dev/null   # exit 0
ok: accepted session=1 gen=0
event: state-changed Idle → Opening / Opening → Buffering / Buffering → Playing
event: TimingDurationKnown { duration: Timestamp(3000000) }
event: MediaInfoReady { … tracks: [Video h264 (320x240 via info), Audio aac 1ch 44100Hz …] }
event: state-changed Playing → Stopped / Stopped → Idle        # = stdin-EOF graceful teardown, NOT end-of-media

$ time (tail -f /dev/null | timeout 30 ./target/release/lumen open samples/sample.mp4)  # EXIT=124
event: state-changed Buffering → Playing        # then NOTHING for 30 s; user 0m0.014s — nothing pumps

$ (advance-pumped run: 'advance','advance','time','advance 50','advance 500')
event: time-tick 0.633334s (21.1%) … 2.200000s (73.3%)
event: PlaybackEndOfTrack { track_kind: Video, id: TrackId(0) }   # at position 1.533333s (video limb drained early; media continues)
event: end-of-media
event: state-changed Playing → Ended
ok: time 3.041814s (100.0%), 1 buffered range(s)
event: state-changed Ended → Idle      # exit 0

$ printf 'seek 1s\ntime\n' | lumen open samples/sample.mp4
event: TimingSeekFailed { error: CommandError { stage: Demux, code: Unsupported,
  message: "[demux] unsupported (plugin.demux.reanchor.unsupported)", details: {"position_us": "0", "reason": "demux"} } }
ok: time 0.000000s (0.0%)          # all seek forms fail identically: 90s|1s|50%|+10s|-1m
event: error [decode.corrupt] the engine recovered 26 damaged unit(s) while resynchronizing

$ printf 'seek 1s\ntime\n' | lumen open samples/sample.wav
event: TimingSeekCompleted { position: Timestamp(1000000), seek_kind: Command }
ok: time 1.000000s (33.3%)          # WAV reanchor works — the contrast that isolates the MP4 gap

$ lumen open samples/sample-hevc.mp4   # exit 65
error: [decode.unsupported]: unsupported (cli.codec_unavailable): no decoder for `hevc` in this build
  (landed: mjpeg, pcm, flac, mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264)
event: state-changed Idle → Opening / Opening → Error / Error → Idle

$ lumen open samples/sample.mp4 --start-time 2s   # exit 0 — the OPEN-time seek (pure query) works
ok: time 2.000000s (66.7%)
event: error [decode.corrupt] the engine recovered 25 damaged unit(s) while resynchronizing

$ lumen engine-info (inside a session)
ok: engine / version: 0.1.3 / host: cli
  features: first-vertical:avi-mjpeg-pcm, audio.null-device, video.software-compositor
  decoders: mjpeg, pcm_s16le

$ lumen decode-bench samples/sample.mp4   # exit 0
bench: video track#0 h264 — not benched (no decoder in this build (first vertical: mjpeg + pcm));
       audio track#1 aac — not benched (…) (container mp4)
BENCH … v_frames=0 v_decoded=0 a_frames=0 a_decoded=0 wall_s=0.000

$ strings target/release/lumen | rg -c 'rust_h264' → 581 ; rg -c 'symphonia' → 523
$ strings target/release/lumen | rg -m1 'H264SwDecoder10emit_frame' →
  …codecs5video4h2642sw13H264SwDecoder…emit_frame…lumen_cli   # the decoder is in the shipped binary

$ ffprobe samples/sample.mp4 → h264 320x240 duration=3.000000 nb_frames=75 ; aac duration=3.000000 nb_frames=131
```

Ground truth of the two headline artifacts:
- The 12-event stdin-closed walk `…Playing→Stopped→Idle` is **not** playback; it is
  the M17 EOF teardown (`graceful_exit`, run.rs:293). Nothing between commands pumps
  (run.rs:14-16: "Between commands nothing pumps; the explicit `advance [ticks]` host
  verb is the run-to-end driver"); `time` stays 0.000000 s (0.0%) across seconds of wall
  clock. True end-of-media is reached only via `advance`.
- `decode-bench`'s `v_frames=0 … no decoder in this build` is **the bench's own lie**
  (bench.rs:334), not the engine's state — the same binary decodes this exact file
  (advance run above; `H264SwDecoder` symbols in the binary).

## 2. The open path, function-by-function (all quotes sed-verified)

1. **run.rs:132** `open` argv → REPL: `}) => repl_mode(&source, start_time, start_paused, no_video, no_audio, input, out, err, invocation.json, interactive,)` — the argv `open` seeds the session, stdin lines drive it.
2. **dispatch.rs:82** route: `let outcome = host.open_session(source.clone(), …)` — the one dispatch entry (ARCH-5: every surface builds a `WireRequest`).
3. **host.rs:890** `pub(crate) fn open_session(` … **host.rs:904**: `let opened: OpenedMedia = open_source(&source, start_time_us, no_video, no_audio)?;` — the demux host owns the whole pre-engine open.
4. **demux_host.rs:392-396** file lane: `let uri = file_uri(path)?;` / `let stream = input::FileAccess::new()` / `.open(&uri)` — canonical `file:` URI + `input` FileAccess (seekable random access ⇒ moov-at-end works; both samples exit 0 identically).
5. **demux_host.rs:444-448** probe: `let report = media_probe::probe_stream(&registry, &mut *stream, &config)` + `let container = report.format.as_str().to_string();` (registry = `ProbeRegistry::with_builtins()`).
6. **containers/mod.rs:67** the single dispatch: `"mp4" => mp4::open(stream, opts),`.
7. **containers/mp4.rs:100-102** construct + pre-pass: `let mut demuxer = Mp4Demuxer::new(stream)…` / `let tracks = demuxer.start()…` / `let facts = harvest_facts(&demuxer, &tracks);`.
8. **containers/mp4.rs:190-192** sample entry harvest: `let entry = demuxer.sample_entry_bytes(track.id);` + `let extra_data = entry.and_then(|entry| config_blob(codec, entry));` (`sample_entry_bytes` = crates/demux/src/mp4/mod.rs:669, the verbatim first `stsd` entry).
9. **containers/mp4.rs:237** avcC: `"h264" => find_child_box(entry, b"avcC").map(|body| SharedBytes::from_vec(body.to_vec())),` — the config blob for H264SwDecoder.
10. **containers/mp4.rs:232 + :375-376** esds ASC: `"aac" => asc_of_entry(entry)…` ; `fn asc_of_entry(entry: &[u8]) -> Option<&[u8]> {` / `let esds = find_child_box(entry, b"esds")?;` — AudioSpecificConfig = esds DSI (AAC geometry from `audio_geometry`, mp4.rs:386-389).
11. **host.rs:919-926** audio limb: `crate::pipeline_host::open_audio_session(channels, rate, anchor_ns)` → **pipeline_host.rs:1598-1599**: `let NullStream { sink, device } = NullBackend::open_stream(&format, RING_FRAMES, NullConfig { capture: false })?;` — the session clock is the NULL device's sample counter.
12. **host.rs:948 + :981-984**: `let mut provider = HostDecoderProvider::new(Arc::clone(&facts));` … `let handle = self.engine.open(config)` — provider rides `SessionConfig`.
13. **pipeline.rs:1708** engine wiring: `let decoder = provider.decoder_for(track, &config)?;` — the engine asks the host's provider.
14. **pipeline_host.rs:346** h264 decode arm: `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),` ; **:288-291** aac arm: `"aac" | "aac-lc" => {` / `self.seeded(track, config, AacDecoder::new)`.
15. **pipeline_host.rs:1340-1342** conversion: `media_types::PixelFormat::Rgba8 | media_types::PixelFormat::Bgra8 => frame.clone(),` / `media_types::PixelFormat::I420 => convert_i420_to_rgba(frame)?,` (BT.601 full-range, module doc :23-24).
16. **pipeline_host.rs:1314-1319** headless renderer: `let renderer = Renderer::new_headless(extent, SurfaceConfig { format: SurfaceFormat::Bgra8Srgb, present_mode: PresentMode::Fifo, })` — software wgpu renderer, **NO WINDOW** (module doc :29-31: "an optional [`PresentTap`] mirrors every request the renderer presents (the desktop's webview frame channel; `None` in the pure CLI…)").
17. **host.rs:997** settle: `self.pump_pipeline_until(|host| host.state() != PlaybackState::Opening, AWAIT_PUMPS);` — pump until settled; fatal check `:998-1000`.
18. **run.rs:226-228** REPL loop: `match input.read_line(&mut line) { Ok(0) | Err(_) => break,` — EOF → **run.rs:293** `graceful_exit(&mut host, err);` (stop→close teardown = the Stopped→Idle tail in every stdin-closed run).
19. **run.rs:252** the only time mover: `CliCommand::Advance { ticks } => { host.advance(ticks);` — deterministic tick pump.

## 3. What actually works (verified, commands above)

- **Open + full-duration play to null sinks** on h264+aac MP4 (both box orders),
  aac-only MP4, and WAV: `advance` runs reach `end-of-media`, `Playing→Ended→Idle`,
  position 100.0% (3.041814 s / 3.000000 s), exit 0. (`sample.mp4`,
  `sample-faststart.mp4`, `sample-aac-only.mp4`, `sample.wav` advance runs.)
- **Real h264 + aac decode in the shipped binary**: full-duration advance runs;
  h264 resync accounting after mid-GOP entry (`--start-time 2s` → "recovered 25
  damaged unit(s)", failed seek → 26, four failed seeks → 41); binary symbols
  `codecs::video::h264::sw::H264SwDecoder::emit_frame` + 581 `rust_h264` / 523
  `symphonia` strings in `target/release/lumen`.
- **Demux truth**: moov-at-end and faststart open identically; `info` reports
  container mp4, duration 3.000000 s, seekable yes, `#0 video h264 320x240`,
  `#1 audio aac 1ch 44100Hz` — matching ffprobe (h264 320x240, 75 video samples,
  131 aac frames, 3.000 s; stsz boxes parsed with python3: video stsz=75 entries,
  audio stsz=131).
- **Open-time seek** `--start-time 2s` lands at 2.000000 s (66.7%) — the MP4 pure
  seek query + skip watermark works.
- **WAV in-session seek works** (`TimingSeekCompleted`, 1.000000 s) — proving the
  engine seek flow itself is complete when the demuxer implements `reanchor`.
- Typed, honest error surface everywhere (exit 65/64/69 ladder; HEVC refusal names
  the codec; chapter seek is typed NotFound).

## 4. Exact failure points for "user watches an MP4" (with citations)

**F-001 [S0] — in-session seek ALWAYS fails on MP4.**
Every seek form (1s/90s/50%/±) on every MP4 variant answers
`TimingSeekFailed … [demux] unsupported (plugin.demux.reanchor.unsupported)` with
position unchanged (runs above). Chain: dispatch.rs:131 `Command::PlaybackSeek { target } => host.dispatch_seek(*target)` → engine `demuxer_reanchor` **pipeline.rs:1499** `self.demuxer.reanchor(target)` → `SelectDemuxer::reanchor` (demux_host.rs:615) → `StartedMp4::reanchor` **mp4.rs:173-177**: "The demuxer keeps the trait's default typed refusal (the moving half of in-session seek is its honest gap)" → trait default **plugin-api/src/demux.rs:113-118** `Err(EngineError::new(… ErrorContext::new("plugin.demux.reanchor.unsupported")) )`. `crates/demux/src/mp4/mod.rs` implements `seekability` (:2039, `Fast { indexed: true }`) and the pure `seek` query (:2050) but has **no `reanchor` override** — unlike avi/wav/ogg/mkv/ivf/adts/aiff/ps which all implement it (rg verified). The engine then rolls back per invariant D4; a stuck clock at 0.0% during Playing.

**F-002 [S2] — audio renders into a null sink; no audio device exists.**
pipeline_host.rs:1598-1599 `NullBackend::open_stream(…)`; module doc :19-22 "→ the null sink's ring (2.C3's null backend — no audio device in this sandbox)". The only real-backend lane is gated off: **crates/audio-output/src/cpal.rs:43** `pub const AUDIO_ADR005_ADMITTED: bool = false;` (ADR-005 pending; `builtin_backends()` = NullBackend + UnsupportedBackend). An MP4 with AAC plays "silently": decode, mix, ring, sample-counter clock all real; the last inch (device) is absent.

**F-003 [S2] — no display surface anywhere in the shipped build.**
Video path ends at the headless renderer: pipeline_host.rs:1314-1319 `Renderer::new_headless(… Bgra8Srgb … Fifo …)`; the PresentTap (the desktop webview frame channel) is `None` in the pure CLI (pipeline_host.rs:29-31). The shipped desktop binary is a skeleton that always exits **86**: apps/desktop/src/main.rs:21 `pub const WEBVIEW_PENDING: u8 = 86;` and :85-86 `eprintln!("no webview host compiled: ADR-004 pending (records/tauri-wry-2.x.md)");` / `ExitCode::from(WEBVIEW_PENDING)`. The frame-channel bridge itself is real (`apps/desktop/src/frame_tap.rs:34` `use lumen_cli::pipeline_host::PresentTap;` + bounded mailbox), but no window ever opens it; desktop-tauri fails at gdk-sys in this sandbox (audit/runs/baseline/cargo-build-desktop-tauri.txt: "The system library `gdk-3.0` required by crate `gdk-sys` was not found" — environment limit, UNVERIFIED, not a repo defect).

**F-004 [S2] — HEVC MP4 refused at open (exit 65).**
pipeline_host.rs:372-378 `other => Err(host_error(… "no decoder for `{other}` in this build (landed: …)")` — the CLI provider has no `"hevc"` arm. The HEVC decoder that exists (crates/codecs/src/video/hevc/decode.rs) is hardware-only by ADR-001 ("no software decode, by design"; a machine without a hardware path gets typed `hevc.hw.required`). The hvcC harvest side is already wired (mp4.rs:238). Honest typed refusal, not a stub — but for HEVC content the "MP4 open path fails" for the user.

**F-005 [S0] — the capability reporters lie about this binary.**
`engine-info` hardcodes 2 decoders (host.rs:1637-1657: features "first-vertical:avi-mjpeg-pcm", decoders mjpeg/pcm_s16le) while the same binary's provider dispatches 14 decoder families (pipeline_host.rs:241-380, incl. h264/aac/vp9/dnxhd) — and just played h264+aac to 100%. `decode-bench` answers `v_frames=0 … no decoder in this build (first vertical: mjpeg + pcm)` (bench.rs:334) because its limbs are hardcoded `fact.codec == "mjpeg"` / `starts_with("pcm_")` (bench.rs:299-313) — the parity instrument measures nothing for 12 of 14 shipped decoder families while exiting 0. Third inconsistent surface: the refusal string's "landed:" list (12 names) omits vp9/dnxhd. Three mutually contradictory capability claims from one binary.

**F-006 [S3] — no wall-clock pump in the shipped REPL.**
run.rs:14-16 (module doc): "Between commands nothing pumps; the explicit `advance [ticks]` host verb is the run-to-end driver." `pump_wallclock` (host.rs:523-525) exists but is called only by the desktop shell (apps/desktop/src/shell.rs:280 `self.shell.engine_host().pump_wallclock();`) and tests — the shipped CLI never advances media time in real time (stdin-held run: 30 s stuck in Playing, `time 0.000000s`). Continuous pumping is card 3.B3, explicitly not landed.

## 5. The minimal set of missing pieces (ordered)

1. **`Mp4Demuxer::reanchor`** (crates/demux/src/mp4/mod.rs) — implement the moving
   half of seek (stss walk-back → per-track landing commit, compute-then-commit per
   D4; the AVI/WAV reanchor implementations are the in-tree pattern). This single
   method fixes every in-session MP4 seek; the engine, host boundary duties
   (`seek_boundary`, host.rs:1201) and the WAV proof show everything else is done.
2. **Audio device admission (ADR-005 flip)** — `AUDIO_ADR005_ADMITTED` +
   platform-* cpal adapter in crates/audio-output/src/cpal.rs (the file documents the
   one-change discipline). Until then every session is inaudible by construction.
3. **A display surface** — land a windowed renderer path or the ADR-004 webview
   wrapper (apps/desktop exits 86; desktop-tauri blocked at gdk-sys in this sandbox).
   The plumbing is staged: `PresentTap` seam + desktop frame_tap/mailbox/bridge all
   exist and compile; what is missing is the actual window and the shell that pumps.
4. **The wall-clock session pump (card 3.B3)** for any "watch" surface — a host
   loop calling `pump_wallclock()` (CLI would need a `--wait`/event-driven REPL
   policy; run.rs:18-19 records the proposal).
5. **HEVC playback** — either a software HEVC decoder (ADR-001 amendment) or the
   hardware bridge + a `"hevc"` arm in `HostDecoderProvider`; the container side
   (hvcC harvest) is already wired.
6. **Capability truth (not needed to watch, needed to stop lying)** — make
   `engine_info()` (host.rs:1631) and `decode-bench` (bench.rs:296-334) report the
   provider's real registry instead of the hardcoded first-vertical story.

## 6. Commands run (all this run, repo root)

`lumen --help`; per-variant: `timeout 30 ./target/release/lumen open <v> </dev/null`
(exit codes captured un-piped: 0,0,65,0,0); `tail -f /dev/null | timeout 30 …` (124);
`time (…)` (30.004 s wall, user 0.014 s); scripted stdin sessions (`seek 1s|50%|+10s|-1m|90s|chapter 3`, `time`, `info`, `tracks`, `engine-info`, `duration`, `advance [n]`, `quit`); `lumen open <v> --start-time 2s`, `--no-video`; `decode-bench <v>` ×5; `lumen info <v>` (usage error — session-scoped verb); `strings target/release/lumen | rg -c 'rust_h264'|'symphonia'|'H264SwDecoder'`; `ffprobe samples/sample.mp4`; python3 MP4 box/stsz parse; `sed -n` verification of every quoted line; `file samples/*`. No git, no cargo, no env printed, no file outside audit/ touched (outputs also parked in /tmp).
