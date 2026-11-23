# AUDIT: crates/codecs/tests/fixtures/breadth/oki_st.vox (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-R2 (recovery; credit W06-A07) |
| Date | 2026-11-23 |
| Size / type | 512 bytes — `data` (raw payload; expected: hand-crafted OKI ADPCM ramp, stereo lane) |
| md5 / sha256 | d671b192ad55c6c6e3c1a7ddb4733fa2 / c029dfc944a023bec6662861a4e633237ad3e4f4bca787399fdd487ca52af8f5 |
| File verdict | REAL — hand-crafted `adpcm_ima_oki` stereo 8 kHz ramp, bit-exact-pinned and live-reproduced against the ffmpeg 7.1.5 oracle |
| Reachable from shipped entry point? | N/A (test fixture; adpcm_breadth_differential.rs:95-102 Case row + :311-312 geometry pin) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `cmp` vs sibling ramps (byte-identical, see observation); live ffmpeg stereo-oracle reproduction; suite `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` → 4 passed / 0 failed (1.19s, this session) |

## 1. One-paragraph verdict
The stereo lane of the hand-crafted OKI (`adpcm_ima_oki`) ramp pair: 512 bytes opening with the exact +7 arithmetic byte ramp (`00 07 0e 15 1c 23 2a 31 …` — od-verified in the oki_mono audit of the identical bytes), matching the suite's row "`adpcm_ima_oki` stereo 8 000 | `oki_st.vox` | 1 × 512 | hand-crafted ramp | forced decoder, `-ac 2` | `bad7f93b285386ba34d2e30ec6d6e5f3`" (adpcm_breadth_differential.rs:22). I reproduced the pinned stereo reference MD5 live with the sandbox ffmpeg 7.1.5: `ffmpeg -c:a adpcm_ima_oki -f mulaw -ar 8000 -ac 2 -i oki_st.vox -map 0:a -c:a pcm_s16le -f md5 -` → `MD5=bad7f93b285386ba34d2e30ec6d6e5f3` — byte-exact equal to `expected_md5` at :101. The consuming suite ran green on these exact bytes.

## 2. What actually works (verified)
- Oracle reproduction: pinned `bad7f93b285386ba34d2e30ec6d6e5f3` == my live ffmpeg output (above) — and it DIFFERS from the mono lane's `60714f…` on the same 512 carrier bytes, proving the stereo nibble-interleaving path is genuinely distinct.
- `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` → **4/4 pass** (1.19s, this session): bit-exact corpus lane (this payload decoded through the repo `AdpcmDecoder` with `channels: 2`, one 512-byte packet, `Emitted { frames: 1 }`, `Drained` flush) + geometry lane pinning `len == 512` (:311-312).
- Prebuilt-binary caveat as in the av1 fixture audits (cargo-mtime staleness; fixture bytes read fresh from disk at runtime).

## 3. Findings
None. Checked: size vs pinned geometry, ramp pattern vs documented provenance, stereo-oracle MD5 pin vs live ffmpeg reproduction (byte-exact, distinct from mono pin), consuming suite executed green, doc citations.

## 4. Observations
- `cmp` proves this file is BYTE-IDENTICAL to oki_mono.vox, ws_mono.bin and ws_st.bin (all four 512-byte ramps share md5 d671b192ad55c6c6e3c1a7ddb4733fa2; recorded in the ws_mono.bin audit). The suite never claims the payloads differ; the mono/stereo discrimination lives in the decode configuration, and the four distinct pinned PCM MD5s prove the decoders do not confuse the lanes.

## 5. Doc citations
- adpcm_breadth_differential.rs:22 (row: forced decoder `-ac 2`, pinned MD5) — reproduced live.
- docs/worklog/archive-2026-10.md:9341 — "| adpcm_ima_oki stereo 8000 | oki_st.vox (1×512) | forced decoder | bad7f93b285386ba34d2e30ec6d6e5f3 |" — all four fields re-verified here.

## 6-9. (binary asset — tables N/A; consuming suite's pins hold; no dead code; no deps; no open questions)
