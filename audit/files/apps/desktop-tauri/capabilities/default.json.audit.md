# AUDIT: apps/desktop-tauri/capabilities/default.json
| Field | Value |
|---|---|
| Auditor | W14-A08 |
| Date | 2026-11-21 |
| Lines / functions | 13 / 10 (5 top-level keys + 5 permission entries) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | YES by construction (lumen-tauri build embeds it: `build.rs:10-12` `tauri_build::build()`, consumed by `generate_context!` at `app.rs:321`; `lumen-tauri` is the third shipped binary — `scripts/packaging/gate.sh:63-90` BIN_TAURI) — **BUT blocked at HEAD**: the host crate cannot compile (`app.rs:301` calls undefined `spawn_session_pump`; definition is `spawn_event_pump` at `app.rs:190` — E0425). Not reachable from the `lumen` CLI (workspace-excluded crate). Binary itself UNVERIFIED-BUILD (baseline #2). |
| Findings | S0: 0 S1: 1 S2: 0 S3: 4 S4: 1 |
| Commands run | `Read` of the file + `tauri.conf.json`, `Cargo.toml`, `main.rs`, `lib.rs`, `app.rs`, `build.rs`, `INFRA-BLOCKED.md`, `tests/single_command.rs` (registration + seams + claims); `python3 json.load` (validity); `rg "invoke\("` + `rg "__TAURI_INTERNALS__"` over `ui/src` (complete IPC surface: `lumen_dispatch`, 4 window commands, `is_maximized`, event listen/unlisten, channel fetch — nothing else); `sed -n` on every quoted range (verbatim check); `rg "spawn_session_pump\|spawn_event_pump"` repo-wide (no second definition); `rg 'type="file"'` (in-webview open seam); `rg "lumen-tauri" scripts/packaging/` (shipped-bin chain); registry searches of all 5 cargo homes (no tauri/wry sources on disk → ACL contents unverifiable). |

## 1. One-paragraph verdict

The capability file is a genuine, minimal, deny-by-default Tauri v2 ACL for the
single `main` window: `core:default` plus exactly the four window-mutator
grants the frameless banner needs, and every grant maps 1:1 to a real UI
invoke site I traced (`window-controls.ts:70-72`, drag region
`AppShell.tsx:541` → drag.js → `start_dragging`). It is NOT the GUI
file-open blocker: all three open seams (drag-drop `app.rs:305-320`, argv
`app.rs:271-285`, in-webview `<input type="file">` `NewPlaylistDialog.tsx`)
bypass the IPC ACL entirely. But three things keep it REAL-UNPROVEN: (1) its
runtime correctness rests on tauri 2.12.5's `core:default` sub-defaults
(`is_maximized`, `internal_toggle_maximize`, event listen/unlisten, channel
fetch) which cannot be verified anywhere on this machine; (2) no executable
gate covers it — UI tests mock `__TAURI_INTERNALS__`, the CI lane only
compiles, launch verdicts are manual; (3) at HEAD the host crate does not
even compile (`app.rs:301` E0425), so the ACL never takes effect until that
is fixed, and its `description` — the file's own security prose —
misenumerates the IPC surface it governs.

## 2. What actually works (verified)

- JSON is valid and structurally sound — verified by `python3 -c
  "json.load(...)"` → keys `$schema, identifier, description, windows,
  permissions`; permissions exactly `core:default` + 4 window grants.
- `windows: ["main"]` matches the only window — verified:
  `tauri.conf.json:13` `"label": "main"`; `app.rs:249`
  `app.get_webview_window("main")`; no second window is created anywhere.
- The four explicit grants map to live UI call sites — verified:
  `window-controls.ts:70-72` invokes `plugin:window|minimize`,
  `plugin:window|toggle_maximize`, `plugin:window|close`;
  `AppShell.tsx:541` `data-tauri-drag-region="deep"` (tauri drag.js invokes
  `plugin:window|start_dragging`).
- "No plugin permissions" matches the crate graph — verified:
  `apps/desktop-tauri/Cargo.toml:49-62` deps are `desktop`, `commands`,
  `tauri`, `serde` only; no `tauri-plugin-*`; `tauri.conf.json` configures no
  plugins.
- The single-ENGINE-command half of the description is real — verified:
  `app.rs:293` `.invoke_handler(tauri::generate_handler![lumen_dispatch])`
  and `tests/single_command.rs:39-65` fails on any second registration.
- The complete UI IPC surface (non-test, `rg`-exhaustive) is:
  `lumen_dispatch` (`tauri-transport.ts:143`), `plugin:window|{minimize,
  toggle_maximize, close, is_maximized}` + `plugin:event|{listen,unlisten}`
  (`window-controls.ts:70-92`, `tauri-transport.ts:91-107`), and the
  defensive `plugin:__TAURI_CHANNEL__|fetch` (`frame-channel.ts:433-437`).
  Nothing in the UI invokes a command outside this set — the grant list is
  consistent with (and only with) these.
- Nothing verified at runtime: the binary cannot be built here (baseline
  `audit/runs/baseline/INDEX.md` #2, exit 101, missing `gdk-3.0.pc`) —
  standing UNVERIFIED-BUILD.

## 3. Findings

### F-001 [S1] — CROSS-FILE: the capability file's ACL can never take effect at HEAD; the host crate does not compile
Where: apps/desktop-tauri/src/app.rs:301 (vs :190) — cross-file blocker, primary owner W14-A01 (`app.rs` is A01's assignment)
Code:
```
            spawn_session_pump(app.handle().clone());
```
vs the only definition (app.rs:190):
```
fn spawn_event_pump(app: AppHandle) {
```
What is wrong: `spawn_session_pump` is defined nowhere. Repo-wide
`rg "spawn_session_pump|spawn_event_pump"` returns exactly three hits: the
doc comment (:42), the definition of the *other* name (:190), and this call
(:301). No glob imports exist (`app.rs:88,92-94` are explicit lists), so this
is a guaranteed E0425 "cannot find function" — the crate cannot compile from
HEAD source, on any host, webkit or not.
Why it matters: `capabilities/default.json` is a build-time input embedded by
`tauri_build::build()` (`build.rs:10-12`) into `generate_context!()`
(`app.rs:321`). A crate that does not compile ships no binary, so the ACL
governs nothing: every grant, deny-by-default posture, and the whole
frameless title bar are dead code paths until this line is fixed. It also
falsifies `INFRA-BLOCKED.md:4-5` ("the crate compiles in its dedicated CI
lane") at HEAD — see F-004/§6.
What correct looks like: `app.rs:301` calls `spawn_event_pump` (or the
function is renamed and all call sites follow). Recorded here because it
zeroes this file's reachability; the defect and its fix belong to the
`app.rs` audit.
Confidence: HIGH — Verification: read (static name resolution; no build
possible — baseline #2).

### F-002 [S3] — Correctness silently depends on tauri 2.12.5 ACL default-set contents that cannot be verified on this machine
Where: apps/desktop-tauri/capabilities/default.json:7 (`"core:default"`)
Code:
```
  "permissions": [
    "core:default",
```
What is wrong: the UI invokes four more commands that this file does NOT
grant explicitly, betting their permissions ride inside `core:default`:
(1) `plugin:window|is_maximized` (`window-controls.ts:73-76`,
  `AppShell.tsx:439` — the □/restore glyph state),
(2) `plugin:window|internal_toggle_maximize` (tauri drag.js double-press —
  promised at `AppShell.tsx:539-540` "and a double-press / toggles maximize"),
(3) `plugin:event|listen`/`unlisten` (`tauri-transport.ts:91,106`,
  `window-controls.ts:79,90` — the entire engine-event stream),
(4) `plugin:__TAURI_CHANNEL__|fetch` (`frame-channel.ts:433-437` — the video
  frame path's defensive branch).
None of these bets is checkable here: tauri/wry/tao sources exist in none of
the five cargo homes on this machine (searched; no network), so the contents
of `core:window:default`, `core:event:default`, and the channel command's
ACL status are UNVERIFIED. The repo's own assertions
(`window-controls.ts:21-22`, `worklog.md:2051` "verified identifiers against
tauri 2.12.5's permission reference") are session-log claims, not citable
evidence.
Why it matters: if any bet is wrong, the failure is a silent runtime ACL
rejection: dead maximize glyph, dead double-click maximize, or a dead
`lumen://event` stream (which would strand every engine event — the UI's
truth source, `app.rs:64-67`). No automated lane would notice (F-006).
What correct looks like: the four dependent commands either granted
explicitly (self-documenting, immune to tauri default-set drift across the
pinned 2.x line) or the assumption pinned by a test that reads the fetched
tauri crate's permission manifests (REQUEST-ORCH-RUN in the log).
Confidence: HIGH that the dependency exists; MEDIUM that tauri's defaults
cover it (ecosystem-standard, unverifiable here) — Verification: read /
UNVERIFIED (no tauri source on disk).

### F-003 [S3] — The description misenumerates the IPC surface it governs (security prose that a reviewer would trust)
Where: apps/desktop-tauri/capabilities/default.json:4
Code (verbatim substrings of the description string):
```
"Every UI-originated command flows EXCLUSIVELY through the single lumen_dispatch command ... The window grants are the ONLY direct core commands the UI may invoke (plugin:window|minimize / toggle_maximize / close / start_dragging)"
```
What is wrong: three accuracy defects in one key. (a) "EXCLUSIVELY" is
contradicted by the code and by the description's own later sentences: the
UI also invokes `plugin:window|is_maximized` (`window-controls.ts:73-76`),
`plugin:event|listen`/`unlisten` (`tauri-transport.ts:91,106`), and
`plugin:__TAURI_CHANNEL__|fetch` (`frame-channel.ts:433`) directly. (b) "The
window grants are the ONLY direct core commands the UI may invoke" is false
as an enumeration — `is_maximized`, the event commands, and drag.js's
`internal_toggle_maximize` are also direct core commands (the first three
are UI code, not injected runtime). (c) The parenthetical "(w28 fix 3 — the
frameless title bar)" misattributes: per `worklog.md:2050-2051`, the
frameless title bar is fix 2 (`decorations:false`) and fix 3 is the
permission grants; the same sentence later cites "decorations:false, w28 fix
2" correctly, so the file contradicts its own attribution.
Why it matters: the description is the security-posture statement of the
app's entire IPC surface, in the very file that enforces it. A threat model
built on "the ONLY direct core commands are these four" misses the
is_maximized query, the event-channel listen (the engine's whole egress),
and the channel-fetch primitive. Doc lies in security config are how
reviews miss holes.
What correct looks like: "Every UI-originated ENGINE command flows
exclusively through lumen_dispatch; the UI additionally invokes the window
mutators granted below, the read-only window/event getters covered by
core:default, and tauri's channel-fetch for the frame path. No plugin
permissions are granted."
Confidence: HIGH — Verification: read (all counter-examples cited to code).
Severity: S3 (doc-only defect inside a security file; the actual grants are
code-consistent — §2).

### F-004 [S3] — Doc drift: INFRA-BLOCKED.md still describes this file as `core:default` only
Where: apps/desktop-tauri/INFRA-BLOCKED.md:39-40 (claim about this file)
Code:
```
- **Capabilities**: unchanged `core:default` only (no plugins — the
  4.H1 posture).
```
What is wrong: the capability file has carried four window grants since w28
fix 3 (`worklog.md:2051`); "unchanged `core:default` only" describes the
pre-w28 state. Anyone auditing the IPC surface via the admission handoff doc
gets the wrong (smaller) surface.
Why it matters: INFRA-BLOCKED.md is the crate's own status ledger (§A "DONE
at admission"); a stale capability line in the security-relevant section
understates the granted surface exactly where a reviewer looks first.
What correct looks like: "Capabilities: `core:default` + the four w28 window
grants (minimize/toggle_maximize/close/start_dragging); no plugins."
Confidence: HIGH — Verification: read (file permissions list at
`default.json:6-12`).
(Ownership note: the INFRA-BLOCKED.md file itself belongs to another agent's
list if assigned; recorded here as a claim touching this file.)

### F-005 [S4] — `$schema` points into a gitignored, build-generated tree
Where: apps/desktop-tauri/capabilities/default.json:2
Code:
```
  "$schema": "../gen/schemas/desktop-schema.json",
```
What is wrong: `apps/desktop-tauri/gen/` is gitignored (`.gitignore:118`
`apps/desktop-tauri/gen/`), so in any fresh clone the `$schema` target does
not exist — editor validation/autocomplete for this file dangles until the
first tauri-build run generates it. This is the stock Tauri template
posture, not a repo invention.
Why it matters: cosmetic; no build or runtime effect (tauri-build does not
read `$schema`).
What correct looks like: acceptable as-is (or commit the generated schema
if editor support matters).
Confidence: HIGH — Verification: read + `.gitignore:118`.

### F-006 [S3] — Zero executable coverage: no test, lane, or gate anywhere validates that the grants match what the UI invokes
Where: apps/desktop-tauri/capabilities/default.json:6-12 (the contract) vs the gates
Code (the seam that mocks the ACL away — `ui/src/client/window-controls.test.ts:19-26`):
```
  const internals = {
    invoke: (command: string, payload?: unknown): Promise<unknown> => {
      invokes.push({ command, payload });
      if (command === "plugin:window|is_maximized") {
        return Promise.resolve(options.maximized === true);
```
What is wrong: the capability↔UI contract has no enforcement anywhere:
- UI unit tests (`window-controls.test.ts` 6 tests, `tauri-transport.test.ts`
  12 tests) fake `__TAURI_INTERNALS__` — a removed or wrong grant changes
  nothing they can observe; they would stay green with the grants deleted.
- `tests/single_command.rs` scans `app.rs` registration only; it never reads
  this file.
- The `tauri-check` CI lane compiles — tauri-build validates identifiers
  (a typo in a grant breaks the build) but NOT that the granted set covers
  the UI's invoke sites; and per F-001 the compile claim is stale at HEAD.
- The launch smokes' deep verdicts (window/file/playback/settings), the only
  tests that would exercise the ACL for real, "remain manual"
  (`INFRA-BLOCKED.md` §B.2).
Why it matters: the failure mode of a missing grant is a runtime IPC
rejection visible only as a console error in the shipped GUI (dead title-bar
button, dead drag, dead event stream). Every layer that could catch it is
either mocked, compile-only, or manual.
What correct looks like: a CI check that extracts the invoke commands from
`ui/src` (they are statically greppable — §2) and diffs them against the
granted permission set, failing on any invoke with no covering grant.
Confidence: HIGH — Verification: read (all four gate layers cited).

## 4. Per-function table (config keys)

| Key / entry | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `$schema` | 2 | REAL | S4 | Points into gitignored `gen/` (F-005); editor-only; tauri-build regenerates. |
| `identifier` ("default") | 3 | REAL | — | Single capability; activation relies on tauri-build's default discovery of `capabilities/` (build.rs:3-8 comment) because `tauri.conf.json` has no `app.security.capabilities` key — mechanism UNVERIFIED (no tauri source; see §9). |
| `description` | 4 | SHALLOW (as documentation) | S3 | Three accuracy defects (F-003); the factual core (single engine command, no plugins, decorations:false) is real (§2). |
| `windows` (["main"]) | 5 | REAL | — | Matches `tauri.conf.json:13` label and `app.rs:249`; the only window; stricter than the omit-defaults form. |
| `permissions[0]` `core:default` | 7 | REAL-UNPROVEN | S3 | Aggregate default whose sub-sets must cover `is_maximized`, `internal_toggle_maximize`, event listen/unlisten, channel fetch (F-002); unverifiable here. |
| `permissions[1]` `core:window:allow-minimize` | 8 | REAL | — | Live call site `window-controls.ts:70`; consumed by `AppShell.tsx:465`. |
| `permissions[2]` `core:window:allow-toggle-maximize` | 9 | REAL | — | Live call site `window-controls.ts:71`; consumed by `AppShell.tsx:470`. |
| `permissions[3]` `core:window:allow-close` | 10 | REAL | — | Live call site `window-controls.ts:72`; consumed by `AppShell.tsx:475`. |
| `permissions[4]` `core:window:allow-start-dragging` | 11 | REAL-UNPROVEN | S3 | Consumed not by UI code but by tauri's injected drag.js on `AppShell.tsx:541` `data-tauri-drag-region="deep"`; the grant is right, but whether stock drag.js honors a non-empty attribute value ("deep", "drag anywhere non-interactive") is unverifiable here — if it checks only the bare attribute on `event.target`, deep-drag silently no-ops (UX defect outside this file). |
| (whole file) | 1-13 | REAL-UNPROVEN | S1 | Embedded at compile time by a crate that cannot compile at HEAD (F-001); runtime effect unobserved anywhere. |

## 5. Test honesty (tests touching this file's contract)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `ui/src/client/window-controls.test.ts` (6 tests) | NO (for capability defects) | Fake `__TAURI_INTERNALS__` (`:19-39`) stubs `invoke`; deleting any grant from this file leaves them green. They do honestly test command names/labels against the mock. |
| `ui/src/client/tauri-transport.test.ts` (12 tests) | NO (for capability defects) | Same mock posture; the `lumen_dispatch` and `plugin:event|listen` shapes are tested against fakes, never the ACL. |
| `apps/desktop-tauri/tests/single_command.rs` (2 tests) | YES (for its own subject) | Source-scans `app.rs`'s `generate_handler!` array and fails on any second entry or second site (mutation: add `foo` to the array → `assert_eq!` fails). Honest — but it never reads this file. |
| Any test reading `capabilities/default.json` | — | Does not exist (rg over the repo: the only non-audit readers of the file are tauri-build at compile time and editors). |

## 6. Doc claims touching this file

| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every UI-originated command flows EXCLUSIVELY through the single lumen_dispatch command" | default.json:4 (description) | FALSE as worded | `window-controls.ts:70-76,79-92`, `tauri-transport.ts:91-107`, `frame-channel.ts:433-437` are UI-originated commands outside `lumen_dispatch` (the single-ENGINE-command rule itself is REAL: `app.rs:293` + `single_command.rs`). |
| "engine events arrive on the lumen://event channel, covered by core:event:default" | default.json:4 | REAL-UNPROVEN | Channel real: `app.rs:100` `EVENT_CHANNEL = "lumen://event"`, emit `app.rs:165`; UI listen `tauri-transport.ts:91`. The "covered by core:event:default" half is unverifiable (F-002). |
| "The window grants are the ONLY direct core commands the UI may invoke" | default.json:4 | FALSE | `plugin:window|is_maximized`, `plugin:event|listen/unlisten`, `plugin:__TAURI_CHANNEL__|fetch`, drag.js `internal_toggle_maximize` are also direct core commands (F-003). |
| "decorations:false, w28 fix 2" (banner IS the chrome) | default.json:4 | REAL | `tauri.conf.json:19` `"decorations": false`; `AppShell.tsx:541` drag-region banner; `worklog.md:2050` fix 2. |
| "No plugin permissions are granted" | default.json:4 | REAL | `default.json:6-12` has no `plugin:*` entry; `Cargo.toml:49-62` has no `tauri-plugin-*` dep; `tauri.conf.json` configures no plugins. |
| "Capabilities: unchanged `core:default` only (no plugins)" | INFRA-BLOCKED.md:39-40 | FALSE | `default.json:8-11` carries four window grants since w28 fix 3 (`worklog.md:2051`) — F-004. |
| "the crate compiles in its dedicated CI lane" | INFRA-BLOCKED.md:4-5 | FALSE at HEAD | Static E0425: `app.rs:301` calls undefined `spawn_session_pump` (F-001); no definition anywhere (repo-wide rg). |
| "core:window:default does NOT include them [the four]" / "verified identifiers against tauri 2.12.5's permission reference" | worklog.md:2051 | REAL-UNPROVEN | Session-log claim; no tauri 2.12.5 source exists on this machine to check (five cargo homes searched); consistent with the explicit grants' existence but uncitable as evidence. |
| "is_maximized and the tauri://resize listen are covered by core:window:default / core:event:default inside core:default" | window-controls.ts:21-22 | REAL-UNPROVEN | Same unverifiable tauri default-set contents (F-002). |
| "tauri.conf.json and capabilities/ ... default discovery finds them" | build.rs:3-6 (comment) | REAL-UNPROVEN | tauri-build behavior unverifiable here; note `tauri.conf.json` sets no `app.security.capabilities`, so this file's activation depends entirely on that default. |

## 7. Dead / duplicated / unreachable code

- No dead or duplicated entries: all five permissions correspond to live UI
  call sites (§2, per-key table); no second capability file exists
  (`capabilities/` holds only `default.json`).
- The file as a whole is UNREACHABLE at HEAD via F-001 (host crate E0425);
  reachable by construction once that is fixed, and only via the
  `lumen-tauri` binary — the `lumen` CLI has no edge to this crate
  (workspace-excluded, ADR-004, `Cargo.toml:1-22`).

## 8. Dependencies & flags

- Governing dependency: `tauri = "2"` pinned to **2.12.5**
  (`apps/desktop-tauri/Cargo.lock:3688-3689`) in the subcrate's own lockfile
  (workspace-excluded per ADR-004). The ACL semantics this file relies on are
  that exact version's — no local copy exists to audit against.
- `tauri-build = "2"` (`Cargo.toml:68`) embeds this file at compile time
  (`build.rs:10-12`); unknown permission identifiers would fail that build —
  the only automated validation this file ever gets.
- No feature flags touch the capability file; `tauri.conf.json`
  `withGlobalTauri: false` is consistent with the UI's raw
  `__TAURI_INTERNALS__` binding posture (no `@tauri-apps/api` in
  `ui/package.json` — rg found no tauri dep).
- Crate lints (`Cargo.toml:34-47`) mirror the workspace policy inline; no
  `#[allow]` in this domain's shell code affects the file.

## 9. Open questions for the next agent

1. **W14-A01 (app.rs):** confirm and own F-001 (`spawn_session_pump` E0425
   at `app.rs:301` vs `spawn_event_pump` at `:190`) — it is the gate for this
   file's entire effect and for INFRA-BLOCKED's compile claim.
2. **Orchestrator (REQUEST-ORCH-RUN, logged):** `cargo fetch --locked
   --manifest-path apps/desktop-tauri/Cargo.toml` + inspection of
   tauri-2.12.5's `permissions/` and `ipc/channel.rs` — settles F-002's four
   dependent commands (`is_maximized`, `internal_toggle_maximize`, event
   listen/unlisten, `__TAURI_CHANNEL__|fetch` exemption) with real source
   instead of ecosystem memory.
3. Does tauri-build's default capability discovery apply when
   `app.security.capabilities` is absent (this repo's situation)? If not,
   this file is inert even in a compiling build — same fetch answers it.
4. Does stock tauri 2.12.5 drag.js honor `data-tauri-drag-region="deep"`
   (non-empty value, ancestor walk) as `AppShell.tsx:536-541` promises, or
   only the bare attribute on `event.target`? If the latter, the "drag
   anywhere non-interactive" UX is a silent no-op — a UI-domain finding, not
   a capability gap.
5. Whoever audits `ui/src/client/frame-channel.ts` / `window-controls.ts`:
   the mocked-test blind spot (F-006) is shared across the whole seam.
