# AUDIT: crates/codecs/tests/fixtures/breadth/oki_mono.vox (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-R2 (recovery; credit W06-A04) |
| Date | 2026-11-23 |
| Size / type | 512 bytes — `data` (raw payload; expected: hand-crafted OKI ADPCM ramp) |
| md5 / sha256 | d671b192ad55c6c6e3c1a7ddb4733fa2 / c029dfc944a023bec6662861a4e633237ad3e4f4bca787399fdd487ca52af8f5 |
| File verdict | REAL — hand-crafted `adpcm_ima_oki` mono 8 kHz ramp, bit-exact-pinned and live-reproduced against the ffmpeg 7.1.5 oracle |
| Reachable from shipped entry point? | N/A (test fixture; adpcm_breadth_differential.rs:88-93 Case row + :311-312 geometry pin) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `od -A d -t x1` (ramp visible); live ffmpeg oracle reproduction; `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` → 4 passed / 0 failed (1.19s) |

## 1. One-paragraph verdict
A 512-byte hand-crafted OKI (`adpcm_ima_oki`) mono payload whose documented provenance is visibly true: `od` shows the exact +7 arithmetic byte ramp (`00 07 0e 15 1c 23 2a 31 …`) — synthetic by construction, matching the suite's row "`adpcm_ima_oki` mono 8 000 | `oki_mono.vox` | 1 × 512 | hand-crafted ramp (no ffmpeg encoder exists)" (adpcm_breadth_differential.rs:21, od-verified). I independently reproduced the row's pinned reference MD5 with the sandbox's ffmpeg 7.1.5-0+deb13u1 using the documented forced-decoder invocation: `ffmpeg -c:a adpcm_ima_oki -f mulaw -ar 8000 -ac 1 -i oki_mono.vox -map 0:a -c:a pcm_s16le -f md5 -` → `MD5=60714fbfc3997f823b7b97dd90e958ed` — byte-exact equal to the `expected_md5` pinned at adpcm_breadth_differential.rs:93. The consuming suite executed green on these exact bytes.

## 2. What actually works (verified)
- Ramp provenance: od header = +7 byte ramp — "hand-crafted ramp" is literal, and "no ffmpeg encoder exists" is consistent (ffmpeg has no adpcm_ima_oki encoder; the row honestly uses the forced-decoder oracle instead).
- Oracle reproduction: pinned MD5 `60714fbfc3997f823b7b97dd90e958ed` == my live ffmpeg output (above) — the pin is fresh-truth, not stale.
- `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` → **4/4 pass** (1.19s): the bit-exact corpus lane decodes this payload through the repo `AdpcmDecoder` contract path (one 512-byte packet, `Emitted { frames: 1 }`, `Drained` flush — decode_case at :197-264) and asserts the MD5; the geometry lane pins `fixture(file).len() == 512` (:311-312).
- Prebuilt-binary caveat as in the av1 fixture audits (cargo-mtime staleness, av1/mod.rs F-001): fixture bytes are read from disk at runtime, so the run is current-bytes × binary-logic.

## 3. Findings
None. Checked: size vs pinned geometry, ramp pattern vs documented provenance, MD5 pin vs live ffmpeg 7.1.5 oracle (reproduced byte-exact), consuming suite executed green, doc citations.

## 4. Doc citations
- adpcm_breadth_differential.rs:21 (row: forced-decoder oracle, pinned MD5) — reproduced live.
- docs/worklog/archive-2026-10.md:9340: "| adpcm_ima_oki mono 8000 | oki_mono.vox (1×512) | forced decoder | 60714fbfc3997f823b7b97dd90e958ed |" — all four fields re-verified here.

## 5-9. (binary asset — tables N/A; consuming suite's pins hold; no dead code; no deps; no open questions)
