# Wave 0 — Orchestrator pre-flight (completed 2026-11-16, branch audit/devil-20261116)

All results below are from real commands run on this host (Linux 5.10, 2 CPU, 4 GB RAM,
rustc 1.99.1 matching `rust-toolchain.toml`). Raw logs: `audit/runs/baseline/*`.
Host-limited items are marked UNVERIFIED with the reason.

## 1. Tree inventory
- `manifest.tsv`: 3,226 tracked files, 827,362 LOC textual; 444,647 Rust LOC (979 files),
  79,640 TS/TSX LOC (464 files), 444 markdown, 155 toml, 30 shell.
- `assignments.tsv`: every file assigned to exactly one (wave, agent) slot; coverage proof
  PASS (`coverage-proof.txt`: missing=[] extra=[] double=[]).
- Environment deviation: tokei/scc unavailable → LOC counted with Python (`wc`-equivalent byte/line
  counts); idiom counts by string scan (`audit/runs/baseline/idiom-counts.tsv`):
  unwrap(=26, expect(=45, todo!=8, unimplemented!=6, unsafe=738, #[ignore=66, #[allow=121, panic!(=13.

## 2. Baseline reality runs (exit codes and outputs in `audit/runs/baseline/`)
| Command | Result |
|---|---|
| `cargo build --release --locked -p lumen-cli -p desktop` (release.yml lanes) | PASS, 3m23s (`-j 1`, OOM at default parallelism on 4 GB host) |
| `cargo test --workspace --locked -j 1` | **74 suites, 1001 passed, 2 FAILED, 20 ignored** — failures in `control-server` (`the_catalog_lists_all_64_commands_from_the_live_spec`: catalog reports 65, test pins 64) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | CLEAN (exit 0, 1m27s) |
| `cargo fmt --all --check` | CLEAN |
| `cargo deny check` | **advisories FAILED** — yanked crates pinned in Cargo.lock: `wasmi 2.0.0`, `wasmi_core 2.0.0` (plugin-host-wasm tree); bans/licenses/sources ok |
| `cargo xtask feature-audit` | **FAILURE, 2 R4 violations** (evidence cites missing paths `crates/codecs/src/video/h264/sw`, `ui/src/screens/settings/areas/advanced.tsx`); 252 rows; platform cells: implemented=193, partial=147, blocked=129, not_started=503, unsupported=36 |
| `cargo xtask parity-report` | TOTAL row prints **0/252 (0.0%)** for windows/linux/macos/web; summary line contradicts: `windows=66/252 linux=61/252 macos=66/252 web=0/252` (discrepancy to be audited in Wave 1) |
| `cargo xtask config-validator` | **FAILURE, 4 violations** (brand.icon_ico/icon_icns/icon_png_512/logo_svg missing) — matches docs claim "RED with 4 violations" |
| `cargo xtask check-architecture` | **FAILURE (edges=fail)**, exit 1 |
| `cargo xtask lint-literals` / `check-identity` / `fixtures --check` | PASS (0 violations; 512/513 allowlisted; 55 fixtures no drift) |
| UI `npm run typecheck` | PASS |
| UI `npm run test` (vitest) | **136 files, 1705 passed, 1 skipped** (149.8s) |
| UI `npm run build` (tsc + vite) | PASS (exit 0) |
| `scripts/full_gate.sh` | components run individually (above); script-level run deferred — see `runs/baseline/NOTES.md` |
| `scripts/rc-smoke.sh` (fixture = the MP4 litmus sample) | **FAIL** (see below) |
| Playwright e2e / visual | UNVERIFIED — chromium not installed in this sandbox; not attempted |

## 3. MP4 smoke (the litmus test) — verbatim
`ffmpeg -f lavfi -i testsrc=duration=3:size=320x240:rate=25 -f lavfi -i sine=duration=3 -c:v libx264 -pix_fmt yuv420p -c:a aac -shortest sample.mp4`
→ H.264 320x240 + AAC, duration 3.000000, 45,682 bytes (`audit/runs/baseline/sample-h264-aac.mp4`).

`$ ./target/release/lumen decode-bench audit/runs/baseline/sample-h264-aac.mp4`
```
bench: video track#0 h264 — not benched (no decoder in this build (first vertical: mjpeg + pcm)); audio track#1 aac — not benched (no decoder in this build (first vertical: mjpeg + pcm)) (container mp4)
BENCH file=audit/runs/baseline/sample-h264-aac.mp4 bytes=45682 v_frames=0 v_decoded=0 a_frames=0 a_decoded=0 wall_s=0.000 decode_s=0.000
EXIT=0
```

`$ printf 'quit\n' | ./target/release/lumen open audit/runs/baseline/sample-h264-aac.mp4`
```
ok: accepted session=1 gen=0
event: state-changed Idle → Opening
event: tracks 2 ready
event: state-changed Opening → Buffering
event: TimingDurationKnown { duration: Timestamp(3000000) }
event: MediaInfoReady { ... tracks: h264 (Video), aac (Audio) ... seekable: true }
event: PlaybackEndOfTrack { track_kind: Video, id: TrackId(0) }
event: buffering 1.00
event: state-changed Buffering → Playing
event: state-changed Playing → Stopped
event: state-changed Stopped → Idle
EXIT=0
```

**Reading:** the MP4 container demuxes (tracks, duration, seekable flag all real). The video
track hits `PlaybackEndOfTrack` with zero frames decoded; the shipped build has NO H.264 and
NO AAC decoder ("first vertical: mjpeg + pcm" — engine-info self-report). The session still
reaches `Playing → Stopped` and exits 0: a silent no-op playback. `rc-smoke.sh` on this
fixture: FAIL (headless-decode: decoded counts below container counts; wasm-web-build
failed for want of the wasm32 target on this host; web checks skipped engine-only).

## 4. Sharding
20 waves × 10 agents, LOC-balanced round-robin per domain; 181 slots used (W18 matrix
handled as one agent + W19 docs; W20 synthesis per §8.1). Files > 800 LOC flagged in
`assignments.tsv` for chunked function-boundary audits.

## 5. Coverage proof
`coverage-proof.txt`: manifest_files=3226, assigned=3226, missing=[], extra=[],
double_assigned=[] → PASS.

## 6. Environment deviations (recorded per §7.4)
- Sandbox recycles killed background builds between agent turns; all runs were re-executed
  with in-call polling. Permission bits of the working tree were altered by a sandbox
  recycle (0644→0755, zero content change); `core.fileMode=false` set so no permission noise
  is committed. Nothing outside `audit/` is committed (git status shows only `?? audit/`).
- cargo/clippy unavailable at start → installed rust 1.99.1 per `rust-toolchain.toml`.
- `cargo test`/`clippy` run at `-j 1` after an OOM kill at default parallelism (4 GB host).
- Chromium/Playwright not present → UI e2e/visual UNVERIFIED.
- Sandbox date: 2026-11-16 UTC (owner timezone Asia/Calcutta).
