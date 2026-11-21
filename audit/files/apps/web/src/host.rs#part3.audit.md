# AUDIT: apps/web/src/host.rs#part3
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-21 |
| Lines / functions | 1389–1954 (566 lines) / 22 |
| File verdict | REAL (native-test-executed; NOT reachable from a shipped entry point) |
| Reachable from shipped entry point? | NO — no crate depends on lumen-web (rg over all Cargo.tomls: zero dependents); inside the wasm artifact: `abi.rs` HOST singleton → `WebHost::dispatch` → `route` (verified chain); native lane: `mod tests` (part4) → passed in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (`lumen_web` 111 passed / 0 failed) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `rg -n "session_error_wire" apps/web/src` (2 callers, l.1528/1600); `sed -n` on every cited range (quotes verified verbatim); `rg "fn submit" crates/playback-engine/src/session.rs` (submit → `Result<(), SessionError>`, SessionClosed+InboxBusy documented); `sed -n '1061,1090p' crates/playback-engine/src/session.rs` (engine.open returns only Invariant/GenerationExhausted); `rg "SessionError::Config"` repo-wide (constructed nowhere in src; single-variant `SessionConfigError::NoDecodableTrack` at error.rs:188); `rg -n "seekable" apps/cli/src/host.rs` (CLI l.1307 also hard-codes true, but CLI serves seek); `rg "lumen-web|lumen_web" --include=Cargo.toml` (no dependents); baseline: `orchrun-test-workspace-nofailfast.log.txt`, `rc-smoke-engine-only.log.txt` |

## 1. One-paragraph verdict
This chunk is the web host's command surface: `dispatch`/`route`/`open_session`/`submit_and_await`/the class-S snapshots plus 10 wire-mapping free functions. The code is genuine, panic-free on the paths traced, and its refusals are typed and named (seek, config, playlist, track selection each name an owning card). It is exercised end-to-end natively by the crate's tests over a real AVI fixture (decode included, codecs linked unconditionally on native) and those tests PASSED in the orchestrator's no-fail-fast baseline rerun — the original two baseline `cargo test --workspace` runs never reached this crate (both aborted earlier: `desktop` zero-alloc fail, then `first-vertical` evidence drift). The defects found: one swallowed engine error (`InboxBusy` misreported as "session closed"), a hard-coded `seekable: true` that contradicts this host's own typed seek refusal, dead arms in `session_error_wire`, and two edge-case hygiene items. From the shipped `lumen`/`lumen-tauri` binaries this code is unreachable — the wasm module builds (rc-smoke stage 6: 38,062,701 bytes, `\0asm` verified) but no shipped product embeds it, and the baseline browser lane was skipped (`RC_SMOKE_SKIP_WEB=1`).

## 2. What actually works (verified)
— `host::tests::snapshots_answer_over_the_open_fixture`, `unserved_surface_refuses_typed`, `frames_to_ns_is_exact_and_saturating`, `pump_slices_play_the_fixture_to_its_end`, `pause_freezes_the_counter_clock`, `shutdown_closes_and_acknowledges` → all `ok` in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (lumen_web: `111 passed; 0 failed`). These drive `dispatch → route → open_session → media_info/current_time/duration/tracks` over `generated/valid/avi_mjpeg_pcm.avi` through the real engine on the native profile.
— `cargo build -p lumen-web --target wasm32-unknown-unknown --locked` → PASS, 38062701 bytes, `\0asm` magic (`audit/runs/baseline/rc-smoke-engine-only.log.txt` lines 25/64) — the wasm artifact containing this code compiles.

## 3. Findings

### F-001 [S2]
Where: apps/web/src/host.rs:1598-1600
Code:
```rust
        handle
            .submit(request, kind)
            .map_err(|_| session_error_wire(&SessionError::SessionClosed))?;
```
What is wrong: `SessionHandle::submit` returns `Result<(), SessionError>` with two documented outcomes — `SessionClosed` and `InboxBusy` ("the bounded inbox is full (the command was NOT queued — retry or shed load)", crates/playback-engine/src/session.rs:315-320). This closure discards the real error (`|_|`) and fabricates `SessionClosed`.
Why it matters: when the 64-slot inbox (`INBOX_CAPACITY: usize = 64`, session.rs:121) fills — realistic whenever commands outpace polls — every command (play/pause/stop/close) is answered "the session closed before the command landed" while the session is in fact alive and the documented remedy is "poll the engine, then retry". The client gets a false terminal diagnosis instead of a retryable one. The file's own `session_error_wire` InboxBusy arm (l.1941-1946) exists precisely for this and is made unreachable from this path. The CLI twin has the identical defect (apps/cli/src/host.rs:863-864), so this is a faithfully mirrored bug, not a web-specific drift.
What correct looks like: `.map_err(|error| session_error_wire(&error))?` — pass the real error through; the mapping function already handles both variants.
Confidence: HIGH — Verification: read (submit signature + doc, session.rs:320-327; INBOX_CAPACITY session.rs:121) and compared with session_error_wire l.1941-1946.

### F-002 [S3]
Where: apps/web/src/host.rs:1929-1946
Code:
```rust
        SessionError::Config(cause) => CommandError::new(
            commands::Stage::Session,
            commands::ErrorCode::Unsupported,
            cause.to_string(),
        )
        .with_detail("code", "session.no-decodable-track"),
```
What is wrong: the `Config` and `InboxBusy` arms of `session_error_wire` are unreachable dead code from this file. The function has exactly two callers (rg): l.1528 maps `engine.open`'s error, which can only be `SessionError::Invariant` or `GenerationExhausted` (session.rs:1061-1090 — `replace_live_session` is never reached because `open_session` refuses a live session first at l.1446-1452); l.1600 synthesizes `SessionClosed`.
Why it matters: dead defensive surface suggests open-path failures can carry `no-decodable-track` to web clients; they cannot (that refusal is produced by the engine event path as a Fatal, surfaced via `last_fatal`, not through this function). A maintainer patching the Config arm would be editing code no input can reach.
What correct looks like: either delete the unreachable arms or route the real open/close errors through the mapping (F-001's fix gives InboxBusy a live path; Config needs an engine.open that can return it).
Confidence: HIGH — Verification: read (both call sites traced to their only possible error constructors).

### F-003 [S3]
Where: apps/web/src/host.rs:1660
Code:
```rust
                duration: wire_duration(facts.duration_us()),
                seekable: true,
                is_live: false,
```
What is wrong: `seekable: true` is hard-coded while this same host refuses every `playback.seek` typed (`owner_of` l.1837-1840: "the seek boundary's host duties … land with the H-pillar pump integration"; `SERVED` l.762-775 omits `PlaybackSeek`).
Why it matters: a web client reads `media.get-info`, enables its seek bar on `seekable: true`, and every seek attempt then fails `unsupported-for-host`. The metadata lies about this host's capability. The CLI host also hard-codes `true` (apps/cli/src/host.rs:1307) but the CLI actually serves seek, so the mirrored constant is false here specifically. (`is_live: false` is correct for blob sources.)
What correct looks like: `seekable: false` on this host until the seek boundary lands (or derive it from served capability).
Confidence: HIGH — Verification: read + passing test `host::tests::unserved_surface_refuses_typed` proves seek refuses typed on this host.
Note: the CLI hard-code is W-audit territory for apps/cli (already noted by its wave); recorded here only because the web host's copy is the contradiction.

### F-004 [S4]
Where: apps/web/src/host.rs:1929-1934
Code:
```rust
        SessionError::Config(cause) => CommandError::new(
            commands::Stage::Session,
            commands::ErrorCode::Unsupported,
            cause.to_string(),
        )
        .with_detail("code", "session.no-decodable-track"),
```
What is wrong: the detail code is hard-coded, duplicating playback-engine's own mapping (`crates/playback-engine/src/error.rs:188`: `SessionConfigError::NoDecodableTrack => "session.no-decodable-track"`). Today the enum has exactly one variant so the string is accidentally correct.
Why it matters: the day a second `SessionConfigError` variant lands, every Config failure is still labeled `no-decodable-track` — silent drift the type system would have caught by matching on `cause`.
What correct looks like: map on the `cause` (the engine already exports the mapping) instead of a literal.
Confidence: HIGH — Verification: read (error.rs:175-190 single-variant match).

### F-005 [S4]
Where: apps/web/src/host.rs:1792
Code:
```rust
            frame_rate: fact.frame_ns.map(|ns| (1.0e9 / ns as f64) as f32),
```
What is wrong: `frame_ns == 0` (a malformed AVI whose stream rate numerator is 0 — open.rs:272-273 computes `num * 1_000_000_000 / den`, `num = 0` yields 0) divides 1e9 by 0.0 in f64 → `+inf`, serialized as the track's `frame_rate`.
Why it matters: file-controlled garbage (`Infinity`) reaches the wire metadata; no panic (IEEE division), so it is a metadata-quality defect only.
What correct looks like: `frame_ns.filter(|ns| *ns > 0).map(...)` or a clamp.
Confidence: MEDIUM — Verification: read; the frame_ns=0 input path was traced in open.rs:269-277 but not executed (no such fixture exists; building one is out of scope for a read-only audit).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `WebHost::dispatch` | 1389-1398 | REAL | — | echoes request_id; session/generation at answer time; reached via abi.rs worker + tests. |
| `WebHost::route` | 1441→ route table 1402-1437 | REAL | — | 13 served ids match `SERVED` exactly; SessionClose tears down only on Ok (rejected close leaves the session live — matches engine's compose-through-stop semantics). |
| `WebHost::open_session` | 1441-1554 | REAL | — | typed refusals for start_time/start_paused/rate/track-selection (all verified reachable by mutation reasoning in tests); `audio_rate` stored at l.1508 before the only later fallible step (engine.open) — a failed open leaves a stale rate, but `fill_audio` early-returns with no device, so the stale value is unread (dead store, not elevated). |
| `WebHost::teardown_after_fatal` | 1558-1563 | REAL | — | closes the Error-state session, then clears halves; `let _ =` on close is honest (nothing to report into). |
| `WebHost::teardown_session_scoped` | 1567-1577 | REAL | — | clears all 8 session-scoped fields incl. pending_receipts/pending_present. |
| `WebHost::session_closed` | 1580-1582 | REAL | — | thin alias. |
| `WebHost::submit_and_await` | 1586-1602 | REAL (defect) | S2 | F-001: swallows InboxBusy. |
| `WebHost::await_receipt` | 1605-1625 | REAL | — | bounded 64 polls, linear receipt scan (≤ pending count, tiny); honest Internal error on expiry. |
| `WebHost::receipt_outcome` | 1628-1645 | REAL | — | 3-arm exhaustive map. |
| `WebHost::media_info` | 1650-1667 | REAL (defect) | S3 | F-003 seekable hard-code; `source.ok_or_else(not_ready)` is a dead defensive check (facts/source set+cleared together). |
| `WebHost::current_time` | 1670-1694 | REAL | — | `buffered: 0..duration` verified honest for this host: the v1 grammar accepts only `blob:<name>` URLs with whole bytes preloaded (open.rs:162-184 refuses File + network), so the full timeline is staged. fraction clamps position; checked duration>0 before divide. |
| `WebHost::duration` | 1697-1704 | REAL | — | thin wire_duration wrapper. |
| `frames_to_ns` | 1708-1714 | REAL | — | saturating u128 math, rate==0 guarded; dedicated exactness/saturation test passed in baseline orchrun. |
| `open_null_lane` | 1720-1757 | REAL | — | clock_source is an Arc clone (null.rs:123-124) — dropping the sink in the lane-less case does not orphan the counter (verified). Default (2ch, 48 kHz) consistent with audio_rate default at l.841. |
| `not_ready` | 1760-1766 | REAL | — | shared typed refusal. |
| `wire_duration` | 1769-1776 | REAL | — | 2-arm map. |
| `enrich_track_info` | 1781-1806 | REAL (edge) | S4 | F-005 inf frame_rate; channels clamped to u16 honestly. |
| `unserved` | 1810-1820 | REAL | — | typed UnsupportedForHost + command/owner details; pinned by passing test. |
| `command_id_string` | 1824-1833 | REAL | — | serde round-trip for the id, honest "(unserializable)" fallback; error-path only. |
| `owner_of` | 1836-1861 | REAL | — | 10-arm ownership table; each names a real card/phase; first arm is the seek refusal that makes F-003 a contradiction. |
| `rejection_wire` | 1864-1923 | REAL | — | 10-arm exhaustive RejectionReason map, mirrors CLI; all messages carry state context. |
| `session_error_wire` | 1927-1953 | REAL (defect) | S3/S4 | F-002 dead arms, F-004 hard-coded code. |

## 5. Test honesty (tests live in part4, lines 1955+; they exercise this part)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `pump_slices_play_the_fixture_to_its_end` | YES | asserts final state Ended + per-track counters over a real decode; breaking open_session/dispatch/route fails it at open. |
| `snapshots_answer_over_the_open_fixture` | YES | asserts duration `Known{time>0}`, fraction ∈ [0,1], exactly 2 tracks with video dims + audio geometry (covers enrich_track_info), engine-info decoders contain mjpeg — mutations of wire_duration/enrich_track_info/engine_info each break a distinct assert. |
| `unserved_surface_refuses_typed` | YES | asserts exact code UnsupportedForHost + `command=playback.seek` detail + NotReady for info without session — covers unserved/owner_of/not_ready. |
| `frames_to_ns_is_exact_and_saturating` | YES | exact-value + u64::MAX saturation asserts. |
| `pause_freezes_the_counter_clock` | YES | position equality before/after pause over 8 pumps — covers submit_and_await(Pause) + clock freeze. |
| `shutdown_closes_and_acknowledges` | YES | covers shutdown()→close→teardown path + ShutdownComplete ordering. |
| `commands_before_boot_refuse_typed` | YES | covers the boot gate in handle() (part2) — dispatch of real commands unreachable pre-boot. |
| `wasm32_profile_decoder_seam_refuses_typed` | YES (cfg-gated `not(feature="builtin-codecs")`) | pins decoders_absent_error (part1 code, part4 test); correctly disabled under --all-features per its comment. |
All listed tests → `ok` in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt`.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`playback.seek` answers the typed refusal naming the missing host duty" | host.rs module docs l.53-57 | REAL | owner_of l.1837-1840 + test unserved_surface_refuses_typed passed (orchrun log). |
| "`config.*` answers the typed refusal naming the web settings card" | host.rs module docs l.57-59 | REAL | owner_of l.1840-1841 + ConfigGetSetting assert in same test. |
| "Everything here is exercised headless-native by this crate's tests" | host.rs module docs l.65-66 | REAL (with caveat) | orchrun nofailfast: lumen_web 111/0; but the two INDEX-listed baseline `cargo test --workspace` runs never reached this crate (aborted at `desktop`, resp. `first-vertical`) — the orchrun rerun is the only execution evidence. |
| "Browser-runtime behavior … is `[VERIFY]`" | host.rs module docs l.67-69 | honest gap | baseline rc-smoke ran with `RC_SMOKE_SKIP_WEB=1`; web stages 7/8 skipped (rc-smoke-engine-only.log.txt line 65). UNVERIFIED remains true. |
| media.get-info reports seekable media | implied by `seekable: true` l.1660 | FALSE (for this host) | seek is unserved — F-003. |

## 7. Dead / duplicated / unreachable code
- `session_error_wire` Config + InboxBusy arms (F-002).
- `media_info`'s `ok_or_else(not_ready)` on `self.source` (facts imply source) — harmless dead defense.
- `open_session`'s stale `audio_rate` after failed `engine.open` — dead store, unread.
- Duplicated across hosts: rejection_wire/session_error_wire/unserved tables mirror apps/cli/src/host.rs (acknowledged design: "the CLI host's web twin"); the duplication already diverged once (F-001 shared bug, F-003 divergent truth).

## 8. Dependencies & flags
- Chunk depends on: commands (catalog/envelope), playback-engine, media-clock, audio-output(null), audio-engine, media-types, codec-api, crate::open/protocol/capabilities/render/audio/source. All workspace-internal except serde/serde_json (declared, used for command_id_string + ABI wire family).
- Feature gates relevant to this part: none inside 1389-1954 (all decoder cfgs live in part1); the chunk compiles identically on wasm32 and native.

## 9. Open questions for the next agent
- open.rs:272-273 (`frame_ns = num*1e9/den`): `den == 0` would panic (u64 div-by-zero) before F-005's inf is even reached — open.rs belongs to another assignment; whoever audits it should check the demuxer's rate normalization.
- apps/cli/src/host.rs:863-864 carries the same F-001 swallowed-error defect — the CLI wave's audit of that file should cross-reference this finding.
- Does any committed doc claim the browser matrix passed? `docs/web/browser-matrix-20261110.md` exists — doc-audit agents should check it against the skipped baseline web stages.
