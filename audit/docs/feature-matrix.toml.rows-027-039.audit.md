# CLAIM AUDIT: docs/feature-matrix.toml#rows-027-039 (video-codec tail + audio-codec head)

| Field | Value |
|---|---|
| Auditor | W18-A09 |
| Date | 2026-11-24 |
| Rows in chunk | 13 (ids codec-video-vp9 … codec-audio-mp3; matrix line refs 471-651) |
| Claims extracted | 13 row-claims + 9 embedded evidence/notes claims |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 5 REAL-rows / 4 REAL-UNPROVEN-platform / 0 / 0 / 0 / 1 FALSE-subclaim / 1 UNREACHABLE-limb |
| Internal contradictions found | 2 |

## 1. Blunt verdict

The blocked rows (028-031, 034, 036, 037) are honest: I verified there is NO decoder code for vp8, theora, mpeg4-asp, ffv1, wmv/vc1 anywhere in crates/. The implemented rows (027, 032, 038, 039) are backed by real, executed code — but only on the Linux lane; windows/macos "implemented" statuses have zero per-OS execution on record (the rows' own notes admit "ALL evidence to date is Linux-only, CI 3-OS PENDING"). One description lies: plfmt-xspf is not in this chunk, but codec-video-prores-dnxhd's row admits the honest split. Two registry lies span this chunk: the shipped binary's `engine-info` hardcodes `decoders: mjpeg, pcm_s16le` (host.rs:1646-1661) while the real decoder dispatch wires 14+ codec families (pipeline_host.rs:238-384) — and the codec-unavailable error prose (pipeline_host.rs:376-378) hardcodes a "landed:" list that omits vp9 and dnxhd whose arms exist at :313 and :319. Three hand-maintained registries, three different lies, zero data-driven.

## 2. Claim ledger

| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | vp9: "windows = implemented, linux = partial, macos = implemented, web = not_started" | docs/feature-matrix.toml:480 | linux REAL; win/macos REAL-UNPROVEN | Code: crates/codecs/src/video/vp9/{decoder.rs,error.rs,mod.rs}; dep rusty_vp9 "=0.1.3" crates/codecs/Cargo.toml:145; CLI dispatch arm `"vp9" =>` apps/cli/src/pipeline_host.rs:313-316. Executed: baseline cargo-test-workspace.txt:1384-1391 `running 4 tests` → `test differential_lanes_vs_ffmpeg_libvpx_vp9 ... ok` (21.12s), 4 passed/0 failed. Compat-matrix doc :138 records 2/3 corpus tracks decoded through the open path, 1 refused → linux partial demotion honest. win/macos: no OS-specific run on record. |
| 2 | vp9 evidence: "vp9_differential 4/4 pass in the codecs suite (643 tests / 624 pass / 0 fail…)" | :481 | REAL (numbers stale-LOW) | Baseline :1384-1391: 4/4 pass ✓. Suite count drifted upward: I summed the codecs block (lines 370-1436) = 54 binaries, 701 passed / 0 failed / 18 ignored — MORE than the cited 643/624. Claim understates current state; not inflation. |
| 3 | vp8: "status blocked on all four; blocked_reason: No validated pure-Rust VP8 engine" | :490-497 | TRUE (accurate) | rg 'vp8\|theora\|ffv1' over crates/ → only demux descriptor maps (crates/demux/src/mp4/mod.rs:444 `b"vp08" => Some("vp8")`), no decoder module (ls crates/codecs/src/video/: av1 dnx h264 hevc mjpeg mpeg12 vp9). |
| 4 | theora: "blocked … 'theora' crate = 0.0.0 reserved placeholder" | :498-506 | TRUE (accurate) | No theora code in crates/codecs/src/{video,audio}/ (ls, see #3). |
| 5 | mpeg1-2: "blocked; notes: sequence-layer parser + typed refusal LANDED" | :510-518 | TRUE (accurate) | crates/codecs/src/video/mpeg12/{bitreader,headers,startcode,facts}.rs exist; refusal: mpeg12/mod.rs:17-18 "The typed refusal the codec-api registry surfaces is [`error::decode_refusal`]". Decode genuinely absent → blocked honest. |
| 6 | mpeg4-asp: "blocked … v1 decode typed refusal" | :522-529 | TRUE (accurate) | No mpeg4 decoder module (ls #3); compat-matrix :246 records 0257-avi (ASP) refused `demux.avi.no-supported-streams`. |
| 7 | mjpeg: "implemented (win/linux/macos); first-vertical codec" | :549 | REAL (executed) | Code: 10-file module (mod/parser/decoder/idct/idct_simd/huffman/bitreader/field/color/fuzz). CLI arms: pipeline_host.rs:242-247 `"mjpeg" =>`; bench.rs:300-309. Executed: baseline rc-smoke.txt:131 `headless-decode: PASS (v_decoded==v_frames, a_decoded==a_frames)` on avi_mjpeg_pcm.avi (60/60 both lanes); compat-matrix :126 records 33 mjpeg tracks decoded, 0 concealed. |
| 8 | mjpeg evidence: "card 2.B4: 120/120 reference hash-match" | :550 | REAL-UNPROVEN | The differential test exists (crates/codecs/tests/mjpeg_field_differential.rs: fixture_lanes_match_pinned_truth, corpus_lanes_match_pinned_truth) but the corpus dir is absent in this sandbox: baseline shows `3 passed … finished in 0.04s` (corpus lanes skip loudly). The 120/120 figure is historical, not reproducible here. |
| 9 | raw-fourcc / ffv1 / wmv-vc1 / legacy-blocked: "not_started / blocked, no code" | :558-623 | TRUE (accurate) | rg over crates/ for vp8/theora/ffv1/cinepak/indeo/realvideo/vc-1 decoders → nothing (only demux descriptor maps + ASF container tests). Honest empty rows. |
| 10 | prores-dnxhd: "partial on all three desktops; DNxHD/DNxHR decode implemented via filmcraft-dnx; ProRes NOT implemented" | :587-589 | REAL (tests) but UNREACHABLE from shipped binary; win/macos REAL-UNPROVEN | Code: crates/codecs/src/video/dnx/ + filmcraft-dnx "=0.9.0" Cargo.toml:158; CLI arm `"dnxhd" =>` pipeline_host.rs:319-326. Executed: baseline :1032-1041 dnx_differential `running 6 tests` → 6 passed/0 failed. ProRes absence verified: no prores code under crates/codecs (rg). BUT the arm's own comment (pipeline_host.rs:323-325) says verbatim: "No container lane mints the `dnxhd` descriptor yet … the codecs-crate differential is the lane" — no shipped demuxer ever produces `dnxhd`, so the shipped binary cannot reach this decoder. |
| 11 | pcm: "win/macos implemented, linux partial (f32 tolerance FAILs), web not_started" | :634 | REAL (executed); linux-partial honest | Code: crates/codecs/src/audio/pcm/{decoder.rs,error.rs,fuzz.rs,mod.rs}; arms pipeline_host.rs:347-373 (`codec if codec.starts_with("pcm_")`), bench.rs:311-331. Executed: baseline mp4-smoke.md decode-bench calibration `a_frames=5 a_decoded=5` on sample.wav; rc-smoke.txt:131 60/60. The 6 f32 FAILs verified in-tree: docs/corpus/vlc-f32-tolerance-20261109.json → all 6 rows verdict FAIL, worst 0305 rms 0.1209/peak 0.5424. |
| 12 | pcm notes: "worst row 0305-avi-av.avi measures rms 1.22e-1 / peak 4.94e-1" | :636 | PARTLY FALSE (peak figure) | The cited tolerance json measures peak 5.42e-1 (0.5424) for 0305, not 4.94e-1; rms 1.21e-1 ✓. Two in-tree artifacts disagree on the same row's peak (compat-matrix :238 also says 4.94e-1). UNVERIFIED which measurement is right — no VLC oracle in sandbox. |
| 13 | mp3: "implemented (win/linux/macos); Linux-only evidence" | :647 | linux REAL (executed); win/macos REAL-UNPROVEN | Code: crates/codecs/src/audio/mp3/{decoder,header,id3,xing,seek,fuzz,error,mod}.rs; dep symphonia-bundle-mp3 "=0.6.1" Cargo.toml:90; CLI arm `"mp3" \| "mp1" =>` pipeline_host.rs:261-265. Executed: baseline :1257-1272 mp3_bitexact 2/2 (`fixtures_decode_the_recorded_window_exactly ... ok`) + mp3_corpus_differential 2/2; compat-matrix :103 records 1 mp3 track decoded via open path. Notes admit the PCM-MD5 table was "lost to a deadline — re-print owed (audit gap G-3)" — honest. |

## 3. Contradictions

| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Matrix header (rule R4/status vocab) demands evidence resolving to artifacts and honest statuses | engine-info output (host.rs:1646-1661) hardcodes `decoders: [mjpeg, pcm_s16le]` while matrix rows 027/039 claim vp9/mp3 "implemented" | Both are true statements about different things (code-complete vs build-snapshot); the LIE is engine-info: it is a stale hand-written list (baseline mp4-smoke.md `decoders: mjpeg, pcm_s16le`), contradicting the actual dispatch arms at pipeline_host.rs:313 (vp9) and :261 (mp3). |
| Row 038 notes: worst f32 row peak 4.94e-1 | docs/corpus/vlc-f32-tolerance-20261109.json row 0305: peak_dev 0.5424 | The json is the ratified measurement artifact; the 4.94e-1 figure (also in compat-matrix :238) does not match it. One of the two artifacts is wrong; UNVERIFIED without a VLC oracle. |

## 4. Fabrication / inflation indicators

- The evidence strings for vp9/dnx cite test counts (643/624) that are now LOWER than reality (701/0 in baseline) — the opposite of inflation; stale in the conservative direction.
- "implemented" on windows/macos with zero windows/macos execution anywhere on record — status inflation per the matrix's own vocabulary ("implemented" = code complete, evidence not yet complete) but legal-by-definition; flagged REAL-UNPROVEN, not FALSE.
- The `landed:` error-prose registry (pipeline_host.rs:376-378) omits vp9 + dnxhd — the prose UNDERSTATES the dispatch; simultaneously engine-info OVERSTATES nothing but omits 12 decoders. Both directions of registry drift in one file pair.

## 5. Matrix rows

| Row id | Statuses claimed (w/l/m/web) | Evidence string (verbatim key) | Code actually found | Executed? | Verdict |
|---|---|---|---|---|---|
| codec-video-vp9 | implemented / partial / implemented / not_started | "crates/codecs/src/video/vp9/** + rusty_vp9 =0.1.3 … vp9_differential 4/4 pass" | vp9 module + Cargo.toml:145 + CLI arm pipeline_host.rs:313-316 | YES — baseline :1384-1391 (4/4, ffmpeg-oracle lanes 21.12s) | linux REAL; win/macos REAL-UNPROVEN; web honest not_started |
| codec-video-vp8 | blocked ×3, blocked web | "" (honest empty) + blocked_reason | none (verified absent) | n/a | TRUE (blocked accurate) |
| codec-video-theora | blocked ×4 | "" + blocked_reason | none | n/a | TRUE |
| codec-video-mpeg1-2 | blocked ×4 | "" + blocked_reason citing parser landed | parser-only module mpeg12/ (bitreader/headers/startcode/facts + decode_refusal) | unit tests in codecs 506-test lib (baseline :370-373) | TRUE (decode blocked honest; parser real) |
| codec-video-mpeg4-asp | blocked ×4 | "" + blocked_reason | none | n/a | TRUE |
| codec-video-mjpeg | implemented ×3, web not_started | "card 2.B4: 120/120 reference hash-match … ledger.json (2.E1, mock-labeled)" | 10-file module + two dispatch arms | YES — rc-smoke.txt:131 headless-decode PASS 60/60; compat-matrix :126 (33 tracks) | REAL |
| codec-video-raw-fourcc | not_started ×4 | "" | none | n/a | TRUE |
| codec-video-wmv-vc1 | not_started/not_started/blocked/unsupported | "" + blocked_reason | none (ASF demux enumerates wmv descriptors only) | n/a | TRUE |
| codec-video-prores-dnxhd | partial ×3, web not_started | "dnx_differential 6/6 pass" + ProRes not implemented | dnx module + filmcraft-dnx + CLI arm | YES — baseline :1032-1041 (6/6) | REAL code/tests; UNREACHABLE from shipped binary (no demuxer mints `dnxhd` — pipeline_host.rs:323-325); win/macos REAL-UNPROVEN |
| codec-video-ffv1 | not_started ×4 | "" | none | n/a | TRUE |
| codec-video-legacy-blocked | blocked ×4 | "" + blocked_reason | none | n/a | TRUE |
| codec-audio-pcm | implemented/partial/implemented, web not_started | "card 2.B5: full format matrix … 6 f32 tolerance-FAIL rows + 29 bit-exact" | pcm module + arms | YES — decode-bench a_decoded=5 (baseline mp4-smoke.md); rc-smoke 60/60 | REAL; linux partial honest; f32 lane = self-reported S2 correctness defect |
| codec-audio-mp3 | implemented ×3, web not_started | "card 5.A3 symphonia + in-house glue … Linux-only evidence" | mp3 module (7 files) + CLI arm + symphonia-bundle-mp3 =0.6.1 | YES — baseline :1257-1272 (bitexact 2/2 + corpus differential 2/2) | linux REAL; win/macos REAL-UNPROVEN |

## 6. Findings routed to repo-level

- F-A (S3): engine-info decoder registry is a hand-maintained stale snapshot — apps/cli/src/host.rs:1646-1661 hardcodes `mjpeg`+`pcm_s16le` only, while apps/cli/src/pipeline_host.rs:238-384 dispatches mjpeg/flac/mp3/mp2/vorbis/opus/aac/alac/adpcm/av1/vp9/dnxhd/h264/pcm. Any consumer of engine-info (docs, compat tooling) under-reports 12 decoders.
- F-B (S3): the codec-unavailable prose registry (pipeline_host.rs:376-378) hardcodes a "landed:" list omitting vp9 (:313) and dnxhd (:319) — the string and the code in the same file disagree.
- F-C (S2): pcm_f32le decode deviates beyond the ratified tolerance on every measured corpus row (6/6 FAIL; worst RMS 0.121) — self-reported in matrix notes + tolerance json; unfixed at audit time.
- F-D (S4): peak figure 4.94e-1 (matrix :641, compat-matrix :238) vs 5.42e-1 (tolerance json) for row 0305 — artifact numeric drift.
