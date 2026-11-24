# AUDIT: apps/cli/src/host.rs
| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-24 |
| Lines / functions | 1940 / 61 (49 methods + 12 free fns) + 2 test mods (5 tests, 3 helpers) |
| File verdict | REAL — with one fabricated class-S answer (S0) and stale hardcoded snapshots |
| Reachable from shipped entry point? | YES — `main.rs:19 run::run` → `run.rs:132 repl_mode` / `:145 one_shot` / `:109 machine_mode` → `dispatch.rs:62 lumen_dispatch` → `route :75-156` → every `pub(crate)` method here; ALSO the desktop: `apps/desktop/src/dispatch.rs:67 host: lumen_cli::host::CliHost` |
| Findings | S0: 1 S1: 0 S2: 3 S3: 3 S4: 3 |
| Commands run | `target/debug/lumen open /tmp/w13a02_flacwav_junk.wav` + `info`/`seek 1` (exit 0, output below); `lumen open tests/fixtures/generated/valid/mp4_h264_aac.mp4` + `engine-info`; `lumen setting get/set …` one-shot + REPL; `lumen --help`, `--help global`, `--global-developer-mode on`; all exit codes noted in log. Binary mtime 22:16 > src 17:35 (current). |

## 1. One-paragraph verdict
This is the real session host: the pump loop (pipeline-only vs full vs wall-bounded), the
seek boundary duties, receipt correlation, and teardown are all implemented and I executed
them end-to-end (open→info→seek on a crafted file; settings round-trip; backpressure tests
read line-by-line). The pump math is careful and its two regression tests are genuine. But
three class-S snapshot answers are fabricated constants that the same process contradicts:
`media.get-info` hardcodes `seekable: true` while the engine's own `MediaInfoReady` event
reported `seekable: false` and the subsequent seek failed `not-seekable` — one run, three
mutually inconsistent answers (S0, executed). `engine-info` hardcodes a 2-decoder list and
a `first-vertical:avi-mjpeg-pcm` feature tag while decoding h264+aac in the same session.
The settings seam is a write-only echo: validated, stored in a process-local BTreeMap,
read by nothing but `setting_get`, persisted nowhere. Remaining defects are error-detail
swallowing, placeholder value fabrication, and test temp-file hygiene.

## 2. What actually works (verified)
— Crafted FLAC-in-WAV (0xFFFE fmt + FLAC GUID + 34-byte STREAMINFO) → `lumen open` →
Playing; `seek 1` answered class-A `ok: accepted session=1 gen=1` then the honest
`TimingSeekFailed … "the source cannot seek" … "not-seekable"` — the engine seek flow,
accept/terminal split, and typed failure all executed (by `/tmp` probe, output in log).
— REPL `setting set global.developer-mode on` → `ok: set on`; `setting get` → `ok: on` —
the BTreeMap round-trip and schema validation executed.
— `lumen open mp4_h264_aac.mp4` → 2 tracks (h264+aac) selected, Buffering→Playing — the
open/settle/auto-play path executed.
— Baseline (cited): release lane exit 0; `apps/cli` tests green per
audit/runs/baseline/cargo-test-workspace.txt.

## 3. Findings

F-001 [S0]
Where: apps/cli/src/host.rs:1307 (in `media_info`, :1283-1314)
Code:
```
                seekable: true,
                is_live: false,
```
What is wrong: `media.get-info` answers a hard-coded capability constant. The engine
maintains a real per-source fact: `crates/playback-engine/src/pipeline.rs:1189-1191`
`seekability()` delegates to the demuxer; `session.rs:2018-2031` refuses seeks when it is
`SeekSupport::None`; the engine's own event carries the truth. I executed it (crafted
FLAC-in-WAV, a demuxer-declared `SeekSupport::None` case per `crates/demux/src/wav/mod.rs:1087`,
`format.rs:626-627`): same run printed engine event `MediaInfoReady { … seekable: false … }`,
then host `info` → `seekable: yes`, then `seek 1` → `TimingSeekFailed … "not-seekable"`.
Why it matters: doc 08 (`docs/08-ui-figma-fidelity.md:346`) states the UI disables seek
"while the engine reports non-seekable media (`media.get-info`), not by guesswork" — this
field is a documented UI decision input and it is a constant lie. `--json` consumers get
the fabricated value on the wire. MPEG-TS is `SeekSupport::None` unconditionally
(`crates/demux/src/ts/mod.rs:1166-1167`), so every TS file hits this.
What correct looks like: read the engine's fact (the `MediaInfoReady` event payload, or an
exposed `pipeline.seekability()`) instead of `true`.
Confidence: HIGH — Verification: ran (probe transcript in audit/runs/W13-A02.log.md).

F-002 [S2]
Where: apps/cli/src/host.rs:1637-1658 (in `engine_info`, :1631-1661)
Code:
```
        features: vec![
            "first-vertical:avi-mjpeg-pcm".to_string(),
            "audio.null-device".to_string(),
            "video.software-compositor".to_string(),
        ],
```
(plus `decoders: vec![DecoderInfo { name: "mjpeg" … }, DecoderInfo { name: "pcm_s16le" … }]` at :1643-1658)
What is wrong: the self-description is a stale hard-coded snapshot of the first vertical.
`apps/cli/src/pipeline_host.rs:236-346` wires 13 decoder families (mjpeg, flac, mp3, mp2,
vorbis, opus, aac, alac, adpcm, av1, vp9, dnx, h264); I ran `engine-info` in a session
whose h264+aac tracks were selected and Playing — the answer was still
`decoders: mjpeg, pcm_s16le`. Same family as decode-bench's stale allowlist (Wave 3,
bench.rs:300-334). Why it matters: `system.get-engine-info` is the capability-discovery
command; scripts and UIs gate on it and see a player that claims 6× fewer decoders than
it has, with a feature tag naming a vertical the build has outgrown.
What correct looks like: enumerate the wired provider (HostDecoderProvider) or a registry
constant shared with pipeline_host, as Wave 3 already recommended for bench.rs.
Confidence: HIGH — Verification: ran (`ok: engine … decoders: mjpeg, pcm_s16le` while
h264/aac tracks played, transcript in log).

F-003 [S2]
Where: apps/cli/src/host.rs:181, :1372, :1404 (the settings seam)
Code:
```
    /// Session-local settings overrides (the settings-schema seam).
    settings: BTreeMap<String, SettingValue>,
```
```
        let value = match self.settings.get(&full) {
```
```
        self.settings.insert(full, value.clone());
```
What is wrong: the store is process-local and read by exactly one consumer
(`setting_get`). No file is ever written (verified: fresh process answers the schema
default; no ~/.config/lumen, no ~/.lumen). No playback parameter ever consults it —
`rg "self.settings" apps/cli/src/host.rs` → only :1372 and :1404. `config.set-setting`
is validate→store→echo; `SettingApplied { applied, restart_required }` implies an effect
that does not exist. This confirms and extends Wave 11 (W11-A07 F-002): no persistence
AND no behavioral consumer. Why it matters: a user who sets `global.developer-mode on`
(or any of the 121 keys) gets `ok: set on` and nothing changes, ever, in any process.
What correct looks like: either a persisted store (config file at exit / load at boot)
plus at least one consumer, or an honest "accepted but not yet applied" answer.
Confidence: HIGH — Verification: ran (REPL round-trip + fresh-process default, transcript
in log) + read (consumer search).

F-004 [S2]
Where: apps/cli/src/host.rs:1351-1357 (in `current_time`, :1335-1363)
Code:
```
        let buffered = match duration_us {
            Some(duration) => vec![TimeRange {
                start: Timestamp(0),
                end: Timestamp(duration),
            }],
            None => Vec::new(),
        };
```
What is wrong: `media.get-current-time`'s `buffered` field is fabricated as the full
duration, always. Nothing in the pipeline tracks buffered ranges (the ring is capped at
`AUDIO_PARK_FILL` = 32 768 frames ≈ 0.74 s, `pipeline_host.rs:135`); at position 0.1 s of
a 6 s file the answer claims 6 s buffered. The engine emits no buffered-range fact.
Why it matters: a UI progress bar driven by this field shows a fully-buffered bar for
every local file and would show the same lie for a network source once those land; it is
wire data with no source of truth. What correct looks like: derive from the demuxer's
consumed extent (packets pulled vs file size) or answer an empty list until the fact
exists. Confidence: HIGH — Verification: read (no buffered-range producer exists anywhere
in the host; `rg TimeRange apps/cli/src` → only this fabrication).

F-005 [S3]
Where: apps/cli/src/host.rs:1569 (in `default_value`, :1565-1571)
Code:
```
        DefaultValue::Ref(path) => SettingValue::String(format!("-> product.toml {path}")),
```
What is wrong: for every `default_ref` setting, `config.get-setting` answers the VALUE
`"-> product.toml defaults.locale"` — a ref marker string, not a value. Executed:
`lumen setting get global.language` → `ok: -> product.toml defaults.locale
(--global-language)`. Why it matters: the wire type says `SettingValue::String`; a
consumer cannot distinguish a real string value from the placeholder. The `[PROPOSAL]`
at :1562-1564 admits the product-config join is owed. What correct looks like: resolve
the ref against product.toml at build/startup, or refuse the key until the join lands.
Confidence: HIGH — Verification: ran.

F-006 [S3]
Where: apps/cli/src/host.rs:862-864 (in `submit_and_await`, :854-866)
Code:
```
        handle
            .submit(request, kind)
            .map_err(|_| session_error_wire(&SessionError::SessionClosed))?;
```
What is wrong: the submit error is discarded (`|_|`) and always reported as
`SessionClosed`. `session_error_wire` (:1548-1553) defines a typed `InboxBusy`
(ResourceLimit, "the session inbox is full — poll the engine, then retry") that can never
surface from this call site — the one place a caller could act on it (retry after a pump).
Why it matters: swallowed error detail on the command-submission path; a caller hitting a
full inbox is told the session closed, which is false and non-recoverable-looking.
What correct looks like: map the actual `SessionError` through `session_error_wire`.
Confidence: HIGH — Verification: read.

F-007 [S3]
Where: apps/cli/src/host.rs:1496-1521 (in `rejection_wire`, :1469-1533)
Code:
```
                "play in {state_name:?} means restart-from-ended — the Phase-3 seek flow (card 3.A1)"
```
(`:1500`; siblings at :1506 "card 3.A2", :1519 "the doc 05 §3.3 ended→buffering wire edge (Lead-owned change)")
What is wrong: wire-visible error strings embed internal planning vocabulary (card ids,
doc sections, phase names, "Lead-owned"). These reach the user verbatim through
`error: [session.not-ready] …` (executed for the chapter variant at :1085, below).
What correct looks like: user-facing message states the refusal; the card/doc reference
belongs in `details`. Confidence: HIGH — Verification: read + ran (chapter error printed).
Note: the same jargon pattern is in the chapter message :1085 ("the chapter card owns
this") — executed live.

F-008 [S4]
Where: apps/cli/src/host.rs:1798-1802 (in `long_wav`, :1777-1806; module doc :1767)
Code:
```
        let path = std::env::temp_dir().join(format!(
            "lumen-backpressure-{}-{}s.wav",
            std::process::id(),
            seconds
        ));
```
What is wrong: (a) the doc comment at :1767 claims "cleaned up by the OS" — false on
Linux; the file is never unlinked, so each test run leaks ~1 MB (6 s stereo 44.1 kHz)
into /tmp per test (2 tests); (b) the name is predictable (pid + seconds) in a
world-readable /tmp — a pre-placed symlink would redirect the write (test-only, low
blast radius); (c) the unmarked identity literal is the standing `check-identity` /
full_gate step-8 red (Wave 1 root-caused here; bench.rs:557 shows the marker pattern).
What correct looks like: `tempfile::NamedTempFile` (unique + dropped) or create+delete,
plus the `// lumen-allow-literal` marker. Confidence: HIGH — Verification: read + Wave-1
baseline (audit/runs/W01-A08.log.md:68).

F-009 [S4]
Where: apps/cli/src/host.rs:1006 (in `open_session`)
Code:
```
            generation: generation.unwrap_or(Generation(0)),
```
What is wrong: after a successful open, a missing generation is fabricated as 0 instead
of being surfaced; the same pattern is duplicated at dispatch.rs:68 for every response.
If `engine.open` succeeded but generation is None that is an engine invariant breach and
is silently papered over. Blast radius: likely unreachable (open always mints a
generation), hence S4 not S2. Confidence: MEDIUM — Verification: read.

F-010 [S4]
Where: apps/cli/src/host.rs:1452 (in `enrich_track_info`, :1441-1466)
Code:
```
            frame_rate: fact.frame_ns.map(|ns| (1.0e9 / ns as f64) as f32),
```
What is wrong: a container that declares `frame_ns == 0` yields `frame_rate: inf` on the
wire (float division, no panic). Defensive zero-check absent. No in-tree container
declares 0 today (UNVERIFIED beyond the demux fixtures read). Confidence: LOW —
Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Default::default / new | 220-251 | REAL | — | 15-field honest init |
| set_event_throttle | 260-262 | REAL | — | determinism seam; prod unset (real clock) |
| set_present_tap | 273-278 | REAL | — | pre-open stash rides first consumer; desktop uses it (frame_tap.rs:34) |
| settle | 288-298 | REAL | — | bounded AWAIT_PUMPS=64 pipeline pumps |
| state / session_id / generation | 302-316 | REAL | — | engine delegates |
| events / state_history | 320-328 | REAL | — | observability trail; survives close (intended) |
| position_us | 346-359 | REAL | — | audio-master vs limb-less latched arm; doc admits F3 wall-elapsed gap, code avoids it |
| facts / track_snapshot / engine_stats | 363-378 | REAL | — | delegates |
| audio_lane_counters / headless_accounting | 387-425 | REAL | — | test/AC observability |
| screenshot_png | 430-435 | REAL | — | consumer delegate |
| pump_pipeline / pump | 447-461 | REAL | — | the scheduler split (module doc honest) |
| pump_pipeline_until / pump_until / pump_until_state / advance | 464-501 | REAL | — | bounded; pred checked before pump |
| pump_wallclock / pump_wallclock_at | 523-536 | REAL | — | injectable instant (deterministic tests) |
| refresh_wall_anchor | 543-570 | REAL | — | re-anchor on Playing entry + rate change; None-arm handled |
| advance_time_wall | 576-634 | REAL | — | due-frames math u128, saturating; renders ≤ wall owed |
| drain_outputs | 642-684 | REAL | — | scratch reuse (D-1); state_history/tracks/last_fatal updated here |
| advance_time | 703-750 | REAL | — | paused-clock guard correct (doc :692-702 explains why) |
| audio_channels / audio_rate_hz | 754-783 | REAL | — | geometry lookup with documented fallbacks |
| drain_tail_at_end | 795-830 | REAL | — | resume→render residue→pause; sized residue, no stranding |
| surface_video_error | 835-841 | REAL | — | one-shot fatal latch |
| next_request / submit_and_await | 848-866 | REAL | S3 | F-006 error swallowing |
| receipt_outcome | 869-887 | REAL | — | honest mapping; `let _ = session` on Completed |
| open_session | 890-1008 | REAL | S4 | F-009 Generation(0); open→settle→fatal-check honest |
| teardown_after_fatal / teardown_session_scoped / session_closed | 1013-1039 | REAL | — | M17 half: run.rs:446-481 graceful_exit → dispatch close → session_closed → this clears handle/consumer/device/clock/facts/tracks/receipts |
| dispatch_seek | 1059-1150 | REAL | — | class-A accept, boundary duties gated on failed_at_accept, bounded terminal wait with typed Internal timeout |
| await_receipt | 1154-1170 | REAL | — | 64 pumps, typed Internal on expiry |
| seek_boundary | 1201-1238 | REAL | — | exact-prefix discard via consumer bookkeeping + fast-completion clock re-anchor compensation |
| discard_ring_prefix | 1258-1280 | REAL | — | render-discard; [PROPOSAL] for real device flush is honest |
| media_info | 1283-1314 | FALSE-field | S0 | F-001 seekable hardcode |
| duration | 1317-1333 | REAL | — | facts delegate |
| current_time | 1335-1363 | FALSE-field | S2 | F-004 buffered fabrication |
| setting_get / setting_set | 1366-1409 | REAL-mechanism | S2 | F-003: echo-only store |
| session_state_as_wire / is_seek_terminal / track_info_wire_host | 1416-1435 | REAL | — | delegates/matches |
| enrich_track_info | 1441-1466 | REAL | S4 | F-010 inf frame_rate |
| rejection_wire | 1469-1533 | REAL | S3 | F-007 jargon in user errors |
| session_error_wire | 1536-1560 | REAL | — | typed; InboxBusy unreachable from submit site (F-006) |
| default_value | 1565-1571 | SHALLOW | S3 | F-005 ref marker as value |
| toml_to_setting_value / setting_value_to_toml / setting_type_name | 1573-1627 | REAL | — | lossless-ish; table→"(table)" string placeholder (S4 edge) |
| engine_info | 1631-1661 | STALE-HARDCODE | S2 | F-002 |
| command_listings | 1665-1676 | REAL | — | all 65 catalog ids; since/deprecated/requires hard-coded consistently with catalog v1 (spec.rs has no such fields); NOTE: 65 listed vs SERVED 15 (dispatch.rs:38-54) — honest introspection by §2.12 design, but menus built from it are 77% refusing rows |
| tests: vorbis/pcm/adts limb seam (3) | 1701-1757 | REAL | — | real fixtures, mutation-sensitive asserts |
| tests: backpressure pair (2) | 1836-1939 | REAL | — | would fail if parking or wall-bound broke; F-008 temp-file hygiene |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| vorbis_session_opens_with_an_audio_limb | YES | asserts device + mono channel count vs stereo default — fails if geometry lookup regresses |
| pcm_session_keeps_its_audio_limb | YES | wav regression pin |
| in_band_config_audio_stays_limb_less | YES | asserts the GAP honestly (device None) |
| settle_loop_parks_instead_of_fatalling_the_ring | YES | fill ≤ AUDIO_PARK_FILL+16384 and < RING_FRAMES; resume-to-exact-end position window 5.99-6.01 s — fails if parking fatals or end position drifts |
| wall_pump_renders_only_what_the_wall_owes | YES | pins exact sample counts (1470, 44_100) and paused no-render — deterministic synthetic instants |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the old stop→close→reopen composition … is FORBIDDEN for seek" | host.rs:35-38 | REAL | dispatch_seek :1059-1150 submits engine Seek; no reopen in the path |
| "Deterministic by construction on the audio-bearing path: nothing reads a wall clock" | host.rs:22-24 | REAL | audio arm derives from pump count; wall clock only in pump_wallclock (documented) |
| "a seek visibly lands before the next line is read" | host.rs:69-70 | REAL | dispatch_seek pumps to terminal before returning; REPL settles class-A (run.rs:279-281) |
| seek is "in-pipeline … the seven-step flow of doc 03 §6" | host.rs:29-33 | REAL-UNPROVEN | engine seek.rs/session.rs:1995-2031 read; full 7-step ordering not independently executed beyond accept+terminal |
| "cleaned up by the OS" (test temp file) | host.rs:1767 | FALSE | no unlink anywhere in the module; Linux does not auto-clean /tmp |
| UI disables seek per `media.get-info` "not by guesswork" | docs/08:346 | FALSE (input is fabricated) | F-001 executed |

## 7. Dead / duplicated / unreachable code
- None dead. `session_state_as_wire` (:1416) is a shim by design (doc comment explains:
  avoids private-field dependency). Duplication note: the wall/audio advance render loops
  (`advance_time` :712-749 vs `advance_time_wall` :598-633) share shape by intent (same
  vsync-then-advance order) — acceptable, documented.
- `HeadlessAccounting` (:107-143) + `audio_lane_counters` are AC/test observability used
  by apps/cli/tests/* (verified: tests/pipeline_accounting.rs, native_seek.rs use
  RUN_PUMPS/headless accounting).

## 8. Dependencies & flags
No feature gates in this file; everything is in the default release lane (baseline
cargo-build-release-lumen-cli.txt exit 0). Imports: audio_output, commands, media_clock,
media_types, playback_engine, settings_schema — all shipped. No unsafe blocks in the file.

## 9. Open questions for the next agent
- dispatch.rs (W13-A06): SERVED=15 vs command_listings' 65 — decide whether
  `system.list-commands` should carry a served marker; CommandListing has no field for it.
- parse.rs: error hint suggests `playback.autoplay-next` (parse.rs:353/363) which the
  121-key schema rejects — the CLI points users at a key that does not exist (executed:
  `setting get playback.autoplay-next` → unknown setting key).
- Engine side: expose `pipeline.seekability()` to hosts so F-001's fix does not require
  scraping the MediaInfoReady event.
- UNVERIFIED: the module-doc measurement claims ("±60-200 µs", "measured overrun ~1.0 s
  … clock at 2.54 s of a 1.5 s file") — historical measurements, not re-run (cargo-build-
  budget discipline; the fix they describe is in the code and its test passes in baseline).
