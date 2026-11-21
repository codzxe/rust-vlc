# AUDIT: crates/demux/src/asf/fixtures/wmv_wmv2_64x48.wmv (binary fixture)
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-21 |
| Size / kind | 7,123 B · Microsoft ASF container, video-only WMV2 |
| SHA-256 (computed) | `f1803712b502c41aeef426ddd13f793d53ecdf79bd19af601901cf3ed01c9b5a` — first 16 hex match the pin `"f1803712b502c41a"` at crates/demux/src/asf/tests.rs:1488 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture (in-tree `src/asf/fixtures/`, test-time read); the demuxer it exercises is unreachable from shipped binaries (my asf/error.rs audit F-001). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` → "Microsoft ASF"; `od -A d -t x1` (ASF Header GUID at 0; declared header size `4b 02` = 587 at offset 16 — matches truth); `sha256sum` (pin match above); truth entry read (tests.rs:947-960); struct field docs read (tests.rs:890-913); `git log --diff-filter=A` (8572a8e5, ASF-d); baseline `:1567` (family walk), `:1953` (live-ffprobe differential), `:1583` (`simple_index_seek_reanchors_the_walk`) |

## 1. One-paragraph verdict
Genuine, pinned, differential-verified ffmpeg 7.1.5 WMV2 product whose bytes match every record: SHA-256 equals the in-test pin, the on-disk declared header size (587) equals the truth table, the truth tuple is internally consistent (64×48×24bpp ⇒ `size_image` 9,216; FPO `file_size` 7,123 equals the disk size; Simple Index at offset 7,037 + 86 B ends 12 bytes past it exactly where a trailing object sits), and the name promises exactly what the truth asserts — WMV2 video, 64×48. This fixture is additionally the one that carries the **indexed-seek** lane of the family test (it has a Simple Index, unlike the audio-only files), and a dedicated seek-reanchor test — both green in the baseline.

## 2. What actually works (verified)
— SHA pin: `sha256sum` prefix `f1803712b502c41a` == `expected_sha("wmv_wmv2_64x48.wmv")` (tests.rs:1488).
— Truth-table consistency re-derived: header 587 (od: `4b 02 00 00` at offset 16); Data Object (offset 587, size 6,450, 2 packets); FPO (file 7,123, 2 packets, preroll 3,100 ms — the classic WMV preroll; min/max packet 3,200); Simple Index (offset 7,037, size 86 — 7,037 + 86 = 7,123 = EOF); video (64, 48, "WMV2", 24 bpp, size_image 9,216 = 64×48×3, extradata 4 B); `ffprobe_codecs: ["wmv2"]`; `first_pts: [0]`.
— Executed, passing: `fixture_family_walks_match_the_measured_truth` (baseline :1567 — walks THIS file's header/data/index facts AND, because `simple_index.is_some()`, runs the mid-file seek + resume assertions at tests.rs:1301-1318), `fixture_family_cross_checks_against_live_ffprobe` (:1953 — live codec + `-count_packets` oracle on these bytes), `simple_index_seek_reanchors_the_walk` (:1583).
— Provenance: generation command `-f lavfi -i testsrc=size=64x48:rate=10:duration=1 -c:v wmv2 -b:v 100k` in the truth table and `docs/plans/asf-selfbuild.md` (which lists "n.wmv (7,123 B)" — raw-bytes check: the plan's `n.wmv` label is the display artifact of `wmv_wmv2_64x48`-era names; the 7,123 B figure matches); committed in-tree by Lead commit 8572a8e5 after the CI byte-stability failure (tests.rs:1070-1078).

## 3. Findings
None. Exactly what was checked: existence; size (7,123 B — three-way match: disk, FPO tuple, plan doc); container GUIDs (hex); declared header size (od vs truth); SHA-256 pin equality; internal arithmetic consistency of every truth tuple (data object + index offsets close the file exactly); code references (truth table, family test, seek test); provenance (command + commit + plan doc); executed evidence (4 baseline-cited tests); name-vs-content honesty (WMV2/64×48 ↔ video tuple). Domain unreachability is filed against the asf domain (error.rs F-001), not this file.

## 4. Per-function table
Not applicable (binary asset). Consumers: `FIXTURES` truth entry (tests.rs:947-960), family walk test (incl. the indexed-seek branch), live-ffprobe differential, `simple_index_seek_reanchors_the_walk`.

## 5. Test honesty
| Test consuming this fixture | Would it fail if the fixture (or code) were broken? | Reason |
|---|---|---|
| fixture_family_walks_match_the_measured_truth | YES | sha pin + header/children/FPO/data/index tuples + track descriptor + first_pts; PLUS the indexed-seek branch (seek at 1000, land, resume, assert ≥1 packet) runs only because this file has an index — the branch is unreachable for the audio-only fixtures. |
| fixture_family_cross_checks_against_live_ffprobe | YES | Live ffprobe must report `["wmv2"]` and per-track packet counts must equal the walk's on these exact bytes. |
| simple_index_seek_reanchors_the_walk | YES | The Simple-Index seek/reanchor contract (baseline :1583 ok). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ffmpeg 7.1.5 sha256-pinned fixture differential incl. live ffprobe codec + -count_packets cross-check" | feature-matrix container-asf-wmv evidence (:216) | REAL | sha verified; both differential tests baseline-ok (:1567, :1953). |
| WMV payload demux: "header tree, Data Object packet walk, Simple-Index seeking" | matrix row description (:210) | REAL at unit level / UNREACHABLE at player level | All three exercised on this file (walk + packets + index seek, baseline-cited); the open-path wiring gap is the domain finding (error.rs F-001). |
| "broadcast files refuse typed (ffprobe agrees, measured)" | matrix notes (:217) | REAL (sibling fixture) | `asf_stream_broadcast.wmv` + `broadcast_fixture_refuses_typed_where_ffprobe_refuses` ok (baseline :1566); not this file's lane. |

## 7. Dead / duplicated / unreachable code
Not applicable. In-`src/` placement is the documented ASF-d byte-stability decision (tests.rs:1068-1089).

## 8. Dependencies & flags
None (test data). Same corpus-license note as my file-9 audit: ffmpeg lavfi self-build, no CORPUS.md row (repo-wide gap for the ASF family, not this file).

## 9. Open questions for the next agent
- The video lane's `first_pts` doc note ("the A/V fixtures' video rides a 64 ms-offset packet grid") applies to `wmv_wmv2_wma_av.wmv` (first_pts non-zero there); this file is video-only with first_pts 0 — if the next agent audits the A/V fixture, the preroll-subtraction oracle is the interesting part.
- If ASF wiring (issue #53) lands, this file is the natural WMV open-path probe (its decode would still refuse codec-side per `codec-video-wmv-vc1` blocked reasons — container-vs-codec lane distinction worth preserving in any matrix update).
