# AUDIT: apps/cli/src/host.rs

| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-22 |
| Lines / functions | 1941 / 76 (incl. 6 test fns, 3 test modules) |
| File verdict | REAL (core pump/seek/session machinery) with a fabricated-data reporting layer |
| Reachable from shipped entry point? | YES — `main.rs:19` → `lumen_cli::run::run` → `repl_mode`/`one_shot`/`machine_mode` (run.rs:132/145/109) → `lumen_dispatch` (dispatch.rs) → `CliHost::{open_session, submit_and_await, dispatch_seek, media_info, current_time, …}`; run.rs:216/252/280 call `settle()`/`advance()` directly. `pump_wallclock` is NOT reachable from the CLI binary (see F-004). |
| Findings | S0:0 S1:0 S2:4 S3:7 S4:7 |

Commands run (adaptation: no cargo/git; static only):
- `sed -n '858,866p;721,729p;1300,1313p;1345,1362p;1643,1658p;1775,1782p' apps/cli/src/host.rs` (quote verification, exit 0)
- `rg -n "pump_wallclock"` across repo → only host.rs (def+tests) and `apps/desktop/src/shell.rs:280`
- `rg -n "self\.settings" apps/cli/src` → host.rs:1372,1404 only (no consumer)
- `rg -n "fn queue"` crates/playback-engine/src/session.rs:437-442 (submit error has TWO variants)
- `rg -n "no decoder in this build"` apps/cli → bench.rs:334 (the string the task asks about is produced in bench.rs, NOT host.rs)
- W00 baseline (cited): MP4 opens, clock stuck at 0.000000s while Playing unless `advance N`; advance pumping reaches Ended/100%.

## 1. One-paragraph verdict

This file is the real session host: the pump model (engine poll → output drain → time advance), the seek boundary duties, receipt correlation, and the class-S snapshot answers are genuine, single-writer, bounded, and covered by in-file tests that drive the real `open_session` over committed fixtures. The W00 baseline (advance-pumping reaches Ended/100%) proves the pump core executes end-to-end. What is NOT real is the reporting layer: `media_info()` hardcodes `seekable: true`, `current_time()` invents a full-duration "buffered" range, `engine_info()` hardcodes a 2-decoder list and a "first-vertical" feature string, the settings store is write-only, and the wall-clock pump that would make the CLI actually play in real time exists and is tested but is wired to nothing in this binary — which is exactly why W00 observed `time` frozen at 0.000000s while `Playing`.

## 2. What actually works (verified)

- Pump/advance core: W00 baseline runs (scripted sessions, `advance` to Ended at 100%) exercise `pump`/`advance`/`drain_tail_at_end` through the real open path; the in-file tests `settle_loop_parks_instead_of_fatalling_the_ring` (host.rs:1836-1877) and `wall_pump_renders_only_what_the_wall_owes` (host.rs:1888-1939) assert exact sample counts (1470, 44100) — mutation of the wall-bound arithmetic would move those counts. Not re-run here (no cargo): REAL-UNPROVEN by me, REAL by W00 baseline evidence for the advance path.
- Seek machinery (`dispatch_seek`/`seek_boundary`/`discard_ring_prefix`, 1059-1280): coherent class-A flow, typed refusal arms, exact-prefix ring discard with clock re-anchor compensation; matches the doc 03 §6 contract as written. Fixtures for its integration tests exist (verified paths).

## 3. Findings

### F-001 [S2]
Where: apps/cli/src/host.rs:1307
Code:
```rust
                seekable: true,
                is_live: false,
```
What is wrong: `media_info()` hardcodes `seekable: true` and `is_live: false` for every session. Nothing probes the source for seekability (no index check, no range check), yet the wire answer asserts it.
Why it matters: any source that is not seekable (stream, file without an index) is reported seekable; a caller that trusts `media.get-info` will attempt seeks that fail late instead of refusing early. This is fabricated wire data, not a default.
What correct looks like: seekability derived from the opened source (the demuxer's own capability flag), `false` when unknown.
Confidence: HIGH — Verification: read (sed -n '1300,1313p').

### F-002 [S2]
Where: apps/cli/src/host.rs:1351-1357
Code:
```rust
        let buffered = match duration_us {
            Some(duration) => vec![TimeRange {
                start: Timestamp(0),
                end: Timestamp(duration),
            }],
            None => Vec::new(),
        };
```
What is wrong: the `buffered` ranges of `playback.get-time` are fabricated as the whole media `[0..duration]` on every call. No demuxer/decode position, no ring fill, no read-ahead window is consulted.
Why it matters: the wire type promises buffered ranges; a UI progress bar renders "fully buffered" the instant playback starts, for any media, forever. Invented telemetry.
What correct looks like: the actually-decoded/available range(s) from the pipeline (e.g., demuxer position minus ring backlog), or an empty vec until measured.
Confidence: HIGH — Verification: read (sed -n '1345,1362p').

### F-003 [S2]
Where: apps/cli/src/host.rs:862-864
Code:
```rust
        handle
            .submit(request, kind)
            .map_err(|_| session_error_wire(&SessionError::SessionClosed))?;
```
What is wrong: `SessionHandle::submit` fails with either `SessionError::InboxBusy` or `SessionError::SessionClosed` (crates/playback-engine/src/session.rs:438-441). The `|_|` discards the real error and always reports `SessionClosed` ("the session closed before the command landed").
Why it matters: on a full inbox the user is told the session closed — false diagnosis, and the honest remedy attached to `InboxBusy` in this very file ("poll the engine, then retry", host.rs:1548-1553) is never surfaced. Swallowed-error pattern (§3.2).
What correct looks like: `.map_err(|error| session_error_wire(&error))` — the mapping function already handles both variants.
Confidence: HIGH — Verification: read both sites.

### F-004 [S2]
Where: apps/cli/src/host.rs:523-526 (and run.rs:216/252/280)
Code:
```rust
    pub fn pump_wallclock(&mut self) {
        let now = std::time::Instant::now();
        self.pump_wallclock_at(now);
    }
```
What is wrong: the wall-clock pump — the only code that advances media time as real time elapses — has zero callers in the CLI. `rg "pump_wallclock"` across the repo returns host.rs (definition + its own tests) and `apps/desktop/src/shell.rs:280` only. The CLI REPL pumps exclusively at command boundaries (`settle()` = pipeline-only, NO time advance) and via the explicit `advance [ticks]` verb (run.rs:252).
Why it matters: this is the machine behind the W00-observed behavior: `time` stays 0.000000s while `Playing` unless the user types `advance N`. The shipped `lumen` binary is a player that does not play; a working, in-file-tested wall pacer is dead code from this entry point. run.rs:17-18 admits it ("Continuous wall-time pumping joins with the session-thread card (3.B3)"), so this is a documented gap, not a hidden lie — but the user-visible core of the CLI is still broken.
What correct looks like: the REPL session thread (or at minimum an `--realtime` flag) driving `pump_wallclock()` on a tick.
Confidence: HIGH — Verification: read + repo-wide grep; W00 baseline corroborates the frozen clock.
Severity note: would be S0 if any doc claimed real-time CLI playback; the honest [PROPOSAL] marker keeps it at S2.

### F-005 [S3]
Where: apps/cli/src/host.rs:725 (same pattern 613, 823, 1273)
Code:
```rust
                    let mut buffer = vec![0.0_f32; RENDER_QUANTUM * channels];
                    if let Err(error) = device.render(&mut buffer) {
```
What is wrong: a fresh ~11.7 KB (stereo, RENDER_QUANTUM=1470) heap allocation per rendered quantum in the pump hot path — once per quantum for the whole session. The same file loudly documents its zero-alloc discipline for the drain path (outbox_scratch, lines 182-185, 642-644: "steady state allocates nothing on the drain path").
Why it matters: per-quantum allocation churn in the only hot loop the host owns; inconsistent with the file's own D-1 standard. Not a correctness bug.
What correct looks like: one reusable render buffer owned by the host (like outbox_scratch), sized to max quantum × channels.
Confidence: HIGH — Verification: read (sed -n '721,729p'); RENDER_QUANTUM=1470 at pipeline_host.rs:92.
Severity: S3 (perf defect, small bounded allocs).

### F-006 [S3]
Where: apps/cli/src/host.rs:1390-1409 (store), 1372 (sole reader)
Code:
```rust
        self.settings.insert(full, value.clone());
        Ok(CommandResult::SettingApplied {
            applied: value,
```
What is wrong: `setting_set` validates against the schema and inserts into `self.settings`; repo-wide grep shows the ONLY reader of that map is `setting_get` (host.rs:1372). No engine, decoder, filter, or output path consumes any stored setting. The wire answer `SettingApplied` is factually false — nothing is applied to playback behavior.
Why it matters: §3.5 "config declared but never read" — a settings seam that can never change what the player does, while telling the caller it was applied.
What correct looks like: either wiring the consumed subset into SessionConfig/engine options, or an honest `Accepted`-without-`applied` answer naming the seam.
Confidence: HIGH — Verification: read + `rg "self\.settings" apps/cli/src` → 2 hits, both inside the getter/setter.
Severity: S3 (the seam is at least schema-validated and honest about restart_required).

### F-007 [S3]
Where: apps/cli/src/host.rs:1643-1658 (and 1637-1641)
Code:
```rust
        features: vec![
            "first-vertical:avi-mjpeg-pcm".to_string(),
            "audio.null-device".to_string(),
            "video.software-compositor".to_string(),
        ],
        plugins: Vec::new(),
        decoders: vec![
            DecoderInfo {
                name: "mjpeg".to_string(),
```
What is wrong: `engine_info()` hardcodes the decoder list (mjpeg, pcm_s16le), the feature list, `catalog_version: "1"`, and `protocol_version: "unwired …"` instead of deriving them from the live `HostDecoderProvider`/build. `active` is always `false` even for the two decoders that do exist.
Why it matters: the wire's `engine-info` is a static string table that will silently lie the moment a decoder is added or gated off; `active: false` is wrong the moment a session is playing. This is also the in-code twin of the bench's refusal string "no decoder in this build (first vertical: mjpeg + pcm)" (bench.rs:334) — the build's decoder poverty is real and consistently stated, but as hand-maintained data, not truth.
What correct looks like: enumerate from the decoder provider registry (which exists — `HostDecoderProvider` in pipeline_host.rs).
Confidence: HIGH — Verification: read; bench.rs:334 grep-confirmed.

### F-008 [S3]
Where: apps/cli/src/host.rs:1294-1311
Code:
```rust
                source: self.source.clone().unwrap_or_else(|| {
                    commands::catalog::types::SourceSpec::File {
                        path: std::path::PathBuf::new(),
                    }
                }),
                container: Some(facts.container.clone()),
                title: None,
                ...
                metadata: BTreeMap::new(),
                tracks: self.tracks.clone(),
                chapters: Vec::<Chapter>::new(),
```
What is wrong: (a) the fallback fabricates an empty-path `File` source — `source` and `facts` are always set/cleared together (open_session 990, teardown 1030), so the branch is dead code that would answer a lie if reached; (b) `title`, `metadata`, `chapters` are hardcoded empty for every media — no metadata harvesting exists anywhere behind them.
Why it matters: media-info answers structurally promise metadata/chapters and always deliver nothing — a missing feature surfaced as empty data instead of an honest absence; plus a dead lying fallback.
What correct looks like: drop the fallback (source is an invariant pair with facts); metadata/chapters either harvested or the type narrowed.
Confidence: HIGH — Verification: read; set/clear sites checked.

### F-009 [S3]
Where: apps/cli/src/host.rs:1080-1088
Code:
```rust
            WireTarget::Chapter { index } => {
                return Err(CommandError::new(
                    Stage::Session,
                    ErrorCode::NotFound,
                    format!(
                        "no chapter {index}: AVI v0 carries no chapter markers (the chapter card owns this)"
                    ),
```
What is wrong: the chapter-seek refusal message names "AVI v0" regardless of the open container (WAV, MP4, MKV, ADTS sessions all get an AVI-specific excuse), and leaks internal work-planning jargon ("the chapter card owns this") into a user-facing error.
Why it matters: misleading error message (§3.5) — a user with an MP4 that has chapters is told AVI has none.
What correct looks like: a container-agnostic message ("no chapter source in this build"), no process jargon.
Confidence: HIGH — Verification: read.

### F-010 [S3]
Where: apps/cli/src/host.rs:171-174, 320-322
Code:
```rust
    /// The wire events observed since construction (tests + printing).
    events: Vec<WireEvent>,
```
What is wrong: `events` (and `state_history`) grow for the lifetime of the process, one entry per wire event, never truncated. The REPL prints incrementally (run.rs tracks `printed`), but the host retains every event forever.
Why it matters: unbounded memory growth proportional to session length × event rate on the only long-lived object in the process. Bounded in practice by CLI usage patterns, so S3 not S1.
What correct looks like: a ring window / watermark with the printed cursor held by the caller.
Confidence: HIGH — Verification: read; no truncation site exists (`self.events` only pushed at 678 and cleared never).
Severity: S3.

### F-011 [S3]
Where: apps/cli/src/host.rs:971-974
Code:
```rust
        config.queues = crate::demux_host::queue_plan_from_product_limits(
            crate::demux_host::PRODUCT_QUEUE_PACKETS_AUDIO,
            crate::demux_host::PRODUCT_QUEUE_PACKETS_VIDEO,
        )?;
```
What is wrong: the "product" queue caps are hand-mirrored constants in demux_host ("the values ride the product.toml mirror constants until the product-config load lands", host.rs:969-970) — a duplicated config source that can silently diverge from product.toml.
Why it matters: config drift risk, self-admitted; S3 because it is documented in-place.
What correct looks like: read product.toml at open (the load is "owed" and never landed).
Confidence: HIGH — Verification: read; demux_host constants checked in file 3 audit below.

### F-012 [S4]
Where: apps/cli/src/host.rs:1565-1571
Code:
```rust
        DefaultValue::Ref(path) => SettingValue::String(format!("-> product.toml {path}")),
```
What is wrong: product.toml-ref defaults are surfaced to the wire as the literal string `-> product.toml …` — a placeholder marker a UI would render as the setting's value.
Why it matters: placeholder leaking into user-visible data; the in-code `[PROPOSAL]` admits the join is owed.
Confidence: HIGH — Verification: read.

### F-013 [S4]
Where: apps/cli/src/host.rs:1588
Code:
```rust
        toml::Value::Table(_) => SettingValue::String("(table)".to_string()),
```
What is wrong: TOML tables are stringified to "(table)" — lossy round-trip in the settings value conversion.
Confidence: HIGH — Verification: read.

### F-014 [S4]
Where: apps/cli/src/host.rs:1274
Code:
```rust
            if device.render(&mut discard).is_err() {
                break;
            }
```
What is wrong: the only `render` call whose error is neither recorded nor reported (`is_err()` + silent break), unlike every sibling call which does `note_video_error`. The partial-discard count is returned honestly, but the error cause vanishes.
Confidence: HIGH — Verification: read.

### F-015 [S4]
Where: apps/cli/src/host.rs:1452
Code:
```rust
            frame_rate: fact.frame_ns.map(|ns| (1.0e9 / ns as f64) as f32),
```
What is wrong: `frame_ns == Some(0)` (a hostile/odd container header) yields `inf` as the wire frame rate — no guard. Float division, so no panic; robustness only. Whether 0 can reach here depends on demux_host's TrackFact producers (cross-checked in demux_host audit).
Confidence: MEDIUM — Verification: read; producer range not yet fully traced at time of writing (see demux_host audit).

### F-016 [S4]
Where: apps/cli/src/host.rs:591
Code:
```rust
            let elapsed_ns = u128::from(now.saturating_duration_since(anchor.at).as_nanos() as u64);
```
What is wrong: nanos are cast `as u64` (truncates at ~584 years) and then widened back to u128 — the u128 intermediate the comment advertises ("u128 intermediate; the saturating conversions…") is neutered by the inner cast. Harmless numerically; the comment misdescribes the code.
Confidence: HIGH — Verification: read.

### F-017 [S4]
Where: apps/cli/src/host.rs:1775-1781
Code:
```rust
    /// Write a `seconds`-long 44.1 kHz mono s16le WAV to a unique
    /// temp path and answer it.
    fn long_wav(seconds: u32) -> PathBuf {
        ...
        let data_len = frames * 2 * 2; // stereo s16le
```
What is wrong: doc comment says "mono s16le"; the generator writes stereo (2ch, block align 4). The test still works (geometry is stereo and assertions match), but the comment is false.
Confidence: HIGH — Verification: read (sed -n '1775,1782p').

### F-018 [S4]
Where: apps/cli/src/host.rs:1665-1676
Code:
```rust
            since: "1".to_string(),
            deprecated: None,
            requires: Vec::new(),
```
What is wrong: every catalog listing gets hardcoded `since: "1"`, `deprecated: None`, `requires: []` — fields the wire type carries but nothing populates from data.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Default::default` | 220-224 | REAL | — | forwards to new() |
| `CliHost::new` | 229-251 | REAL | — | all fields empty; no hidden state |
| `set_event_throttle` | 253-262 | REAL | S3? no | test/evidence seam; production leaves unset (comment); used by evidence harness only — check callers: tests only (grep in demux_host/bench confirms usage; UNVERIFIED outside cli) |
| `set_present_tap` | 264-278 | REAL | — | stashes pre-open or installs live; desktop-only consumer |
| `settle` | 288-298 | REAL | — | pipeline-only, bounded AWAIT_PUMPS=64; predicate checked pre-pump |
| `state` | 301-304 | REAL | — | identity shim |
| `session_id` / `generation` | 307-316 | REAL | — | forwarders |
| `events` / `state_history` | 319-328 | REAL | S3 | unbounded growth (F-010) |
| `position_us` | 345-359 | REAL | — | audio-master = counter/1000; limb-less latches last present or anchor; documented wall-elapsed avoidance |
| `facts` | 361-365 | REAL | — | |
| `track_snapshot` | 368-372 | REAL | — | clones Vec |
| `engine_stats` | 374-378 | REAL | — | forwarder |
| `audio_lane_counters` | 381-395 | REAL | — | lock + tuple |
| `headless_accounting` | 397-425 | REAL | — | requires video limb (`guard.video()?`), documented None case |
| `screenshot_png` | 427-435 | REAL | — | None before video limb |
| `pump_pipeline` | 437-451 | REAL | — | poll+drain+surface; NO time advance (the determinism contract) |
| `pump` | 447-461 | REAL | — | + advance_time + drain_tail |
| `pump_pipeline_until` | 463-476 | REAL | — | bounded; predicate checked before first pump |
| `pump_until` | 478-489 | REAL | — | FULL pumps |
| `pump_until_state` | 491-494 | REAL | — | wrapper |
| `advance` | 496-501 | REAL | — | the scripted-session verb (run.rs:252) |
| `pump_wallclock` | 503-526 | REAL / UNREACHABLE-from-CLI | S2 | F-004; only desktop shell + tests call it |
| `pump_wallclock_at` | 528-536 | REAL | — | injectable now; tested with exact counts |
| `refresh_wall_anchor` | 538-570 | REAL | — | exhaustive 4-arm match; rate-change re-anchor |
| `advance_time_wall` | 572-634 | REAL | S3/S4 | per-quantum alloc (F-005); no-device arm ticks 16× per pump unbounded by wall (comment: monotonic master paces itself — depends on scheduler.tick() internals, cross-ref pipeline_host; UNVERIFIED here) |
| `drain_outputs` | 636-684 | REAL | — | reused scratch buffer; state/fatal/tracks recorded only for pipeline-scoped kinds |
| `advance_time` | 686-750 | REAL | S3 | whole-ring drain per pump (documented determinism contract); paused-clock early return; alloc per quantum (F-005) |
| `audio_channels` | 752-765 | REAL | — | geometry lookup, stereo default |
| `audio_rate_hz` | 767-783 | REAL | — | 44100 default; only read by wall pacer (dead when no limb — the anchor is None then) |
| `drain_tail_at_end` | 785-830 | REAL | — | resumes/pauses clock around residue render; breaks on error are noted |
| `surface_video_error` | 832-841 | REAL | — | take-once semantics |
| `next_request` | 845-850 | REAL | — | |
| `submit_and_await` | 852-866 | REAL | S2 | F-003 error swallow |
| `receipt_outcome` | 868-887 | REAL | — | `let _ = session` (ignored payload, harmless) |
| `open_session` | 889-1008 | REAL | S3 | single-session refusal; audio limb from generic geometry; queue caps from mirrored constants (F-011); pumps until non-Opening, fatal → teardown + typed error. Good error honesty |
| `teardown_after_fatal` | 1010-1018 | REAL | — | close only if Error state; `let _` on Close receipt (acceptable: teardown best-effort) |
| `teardown_session_scoped` | 1020-1033 | REAL | — | clears halves; events/state_history deliberately retained |
| `session_closed` | 1035-1039 | REAL | — | |
| `dispatch_seek` | 1041-1150 | REAL | S3 | chapter arm message (F-009); bounded waits, typed failures; snapshot fallback `stale_ring_frames` only used when consumer is None (dead in practice — both set together) |
| `await_receipt` | 1152-1170 | REAL | — | linear scan of pending receipts; unmatched receipts persist until teardown (bounded by command flow) |
| `seek_boundary` | 1172-1238 | REAL | — | exact-prefix via consumer bookkeeping; re-anchor compensation; typed reanchor failure |
| `discard_ring_prefix` | 1240-1280 | REAL | S4 | silent render error (F-014) |
| `media_info` | 1282-1314 | SHALLOW | S2 | hardcoded seekable/is_live (F-001), empty metadata/chapters (F-008), dead source fallback |
| `duration` | 1316-1333 | REAL | — | honest Unknown arm |
| `current_time` | 1304-1363 | SHALLOW | S2 | fabricated buffered (F-002); fraction clamp OK |
| `setting_get` | 1365-1387 | REAL | S4 | reads the write-only store (F-006) |
| `setting_set` | 1389-1409 | REAL | S3 | F-006; validates against schema before insert |
| `session_state_as_wire` | 1412-1418 | REAL | — | documented shim |
| `is_seek_terminal` | 1420-1429 | REAL | — | |
| `track_info_wire_host` | 1431-1435 | REAL | — | deliberate reuse of engine impl (anti-duplication, good) |
| `enrich_track_info` | 1437-1466 | REAL | S4 | inf frame rate on 0 (F-015); channels cast min(u16) OK |
| `rejection_wire` | 1468-1533 | REAL | — | all 10 arms mapped; messages name owning card (process jargon in user errors — S4 tone) |
| `session_error_wire` | 1535-1560 | REAL | — | covers all 5 variants incl. InboxBusy — which F-003 then hides |
| `default_value` | 1562-1571 | SHALLOW | S4 | Ref → marker string (F-012) |
| `toml_to_setting_value` | 1573-1591 | SHALLOW | S4 | table → "(table)" (F-013) |
| `setting_value_to_toml` | 1593-1611 | REAL | — | int/float split sound |
| `setting_type_name` | 1613-1627 | REAL | — | total match |
| `engine_info` | 1629-1661 | SHALLOW | S3 | F-007 hardcoded decoder/feature tables |
| `command_listings` | 1663-1676 | SHALLOW | S4 | F-018 placeholder fields |
| test `fixture` | 1691-1695 | REAL | — | |
| test `vorbis_session_opens_with_an_audio_limb` | 1701-1718 | REAL | — | fixture verified to exist; asserts mono=1 (would catch stereo-default regression) |
| test `pcm_session_keeps_its_audio_limb` | 1723-1739 | REAL | — | |
| test `in_band_config_audio_stays_limb_less` | 1745-1757 | REAL | — | asserts None honestly |
| test `long_wav` | 1777-1806 | REAL | S4 | mono/stereo comment lie (F-017); writes to temp dir, unique name |
| test `playing_host` | 1810-1823 | REAL | — | |
| test `settle_loop_parks_instead_of_fatalling_the_ring` | 1835-1877 | REAL | — | strong: 6s media > ring, park bound asserted, then plays to exact end |
| test `wall_pump_renders_only_what_the_wall_owes` | 1887-1939 | REAL | — | strong: exact 1470/44100 counts, pause renders nothing |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| vorbis_session_opens_with_an_audio_limb | YES | asserts `audio_channels()==1` against the stereo default — a broken geometry seam fails it |
| pcm_session_keeps_its_audio_limb | YES | regression pair of the above |
| in_band_config_audio_stays_limb_less | YES | asserts device None (a geometry false-positive fails) |
| settle_loop_parks_instead_of_fatalling_the_ring | YES | park bound + exact end position 5.99-6.01s would break under both fatal-regression and wedge-regression |
| wall_pump_renders_only_what_the_wall_owes | YES | exact sample-count assertions on synthetic instants; any due-math drift fails |
Caveat: none of these run through `lumen` argv — the wall-pump behavior they prove is unreachable from the CLI (F-004). Not executed in this audit (no cargo): verdicts from reading + W00 baseline.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "pump is therefore the whole scheduler" / deterministic, no wall clock on audio path | host.rs module docs 5-24 | REAL (audio path) / FALSE-as-implied (CLI never advances) | W00: clock frozen while Playing in CLI; only `advance` moves it |
| "a seek visibly lands before the next line is read" | host.rs:68-71 | REAL | dispatch_seek awaits terminal with pipeline pumps (1138-1148) |
| "engine-info" data | wire type implies live data | SHALLOW | F-007: hardcoded tables |
| "media.get-info … seekable" | implied capability | SHALLOW/FALSE-data | F-001 |

## 7. Dead / duplicated / unreachable code

- `pump_wallclock`/`pump_wallclock_at`: unreachable from the CLI binary (F-004) — reachable only via apps/desktop shell.rs:280 and tests.
- `media_info`'s empty-path `SourceSpec::File` fallback (1294-1298): dead (source/facts are set and cleared together).
- `dispatch_seek`'s `stale_ring_frames` snapshot: dead in practice (consumer and device are always both present; the consumer arm of `seek_boundary` always wins).
- `audio_rate_hz`'s 44_100 fallback: only read in the wall pacer's due math, which only runs when a device exists, which only happens when geometry existed — the fallback is unreachable through open_session.

## 8. Dependencies & flags

No feature gates in this file; everything is unconditional in `apps/cli` (the binary the release workflow builds — UNVERIFIED which features the workflow enables, adaptation: no workflow run). Heavy doc-comment dependency on playback-engine/media_clock/audio-output seams, all present in the tree.

## 9. Open questions for the next agent

- pipeline_host.rs: does the video scheduler's `tick()` pace on a monotonic source for limb-less sessions (the claim at host.rs:622-624)? If it paces on the media clock anchored at open, the "wall-clock presents" comment is false and limb-less wall playback would free-run.
- demux_host.rs: can `TrackFact::frame_ns` be `Some(0)` (F-015 robustness)?
- Who consumes `set_event_throttle` outside the cli crate's tests (evidence harness)? If nobody in a shipped entry point, it is a test-only seam in production code.
- The desktop shell (shell.rs:280) drives `pump_wallclock` — does its cadence match the anchor math (drift if the shell ticks slower than the ring drains)?
