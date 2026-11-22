# Wave 0 — Baseline reality runs (orchestrator, 2026-11-22)

Host: 2 vCPU, 3.9 GB RAM, Linux sandbox. Rust 1.99.1 (matches rust-toolchain.toml pin).

| Baseline command | Result | Evidence |
|---|---|---|
| cargo build --release --locked -p lumen-cli -p desktop (release.yml exact cmd) | PASS — 111 crates, 2m31s | build-release.log |
| cargo test --workspace --locked | 1751 passed / 2 FAILED / 25 ignored | cargo-test.log |
| failing tests | first-vertical `evidence`: committed transcript + final-frame.png goldens DRIFTED from fresh runs | cargo-test.log |
| cargo fmt --check | FAIL — 2 files drift | cargo-fmt-check.log |
| cargo xtask feature-audit | FAILURE — 252 rows; 193 "implemented" cells; tool checks PATH EXISTENCE ONLY; 2 evidence paths missing | xtask-feature-audit.log |
| cargo xtask parity-report | 0/252 verified (0.0%) all platforms; unverified parity windows=66 linux=61 macos=66 web=0 | xtask-parity-report.log |
| cargo xtask config-validator | FAILURE — 4 violations (brand icon assets missing) | xtask-config-validator.log |
| cargo clippy --workspace --all-targets -D warnings | NOT RUN YET (disk/time) | — |
| UI (vitest/typecheck/build) | NOT RUN YET | — |
| scripts/full_gate.sh, rc-smoke.sh | NOT RUN YET | — |

## MP4 smoke (the litmus test) — sample: ffmpeg testsrc 320x240 h264+aac 3s

```
$ ./target/release/lumen open sample-h264-aac.mp4   (then `time` after 8 s)
ok: accepted session=1 gen=0
event: state-changed Idle → Opening
event: tracks 2 ready            (h264 video + aac audio identified)
event: TimingDurationKnown { duration: Timestamp(3000000) }
event: state-changed Buffering → Playing
ok: time 0.000000s (0.0%)        <-- CLOCK NEVER ADVANCES

$ ./target/release/lumen decode-bench sample-h264-aac.mp4
bench: video track#0 h264 — not benched (no decoder in this build (first vertical: mjpeg + pcm))
bench: audio track#1 aac — not benched (no decoder in this build (first vertical: mjpeg + pcm))
BENCH ... v_frames=0 v_decoded=0 a_frames=0 a_decoded=0

$ ./target/release/lumen decode-bench sample-hevc-aac.mp4  → same: no decoder
$ ./target/release/lumen decode-bench sample-aac-only.mp4  → same: no decoder

$ ./target/release/lumen decode-bench sample-mjpeg-pcm.avi
BENCH ... v_frames=75 v_decoded=75 a_frames=130 a_decoded=130 wall_s=0.047  <-- REAL

$ ./target/release/lumen decode-bench sample-mjpeg-pcm.mov (MP4 container)
BENCH ... v_frames=75 v_decoded=75 a_frames=130 a_decoded=130  <-- MP4 demux REAL for mjpeg/pcm

$ interactive session on mjpeg AVI: time 0.000000s (0.0%) after 5 s of "Playing"
$ after `advance 200`/`advance 500` ticks: state Playing → Ended, time 3.000000s (100.0%)
```

**Baseline verdict:** the MP4 container demux path is REAL (tracks/duration/seekable),
but (a) the shipped build contains NO H.264/HEVC/AAC decoders — "first vertical: mjpeg + pcm"
is the binary's own admission — and (b) the interactive session's wall-clock pump never
advances playback; only manual `advance` ticks drive the engine to EOF. An MP4 therefore
"opens" and then plays nothing, forever.

## Honesty counters (git-tracked *.rs, excluding audit/)
unwrap(: 26 lines | expect(: 45 | todo!: 8 | unimplemented!: 6 | panic!: 30
unreachable! mentions: 4154 lines (incl. comments/strings) | unsafe { blocks: ~23 files
(codecs FFI + platform-* + tools) | #[ignore: 0 | #[allow: 0
