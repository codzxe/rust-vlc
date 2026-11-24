# CLAIM AUDIT: docs/formats/video-wmv-vc1.md
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R5) |
| Claims extracted | 8 |
| REAL 4 / TRUE-at-time 3 / UNVERIFIED 1 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Honest deferred decision record dated 2026-10-24. Its matrix status claim (win/linux
not_started, macos blocked, web unsupported) is byte-exact today; its "codecs crate admits
exactly 4 symphonia engines" claim is byte-exact today (aac/alac/mp3/vorbis — no more have
been added); no WMV/VC-1 code exists. Two claims were true then and are stale-now in the
honest direction: the ASF container has since LANDED (demux/src/asf + implemented matrix row)
and ADR-010 has since been written. crates.io facts (wmv-decoder 0.2.0 provenance, NOT_FOUNDs)
are UNVERIFIED offline.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Matrix row codec-video-wmv-vc1: win/linux not_started, macos blocked, web unsupported | :15-16/:75 | REAL | feature-matrix.toml:567-575: `id = "codec-video-wmv-vc1"` ... `status = { windows="not_started", linux="not_started", macos="blocked", web="unsupported" }` (sed-verified). |
| 2 | codecs crate admits exactly 4 symphonia engines (mp3/vorbis/aac/alac); no WMV/VC-1 edge | :52-54 | REAL | crates/codecs/Cargo.toml symphonia deps = symphonia-codec-aac/-codec-alac/-bundle-mp3/-codec-vorbis (+core) — exactly 4 engines (rg); `ls crates/codecs/src/video` -> av1 dnx h264 hevc mjpeg mpeg12 vp9 — no WMV. |
| 3 | ASF container itself is not landed | :10-11/:54 | TRUE-at-time-since-landed | crates/demux/src/asf/ now exists (15 files incl. tests.rs/fuzz.rs); matrix container-asf-wmv :207 `status = { windows="implemented", linux="partial", macos="implemented", web="not_started" }` — landed AFTER this record's 2026-10-24 date (issue #53 notes dated 2026-11-10). |
| 4 | ADR-010 not yet written [VERIFY at ADR-010] | :78-79 | TRUE-at-time-since-written | docs/adr/ADR-010-codec-patent-shipping.md exists today (ls docs/adr). |
| 5 | WMV2 probed via avcodec; asf_wmv2_wmav2.log exists; VC-1 census-only not-probe-verified | :47-50 | REAL | `rg -o asf_wmv2_wmav2[^"]* docs/inventory/formats.json` -> asf_wmv2_wmav2.log (6 refs); wmv2 key present; oracle-side, not our code. |
| 6 | crates.io: vc1/vc1-rs/vc-1/wmv9 NOT_FOUND; `wmv` reserved 0.0.0 since 2017 | :30-32 | UNVERIFIED | offline sandbox; no registry query possible; consistent with the 2026-10-24 evidence-date framing. |
| 7 | wmv-decoder 0.2.0 (MPL-2.0, siglus_rs provenance) — not a candidate; Phase-7 recheck trigger | :33-41 | UNVERIFIED (external) / REAL (posture) | the record explicitly refuses adoption pending `[VERIFY: purity + conformance]` — the safest possible claim shape. |
| 8 | Typed-unsupported stream semantics for WMV/VC-1 once ASF lands | :20-26 | REAL (policy now exercised) | pipeline_host.rs:374-379 refusal path "no decoder for `{other}` in this build" (sed-verified) — the semantics the record mandates exist and fire for non-landed codecs. |

## 3. Contradictions
None — the record's forward-looking conditions are now observable as landed (ASF row,
ADR-010) while its own subject (WMV/VC-1 decode) remains not_started exactly as recorded.

## 4. Fabrication / inflation indicators
None — a decision record that schedules nothing and defers everything with named recheck
triggers.

Commands run: rg codec-video-wmv-vc1 + container-asf-wmv + status lines in feature-matrix.toml; rg symphonia in crates/codecs/Cargo.toml; ls crates/demux/src/asf, crates/codecs/src/video, docs/adr; rg wmv2/asf_wmv2_wmav2 in docs/inventory/formats.json; sed 374-380p apps/cli/src/pipeline_host.rs.
