# AUDIT: apps/desktop-tauri/src/app.rs
| Field | Value |
|---|---|
| Auditor | W14-A02 |
| Date | 2026-11-24 |
| Lines / functions | 322 / 8 (6 fns + 2 impls) + 2 consts, 2 types |
| File verdict | UNREACHABLE (real implementation, fatally compile-broken: E0425) |
| Reachable from shipped entry point? | NO — sole chain main.rs→`lumen_desktop_tauri::app::run()` is unbuildable (F-001); crate is workspace-excluded (root Cargo.toml:52) so the green release lane never compiles it |
| Findings | S0: 2 S1: 0 S2: 2 S3: 3 S4: 6 |
| Commands run | Read(full); rg spawn_{event,session}_pump repo-wide; rg/od byte-verify (ci.yml triggers, single_command.rs, serde renames); sed quote-verify ×12; rg API cross-check (shell.rs, window_config.rs, frame_channel.rs, commands, ui/); NO cargo (gdk-sys absent + cargo-build-budget) |

## 1. One-paragraph verdict
The composition root of the entire desktop GUI is a one-character-class
compile error away from working and does NOT compile:
`app.rs:301` calls `spawn_session_pump`, which exists nowhere; the defined
ticker is `spawn_event_pump` (app.rs:190). Name resolution is host-independent
(no glob imports at :84-94, no `cfg`, `pub mod app` unconditional in lib.rs:7),
so every build of this crate fails with E0425 on ANY host — Linux+webkit2gtk,
macOS, Windows alike. The sandbox's gdk-sys failure
(audit/runs/baseline/cargo-build-desktop-tauri.txt:310) stops compilation
before the leaf crate's sources are ever type-checked, masking the break and
letting the baseline read as "environment limit". The rest of the file is a
genuine, coherently wired wrapper (single command, one lock, frame channel,
window spec) — but it is dead on arrival, and two docs claim the opposite.

## 2. What actually works (verified)
— Static API cross-check by rg: every symbol app.rs calls exists with
matching signatures: `ShellRuntime::{new(NonZeroUsize), dispatch(&WireRequest),
pump_session, pump_events, take_pending_events, take_frame_channel_binding->
Option<u32>, take_frame_records(&mut Vec<Vec<u8>>), open_path, on_files_dropped}`
(apps/desktop/src/shell.rs:126,153,202,219,229,279,286,300,308);
`parse_window_table`/`build_spec`/`WindowBuildSpec{inner_size,min_inner_size,
maximized}` (window_config.rs:136-146,170-183);
`frame_channel(&AppHandle<Wry>, u32)`/`send_record(&FrameChannel,&[u8])`
(frame_channel.rs:39,55).
— IPC mirror byte-verified: `EVENT_CHANNEL = "lumen://event"` (app.rs:100) ==
ui/src/client/tauri-transport.ts `LUMEN_EVENT_CHANNEL = "lumen://event"`;
single command `lumen_dispatch` mirrored (`LUMEN_DISPATCH_COMMAND`); frame
bind command serde name byte-verified `"session.set-frame-channel"`
(crates/commands/src/catalog/command.rs:95, ids.rs:100 — od -c).
— ci.yml trigger byte-verified by od -c: `branches: [main]` on
push/pull_request — my first display read "ain]" (ANSI-eating artifact, same
class as the repo's own TRIGGER NOTE); tauri-check has no `if:` gate.
— tests/single_command.rs is syntactically sound (od -c showed real bytes
`source[handler..]`; my display ate `[h`) and is an honest source-scan
anti-bypass test. Nothing executed: no build possible in sandbox.

## 3. Findings
F-001 [S0]
Where: apps/desktop-tauri/src/app.rs:300-302
Code:
```
            apply_window_spec(app.handle())?;
            spawn_session_pump(app.handle().clone());
            open_argv_files(app.handle());
```
What is wrong: `spawn_session_pump` has no definition in the repository.
The ticker is `fn spawn_event_pump(app: AppHandle)` (app.rs:190; repo-wide rg
for `spawn_session_pump` matches ONLY this call site). Imports (:84-94) are
all explicit — no glob can resolve it; no `#[cfg]` guards the call; lib.rs:7
`pub mod app;` is unconditional. E0425 fires on every host that compiles the
crate: ci.yml tauri-check (`cargo test --manifest-path
apps/desktop-tauri/Cargo.toml --locked --all-targets`, ci.yml:731 — runs on
every push/PR to main), tauri-launch.yml (manual verdict lane), and
release.yml:219 `cargo build --release --locked --manifest-path
apps/desktop-tauri/Cargo.toml` (the payload build). Can `cargo build`
succeed anywhere with this typo? NO. Why it matters: this file is the ONLY
entry point that constructs ShellRuntime, drives pump_session, and ships LUF1
frames — the whole "GUI plays video" story. What correct looks like: rename
the call to `spawn_event_pump` (one line) and re-run tauri-check.
Confidence: HIGH — Verification: read + rg (static proof; cargo impossible
here: gdk-sys absent, baseline :310, cargo-build-budget).

F-002 [S0]
Where: apps/desktop-tauri/INFRA-BLOCKED.md:4-5
Code:
```
> anti-bypass audit exists, the crate compiles in its dedicated CI lane
> (`tauri-check`, Linux + webkit2gtk), and the 3-OS bundling + launch
```
What is wrong: present-tense claim, FALSE against the current tree (F-001 —
the lane's first cargo step cannot pass). Same doc §B.1 marks "the webview
app ships in every release payload … DONE (w30.8/w30.9)" — also impossible
from this tree: release.yml:219 builds the same broken crate. Either the
docs are stale or the lane/payload builds are red; both cannot be green.
Why it matters: this is the admission record's status line — the trust anchor
other docs cite. What correct looks like: revert §A/§B.1 to blocked, or fix
F-001 and show a green run id.
Confidence: HIGH — Verification: read + rg (release.yml:219, ci.yml:731).

F-003 [S2]
Where: apps/desktop-tauri/src/app.rs:197-201 (doc claim :188-189)
Code:
```
            std::thread::sleep(SESSION_TICK);
            let state = app.state::<ShellState>();
            let Ok(mut runtime) = state.0.lock() else {
                return;
            };
```
What is wrong: one panic in ANY handler (lumen_dispatch, drag-drop) poisons
the mutex; the ticker then `return`s — the session thread dies permanently
with NO diagnostic (contrast: every other failure face in this crate prints;
main.rs installs a panic hook, but the hook fires at poison time, not at
pump death). The doc's justification is false: "Stops silently when the
state is poisoned (the app is dying anyway)" — tauri/tokio CATCH handler
panics; the process typically keeps running with a live window and a frozen,
mute session (no playback, no time-ticks, forever). Same silent-return
pattern at :273-275 and :311-313 (lower stakes). Why it matters: the pacer
is the only thing that drives the engine between invokes; its death is the
app's functional death, invisible to the user. What correct looks like: log
+ surface a ShellError event / exit rather than silent return.
Confidence: MEDIUM — Verification: read (tauri panic-catch semantics from
knowledge; no execution possible).

F-004 [S2]
Where: apps/desktop-tauri/src/app.rs:276-284
Code:
```
    for arg in std::env::args().skip(1) {
        let path = PathBuf::from(arg);
        if path.is_file() {
            // Receipt observed for honesty; the UI's truth is the event
            // stream (§5.1: responses and events never share a channel).
            let _receipt = runtime.open_path(&path);
        }
    }
    forward_pending(app, &mut runtime);
```
What is wrong: this runs inside the tauri `setup` hook, i.e. before the
webview's JS has registered its `lumen://event` listener; tauri `emit`
delivers to registered listeners only (no replay), and the response receipt
is discarded. The argv-opened file's playlist/session events are therefore
emitted into the void — the file's own doc (:64-67) says "the UI's truth
source is the event stream", and that truth is lost for the startup path.
The launch lane's assertion (no `event emit failed` receipts) passes even
when every event is lost — emit to zero listeners is not an error. Whether
the UI re-syncs state after load is UNVERIFIED (out of this file; no
initial-state query found in the transport's listen path I read).
Why it matters: `lumen-tauri file.mkv` is the CI launch lane's own fixture
path and the primary UX path; "file opens at startup" is exactly what it
must guarantee. What correct looks like: buffer events until the UI signals
readiness, or a state-sync command on UI boot.
Confidence: MEDIUM — Verification: read + rg (tauri event semantics from
knowledge).

F-005 [S3]
Where: apps/desktop-tauri/src/app.rs:196-211
Code:
```
        loop {
            std::thread::sleep(SESSION_TICK);
            ...
            let bind = runtime.take_frame_channel_binding();
            let mut records = Vec::new();
            if frame_channel.is_some() {
                runtime.take_frame_records(&mut records);
            }
```
What is wrong: (a) the ticker wakes at 100 Hz for the process's entire
lifetime — idle, paused, no session: lock + pump + `Vec::new()` + drain
checks every 10 ms forever (documented as "a cheap no-op poll"; it is still
a permanent 100 Hz wake source for a media player that is usually idle).
(b) A fresh bind takes effect one tick late: records are drained only when
the OLD channel exists, so the first frame after a (re)bind waits ≥10 ms.
(c) `Vec::new()` per tick, and only `records.last()` (:223) is ever used —
the drained older records are allocated then discarded (bounded, but
needless per-tick churn). What correct looks like: park the ticker when no
session is active (state-driven wake), reuse the Vec, drain on bind too.
Confidence: HIGH — Verification: read.

F-006 [S3]
Where: apps/desktop-tauri/src/app.rs:249-262 vs tauri.conf.json vs product.toml:63-65
Code (conf):
```
        "label": "main",
        "title": "Lumen",
        "width": 1280,
        "height": 800,
```
Code (product.toml): `width = 1280` / `height = 720`.
What is wrong: the first-paint window is 1280×800; `apply_window_spec` then
re-asserts 1280×720 at setup — a guaranteed visible one-time height jump on
EVERY launch. The doc (:71-72) calls the conf values "fallbacks only", but a
fallback that disagrees with the real spec by 80 px is a flash, not a
fallback. Also: if the conf's window label ever changes from `"main"`,
`get_webview_window("main")` (and frame_channel.rs:45) silently returns None
— sizes unapplied, frame bind fails, no diagnostic at this site. What
correct looks like: conf height = 720 (match the spec), and a loud error
when the expected window is absent.
Confidence: HIGH — Verification: read (conf + product.toml byte-verified).

F-007 [S3]
Where: apps/desktop/src/window_config.rs:132-134, :141 vs apps/desktop-tauri/src/app.rs:248-267
Code (window_config.rs):
```
    /// set the ADR-004 wrapper feeds `WebviewWindowBuilder` (issue #3,
    /// 4.H1 handoff c: the `[window]` table is CONSUMED at window
    /// build, not just parsed). `remember_geometry` rides along so the
    /// wrapper branches on config (restore persisted geometry vs
    /// apply `inner_size`), never on a guess.
```
What is wrong: the wrapper does NOT branch. `apply_window_spec` consumes only
`inner_size`, `min_inner_size`, `maximized`; `WindowBuildSpec.remember_geometry`
(:180) and `title` (:182) are parsed, carried, and dropped. No geometry
persistence exists anywhere in the tree (rg `remember_geometry`: parse +
spec + a UI settings schema row `defaultRef: "window.remember_geometry"`
(ui/src/screens/settings/generated/schema.ts:411) — a user-facing settings
toggle wired to nothing on the shell side). product.toml's
"Remember window geometry across runs (position + size, per monitor)" is a
promise no shipped code keeps; the title_template/title_idle_key keys are
equally unapplied here (admitted at app.rs:74-76). What correct looks like:
either implement the store or stop surfacing the toggle.
Confidence: HIGH — Verification: read + rg.

F-008 [S4]
Where: apps/desktop-tauri/src/app.rs:243
Code:
```
    let config = parse_window_table(include_str!("../../../product.toml")).map_err(|error| {
```
What is wrong: the "REAL config" (doc :71-72) is baked at COMPILE time;
a shipped binary never re-reads product.toml, so the settings-UI → config
story (F-007) is structurally impossible even if persistence landed.
Confidence: HIGH — Verification: read.

F-009 [S4]
Where: apps/desktop-tauri/src/app.rs:150-152, 161-168
Code:
```
    let mut runtime = state.0.lock().map_err(|_| ShellError::StatePoisoned)?;
    let response = runtime.dispatch(&request);
    forward_pending(&app, &mut runtime);
```
What is wrong: `lumen_dispatch` is `async fn` — tauri runs it on its async
runtime — yet it takes a blocking `std::sync::Mutex` and holds it across the
full dispatch (open/demux can be slow) AND across every `app.emit` in
`forward_pending` (emit-under-lock is documented as deliberate, :158-160).
The pump thread needs the same lock every 10 ms. No await is held across the
lock (so clippy's await_holding_lock cannot see it); worst case is executor
thread starvation under a slow open, not a deadlock. What correct looks like:
`blocking_lock` off the async path, or emit after unlock accepting the
documented ordering trade-off.
Confidence: MEDIUM — Verification: read.

F-010 [S4]
Where: apps/desktop-tauri/src/app.rs:276-281
Code:
```
    for arg in std::env::args().skip(1) {
```
What is wrong: `std::env::args()` PANICS on non-UTF-8 argv (args_os is the
correct API; a panic here kills startup pre-window) and every argv token
that happens to name an existing file is opened as media — a flag like
`--log` that names a file gets dispatched. Receipts are discarded
(`let _receipt`) by design, but combined with F-004 the argv path has no
working feedback at all. Confidence: HIGH — Verification: read.

F-011 [S4]
Where: apps/desktop-tauri/INFRA-BLOCKED.md §A ("the 40 ms event pump");
.github/workflows/tauri-launch.yml:19,189; docs/packaging/evidence/w26m-t5-launch-verdict.md:41
Code (app.rs:109):
```
const SESSION_TICK: Duration = Duration::from_millis(10);
```
What is wrong: three prose/workflow sources say "40 ms pump"; the code ticks
at 10 ms. The launch-lane greps only read stderr receipts, so nothing fails
— the cadence in the evidence narrative is simply wrong for this tree.
Confidence: HIGH — Verification: read + rg ("40 ms" hits above).

F-012 [S4]
Where: apps/desktop-tauri/INFRA-BLOCKED.md §A ("Capabilities: unchanged
`core:default` only (no plugins — the 4.H1 posture)") vs
apps/desktop-tauri/capabilities/default.json
Code (default.json permissions):
```
    "core:default",
    "core:window:allow-minimize",
    "core:window:allow-toggle-maximize",
    "core:window:allow-close",
    "core:window:allow-start-dragging"
```
What is wrong: the capability set grew (w28 fix 3, frameless title bar) and
the admission record still says "core:default only". The JSON's own
description documents the grants; the §A list is stale. Still no plugins —
that half holds. Confidence: HIGH — Verification: read.

F-013 [S4]
Where: apps/desktop/src/shell.rs:210-212 vs apps/desktop-tauri/src/app.rs:210
Code (shell.rs):
```
    pub fn frame_channel_bound(&self) -> bool {
```
What is wrong: the API built as "the ticker's cheap guard" is never called
by the ticker (app.rs guards on its local `frame_channel.is_some()` instead);
its only caller is a shell.rs test (:667). Production-dead API shipped to
support a call pattern that was then not used. Confidence: HIGH —
Verification: rg (`frame_channel_bound` → shell.rs:210 def, :667 test only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| const EVENT_CHANNEL | 96-100 | REAL | — | byte-mirrored in tauri-transport.ts |
| const SESSION_TICK | 102-109 | REAL | S3→F-005 | 10 ms; docs say 40 ms (F-011) |
| struct ShellState | 111-113 | REAL | — | one Mutex<ShellRuntime>, the serialization point |
| enum ShellError + Display + Error | 115-138 | REAL | — | single variant StatePoisoned; serde-tagged |
| lumen_dispatch | 140-154 | REAL-UNPROVEN | S4→F-009 | the ONE command; blocks async runtime on std Mutex; unreachable until F-001 fixed |
| forward_pending | 156-169 | REAL | S4→F-009 | emits under lock; errors only eprintln'd |
| spawn_event_pump | 171-232 | REAL-UNPROVEN | S2→F-003, S3→F-005 | 10 ms ticker; frame bind+ship outside lock; dies silently on poison; CALLED BY NOTHING (F-001) |
| apply_window_spec | 234-268 | REAL | S3→F-006/007/008 | honest-parse (no invented window) ✓; ignores remember_geometry/title; compile-time-baked config; silent on missing "main" |
| open_argv_files | 270-285 | REAL-UNPROVEN | S2→F-004 | events emitted before UI listener exist; args() panic on non-UTF-8 |
| run | 287-322 | UNREACHABLE | S0→F-001 | the admission entry; compile-broken at :301; drag-drop seam + one handler registration coherent |

## 5. Test honesty
No tests in this file. Crate test tests/single_command.rs (anti-bypass,
§7.4): honest source-scan — it strips comments and would FAIL if a second
command were registered (mutation reasoning: changing
`generate_handler![lumen_dispatch]` to two entries breaks the
`assert_eq!(registered, vec!["lumen_dispatch"])`). It cannot run: the crate
does not compile (F-001) — `cargo test --all-targets` dies at the lib first.
The test proves nothing about the CURRENT tree's behavior.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the crate compiles in its dedicated CI lane (tauri-check…)" | INFRA-BLOCKED.md:4-5 | FALSE | E0425 (F-001); ci.yml:731 compiles the crate unconditionally |
| "the webview app ships in every release payload … DONE" | INFRA-BLOCKED.md §B.1 | FALSE (this tree) | release.yml:219 builds the same broken crate |
| "the 40 ms event pump" | INFRA-BLOCKED.md §A; tauri-launch.yml:19,189; w26m-t5 evidence:41 | FALSE | SESSION_TICK=10 ms (app.rs:109) |
| "Capabilities: unchanged core:default only" | INFRA-BLOCKED.md §A | FALSE | capabilities/default.json: 4 window grants |
| "the wrapper branches on config (restore persisted geometry vs apply inner_size)" | window_config.rs:132-134 | FALSE | apply_window_spec (:248-267) never reads remember_geometry (F-007) |
| "the UI's truth source is the event stream" | app.rs:64-67 | SHALLOW | startup-path events emitted before any listener (F-004) |
| "Stops silently when the state is poisoned (the app is dying anyway)" | app.rs:188-189 | FALSE | tauri catches handler panics; app survives with dead pacer (F-003) |
| EVENT_CHANNEL mirrored in ui/src/client/tauri-transport.ts | app.rs:98-100 | REAL | byte-verified both sides |
| "one-frame-per-tick contract" (records.last()) | app.rs:182-187 vs :223 | REAL | matches shell.rs:214-218 docs |

## 7. Dead / duplicated / unreachable code
- The ENTIRE FILE is unreachable as shipped: the binary cannot be built
  (F-001) and the workspace release lane never compiles the crate
  (root Cargo.toml:52 `exclude = ["apps/desktop-tauri"]`).
- `ShellRuntime::frame_channel_bound` (shell.rs:210) — production-dead guard
  API (F-013).
- `WindowBuildSpec.{remember_geometry, title}` — parsed, carried, never
  consumed by the only production consumer (F-007).

## 8. Dependencies & flags
Crate graph (apps/desktop-tauri/Cargo.toml): desktop (path), commands (path),
tauri = "2" (registry, ADR-004 E1 exception), serde; build-dep tauri-build.
No features defined — nothing is feature-gated; the E0425 cannot be escaped
by any flag combination. Workspace-excluded (own Cargo.lock), lints mirrored
inline (unsafe_code deny; main.rs win32 module is the sole cfg(windows)
unsafe exception, not in this file). Edition 2024.

## 9. Open questions for the next agent
1. WHEN did F-001 land? If tauri-check is merge-blocking on every push to
   main (byte-verified trigger), the lane must be red since the typo's
   commit — check CI history (needs git/gh; banned here → UNVERIFIED) and
   whether the w31.1 frame-channel push ever ran green.
2. Does the UI re-sync state after load (F-004)? Read
   ui/src/client/tauri-transport.ts listen path + app boot for an initial
   state query; if absent, `lumen-tauri file.mkv` opens headless-blind.
3. tauri-launch.yml deep verdicts are workflow_dispatch-manual — did any
   recorded run postdate the typo? (evidence doc cites runs; comment-only,
   untrusted.)
