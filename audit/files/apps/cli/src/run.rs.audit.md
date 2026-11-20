# apps/cli/src/run.rs — W13-A01 audit

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A01 |
| Date | 2026-11-16 |
| Lines | 1053 (≈21 functions + test module) |
| File verdict | **REAL** |
| Reachable from shipped entry point | YES — `main` → `run::run` (main.rs:19); chain: main.rs:19 `run::run(&argv,…)` → run.rs:66 `run` → run.rs:132 `repl_mode` (open) / run.rs:145 `one_shot` / run.rs:109 `machine_mode` / run.rs:144 `bench::run_bench` |
| Findings | F-001 [S2], F-002 [S3], F-003 [S3], F-004 [S4], F-005 [S4] |

## 1 One-paragraph verdict

The runner is real and complete for its three modes: argv one-shot, the interactive REPL, and `--control-stdio` NDJSON machine mode, all funneling through `lumen_dispatch` exactly as the ARCH-5 claim states. Every REPL command the grammar can parse has a live handler; there is no "not implemented" print anywhere in this file — unserved catalog verbs get the typed `[command-layer.unsupported-for-host]` refusal from dispatch.rs:155. The genuinely weak spots are presentational honesty, not pipeline honesty: the `help` verb re-prints a stale seek line that describes a mechanism that no longer exists (and that fails on MP4 in reality), the human event printer falls back to raw `Debug` for every engine event outside its frozen subset, and the open path prints response-then-events while every subsequent class-A command prints events-then-response.

## 2 What actually works (verified)

- The argv `open` mode seeded and drove the full session: orchestrator run (audit/runs/baseline/mp4-smoke-open-advance.txt) — `ok: accepted session=1 gen=0`, `event: tracks 2 ready`, `TimingDurationKnown`, `end-of-media`, `state: Ended (position 3.041814s)`, `ok: time 3.041814s (100.0%)`, `event: state-changed Ended → Idle`, `EXIT=0`.
- The graceful-exit teardown (run.rs:293 `graceful_exit`) is visible in BOTH executed transcripts: `Playing → Stopped → Idle` after `quit` on a live session (mp4-smoke-open.txt lines 13-14) and `Ended → Idle` after end-of-media (mp4-smoke-open-advance.txt line 17).
- My own live runs of the shipped binary (2026-11-16, no cargo, binary as built by orchestrator):
  - `lumen engine-info` → `ok: engine … decoders: mjpeg, pcm_s16le`, `EXIT=0`.
  - `printf 'info\ntracks\nquit\n' | lumen open …/sample-h264-aac.mp4` → `ok: media / container: mp4 / duration: 3.000000s / tracks: 2 / #0 video h264 320x240 (selected) / #1 audio aac 1ch 44100Hz (selected)` — the class-S snapshot path (run.rs:151 `one_shot`-shaped dispatch through `to_catalog` → `lumen_dispatch`) works against the real h264+aac file.
  - REPL error path: `bogus` line → parse error printed, session torn down cleanly, exit 0.
- One-shot exit-code honesty is pinned by the in-file tests (run.rs:927-971): unknown area/verb → 64, `play` without session → 69 `[session.not-ready]`.
- Machine mode: NDJSON round-trip and the malformed-request teardown-before-exit are pinned by run.rs:998-1019 and were part of the 1001-pass suite.

## 3 Findings

### F-001 [S2] The REPL `help` verb prints a false seek mechanism description
Where: surfaced at run.rs:243-250 (the REPL `Help` arm); the false text lives in help.rs:55-57
```rust
    (
        "seek 90s | +10s | -1m | 50% | chapter 3",
        "playback.seek",
        "seek (v0: reopen at the target through the engine's real transitions)",
    ),
```
Wrong: two independent lies. (1) There is no reopen: host.rs:36-38 states the old stop→close→reopen composition "is FORBIDDEN for seek (D-4/D-7)"; the current `dispatch_seek` (host.rs:880-971) submits the engine's native `Seek` command with zero re-open. (2) The real behavior on the flagship MP4 lane: I ran `printf 'seek 1s\ntime\nquit\n' | ./target/release/lumen open audit/runs/baseline/sample-h264-aac.mp4` and got `ok: accepted session=1 gen=1` followed by `event: TimingSeekFailed { error: CommandError { stage: Demux, code: Unsupported, message: "[demux] unsupported (plugin.demux.reanchor.unsupported)" … } }` and `ok: time 0.000000s (0.0%)`. So the user is told "reopen at the target" while the shipped binary neither reopens nor lands the seek on MP4 — the accept receipt plus terminal failure event is the honest class-A shape (host.rs:64-74 documents it), but the help line describes a third mechanism that exists nowhere.
Correct: describe the engine-native seek and its per-container seekability (AVI seeks land, mp4 reanchor is the demuxer's typed gap — containers/mp4.rs:173-178).
Confidence: high — live binary transcript + sed-verified quotes (`sed -n '54,58p' apps/cli/src/help.rs`, `sed -n '880,971p'` host.rs).

### F-002 [S3] The human event printer's vocabulary is frozen at the first vertical; everything else dumps raw `Debug`
Where: run.rs:718-741
```rust
fn print_event<W: Write>(event: &WireEvent, out: &mut W) {
    use commands::EventKind;
    let what = match &event.kind {
        EventKind::SessionStateChanged { from, to } => {
```
… and the fallback at run.rs:738:
```rust
        other => format!("{other:?}"),
    };
```
Wrong: the doc comment (718-719) claims "the stable event vocabulary of doc 05 §3.2, this host's subset" — but the engine emits `TimingDurationKnown`, `MediaInfoReady`, `PlaybackEndOfTrack`, `TimingSeekFailed` and more, and every one of them reaches the user as an unredacted Rust `Debug` dump. Verified in BOTH executed transcripts: `event: TimingDurationKnown { duration: Timestamp(3000000) }` (mp4-smoke-open-advance.txt line 7) and the multi-line `event: MediaInfoReady { info: MediaInfo { … } }` blob (line 8). The class-A seek failure — the single most important terminal a script must see — prints as a raw `Debug` struct with the full `CommandError` inside.
Correct: extend the human vocabulary to the engine's current event set (the wire types are in `commands::EventKind`); the file already demonstrates the pattern six times.
Confidence: high — executed transcripts + sed-verified fallback arm.

### F-003 [S3] Inconsistent event/response ordering between the open and every later command
Where: run.rs:205-206 vs run.rs:282-284
```rust
    print_response(&response, json, out, err);
    print_new_events(&host, printed, out);
```
versus
```rust
                print_new_events(&host, printed, out);
                printed = host.events().len();
                print_response(&response, json, out, err);
```
Wrong: the open prints response-then-events; every subsequent REPL command prints events-then-response. My live transcript shows the causal inversion: the seek's `TimingSeekFailed` event line prints BEFORE the `ok: accepted session=1 gen=1` line that causally precedes it. Deterministic, but the transcript's causal order is scrambled for every command after the first, and the two halves of the same file disagree with each other.
Correct: pick one order (response first matches "ok:" as the command's verdict) and apply it to both call sites.
Confidence: high — both call sites sed-verified; live transcript exhibits the inversion.

### F-004 [S4] Dead no-op statement in repl_mode
Where: run.rs:219
```rust
    let _ = interactive;
```
`interactive` is genuinely used at line 221 (`if interactive {` prompt). The line-219 discard is a leftover — confusing (it asserts the parameter is unused when it is not). Confidence: high. Severity S4.

### F-005 [S4] `decode-bench` refusal messages hard-code usage prose in two arms
Where: run.rs:836-839 — the argv-only refusal for `decode-bench` in a one-shot position is honest (typed `invalid-argument`, exit 64, pinned by run.rs:1046-1052 `host_verbs_are_repl_only`). No defect beyond the duplicated `lumen` literal already tracked by the repo's own `lumen-allow-literal` markers. S4, recorded only.

## 4 Per-function table

| Function (line) | Role | Verdict |
|---|---|---|
| `run` (66-147) | argv → parse → version/help/machine/one-shot/repl/bench routing; exit codes | REAL — orchestrator runs |
| `one_shot` (151-171) | one catalog command, one dispatch, print, exit code | REAL — live `engine-info`/`info` runs |
| `repl_mode` (180-296) | open seeds session; stdin loop; advance/quit/help host verbs; class-A settle; graceful exit | REAL — both smoke transcripts |
| `machine_mode` (302-372) | `--control-stdio` NDJSON loop; teardown on EOF/protocol/serialization errors | REAL — test-pinned (998-1019) |
| `print_new_events_json` (376-389) | NDJSON event tail, serialization failure surfaced | REAL |
| `graceful_exit` (446-481) | stop→close twice + bounded settle + tail pumps; never abandons session | REAL — `Playing → Stopped → Idle` in live transcripts |
| `teardown_step` (486-498) | one teardown dispatch; typed failure to stderr, exit proceeds | REAL |
| `print_response` (506-528) | human/NDJSON envelope; json-serial failure typed | REAL |
| `print_outcome` (531-550) | Accepted/Completed human forms | REAL |
| `print_result` (553-676) | class-S payload human forms (info/time/duration/tracks/engine/commands/settings) | REAL — live `info` transcript |
| `print_track` (679-709) | one track row incl. harvested geometry | REAL — `#0 video h264 320x240` live |
| `print_new_events` (712-716) | drain host events since `printed` | REAL |
| `print_event` (720-741) | human event line; frozen vocabulary + Debug fallback | REAL with F-002 |
| `print_parse_error` (744-747) | two-line parse failure | REAL |
| `format_us` (750-752) | µs → fixed 6-decimal seconds | REAL — pinned (1023-1027) |
| `format_setting_value` (755-763) | setting value → human | REAL |
| `format_setting_number` (766-772) | integral vs fractional number print | REAL |
| `is_class_a` (776-787) | event-evidence command classes (settle targets) | REAL |
| `to_catalog` (795-845) | CLI verb → catalog command; the full command→handler map (see §7 note) | REAL — complete for the grammar |
| `split_setting_key` (852-860) | `area.key` split, typed refusal | REAL |
| `parse_setting_value` (864-887) | on/off/number/list/string scalar grammar | REAL — pinned (1030-1044) |
| tests (889-1053) | 13 in-file tests incl. exit codes 0/64/69, NDJSON shapes | REAL — in the 1001-pass suite |

Command → handler map (the hostile question, answered): REPL verbs `quit|q`→break (242); `help`→`help::help_text` (243); `advance [ticks]`→`host.advance` (251-261); every other parsed verb → `to_catalog` (795-845) → `lumen_dispatch` → dispatch.rs `route` (75-156): `open`→`host.open_session` (+`--start-paused` settle+pause composition, dispatch.rs:88-99); `close`→`submit_and_await(Close)`+`session_closed`; `play|pause|toggle|stop`→`submit_and_await(kind)`; `seek`→`host.dispatch_seek`; `info|time|duration`→host snapshots; `tracks`→`host.track_snapshot`; `engine-info`→`host::engine_info()` (static — see host.rs audit); `commands`→`host::command_listings()`; `setting get/set`→`host.setting_get/set`. Catalog verbs OUTSIDE this grammar (machine-mode-only, e.g. `playback.set-rate`) → dispatch.rs:155 `other => Err(unserved(other))` — typed `[command-layer.unsupported-for-host]` with owning-subsystem detail, never "not implemented" prose. Machine mode verified live-adjacent by test `control_stdio_answers_ndjson` (run.rs:998-1011: `playback.set-rate` answers `unsupported-for-host`).

## 5 Test honesty

- run.rs's own 13 tests (889-1053) cover version/help exits, usage errors, one-shot class-S, `--json` envelopes, settings seam, machine mode, µs formatting, host-verb refusals — all in the 1001-pass suite.
- apps/cli/tests/command_surface.rs drives the REAL binary for `engine-info` (incl. pinning the stale decoder list — see host.rs audit F-001), `--help engine`, control-stdio.
- apps/cli/tests/container_conformance.rs:426 `mp4_h264_family_lane` drives `open`+`advance`+`time`+`quit` through the REAL binary — the same repl_mode path audited here; asserts 100% time, `Playing → Ended`, and NO `cli.codec_unavailable`. That test is the strongest single honesty pin on this file.

## 6 Doc claims

- docs/feature-matrix.toml:152 and :428 cite "apps/cli mp4_h264_family_lane (mp4_h264_aac.mp4 plays to Ended, exit 0)" — CONSISTENT with the executed smoke and with this file's role.
- docs/feature-matrix.toml:1288 cites "apps/cli/src/demux_host.rs open_http_source" (not this file).
- The module doc (run.rs:42-47) says decode-bench "bypasses the catalog dispatch by design" — TRUE (run.rs:144 calls `bench::run_bench` directly).
- lib.rs:46-48 claims the REPL is "doc 05 §5.3's controller mode" with "pumps at command boundaries" — TRUE per code (settle only for class-A, `advance` as the time mover).

## 7 Dead/duplicated/unreachable

- run.rs:219 `let _ = interactive;` — dead statement (F-004).
- `print_result`'s final arm (669-674) handles result variants "outside the served subset … printed generically rather than panicked on" — genuinely unreachable through this host's dispatch but defensibly defensive; not a lie (doc says so).
- No duplicated logic of consequence; the printing helpers are single-sourced.

## 8 Dependencies & flags

The runner itself is std-only (BufRead/Write). `serde_json` enters for machine mode (`serde_json::from_str` at 332, `to_string` at 350/378/513) exactly as the Cargo.toml header claims. `commands`, `crate::host`, `crate::parse`, `crate::exit`, `crate::help`, `crate::bench`, `crate::dispatch` — all internal. Codec feature posture is irrelevant at this layer except that engine-info prints through `host::engine_info()` (see host.rs F-001): the shipped default includes `codec-h264-sw` (crates/codecs/Cargo.toml:47-53), so the binary the runner launches can decode h264 while the runner's `engine-info` output says it cannot.

## 9 Open questions

- Does any consumer of machine-mode NDJSON depend on the events-then-response order of repl_mode? No consumer found in-tree (`--control-stdio` mode always prints response first, then events — run.rs:349-370); the inconsistency is REPL-only. UNVERIFIED for external scripts.
- `host.settle()` after every class-A command is bounded by AWAIT_PUMPS=64 — a pathological file could leave the session in `Buffering` past the bound and the REPL would proceed with the next command against a transient state. No observed instance in executed runs.
