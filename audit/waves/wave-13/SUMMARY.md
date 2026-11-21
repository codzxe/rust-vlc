# WAVE 13 — SUMMARY (apps: cli, desktop, control-server)

Date: 2026-11-21 (session 2) · Agents W13-A01…A10 (A05's final message was cut
but all files verified on disk) · Domain: apps/cli (main, dispatch, parse,
demux_host, host, bench, help, exit), apps/desktop (bridge, LUF1 wire, host,
main), apps/control-server.
Gate: **PASS — 57/57 assignments audited, 0 missing, 0 thin, citation
spot-checks clean (11 checked).**

## Verdicts (file level)
REAL 54 · UNREACHABLE 2 · REAL/REAL-UNPROVEN 1

## Findings
**S0:0 S1:2 S2:27 S3:67 S4:113 — total 209**

## The wave's hostile question: argv → open → demux → decode → render
The full chain is now traced and REAL for the headless CLI (4:2:0 H.264+AAC
MP4 → Playing → exit 0). Where MP4 falls out, consolidated with W05/W06:
1. **4:4:4 H.264** → `decode.unsupported` AND the process hangs forever
   (never exits; consumes stdin after Error) — plus `open(bad)→close→
   open(good)` bricks the process via stale `last_fatal` (A08, CLI twin of
   W12-A05, worse: engine session leaked).
2. **In-session MP4 seek** — broken (W05: reanchor unimplemented).
3. **FLAC-in-Ogg extra_data** ships the raw 38-byte packet into a decoder
   parsing the 34-byte STREAMINFO — every field shifted 4 bytes; zero
   fixtures (A03; engine-side twin of W07-A10's finding).
4. **MKV A_VORBIS lacing count** misread (byte0 = "length-byte count" vs
   spec "headers−1") — spec-valid files accepted by ffprobe fail to open,
   EXIT 65, proven live against an identical control that plays (A09).
5. **Audio** → null discard sink (W08).

## Worst findings
1. [S1, EXECUTED] Shipped `lumen` panics (exit 101) on non-UTF-8 filenames
   via `env::args()` at main.rs:14 — proof captured in
   audit/runs/W13-A04-argv-panic.txt (A04).
2. [S1] ts.rs:354-377 ADTS AU-split loop lacks the demux crate's
   `frame_length ≥ header_len` check — crafted aac-typed PES infinite-loops,
   hanging `lumen open` with unbounded queue growth (A06).
3. [S2] pipeline_host.rs:1383 window-full hold stashes the UNCONVERTED I420
   original — hold-retried H.264 frames refused by the renderer (staging.rs:275)
   and wedge the presentation queue (A07).
4. [S2] Doc 05 §5.3's "stable" Top-30 CLI table is ~60% fiction — 18
   documented verbs plus `--rate` refuse as `unknown command` in the shipped
   grammar (A01).
5. [S2] decode-bench exits 0 with zero-frame BENCH lines (fake parity wins in
   speed_bench.py) + the false "no decoder in this build" banner (A04).
6. [S2] engine_info() frozen literal reports 2 decoders while 12+ real ones
   ship — "the lie that burned Wave-0" (A10, dispatch.rs:139).
7. [S2] es.rs doc claims raw-h264 refuses decode — FALSE; the corpus test
   asserting the refusal would PANIC if run; it "passes" only by silently
   skipping (A02).

## Blocked / UNVERIFIED
- lumen-tauri binary never built in sandbox (gtk/webkit) — standing
  UNVERIFIED-BUILD; `apps/desktop-tauri/src/app.rs` flagged as likely hang
  site (A06) — Wave 14's first stop.
- corpus-v4 absent → mkv/es/adts corpus-gated tests vacuous in all recorded
  runs (multiple agents).
