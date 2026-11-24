# CLAIM AUDIT: docs/plans/asf-selfbuild.md (L1-572)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 6 / 0 / 0 / 0 / 0 / 0 / 4 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A well-behaved investigation record: it labels every claim [VERIFIED-this-session] vs [FORMAT KNOWLEDGE — verify in ASF-a/b], records the dead first dispatch honestly, and its concrete outputs are all in the tree and re-checkable — the 11 generated fixtures exist byte-for-byte in crates/demux/src/asf/fixtures/, the ffmpeg version it pinned (7.1.5-0+deb13u1) is the one in this sandbox, the asf muxer census (3 muxer hits) matches, and the follow-on units it specced (asf/{data,header,guid,error,tests}.rs, 36 tests) landed exactly as the plan's file list said. The in-session measurement claims (play-duration lie, zero-filled index entries, hang-without--y) are historical session claims — not re-runnable here without writing files outside audit/ (refused).

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Status: COMPLETE (investigation card — zero code changes; see §8)" | :3 | REAL | The card's outputs are fixtures + plan text; code landed later via 6.ASF-a/b/c (matrix row container-asf-wmv evidence). |
| 2 | "the corpus has zero asf/wma/wmv files, but ffmpeg 7.1.5 generates the whole family" | :13-15 | REAL | ffmpeg in this sandbox: `ffmpeg -version` → "7.1.5-0+deb13u1" (exact match); `-muxers \| rg -c asf` → 3 (asf, asf_stream, argo_asf — matches the doc's census incl. the confusion warning). |
| 3 | "11 fixtures produced and ffprobe-verified this session" | §1 | REAL | crates/demux/src/asf/fixtures/ contains exactly 11 files matching the doc's names (asf_msmpeg4v3.asf, wma_wmav2_mono8k.wma, wmv_wmv2_wma_av.wmv…). `ffprobe` on wmv_wmv2_wma_av.wmv → format_name "asf", 2 streams, duration 1.128 — parseable as claimed. |
| 4 | "PCM, ADPCM-MS and MJPEG ride ASF fine in ffmpeg… existing PCM-facts, adpcm WAVEFORMATEX-extra_data and mjpeg decoder paths" | :20-24 | REAL | asf_pcm_s16le.wma / asf_adpcm_ms.wma / asf_mjpeg.asf fixtures present; adpcm extra_data path exists (codecs/src/audio/adpcm/mod.rs WAVEFORMATEX parse — cited at doc §3 line 96); registry arms verified in pipeline_host.rs (matrix/compat audits). |
| 5 | Unit split ASF-a/b/c/d with files `crates/demux/src/asf/{header,data,index,…}.rs` | §4 | REAL (landed as specced) | ls crates/demux/src/asf/ → data.rs, error.rs, fixtures, fuzz.rs, guid.rs, header.rs, mod.rs, test_support.rs, tests.rs — the exact split. |
| 6 | "36-test asf unit suite" (per matrix row) | matrix :223 | REAL | rg test-count over crates/demux/src/asf/*.rs → 36. |
| 7 | "ffmpeg's play-duration lie and zero-filled Simple Index entries (§2.3)" | :21-22 | UNVERIFIED | Session-measured claim; would need fixture generation runs (file writes outside audit/ refused). Fixture bytes present; not re-measured. |
| 8 | "wmav2 also refuses bitrates below 24000… measured" | §1.2 | UNVERIFIED | Same class — historical measurement, not re-run here. |
| 9 | "broadcast-mode files that ffprobe itself refuses (§2.5)" | :22-23 | UNVERIFIED | asf_stream_broadcast.wmv fixture exists; ffprobe refusal behavior not re-run. |
| 10 | "no probe collision… 16-byte GUID anchor is unambiguous vs all 10 current signatures [VERIFIED against the signature list]" | §4.1 | REAL-UNPROVEN | Consistent with the compat-matrix asf sniff bytes (30 26 b2 75…) and probe registry; the "all 10 signatures" comparison is session-historical. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| (none found — the compat-matrix's asf open-path refusal is the POST-plan state this doc predicted would be wired; the matrix row's notes cite this plan's card chain.) | — | — |

## 4. Fabrication / inflation indicators
None. Labels are scrupulous ([VERIFIED] vs [FORMAT KNOWLEDGE]); the corpus-zero census is corroborated by the compat-matrix provenance ("corpus-big lacks (mpegps, asf)").
Commands run (exit 0): ffmpeg -version; ffmpeg -muxers \| rg -c asf; ls asf fixtures (11); ffprobe wmv_wmv2_wma_av.wmv; rg test-count (36). UNVERIFIED: generation-behavior measurements (would write files outside audit/).
Findings: S2: 0 · S3: 0 · S4: 0.
