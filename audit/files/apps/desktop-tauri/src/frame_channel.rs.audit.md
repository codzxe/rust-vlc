# AUDIT: apps/desktop-tauri/src/frame_channel.rs
| Field | Value |
|---|---|
| Auditor | W14-A03 |
| Date | 2026-11-21 |
| Lines / functions | 57 / 3 (1 type alias, 2 functions) |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO (as committed). Intended chain: `lumen-tauri` bin (main.rs:50) → `app::run()` setup (app.rs:301) → `spawn_event_pump` ticker → app.rs:215 `frame_channel(&app, id)` / app.rs:224 `send_record(channel, record)`. The chain is severed one level up: app.rs:301 calls `spawn_session_pump`, a name that exists nowhere (repo-wide rg: exactly one hit — the call; the only definition is `spawn_event_pump`, app.rs:190) → E0425, the crate cannot compile, no `lumen-tauri` binary can be produced from this tree. Independently of that: the sandbox cannot build the tauri crate at all (baseline #2, exit 101 at gdk pkg-config, dependency build script dies before this crate's sources are type-checked). If app.rs's typo were fixed, reachability would become YES-by-code (UI bind: MediaSurface.tsx:147-152 `session.set-frame-channel {channel: shim.id}` → shell.rs:160-161 intercept → :202 `take_frame_channel_binding` → this file), still REAL-UNPROVEN for execution. |
| Findings | S0: 1 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | rg `frame_channel\|send_record\|FrameChannel` repo-wide (callers: app.rs:195/215/224, lib.rs:8; UI counterparts frame-channel.ts / MediaSurface.tsx) · rg `spawn_session_pump\|spawn_event_pump` repo-wide (E0425 evidence: 1 call site vs 1 differently-named definition) · sed -n '39,48p'/'55,57p' frame_channel.rs + '294,304p'/'188,192p' app.rs (verbatim quote verification) · read app.rs/main.rs/lib.rs/tauri.conf.json/capabilities/default.json/tests/single_command.rs/INFRA-BLOCKED.md · read ui/src/client/frame-channel.ts + ui/src/components/player/MediaSurface.tsx (receive-side contract) · rg `take_frame_records\|take_frame_channel_binding` apps/desktop/src/shell.rs (:202/:219 exist) · sed -n '515,545p' + rg L5/L6 boundaries.toml (layer facts) · Cargo.lock:3688-3690 (tauri 2.12.5) · baseline INDEX.md run #2; frame_channel_proof test green ×2 (test-workspace.log.txt:2339, orchrun-test-workspace-nofailfast.log.txt:2298) |

## 1. One-paragraph verdict
This 57-line file is the honest, competent half of the GUI frame lane: `frame_channel()` rehydrates the UI-supplied channel id through tauri's `JavaScriptChannelId` parse (`__CHANNEL__:<id>` — the same form the UI shim allocates, frame-channel.ts:314) onto the "main" webview (label verified in tauri.conf.json:13), and `send_record()` ships one LUF1 record per message as `InvokeResponseBody::Raw` — the binary lane both sides of the seam document. The code has no panics, no unsafe, no swallowed errors, and its error contract matches its sole caller. But none of it is reachable: the only call chain runs through `spawn_event_pump` (app.rs:190), which is never called — app.rs:301 calls the nonexistent `spawn_session_pump` (E0425; the crate cannot compile, so no shipped `lumen-tauri` exists from this tree), and even on a webkit host the sandbox baseline died before type-checking this crate (run #2). Wave-0's "GUI mounts no canvas" evidence is superseded in-tree: the receive half now exists (MediaSurface.tsx:174 mounts a `<canvas>` inside the desktop shell and binds at :147-152), but the lane is dead at the Rust build, before any hang question can even arise. Verdict: real code, unreachable as committed.

## 2. What actually works (verified)
— Cross-seam contract consistency, verified by read: the `__CHANNEL__:<id>` rehydration format (frame_channel.rs:40) matches the UI shim's id allocation and its own cross-reference (ui/src/client/frame-channel.ts:314 "The id the shell rehydrates (`__CHANNEL__:<id>` — frame_channel.rs)"), and `session.set-frame-channel` with a bare numeric channel is what MediaSurface.tsx:150-151 dispatches and shell.rs:160-161 intercepts (stored u32, taken one-shot at shell.rs:202).
— Window-label match, verified by read: `get_webview_window("main")` (frame_channel.rs:45) matches the single configured window `label: "main"` (tauri.conf.json:13).
— Frame-math claim, verified by arithmetic: 1280×720 RGBA = 921,600 px × 4 B = 3,686,400 B ≈ 3.7 MB — the module doc's number (frame_channel.rs:17) is correct.
— Engine→mailbox half of this lane executes headlessly (NOT this file): `frame_channel_proof_pump_once_ships_luf1_records` green in both baseline workspace runs (test-workspace.log.txt:2339; orchrun-test-workspace-nofailfast.log.txt:2298).
Nothing in THIS file was executed — no build lane can reach it (see F-001 and §5).

## 3. Findings

### F-001 [S0]
Where: apps/desktop-tauri/src/app.rs:301 (call) vs app.rs:190 (definition); blocks frame_channel.rs:39-48 and :55-57 entirely. Cross-file: primary ownership W14-A01 (same defect recorded S0 by W14-A02 on main.rs) — recorded here because it voids this file's only callers and any reachability claim.
Code:
```rust
// app.rs:300-302 — inside the .setup closure of run():
            apply_window_spec(app.handle())?;
            spawn_session_pump(app.handle().clone());
            open_argv_files(app.handle());
```
```rust
// app.rs:190 — the ONLY ticker definition (repo-wide rg: `spawn_session_pump`
// has exactly one hit — the call site; no definition exists anywhere):
fn spawn_event_pump(app: AppHandle) {
```
What is wrong: `spawn_session_pump` does not exist. The function is `spawn_event_pump` (and app.rs's own module docs at :42 already say `spawn_event_pump`). E0425, unconditional on every platform — no cfg, no feature, no glob import in scope (app.rs:84-94 has none). The lib target does not compile → the `lumen-tauri` bin does not compile → both functions in THIS file are unreachable from any shipped entry point, and the GUI frame lane (open → dispatch → engine → LUF1 tap → this file → webview canvas) is dead at the build step.
Why it matters: this is the frame-presentation seam of the desktop product; while the crate cannot compile, the entire w31.1/w31.4 story (shell shipper + UI canvas consumer) is unshippable, and INFRA-BLOCKED.md:5-6's "the crate compiles in its dedicated CI lane (`tauri-check`)" is false for this tree. The baseline's UNVERIFIED-BUILD (gdk pkg-config, run #2) masked this: the build died in a dependency build script before this crate's sources were type-checked.
What correct looks like: app.rs:301 calls `spawn_event_pump` (one word), and a green tauri-check on this exact tree is cited; this file's verdict then flips to REAL-UNPROVEN.
Confidence: HIGH — Verification: read (repo-wide rg for both identifiers; sed-verified quotes). Static proof; no build needed to establish E0425.

### F-002 [S3]
Where: apps/desktop-tauri/src/frame_channel.rs:20-22
Code:
```rust
//! exists for). The webview side (the UI's channel consumer, the
//! `lumen-frames.mjs` reference port) receives an `ArrayBuffer` per
//! message — one complete LUF1 record.
```
What is wrong: The module doc names the WRONG consumer. `apps/desktop/prototype/lumen-frames.mjs` is a prototype loaded by nothing — the existing audit (audit/files/apps/desktop/prototype/lumen-frames.mjs.audit.md) proves its only "consumers" are comments (this line included) and verdicts it UNREACHABLE. The real shipped consumer is `ui/src/client/frame-channel.ts` (`FrameChannelShim` + `decodeLuf1Record`) feeding `MediaSurface.tsx`'s canvas — which the doc never mentions.
Why it matters: the seam's documentation points a maintainer at dead code instead of the live contract pair; same defect class W-wire.rs#part1 flagged ("the actual shipped consumer path … exists and is different from all three named transports"). The substantive claim (ArrayBuffer per message) IS true of the real consumer (frame-channel.ts:419-420 `if (message instanceof ArrayBuffer)`, plus the 2.12.x note :28-33) — the citation is what lies.
What correct looks like: cite `ui/src/client/frame-channel.ts` (+ MediaSurface.tsx) as the consumer; keep `lumen-frames.mjs` labeled as the non-shipped reference decoder.
Confidence: HIGH — Verification: read (both files; prior audit cited).

### F-003 [S3]
Where: apps/desktop-tauri/src/frame_channel.rs:52-54 (doc) with 55-57 (code) and 39-43; the failure path is app.rs:223-228 + ui/src/components/player/MediaSurface.tsx:146-152
Code:
```rust
/// # Errors
/// The tauri send error (the webview went away — a reload or close;
/// the caller drops the channel and the next bind re-establishes).
```
```rust
pub fn send_record(channel: &FrameChannel, record: &[u8]) -> Result<(), tauri::Error> {
    channel.send(InvokeResponseBody::Raw(record.to_vec()))
}
```
What is wrong: The documented recovery story — "the caller drops the channel and the next bind re-establishes" — assumes a re-bind always follows a send failure. It does not: the UI binds exactly once, at surface mount (MediaSurface.tsx:146-152, inside the mount effect; re-bind happens only on remount, i.e. a webview reload), while the caller (app.rs:223-228) sets `frame_channel = None` on ANY send error and then skips `take_frame_records` forever until a new bind arrives. A send failure on a live webview (eval/IPC error without a reload) therefore permanently stops video while the pacer keeps driving audio/events — black screen, sound continues, no re-establishment.
Why it matters: the doc sells a self-healing contract the shipped pair cannot deliver in the non-reload failure mode; when the GUI lane is fixed and run, this is the shape of a "video froze/died, audio kept playing" bug report. Narrow window (dominant failures ARE reloads, which remount and re-bind), hence S3 not S2 — but the claim is on this file's public doc.
What correct looks like: either the caller retries the send on the next tick with the same channel (only dropping it after N consecutive failures), or the UI re-binds on a frames-timeout heartbeat — and the doc states which side owns recovery.
Confidence: MEDIUM — Verification: read (all three sites); the transient-failure behavior of tauri 2.12.5 `Channel::send` on a live webview is UNVERIFIED (no registry cache to read the crate source; no GUI build possible).
### F-004 [S4]
Where: apps/desktop-tauri/src/frame_channel.rs:35-38 (doc) with 39-43 (code)
Code:
```rust
/// # Errors
/// A `String` sentence (logged by the ticker, never fatal): the id
/// form is invalid (a UI bug), or the main webview is absent (the
/// window closed mid-bind — the next bind retries).
```
```rust
pub fn frame_channel(app: &AppHandle<Wry>, channel: u32) -> Result<FrameChannel, String> {
    let reference = format!("__CHANNEL__:{channel}");
    let id: JavaScriptChannelId = reference
        .parse()
        .map_err(|err: &str| format!("the channel reference {reference:?} is invalid: {err}"))?;
```
What is wrong: The first documented failure mode cannot occur. The input is a `u32`; `format!("__CHANNEL__:{channel}")` always produces a well-formed reference, so the `parse` error branch (and its "the id form is invalid (a UI bug)" doc line) is dead code — the parse round-trip exists only because tauri exposes no public constructor for `JavaScriptChannelId`. The only reachable error here is the absent-window one.
Why it matters: a documented "UI bug" failure mode that no input can trigger is a small doc lie; a future reader will hunt for a UI-side id-format bug that cannot exist.
What correct looks like: doc says "the parse is the only public constructor path (always succeeds for a u32); the real failure is the absent main webview", or the branch is removed with a comment explaining why it is kept (defense against tauri changing the reference grammar).
Confidence: HIGH — Verification: read (input type + format string are in the same function; no other caller passes anything but a u32 — app.rs:208-215 passes the shell-stored u32).
### F-005 [S4]
Where: apps/desktop-tauri/src/frame_channel.rs:55-57 (with app.rs:223 for the caller's ownership of the record)
Code:
```rust
pub fn send_record(channel: &FrameChannel, record: &[u8]) -> Result<(), tauri::Error> {
    channel.send(InvokeResponseBody::Raw(record.to_vec()))
}
```
What is wrong: `record.to_vec()` is a full-record copy per shipped frame — ~3.7 MB for the doc's own 1280×720 RGBA example — allocated and memcpy'd on the pacer ticker thread every tick that ships. The caller owns the records (`Vec<Vec<u8>>` from `take_frame_records`, app.rs:209-211) and ships only `records.last()` (app.rs:223), so an ownership-taking signature (`record: Vec<u8>`, or the caller passing the last element by value) would make the ship zero-copy from the mailbox onward.
Why it matters: avoidable per-frame megabyte-scale alloc+copy in the presentation hot path; bounded and off the runtime lock (app.rs drops the lock before shipping), so hygiene-grade, not a stall risk.
What correct looks like: `send_record(channel: &FrameChannel, record: Vec<u8>)` with the caller moving `records.pop()`/last-owned record in; the type-alias doc already frames the record as "one complete LUF1 record" per message, so nothing else changes.
Confidence: HIGH — Verification: read (this file + app.rs:208-229).

### F-006 [S4]
Where: apps/desktop-tauri/src/frame_channel.rs:6-11
Code:
```rust
//! # Why this lives in the wrapper (not `apps/desktop`)
//!
//! The std-only library cannot name `tauri` types (boundaries.toml:
//! the L5 wrapper owns the webview seam). The id → channel
//! rehydration is three lines of tauri API — it lives here, beside
//! the invocation seam it serves.
```
What is wrong: The authority citation is wrong on both counts. boundaries.toml contains no "wrapper owns the webview seam" rule — the wrapper crate is workspace-EXCLUDED from boundaries.toml governance entirely (its own Cargo.toml:13-15: "this one is excluded, so its edges are declared here"), and in boundaries.toml `apps/desktop` is layer SIX (`# L6 — the desktop composition-root SKELETON`, boundaries.toml:519; `[crates.desktop] layer = 6`, :538-539) while L5 is the platform-adapter layer (:441, audio). The true and verifiable support for the claim is boundaries.toml:524 ("std-only, NO tauri/wry import (the webview host is a typed seam") plus ADR-004/the dependency record.
Why it matters: a layering citation that names the wrong layer and a rule that is not in the cited file is exactly the "evidence string cites the wrong path" defect class the repo's own feature-audit flags elsewhere.
What correct looks like: cite boundaries.toml's std-only/no-tauri-import line for `apps/desktop` (L6) and the crate's own exclusion note; call the wrapper the webview seam owner without pinning a wrong layer number on it.
Confidence: HIGH — Verification: read (sed -n '515,545p' boundaries.toml; rg L5/L6; desktop-tauri/Cargo.toml:13-15).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `type FrameChannel` (alias) | 27-29 | REAL | — | `tauri::ipc::Channel<InvokeResponseBody>` — the raw-binary channel form; sole user app.rs:195. Correct and minimal. |
| `frame_channel` | 31-48 (body 39-48) | REAL | S4 | No panics/unsafe; `err: &str` closure matches tauri's `JavaScriptChannelId` FromStr error type (API shape UNVERIFIED — no registry cache); `get_webview_window("main")` matches conf label; returns owned `Channel` on the main webview. First error branch dead for u32 input (F-004); assumes the bind came from the "main" webview — true today (single window, tauri.conf.json:11-21), silently wrong if a second window ever ships (no per-bind webview tracking — tauri's design, not a bug in this file). Sole caller app.rs:215 — unreachable as committed (F-001). |
| `send_record` | 50-57 (body 55-57) | REAL | S3/S4 | One `to_vec()` full-record copy per ship (F-005); error contract matches caller (drop + re-bind, app.rs:224-228) but the doc's recovery story has no UI trigger for non-reload failures (F-003); no panic paths; `InvokeResponseBody::Raw` is the binary lane both sides document. Sole caller app.rs:224 — unreachable as committed (F-001). |
## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| tests/single_command.rs (crate's only test file) | NO (for this file) | It scans app.rs's `generate_handler!` registration text (single_command.rs:22-32); it never reads frame_channel.rs. |
| ui/src/client/frame-channel.test.ts | NO (for this file) | Tests the JS shim against FAKE internals (`FakeInternals`, test:254) — the Rust half is mocked away by construction. |
| apps/desktop/src/shell.rs `frame_channel_proof_pump_once_ships_luf1_records` | NO (for this file) | Proves engine→mailbox records headlessly (baseline green ×2: test-workspace.log.txt:2339, nofailfast:2298) — it stops at `take_frame_records`, before this file's rehydration/send. |
| Mutation check (reasoned, not run) | — | Swap `"main"`→`"mainx"` at :45, or `Raw`→`Json` at :56, or drop the `channel_on` binding: every existing suite in the repo stays green. This file has ZERO test coverage — and zero executable lanes (F-001 + baseline #2). |
## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "raw bytes ride the fetch-based IPC path for large payloads … the Raw form is the binary lane tauri's channel protocol exists for" | frame_channel.rs:15-20 | REAL-UNPROVEN | Matches tauri 2.12 channel design as studied by the UI side (frame-channel.ts:28-33, :427-447 token-fetch branch serves the same protocol); cannot execute — crate unbuildable (F-001, baseline #2). Tauri source not readable in sandbox (no registry cache). |
| "a 1280×720 RGBA frame is ~3.7 MB" | frame_channel.rs:17 | REAL | 1280×720×4 = 3,686,400 B ≈ 3.69 MB. |
| "the JSON-array form a plain `Channel<Vec<u8>>` would produce is 4-6× that and evaluates as a script" | frame_channel.rs:17-19 | REAL-UNPROVEN | Mechanism plausible per channel protocol (UI side serves both shapes); the 4-6× multiple is inflated for the payload itself — uniform-byte JSON arrays average ≈3.6× (10·2+90·3+156·4 = 914 chars/256 B); envelope overhead may close the gap. No execution possible. |
| "The webview side (the UI's channel consumer, the `lumen-frames.mjs` reference port) receives an `ArrayBuffer` per message — one complete LUF1 record" | frame_channel.rs:20-22 | SHALLOW (right fact, wrong consumer) | Real consumer frame-channel.ts:419-420 handles ArrayBuffer; the named `lumen-frames.mjs` is prototype/dead (F-002; prior audit UNREACHABLE). |
| "the pacer's per-tick drain ships LUF1 records as BINARY payloads" | frame_channel.rs:2-4 | REAL-UNPROVEN | Engine→mailbox half proven (shell.rs proof test, baseline ×2); this file's rehydrate+send half unexecutable (F-001). |
| "boundaries.toml: the L5 wrapper owns the webview seam" | frame_channel.rs:8 | FALSE | boundaries.toml has no wrapper rule (crate excluded; Cargo.toml:13-15) and apps/desktop is L6 (:519/:538); L5 = platform adapters (:441) — F-006. |
| INFRA-BLOCKED.md: "the crate compiles in its dedicated CI lane (`tauri-check`, Linux + webkit2gtk)" | INFRA-BLOCKED.md:5-6 | FALSE (for this tree) | E0425 at app.rs:301 (F-001); the baseline's pkg-config wall (run #2) means no lane has type-checked this tree's crate sources. |
## 7. Dead / duplicated / unreachable code
- BOTH functions + the alias: unreachable as committed — sole call chain severed at app.rs:301 (F-001); the module itself is properly declared (lib.rs:8) and would be live the moment the one-word typo is fixed.
- Dead error branch in `frame_channel` (parse failure of a self-formatted u32 reference) — F-004.
- No duplication found: this is the only `tauri::ipc::Channel` construction site in the repo (rg over apps/ + crates/); the `[PROPOSAL]` TauriChannelTransport in apps/desktop/src/transport.rs is a different, unwired type (noted by W13-A05/W-wire.rs audits).
## 8. Dependencies & flags
- `tauri = "2"` → resolved 2.12.5 (apps/desktop-tauri/Cargo.lock:3688-3690), the version the UI side says it studied (frame-channel.ts:28-33).
- Crate is workspace-EXCLUDED (root Cargo.toml `exclude`; own lockfile); lints mirrored inline (Cargo.toml:34-47): `unsafe_code`/`unwrap_used`/`expect_used`/`panic`/`todo`/`unimplemented` all denied — this file violates none (no unsafe, no unwrap/expect/panic).
- No feature flags gate this file. Security posture of the seam (hostile review): the channel id is webview-supplied, but `frame_channel` only rehydrates it onto the same single trusted webview the UI runs in (CSP locked to 'self', capabilities `core:default` + 4 window grants only — capabilities/default.json); frames are the playing video, not secrets; no file/command surface is opened by this file. No security hole found.
## 9. Open questions for the next agent
- W14-A01 (app.rs owner): F-001's one-word fix (`spawn_session_pump`→`spawn_event_pump`) flips this file's verdict to REAL-UNPROVEN; until then every GUI-lane finding downstream is moot. Also: F-003's recovery gap (send-error → permanent black screen with live audio) is a policy decision sitting in YOUR file (app.rs:223-228) — decide retry-vs-heartbeat.
- A webview-equipped wave: (a) confirm tauri 2.12.5 `Channel<InvokeResponseBody>` Raw delivery arrives as ArrayBuffer (not token) on the real shell, matching frame-channel.ts's primary branch; (b) confirm `JavaScriptChannelId::channel_on` + `WebviewWindow::as_ref::<Webview>` compile as written — both UNVERIFIED here (no registry cache, no build).
- Wave gate: REQUEST-ORCH-RUN (logged) — `cargo check --manifest-path apps/desktop-tauri/Cargo.toml --locked` on a webkit host, to surface E0425 officially and expose any further compile breaks hiding behind the sandbox's pkg-config wall.
