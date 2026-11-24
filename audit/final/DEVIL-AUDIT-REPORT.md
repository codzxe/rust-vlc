# DEVIL AUDIT REPORT — Lumen (rust-vlc)

| Field | Value |
|---|---|
| Report | W20-A10 — §8.1 executive report (the brutal one-page truth + appendices) |
| Date | 2026-11-26 |
| Repo | codzxe/rust-vlc @ e83e5bd3 (branch audit/devil-20261124; repo root /home/z/my-project/audit-workspace) |
| Corpus | 3,279 files / 1,070,164 LOC audited over 20 waves (3,383 audit files; 19 domain summaries; 42-entry contradiction register) |
| Inputs | WAVE-00-SUMMARY + baseline/mp4-smoke.md (verbatim); REALITY-SCORE, MP4-REALITY-TRACE, REMEDIATION-MAP, CONTRADICTIONS, W20-A04 citation sweep, INCIDENT-LOG, fabrications.md |

## 1. The one-sentence truth

**Lumen is a real, disciplined demux/decode engine that plays H.264+AAC MP4 into a null audio device and a headless renderer — no shipped build has ever shown a user a pixel or played them a sound — and the documentation layer around it lies in both directions about what the machine can do.**

## 2. The numbers

- **REAL 49 / 252 matrix rows = 19.44%** (floor 19%; no rounding up). Distribution
  (REALITY-SCORE §1): ACCURATE-ABSENT 110 (43.7%), SHALLOW 29 (11.5%),
  UNDERSTATED 24 (9.5%), UNREACHABLE 18 (7.1%), FALSE 14 (5.6%),
  REAL-UNPROVEN 8 (3.2%). The matrix is 43.7% honest about absence and
  mis-states the rest **in both directions**: 14 rows overstate, 24 rows claim
  `not_started` for landed, tested code.
- The **inventory/census layer is bulletproof** (byte-identical regeneration,
  W18/W19); the **status layer is the rot** (drifts both ways, never refreshed).
- **Wave 0 MP4 smoke, verbatim** (audit/runs/baseline/mp4-smoke.md; binary
  `target/release/lumen`, release lane exit 0):

```
$ timeout 30 ./target/release/lumen open /home/z/my-project/samples/sample.mp4 </dev/null
ok: accepted session=1 gen=0
event: state-changed Idle → Opening
event: tracks 2 ready
event: buffering 0.00
event: state-changed Opening → Buffering
event: TimingDurationKnown { duration: Timestamp(3000000) }
event: MediaInfoReady { info: MediaInfo { source: File { path: "/…/sample.mp4" }, container: None, … tracks: [TrackInfo { … codec: CodecDescriptor { … description: Some("h264") … }, … }, TrackInfo { … description: Some("aac") … }] … } }
event: buffering 0.02
event: buffering 1.00
event: state-changed Buffering → Playing
event: state-changed Playing → Stopped
event: state-changed Stopped → Idle
exit_code=0
```
(The MediaInfoReady line is abbreviated here only; the full verbatim is
mp4-smoke.md:14 — tracks h264+aac, `language: Some("voe")` on both, the
misparse root-caused at W05-A01: `unpack_language` adds 'a' to 1-based packed
letters instead of 'a'-1.)
```
$ ./target/release/lumen decode-bench /home/z/my-project/samples/sample.mp4
bench: video track#0 h264 — not benched (no decoder in this build (first vertical: mjpeg + pcm)); audio track#1 aac — not benched (no decoder in this build (first vertical: mjpeg + pcm)) (container mp4)
BENCH file=/home/z/my-project/samples/sample.mp4 bytes=90034 v_frames=0 v_decoded=0 a_frames=0 a_decoded=0 wall_s=0.000 decode_s=0.000
exit_code=0
```
```
$ ./target/release/lumen engine-info
ok: engine
  version: 0.1.3
  host: cli
  features: first-vertical:avi-mjpeg-pcm, audio.null-device, video.software-compositor
  decoders: mjpeg, pcm_s16le
exit_code=0
```

- **Corrected interpretation** (W03 correction; W20-A05 executed trace; W20-A02
  re-verification): the 12-event walk is stdin-EOF teardown (`graceful_exit`,
  run.rs:293), not playback — with stdin held open nothing pumps for 30 s
  (`user 0m0.014s`); the `advance` verb drives the SAME binary to
  end-of-media at 100.0% with h264+aac decode live. The binary embeds
  rust_h264-0.6.1 + symphonia (`H264SwDecoder::emit_frame` symbols in
  `strings`). **The bench registry lies while the binary decodes**: `v_frames=0`
  is bench.rs's hardcoded two-name registry (bench.rs:299-334), not engine
  truth. Wave-0's "zero decode" litmus was the instrument's artifact.

## 3. The top 25 findings (ranked by damage; pointers to wave summaries)

1. [S0] In-session MP4 seek ALWAYS fails: `Mp4Demuxer` implements `seek()` but not `reanchor` — the only entry the engine calls — while MediaInfo says `seekable: true`. crates/demux/src/mp4/mod.rs:2008-2084 + plugin-api/src/demux.rs:113-120 (W05-A01).
2. [S0] The shipped GUI cannot compile on any host: `app.rs:301` calls undefined `spawn_session_pump` (E0425; real name `spawn_event_pump` at :190). apps/desktop-tauri/src/app.rs:301 (W13-A10, W14; 4 independent confirmations).
3. [S0] Four decoder registries in one binary, four answers, one true: engine-info=2, refusal-string=12, bench="none", dispatch=16 arms/14 families. host.rs:1643-1658, bench.rs:299-334, pipeline_host.rs:242-380 (W13).
4. [S0] CLI `info` hardcodes `seekable: true` — one crafted FLAC-in-WAV run: engine says false, host prints yes, seek fails. host.rs:1307 (W13-A02, executed).
5. [S0] No audio output exists: every shipped host renders to `NullBackend`; the cpal seam is a stub that panics if admitted; WASAPI is enumeration-only. pipeline_host.rs:1598-1599, audio-output/src/cpal.rs:43-64 (W07/W08).
6. [S0] Windows ABI rot: dxgi vtables omit the 4 IDXGIObject slots (`get_desc1` calls `EnumOutputs`); PropVariant pinned 16 bytes vs 24 on x64 — stack under-write class. platform-windows/src/windows/dxgi.rs, com.rs (W08-A10).
7. [S0] The entire subtitles crate (25,416 LOC, 267 tests) is unreachable from every shipped binary while matrix rows claim "implemented". crates/subtitles/** (W09).
8. [S0] CC-608 PAC range bug: keying 0x40-0x5F instead of 0x10-0x1F renders "HELLO WORLD!" as "D!". crates/subtitles/src/cc/mod.rs (W09-A07, executed).
9. [S0] Generated settings help advertises 121 `--<area>-<key>` flags; the parser knows 4 global flags; every settings flag exits 64. schemas/generated/settings-help.txt vs parse.rs:156-159 (W11-A04, executed).
10. [S0] video-h264.md: "There is no software H.264 decode, and none is scheduled" — the shipped binary decodes H.264 (rust_h264 0.6.1). docs/formats/video-h264.md:8-12 (W19-S2; C-001).
11. [S0] Matrix rows 099/100/101 mark ~4,900 LOC of landed, tested, executed streaming code `not_started`, `evidence = ""`; rows 084/095 do the same for the shipped file and HTTP lanes. docs/feature-matrix.toml:1253-1262, 1405-1481 (W18).
12. [S0] Matrix row 007 `container-asf-wmv` "implemented" (win/macos): no ASF probe signature, no dispatch arm; open path refuses ASF (executed disproof). media-probe signature/mod.rs + containers/mod.rs:52-75 (W04, W18).
13. [S0] 23_alloc_budget.sh: a CI gate whose `sys.exit(1)` is swallowed by trailing `say`s — executed repro, exit 0. scripts/quality/23_alloc_budget.sh (W17-A03).
14. [S0] rust-clippy.yml can never fail on lint (`continue-on-error: true`, no `-D warnings`); 26_complexity.sh ratchet baseline never committed. .github/workflows/rust-clippy.yml (W17-A04).
15. [S0] Committed gate evidence is stale-green: wave3-merge log 1718/0 vs today's 4744/2 FAILED; phase3-exit signature backdated; 4h soak "discharged" while never taken. docs/phase-exit/**, docs/soak/** (W17-A03/A06).
16. [S1] A 188-byte 0x47-led file crashes the shipped CLI at open (exit 101, index out of bounds) — triggerable over HTTP. media-probe/src/signature/phase5.rs:159-161 (W04-A01, W05-A04 executed).
17. [S1] MKV lacing inversion: frame-count byte read as literal (spec: frames-1); ffmpeg-legal blocks silently mis-decode; 3 in-tree tests PIN the bug. demux/src/mkv/cluster.rs:258 (W05-A06).
18. [S1] MKV window-slide livelock: block sizes in (4MiB, 32MiB] re-read the same window forever (watchdog kill exit 124). demux/src/mkv/mod.rs:644-663 (W05-A06, executed).
19. [S1] MP4 stts/ctts entry-count loops unclamped — ~4.3e9 iterations from a 12-byte box (CPU DoS on `open`); trex defaults +4-skewed (fragmented MP4 broken). demux/src/mp4/mod.rs:1253-1254, 1316-1317 (W05-A01).
20. [S1] The test headline undercounts: first-vertical lib failure (exit 101) aborted the package before journey/recovery_matrix ran; transcripts stale at HEAD. tests/first-vertical (W16-A06).
21. [S1] Streaming panics: dash/resolve.rs div-by-zero (2 sites, exit 101 — falsifies the fuzz "panic-free" contract); hls/master.rs usize underflow; timestamp_map.rs i64 overflow (executed). streaming/src/dash/resolve.rs:410,735; hls/master.rs (W10).
22. [S1] ASS parser overflow panic on crafted PlayResX+\pos — "no panics on any input" FALSE. subtitles/src/ass/parser.rs:722 (W09-A01, executed).
23. [S1] ADTS mint loop: `frame_length == 0` never advances — infinite loop + unbounded queue (replica-proof 1e6 packets). apps/cli/src/containers/ts.rs:354-376 (W13-A08).
24. [S1] CLI panics on non-UTF8 argv at `env::args()` (exit 101) before reading a byte of media. apps/cli/src/main.rs:14 (W13-A05).
25. [S0] single_command.rs — cited as "live enforcement" — is a text scan that links the compile-broken lib: an unrunnable test used as evidence. apps/desktop-tauri/tests/single_command.rs (W14-A05).

## 4. Process-failure analysis — how 15 planning docs, a 252-row matrix, and a path-existence auditor produced paperwork instead of a video player

The program optimized **status-string maintenance**, not playback. Mechanisms:

1. **Gates that cannot fail.** alloc-budget's failure exit is swallowed (executed proof); clippy is `continue-on-error`; the complexity ratchet baseline was never committed; the architecture gate is blind to table-form `[dependencies.foo]` edges (executed); compat-matrix writes spec-invalid TOML with exit 0 "APPLIED" while its note-insertion self-rejects. A green wall that cannot turn red certifies nothing — and it never turned red.
2. **Stale-green evidence.** Committed logs describe a tree at 1718/0 and FMT_EXIT=0; the audited tree is 4744/2 FAILED and fmt FAILED. Evidence was a snapshot at gate-cut time, then rotted silently; nobody re-ran anything.
3. **The audit trail stops at path-existence.** feature_audit.rs R4 = non-empty evidence string + `Path::exists()` — an EMPTY FILE passes as evidence (proven by execution). parity-report's "verified" is a string-match no row has ever claimed. The repo's only automated verification verifies that a path exists, never that a claim is true.
4. **Registries lie in both directions.** Four decoder lists in one binary answer 2 / 16 / 12 / "none". The matrix overstates (asf, seek, subtitles "implemented") and understates (http, reconnect, protocol-file "not_started").
5. **The understatement paradox.** 24 rows claim LESS than reality while 14 claim MORE. The docs are not marketing inflation — they are an unrefreshed snapshot drifting both ways, which is worse: a status layer that errs in both directions carries zero information; you cannot even invert it. This is why the repo could not re-plan itself out of the hole: reading the matrix tells you nothing about the code.
6. **The compile-broken shipped GUI was masked by an environment failure.** desktop-tauri dies at gdk-sys in this sandbox, so E0425 never surfaced here; the workspace excludes the crate, so the green release lane never builds it; INFRA-BLOCKED.md claims "the crate compiles in its dedicated CI lane" against a ci.yml step that compiles the lib and would be red on this tree. One env failure + one exclude + one false doc = a broken product lane blessed by every check.
7. **Evidence lives in /tmp and .gitignore.** w26 verification captures never committed; ~120 probe-work log refs lost to `*.log`; "61 logs + status.txt" cited and absent; av1 conformance keyed to /tmp caches. The strongest paperwork in the repo has no committed artifacts.

## 5. What to stop doing

1. **Stop parallel status layers.** At least nine (matrix, compat-matrix, parity-report, engine-info, bench strings, landed-string, INFRA-BLOCKED, PARITY-REPORT, risk register) — all drifted. Write ONE status layer, generated from the code, refreshed by the release CI.
2. **Stop per-lane registries.** Four decoder lists in one binary. One dispatch table is the registry; every reporter derives from it.
3. **Stop gates that cannot fail.** Kill `continue-on-error`, swallowed exits, uncommitted baselines, gates that skip their subject. A gate that cannot fail is decoration — delete it or make it fail.
4. **Stop trusting doc claims as facts.** The three worst doc lies this audit proved, verbatim-verified:
   - `docs/formats/video-h264.md:8-12` — "There is no software H.264 decode, and none is scheduled" while the shipped binary decodes H.264 (C-001).
   - `apps/desktop-tauri/INFRA-BLOCKED.md:4-5` — "the crate compiles in its dedicated CI lane" while `app.rs:301` calls a symbol that does not exist (B1).
   - `docs/PARITY-REPORT.md:123-126` — "CI is billing-blocked … every GitHub Actions run dies at 0 steps" while six in-tree run IDs prove executions (C-017).
5. **Stop evidence in /tmp and gitignored paths**; stop committing run snapshots without the command that regenerates them.

## 6. The minimum credible next goal

**"An MP4 plays with picture and sound on Windows."** Not parity, not a green matrix, not passing gates. Critical path (REMEDIATION-MAP §2):

1. **E0425, the 1-LOC fix** — rename to `spawn_event_pump` (app.rs:301 -> :190). Nothing Windows-GUI compiles, tests, or CI-proves until it lands. Hours.
2. **Windows audio output** — the dominant cost (L, ~300-1,500+ LOC): cpal admission or WASAPI render on the existing 3,093-LOC FFI substrate, plus PropVariant 16→24 and the dxgi slot fixes. Start immediately.
3. **Verify the tauri frame channel on Windows hardware** — mailbox (cap 2) → 10ms ticker → Channel → `MediaSurface.tsx:227` putImageData; harden backpressure.
4. **MP4 reanchor** (M, ~150-400 LOC) so in-session seek stops failing; then registry single-sourcing and the panic guards (TS-188, stts/ctts clamp, MKV lacing/livelock, adts) before any hostile-input exposure.

Effort band (REMEDIATION-MAP §3): **~1-2 days to a compiled GUI; ~2-6 engineer-weeks to picture+sound on Windows**, dominated by audio. Open no new feature lane until this closes.

## 7. The audit's own integrity

- **Citation sweep (W20-A04, 10% deterministic sample, 296 checked): 0 fabricated / 0 misattributed / 0 not-found** (95% CI upper bound ≈ 1%). 12.8% anchor-precision defects — text real, line anchors imprecise (median drift 1 line).
- **One refuted finding, on the record:** W12-A06's S0 ("evidence label cites a fetch-lane playback proof that does not exist") was REFUTED by W20-A02 execution — the test exists and passes. The audit's single confirmed finding-level fabrication — of a NEGATIVE claim. The fabrication vector is existence/execution claims, not quoted code.
- **The sandbox recycle is honestly recorded** (audit/runs/INCIDENT-LOG.md, 2026-11-24, mid-Wave-16): full state loss, ~225 uncommitted audit files destroyed, Wave 16 re-run in full; lost-run reports preserved as prior-agent claims and later corroborated by RR1's re-derivation.
- Orchestrator gate repairs (3 historical citation misattributions, byte-verified) are logged in audit/verify/fabrications.md.

---

## Appendix A — the 25 findings, detail

| # | Sev | What is broken (evidence pointer) | Why it matters |
|---|---|---|---|
| 1 | S0 | `Mp4Demuxer::reanchor` absent (rg 0 hits); engine's only seek entry returns `plugin.demux.reanchor.unsupported`; executed `seek 1s` → `TimingSeekFailed`, position 0.0% (W20-A05 §F-001; W20-A02 F1) | The user-visible seek story is fiction on the flagship container; `seekable: true` sold to the user |
| 2 | S0 | `spawn_session_pump` called, never defined; static proof decisive; workspace `exclude` keeps the release lane green (REMEDIATION-MAP B1) | The product's GUI lane is dead on every host and hidden by two masks |
| 3 | S0 | bench.rs:334 hardcoded `"no decoder in this build (first vertical: mjpeg + pcm)"`; host.rs:1643 hardcoded 2 decoders; landed-string omits vp9/dnxhd | The parity instrument measures nothing for 12 of 14 shipped decoder families — it produced Wave-0's false "zero decode" litmus |
| 4 | S0 | host.rs:1307 `seekable: true,` constant; engine truth never read | CLI reports capabilities it invents |
| 5 | S0 | NullBackend at pipeline_host.rs:1598-1599; cpal.rs:43 `AUDIO_ADR005_ADMITTED: bool = false` + `unreachable!()`; WASAPI 787 LOC with 0 IAudioClient hits (W20-A02/W20-A03) | "Sound" is architecturally impossible in every shipped build |
| 6 | S0 | dxgi vtable arithmetic + PropVariant `size_of=16` executed (W20-A03 F2) | The Windows substrate corrupts memory if ever wired |
| 7 | S0 | Sole dependent: fuzz/Cargo.toml:50; CLI refuses subtitle.*; Frame::Subtitle no-op | 25k LOC of tested code, zero users; rows claim it shipped |
| 8 | S0 | PAC keyed on text range; dialect vectors tests pass; codecs SEI discards type-4 anyway | Two independent bugs make CC doubly non-functional while marked "implemented" |
| 9 | S0 | parse.rs knows `--json/--control-stdio/--version/-h` only; `--video-hue 50` → exit 64 (executed) | The generated CLI surface is a catalog of commands that exit as errors |
| 10 | S0 | doc vs `Cargo.lock:769` rust_h264 0.6.1 + pipeline_host.rs:346 arm | The doc that would make a reader believe MP4 video is impossible |
| 11 | S0 | Rows 084/095/099/100/101: `not_started`, `evidence = ""` vs shipped lanes; streaming lib 212 tests pass (W20-A02 F10 executed) | The planning layer hides working code; re-planning impossible from the docs |
| 12 | S0 | No `asf` arm in containers/mod.rs:52-75; no probe signature; compat-matrix run refuses corpus ASF | Inflated status on a demuxer no binary can open |
| 13 | S0 | Script-exit=0 repro despite sys.exit(1) (W17-A03) | A budget gate that passes when it fails |
| 14 | S0 | ci.yml clippy lane `continue-on-error: true`; complexity baseline uncommitted | Quality gates are theatre |
| 15 | S0 | wave3-merge 1718/0 vs 4744/2; phase3-exit quotes the passing middle of a log ending EXIT_CODE:101 twice; signature says 2026-10-24, square-up 2026-11-13 | The exit-evidence culture overstates tree health systematically |
| 16 | S1 | `prefix[TS_PACKET_BYTES]` on a 188-byte window; 188B→101, 187/189B→65 (W20-A01 F1 re-executed) | Remote-triggerable panic on the shipped open path |
| 17 | S1 | `body[0] as usize` vs spec frames-1; 3 tests pin the inversion | Corrupt decode of legal files, blessed by tests |
| 18 | S1 | Window-slide loop has no exit (W20-A01 F4 static re-derivation) | Hang on valid MKV block sizes |
| 19 | S1 | `count = u32be(body,4)` unclamped loops vs clamped siblings (stsz/stco) | CPU DoS from a 12-byte box; fragmented MP4 durations wrong |
| 20 | S1 | Lib exit 101 aborted the package; committed transcripts stale (baseline 2 failures) | The headline test number is an undercount, used as evidence |
| 21 | S1 | resolve.rs:410,735 div-by-zero; master.rs underflow; timestamp_map.rs overflow executed | Panic-free contracts falsified by config-accepted input |
| 22 | S1 | parser.rs:722 add-overflow on crafted header | "No panics on any input" contract broken |
| 23 | S1 | `at = end` never advances when frame_length==0; 1e6-packet replica | Unbounded queue growth on a hostile stream |
| 24 | S1 | `env::args()` collects non-UTF8 → panic (executed exit 101) | Crash before any media is touched |
| 25 | S0 | include_str text scan; links broken lib; cited by INFRA-BLOCKED as enforcement | Unrunnable test deployed as evidence of the compile claim |

## Appendix B — near-miss S1/S2 cluster (not in the 25, still shipped damage)

- PGS 4 GiB retained from ~3 KiB input; DVB stale-CLUT OOB panic; VobSub 67 MiB/cue (W09-A04/A08/A10).
- ogg.rs seeds a 38-byte flac second header where the decoder parses 34 — spec-shaped .oga fails; zero coverage (W13-A06).
- CoreAudio ELEMENT_MAIN=1 vs master=0 in every property address (W08-A02).
- mp3 xing.rs VBRI `rest[14..21]` panic (library API; W07-A04). PS payload_len u64 underflow (W05-A03). asf/header.rs:558 unchecked child-size panic (W04-A10).
- WindowFull I420 stash → shipped video freeze defect (W06-A02/W13-A01). ids.rs redaction misses OAuth tokens — credentials print verbatim (W02-A03). elst empty-edit units wrong AND the test enshrines the wrong math (W05-A05). Web lane: decoder-less shell refuses every codec — no file plays in the browser (W12).
- ui/.vitest/json/output.json: committed foreign-checkout run, `success:false`, zero consumers (W15-R3).

## Appendix C — sources

audit/waves/wave-00/{WAVE-00-SUMMARY.md,AGENT-SPEC.md}; audit/runs/baseline/mp4-smoke.md;
audit/runs/INCIDENT-LOG.md; audit/verify/{W20-A04-citation-sweep.md,fabrications.md};
audit/waves/wave-{01..19}/SUMMARY.md; audit/final/{REALITY-SCORE.md,MP4-REALITY-TRACE.md,
REMEDIATION-MAP.md,CONTRADICTIONS.md}. All doc-lie quotes in §5 verified this session
with `sed -n` against the live tree.
