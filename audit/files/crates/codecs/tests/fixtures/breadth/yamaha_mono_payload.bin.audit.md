# AUDIT: crates/codecs/tests/fixtures/breadth/yamaha_mono_payload.bin (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-R2 (recovery; credit W06-A07) |
| Date | 2026-11-23 |
| Size / type | 18,432 bytes — `data` (raw payload; ffmpeg-authored `adpcm_yamaha` essence, extracted from its NUT carrier) |
| md5 / sha256 | e01c59bee9cdbc74638e929aa6d1ff8e / aac09834ead7cfe9cb610bb3a99183121341d883a89272baaf0dd64b9e232d84 |
| Geometry | 18,432 == 18 × 1024 == sum of the 18 all-1024 entries in sibling `yamaha_mono_sizes.json` (108 B, cross-parsed) |
| File verdict | REAL — `adpcm_yamaha` mono 22 050 Hz payload, bit-exact-pinned and independently reproduced against the live ffmpeg oracle |
| Reachable from shipped entry point? | N/A (test fixture; adpcm_breadth_differential.rs:46-52 Case row + :302 geometry shape pin) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; python3 sizes-JSON cross-parse; live ffmpeg forced-decoder oracle reproduction; suite `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` → 4 passed / 0 failed (1.19s, this session) |

## 1. One-paragraph verdict
The mono lane of the ffmpeg-authored Yamaha ADPCM payload pair: 18,432 bytes exactly tiling the documented "18 × 1024" packet shape (suite row :17; geometry pin `("yamaha_mono_payload.bin", 18, 1024)` at :302; the 18-entry all-1024 sizes sidecar sums to the payload byte-exactly — cross-parsed here even though the sidecar is a separate assignment). I independently reproduced the lane's pinned reference MD5 with the sandbox ffmpeg 7.1.5 via the raw forced-decoder path (`-c:a adpcm_yamaha -f mulaw -ar 22050 -ac 1`): `MD5=aeddc262b3dd010955f9c73c6cfc86ad` — byte-exact equal to `expected_md5` at adpcm_breadth_differential.rs:61, and distinct from the stereo lane's pin on different bytes. The consuming suite ran green on these exact bytes.

## 2. What actually works (verified)
- Oracle reproduction: pinned `aeddc262b3dd010955f9c73c6cfc86ad` == my live ffmpeg output (above). The suite's original oracle was the NUT-carried encode (worklog :9332), but the continuous-flavor forced-decoder path reproduces the identical digest — so this pin is re-provable from committed artifacts alone (unlike the qt lane).
- `./target/debug/deps/adpcm_breadth_differential-13f5db9dabd4b` → **4/4 pass** (this session): this payload decodes through the repo `AdpcmDecoder` as 18 packets × 1024 B driven by the sizes sidecar, each `Emitted { frames: 1 }`, flush `Drained`, MD5 asserted.
- Prebuilt-binary caveat as in the av1 fixture audits (cargo-mtime staleness; fixture bytes read fresh from disk at runtime).

## 3. Findings
None. Checked: size vs pinned geometry and vs the sizes sidecar sum, ffmpeg-authored provenance, MD5 pin vs live forced-decoder reproduction (byte-exact), consuming suite executed green, doc citations.

## 4. Doc citations
- adpcm_breadth_differential.rs:17 — row: "`adpcm_yamaha` mono 22 050 | `yamaha_mono_payload.bin` | 18 × 1024 | ffmpeg enc, NUT-carried (`-c:a adpcm_yamaha`) | NUT demux → dec → s16le | `aeddc262b3dd010955f9c73c6cfc86ad`" — shape + MD5 re-verified live.
- docs/worklog/archive-2026-10.md:9332 ff. — the measurement record (extraction `-map 0:a -c:a copy -f data`, sizes via ffprobe).

## 5-9. (binary asset — tables N/A; consuming suite's pins hold; no dead code; no deps; note: `yamaha_mono_sizes.json` itself remains an unaudited separate file — flagged in the qt_sizes.json audit for the orchestrator)
