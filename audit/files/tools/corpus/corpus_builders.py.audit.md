# AUDIT: tools/corpus/corpus_builders.py
| Field | Value |
|---|---|
| Auditor | W16-A05 |
| Date | 2026-11-24 |
| Lines / functions | 1663 / 47 (incl. BUILDERS registry; 27 builder entry points) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO from lumen-cli/desktop release binary; YES from test infra: scripts/corpus_supervisor.sh:14 -> `python3 tools/corpus/gen_corpus.py` -> gen_corpus.py:91 `from corpus_builders import BUILDERS, Ctx, draw_audio_params` (also .github/workflows/compat-matrix.yml corpus regen lane) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 -m py_compile (exit 0); python3 harness invoking all 27 BUILDERS (exit 0, 27/27 OK); real ffmpeg 7.1.5 encode of build_odd_dims argv (rc 0, ffprobe: mjpeg 17x9); 20000-seed draw_resolution repro |

## 1. One-paragraph verdict
This is the ffmpeg argv-assembly layer of the corpus generator: 27 builder functions producing randomized-but-replay-safe argv lists, aux files (SRT/VTT/ASS/FFMETADATA/concat/master m3u8), and v4 byte post-op specs. It is genuinely adversarial (metadata bombs, chapter bombs, 1-byte AVI chunks, RIFF size lies, HEVC-in-AVI fourcc lies, zero-frame tracks, 24h stills with documented 2^31/16384 movenc overflow handling) and capability-probed at every turn — unsupported draws return None and land in a rejected-draws ledger per gen_corpus. I executed every builder and ran one argv through the real ffmpeg 7.1.5: the file decodes as claimed. Two defects: the odd-resolution label lies for tiny16/tiny32 (manifest name says odd, dims stay even — reproduced 449/20000 seeds), and build_hls hardcodes libx264 while recording the drawn vcodec in params.

## 2. What actually works (verified)
— python3 -m py_compile corpus_builders.py -> exit 0.
— python3 loop over BUILDERS with seeded rng and stub caps: `builders returned successfully: 27 / 27`, correct tuple shapes (argvs/aux/post).
— Executed `ffmpeg -hide_banner -loglevel error` on build_odd_dims(forced 17x9, mkv/mjpeg) argv -> rc 0, 6616-byte file; `ffprobe -show_entries stream=codec_name,width,height` -> `mjpeg,17,9` — odd dims really preserved end-to-end.
— 20000-seed draw_resolution sweep -> mislabel repro (see F-001).

## 3. Findings
F-001 [S3]
Where: tools/corpus/corpus_builders.py:74-80
Code:
```
    if not force_even and rng.random() < 0.18:
        w2, h2 = w, h
        if rng.random() < 0.5 and w2 >= 40:
            w2 -= 1
        elif h2 >= 40:
            h2 -= 1
        return f"{name}odd", w2, h2
```
What is wrong: for resolutions whose width AND height are < 40 (tiny16=16x16, tiny32=32x32 — corpus_vocab.py:44), neither branch decrements, yet the returned res_name is `tiny16odd`/`tiny32odd` with even unchanged dims. Reproduced: 449 of 20000 seeds (e.g. seed=31 -> `tiny16odd dims=16x16`). Why it matters: res_name feeds the corpus MANIFEST and corpus-plan strata accounting; an "odd" stratum silently contains even-dimension files, so stratification counts lie (width/height fields themselves stay honest, limiting blast radius). What correct looks like: skip the odd suffix when no dimension changed. Confidence: HIGH — Verification: ran (20000-seed repro).

F-002 [S4]
Where: tools/corpus/corpus_builders.py:779 and :801
Code:
```
        a += ["-c:v", "libx264", "-preset", "veryfast", "-pix_fmt", "yuv420p",
```
```
    params = {"class": "hls-av", "container": "hls", "codec": vcodec,
```
What is wrong: build_hls ignores the drawn `vcodec` (always libx264) while `params["codec"]` records whatever the family tuple carried; a non-h264 family would produce a manifest lying about the codec. Today unreachable in practice: gen_corpus.py:646-647 only routes `("hls"/"hlsf", "h264+aac")` to this builder, so no mismatch is currently generated. Confidence: HIGH — Verification: read + executed builder (always libx264 in argv).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| _fmt_fps | 60-61 | REAL | — | %g format, exercised |
| draw_resolution | 64-84 | REAL (buggy label) | S3 | F-001 |
| draw_fps | 87-90 | REAL | — | weighted pool |
| draw_duration | 93-98 | REAL | — | pixel-tiered clamp |
| draw_audio_params | 101-130 | REAL | — | ac3/mp2 rate floors per measured encoders |
| audio_source_args | 133-158 | REAL | — | with_ac leak fix documented |
| video_source_args | 167-178 | REAL | — | output-side -t for infinite sources |
| out_t | 181-182 | REAL | — | |
| pick_pix | 185-193 | REAL | — | returns None when unsupported (honest reject) |
| venc_args | 196-229 | REAL | — | codec-specific rc draws |
| aenc_args | 232-240 | REAL | — | per-stream -c:a:i variant |
| sar_filter_for | 243-248 | REAL | — | fallback first entry |
| _ts_hmsms | 255-260 | REAL | — | |
| write_srt | 263-284 | REAL | — | BOM/CRLF/zero-dur/idx chaos; vocab backs '-->' + null byte (corpus_vocab.py:234-240) |
| write_vtt | 287-297 | REAL | — | |
| write_ass | 300-332 | REAL | — | pos/karaoke overrides |
| write_ffmetadata | 335-345 | REAL | — | min 1ms chapter clamp |
| subtitle_aux | 348-353 | REAL | — | |
| Ctx | 360-364 | REAL | — | dataclass-ish |
| _res_cap_for | 367-368 | REAL | — | lossless codec pixel cap |
| _resolve_res | 371-386 | REAL | — | encoder min floors |
| _ensure_frames | 389-397 | REAL | — | >=2 frames guarantee, logic checked correct |
| _theora_min | 400-407 | REAL | — | 32x32 floor |
| build_video | 410-446 | REAL | — | executed OK |
| _av_common | 449-513 | REAL | — | settb-before-setpts VFR fix documented |
| build_av | 516-529 | REAL | — | executed OK |
| build_audio | 532-546 | REAL | — | |
| build_multi_av | 549-646 | REAL | — | input/output section split; per-stream -ar:a:i |
| build_dynres | 649-701 | REAL | — | res ladder + concat; dup-res retry cap 6 may yield dupes (harmless) |
| build_raw | 704-725 | REAL | — | |
| build_fmp4 | 728-751 | REAL | — | frag_duration draws |
| build_hls | 754-808 | REAL (vcodec ignored) | S4 | F-002; relative init filename fix documented |
| build_cover | 815-842 | REAL | — | attached_pic disposition |
| build_multivideo | 845-891 | REAL | — | 2 video programs |
| build_drift | 894-920 | REAL | — | dur 45-110s bypasses preset max_dur BY DESIGN (DRIFT_DUR_RANGE corpus_vocab.py:366) |
| build_bomb | 927-950 | REAL | — | CJK 21845-char ≈ 64KiB cap consistent |
| _odd_source | 959-969 | REAL | — | odd-preserving sources only |
| _tiny_res | 972-977 | REAL | — | |
| build_odd_dims | 980-1018 | REAL | — | executed with real ffmpeg (rc 0) |
| build_rot_display | 1021-1066 | REAL | — | two routes, probe-gated |
| build_maxtracks_avi | 1069-1120 | REAL | — | input index bookkeeping verified correct (one -i per input) |
| build_bframe_storm | 1123-1166 | REAL | — | |
| build_packed_bs | 1169-1218 | REAL | — | xvid probe-gated |
| _concat_segs_pass | 1221-1223 | REAL | — | |
| build_param_midchange | 1226-1276 | REAL | — | per-seg bitrate/profile switch |
| build_vbv_burst | 1279-1335 | REAL | — | concat-step or single-pass VBV |
| build_fps_extremes | 1338-1368 | REAL | — | |
| build_sr_chaos | 1371-1402 | REAL | — | pan-built odd layouts |
| write_chapter_bomb | 1405-1418 | REAL | — | 1.2ms min slice |
| build_chapter_bomb | 1421-1450 | REAL | — | |
| build_zeroframe_stream | 1453-1481 | REAL | — | select=0, probe-gated container |
| build_still_24h | 1484-1533 | REAL | — | 2^31/16384 movenc overflow switch to slideshow documented |
| build_riff_size_lie | 1536-1566 | REAL | — | post-op spec |
| build_odd_chunk_avi | 1569-1609 | REAL | — | est math documented from measurement |
| build_hevc_in_avi | 1612-1645 | REAL | — | fourcc lie class |
| BUILDERS | 1648-1663 | REAL | — | 27 entries, matches gen_corpus KIND_FOR_CLASS |

## 5. Test honesty
No tests in this file; it is the generator under test elsewhere (run_harness.py/fidelity.py). Not applicable.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ffmpeg speed parity is a tracked [metric] ... corpus_builders.py" | docs/engineering-directives.md:126 | REAL-UNPROVEN | file exists and builds argvs (executed); parity bench itself not run by me |
| v4 mechanisms "probed against the real ffmpeg 7.1.5" | corpus_builders.py:954-956 docstring | REAL | sandbox ffmpeg is exactly 7.1.5; my encode of an odd-dims argv succeeded rc 0 |
| "(measured: 49 584 chunks from a 1.2 s file)" | corpus_builders.py:1598-1599 | UNVERIFIED | would require running the avi-rechunk post-op; not executed |

## 7. Dead / duplicated / unreachable code
No dead builders found: all 27 keys in BUILDERS are consumed via gen_corpus.py:835 `BUILDERS.get(KIND_FOR_CLASS.get(cls, cls))`; `draw_audio_params` also imported directly (gen_corpus.py:91).

## 8. Dependencies & flags
Pure stdlib (`random`, `__future__`) + corpus_vocab (same dir). No network, no env reads. External dependency: ffmpeg/ffprobe binary (present: 7.1.5).

## 9. Open questions for the next agent
- Does run_harness.py enforce the est_hint budget against actual bytes, catching build_still_24h's fixed 131_072 est_hint (line 1532) if a real file overshoots?
- gen_corpus.py:807 Ctx(limits, {}, RES_ORDER) passes EMPTY caps at plan time — which builders silently return None there? (Worth one look; not my file.)
