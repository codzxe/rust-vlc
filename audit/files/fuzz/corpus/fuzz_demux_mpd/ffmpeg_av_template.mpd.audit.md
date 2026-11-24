# AUDIT: fuzz/corpus/fuzz_demux_mpd/ffmpeg_av_template.mpd
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 2288 B / b12ec1b1a607df828ecf6a077e0fb7351cfd494c88b47f21b0f1e1afece05d8c |
| File verdict | REAL (as data); the corpus copy itself is never read |
| Reachable from shipped entry point? | NO — fuzz corpus seed; byte-twin consumed via include_str! in playlists/streaming unit tests |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. Verdict (corpus-seed audit)
Real DASH MPD template: `<?xml …?>` + `MPD xmlns=urn:mpeg:dash:schema:mpd:2011`, static, mediaPresentationDuration PT6.0S, isoff-live profile, one adaptation set (ffmpeg's own `-f dash` template shape). It is sha256-identical to `crates/playlists/tests/fixtures/dash/ffmpeg_av_template.mpd`, whose bytes ARE executed on stable through `include_str!` unit tests in crates/playlists/src/mpd.rs (3 call sites) and the streaming crate's dash resolve/fuzz tests. The corpus copy is read by no stable gate (campaign unwired).

## 2. What was checked and how
- batch.py: 2288 B, 45 text lines, XML head verified.
- sha256 equality with the playlists twin: b12ec1b1… on both (verified).
- Twin consumers: crates/playlists/src/mpd.rs:2117 `include_str!("../tests/fixtures/dash/ffmpeg_av_template.mpd")` (+ :2515, :2804); crates/streaming/src/dash/resolve.rs:998 `first_video_plan(&fixture("ffmpeg_av_template.mpd"))`; crates/streaming/src/dash/fuzz.rs:135.
- Registry gate run by me: exit 0 (35 targets, corpora present — existence only).
- Execution of twin bytes: playlists + streaming unit suites pass in baseline (audit/runs/baseline/cargo-test-workspace.txt).

## 3. Findings
F-001 [S3] Same corpus-lane gap as every other fuzz/corpus seed: the file's only consumers are the existence check (scripts/fuzz_registry_check.sh) and the unwired nightly campaign (scripts/quality/25_fuzz_campaign.sh: "The repo ships 35 fuzz targets with seeds but wires none of them into a gate"). The identical bytes run on stable via the include_str! twin — so the fuzz material is real, but the corpus directory adds no executed coverage. Verification: ran (rg + sha256).
