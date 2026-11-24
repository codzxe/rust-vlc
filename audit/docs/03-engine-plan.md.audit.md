# CLAIM AUDIT: docs/03-engine-plan.md (L1-1113, chunks part1-2)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 16 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 8 / 1 / 0 / 0 / 6 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Doc 03 declares itself a "planning draft for Lead review" and mostly behaves like one — [PROPOSAL]/[VERIFY] markers are used consistently. Where it makes checkable claims against the tree, the majority are REAL and verified (crate layout, probe/queue default numbers, two-level generations, the AVI+MJPEG+PCM first vertical, CLI play command, state-machine walk). Two spec sections are stale beyond excuse given the project's actual progress: §8's promised testkit drivers (ScriptSource, SessionHarness, Vanishing/Stalling sinks, FrameRecorder) exist NOWHERE in the tree — the crates landed, the drivers never did; and §4.13's deterministic seeded shuffle ("pure function (seed, repeat, item ids, queue)") has no implementation anywhere — the shipped `playlist.set-shuffle` command just flips a flag with sequential advancement (the demo transport's own comment admits "a real shuffle re-order is engine work"). As a plan this is PAPER-ONLY for those parts, not FALSE — but both are Phase-2/5-era promises still unmet at wave 26.

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Status: planning draft for Lead review. Task ID 1.A3." | :3 | REAL (self-honest) | Doc header matches its [PROPOSAL]-heavy content. |
| 2 | Crate layout: media-types…playlists (14 subsystem crates) | §2.3, §4.1-4.13 | REAL | All present in crates/: media-types, input, media-probe, demux, packet-queue, codec-api, playback-engine, media-clock, audio-engine, audio-output, video-engine, renderer-wgpu, subtitles, playlists. |
| 3 | Probe defaults "max_lookahead = 64 KiB hard-capped at 1 MiB; min_score = 60" | §4.2(b) | REAL | crates/media-probe/src/config.rs:18 `DEFAULT_MAX_LOOKAHEAD: usize = 64 * 1024`; config.rs:5-6 comment cites doc 03 defaults verbatim. |
| 4 | Queue defaults "audio queue 1.0 s / 200 packets, video queue 2.0 s / 300 packets" | §4.4(b) | REAL | crates/packet-queue/src/limits.rs:40 `AUDIO_DEFAULT_PACKETS = cap(200)`, :43 `VIDEO_DEFAULT_PACKETS = cap(300)`; comments cite "Doc 03 §4.4(b)". |
| 5 | Session state machine Idle→Opening→…→Playing→Stopped→Idle | §4.6 | REAL | Orchestrator baseline mp4-smoke: "`lumen open` H.264+AAC MP4 walks Idle->Opening->Buffering->Playing->Stopped->Idle, exit 0" (audit/runs/baseline/mp4-smoke.md). |
| 6 | §5 two-level generations: epoch minted on Open/SetSource; pipeline resets each epoch | §5 | REAL | crates/playback-engine/src/epoch.rs:1-20 header implements exactly this, citing "doc 03 §5". |
| 7 | First vertical = "candidate 2, AVI + MJPEG + PCM" | §7 | REAL | Baseline: rc-smoke "headless-decode (AVI/MJPEG/PCM fixture) PASS"; mp4-smoke "first vertical: mjpeg + pcm". |
| 8 | "lumen play fixture.avi" command surface | §7 crit. 1 | REAL | apps/cli/src/parse.rs:198 `"play" => no_args(CliCommand::Play)` (also open/pause/stop :197-201). |
| 9 | §8 testkit: VirtualClock impl, ScriptSource, Null/Capturing/Vanishing/Stalling sinks, FrameRecorder, SessionHarness | §8 (":1020 testkit crate") | PAPER-ONLY / STALE | rg repo-wide: only `VirtualClock` TRAIT + `StepClock` exist (crates/testkit/src/lib.rs:261,274); SessionHarness/ScriptSource/VanishingSink/StallingSink/FrameRecorder: 0 hits. testkit lib.rs:10-13 says they "land with the Phase-2 engine traits" — Phase 2 is long past. NullSink/CapturingSink DID land in crates/audio-output/src/null.rs:115 with a recorded "doc-03 deviation" (placement), but the harness/fault-script drivers never did. |
| 10 | "Deterministic shuffle: the play order is a pure function (seed, repeat, item ids, queue)" | §4.13 (":815") | PAPER-ONLY | rg `shuffle_seed\|PlayOrder` over all Rust: 0 hits. crates/playlists/src has only format parsers (m3u/pls/xspf/cue/hls/mpd). The command exists (commands/src/catalog/command.rs:338 `playlist.set-shuffle` — "affects advancement order only (S)") but no seeded Fisher–Yates order function anywhere; ui demo-transport.ts:543-546 comment: "sequential advancement (a real shuffle re-order is engine work)". |
| 11 | `Ts`/`PacketTs` vocabulary with checked conversions, "no sentinel values" | §3(b) | REAL (renamed) | crates/media-types/src/time.rs:59 TimeBase::new rejects zero components; Timestamp/TrackTiming/Stamp vocabulary (lib.rs:54) — names evolved from the sketch, semantics (checked conversion) present. |
| 12 | Audio callback: allocation forbidden; "fault-injecting allocator … lock-tracer" | §4.9(c) | REAL-UNPROVEN | testkit alloc_count.rs exists ("Counting-allocation instrument (ED-FIX-2)"); lock-tracer: no hits in testkit (UNVERIFIED — not found). |
| 13 | Seek: "never reported as success" failed-seek contract | §6 | REAL (code-side) | epoch/flush machinery exists (epoch.rs); no baseline run covers a failed seek — behavior REAL-UNPROVEN. |
| 14 | Slice acceptance "No `unsafe`, no C toolchain in the build" | §7 crit. 6 | UNVERIFIED (historical) | Current tree has 660 unsafe sites (audit/runs/baseline/loc-patterns.txt) — criterion was Phase-2-scoped; Phase-2-era tree not inspectable (git forbidden). |
| 15 | "WASM variant note [PROPOSAL]" single-threaded cooperative mode | §8 | REAL | apps/web lane exists; compat-matrix records wasm32 module building, decoder-less. |
| 16 | Ownership: "CI enforces this with the boundary check" | §2.3.4 | REAL | Baseline full_gate lists an "arch" section (boundary/architecture check) in scripts/full_gate.sh run. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| §8: testkit will host ScriptSource/sinks/SessionHarness for the engine test strategy | testkit lib.rs:10-13: they "land with the Phase-2 engine traits"; engine (Phase 2+) landed; drivers absent | The drivers never landed; testing took another form (per-crate tests + tests/compat runner). Doc section is stale, not lying about the present. |
| §4.13: deterministic seeded shuffle is the playlist semantic | commands catalog: set-shuffle "affects advancement order only (S)"; demo transport keeps sequential advancement | Shuffle as specced is unimplemented; today it is a flag with no reorder. |

## 4. Fabrication / inflation indicators
None: the document does not claim implemented status for unbuilt things; it marks them [PROPOSAL]. The risk is staleness, not fabrication — two sections promised for Phase 2 remain unimplemented at wave 26 (§8 drivers, §4.13 shuffle), which downstream docs must not cite as existing.
Commands run: rg over crates/{testkit,media-probe,packet-queue,playlists,media-types,playback-engine}, apps/cli/src/parse.rs, commands catalog; sed quotes; baseline citations. Exit 0.
Findings: S2: 0 · S3: 2 (F-008 §8 testkit drivers never landed; F-009 §4.13 shuffle unimplemented while the command name implies it) · S4: 0.
