# AUDIT: apps/desktop-tauri/src/app.rs
| Field | Value |
|---|---|
| Auditor | W14-A01 |
| Date | 2026-11-21 |
| Lines / functions | 322 / 12 items (+1 phantom call site) |
| File verdict | FALSE (real architecture beneath, but the file cannot compile and its central startup claim is broken; see F-001) |
| Reachable from shipped entry point? | YES by construction — `lumen-tauri` bin → main.rs:50 `lumen_desktop_tauri::app::run()` (sole caller; rg verified) — but NO binary can be produced from this tree (F-001); the w30.8/30.9 release payloads were built from earlier source (UNVERIFIED which) |
| Findings | S0: 1 S1: 2 S2: 2 S3: 3 S4: 2 |
| Commands run | `rg -n "spawn_session_pump\|spawn_event_pump" apps/desktop-tauri` + repo-wide (call site app.rs:301 is the ONLY hit for `spawn_session_pump`; definition is `spawn_event_pump` at :190) · `sed -n` verification of every quoted range (96-113, 144-154, 161-169, 188-196, 196-213, 223-229, 243-247, 271-285, 296-304, 305-322) · Read apps/desktop/src/shell.rs (all 10 APIs app.rs calls exist), dispatch.rs, files.rs, frame_tap.rs, window_config.rs, frame_channel.rs, main.rs, lib.rs, Cargo.toml, tauri.conf.json, capabilities/default.json, tests/single_command.rs, INFRA-BLOCKED.md · `rg -n "last_fatal" apps/cli/src/host.rs` (659/839 set, 998 only clear) + Read host.rs 440-570, 790-1019 (open path, pump bounds) · `rg -n "fn open_source" -A30 apps/cli/src/demux_host.rs` (:362-398) + `rg "std::fs::File::open" crates/input/src/file.rs` (:56) · `rg "timeout" crates/streaming/src/fetch.rs` (TimeoutConfig 5s defaults) · `rg -n "session.close" ui/src --glob '!*.test.*' --glob '!generated/*'` (only demo-transport implements it) · Read ui/src/client/tauri-transport.ts, open-path.ts, frame-channel.ts, components/player/MediaSurface.tsx, screens/player/context.tsx 280-400 · `rg -n "SessionSetFrameChannel" crates/commands/src/catalog/command.rs` (:95 serde rename matches UI wire name) · baseline: `audit/runs/baseline/INDEX.md` #2 (build-desktop-tauri exit 101), CORRECTION v2 (MP4 matrix), `audit/runs/W13-A04-argv-panic.txt` (executed argv panic) |

## 1. One-paragraph verdict

This is the ADR-004 admission wrapper — one command, one lock, one ticker — and the architecture beneath it is real and traced: `lumen_dispatch` (app.rs:144) → `ShellRuntime::dispatch` (shell.rs:153) → `crate::dispatch::lumen_dispatch` (dispatch.rs:211) → `lumen_cli::dispatch::lumen_dispatch` → `CliHost::open_session` (host.rs:890), with the pacer (`pump_session` → `pump_wallclock`, host.rs:523) driving the engine between invokes — every wait in that chain is pump-bounded (≤64, AWAIT_PUMPS host.rs:103/997, confirmed by W13-A09's audit of the same lines). **But the file is dead on arrival: `run()` calls `spawn_session_pump(app.handle().clone())` at line 301 and no such function exists anywhere in the repository — the only defined ticker is `spawn_event_pump` (line 190). That is a guaranteed E0425; the crate cannot compile, the anti-bypass test cannot run, and INFRA-BLOCKED.md's "the crate compiles in its dedicated CI lane" is false at HEAD.** The suspected "GUI file-open hang" therefore has a blunter answer: from this tree there is no GUI to hang. Even on a tree where it compiled, the trace says: the open invoke itself does not hang (bounded pumps; URL opens capped at 5 s by ureq's TimeoutConfig) — but a FIFO/stalled-mount open blocks forever in `std::fs::File::open` (input/file.rs:56) *while holding the one global lock* (F-004), one unplayable file poisons the next open and wedges all later ones with no recovery verb in the UI (F-003), and any panic in a handler (non-UTF-8 argv panics inside the lock, F-002) poisons the shell permanently. What the user would see: a frameless 1280×720 window (decorations:false, ui/dist embedded), an error overlay for refused media, video on a real `<canvas>` (w31.4 — Wave-0's "no canvas" evidence is stale for this tree, MediaSurface.tsx:174-183) that dies silently after one frame-send error (F-005); what the user would hear: **nothing, ever — `open_audio_session` hard-codes the NullBackend (pipeline_host.rs:1598), the same `audio.null-device` posture baseline INDEX.md records.** Standing UNVERIFIED-BUILD: baseline #2 (`cargo build --manifest-path apps/desktop-tauri/Cargo.toml`) died at the gdk/gtk pkg-config stage — before type-checking — which is exactly why the E0425 went unnoticed.

## 2. What actually works (verified)
— The single-command registration is exactly one and is `lumen_dispatch`: read of app.rs:293 `generate_handler![lumen_dispatch]`, sole `generate_handler` site in the crate (rg) — the §7.4 rule the anti-bypass test enforces holds in source.
— The event channel name matches on both sides: app.rs:100 `"lumen://event"` == ui/src/client/tauri-transport.ts:31 `LUMEN_EVENT_CHANNEL = "lumen://event"` (rg both files).
— The frame-channel bind wire name matches end to end: UI sends `cmd: "session.set-frame-channel"` (generated/commands.ts:24) == `#[serde(rename = "session.set-frame-channel")]` (crates/commands/src/catalog/command.rs:95) == the intercepted variant (shell.rs:160) — the bind path is name-correct.
— Every `desktop::` API app.rs calls exists and is baseline-exercised headlessly: `ShellRuntime::{new, dispatch, pump_events, pump_session, take_pending_events, take_frame_channel_binding, take_frame_records, open_path, on_files_dropped}` (shell.rs:126-324), `PathDragDrop` (files.rs:100-113), `parse_window_table`/`build_spec` (window_config.rs:136-146, 191) — verified by direct read; the shell library itself ran green in baseline #3 (`cargo test --workspace` 1681 passed).
— The argv `std::env::args()` panic pattern is EXECUTED proof in the CLI twin: `audit/runs/W13-A04-argv-panic.txt` — `lumen open $'\xff\xfe.mp4'` → exit 101, `called Result::unwrap() on an Err value: "\xFF\xFE.mp4"`.
— Baseline #2 `build-desktop-tauri.log.txt` (exit 101, missing gdk-3.0/gtk pkg-config) — the standing UNVERIFIED-BUILD fact this audit cites instead of building.

## 3. Findings

### F-001 [S0]
Where: apps/desktop-tauri/src/app.rs:301 (call) vs :190 (definition); no other hits repo-wide
Code:
```rust
            apply_window_spec(app.handle())?;
            spawn_session_pump(app.handle().clone());
            open_argv_files(app.handle());
```
What is wrong: `spawn_session_pump` is not defined in this crate, this file, or anywhere in the repository (`rg` over the whole tree: app.rs:301 is the single hit). The ticker defined at line 190 is `fn spawn_event_pump(app: AppHandle)` — the w31.1 rename ("the ticker is also the frame shipper", comment at 179) updated the definition and never the call site. Nothing in scope can supply the name: the file's only imports are app.rs:84-94 (`tauri` items, `commands::*`, `desktop::*`), no glob imports, no macros. This is a guaranteed E0425 at type-check.
Why it matters: the admission crate cannot compile, so there is no `lumen-tauri` binary from this tree: no GUI, no file open, no hang to debug — and every claim built on "the crate compiles" falls with it. INFRA-BLOCKED.md:4-5 asserts "the crate compiles in its dedicated CI lane (`tauri-check`, Linux + webkit2gtk)" — false at HEAD; the last recorded tauri-check run (43400610890, 2026-11-13, docs/packaging/evidence/w26m-t5-launch-verdict.md:160) predates the w31.1 rename and was already RED. Because the lib is a dependency of `tests/single_command.rs`, even the §7.4 anti-bypass test cannot run (`cargo test --manifest-path apps/desktop-tauri/Cargo.toml` fails at compile). The w30.8/w30.9 release payloads INFRA-BLOCKED.md:60-65 says "exec the real shell" must carry older source — the current tree cannot reproduce them.
What correct looks like: the call site reads `spawn_event_pump(app.handle().clone());` (the parameter type already matches: it passes an owned `AppHandle`). A CI lane that compiles this crate per commit would have caught the rename.
Confidence: HIGH — Verification: read + repo-wide rg (deterministic name resolution; no candidate import can supply the name). Runtime compile UNVERIFIED (sandbox has no webkit/gtk dev libs — baseline #2 dies earlier at the gdk-sys build script, which is precisely why this was never reached). REQUEST-ORCH-RUN logged.

### F-002 [S1]
Where: apps/desktop-tauri/src/app.rs:271-283 (panic source :276), lock held from :273
Code:
```rust
    let Ok(mut runtime) = state.0.lock() else {
        return;
    };
    for arg in std::env::args().skip(1) {
        let path = PathBuf::from(arg);
```
What is wrong: `std::env::args()` panics on non-UTF-8 argv (std converts via `unwrap`); the identical pattern is executed proof in the CLI twin (W13-A04: exit 101 on `lumen open $'\xff\xfe.mp4'`). Here the panic strikes inside `setup` *while holding the ShellState mutex* — the poisoning means that even if the panic were survived, every later `lumen_dispatch` answers `ShellError::StatePoisoned` (app.rs:150) and the pump thread exits on poison (app.rs:199-201): a dead shell behind a live window. `PathBuf::from(arg)` one line later shows the author wanted arbitrary paths — `args_os()` is the correct API and is used nowhere in the repo (W13-A04 grep).
Why it matters: `lumen-tauri /data/media/\xff.mp4` (a legal Unix filename; drag-drop/argv is an advertised open seam, app.rs:61-62) kills the app at startup. The GUI variant is UNVERIFIED (cannot build — F-001) but the panic site is std behavior proven live on the twin.
What correct looks like: `std::env::args_os().skip(1)` + `PathBuf::from(OsString)`, and the argv loop moved outside the lock (collect first, then lock once).
Confidence: HIGH — Verification: read + executed twin evidence (W13-A04-argv-panic.txt); GUI run itself UNVERIFIED.

### F-003 [S1]
Where: apps/desktop-tauri/src/app.rs:151 (`runtime.dispatch` — the only engine path the GUI has) — defect lives in the inherited chain: apps/cli/src/host.rs:897-903, 998-1001; UI gap: no `session.close` sender outside demo-transport
Code:
```rust
    let mut runtime = state.0.lock().map_err(|_| ShellError::StatePoisoned)?;
    let response = runtime.dispatch(&request);
```
```rust
        if let Some(error) = self.last_fatal.take() {
            self.teardown_after_fatal();
            return Err(error);
        }
```
What is wrong: the GUI dispatches through the SAME `CliHost` the CLI drives (app.rs:93 → shell.rs:163 → dispatch.rs:212 → lumen_cli host.rs `open_session`), which W13-A09 proved (part2 F-001, S2): a decode fatal that lands after the open response (exactly the 4:4:4-H.264 case: open answers `Accepted`, the `decode.unsupported` fatal arrives later as an event — baseline mp4-smoke-open-session.txt) sets `last_fatal`, which is cleared ONLY by the next `open_session` (host.rs:998) and never by teardown — so the NEXT file's open fails with the PREVIOUS file's error, and the failed teardown orphans the engine session ("a session is already open — close it first", host.rs:897-903). In the CLI that is recoverable by `close`. **In the GUI there is no recovery: `rg "session.close" ui/src` shows the command exists in the catalog (generated/commands.ts:24) and in the browser demo transport (demo-transport.ts:202,417) but no real screen/context ever sends it; app.rs exposes no reset and `ShellState` is built exactly once (app.rs:290).** One unplayable file therefore wedges every subsequent open until the user kills the app.
Why it matters: this is the GUI twin of the brick the wave prompt names, and it is strictly worse than the CLI twin (no close verb). Open bad MP4 → error chip → open good MP4 → same stale error → open anything → "a session is already open". The user-visible core (open a file) is broken after a single failure.
What correct looks like: host-side (per W13-A09): clear `last_fatal` in `teardown_session_scoped` and scope it per session generation; GUI-side: an error-event handler that dispatches `session.close` (or app.rs resetting the runtime on fatal), so the wedge has an exit.
Confidence: HIGH — Verification: read (full chain traced both sides); CLI twin sequence baseline-evidenced (W13-A09 part2 F-001 + mp4-smoke transcript); GUI run UNVERIFIED (F-001).
Severity note: rated S1, one above the CLI twin's S2, because this host removes the only recovery path.

### F-004 [S2]
Where: apps/desktop-tauri/src/app.rs:150-153 (lock held across the whole synchronous open) → host.rs:904 `open_source` → demux_host.rs:394-396 → crates/input/src/file.rs:56
Code:
```rust
    let mut runtime = state.0.lock().map_err(|_| ShellError::StatePoisoned)?;
    let response = runtime.dispatch(&request);
    forward_pending(&app, &mut runtime);
```
```rust
        let file = std::fs::File::open(path).map_err(error::open_io)?;
```
What is wrong: the file-open command performs synchronous, untimeoutable std I/O *under the one global lock*. `std::fs::File::open` on a FIFO (opening a FIFO for read blocks until a writer appears — a FIFO named `video.mp4` passes both the drop classifier files.rs:39-45 and the UI classifier open-path.ts:45-50, which treats every non-playlist suffix as media) blocks forever; a stalled hard-mount (NFS) blocks the probe read indefinitely. While the lock is held: the pacer thread parks at app.rs:199 (no pump, no events, position frozen), the invoke never resolves (UI spinner forever), and any drag-drop during the stall blocks the MAIN/event-loop thread at app.rs:311 — the window stops repainting. URL opens are merely 5-second freezes per op (streaming fetch.rs TimeoutConfig 5/5/5s defaults) — same lock, bounded.
Why it matters: this is the only true *indefinite* hang on the GUI open path (everything in the engine host is pump-bounded — W13-A09 verified the ≤64 bounds), and its blast radius is the whole shell, not one command, because everything shares the one mutex. VLC-class players do source I/O off the UI path; here the open IS the UI path.
What correct looks like: run `open_source`'s I/O outside the lock (or on a worker thread) and take the lock only for engine mutation; or make FileAccess non-blocking/async with a deadline. At minimum, document that a stalled source wedges the shell.
Confidence: HIGH on the mechanism (every hop read; `std::fs::File` semantics are std-documented), MEDIUM on real-world frequency (FIFO/NFS required) — Verification: read; no execution (F-001).

### F-005 [S2]
Where: apps/desktop-tauri/src/app.rs:223-229
Code:
```rust
            if let (Some(channel), Some(record)) = (&frame_channel, records.last()) {
                if let Err(error) = crate::frame_channel::send_record(channel, record) {
                    // lumen-allow-literal: diagnostic text (the ship failure notice), not identity wiring
                    eprintln!("frame channel send failed (dropping until the next bind): {error}");
                    frame_channel = None;
                }
            }
```
What is wrong: one failed `send_record` permanently discards the channel (`frame_channel = None`) and the recovery story is "until the next bind" — but the UI binds exactly ONCE, at surface mount (MediaSurface.tsx:146-152 sends `session.set-frame-channel` in the mount effect and never again; no re-bind logic exists anywhere in ui/src). A single transient send error (busy webview, momentary IPC failure — anything short of a full reload, which is the only thing that re-mounts the surface) freezes video for the rest of the session, silently: the only diagnostic is an `eprintln` that a double-clicked Windows release never shows (`windows_subsystem = "windows"`, main.rs:19 — stderr is a null sink), and no event reaches the UI. Playback (audio-clock, position, events) continues — a playing-but-black picture with zero signal.
Why it matters: the frame channel is the ONLY video path (the `<video>` element is src-less for file sources, MediaSurface.tsx:62-85); killing it on first error with no retry and no UI signal converts any transient IPC hiccup into permanent, undiagnosable video loss.
What correct looks like: retry the next tick with the same channel id (the failure mode frame_channel.rs documents is "the webview went away" — distinguish reload from transient); on repeated failure emit a typed event so the UI can re-bind.
Confidence: HIGH — Verification: read both sides (app.rs + MediaSurface.tsx + frame-channel.ts); runtime UNVERIFIED (F-001).

### F-006 [S3]
Where: apps/desktop-tauri/src/app.rs:248-266 (only 3 of 5 spec fields consumed); product.toml:73-74; window_config.rs:176-180
Code:
```rust
    let spec = config.build_spec();
    if let Some(window) = app.get_webview_window("main") {
```
What is wrong: `build_spec()` produces `inner_size`, `min_inner_size`, `maximized`, `remember_geometry`, `title` (window_config.rs:170-183); `apply_window_spec` applies the first three and silently drops `remember_geometry` and `title`. product.toml:73-74 promises `remember_geometry = true` — "Remember window geometry across runs (position + size, per monitor)" — and nothing in the repo persists geometry (rg `remember_geometry`: parsed in window_config, listed in the spec, no consumer). The title deferral is at least admitted in the module doc (app.rs:74-76); the geometry promise is dropped without a word.
Why it matters: config declared, validated, shipped into the spec, and never read — the "config-driven system" (doc 07) has a dead promise in its first-consumed table. Users resizing/repositioning lose it on every restart.
What correct looks like: either persist/restore geometry at setup (settings-backed store) or mark the key `[PROPOSAL]` in product.toml and skip it in build_spec until it is real.
Confidence: HIGH — Verification: read + rg (no consumer).

### F-007 [S3]
Where: apps/desktop-tauri/src/app.rs:109 vs INFRA-BLOCKED.md:27 ("the 40 ms event pump") vs shell.rs:80-81 (PACER_TICK ≈ 30.6 ms)
Code:
```rust
const SESSION_TICK: Duration = Duration::from_millis(10);
```
What is wrong: three cadence stories coexist: the wrapper hard-codes 10 ms; the admission doc says 40 ms; the shell library documents PACER_TICK (30.625 ms, "one null-device render quantum… the same constant the CLI host renders per advance tick", shell.rs:67-81) as THE cadence seam — and the wrapper never references it (rg: PACER_TICK has no caller in apps/desktop-tauri). The wrapper's own comment says "every 33 ms audio quantum" (app.rs:102) which is the 44.1 kHz quantum, not the 48 kHz one PACER_TICK names (30.6 ms). Behavior is safe (pump_session is wall-bounded, so cadence is granularity only), but the "delegated, not duplicated" claim (shell.rs:243-246) is false at the one seam it names: the cadence decision IS duplicated and has already drifted three ways.
Why it matters: doc-drift and a dead exported constant — exactly the "naming that lies / config declared but never read" pattern; the next maintainer tuning PACER_TICK will change nothing in the shipped shell.
What correct looks like: `const SESSION_TICK: Duration = PACER_TICK.div_duration_from(...)`-style derivation or at minimum a comment binding the two constants; INFRA-BLOCKED corrected to 10 ms.
Confidence: HIGH — Verification: read + rg.

### F-008 [S3]
Where: apps/desktop-tauri/src/app.rs:161-169 (emit under lock), :159-160 (the assumption), :305-319 (main-thread lock site)
Code:
```rust
fn forward_pending(app: &AppHandle, runtime: &mut ShellRuntime) {
    runtime.pump_events();
    for event in runtime.take_pending_events() {
```
```rust
            let state = app.state::<ShellState>();
            let Ok(mut runtime) = state.0.lock() else {
                return;
            };
```
What is wrong: `app.emit` runs while the global mutex is held (from the pacer thread every 10 ms, and from the async command). The module doc asserts this is safe — "an emit is a fast queue-to-webview handoff — worth holding the lock for" (app.rs:158-160) — but that is a comment, not a verified property, and tauri 2.12.5's emit path cannot be audited in this sandbox (no registry source; UNVERIFIED-BUILD). If emit ever needs the main/event-loop thread synchronously (wry's evaluate_script is main-thread on gtk), the interleaving [pacer holds lock → emit waits on main thread] + [main thread in `on_window_event` drop handler waits on lock, :311] is an ABBA deadlock — the entire window freezes, permanently.
Why it matters: this is the classic frameless-shell/IPC hazard the wave's hostile question asks about, and the code's safety argument is one unverified sentence. It is also cheap to make robust.
What correct looks like: drain events under the lock, emit after `drop(runtime)` — the exact pattern the frame shipper already uses two lines later (app.rs:208-213: "take the bind + the drained records under the lock, ship after releasing it"). The event path should follow its own advice.
Confidence: MEDIUM — Verification: read (the interleaving is real and reachable); tauri internals UNVERIFIED (no source available, no build). REQUEST-ORCH-RUN: a webkit-host launch smoke dropping a file during playback would settle it.

### F-009 [S4]
Where: apps/desktop-tauri/src/app.rs:276-282; cross-file note files.rs:51 vs ui/src/client/open-path.ts:44-50
Code:
```rust
    for arg in std::env::args().skip(1) {
        let path = PathBuf::from(arg);
        if path.is_file() {
```
What is wrong: (a) argv is treated as a bare file list — `lumen-tauri --help`, `--version`, any flag: no parsing, no diagnostic, silently skipped by `is_file()`; the CLI twin has a full parser. (b) Classification drift: files.rs:51 claims the Rust rule "Mirrors ui/src/client/open-path.ts exactly", but the drop path whitelists media suffixes (files.rs:39-45) while the UI sends `session.open` for ANY non-playlist path (open-path.ts:44-50 "Anything else is media") — a dropped README is a no-op, the same README through the in-webview input becomes a doomed session.open. Two acquisition sides, two rules.
Why it matters: silent flag-swallowing is user-hostile; the drift means the "one rule, two sides" invariant the docs lean on is false at the edges.
What correct looks like: share one classifier (the UI imports a generated suffix list, or the shell stops pre-filtering), and at minimum log skipped argv entries.
Confidence: HIGH — Verification: read both files.

### F-010 [S4]
Where: apps/desktop-tauri/src/app.rs:166, 219, 226; main.rs:19
Code:
```rust
        if let Err(error) = app.emit(EVENT_CHANNEL, &event) {
            eprintln!("event emit failed on the shell channel: {error}");
        }
```
What is wrong: all runtime failure diagnostics in this file are `eprintln!` — but the release binary is `windows_subsystem = "windows"` (main.rs:19), so a double-clicked Windows GUI has a null stderr (main.rs's own doc: "Rust std maps the null stderr handle to a silent sink"). The three failure faces in main.rs cover STARTUP only; runtime emit/bind/send failures (the shell's lifeline events — tauri-transport.ts:174) vanish. CI greps them from redirected pipes; real users see nothing.
Why it matters: the failure modes most likely to bite a real desktop user (webview event channel dead, frame channel dead) are exactly the ones with no visible face — support cannot triage "black window" reports.
What correct looks like: route runtime failures through the diagnostics crate (the crash-log writer already exists, main.rs:53) or emit a typed event the UI can surface.
Confidence: HIGH — Verification: read app.rs + main.rs doc.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `EVENT_CHANNEL` const | 96-100 | REAL | — | Matches UI constant verbatim (verified §2); literal-allow annotation present |
| `SESSION_TICK` const | 102-109 | REAL | F-007 | 10 ms; duplicates PACER_TICK's decision; comment's "33 ms" is the 44.1 kHz quantum |
| `ShellState` struct | 111-113 | REAL | — | One Mutex, the whole shell's serialization point; poisoned by any handler panic (F-002) |
| `ShellError` enum | 115-123 | REAL | — | Transport-level only; StatePoisoned is the sole variant; serde tag kebab-case |
| `Display for ShellError` | 125-136 | REAL | — | One arm; message tells the user to restart — accurate (poison is terminal) |
| `impl Error for ShellError` | 138 | REAL | — | Empty impl, correct |
| `lumen_dispatch` (tauri command) | 140-154 | REAL | F-003/F-004 | async fn doing blocking engine work under a std Mutex (no await holds the guard — await_holding_lock safe); the ONLY engine path; response+events under one lock per §4.3 |
| `forward_pending` | 156-169 | REAL | F-008 | pump_events + drain + emit under lock; double pump (dispatch already pumped, shell.rs:165) — harmless; eprintln face invisible in GUI release (F-010) |
| `spawn_event_pump` | 171-232 | REAL | F-005 | 10 ms loop; exits permanently on poison (by design); takes bind+records under lock, ships after drop — correct pattern the event half (F-008) doesn't follow; frame shipper drops channel permanently on one send error |
| `spawn_session_pump` (call site) | 301 | **FALSE** | **F-001** | Phantom: called, never defined anywhere in the repo — E0425, crate unbuildable |
| `apply_window_spec` | 234-268 | REAL | F-006 | include_str! bakes product.toml at compile time (runtime config changes invisible — accepted ADR posture); honest-parse via Io-error wrapping; consumes 3/5 spec fields; `f64::from(u32)` sizes type-check vs window_config.rs:172-174 |
| `open_argv_files` | 270-285 | REAL | F-002/F-009 | env::args() panics on non-UTF-8 UNDER the lock; is_file() filter silently skips flags/dirs/FIFOs; forward_pending after loop correct |
| `run` | 287-322 | **FALSE at HEAD** | F-001 | Builder chain correct in shape (manage → one handler → setup → drag-drop → generate_context); the setup closure cannot type-check (phantom call); drag-drop arm (305-320) locks on the main thread (F-008 interleaving) |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| tests/single_command.rs `the_shell_registers_exactly_one_command…` | Only for ITS claim | Source-scans the registration array (strips comments, parses `generate_handler![…]`); removing the registration or adding a second command fails it. It cannot catch non-compiling code — and at HEAD it cannot run at all: the test target links the lib, which fails E0425 (F-001). Honest about its scope (docs say "source-scanning (not compiling)… keeps the test honest on every host") — but "CI compiles it for real" (INFRA-BLOCKED.md:33) is currently false. |
| tests/single_command.rs `the_registration_site_is_unique_in_the_crate` | Only for ITS claim | Counts `generate_handler!` occurrences in app.rs code; mutation-proof for smuggling a second site; blind to everything else (including F-001). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the crate compiles in its dedicated CI lane (`tauri-check`, Linux + webkit2gtk)" | INFRA-BLOCKED.md:4-5 | **FALSE** | F-001: E0425 at app.rs:301; last tauri-check run 43400610890 (2026-11-13) predates the w31.1 rename and was RED (w26m-t5-launch-verdict.md:160-161) |
| "the 40 ms event pump" | INFRA-BLOCKED.md:27 | FALSE | app.rs:109 SESSION_TICK = 10 ms (F-007) |
| "The shell's ticker thread (`spawn_event_pump`) … IS the session thread" | app.rs:41-43 module doc | FALSE at HEAD | run() never reaches any ticker — it calls the phantom `spawn_session_pump` (F-001); the doc names the function the call site forgot |
| "the GUI mounts no `<canvas>` so it displays no video" | Wave-0 evidence (cited by wave-14 prompt) | STALE for this tree | w31.4 added the canvas: MediaSurface.tsx:173-183 mounts `<canvas data-media-surface="frames">`, binds via `session.set-frame-channel` (:146-152), blits LUF1 via putImageData (:198-228). The user still sees nothing — because the app cannot build (F-001), and if built, F-005/F-003 stand ready |
| "media time advances at 1.0× real time … EOS arrives without waiting for the next invoke" | app.rs:46-49 | REAL-UNPROVEN | Mechanism verified by read (pump_session → pump_wallclock, host.rs:523-536, wall-bounded anchor math read; W13-A09 part1 audited the same); never executed in a webview (UNVERIFIED-BUILD, baseline #2) |
| "drag-drop … argv files at startup … the open paths the launch verdicts exercise" | app.rs:57-62 | REAL-UNPROVEN | Both seams exist and route through the real funnel (files.rs:100-113, dispatch.rs:253-261); argv path panics on non-UTF-8 (F-002); launch verdicts remain manual (INFRA-BLOCKED.md:66-77) |
| "apply_window_spec consumes `product.toml [window]` (the REAL config …)" | app.rs:69-72 | SHALLOW | 3 of 5 fields applied; remember_geometry silently dropped, title admitted-deferred (F-006) |
| "an emit is a fast queue-to-webview handoff — worth holding the lock for" | app.rs:158-160 | UNVERIFIED | Tauri 2.12.5 internals unavailable in sandbox; no run ever exercised it (F-008) |
| "capabilities stay `core:default` only, no plugins" | app.rs:78-80 | PARTIALLY FALSE | capabilities/default.json = core:default + FOUR window grants (minimize/toggle_maximize/close/start_dragging, w28 fix 3) — broader than "core:default only"; still no plugins, and consistent with the frameless shell |

## 7. Dead / duplicated / unreachable code
- **`spawn_event_pump` (190-232)**: defined, documented, never called — dead at HEAD *because* its call site was renamed to a phantom (F-001). The inversion of the usual dead-code finding: the definition is live code killed by a broken call.
- **`PACER_TICK` (shell.rs:80)**: exported cadence seam, zero callers in the wrapper (F-007).
- **`ShellRuntime::frame_channel_bound` (shell.rs:210)**: the documented "ticker's cheap guard" — the ticker (app.rs:210-212) uses its own `frame_channel.is_some()` mirror instead; the pub API is unwired from the only consumer its doc names.
- **`ShellRuntime::pick_and_open` + `NativeFileDialog`**: reachable only from tests; app.rs explicitly does not wire the dialog (app.rs:78-82, honest) — `pick_and_open` is UNREACHABLE from the shipped shell.
- **`WindowBuildSpec.title` / `.remember_geometry`**: constructed, never read by the only builder consumer (F-006).

## 8. Dependencies & flags
- Crate is workspace-EXCLUDED (own Cargo.lock, 468 pkgs, tauri 2.12.5 per lock) — buildable only on webkit hosts; the workspace graph stays tauri-free (verified: Cargo.toml deps are exactly `desktop`, `commands` (path), `tauri`, `serde`).
- Inline lints mirror the workspace posture (Cargo.toml:34-47: `unwrap_used`/`expect_used`/`panic`/`todo` denied) — none violated in this file's own code; the argv panic (F-002) is std-internal and invisible to `clippy::panic`.
- `tauri.conf.json`: `decorations: false` (frameless — the UI's custom title bar + the four window grants are the chrome), `withGlobalTauri: false` (the UI binds `__TAURI_INTERNALS__` directly — always injected regardless, per tauri-transport.ts:8-10 — UNVERIFIED at runtime), real CSP with IPC origins, `frontendDist: ../../ui/dist` embedded at compile time (build.rs).
- Feature posture inherited from the engine build: `audio.null-device` (baseline engine-info) — the GUI's only audio device is the null sink (pipeline_host.rs:1598), so "no sound" is by-build, not a defect in this file.

## 9. Open questions for the next agent
1. **W14-A02 (main.rs)**: confirm whether any shipped payload artifact exists that still runs — and from which commit; F-001 makes every "GUI payloads exec the real shell" claim unreproducible from HEAD.
2. **W14-A03 (frame_channel.rs)**: `send_record` clones the record (`record.to_vec()`) per frame — per-tick allocation on the video path; check against the D-1 zero-alloc discipline. Also whether `JavaScriptChannelId::parse` of `__CHANNEL__:<id>` can fail for UI-chosen ids (the shim allocates via transformCallback — collision behavior?).
3. **Desktop/UI agents**: the classification drift (F-009b) — files.rs:51's "mirrors open-path.ts exactly" vs open-path.ts's "anything else is media"; one of the two rules is lying about the other.
4. **Whoever gets a webkit host**: REQUEST-ORCH-RUN items below settle F-001 (trivially) and F-008 (the emit-under-lock deadlock question).
5. W13-A07's present-queue wedge (pipeline_host.rs:1383, I420 hold-retry refused by the renderer) intersects F-005: a wedged presentation queue + a dropped channel = a GUI that plays audio-clock-time over a frozen picture with no diagnostic — worth a combined repro once a build exists.
