# AUDIT: crates/codecs/tests/fixtures/breadth/ws_mono.bin (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-R2 (recovery; credit W06-A04) |
| Date | 2026-11-23 |
| Size / type | 512 bytes — `data` (raw payload; expected: hand-crafted Westwood ADPCM ramp) |
| md5 / sha256 | d671b192ad55c6c6e3c1a7ddb4733fa2 / c029dfc944a023bec6662861a4e633237ad3e4f4bca787399fdd487ca52af8f5 |
| File verdict | REAL — hand-crafted `adpcm_ima_ws` mono 22 050 Hz ramp, bit-exact-pinned and live-reproduced against the ffmpeg 7.1.5 oracle |
| Reachable from shipped entry point? | N/A (test fixture; adpcm_breadth_differential.rs:103-110 Case row + :311-312 geometry pin) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `od -A d -t x1`; `cmp` vs sibling ramps; live ffmpeg oracle reproduction; suite `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` → 4 passed / 0 failed (1.19s, this session) |

## 1. One-paragraph verdict
A 512-byte hand-crafted Westwood (`adpcm_ima_ws`) mono payload whose documented provenance is visibly true: `od` shows the exact +7 arithmetic byte ramp (`00 07 0e 15 1c 23 2a 31 …`), matching the suite's row "`adpcm_ima_ws` mono 22 050 | `ws_mono.bin` | 1 × 512 | hand-crafted ramp (encoder exists, no muxer carries it) | forced decoder `-c:a adpcm_ima_ws -f mulaw -ar 22050 -ac 1`" (adpcm_breadth_differential.rs:23, od-verified). I independently reproduced the pinned reference MD5 with the sandbox ffmpeg 7.1.5: `ffmpeg -c:a adpcm_ima_ws -f mulaw -ar 22050 -ac 1 -i ws_mono.bin -map 0:a -c:a pcm_s16le -f md5 -` → `MD5=cbbfd8928ee6a5b2c019330d3d5ff7fb` — byte-exact equal to `expected_md5` at adpcm_breadth_differential.rs:109. The consuming suite ran green on these exact bytes (4/4, this session — the bit-exact corpus lane decodes this payload through the repo `AdpcmDecoder` contract path and asserts that MD5; the geometry lane pins len == 512 at :311-312).

## 2. What actually works (verified)
- Ramp provenance + honest "encoder exists, no muxer carries it" rationale for the forced-decoder oracle path (suite header :23, :26-31 — mulaw raw demuxer as pure byte carrier, single 512-byte packet).
- Oracle reproduction: pinned `cbbfd8928ee6a5b2c019330d3d5ff7fb` == my live ffmpeg 7.1.5 output (above).
- Suite execution: 4/4 pass (bit-exact corpus incl. this lane, geometry 512-pin, fuzz-lite, RFC-1321 MD5 self-check). Prebuilt-binary caveat as in the av1 fixture audits (cargo-mtime staleness; fixture bytes read fresh from disk).

## 3. Findings
None. Checked: size vs pinned geometry, ramp pattern vs documented provenance, MD5 pin vs live ffmpeg oracle (reproduced byte-exact), consuming suite executed green, doc citations.

## 4. Observations
- `cmp` proves this file is BYTE-IDENTICAL to oki_mono.vox, oki_st.vox and ws_st.bin (all four 512-byte ramp fixtures share md5 d671b192ad55c6c6e3c1a7ddb4733fa2). The suite never claims the payloads differ; the four lanes' discrimination lives entirely in the decode configuration (flavor × channels), and the four distinct pinned PCM MD5s prove the decoders do NOT confuse them. Same-carrier-different-interpretation is a legitimate differential design — recorded here because W06-A06's ws_st.bin audit did not note the identity.

## 5. Doc citations
- adpcm_breadth_differential.rs:23 (row, oracle invocation, pinned MD5) — reproduced live.
- docs/worklog/archive-2026-10.md:9342: "| adpcm_ima_ws mono 22050 | ws_mono.bin (1×512) | forced decoder | cbbfd8928ee6a5b2c019330d3d5ff7fb |" — all four fields re-verified here.

## 6-9. (binary asset — tables N/A; consuming suite's pins hold; no dead code; no deps; no open questions)
