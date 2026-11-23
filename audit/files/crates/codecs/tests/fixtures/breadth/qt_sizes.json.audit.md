# AUDIT: crates/codecs/tests/fixtures/breadth/qt_sizes.json (binary asset / data)
| Field | Value |
|---|---|
| Auditor | W06-R2 (recovery; credit W06-A07) |
| Date | 2026-11-23 |
| Type / size / md5 / sha256 | JSON text (flat array) / 692 bytes / 386652c1fa48e274fcd5cdc90de236f8 / 8f047097676f744914f77f525ca6b7dfb7ebb1c2847c57251fa7414b960683f9 |
| Contents (my parse) | 173 entries, all exactly 68; sum 11,764 == qt_payload.bin size == 173 × 68 |
| File verdict | REAL — packet-shape pin, consistency-verified and exercised by the passing suite |
| Reachable from shipped entry point? | N/A (test data; adpcm_breadth_differential.rs:79-86 Case row `sizes: Some("qt_sizes.json")` + :305 geometry shape pin) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; python3 JSON parse + sum/min/max + cross-stat of sibling payload; suite `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` → 4 passed / 0 failed (1.19s, this session) |

## 1. One-paragraph verdict
The ffprobe-captured packet-size record for the `adpcm_ima_qt` stereo lane of the ADPCM breadth differential: a valid flat JSON array of 173 entries, every one exactly 68 bytes (uniform MOV block size — consistent with ffmpeg's MOV muxing of adpcm_ima_qt), summing to exactly the sibling `qt_payload.bin`'s 11,764 bytes and to the geometry test's pinned shape `("qt_payload.bin", 173, 68)` (adpcm_breadth_differential.rs:305). The consuming suite parses it (`parse_sizes`) and drives the decode packet-by-packet through those shapes, requiring the bit-exact MD5 `a2cb46ce5b9ad2da111403ad6195bf3b` — that suite ran 4/4 green this session on these exact bytes.

## 2. What actually works (verified)
- Geometry: 173 × 68 == 11,764 == payload size — the "sizes must tile the payload" assert (decode_case, :217-222) holds by construction and was exercised in the green run.
- `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b` → 4/4 (this session): the qt lane decoded 173 packets through the repo `AdpcmDecoder` (each `Emitted { frames: 1 }`, flush `Drained`) with the pinned MD5 asserted.
- Prebuilt-binary caveat as in the av1 fixture audits (cargo-mtime staleness; files read fresh from disk at runtime).

## 3. Findings
None. Checked: JSON validity, entry count vs pinned shape, uniform 68-byte block size vs MOV provenance, exact tiling of the sibling payload, consuming suite executed green, doc citations.

## 4. Provenance note (UNVERIFIED)
The reference MD5 for this lane was measured from the ORIGINAL MOV container (`MOV enc→dec`, worklog :9339) — the MOV itself is not committed, so the ffmpeg oracle for `a2cb46ce…` cannot be independently reproduced from in-tree artifacts (unlike the oki/ws forced-decoder lanes I reproduced live). The guard is the suite's MD5 pin re-proved on every run; provenance rests on the worklog record. UNVERIFIED: original-MOV oracle equality (container absent by design).

## 5. Doc citations
- adpcm_breadth_differential.rs:20 — row: "`adpcm_ima_qt` stereo 22 050 | `qt_payload.bin` | 173 × 68 | ffmpeg enc, MOV-carried (`-c:a adpcm_ima_qt`) | MOV demux → dec → s16le | `a2cb46ce5b9ad2da111403ad6195bf3b`" — shape fields re-verified here.
- docs/worklog/archive-2026-10.md:9332, :9339 — the measurement record (extraction via `-map 0:a -c:a copy -f data`, sizes via ffprobe).

## 6. Open questions for the next agent
- `qt_payload.bin` itself has no audit file yet (breadth dir still missing: qt_payload.bin, yamaha_mono_payload.bin, yamaha_mono_sizes.json) — presumably other recovery batches; flagging so nothing is dropped.

## 7-9. (data asset — tables N/A; consuming suite's pins hold; no dead code; no deps)
