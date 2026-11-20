# CONTRADICTIONS — every doc/code disagreement found (register)

Full 16-entry register with both sides cited: `audit/docs/_CROSS-DOC-CONTRADICTIONS.md` (W19-A01).
This file condenses it into the final deliverable's canonical list.

## Doc-vs-doc

| # | This doc says | That doc says | Resolution (proof) |
|---|---|---|---|
| C1 | release.yml:26 "THIS WORKFLOW HAS NEVER EXECUTED (CI billing blocked)" (present tense) | RELEASE.md:145-149 "billing unblocked 2026-11-12, executed; v0.1.0–v0.1.2 shipped" + README:127-131 + KNOWN-LIMITS:17 | Undecidable from inside the tree (no Actions access); one side is false in its present tense — the workflow file's comment is wave-30-edited yet unrefreshed |
| C2 | PARITY-REPORT.md:109 64/252, "windows/linux/macos identically" | live matrix + executed parity-report: 66/61/66, asymmetric | Live run wins; PARITY-REPORT is 7+ days stale; KNOWN-LIMITS:17 concedes the refresh is owed |
| C3 | PARITY-REPORT §3 "result: PASS" | executed feature-audit: FAILURE, 2 R4 violations | Executed run wins |
| C4 | KNOWN-LIMITS.md:52 "no `<canvas>` exists in `ui/src`" (pinned by a test) | MediaSurface.tsx:174 mounts the canvas; putImageData at :225-227; w31.4 commit e983c683 | Code wins; the doc row and its pinning test are stale |
| C5 | ROADMAP D-017 "61 commands / 23 events … pins the exact counts" | live catalog lists 65 commands; 2 control-server tests RED over the drift (executed suite) | Live catalog wins; the "authority" doc is under-counting by 4 |
| C6 | ROADMAP D-016 "commit `2d06607`" | no such revision exists in git (executed check) | Doc cites a phantom commit |
| C7 | KNOWN-LIMITS "Opus: no pure-Rust decoder … unplayable" | matrix row implemented since 2026-11-09 (ruopus); suite-passed | Code wins; stale-at-birth doc row |

## Doc-vs-code

| # | The doc/matrix says | The code does | Proof |
|---|---|---|---|
| C8 | engine-info "decoders: mjpeg, pcm_s16le" (static vec, host.rs:1452-1482, pinned by command_surface.rs:125-133) | decodes H.264+AAC MP4 end to end (13 wired codec families per pipeline_host.rs:355-360) | executed open+advance: Ended, 100.0% |
| C9 | decode-bench "no decoder in this build (first vertical: mjpeg + pcm)" | same binary decodes h264/aac | bench.rs:300-334 stale allowlist; executed open run |
| C10 | containers/mp4.rs:27-32 doc claims no h264 decoder dispatch | dispatches h264 (containers/mp4.rs:190/:237/:440 → pipeline_host.rs:328) | W13-A01 chain, executed |
| C11 | PlayerScreen.tsx:36-42 "the DECODE/FRAMES half is engine-wired nowhere" | the same file mounts the w31.4 canvas and the frame channel presents to it | sed-verified quote vs :174/:225-227 |
| C12 | README "URL sources (http/hls) are refused" | http(s) sources open since w31.3 (demux_host.rs:427; product.toml streaming-http=true) | W19-A01 |
| C13 | PERF-VS-VLC "h264/hevc: parsers only, no decoder" | H.264 software decode landed w27 and was executed by this audit | mp4-smoke-open-advance.txt |
| C14 | matrix row 23 (codec-video-h264-software) evidence `crates/codecs/src/video/h264/sw` | path does not exist; the decoder lives elsewhere and IS real | feature-audit R4 (executed exit 1) + open+advance run |
| C15 | matrix row 163 (config-advanced-folding) evidence `ui/src/screens/settings/areas/advanced.tsx` | path does not exist | feature-audit R4 (executed) |
| C16 | worklog.md:2294-2296 "audio output remains the null device engine-wide" | TRUE — and the premise's "no audio device" claim stands | audio-output lib.rs:152-157; cpal.rs:43 |

## The pattern

Every contradiction has the same shape: **the code moved, the paperwork didn't.** Not one
fabricated result was found; seventeen stale claims were. The repo's own w30.10 remediation
entry diagnosed exactly this ("the registers lying in both directions") and the pattern
regressed within one day of that diagnosis.
