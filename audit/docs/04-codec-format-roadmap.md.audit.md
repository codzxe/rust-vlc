# CLAIM AUDIT: docs/04-codec-format-roadmap.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 14 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 8 / 3 / 2 / 0 / 0 / 1 / 0 |
| Internal contradictions found | 1 (repo-side, not doc-side) |

## 1. Blunt verdict
A Wave-1 planning doc with an unusually strict honesty discipline: every uncertain figure is
[VERIFY]/[VERIFY-P0]-tagged, the two "filled" record examples are self-declared mock
placeholders, and its closing rule is "When reality disagrees with a row, the row loses."
Its two big deliverable claims landed and are verifiable in-tree (fuzz-target registry with
exact mandated names; 53 per-format record files). One claim is now FALSE by drift in the
GOOD direction: "no production pure-Rust H.264 decoder exists today" — `rust_h264` 0.6.1
is integrated (apps/cli/src/pipeline_host.rs:332-335, codecs/src/video/h264/sw.rs). The doc
is stale there, not fraudulent.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Verification date for all crate claims ... 2026-10-19, via the crates.io HTTP API and ... tarballs" | L9-12 | REAL-UNPROVEN | crates.io not reachable from sandbox; crate names/versions are consistently cited and the doc's own §1.2 method is sound — cannot re-verify today (network) |
| 2 | "every named crate was confirmed to exist on crates.io on 2026-10-19" | L98-99 | REAL-UNPROVEN | same network limitation |
| 3 | Fuzz registry: "one target per parser named exactly as in the tables (fuzz_demux_*, fuzz_codec_*, fuzz_sub_*)" | L604-607 | REAL | fuzz/fuzz_targets/ contains fuzz_codec_{aac,adpcm,alac,av1,flac,h264,hevc,mjpeg,mp2,mp3,pcm,vorbis}.rs + fuzz_demux_{adts,aiff,avi,es,flac,ivf,mkv,mp3stream,...}.rs + fuzz/corpus/<target>/ seeds — names match the tables |
| 4 | "A parser crate may not reach implemented until its target exists ... and has a seed corpus committed" | L609-611 | REAL (registry present) / enforcement UNVERIFIED | corpora committed; whether the CI 10-min-clean rule runs is other agents' CI scope |
| 5 | "Every format Lumen claims gets a record file docs/formats/<kind>-<name>.md" | L382-384 | REAL | ls docs/formats -> 53 files (container-*, audio-*, video-*, sub-*) |
| 6 | "Both examples below are illustrative mock records ... placeholders ... not actual measurements" | L414-417 | REAL | verbatim honesty label; prevents mock data being read as results |
| 7 | "Primary: MP4 + PCM audio + Motion-JPEG video" (Phase 2 first vertical) | L565-566 | SHALLOW | delivered vertical per baseline: MJPEG+PCM (rc-smoke: AVI/MJPEG/PCM headless-decode PASS; engine-info: decoders mjpeg, pcm_s16le; mp4-smoke walks states but decode-bench v_frames=0 for H.264) — MP4 demux real, its H.264/AAC payloads not decodable in the first vertical |
| 8 | "there is no production-grade pure-Rust H.264 decoder" (§4.2) | L272-273 | FALSE (stale) | apps/cli/src/pipeline_host.rs:332-335: "2026-11-15 — `rust_h264` 0.6.1 behind the codec-api ... differential evidence crates/codecs/tests/h264_sw_differential.rs"; codecs/src/video/h264/{sw.rs,decode.rs} exist |
| 9 | "rav1d ... default build fails §9; the asm feature can be disabled" | L316-319 | REAL | crates/codecs/src/video/av1/ exists + fuzz_codec_av1.rs; asm-off integration claim matches tree (Cargo.toml check is dependency agent's scope) |
| 10 | "Symphonia is audio-only" | L274 | REAL | crates/codecs/src/{audio,video}/ split; symphonia crates under audio only (rg) |
| 11 | "Opus ... Symphonia lists Opus '—' in work, symphonia-codec-opus not yet on crates.io" | L191 | STALE (now shipped differently) | crates/codecs/src/audio/opus/ exists in-tree (self/opus integration); CLI error text L376-378 lists opus among "landed" |
| 12 | "v1's software video story is: AV1 + Motion-JPEG + uncompressed video" | L330-331 | SHALLOW (understates) | video/ also has h264 (sw via rust_h264), mpeg12, vp9, dnx, hevc dirs in-tree — reality outran the doc's "honest ceiling" |
| 13 | "the feature matrix computes parity only from verified rows" | L552-553 | REAL-UNPROVEN | consistent with feature-matrix.toml header vocabulary (statuses require evidence); the tool that enforces it is admitted-owed debt (matrix header L12-14) |
| 14 | "claxon unmaintained since 2020" / "lewton unmaintained since 2021" | L82, L190 | REAL-UNPROVEN | external facts, cannot re-verify offline |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| §4.2: no production pure-Rust H.264 decoder exists | CLI: "landed: ... h264" (pipeline_host.rs:376-378); codecs/src/video/h264/sw.rs + rust_h264 0.6.1 | The CLI — a decoder exists now (doc is stale, written 2026-10-19; decoder landed 2026-11-15 per in-file comment) |
| (side observation) CLI error claims 12 codecs "landed" | baseline engine-info: "decoders: mjpeg, pcm_s16le"; decode-bench v_frames=0 | engine-info baseline is the executable truth; the "landed:" list overstates the release registry (for the engine-info owner to reconcile) |

## 4. Fabrication / inflation indicators
None in the doc itself. Its [VERIFY] discipline is the opposite of inflation. The one
repo-side inflation touching this roadmap is the CLI error string's "landed:" list (row 14 /
contradiction 2) — copy that contradicts the doc's own truthful-reporting rule.

## 5. Matrix rows
N/A (roadmap feeds matrix rows; those rows are other agents' scope). Commands: ls fuzz/
fuzz_targets + corpus; ls docs/formats; sed pipeline_host.rs:370-380; rg codecs tree.
