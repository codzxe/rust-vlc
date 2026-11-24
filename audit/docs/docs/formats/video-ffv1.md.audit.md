# CLAIM AUDIT: docs/formats/video-ffv1.md
| Field | Value |
|---|---|
| Auditor | W19-A09 (recovered by W19-R5) |
| Claims extracted | 6 |
| REAL 4 / TRUE-at-time 1 / UNVERIFIED 1 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Honest deferred decision record. Matrix row (not_started ×4, phase 8, candidate names in
notes) is byte-exact; no FFV1 code exists anywhere in the codecs crate; the MKV
typed-unsupported behavior it describes as "honest v1" is the real shipped behavior. crates.io
census claims (oxideav-ffv1 0.0.9, codec_ffv1 0.1.0, reserved ffv1) are offline-unverifiable.
ADR-010 "not yet written" was true then and exists now.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Matrix row codec-video-ffv1: not_started ×4, notes already carry the candidates | :15-17/:76 | REAL | feature-matrix.toml:594-604: `id = "codec-video-ffv1"` ... `status = { windows="not_started", linux="not_started", macos="not_started", web="not_started" }`, phase_target 8, notes "oxideav-ffv1 0.0.9 + codec_ffv1 0.1.0 exist (young); ADR-010 patent check item" (sed-verified). |
| 2 | Nothing implemented (no FFV1 decode) | :21 | REAL | `ls crates/codecs/src/video | grep -c ffv1` -> 0 (av1 dnx h264 hevc mjpeg mpeg12 vp9 only). |
| 3 | FFV1-in-MKV tracks surface as typed-unsupported streams (codec id pass-through + decode refusal) | :65-67 | REAL | apps/cli/src/pipeline_host.rs:376-378 `"no decoder for `{other}` in this build (landed: mjpeg, pcm, flac, ...)"` (sed-verified in this run); MKV demuxer passes unknown codec ids through (crates/demux/src/mkv). |
| 4 | formats.json video-census-tail carries FFV1 via avcodec, census-only not-probe-verified | :51-54 | REAL | python3: 'ffv1' present (case-insensitive) in docs/inventory/formats.json; matrix inventory_ref :601 points at the same census-tail. |
| 5 | ADR-010 not yet written [VERIFY at ADR-010] | :77-78 | TRUE-at-time-since-written | docs/adr/ADR-010-codec-patent-shipping.md exists today (ls docs/adr). |
| 6 | crates.io census: ffv1 reserved 0.0.0 since 2017; oxideav-ffv1 0.0.9 MIT; codec_ffv1 0.1.0 MIT; cu-ffv1-codec disambiguation | :29-46 | UNVERIFIED | offline sandbox — no registry re-query possible; the record's [VERIFY] markers already fence exactly these facts. |

## 3. Contradictions
None. The record corrects its own upstream (doc 04 §4.1's flat "none" stale since
2026-04-17) rather than repeating it — and the matrix notes reflect the correction.

## 4. Fabrication / inflation indicators
None — schedules nothing, demands a patent check before any claim, and disambiguates a
name-collision crate so future searches aren't fooled.

Commands run: rg codec-video-ffv1 + status/notes lines feature-matrix.toml; ls codecs/src/video (grep -c ffv1 = 0); python3 formats.json scan (ffv1 present); ls docs/adr (ADR-010); sed 374-380p pipeline_host.rs (prior verification in this run).
