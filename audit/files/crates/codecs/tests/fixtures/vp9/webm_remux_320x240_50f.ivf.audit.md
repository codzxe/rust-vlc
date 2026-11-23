# AUDIT: crates/codecs/tests/fixtures/vp9/webm_remux_320x240_50f.ivf (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| File verdict | REAL |
| Type / size | Duck IVF video, codec VP90, 320×240, 50 frames, 83223 bytes — `file` output |
| Structure (verified by my od walk) | `DKIF` magic, version 0, header len 0x20, fourcc `VP90`, width 0x0140=320, height 0x00f0=240, rate 1/1000, frame count 0x32=50 — every field matches the filename; first frame 7258 B at pts 0. |
| Cited as evidence by | tests/vp9_differential.rs:91 (`name: "webm_remux_320x240_50f"`) — the rusty_vp9 differential lane |
| Commands run | `file` → Duck IVF VP90 320x240 50 frames; `od` header dump (above); `./target/debug/deps/vp9_differential-4ec38b32b2dc3e15` → 4 passed (exit 0, 21.6 s — full decode differential) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
Genuine IVF/VP9 fixture; header fields byte-verified against the advertised name (320×240, 50 frames, VP90), and the consuming differential suite (in-house VP9 walk vs rusty_vp9) passes under my execution, decoding the full 50-frame stream. No findings.
