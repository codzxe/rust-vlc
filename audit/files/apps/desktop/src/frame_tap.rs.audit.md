# apps/desktop/src/frame_tap.rs — the presented-frame tap (W13-A02)

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A02 |
| File | `apps/desktop/src/frame_tap.rs` (233 LOC; 134 production / 99 test) |
| Reachability | lumen-tauri binary → `ShellRuntime::bind_frame_channel` (shell.rs:175-187) constructs the tap and `set_present_tap(tap)` on the CliHost → the engine's presenter calls `tap_present` per successful present on the pump thread → the wrapper's ticker drains via `ShellRuntime::take_frame_records` (shell.rs:219-224) → LUF1 records → `frame_channel.rs::send_record`. |
| Verdict | **Real. This is the SEE-path's engine-side seam, and it is correctly placed: the tap mirrors the same presents the CLI renderer receives (not a second decode).** Committed tests prove RGBA/BGRA records with the LUF1 magic and latest-wins bounding. |

## Per-function table

| Function | Lines | What it does (sed-verified) |
|---|---|---|
| `FrameTap::new` | 56-61 | `CanvasBridge` over `MailboxTransport` behind one `Mutex` (mailbox default capacity 2 — one frame in flight plus one landing). |
| `drain_records` | 72-89 | locks (poison → `into_inner`, frames keep flowing past a presenter bug, lines 75-80); `mailbox.draw_next()` loop → `encode_record` → appended oldest-first. |
| `stats` | 92-97 | bridge accounting (offered/drawn/dropped) — diagnostics. |
| `tap_present` | 107-118 | `PresentTap` impl — `present`/`present_repeat` into the bridge; called only after a SUCCESSFUL present ("a stale or failed present never taps", lines 16-17). |
| `encode_record` | 125-134 | `encode_into(&frame.header, &frame.payload, &mut record)` — geometry/length inconsistency is SKIPPED, never shipped (lines 121-124). |

## The I420→RGBA answer (trace question)

The tap receives `PresentRequest` frames already in the host's RGBA vocabulary. The conversion lives in the CLI host, not here and not in the UI:
- Sed-verified `apps/cli/src/pipeline_host.rs:1298-1299`:
  ```
  media_types::PixelFormat::Rgba8 | media_types::PixelFormat::Bgra8 => frame.clone(),
  media_types::PixelFormat::I420 => convert_i420_to_rgba(frame)?,
  ```
- `convert_i420_to_rgba` (pipeline_host.rs:1432-1440+) converts under the frame's own tagged colorimetry — the four landed combinations BT.601/BT.709 × full/limited (BT.2020 refuses typed). Frame_tap's own test comment pins the contract (lines 147-149): "host-converted RGBA; the bridge's format table is RGBA/BGRA only, the YUV family is the GPU shader path's."
- The wire table accepts only RGBA8 (code 1) / BGRA8 (code 2) — `frame-channel.ts:49-51` and `bridge.rs:201-224` (`FormatCode::for_pixel_format`, refusing with `render.stream.format-unsupported`, bridge.rs:252).
- The UI does the LAST quarter-mile: BGRA8 records are swizzled B↔R in JS before `putImageData` (MediaSurface.tsx:215-221). RGBA8 crosses untouched.

So: **I420→RGBA = `apps/cli/src/pipeline_host.rs::convert_i420_to_rgba`, engine-side, before the scheduler; the UI never sees YUV.**

## Findings

- **S3 — latest-wins drop is uncounted at the ship layer.** The mailbox counts its own drops (`MailboxStats`), and the ticker's "ship newest, drop the rest" decimation (app.rs:221, `records.last()`) silently discards every drained-but-unshipped record with NO counter anywhere on that path. The compat-matrix "dropped frames" cell (mailbox.rs:21-23 promises "measured arithmetic") does not cover the ticker decimation. Frame loss is real by design and unobservable in the shipped path.
- **S4 — capacity-2 mailbox** (frame_tap.rs:50-51): two presents between ticks (e.g., a repeat + a fresh frame) always lose the older one at the mailbox itself. Consistent with the contract; noted because the decimation compounds with it.
- **S4 — `Default` for `FrameTap` uses `unreachable!`** (line 102) — same impossible-arm convention as shell.rs.

## Test honesty

Tests are real and pin the wire contract: `tapped_frames_drain_as_luf1_records` (175-195), `a_full_mailbox_drops_oldest_never_blocks` (201-215 — 7 offers, 2 survive, pump never blocked), `repeat_taps_ship_the_repeat_flag` (220-232 — decoded through the crate's own `WireDecoder`). All drive the real `CanvasBridge`/`MailboxTransport`; no mocks.

## Doc claims

- Module doc lines 23-29 state the one-frame-per-tick contract honestly ("the ticker ships the newest and drops the rest"). Matches app.rs:221.
- Worklog w31.1 "the pacer ticker ships the newest LUF1 record per tick as a raw binary channel message" — matches.

## Dead code / deps / flags

- `stats()` — is it consumed? `rg -n "\.stats\(\)" apps/desktop-tauri/src apps/desktop/src/frame_tap.rs` shows test usage only in the bridge/mailbox modules' own tests; the shipped ticker never reads `FrameTap::stats`. S4 dead-in-production diagnostic API.

## Open questions

1. With the tap installed but the webview absent (browser mode), `tap_present` still pays the encode + mailbox cost on the pump thread — no, the tap is installed only on bind (`bind_frame_channel`), and in-browser the UI never binds. Confirmed no cost when unbound. No open question stands beyond the S3 counting gap.
