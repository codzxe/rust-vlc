# AUDIT: crates/codecs/tests/fixtures/vorbis_tagged_sine.ogg
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | binary (7564 bytes) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture; vorbis_bitexact.rs `[fixture.vorbis_tagged_sine]` lane + vorbis_vlc_differential/vorbis_fuzz_survival) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | file; ffmpeg native+f32le decode -> md5sum (both == reference pins); direct run of vorbis_bitexact suite -> 5/5 ok incl. the tag test |

## 1. One-paragraph verdict
The most deeply verified fixture of my batch (it is the subject of one of my other assignments, tests/data/vorbis_reference.txt, whose numbers I reproduced independently): a genuine Ogg/Vorbis stream ("Ogg data, Vorbis audio, stereo, 44100 Hz, ~128000 bps") carrying the documented comment metadata. My own ffmpeg reproduces BOTH recorded digests exactly — native `MD5=a3b5a3d6be1282de2f8b9f960bb1b7cc` and f32le `29179dc21a985b00ba9fa89bfbb0c266` (351776 bytes = 43972 stereo samples, the recorded window) — and the executed vorbis_bitexact suite passed 5/5 including `tagged_fixture_exposes_its_vorbis_comments` (vendor `Lavf61.7.103`, TITLE/ARTIST/ALBUM/DATE per the reference row) and the tagged lane's live max-delta 2.98e-8 within the recorded 1e-7 bound.

## 2. What actually works (verified)
- `ffmpeg -v error -i vorbis_tagged_sine.ogg -map 0:a -f md5 -` → `a3b5a3d6…` and `pcm_f32le/f32le | md5sum` → `29179dc2…` — both == vorbis_reference.txt:132-133 pins; f32le length 351776/8 = 43972 == `window_samples`.
- `./target/debug/deps/vorbis_bitexact-e8223d5cf8b5f437 --nocapture` → 5/5 ok; live `DIFF-ROW vorbis_tagged_sine.ogg: … lumen_window_md5=546fc5d9d0d8d211a767ddca59984c02 … max_delta=2.9802322e-8 (bound 1e-7)` — the reference pin `546fc5d9…` (:134) held byte-for-byte; `tagged_fixture_exposes_its_vorbis_comments ... ok`.
- Generation command documented in vorbis_reference.txt:14-18 (lavfi sine 330 Hz + the four metadata tags) — consistent with the tag test's assertions.

## 3. Findings
None. Checked: container/codec facts vs reference row (channels 2, rate 44100, packets 45 — asserted by the suite), both digests, window length, tag metadata (via the passing suite), consumers.

## 4-9. (binary asset — tables N/A; consuming suites' pins hold and were executed; no dead code; no deps; no open questions)
