# AUDIT: crates/demux/src/asf/fixtures/wmv_wmv2_wma_av.wmv (binary asset)
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-23 |
| File verdict | REAL (genuine fixture, pinned, exercised, oracle-documented) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

- `file` -> "Microsoft ASF" (correct for a .wmv). Size 10,499 bytes; sha256 `5bd8c76fa14e31aa2f2ddcfaa7acfd84394419e1316c809d959335b8129d44be` (computed); md5 `6b8abacf467c3c9edef15e7bde2cd41b`.
- Digest pin verified: crates/demux/src/asf/tests.rs:1492 `"wmv_wmv2_wma_av.wmv" => "5bd8c76fa14e31aa"` — first 16 hex of my computed sha256 MATCH. Also pinned in docs/plans/asf-selfbuild.md:457 (`| wmv_wmv2_wma_av.wmv | 10,499 | 5bd8c76fa14e31aa | A/V multi-payload |`) — size matches.
- Tests referencing it (rg): asf/tests.rs FixtureTruth row at 1003-1022 — the A/V multi-payload shape: 2 tracks (wmv2 video 64x48 + wmav2 audio, first_pts [64, 0]), header 757 B, Data Object (757, 9650, 3), Simple Index Object (10_407, 92 entries), VIDEOINFOHEADER + WAVEFORMATEX facts; driven by `fixture_family_walks_match_the_measured_truth` (sha asserted at 1138) and the live-ffprobe cross-check (tests.rs:1333+). Baseline: line 1780 `... ok`.
- Doc/matrix evidence: generation command at asf-selfbuild.md:64; the ffprobe ORACLE note (selfbuild.md:82-84) documents this exact file's differential shape — "26 payloads with pos values 807/4007/7207 — 26 demuxed payloads across 3 container packets of exactly 3200 bytes each" (multi-payload packets), plus byte-layout tables at :119 and :148. The feature-matrix `container-asf-wmv` row (docs/feature-matrix.toml:207-217) cites `crates/demux/src/asf/**`; its notes record the linux open path REFUSING asf/wmv (probe.no-demuxer — no ASF probe signature), so this fixture backs the unit-level container claim only, not an end-to-end player run.
- Not referenced by fuzz targets or integration tests (rg: only asf/tests.rs + asf-selfbuild.md).
