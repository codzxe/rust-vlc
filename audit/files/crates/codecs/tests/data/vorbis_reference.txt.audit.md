# AUDIT: crates/codecs/tests/data/vorbis_reference.txt
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | 157 / reference-data file (5 sections: 4 fixtures + 1 corpus row) |
| File verdict | REAL (all 8 recorded digests + all 5 window geometries independently reproduced with my own ffmpeg; all 4 lumen_window_md5 pins + live tolerances verified by executing the consumer suite) |
| Reachable from shipped entry point? | N/A (test reference data; consumed by crates/codecs/tests/vorbis_bitexact.rs + vorbis_vlc_differential.rs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | ffmpeg -f md5 + pcm_f32le/f32le md5sum + wc -c on all four fixtures (8/8 digests + 4 byte-counts match); ./target/debug/deps/vorbis_bitexact-e8223d5cf8b5f437 --nocapture -> 5/5 ok EXIT=0 with live DIFF-ROW output; sed quote checks |

## 1. One-paragraph verdict
Reference data of unusual quality, and every number in it survives independent verification: my own ffmpeg reproduces all eight recorded digests exactly (native `-f md5` and f32le for each of the four fixtures), the f32le byte counts reproduce the recorded window geometries to the sample (66150, 9600, 47872 — including the documented ffmpeg tail-trim quirk — and 43972), and the executed consumer suite confirms the four `lumen_window_md5` pins byte-for-byte with live max-delta measurements (3.7e-8 / 4.5e-8 / 0.0 / 3.0e-8, all inside the recorded 1e-7 / 0 bounds). The file also documents its own history honestly: a FINISHER CORRECTION note admits a prior session recorded mis-aligned windows whose `max_delta = 1e-7` was "never actually measured (live delta was ~9e-2)" — a fabricated tolerance, caught and corrected by exhaustive alignment scan; my live run confirms the corrected values are real measurements. The silence lane's bit-exact claim (lumen MD5 == ffmpeg f32le MD5 after −0.0 canonicalization) is verified live (both 9f3a19b1…, delta 0e0).

## 2. What actually works (verified)
- Independent digests (my ffmpeg, this sandbox): sine_44100 native `MD5=1cf8c053595142580693d341b4114c20` / f32le `7e36fc4df7e261a2a743f543d8d9b80b`; sine_8000 `c6dbc283…` / `f60a54f4…`; silence `4c1f6822…` / `9f3a19b1…`; tagged `a3b5a3d6…` / `29179dc2…` — 8/8 EXACT matches with the file's `ffmpeg_md5_native` / `ffmpeg_md5_f32le`.
- Window geometry: f32le byte counts 529200/8=66150, 38400/4=9600, 382976/8=47872, 351776/8=43972 — each equals the recorded `window_samples`; the silence row's "ffmpeg decodes 47 872" quirk reproduces.
- Executed `./target/debug/deps/vorbis_bitexact-e8223d5cf8b5f437 --nocapture` → 5/5 ok, EXIT=0, with `DIFF-ROW … lumen_window_md5=658af0c3…` (== pin), `max_delta=3.7252903e-8 (bound 1e-7)`, silence row `max_delta=0e0` and identical MD5s, plus `HEADTRIM … emission 66150 == granule (delay 128 trimmed decoder-side)` — the 6.FIX re-pin verified live.
- Tagged-row metadata (vendor Lavf61.7.103, TITLE/ARTIST/ALBUM/DATE): covered by the passing `tagged_fixture_exposes_its_vorbis_comments` test in the same run.

## 3. Findings
None. Checked and how: all digests re-computed externally; window/sample arithmetic (raw − granule = delay + padding: 636 = 128+508 documented and consistent with head_trim 128 and window 44100−128=43972); the two-digest divergence explanation (planar fltp vs interleaved f32le) is sound and matches ffmpeg's actual output shapes; the corpus row's path documented (absent in this sandbox — the row's consumer is corpus-gated, standard skip); historical fabrication note verified as corrected (live deltas now match the recorded bounds).

## 4. Per-function table
N/A (data file).

## 5. Test honesty
The consuming suite's assertions are strong and EXECUTED here: exact sample counts vs ffmpeg, hard lumen_window_md5 regression pins, live per-sample max-delta bounds, head-trim decoder-side emission == granule, and the bit-exact silence corridor. Would fail if the decoder drifted (MD5 pin) or diverged from ffmpeg (live delta) — neither happened.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every value below was produced in the build sandbox … with the recorded commands" | :3-5 | REAL | my re-run of the recorded commands reproduces every digest/length |
| "the comparable differential base is the f32le digest" (two-digest explanation) | :25-29 | REAL | both digests reproduce; f32le lengths align with windows |
| FINISHER CORRECTION: prior windows mis-aligned, "max_delta = 1e-7 … never actually measured (live delta was ~9e-2)" | :44-51 | REAL (the correction) | my live run: deltas 3e-8..4e-8 within 1e-7 — the corrected values ARE measured |
| "The silence lane is the true bit-exact corridor … MD5s equal after -0.0 canonicalization" | :60-63 | REAL | live: both MD5s 9f3a19b1740ac999088756035046c146, delta 0e0 |
| silence: "ffmpeg decodes 47 872 samples here (one 128-sample final block short)" | :109-111 | REAL | my ffmpeg: 382976 B / 8 = 47872 samples |

## 7. Dead / duplicated / unreachable code
The `[corpus.0085-audio-only]` row is inert in this sandbox (corpus root absent — same standard skip pattern as the other corpus-gated suites; the row documents its own path at :143-144). Not dead — activated when the corpus is imaged.

## 8. Dependencies & flags
Data only. Companions: the four .ogg fixtures (vorbis_tagged_sine.ogg is separately in my assignment — audited next) and the vorbis_bitexact/vorbis_vlc_differential suites.

## 9. Open questions for the next agent
- The FINISHER CORRECTION note is evidence that this lane once shipped fabricated tolerance data — worth a line in any aggregate fabrication report (it was self-caught, but the pattern matters for trust calibration of `max_delta` values elsewhere recorded the same way).
