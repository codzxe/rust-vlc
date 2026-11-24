# AUDIT: apps/desktop-tauri/src/frame_channel.rs
| Field | Value |
|---|---|
| Auditor | W14-A07 |
| Date | 2026-11-24 |
| Lines / functions | 57 / 2 fns + 1 type alias |
| File verdict | UNREACHABLE (real, API-correct code; sole caller chain is compile-broken upstream) |
| Reachable from shipped entry point? | NO — only callers are app.rs:215/224 inside `spawn_event_pump`, which is never called: app.rs:301 calls undefined `spawn_session_pump` (E0425, W13-A10 F-001); crate also excluded from the release lane (root Cargo.toml:51 `exclude = ["apps/desktop-tauri"]`) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 2 |
| Commands run | rg (callers, spawn_session_pump repo-wide, cfg(test), mailbox, window label); sed -n on every quoted range incl. vendored ~/.cargo tauri-2.12.5/src/ipc/channel.rs; all exit 0. No cargo (gdk-sys env limit + no-GTK baseline; cargo-build-budget). |

## 1. One-paragraph verdict
This 57-line module is the Rust end of the LUF1 frame-to-canvas wire: `frame_channel()` rehydrates the webview's channel id into a `tauri::ipc::Channel` and `send_record()` ships one LUF1 record as `InvokeResponseBody::Raw`. The code is genuine and its tauri API usage is byte-verified correct against the vendored tauri 2.12.5 source (prefix constant, Raw-lane semantics, ArrayBuffer delivery). But it is dead in the shipped tree: its only callers (app.rs:215/224) live in `spawn_event_pump`, which nothing calls because app.rs:301 invokes the nonexistent `spawn_session_pump` — the whole GUI frame path cannot compile on any host, so no shipped entry point (lumen-tauri or the release lane) reaches this file. Beyond reachability, the design carries a real memory hazard: every ≥1 KiB Raw send parks the full ~3.7 MB record in tauri's unbounded `ChannelDataIpcQueue`, which a stalled webview drains at zero rate while the 10 ms ticker keeps filling it.

## 2. What actually works (verified)
- The `__CHANNEL__:{channel}` rehydration format (frame_channel.rs:40) matches tauri 2.12.5's `pub const IPC_PAYLOAD_PREFIX: &str = "__CHANNEL__:";` — verified by sed in ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.12.5/src/ipc/channel.rs:27.
- The Raw-lane claim matches tauri's own channel docs (vendored channel.rs:136-141: `InvokeResponseBody::Raw` → "received in JavaScript as an `ArrayBuffer`"; "a `Channel<Vec<u8>>` … is sent as a JSON array of numbers"). UI side (ui/src/client/frame-channel.ts:419-447) consumes exactly that ArrayBuffer.
- Hard-coded `"main"` window label (frame_channel.rs:45) matches apps/desktop-tauri/tauri.conf.json:13 `"label": "main"`.
- No unwrap/expect/panic in the file — consistent with the crate's lint posture (Cargo.toml:40-47 denies `unwrap_used`/`expect_used`/`panic`).
- Verified by sed -n on all ranges quoted below (exit 0 each). Nothing executed — no GTK/webview in sandbox (baseline cargo-build-desktop-tauri.txt, gdk-sys failure).

## 3. Findings
F-001 [S2]
Where: apps/desktop-tauri/src/frame_channel.rs:39-57 (file) via apps/desktop-tauri/src/app.rs:301
Code (app.rs:299-303):
```
            // (CI tauri-check, run 43384519598, two E0308s).
            apply_window_spec(app.handle())?;
            spawn_session_pump(app.handle().clone());
            open_argv_files(app.handle());
            Ok(())
```
What is wrong: `spawn_session_pump` has no definition anywhere in the repository (repo-wide rg → only app.rs:301; the defined ticker is `fn spawn_event_pump` at app.rs:190, the sole caller of `frame_channel()`/`send_record()` at app.rs:215/224). E0425: the lumen-tauri binary cannot compile on ANY host, so this module — the only LUF1 shipper to the webview — is unreachable from every shipped entry point (the release lane additionally excludes the crate: root Cargo.toml:51 `exclude = ["apps/desktop-tauri"]`). The break itself is W13-A10's S0 on app.rs; logged here at S2 because for THIS file it voids the module's entire reason to exist rather than being a false claim in this file. Why it matters: every "GUI shows video" narrative routes through these 57 lines, and none of it runs. What correct looks like: fix the call site (rename to `spawn_event_pump`) and prove it in the tauri-check lane.
Confidence: HIGH — Verification: read + rg (static; compile impossible here: no GTK + cargo-build-budget).

F-002 [S2]
Where: apps/desktop-tauri/src/frame_channel.rs:55-57; tauri-2.12.5/src/ipc/channel.rs:47-54, 302-329 (vendored)
Code (frame_channel.rs:55-57):
```
pub fn send_record(channel: &FrameChannel, record: &[u8]) -> Result<(), tauri::Error> {
    channel.send(InvokeResponseBody::Raw(record.to_vec()))
}
```
Code (tauri 2.12.5 channel.rs:47-54):
```
pub struct ChannelDataIpcQueue(Arc<Mutex<HashMap<String, WebviewChannelDataQueue>>>);

/// Pending channel data of one webview.
#[derive(Default)]
struct WebviewChannelDataQueue {
  next_id: u32,
  entries: HashMap<u32, InvokeResponseBody>,
}
```
What is wrong: unbounded memory on a slow consumer. Every LUF1 record ≥ 1024 bytes (all real frames; a 720p RGBA record is ~3.7 MB) takes tauri's fetch path: `insert_if` parks the whole `InvokeResponseBody` in a per-webview `HashMap` with NO capacity bound (only a webview-registered check), and `Channel::send` returns as soon as the fetch script is eval'd — fire-and-forget. Lumen's own backpressure stops one step earlier (the tap's mailbox, capacity 2, latest-wins — apps/desktop/src/mailbox.rs:56 `NonZeroUsize::new(2)`), and the ticker ships at most one record per 10 ms tick (app.rs:109 `SESSION_TICK`), but nothing bounds the tauri queue: a webview main thread stalled for T seconds (busy JS, GC, devtools) accumulates up to 100 × 3.7 MB/s ≈ 370 MB/s worst-case, OOM-class on a 3 GB machine in seconds; entries are freed only when the JS fetch arrives. What correct looks like: a ship-side bound (skip/flag when in-flight fetches exceed N) or a coalescing check before send. Why it matters: "never fatal" error handling keeps the ticker shipping into a queue nobody drains.
Confidence: MEDIUM — Verification: read (vendored tauri source sed-quoted; behavior not executable here — no webview).

F-003 [S3]
Where: apps/desktop-tauri/src/frame_channel.rs:55-57 + apps/desktop/src/frame_tap.rs:125-133
Code (frame_tap.rs:125-131):
```
fn encode_record(frame: &ShippedFrame, out: &mut Vec<Vec<u8>>) -> bool {
    let mut record = Vec::with_capacity(frame.header.payload_len as usize + MIN_HEADER_LEN);
    match encode_into(&frame.header, &frame.payload, &mut record) {
        Ok(()) => {
            out.push(record);
```
What is wrong: double full-frame copy in the hot path. `encode_record` already allocates a fresh record Vec per drained frame; `send_record`'s `record.to_vec()` then copies the identical ~3.7 MB again because the signature borrows `&[u8]` while tauri requires an owned `Vec<u8>`. At the ticker's shipping cadence (up to one per 10 ms tick; realistically 30-60 fps) that is ~110-220 MB/s of needless allocation+memcpy churn on a 2-core box, plus the same again for the mandatory first copy. What correct looks like: `send_record(channel, record: Vec<u8>)` (the caller already owns `records.last()` and could hand it over via `Vec` pop/swap).
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: apps/desktop-tauri/src/frame_channel.rs:15-19 (module doc); docs/audits/2026-10-22-directives-audit.md:23
Code (frame_channel.rs:15-19):
```
//! `Channel<InvokeResponseBody>` with [`InvokeResponseBody::Raw`]:
//! raw bytes ride the fetch-based IPC path for large payloads (a
//! 1280×720 RGBA frame is ~3.7 MB — the JSON-array form a plain
//! `Channel<Vec<u8>>` would produce is 4-6× that and evaluates as a
//! script; the Raw form is the binary lane tauri's channel protocol
//! exists for). The webview side (the UI's channel consumer, the
```
What is wrong: the doc frames the JSON-eval form as what "a plain `Channel<Vec<u8>>` would produce", but tauri 2.12.5 uses that exact JSON-eval form for THIS module's own `Raw` payloads below 1024 bytes (`MAX_RAW_DIRECT_EXECUTE_THRESHOLD: usize = 1024`, vendored channel.rs:38; `format!("{{ message: new Uint8Array({bytes_as_json_array}).buffer, ...`", channel.rs:302-305). directives-audit.md:23's "no JSON on frame path" is therefore false for sub-1 KiB records (frames ≤ 15×15 RGBA + 48-byte header). Practical impact nil (real video frames are ≥ ~0.9 MB), but the audit doc's blanket claim is not what the code does. What correct looks like: state the 1 KiB threshold.
Confidence: HIGH — Verification: read (vendored source).

F-005 [S4]
Where: docs/KNOWN-LIMITS.md:52 vs ui/src/components/player/MediaSurface.tsx:173-183 + this file
Code (KNOWN-LIMITS.md:52, excerpt):
```
| **GUI video display: not wired** | The desktop UI mounts an honest empty media surface — no `<canvas>` exists in `ui/src` (pinned by test: the surface is a labelled region, never a fake video); ...
```
What is wrong: stale. As of w31.4 a real `<canvas data-media-surface="frames">` mounts in MediaSurface.tsx:173-183, ui/src/client/frame-channel.ts is a full channel consumer, and this file is its Rust shipper. The referenced "no-fake-canvas pin" is outdated. Doc-vs-tree contradiction, not a code defect.
Confidence: HIGH — Verification: read (sed-verified both sides).

## 4. Per-function table (57 LOC — table included anyway)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FrameChannel` (type alias) | 29 | REAL | — | `tauri::ipc::Channel<InvokeResponseBody>`; the correct binary-lane type per tauri 2.12.5 docs |
| `frame_channel()` | 39-48 | REAL-UNPROVEN | — | Prefix/parse/`channel_on` all match vendored tauri 2.12.5; "main" label matches tauri.conf.json:13; never executed anywhere (F-001); zero test coverage |
| `send_record()` | 55-57 | REAL-UNPROVEN | S3 | `to_vec()` double copy (F-003); feeds the unbounded tauri fetch queue (F-002); error contract (drop channel, rebind) honored by caller app.rs:223-228 |

## 5. Test honesty
No tests exist for this file — no `#[cfg(test)]` module in frame_channel.rs and none anywhere in apps/desktop-tauri/src (rg `cfg\(test\)` → no hits; the crate's only test is tests/single_command.rs, the anti-bypass registration check). The surrounding seam is tested only in neighbors: apps/desktop/src/frame_tap.rs:174-232 (drain/LUF1/latest-wins/repeat — genuine, passing in baseline cargo-test-workspace.txt) and ui/src/client/frame-channel.test.ts (mocked `__TAURI_INTERNALS__`, no real webview). The Rust rehydration+send pair itself has NEVER been executed by any test or CI lane — honest gap, execution impossible in this sandbox (no GTK; baseline gdk-sys failure).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "raw bytes ride the fetch-based IPC path for large payloads" | frame_channel.rs:15-16 | REAL (for ≥1024 B; blanket framing misleading below) | tauri 2.12.5 channel.rs:38, 302-329 (F-004) |
| "a 1280×720 RGBA frame is ~3.7 MB" | frame_channel.rs:16-17 | REAL | 1280×720×4 = 3,686,400 B ≈ 3.69 MB |
| "JSON-array form … 4-6× … and evaluates as a script" | frame_channel.rs:17-19 | REAL (directionally; ratio is arithmetic estimate, exact 4-6× UNVERIFIED) | tauri docs: `Channel<Vec<u8>>` → JSON array; channel.rs:302-305 shows the eval |
| "receives an `ArrayBuffer` per message" | frame_channel.rs:20-22 | REAL | both tauri paths deliver ArrayBuffer: fetch path runCallback(response), eval path `new Uint8Array(…).buffer` (channel.rs:302-329) |
| "A `String` sentence (logged by the ticker, never fatal)" | frame_channel.rs:36-38 | REAL-UNPROVEN | caller eprintln+continue (app.rs:217-221) — but the ticker itself is dead (F-001) |
| "the crate compiles in its dedicated CI lane (`tauri-check`)" | apps/desktop-tauri/INFRA-BLOCKED.md:4-5 | FALSE (against current tree) | `spawn_session_pump` undefined → E0425 (W13-A10 F-001; my rg confirms) |
| "desktop wire = binary LUF1 (no JSON on frame path)" | docs/audits/2026-10-22-directives-audit.md:23 | SHALLOW | false for <1024 B records (F-004); true for all real frames |
| "GUI video display: not wired … no `<canvas>` exists in `ui/src`" | docs/KNOWN-LIMITS.md:52 | FALSE (stale) | MediaSurface.tsx:173-183 mounts the canvas (F-005) |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable dead code in the current tree (F-001): lib.rs:8 `pub mod frame_channel;` exposes it, app.rs:215/224 are the only call sites, and they sit in a function (`spawn_event_pump`, app.rs:190) that no live code calls. No duplication: the `apps/desktop` (std-only) side deliberately cannot name tauri types; this wrapper holds the only id→Channel rehydration.

## 8. Dependencies & flags
tauri 2.12.5 / tauri-runtime-wry 2.12.3 / wry 0.57.2 (apps/desktop-tauri/Cargo.lock:3688-3689, 3828-3829, 4961-4962). Crate is workspace-EXCLUDED (root Cargo.toml:51) with its own lockfile — the release lane `cargo build --release --locked -p lumen-cli -p desktop` never touches it. Lints inline (Cargo.toml:34-47): `unwrap_used`/`expect_used`/`panic`/`todo`/`unimplemented` denied — the file complies. No feature flags gate this code.

## 9. Open questions for the next agent
- W15+ (app.rs owner): does the tauri-check CI lane's last green run predate the `spawn_event_pump` rename (i.e., is INFRA-BLOCKED.md:4-5 stale or the lane red)? Decides whether F-001 is a fresh regression or a never-green claim.
- Does any launch smoke actually drive `session.set-frame-channel` end-to-end on a webview host (the UI bind is mount-time, MediaSurface.tsx:146-166)? If yes, it would have caught E0425 — find why it didn't.
- The JS shim's token-string fetch branch (frame-channel.ts:427-445) is defensive-only on tauri 2.12.5 (the shell evals the fetch itself, channel.rs:324-328) — confirm no other tauri version is shipped in any bundling lane.
- F-002's real-world blast radius: measure whether a stalled webview is reachable in practice (devtools open, heavy putImageData on 4K frames) before deciding if it escalates to S1.
