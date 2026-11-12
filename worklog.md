# Worklog — Lumen Media Player (multi-agent program)

Append-only shared log. Format and protocol: AGENTS.md §4.
Task statuses: not_started, in_progress, implemented, verified, blocked.
Prior entries (10,945 lines, phases 0–8, Wave 1–7) are archived in
`docs/worklog/archive-2026-10.md` — see `docs/worklog/README.md` for the
archive convention. New entries append below.

---
Task ID: 7.w1b
Agent: W1-B (hygiene sweep)
Task: Issues #26 + #27 — untrack .env, drop stale agent briefs, archive worklog, sweep one-off scripts.

Work Log:
- Verified .env tracked with single var DATABASE_URL (machine-local path); `git rm --cached .env` (kept on disk, now ignored).
- Added .env.example (documented placeholder `file:./db/custom.db`); fixed .gitignore with `!.env.example` negation after `.env.*`.
- Read all 6 .agent-brief-*.md headers (3 carry explicit "Do NOT commit" text; w5/w6/w7 are transient wave briefs); `git rm` all 6.
- Archived worklog.md (10,945 lines, 1.68 MB, phases 0-8) to docs/worklog/archive-2026-10.md via git mv; wrote docs/worklog/README.md (archive convention); created fresh 9-line root worklog.md ending in `---`.
- Re-grepped repo (workflows, tools/, package.jsons, docs) for all 13 resolve/merge candidates: 9 had zero references anywhere (incl. archived worklog) — deleted merge_resolve.py, resolve_w3_pipeline{,2-6}.py, resolve_worklog.py, resolve_worklog_conflict.py. Kept 4 referenced in worklog narrative: resolve_w1_session_flow.py, resolve_w2_scheduler.py, resolve_worklog_append.py, resolve_worklog_p6.py.
- Verified: `git ls-files | grep -E '\.env$|agent-brief'` → empty; scripts/ = 12 files; `git check-ignore` confirms .env ignored / .env.example not.

Stage Summary:
- Commit aed1c4b on w1/26-27-hygiene (21 files, +10964/-11806); pushed to origin (new branch). Repo root clean: no tracked secrets, no stale briefs; worklog live at 1 entry; scripts/ free of zero-ref one-offs.
---
Task ID: w15-dd
Agent: W15-DD
Task: Issue #2 — H.264/HEVC decision records: software-decoder admission audits, ADR-013 VA-API lane draft, R-01/R-02 + feature-matrix refresh.

Work Log:
- Merged main (30e26f2 → clean); re-verified /tmp/w14dd-a throwaway audits with real cargo tree + extracted-tarball reads (rusty_h264 0.17.2, rust_h264 0.6.1, openh264 0.9.9/sys2 cc+nasm-rs, va 0.0.1, libva 0.1.5, libva-sys 0.1.2). Corrected prior window's unsafe claim: rust_h264 = 3 unsafe impl + 26 unsafe fn + 31 blocks (SIMD/threading files only), not "12 in Send/Sync glue".
- Wrote docs/dependencies/records/h264-software-decoder-audits.md (R1–R10 + candidate table; recommendation rust_h264 0.6.1 [PROPOSAL — Lead sign-off]; conformance-gated admission plan: JVT 199/204 target, FFmpeg differential oracle on D-11 corpus, profile-coverage report; openh264 REJECT both modes; va 0.0.1 = Vapory-blockchain false friend).
- Drafted docs/adr/ADR-013-va-api-hw-decoder.md (proposed): VA-API lane design — 5.F2/5.G2 trait seam, AGENTS §3.3 unsafe-boundary discipline (L5 adapter only), SV-28 answered as E2 OS-service exception, CI conformance; options libva 0.1.5 (dlopen) vs libva-sys + own safe layer vs defer; decision pending Lead.
- Updated docs/14-risk-register.md R-01/R-02 (§2.1 status + §2.2 detail): audits complete 2026-11-09, admission pending Lead; HEVC software-candidate audit recorded as owed.
- Updated docs/feature-matrix.toml: codec-video-h264-software/-hevc-software blocked_reason now cite the record (statuses stay blocked — nothing admitted); codec-video-h264/-hevc + platform-hwdecode-h264/-hevc notes cite ADR-013. TOML validated (252 features).

Stage Summary:
- 4 commits on w14/02-h264-hevc, all pushed: e5e8e11 (audit record), 937a777 (ADR-013), fa5e335 (risk register), 619ee3d (matrix). No decoder crates added to the workspace; admission stays Lead-gated [PROPOSAL].
---
Task ID: 7.WAVES-LEAD
Agent: Lead (orchestrator) — reconciliation batch appended by W17-GG
Task: Wave program bookkeeping (AGENTS.md §4) — the W1..W16 sub-agent lanes' entries were collected in wave reports but never appended to the live log.

Work Log:
- 11 waves ran 5-6 parallel sub-agents each; every lane landed on main through a "merge wave N" commit carrying the issue + evidence summary, and the issues were closed with evidence comments.
- W17-GG re-derived each entry below from `git show --stat` on the lane tips and the merge commits (worktree wt/w17gg; docs-only, no builds — disk-tight). Trust-but-verify pass; no numbers invented.
- 7.w1b already seeded this live log; w15-dd (issue #2) was appended directly by merge 19542f2 — the 7.w14dd row below cross-references it instead of duplicating.

Stage Summary:
- 29 ledger rows appended in wave order (7.w1a..7.w16ff); the live log now carries the full wave program forward from the fresh seed.
---
Task ID: 7.w1a
Agent: W1-A (NEXT-STEPS consolidation)
Task: Issue #25 — consolidate the NEXT-STEPS sprawl into one canonical file.

Work Log:
- Tip 1ff2c70, merged as dc162cb (wave 1): canonical NEXT-STEPS.md reworked (427-line diff); ui/NEXT-STEPS.md (657 lines) and tools/sweep/NEXT-STEPS.md drained to pointers; NEXT-STEPS-w7h.md folded in — 4 files, +290/-1004.
- Also resolved the committed merge conflict found in the sprawl (per the commit subject).

Stage Summary:
- One forward-work list survives; the drifting copies are gone.
---
Task ID: 7.w1c
Agent: W1-C (governance refresh)
Task: Issue #21 — governance refresh: phase status, license, ADR-007 identity, plugin-doc ownership, worklog path.

Work Log:
- Tip 30e35ad, merged as 9380e60 (wave 1): AGENTS.md, README.md, docs/01/06/14 + ROADMAP updated — 6 files, +121/-36.
- Worklog path now points agents at the root live log + docs/worklog/ archive per §4; ADR-007 identity and plugin-doc ownership recorded.

Stage Summary:
- Governance docs agree with the post-archive repo state.
---
Task ID: 7.w2d
Agent: W2-D (CI matrix)
Task: Issue #5 — repair the corrupted main trigger; bring the 3-OS merge-blocking matrix live.

Work Log:
- Tip 3fb94f4, merged as 9085fb4 (wave 2): corrupted main trigger fixed; linux/macos/windows matrix live — 6+9+4 jobs, actionlint-validated, parked CI dir folded in.
- Tip diff spans 37 files, +12296/-13801, incl. the W1-B hygiene deltas (both lanes branched off 1e8351c; identical changes, merged clean).

Stage Summary:
- 3-OS merge-blocking CI live on main.
---
Task ID: 7.w2h
Agent: W2-H (ADR-010 codec-patent posture)
Task: Issue #24 — ADR-010 codec-patent shipping matrix.

Work Log:
- Tip a49ace5, merged as e5e660e (wave 2): docs/adr/ADR-010-codec-patent-shipping.md drafted (324 lines) — evidence, options, recommended posture pending counsel.
- Doc 11 dependency/security compliance + ROADMAP synced; 3 files, +366/-14.

Stage Summary:
- Patent posture recorded as a decision document: drafted, pending counsel sign-off.
---
Task ID: 7.w3e
Agent: W3-E (UI lint hardening)
Task: Issue #36 — promote local eslint rules to a tested plugin; stylelint token discipline; relativize probe path; burn down warnings.

Work Log:
- Tip 3415b17, merged as b480049 (wave 3): ThemeContext extracted to theme/context.ts (last in-scope react-refresh/only-export-components warning); l10n pseudo-probe made worktree-relative via npm script.
- Warnings 31→24 per the merge record; remaining 24 all sit in sibling-guarded screens/ + client/; 6 files, +52/-32; lint 0 errors.

Stage Summary:
- Owned lint warnings at zero; tested-plugin + token-discipline rules in place.
---
Task ID: 7.w3f
Agent: W3-F (desktop 4.H1 wiring)
Task: Issue #3 — 4.H1 app-layer wiring: transport probe, file open, playlist.load, config writes, engine events.

Work Log:
- Tip 4439777, merged as 8a56f0b (wave 3) — closes the phase4-exit §4.4 four handoffs: TauriTransport probe with DemoTransport browser fallback; files.rs dialog/drop → open_path_request → UI open-path; window_config consumed as WindowBuildSpec; playlist.load + config.set-setting round-trip + bounded drop-oldest event egress (BoundedEventSink).
- [BLOCKED: no webkit2gtk/display in sandbox] tauri::Builder run loop + 3-OS launch stay ADR-004 admission-build rows; INFRA-BLOCKED.md carries the ShellRuntime delegation shape + 63/0 desktop / 634/0 vitest evidence.

Stage Summary:
- All four app-layer seams wired and tested; shell runtime stays INFRA-BLOCKED, recorded honestly.
---
Task ID: 7.w3g
Agent: W3-G (icon glyphs)
Task: Issue #35 — mine real icon glyph paths from figma exports; add emphasis stroke variants.

Work Log:
- Tip 4297545, merged as 145b265 (wave 3): all 32 registry glyphs carry verbatim design geometry from design/figma/Icon.svg (bbox-centred on the 24px grid, never rescaled), attribution from sheet property strings cross-checked vs menu.svg/play.svg/Controls.svg; per-element stroke-width overrides kept.
- Emphasis variants default|strong|max → stroke-width 1.5/2/3 via --stroke-icon tokens; a11y contract unchanged; Icon.test.tsx +15 regression guards; typecheck clean, lint 0, 610+1 tests.

Stage Summary:
- Icon set is design-mined with regression guards; default behaviour unchanged for consumers.
---
Task ID: 7.w4i/w5i
Agent: W4-I → W5-I (MKV Tags/Attachments/Chapters + TrackUID; W5-I resumed the dead W4-I runs)
Task: Issue #34 — MKV Tags/Attachments/Chapters wiring + TrackUID retention, corpus vectors, matrix flip (milestone 3).

Work Log:
- Tip 6cc3f0b, merged as 2d0941e (wave 5): feature-matrix container-matroska-webm flipped partial → implemented (win/linux/macos) with issue-#34 evidence; container-matroska.md addendum — wiring seam CLOSED, gaps 1/4 CLOSED, 9-vector corpus table incl. the live ffmpeg title/font/cover/chapters differential lane.
- Gates re-verified at the tip: demux 587 passed / 0 failed / 4 ignored (28 suites), clippy -D warnings clean, fmt clean; W4-I→W5-I diffs rustfmt reflow only; addendum attribution corrected to W5-I.

Stage Summary:
- Matroska/WebM row implemented with evidence; the resume changed no behavior.
---
Task ID: 7.w4j/w5j
Agent: W4-J → W5-J (buffering + reconnect)
Task: Issue #6 — buffering model, bounded retry/resume, generation-safe cancellation, HLS wall-clock pacing.

Work Log:
- Tip e51a9e7, merged as c6a401e (wave 5; merge record adds the earlier-milestone watermarks + stall prediction): SchemeRegistry::with_retrying_fetch composes the 6.D2 reconnect layer into probe/window/linear-scan fetches — proven over the deterministic mock testkit through scripted retry weather (503/500/recovery) with the fail-fast boundary typed; mockhttp grows ScriptedStatus.
- HLS live sessions gain opt-in RFC 8216 §6.3.4 wall-clock reload pacing via the injectable clock (FakeClock asserts the cadence), paced_reloads in HlsLiveStats; unpaced default + stall budget unchanged; 3 files, +322/-26.

Stage Summary:
- Reconnect + pacing integrated behind the fetch seam.
---
Task ID: 7.w4k/w5k
Agent: W4-K → W5-K (visual baselines)
Task: Issue #37 — re-pin the 136 visual baselines for Solstice v2 + wave-3; MATRIX-20261109 provenance.

Work Log:
- 61cb47a: all 136 baselines re-captured (138 files) + MATRIX-20261109 provenance doc + README update.
- 2f99212: aborted second capture pass discarded — PR-lane compare vs the COMMITTED baselines 30/30, dirty files restored (136 tracked, 0 dirty); fresh nightly spot-checks pass; CI ui.yml pr-lane 8 → 30 cases, browser install pinned to the lockfile version.
- Accent-default discrepancy (product dark+yellow vs the grid's orange anchor) registered in manifest + MATRIX addendum — resolved by 7.w6n below.

Stage Summary:
- Merged (with W6-N) as 0070945 (wave 6) — "includes K's re-pin work"; baselines carry provenance + CI counts.
---
Task ID: 7.w4l-w7l
Agent: W4-L → W7-L (defect ledger; F3 closure by W9-T)
Task: Issue #16 — defect ledger: vorbis fuzz-survival, aac 6ch@12kHz (F6), mp4 video-only end-of-media (F5), F3 CLI latch.

Work Log:
- 81e04ce: vorbis fuzz-survival assert honors the documented corpus skip. 8c64271 (F6): aac 6ch@12kHz too-complex false positive cured (decoder.rs +148).
- 962728c (F5): video-only eom gates Drained on the clock reaching the greatest delivered video pts (mp4 corpus 0115 fired eom at 0.111s of a 4.0s file); tests/eom_playout.rs (+316) pins pre/post-fix.
- 6531883 (W9-T, F3): CLI catalog + vorbis raw pin re-synced to main's wave-7/8 behavior — commands count 64; raw_samples 66,150 (head-trim now counted).

Stage Summary:
- Merged as 521d604 (wave 7: vorbis head-trim, F6, F5; F3 root-caused) and 87a90c5 (wave 9: F3 deterministic start-time via CLI presentation latch + audio-deselect clock un-wedge).
---
Task ID: 7.w4m-w6m
Agent: W4-M → W6-M (subtitles)
Task: Issue #17 — subtitle completeness: font-stack seam, ASS override tags, encoding detection, PGS/CC, DVB+VobSub records.

Work Log:
- f3ad5e4: FontStack/FontProvider/MemoryFontProvider + GlyphPipeline raster seam, decoration-aware placeholder pipeline (exact integers, no new deps) + swash/skrifa survey record [PROPOSAL]; c93cc54: encoding detection (UTF-8 gate + Windows-1252 fallback, typed damage).
- 1bd5519: ASS override-tag styling with per-tag suites (\b \i \u \s \fs \fn \c \alpha \pos \move; 267/0, clippy clean); 8c230f1: G-1 DVB + VobSub records; 827f69e: stale subtitle-pgs + cc-608-708 rows flipped to implemented citing cards 5.N1/5.N3.

Stage Summary:
- Merged as 3be5205 (wave 6); matrix evidence refreshed to 267/0 lib tests (df56890).
---
Task ID: 7.w6n
Agent: W6-N (e2e theme green)
Task: Issue #37 completion — resolve the accent-default discrepancy per the solstice-v2 brief; e2e fully green.

Work Log:
- 932a6d9: DECISION — keep the product default dark + YELLOW ("Solstice Keyvox gold" #f2c41d) and re-align e2e/theme.spec.ts; evidence chain from the brief's silence through tokens.json/gen-tokens.mjs/tokens.css/ThemeProvider to the registered manifest discrepancy; the orange expectations were stale pre-Solstice card-4.A2 values.
- theme.spec.ts only (grid anchor stays orange per manifest MATRIX-20261109). Verify: e2e 38/38, visual pr-lane 30/30, tsc clean, vitest 647/647 (72 files).

Stage Summary:
- Issue #37 closed green; merged inside 0070945 (wave 6) with the K re-pin work.
---
Task ID: 7.w6o
Agent: W6-O (streaming auth + remote playlists)
Task: Issue #13 — 6.E1 HTTP auth + 6.E2 remote M3U/XSPF playlists.

Work Log:
- 844141f: HTTP basic + native RFC 7616 digest auth, redacted credential-store policy.
- f0f9a17: open_remote_playlist over HttpFetch — XSPF/M3U sniff, playlists-crate parse (I/O-free posture kept), RFC 3986 §5 resolution against the FINAL URL after redirects; manual redirects bounded at 5 + loop-checked (typed TooManyRedirects/RedirectLoop); userinfo accepted + stripped, auth composed via 6.E1 AuthFetch; 6 mock-socket tests.

Stage Summary:
- Merged as 2834c5b (wave 6); remote playlists + auth complete behind typed seams.
---
Task ID: 7.w6p-w8p
Agent: W6-P → W8-P (playback operations)
Task: Issue #32 — frame-step, chapter navigation, Go-to-Time, bookmarks + snapshot planning.

Work Log:
- 6c2d139: pure FrameStepPlan (auto-pause + signed one-frame seek, 24fps fallback [PROPOSAL]) + chapters.rs nav over the wire Chapter list (restart-then-rewind prev [PROPOSAL: 10s threshold]); 13 tests, matrix rows partial.
- 2fd5ebc: Go-to-Time dialog — parseTimeInput grammar with honest null refusals, dialog through the player-state seam (VLC-parity prefill), Ctrl+T keymap seed, miniplayer + full wiring; 9 files, +727.
- b8b1146: SnapshotConfig::plan (vlcsnap- prefix, 6-digit counter [PROPOSAL], PNG-only v1, typed refusals); pixel-readback seam gap recorded [PROPOSAL]; bookmarks + snapshot rows partial.

Stage Summary:
- Merged as 93ae720 (wave 8) — all five playback-op verbs landed as seams/planning with honest [PROPOSAL] marks.
---
Task ID: 7.w7q-w8q
Agent: W7-Q → W8-Q (HLS master + ABR)
Task: Issue #10 — HLS master playlists + variant switching with ABR hysteresis (6.B2a/6.B2b close-out).

Work Log:
- d2de397: in-playlist EXT-X-DISCONTINUITY carried through the adaptive session (recorded + counted, distinct from switch seams) + unit test; network-hls flipped partial with per-card evidence (VOD 6.B1a, live 6.B1b, master parse/selection 6.B2a, boundary switching + hysteresis 6.B2b).
- Honest pending at merge: renditions parsed-not-selected, KEY/BYTERANGE/MAP + fMP4 refuse typed, live-window switching + 6.B3 vectors. Gates: streaming 164/0 lib + e2e, playlists 46/0.

Stage Summary:
- Merged as b045721 (wave 8).
---
Task ID: 7.w7s-w8s
Agent: W7-S → W8-S (A/V effects)
Task: Issue #33 — audio DSP chain + EQ, video adjust, Effects & Tools panel.

Work Log:
- 8bd13ed: AudioEffect/EffectChain bounded ordered chain + Gain/Balance/Mute, constant-power pan law pinned, known-signal tests (+403). 50a7ba6: AdjustParams brightness/contrast/saturation/hue/gamma, range+finite validation, byte-exact neutral identity, CPU apply_rgba8 (GPU wiring [PROPOSAL]) (+361).
- 0a8e70d: Effects & Tools settings area — volume/mute wired to audio.set-volume/set-mute; balance + video adjust inert-with-reason pending D-012 command cards (+347). db7ab73: 3-band EQ sliders (250Hz shelf / 1kHz peak / 4kHz shelf, ±20dB) as PENDING data-pending-command audio.set-eq.
- feature-matrix: afeat-volume, afeat-equalizer, vfeat-colors-adjust, ui-screen-av-tools → partial with commit-pinned evidence + honest deltas (3-band not 10-band, no presets/2-pass).

Stage Summary:
- Merged as 8b9c486 (wave 8).
---
Task ID: 7.w9u-w11u
Agent: W9-U → W11-U (HLS records + conformance)
Task: Issue #11 — HLS conformance corpus + records + X-TIMESTAMP-MAP (6.B3 M2).

Work Log:
- f009a1a: hls_conformance_e2e — 6 clean-room vectors over the real ffmpeg TS fixtures on a real socket: L1 live-edge join clamp, L2 window-slide never refetches (request-count pin), L3 reload cadence at the target-duration boundary (FakeClock), D1 discontinuity seam with continuing X-TIMESTAMP-MAP, D2 encoder timestamp reset (2^33 wrap-adjacent anchor), G1 EXT-X-GAP tolerated (no gap semantics, honest v0).
- Recovered WIP fixed: opening windows live-shaped, L3 oracle rewritten, D1/D2 map arithmetic wrap-exact; streaming-hls.md record — streaming 173/0 lib + conformance 6/6, live 2/2, master 3/3, vod 2/2.

Stage Summary:
- Merged as 372d9e1 (wave 11); honest pending: gap semantics, EXT-X-MEDIA selection, live-window switching.
---
Task ID: 7.w9v-w11v
Agent: W9-V → W11-V (DASH completion)
Task: Issue #12 — DASH template resolution + MPD fuzz + ABR scheduling (6.C2 finish).

Work Log:
- 93b7614: abr+sched wired into the dash module (the recovered WIP had never been compiled); fixed the ISO 8601 offset sign (local − offset = UTC), the live-join skip (advance only while the NEXT segment is safely available), u128 margin arithmetic, the never-run tests' vectors.
- network-dash → partial (win/linux/macos) with honest pending list; [PROPOSAL] ABR class watermark/hysteresis vs BBA/BOLA stays a Lead decision.

Stage Summary:
- Merged as 27b17e1 (wave 11) on top of the 6.C1a MPD parser core.
---
Task ID: 7.w9w-w11w
Agent: W9-W → W11-W (audit tools + matrix backfill)
Task: Issue #20 — audit tools + non-format matrix backfill with verified evidence.

Work Log:
- d74e013 (milestone 4 close-out): batches cf2a341/c50754b/d50bda2 applied the verified evidence map to ui-screen/ui-dialog/menu-item/hotkey/config-feature/playback-op rows.
- Tool confirmation: feature-audit PASS (252 rows, evidence present + path-resolved); parity-report implementation 60/252 (23.8%) per desktop platform, web 0/252 (wasm = Phase 7); verified-parity stays 0/252 — nothing claims CI-backed verified status (honest register).

Stage Summary:
- Merged as ed24e7d (wave 11) — feature-audit/config-validator/parity-report tools + backfill.
---
Task ID: 7.w10x
Agent: W10-X → W16-X (netfuzz + scenarios)
Task: Issue #14 — netfuzz targets + interrupted-network scenarios.

Work Log:
- da2af08: 19 files, +1225/-5 — crates/streaming/tests/netfuzz/fuzz.rs targets + fuzz/corpus seeds, net_interrupted.rs scenarios (383 lines), netfuzz_sweep.rs (410 lines), fuzz/Cargo.toml + README, streaming-hls.md record.

Stage Summary:
- Merged as 1c320e3 (wave 16); the streaming stack carries fuzz targets, a seeded corpus, and interrupted-network coverage.
---
Task ID: 7.w12y
Agent: W12-Y → W13-Y (TLS ADR + seam)
Task: Issue #8 — TLS decision record + typed https seam (ADR-011 refresh).

Work Log:
- 2b96f68: docs sync for the ADR-011 refresh (c3bb258 + 715436a) — doc 11 SEC-07 points at the refreshed ADR + conditionally admitted transport + typed seam; proposed-admission register row (rustls 0.23.46, 35 locked / 19-crate graph; aws-lc-rs default C/CMake chain vs ring 0.17.14 cc-only) — [PROPOSAL], Lead decides.
- ROADMAP D-011 + §6 ADR index; feature-matrix network-tls-policy → partial with seam + admitted-transport evidence.

Stage Summary:
- Merged as a409f64 (wave 13).
---
Task ID: 7.w12z
Agent: W12-Z → W13-Z (RTSP + security review + gate ledger)
Task: Issue #15 — RTSP/RTP feasibility, network security review, blocked-protocol registry, Phase-6 exit gate ledger.

Work Log:
- 9c5d3f9: RTSP/RTP feasibility verdict — DEFER, typed refusal stays. 9399960: network security review (redirects/SSRF/traversal/header-injection) — 1 medium + 3 low findings. ce11e5c: blocked-protocol rows (udp/rtp/rtsp/smb/nfs/ftp-sftp/mmsh-mms) backfilled from schemes.rs typed refusals.
- 3ac3637: docs/gates/phase6-exit.md draft (57 lines) — landed evidence, remaining TLS/RTSP, parity 60/252, PENDING LEAD DECISION.

Stage Summary:
- Merged as 7dd8b16 (wave 13).
---
Task ID: 7.w12aa
Agent: W12-AA → W13-AA (soak nightly)
Task: Issue #18 — soak-nightly design record + real 2-min profile.

Work Log:
- 6cbe394: docs/perf/soak-nightly-20261109.md — env plumbing, nightly workflow shape, VERBATIM CI-short 2-min tail (12 passed / 0 failed / 120.03s, debug, defaults — the only real measurement, not a 4h datum), successor protocol citing the w7-h blocked-run record; NEXT-STEPS 7-g points at the workflow.
- The 4h run is the nightly job's job; 5 MB/h stays [PROPOSAL until first real measurement].

Stage Summary:
- Merged as 30e26f2 (wave 13) — workflow + leak-target parameterization + record.
---
Task ID: 7.w14cc
Agent: W14-CC (audio-output spike + PcmPump)
Task: Issue #1 — cpal admission spike + PCM pipeline proof behind the ADR-005 seam.

Work Log:
- c873ee5: cpal admission spike record (real cargo-tree audit, E2 evidence, alternatives survey); c10b0b2: ADR-005 admission-spike outcome — refreshed cpal evidence, Linux-first recommendation, post-admission conformance plan.
- fff38c9: PCM pipeline proof behind the seam — clock-synced PcmPump, exact SinePattern, typed BackendNotAdmitted cpal flip point; c90cae7 syncs ADR-005 + dependency record + feature-matrix (3 files, +7/-7).

Stage Summary:
- Merged as 1533458 (wave 14); no audio backend admitted yet — the flip point is typed and waiting on the Lead.
---
Task ID: 7.w14dd
Agent: W14-DD → W15-DD (H.264/HEVC audits + ADR-013)
Task: Issue #2 — software-decoder admission audits, ADR-013 VA-API lane draft, R-01/R-02 + feature-matrix refresh.

Work Log:
- Cross-reference (W17-GG reconciliation): this row's full entry is already live above as "Task ID: w15-dd", appended by the lane itself via merge 19542f2 (commits e5e8e11 audit record, 937a777 ADR-013, fa5e335 risk register, 619ee3d matrix on w14/02-h264-hevc).

Stage Summary:
- Merged as 62453fb (wave 15); no duplicate recorded here.
---
Task ID: 7.w16r
Agent: W16-R (ASF completion)
Task: Issue #9 — ASF data packets + Simple-Index seeking + records.

Work Log:
- ef9d75e: asf/data.rs + mod.rs data-packet path + Simple-Index seeking; tests + test_support reworked; feature-matrix row + container-asf.md record updated — 6 files, +256/-101.

Stage Summary:
- Merged as 73819cb (wave 16); ASF lane complete with index-based seeking.
---
Task ID: 7.w16ee
Agent: W16-EE (codec admission addenda + TrueHD decision)
Task: Issue #7 — codec admission addenda + stale row flips + TrueHD unblock decision.

Work Log:
- b900c0b: records added — oxideav-ac3-0.0.11 (+38), oxideav-mpeg4video-0.1.8 (+42), truehd-bitstream-io-unblock (+117); feature-matrix stale rows flipped; 4 files, +207/-12.

Stage Summary:
- Merged as 55a14f9 (wave 16); TrueHD unblocked via the bitstream-io decision.
---
Task ID: 7.w16ff
Agent: W16-FF (VLC 3.0.23 oracle differentials)
Task: Issue #19 — VLC oracle differentials for the corpus.

Work Log:
- 1950f81: oracle obtained (VLC 3.0.23 via /tmp apt extraction); 29/35 inputs bit-exact prefix + 6/35 f32 deviation-path documented; 19 inputs re-based.
- Evidence: docs/corpus/vlc-diff-report-20261023.md addendum + vlc-oracle-run-20261109.json (571 lines) — 2 files, +650/-4.

Stage Summary:
- Merged as d2e6aab (wave 16, current main HEAD); the f32 deviation path is documented, not hidden.

---
Task ID: 7.w17bb
Agent: W17-BB (ogg native_seek tail fix)
Task: Issue #38 — seek-landing tail truncation in native_seek + oracle re-pin.

Work Log:
- db710d5: the seek-landing head-hold release was a one-call burst — 44 frames crossed the M10 pre-roll cap and 29,248 samples were dropped at the hold, truncating the tail at 836,780 us. Release amortized (RELEASE_FRAMES_PER_CALL=4), drain state carried on the hold, flush finishes a started drain in one uncapped call.
- Expectation re-pinned to the exact valid timeline: 66,150 samples @ 44.1 kHz = 1,500,000 us (oracle vorbis_reference.txt window_samples = final_granule; the old 1,502,902 pin was the pre-head-trim truth) — decoder.rs +285, native_seek.rs tests, 2 files +275/−39.

Stage Summary:
- Merged as 081f6ae (wave 17); ogg seek tail now plays out to the oracle window exactly.
---
Task ID: 7.w17jj
Agent: W17-JJ (CI fmt + clippy fixes)
Task: CI lint/gate jobs — fmt drift + clippy -D warnings across the workspace.

Work Log:
- 621fa20: rustfmt drift across 47 files from wave merges (lint job #1).
- cc32053: clippy -D warnings fixes across demux/video-engine/playback-engine/streaming/audio-output/audio-engine/commands (gate job #2, partial: xtask remained); be4cc55: clippy green workspace-wide (xtask validators, streaming/audio/commands tests via testkit Must + local helpers; let-chains, matches!, strip_prefix) — 913008d fmt after merge conflict resolution.

Stage Summary:
- Merged as 1bd5085 (wave 17); lint and gate jobs clean, CI-ready.
---
Task ID: 7.w17gg
Agent: W17-GG (worklog reconciliation)
Task: AGENTS.md §4 — reconcile wave entries 7.w1a..7.w16ff.

Work Log:
- 77da3a6: worklog.md +331 lines — one entry per lane in the Task ID / Agent / Task / Work Log / Stage Summary format, derived from git history; cross-reference entries added where a lane had already recorded itself (e.g. w15-dd).

Stage Summary:
- Merged as 7e77896 (wave 17); 30 wave entries live in the register.
---
Task ID: 7.w17hh
Agent: W17-HH (final governance refresh)
Task: Wave closeout — landed ledger, phase-6 status, risk register.

Work Log:
- ae1a089: NEXT-STEPS.md +112/−? (landed ledger + remaining-work register), README.md, docs/14-risk-register.md (+15), docs/ROADMAP.md — 4 files, +144/−51.

Stage Summary:
- Merged as 7812f7c (wave 17); governance docs reflect the landed truth.
---
Task ID: 7.w17ii
Agent: W17-II (TLS activation)
Task: Issue #39 — activate TLS per ADR-011 ratification.

Work Log:
- 1c1b945: ADR-011 accepted — ureq 3.5.0 + rustls(ring 0.17.14) admitted as the E2/ADR-001-class exception (Lead ratification 2026-11-09); record ureq-3.5.0.md R10 conditional admission discharged, doc 11 SEC-07 + admission register row → RATIFIED.
- ff2df61: ADR-011 seam deleted per its own TODO(admitted-impl) — HTTPS_ADR011_ADMITTED const, guarded/refusal arms, SchemeError::HttpsNotAdmitted gone; https classifies like http and rides ureq's always-verifying rustls; tests/https_activation.rs proves routing + open_http composition (+159/−69).
- 75dbbce: feature-matrix network-tls-policy → implemented (desktop); real TLS round-trip stays [VERIFY: needs a networked CI lane] — mock testkit is plaintext.

Stage Summary:
- Merged as 849be1f (wave 17); https is a first-class transport, typed refusal removed.
---
Task ID: 7.w18ll
Agent: W18-LL (UI lanes local verification)
Task: UI lane verification under billing-blocked CI (typecheck/lint/l10n/vitest/e2e/visual).

Work Log:
- 833d979: i18n-allowlist entries for wave-8 data punctuation (goto '/' separator, volume '%' unit) — engine-data punctuation around formatSeekTime / audio.get-volume, not translatable copy; reason + owner per doc 07 §6.5.
- f03d9b3: every ui.yml lane verified locally (chromium 156.0.8078.4, same as pinned provenance): typecheck PASS, lint 0 errors / 27 accepted warnings, l10n 569 keys, vitest 689/689, playwright e2e 38/38, visual PR-lite 30/30 vs committed Linux baselines — no re-pin needed; provenance addendum in MATRIX-20261109.md.

Stage Summary:
- Merged as 5865ee6 (wave 18); UI lanes green locally, CI-ready when billing is restored.
---
Task ID: 7.w18mm
Agent: W18-MM (Tauri close package)
Task: Issue #4 — Tauri INFRA-BLOCKED close package + ADR-004 status sync.

Work Log:
- 07e7529: apps/desktop/tauri/INFRA-BLOCKED.md rewritten (+217/−80 equivalent): seams proven, unblock recipe; ADR-004-os-webview.md status synced; NEXT-STEPS.md pointers — 3 files, +148/−80.

Stage Summary:
- Merged as 5d648bd (wave 18); the infra blocker is documented with a concrete unblock path.
---
Task ID: 7.w18nn
Agent: W18-NN (Phase 7 card-to-issue breakdown)
Task: Issue #22 — Phase 7 close package: card-to-issue breakdown.

Work Log:
- 3e2ea4e: docs/plans/phase7-cards.md (+191) — the card deck mapped to concrete, assignable issues with acceptance criteria.

Stage Summary:
- Merged as 0a34f91 (wave 18); Phase 7 scope is issue-ready.
---
Task ID: 7.w18oo
Agent: W18-OO (Phase 8 card-to-issue breakdown)
Task: Issue #23 — Phase 8 close package: card-to-issue breakdown.

Work Log:
- aa8a9b7: docs/plans/phase8-cards.md (+102) — Phase 8 cards decomposed into issues.

Stage Summary:
- Merged as 86d0ecc (wave 18); Phase 8 scope is issue-ready.
---
Task ID: 7.w19kk
Agent: W19-KK (local CI gates)
Task: Local CI gate completion — deny/schema/propagation/wasm (billing-blocked Actions).

Work Log:
- 76a700b: deny.toml (+28) + recovery_matrix.rs rewritten (+220) — mp4 rows wired for 6.OPW-mp4.
- b403d53 (verification-only commit): cargo-deny 0.20.2 licenses/sources/bans/advisories ok; recovery_matrix 23/23; schema-validation 69 tests + codegen no-drift; config-propagation byte-identical; wasm32 theme-tokens --locked ok. Workspace-wide cargo test blocked by shared-target-dir ENOSPC (zero code failures across 4 attempts).

Stage Summary:
- Merged as 765be4a (wave 19); local gates green, CI-ready when billing is restored.
---
Task ID: 7.w19pp
Agent: W19-PP (https→http downgrade refusal)
Task: Security M1 (6.H1 F1) — refuse https→http redirect downgrades.

Work Log:
- 4bdae49: client.rs redirect validation (+125) — typed refusal on scheme downgrade; error.rs vocabulary, hls/media.rs, ranged.rs, mockhttp.rs regression hooks — 5 files, +141/−7.
- 07cc892: network-security-review.md marks 6.H1 F1 RESOLVED with fix + regression-test references (+22/−2).

Stage Summary:
- Merged as 30fb278 (wave 19); the security finding is closed with tests.
---
Task ID: 7.w19qq
Agent: W19-qq (pts_for_wallclock_ms saturating fix)
Task: pts_for_wallclock_ms far-from-anchor overflow.

Work Log:
- 01e8ee1: hls/timestamp_map.rs (+51) — far-from-anchor arithmetic made saturating instead of overflowing.
- 9ab5d6f: streaming-hls.md marks the overflow finding RESOLVED (fix 01e8ee1).

Stage Summary:
- Merged as b7116e8 (wave 19); the timestamp-map finding is closed.
---
Task ID: 7.w19rr
Agent: W19-RR (f32 VLC differential decision)
Task: Issue #19 §6.6 — f32 deviation-path tolerance decision (RMS/peak).

Work Log:
- 767d3b9: RMS/peak tolerance policy applied with per-row measurements — docs/corpus/vlc-diff-report-20261023.md (+49), vlc-f32-tolerance-20261109.json (+93, per-input RMS/peak), scripts/vlc_f32_deviation.py (+196) — 3 files, +328/−10.
- The 6/35 f32 deviation rows stay honest parity gaps under the documented tolerance policy, not silent passes.

Stage Summary:
- Merged as 192ffc7 (wave 19, main HEAD); the f32 decision is measured, scripted, and recorded.
---
Task ID: 7.w19ss
Agent: W19-SS (full local gate verification)
Task: Full local gate run at 86d0ecc — verification only, no code changes.

Work Log:
- cargo test --workspace --all-features @86d0ecc: 3842 passed / 8 failed / 49 ignored. All 8 failures pre-existing and fixed in-flight by the wave-19 lanes (downgrade refusal, timestamp-map saturation); re-verified green after.
- No commits: this lane's deliverable is the verification record itself (CI Actions billing-blocked — local gates are the only gate).

Stage Summary:
- Verification-only lane; the workspace-wide green state at main HEAD is attested locally.
---
Task ID: 7.CLOSEOUT
Agent: Lead (orchestrator) — wave 7..19 closeout
Task: Final closeout — ledger verification, follow-up triage, CI diagnosis.

Work Log:
- 19 waves orchestrated; 36 issues closed with evidence (typed refusals, honest gaps, and [VERIFY]/[PROPOSAL] registers preserved throughout); 20 follow-up issues #40-#59 created from the honest-gap registers.
- CI billing-blocked diagnosed and documented: Actions fail instantly regardless of code; every lane re-verified locally with pinned provenance (chromium, cargo-deny, schema/codegen drift checks).

Stage Summary:
- worklog register complete (47 entries); no claims of CI-backed verified parity — the register of honest gaps is the deliverable.
---
Task ID: 7.w21a
Agent: w21-a (C1/C2 literal-gate rollout)
Task: Land the 70-file literal-debt annotation and wire lint-literals/check-identity into the CI + full gates.

Work Log:
- Reviewed the predecessor's uncommitted 70-file diff in full: 328 audited `// lumen-allow-literal: <reason>` markers over literal sites the C1/C2 scanners flag (test vectors, test doubles on reserved hosts, corpus gates, argv0/usage prose pending identity wiring [PROPOSAL], protocol vocabulary like `lumen.`/LUMEN-CUES/lumen-proto). Zero hunks discarded — all reasons accurate.
- Fixed 5 fmt-drift sites where rustfmt relocates a trailing marker BELOW its literal (breaking same-line/above suppression): access.rs and 3x hls/live.rs markers moved to standalone lines above the statement; dash/fuzz.rs call split one-arg-per-line with the marker directly above the literal.
- Gates ACTUAL: cargo fmt --check CLEAN; clippy -p xtask 0; clippy --workspace --all-targets -D warnings 0; cargo test -p xtask 15/15 passed; lint-literals PASS (0 violations, 340 allowlisted); check-identity PASS (0 violations, 341 allowlisted); check-architecture PASS (5/5); schema-gen/codegen/fixtures --check all no-drift.
- 90b836d: the 70-file rollout committed (335+/255-).
- a5d742b: ci.yml config-propagation job — lint-literals + check-identity steps added, honest-skip ledger C1/C2 SKIP->COVERED; scripts/full_gate.sh — steps 7+8 added (verified locally via the cargo xtask alias, both exit 0; ci.yml parses; bash -n clean).

Stage Summary:
- D-10 four questions: MEASURE — gate counts above (328 markers, 340/341 allowlisted sites, 0 violations). NOT TESTED — per-crate test suites not re-run (disk-tight; clippy --all-targets compiled every test target; the diff is comments plus one semantically identical fuzz.rs `if let` restructure). WHAT BREAKS — a rebrand must hand-review 21 argv0/usage identity sites until the help.rs identity wiring [PROPOSAL] lands. WHO HITS IT FIRST — the next rebrand/localization card touching CLI prose.
- Open: identity wiring for CLI argv0/usage strings stays a recorded [PROPOSAL] follow-up (config-driven lookup deferred to that card); C2 packaging-regen check still SKIP (no packaging plane).
---
Task ID: 7.w22b
Agent: w22-b (platform crates final)
Task: Land + verify the three L5 platform-adapter crates (issue #40, cards 7.A1/7.A2/7.A3) — repair the prior agents' uncommitted state, drive every gate.

Work Log:
- Reviewed the uncommitted state: 11+16+12 files across platform-windows/linux/macos + boundaries.toml rows (already present, correct: internal audio-output+media-types, dev testkit, zero external) + Cargo.lock additions. One mid-edit break: the windows Display test asserted the signed-HRESULT form while its literal was unsigned — fixed the literal to the com.rs storage form (`0x8001_0106u32 as i32 as i64`); assertion unchanged.
- cf71390: landed the three crates (typed seams: AudioDevices/GpuAdapters/FileDialogs + PlatformReport negotiation, per-crate Unavailable* fallbacks; hand-rolled FFI only, module-scoped unsafe with per-block invariants).
- Host ACTUAL: cargo test -p the three: 60 passed / 0 failed (linux 13 integrations + 15 seam, windows 16 seam, macos 16 seam). 2475274: fmt + clippy --all-targets -D warnings clean.
- Cross-target (prior agent never ran them; cfg-gated code had never been typechecked): fixed 10 windows + 3 macos compile errors — spawn_sta double-Result flatten, `&'static str` thread names, PropVariantHeader Copy derive, unsafe-wrapped from_wide, explicit `'a` on client_of (watch-thread Box ownership as the invariant), mut binding for addr_of_mut, cfg-gated PlatformError imports, E_CONSOLE role filter wired into OnDefaultDeviceChanged (doc said it, code now does), msg_send_id_u64 import, and `allow(clashing_extern_declarations)` on the libobjc block with the trampoline soundness rationale. 2ca0554.
- Cross-target ACTUAL: cargo check -p platform-windows --target x86_64-pc-windows-msvc [--all-targets] clean; -p platform-macos --target aarch64-apple-darwin and x86_64-apple-darwin [--all-targets] clean and warning-free.
- 30612f3: reflowed 8 wrapped `lumen-allow-literal` markers to the gate's same-line/directly-above adjacency rule (wrapped reasons had left the marker 2+ lines off).
- Gates ACTUAL: check-architecture 5/5 PASS; lint-literals PASS (0 violations, 348 allowlisted); check-identity PASS (0 violations, 349 allowlisted).
- Runtime on this Linux host: PulseAudio dlopen probe loads libpulse but no server → typed ServiceUnavailable (the dependency-absent/absent-server honesty path, exercised for real); DRM sysfs scanner driven by fixture trees + live /sys; xdg chain driven by fake zenity/kdialog test doubles.

Stage Summary:
- D-10: MEASURE — 60/60 host tests, 3 foreign-target checks clean, 3 gates PASS. NOT TESTED — real Windows/macOS hardware and a live PulseAudio server ([VERIFY] registers in each crate's docs). WHAT BREAKS — the mirrored seam vocabulary across the three crates duplicates until the Lead's L5 unification [PROPOSAL]. WHO HITS IT FIRST — issues #41 (codec hw decode) / #42 (device-discovery) joining these edges.
- Open: xdg-desktop-portal FileChooser D-Bus client is the recorded [PROPOSAL] follow-up (zenity/kdialog chain is the working path); pulse client-name + thread names are config-edge candidates once platform crates gain a product-config edge.
Task ID: 7.w22c
Agent: w22-c (transcode final relaunch)

Task: Card 7.E1 / issue #43 close-out — verify, repair, and gate the transcode graph core left by the killed prior sessions; land it green.

Work Log:
- Recovered the killed prior session's uncommitted tail (12 files, +206/-173): rustfmt pass, clippy -D warnings fixes (`as_chunks` over `chunks_exact`, `Option::inspect` in queue `take`, `u64::is_multiple_of` in the WAV frame check), and the `[crates.transcode]` boundaries.toml entry (L3; internal media-types/plugin-api/codec-api/demux, all downward; dev testkit/codecs/input; zero externals). Verified before committing: check clean, 55/55 tests.
- 8c98a39: that recovery committed as-is (state was sound — no repairs needed).
- 1761d34: residual fmt drift in `build_plan`'s WavCopyPcm match arm.
- Gates ACTUAL (all rerun after the last code change): `cargo check -p transcode` clean; `cargo test -p transcode` 55/55 (43 unit + 6 lifecycle e2e + 5 round-trip + 1 doctest); `cargo fmt -p transcode --check` clean; `cargo clippy -p transcode --all-targets -- -D warnings` zero; `check-architecture` PASS 5/5 (31 members, transcode registered); `lint-literals` PASS 0 violations/340 allowlisted; `check-identity` PASS 0 violations/341 allowlisted (no transcode annotations needed — WAV magic bytes are format vocabulary).
- Requirement audit vs the card: (a) typed graph source→decode→[filters]→sink with `validate()` order pinned by tests; (b) queued→running→progress→terminal with first-set-wins OnceLock, bounded drop-and-count event hub, cooperative cancel checked at every stage boundary, saturating byte/sample accounting; (c) two-axis stage queue (64 slots/1 MiB default, 1024/16 MiB ceilings) with parking backpressure + queue high-water stats; (d) RIFF/WAVE writer for u8/s16/s24/s32/f32/f64, identity + lossless widening only, 4-fixture bit-exact round-trip through our own WavDemuxer/PcmDecoder; (e) copy paths WAV-from-PCM (LE unpacked only, BE typed-refused) + raw ES dump, no MP4-remux claim (7.E2); (f) lifecycle/cancel/progress + fill-queue→park proofs + fixture round-trips — no new media binaries committed (fixtures pre-existing; e2e PCM synthesized programmatically).

Stage Summary:
- Card 7.E1 landed green on w21/c at 1761d34: 11 src files + 2 e2e suites (5834 lines), 55/55 tests, all five local gates PASS. Honest capability surface: WAV PCM encode + the two stream-copy paths only; filters modeled-and-refused, codec encoders deferred to 7.E3 via the recorded `[PROPOSAL]` ES-sink seam, MP4 remux to 7.E2. No new deps, no unsafe, no workspace-wide test run (disk-tight).
Task ID: 7.w22d
Agent: w22-d (plugin host final — issue #45 cards 7.F1–7.F3)
Task: Repair and complete the uncommitted plugin-host-wasm state; land F1–F3 green through the full gate ladder.

Work Log:
- Inspected the prior agents' uncommitted diff (api/error/fsjail/http/imports/state + tests/, boundaries.toml, Cargo.toml, Cargo.lock): compiling but 3 of 6 §9.4 sandbox tests failing; boundaries entry accurate (L6, internal [plugin-api], external [wasmi]).
- bcf4469: committed the compiling foundation (prior agent had dropped the `wat` dev-dep for a std-only in-test wasm binary emitter — tests/common — keeping the production closure at the audited 8-crate graph).
- Root-caused the 3 failures: (1) plugin_trap_loop SIGABRT stack overflow — wasmi 2.0's default-features=false pin compiles the tail-call dispatch backend unconditionally, and LLVM performs no sibling-call optimization at opt-level 0, so any debug/test build overflows the native stack after a few thousand guest instructions; (2) plugin_memory_exhaustion got the raw `GrowthOperationLimited` trap instead of the typed MemoryLimit (wasmi reports limiter-denied grow as a trap code under trap_on_grow_failure, not as MemoryError::ResourceLimiterDeniedAllocation); (3) plugin_host_pointer_smuggling fixture called media_jail twice on one TempDir — the unix symlink fixture is create-once, second call EEXIST.
- 1ce1ec9: three repairs — (a) wasmi features += `auto-dispatch` (zero-crate feature, wasmi's own default; build.rs falls back to the portable loop backend when tail calls are unavailable — Cargo.lock byte-identical, closure unchanged); (b) is_memory_denied also matches TrapCode::GrowthOperationLimited (both shapes are the ceiling holding); (c) fresh TempDir for the handle-cap fixture (assertions unchanged — fixture repair, not a weakening).
- 5b5a716: clippy --all-targets -D warnings clean (one useless_format in the lifecycle test) + rustfmt.
- Gates ACTUAL at HEAD: cargo test -p plugin-host-wasm = 33 unit + 2 hello-world + 4 install-lifecycle + 6 sandbox-security = 45 passed / 0 failed; check-architecture PASS 5/5 (69 internal edges); lint-literals PASS (0 violations, 376 allowlisted); check-identity PASS (0 violations, 377 allowlisted).

Stage Summary:
- Doc 06 §9.4 escalation tests all DENIED at the real wasm boundary: plugin_sandbox_escape (OOB ptrs → typed E_OOB, symlink/traversal → E_PERM, events logged), plugin_permission_escalation (undeclared import refuses instantiation; WASI module unknown; allowlist violation E_PERM ×3 → threshold-3 disable), plugin_trap_loop (fuel kill, CpuLimit, host survivor spawns), plugin_memory_exhaustion (MemoryLimit at the 1 MiB store cap, module max above it), plugin_io_abuse (E_QUOTA burst, E_TOO_LARGE transfer, E_QUOTA http window, E_TRANSPORT honest seam), plugin_host_pointer_smuggling (fabricated/double-close handles → E_HANDLE, handle-table cap E_LIMIT), plugin_malicious_manifest (11 refusal cases: ELF magic fence, origin forgery, hash mismatch/placeholder, api-window, refused kind, CRC, deflate, missing entries, non-ZIP, 64 MiB cap, signature bookkeeping ×4), plugin_no_auto_exec (inspect/install/load execute nothing; first execution — and its trap — lands at spawn).
- Decisions: import registry = typed V1_IMPORTS catalog (6 modules, permission-gated, additive-only since-fields, deprecation-window slots exercised synthetically in v1.0); ceilings per doc 06 §5.4 rows (32/64 MiB memory classes, fuel for the deadlines, token-bucket I/O, 2/min http test window); .lumenext = stored-ZIP manifest.toml + plugin.wasm + detached versioned signature-suite block — statuses Unsigned/Unverified only, never "verified" (no crypto dep; explicit user-consent match required, doc 06 §10 Q5 stays open); crash backoff/restart stays card #46 — only natural trap containment + escalation disable.
- D-10 four questions: MEASURE — 45/45 tests, 0 gate violations, Cargo.lock unchanged by the feature fix. NOT TESTED — no real transport behind the seam (E_TRANSPORT is the honest v1 answer; wiring `streaming`'s ureq client needs a Lead-approved doc 01 §2.8 edge); signature verification deferred to the crypto ADR; wall-clock fuel calibration is a [PROPOSAL] constant. WHAT BREAKS — a plugin needing >64 MiB, >16 MiB single transfers, or non-HTTPS hosts is refused by design; genuine signature enforcement does not exist yet. WHO HITS IT FIRST — the first real extension consumer (metadata-provider fetching cover art hits the unwired transport immediately).
Task ID: 7.w22e
Agent: w22-e
Task: apps/web v1 — engine-in-Worker (issue #47, card 7.H1): finish, repair and prove the wasm host scaffold.

Work Log:
- Inherited prior agents' state: scaffold checkpointed at 9b7df6a (1459 lines) + further uncommitted edits (abi.rs, host.rs, open.rs, protocol.rs, worker.js, README, Cargo.toml, boundaries.toml, Cargo.lock). Reviewed every file against doc 09 (§2 build, §3 pump/slice budgets, §10 blocked-capability policy) and phase7-cards item 8; the design was sound, the execution had drifted — repaired, never redesigned.
- Compile fixes (test cfg): EventKind::StateChanged → SessionStateChanged (catalog's real variant); u64/i64 audio-bound mismatch; BlobRegistry::load takes &str.
- Product fix — ABI staging invariant: output staging was destructive per read (take_outbound drains), so a re-read between dispatches silently answered `[]`, violating the module's own documented "valid until the next dispatch" contract. Staging is now idempotent per dispatch (staged flag, invalidated by dispatch); pinned by the re-read assertion in output_addr_and_len_describe_the_staged_bytes.
- Test-harness honesty fixes: open_fixture now sends Boot before LoadBlob (the not-booted lane refuses everything else — the same order the port contract pins); the word-lane test boots first via the word lane; the addr compare uses the ABI's u32 width (exact on wasm32, truncated on native); the audio-fill bound multiplies before dividing (exact sample-count µs, no per-quantum pre-truncation); blob_uri_parses pins that space-bearing blob names are the typed L0 refusal (uri.control_chars).
- wasm32 build: cfg-gated DrainGatedDecoder + its codec_api imports (native-only, sole constructor is native_decoder_for) — wasm32 build now warning-free; clippy pedantic set fixed (mem::take over drain(..).collect ×2, contains() over manual any ×2, byte-str word, is_multiple_of).
- worker.js: pre-instantiation messages are now QUEUED and drained in arrival order — a boot sent while the module fetch is in flight would hang the host page forever if dropped (node --check clean).
- Gates ACTUAL: cargo test -p lumen-web 35/35 passed (abi 5, capabilities 5, host 10, open 5, protocol 5, source 5 — the SAME message paths the JS glue drives, through the ABI dispatch core with the in-memory MessagePort double); cargo build -p lumen-web --target wasm32-unknown-unknown --locked: real artifact, 5,840,517 B (debug), sha256 cf6f533d57e954ac…, exports parsed and verified = memory + lumen_web_write_u32/dispatch/output_addr/output_len (5 exports, no others); cargo fmt clean; clippy -p lumen-web --all-targets -D warnings clean AND --target wasm32-unknown-unknown clean; xtask check-architecture PASS 5/5 (79 internal edges incl. the new L6 lumen-web row); lint-literals PASS (0 violations, 342 allowlisted); check-identity PASS (0 violations, 343 allowlisted).
- Commits: 7af21bf (crate complete + tests green), 27123e9 (wasm32 dead-code cfg), 9f5ad9f (fmt+clippy), 2043250 (worker.js queue).

Stage Summary:
- ABI choice: hand-rolled extern "C" + shared linear memory, ZERO new deps — doc 09 §2.2's raw-import fallback lane (wasm-bindgen is the doc's [PROPOSAL], OQ-1 undecided; nothing in §3's single-threaded v1 requires it). Input: JSON pushed as u32 words (space-padded tail) — the no-unsafe price; output: one staged JSON array read straight from memory.buffer. Reversal seam recorded in abi.rs docs.
- Slice budget: COUNT form, not deadline — std::time::Instant panics on wasm32-unknown-unknown; SliceBudgets = 16 audio quanta/tick, 4 engine polls/tick (engine TickBudget: demux ≤64 events, decode ≤16 packets/limb), 64 events/flush, 32 ticks/pump cap, 8 settle polls. Deadline form slots in behind the same pump entry when performance.now() arrives (H-pillars). Honest deviation documented in host.rs.
- Message contract: typed HostToWorker/WorkerToHost (camelCase, "type"-tagged); the 64-command/23-event catalog rides inside as crates/commands' own WireRequest/WireResponse/WireEvent — never re-defined. Control lane has its own typed ControlError.
- Capability report: boot answers EngineInfo + HostCapabilities (threading: single — the truth rule; 9 doc 09 §10 rows as typed BlockedCapability data; wasm32 adds the builtin-software-decoders blocked row: the codecs crate's external engines do not build for wasm32 — rav1d; native links real MJPEG+PCM so the headless proof decodes for real).
- Lane proof (headless-native, merge-blocking grade): boot → loadBlob(first-vertical AVI) → session.open → pump slices → Playing → end-of-media → position ≈ duration (0.95..=1.0) → graceful shutdown. Browser-runtime behavior (worker instantiation, postMessage transport, setTimeout re-arming) is [VERIFY] — no browser in this sandbox, nothing claims it.
- Limitations/open: wasm module ships decoder-less (Lead [PROPOSAL]: split in-house mjpeg/pcm out of crates/codecs); seek + config + unserved commands answer typed refusals naming owners (seek needs the H-pillar ring-discard duties; config is the web settings card); v1 presents headless (7.H2) with null audio (7.H3); http(s) sources typed-refused pending 7.H4.
Task ID: 7.w22f
Agent: w22-f
Task: Issue #50 / card 7.J1 — finalize crates/control-protocol (in-tree RFC 6455 WS server) + build apps/control-server (the loopback control-surface host); diagnose and fix the conformance-suite hang.

Work Log:
- Inspected the inherited state: prior agent's crate (checkpointed bc172d3) + 594 lines of uncommitted edits; apps/control-server empty; boundaries.toml missing both my entries. Fixed 2 compile errors (if-let-Some shape check in server.rs, unused binding in http.rs) — the uncommitted edits carried the prior agent's hang fix in progress.
- Hang root cause (found documented in the uncommitted frame.rs edit, verified by me): the CLIENT-side frame reader consumed 4 mask-key bytes unconditionally, but server->client frames are UNMASKED — the reader ate 4 payload bytes of every welcome and then blocked forever waiting for bytes that had already arrived. Fix already present (conditional mask read) + I verified: full suite green parallel AND --test-threads=1 (0.06s/0.17s).
- Bounded-waits hardening (defense in depth, new work): ProtocolError::TimedOut { what } + CloseCode::GoingAway (1001) with Network/ErrorCode mapping; read/write paths in frame.rs/http.rs map WouldBlock/TimedOut to it; ServerConfig.timeouts (Timeouts, documented 60s read / 10s write) applied to the accepted socket in serve_one BEFORE session work; TestClient runs under a 10s per-op deadline both directions; conformance test idle_session_is_closed_by_the_read_deadline_1001.
- Clippy -D warnings pass on the crate (as_chunks, derive Default, collapsible if, unnecessary to_owned).
- Built apps/control-server: config.rs (control-server.toml -> PolicyFile; missing file = strict defaults; product.toml gate via product-config — features.control-server must be true, doc 07 rule F2; engineInfo from product identity + live catalog checksum), dispatch.rs (WS<->command-layer bridge: system.get-engine-info + system.list-commands served from the live spec table, all 64 rows; everything else typed not-ready — no engine embedded, honest gap), logging.rs (structured JSON-lines LogSink: JsonStderr + MemoryLog; token NEVER logged, V8), run.rs (accept/serve loop, session cap), main.rs (lumen-control-server binary: --config/--product/--schema/--sessions; exit codes 0/2/3/4; generated token printed to stderr only — the single sanctioned reader; ws:// address on stdout).
- tests/e2e.rs: real-files + real-socket conformance through the app stack — temp copy of the REAL product.toml with the flag flipped exactly once, dirty TOML cases, typoed sensitive_allow grant = typed startup error, bind 0.0.0.0 refused at bind, full session (welcome / list-commands=64 / granted sensitive passes policy then honestly not-ready / ungranted denied + connection survives / engine-info / clean close) + structured-log assertions incl. no-token-in-log.
- Registered boundaries: [crates.control-protocol] (L1, commands via new same-layer pair control-protocol->commands, externals serde+serde_json) + [crates.control-server] (L6, commands+control-protocol+product-config, externals serde_json+toml); dropped the unused testkit dev-dep.
- Binary smoke: --help exit 0; repo product.toml (flag false) exits 2 with the honest rule-F2 message; flipped product --sessions 0 binds 127.0.0.1, prints ws:// address (stdout) + generated token (stderr), exit 0.

Stage Summary:
- ACTUAL results: cargo test -p control-protocol -p control-server = 87 tests green (46+24 crate, 11+6 app), parallel and single-threaded, no timeouts; cargo check/fmt --check clean; clippy --all-targets -D warnings clean; xtask check-architecture PASS (5/5, 32 members), lint-literals PASS (3 new annotated sites), check-identity PASS.
- Commits on w21/f: 2571398 (compile fix + prior agent's hang fix landed), eadc6af (bounded waits), 30ca309 (apps/control-server), d46ec25 (boundaries registration), plus this worklog commit. Never pushed; main untouched.
- Decisions: WS scope = minimal RFC 6455 server + test client, std-only transport (SHA-1/base64 in-tree, serde only for the command-layer JSON that already owns that format); token = 256-bit per-start (fixed option for tests), constant-time compare, 5-failure/60s lockout, wrong token = 4401 disconnect; config keys = [server] bind/port/read_only, [auth] token, [policy] sensitive_allow (deny-by-default, 10 sensitive ids); product gate = features.control-server (repo flag honestly false — Lead flips it with the feature-matrix row to ratify; app refuses to start otherwise).
- Limitations (honest): deep security review deferred to 8.A2 (rate limits 100rps/8-in-flight and Origin allowlist recorded gaps, not implemented); single-connection serial server (bounded by design, now with idle deadlines); no signal handling (std-only — SIGTERM ends process); engine-embedding join (playback behind dispatch) is the desktop composition card's — v1 serves catalog introspection, all else typed not-ready.
Task ID: 7.w22g
Agent: w22-g (release pipeline, issue #56 release half)
Task: Tag-triggered cross-platform release workflow + packaging scripts + install/release docs (zip/msi/dmg/deb/AppImage, SHA256SUMS, gh release).

Work Log:
- Pin resolutions (network, recorded): actions/download-artifact v8.0.2 @ 9000827c (git ls-remote, 2026-11-10; other four actions mirror ci.yml's table verbatim); WiX v5.0.2 nupkg at api.nuget.org flat-container (sha256 f30ef0c7..., contains tools/net6.0/any/wix.exe, runtimeconfig rollForward=Major); appimagetool 1.9.1 x86_64/aarch64 release assets (sha256 ed4ce84f.../f0837e74...; binary self-reports git 8c8c91f = the 1.9.1 tag commit — pin cross-checked); type2-runtime dated tag 20251108 x86_64/aarch64 (2fca8b44.../00cbdfcf...).
- eff49d1: scripts/release/ — product-env.py (product.toml + locale catalog + cargo metadata -> GITHUB_ENV; hard-fail posture), gen-icon.py (deterministic 512x512 accent-color PNG, stdlib zlib/struct), gen-wxs.sh, package-windows.sh, make-app.sh, make-dmg.sh, package-deb.sh, make-appimage.sh, release-notes.sh (all POSIX sh, env-required not defaulted).
- 3b1612f: .github/workflows/release.yml — push tags v* + dispatch; contents:write; jobs windows (zip+MSI), macos-arm64 (macos-latest), macos-x64 (macos-15-intel — actionlint 1.7.12 label DB rejects macos-13; documented deviation), linux-x64 (tar+deb+AppImage), linux-arm64 (ubuntu-24.04-arm, continue-on-error with honest comment), release (download-artifact merge, SHA256SUMS.txt, release-notes.sh, gh release create with GITHUB_TOKEN, tag-ref-gated; dispatch = dry-run). Zero product/version literals in YAML (audit: rg -i lumen|0.1.0 -> only sanctioned crate ids + comments).
- 1b35f6e: dry-run findings fixed — appimagetool given explicit ARCH (dummy-binary dry-run exposed header-guess failure) and pinned --runtime-file (first dry-run silently fetched the ROLLING continuous type2 runtime — now pinned 20251108; rebuilt AppImage BuildID a87aaf5d matches the pinned runtime file exactly); deb mode normalization (0644 docs/entries).
- 68c2ea4: README "Install" section (checksum verification, per-OS install incl. unsigned-binary Gatekeeper/SmartScreen workarounds, build-from-source pointer) + docs/RELEASE.md (cut-a-release process, RC checklist, 7-item honest known-limits register).
- ACTUAL validation: actionlint 1.7.12 clean on ALL 5 workflows; bash -n + sh -n all 7 scripts; python YAML parse all workflows; REAL .deb built via package-deb.sh on this host (dpkg-deb --info/--contents + -x roundtrip verified: control fields config-driven, usr/bin + applications + hicolor icon layout, root ownership); REAL AppImage built via make-appimage.sh with the pinned tools AND EXECUTED (APPIMAGE_EXTRACT_AND_RUN -> bundled dummy binary ran, exit 0); make-app.sh bundle layout + Info.plist field-assert on Linux; gen-wxs.sh output XML-parsed (Name/Manufacturer/Version/UpgradeCode asserted); package-windows.sh zip mode verified (4 flat entries); release-notes.sh both modes verified.

Stage Summary:
- Workflow never executed (CI billing blocked) — by-construction correctness only, stated in workflow header + docs/RELEASE.md; runtime claims marked [VERIFY]. Start with a workflow_dispatch dry-run when billing unblocks.
- D-10: MEASURE — actionlint/YAML/syntax all green; deb 4924 bytes validated; AppImage 948728 bytes executable. NOT TESTED — Windows MSI (wix.exe) and macOS dmg (hdiutil)/ditto paths (OS-bound, [VERIFY]); no real cargo release build run (disk-tight; dummy binaries used for packaging logic). WHAT BREAKS — first live run may hit python3/python PATH differences on images or the macos-15-intel label; both have documented fallbacks/checks. WHO HITS IT FIRST — the Lead cutting the first tag.
- Open: signing/notarization certs (issue #56 external); brand icons (placeholder accent-color PNG ships); AppStream metadata absent (appimagetool warning, documented).
Task ID: 7.w22h
Agent: w22-h (issues #55 + #57 — Phase 8 perf/parity docs finalization)
Task: Final perf-vs-VLC benchmark report (#55) + parity-report finalization with honest parity statement and consolidated known-limits doc (#57).

Work Log:
- Read the evidence base in full: docs/corpus/vlc-diff-report-20261023.md (oracle run 2026-11-09 + §6.6 decision record), vlc-oracle-run-20261109.json, vlc-f32-tolerance-20261109.json, docs/perf/{breadth,baseline,mjpeg-simd-20261024,memory-audit,soak-nightly-20261109,parity-v4-20261024}.md + raw/, docs/plans/phase8-cards.md, docs/10 §6.6–§8, docs/12 §1.3/§9, docs/09 §10, ADR-001/002/003/004/005/010/011/012/013, docs/14-risk-register.md, tools/xtask parity/audit sources, worklog prior entries.
- ACTUAL tool runs (deterministic, non-timing): cargo xtask parity-report -> rows=252, windows/linux/macos 64/252 (25.4%) each, web 0/252, verified-only 0/252 everywhere; cargo xtask feature-audit -> initially FAILURE (2 R4 violations: evidence tokens tests/https_activation.rs + tests/pcm_pipeline.rs crate-relative, root-unresolvable though both files exist at crates/streaming/tests/ and crates/audio-output/tests/).
- c9a89c6: fixed the 2 evidence-path spellings in docs/feature-matrix.toml (evidence edits only, no status cell changed); feature-audit re-run ACTUAL: PASS (252 rows, all evidence present + path-resolved).
- 5aa3e7d: docs/PERF-VS-VLC.md (#55 final report: differential methodology incl. mock/real binding + corpus re-basing honesty, 29/35 bit-exact prefix rows, the 6 f32 rows with the ratified RMS<=1e-4/peak<=1e-3 policy and per-row measurements, honest "no wall-clock Lumen-vs-VLC timing exists" statement, prior-wave ffmpeg-oracle timing evidence quoted with sandbox caveats, §9.2 verdict table, limitations); docs/PARITY-REPORT.md (#57: tooling lockdown state, ACTUAL 252-row audit output, ACTUAL derived parity table, the honest "verified parity 0/252 pending CI" statement and what it means, VLC-differential cross-ref, unblock path); docs/KNOWN-LIMITS.md (#57: consolidated register harvested from MP §14, ADRs, doc 09 §10, risk register R-01..R-36, perf records — every row source-cited; RELEASE.md release-specific limits linked not duplicated).
- No timing benchmark was run for these docs (contended shared sandbox; doc 12 §9.3 anti-gaming) — all timing rows are quoted from prior-wave committed records.

Stage Summary:
- Deliverables: docs/PERF-VS-VLC.md, docs/PARITY-REPORT.md, docs/KNOWN-LIMITS.md + 2 evidence-path fixes in docs/feature-matrix.toml; commits c9a89c6, 5aa3e7d, this entry.
- Headline honest numbers: differential 29/35 bit-exact + 0/6 f32 tolerance-pass = 29/35 (Lead decision applied, per-row JSON record); implementation parity 64/252 per desktop platform, web 0/252; verified parity 0/252 pending CI (billing-blocked, all evidence hardware_class=mock).
- Cross-wave coordination: docs/RELEASE.md (known-limits register) is referenced as owned by 7.w22g branch w22/g — the link resolves when that wave merges.
- Observed for the Lead (not touched, out of scope): docs/feature-matrix.toml header comment still says the full tools/feature-audit is "OWED and unassigned debt" — stale since ed24e7d (wave 11); header is Lead-owned per the schema-freeze note.

---
Task ID: 7.w23a
Agent: w23-a (Lead-finalized)
Task: Hardware decode adapters — VideoToolbox / Media Foundation / VA-API (issue #41, cards 7.B1–7.B3)

Work Log:
- Landed crates/codecs/src/video/h264/hw/: capability.rs (negotiation + truthful hw-active reporting), frame_pool.rs, vaapi/ (device-node discovery, permission-failure handling, session), mediafoundation/ (MFT/DXVA declarations), videotoolbox/ (VTDecompressionSession declarations), feed.rs (frame feed path), settings-driven wiring (installed_bridges + negotiate_lane + H264Decoder::from_setting), default OFF.
- Host tests: capability negotiation, no-device (DRI-less root), permission-denied, fallback-to-software — 684 codec tests green.
- Cross-target compile verification: aarch64-apple-darwin (VT), x86_64-pc-windows-msvc (MF) — typecheck of hand-rolled FFI only; runtime hardware behavior NOT exercised in this environment [VERIFY].
- lint-literals/check-identity PASS after marking 3 cfg(test) scratch-dir prefixes.
- Feature-matrix codec-video-h264 note updated with landed-lane evidence.

Stage Summary:
- All three lanes implemented behind typed seams with honest capability negotiation; compile-verified cross-OS, host-tested fallback paths. ADR-013 remains [proposed] (VA-API activation Lead-gated). Real-hardware validation pending [VERIFY].
Task ID: 7.w23b
Agent: w23-b (Lead-finalized)
Task: Device hotplug, output fallback & discovery (issue #42, cards 7.C1 + 7.D1)

Work Log:
- C1: playback-engine device-loss hooks (suspend in-pipeline, bounded reopen window, resume from remembered position); audio-output renegotiation; renderer-wgpu backend fallback chain (hw->sw auto-negotiate, typed capability reports, mid-run degradation); catalog events video.backend-changed + devices.volume-added/-removed (64 cmd / 26 events).
- D1: crates/device-discovery — ServiceDiscovery port, bounded volume watcher, SSDP lane with honest verdict (UDP multicast: feasible natively, blocked on web); typed Unavailable { reason } where OS features absent.
- UI: /discovery panel (React/TS, semantic tokens, l10n catalog), live devices.* event lane, honest verdicts surfaced; 8 new pinned visual baselines + manifest route.
- Verification: fmt/clippy -D warnings clean on 4 crates; 490 engine tests + 697 UI tests (78 files) green; codegen --check no drift (64/26); lint-literals PASS; check-architecture PASS.

Stage Summary:
- Hotplug + fallback chains + discovery landed end-to-end with honest capability reporting; SSDP verdict recorded; real-hardware hotplug events [VERIFY] on OS runners.
---
Task ID: 7.w23c
Agent: w23-c
Task: Issue #44 / cards 7.E2–7.E3 — finish recording + MP4 remux engine (wave-23 r2/r3) and ship the conversion UI subset.

Work Log:
- Inherited b3f8445 (record-to-WAV one-call assembly) + 7 dirty files carrying an in-progress in-tree MP4 muxer. The lib built but the mp4mux test module had 5 compile errors (missing Packet import, `[u8]` vs `[u8;4]` mismatches, Box-field access past finalize) and 2 wrong assertions (mdat size read at 36 instead of 32; a ceiling test that expected a 3-sample refusal against the 1 Mi constant). Fixed all: SharedCursor-with-twin pattern so output bytes outlive finalize; ceiling test now pre-fills the full 1 Mi-row table directly (the WAV `riff_ceiling` precedent — guard arithmetic driven, no bytes materialized).
- Clippy pass on the inherited code: stream_position() for the two Current(0) seeks, collapsed if-lets via .filter(), array for the two-config list in tests/mp4_remux.rs. Zero warnings after.
- E2 acceptance complete (verified, not re-implemented — the inherited design was sound): record suite = fixture → Recorder → re-demux (our WavDemuxer) → re-decode → bit-exact compare, factory-refusal typed, lossless-widening, no-audio typed, mid-record cancel leaves an honest playable prefix (sizes patched to the truth); mp4_remux suite = progressive/fragmented/elst fixtures → Remuxer → re-demux via our Mp4Demuxer → per-track packet-identical compare (payloads, DTS/PTS, keyframes, durations, counts) + codec-outside-set / missing-track / empty-config typed refusals + mid-remux cancel honest prefix. Muxer is std-only, no unsafe, no new deps; 32-bit mdat/stco ceilings typed (mp4_size_overflow), sample ceiling 1 Mi, sample-entry ceiling 64 KiB; MP4-to-pipe honestly NOT offered (mdat patch needs Seek).
- E3 UI (`ui/src/screens/convert`, new): convertTargets.ts — closed two-target table mirroring crates/transcode's real operations (WAV record / MP4-remux) with honest per-target notes; [PROPOSAL] comment records the unsurfaced RawEsCopy ES-sink third target (card 7.E3 item 2 — engine side exists, surface awaits Lead ratification).
- convertQueue.ts — pure lifecycle reducer (queued/running/completed/failed/cancelled/blocked) mirroring the engine's JobOutcome; bounded at 8 rows (typed bound error surfaced by the screen); cancel of a running job keeps its last progress (the honest-partial shape); total reducer — refused transitions return the SAME array reference; laneCommandsOf probes a `system.list-commands` result for convert./record. namespace commands (the customize-panels capability-gate precedent).
- ConvertScreen.tsx + convert.css (Solstice v2 shapes, semantic tokens only, calm backdrop): source field, RadioGroup target picker, action row, queue panel with per-row status badge, progress element (running), partial percent (cancelled), cancel affordance (queued/running only), terminal detail line, and the honest unavailable banner. CAPABILITY HONESTY: the command catalog carries NO convert/record commands today (verified against crates/commands — Lead-owned interface; product.toml keeps recording/transcode false per rule F2), so an enqueued job transitions queued → blocked with the typed not-exposed reason and the only wire traffic is the catalog probe — never a simulated progress bar, never a fabricated command id. The engine-binding seam is one branch in startJob: when the Lead lands catalog commands, dispatch starts there and progress/complete/fail/cancel events drive the same reducer.
- Wiring: route /convert in App.tsx; File-menu item in AppShell.tsx (menu.file.convert; NO accelerator — the hotkey data carries media.convert Ctrl+R but the player keymap's SHELL_UI_ACTIONS doesn't route it; an unbound accelerator would be a false promise — Lead owns that row). 26 l10n keys added to ui/src/i18n/messages.ts.
- Observed, NOT touched (out of scope): `media.convert` hotkey row exists in generated bindings with uiAction "convert" but no SHELL_UI_ACTIONS entry; pre-existing stylelint errors in components/icon/icon.css:43 and screens/player/dialogs/dialogs.css:16; screen.* keys are defined but unconsumed.

Stage Summary:
- ACTUAL results: cargo fmt -p transcode --check clean; cargo clippy -p transcode --all-targets = 0 warnings; cargo test -p transcode = 75 passed / 0 failed (50 lib incl. 4 mp4mux unit, 6 job_lifecycle, 7 mp4_remux, 5 record, 5 wav_roundtrip, 2 doc). xtask check-architecture PASS (5/5), lint-literals PASS (0 violations). UI: npm run typecheck clean; npm run lint exit 0 (27 pre-existing react-refresh warnings, none in convert files); npm run l10n:check PASS (596 keys, missing 0, orphans 0); npm run test = 79 files / 710 tests passed (incl. 21 new: 13 convertQueue + 8 ConvertScreen). convert.css passes stylelint individually.
- Commits on w23/c: f037ba6 (E2 engine: mp4mux test fixes + clippy), 62dfade (E3 UI convert screen), plus this worklog commit. Never pushed; main untouched.
- D-10: MEASURE — all counts above are executed-command outputs, not estimates. NOT TESTED — no real engine process behind the UI (no transport exposes a conversion lane; that wiring is the Lead's catalog extension); browser/E2E visual baselines for /convert not seeded (visual matrix is Lead-maintained); MP4 output not byte-compared against ffmpeg's muxer (round-trip is through OUR demuxer — the issue's AC, deliberately). WHAT BREAKS — first user of /convert on a desktop build sees blocked rows + the unavailable banner (by design, honest); if the Lead adds convert.* catalog commands with different argument shapes than the seam assumes, startJob's dispatch branch needs the real request schema. WHO HITS IT FIRST — the Lead flipping product.toml recording/transcode + landing catalog commands; until then every surface states the truth.
- Limitations/open: engine lane not wired into crates/commands (Lead-owned shared interface — ask-don't-invent); no live-capture tap into a playing session (record.rs docs record the gap honestly); ES-sink UI target [PROPOSAL]; AppShell icon-rail entry not added (menu item only — rail is shared chrome).
---
Task ID: 7.w23d
Agent: w23-d
Task: W23-D r3 — issue #46 close-out: verify the F4 crash-isolation suite (cards 7.F4) and complete G1 example extensions (metadata-provider + ui-panel) with conformance kit and walkthrough.

Work Log:
- Inherited 3 committed waves (c71cd5a backoff/Disabled supervisor; b5e2560 crash-isolation suite + playback-unaffected proof; 0e43bc2 host→guest writes + manifest stamping) plus 2 dirty files (Cargo.toml bin entry + untracked lumenext-examples.rs generator). Reviewed both; design was sound, kept and finished them.
- F4 verification: first `cargo test -p plugin-host-wasm` failed to COMPILE — E0004 non-exhaustive EventKind match in playback-engine. Root-caused (fingerprint dirs + dep-info + rmeta strings): the shared CARGO_TARGET_DIR lets sibling worktree w23b's `commands` build (which adds VideoBackendChanged/DevicesVolume* variants) clobber the identically-named `libcommands-*.rmeta` my units link — same package name+version+dep-metadata, so same artifact filename across worktrees; last writer wins. Not a source bug; fixed by rebuilding w23d's commands (touch + check). Recurred twice more during the session; each time the same one-command repair. Flagged for the orchestrator/Lead.
- After the repair: full crate suite 65/65 green with zero source changes — F4 (trap→Disabled backoff, settings-driven; playback-unaffected proof; OOM/fuel containment; crash-injection fixtures) verified as landed.
- G1: fixed the generator's two real bugs — (1) both `init` bodies used local index 2 with only one declared local on a 0-param function (invalid wasm; caught by wasmi's validating Module::new through the kit, exactly what the kit is for); (2) first-run write when the committed blob is absent (was an Io failure) + parent-dir creation. Removed the dead CMD_AT constant; clippy type_complexity fix (ModuleBuilder type alias).
- G1: plugins/extensions/{metadata-provider,ui-panel}/manifest.toml (doc 06 §2.3 placeholder provenance hash; kind-correct capabilities provides=["tags"] / surfaces=["side-panel"]; permissions fs-sandboxed-read / ui-surface), committed plugin.wasm blobs (764 B each, deterministic), .gitignore for stray .lumenext artifacts.
- G1: tests/plugin_examples_conformance.rs — the §7 kit: drift gate (runs the real lumenext-examples --check binary), install-with-consent → load → spawn under production ResourceLimits::for_kind, the command-layer extract round trip through write_guest_memory/read_guest_memory (provides=["tags"] demonstrated with the sidecar's verbatim bytes), panel registration + on_command acknowledgment, typed E_PERM error paths (missing + traversal) with bounded recovery and ledger accounting, api-window refusal (§7.6 slice), and the real lumenext CLI package/inspect round trip. 5/5 green.
- lint-literals hygiene: import module names via MOD_* constants carrying the same lumen-allow-literal markers as src/api.rs; example provenance URL shortened to the tests/common fixture form with its own marker (RFC 2606 example.com placeholder). 0 violations, 408 allowlisted.
- plugins/extensions/README.md — the walkthrough: build (regenerate) → package → inspect → install+verify (the kit) → cleanup, every output block captured from a real run on this tree (764-byte modules, digests ed7ec5be…/acfe368e…); memory-layout table, both protocols, write-your-own guide, honest limitations (unsigned; no guest SDK [PROPOSAL]; no in-guest parser → no fuzz target owed per doc 06 §7.7; sidecar-text not ID3/Vorbis).
- Gates ACTUAL at HEAD (3de83e5): cargo fmt -p plugin-host-wasm -- --check clean; cargo clippy -p plugin-host-wasm --all-targets -- -D warnings clean; cargo test -p plugin-host-wasm = 65 passed / 0 failed (42 unit + 6 crash-isolation + 5 examples-conformance + 2 hello-world + 4 install-lifecycle + 6 sandbox-security); xtask check-architecture PASS 5/5; xtask lint-literals PASS (0 violations, 408 allowlisted). No new dependencies (wasmi closure unchanged), no unsafe, no workspace-wide test run.
- Commits: 3a77606 (examples + conformance kit), 3de83e5 (walkthrough + lint hygiene), this worklog entry. Never pushed; main untouched.

Stage Summary:
- F4 verified green as inherited (zero source changes needed; the only red was a shared-target-dir cross-worktree artifact clobber, not code). G1 complete: both example extensions build deterministically, package into valid .lumenext archives through the real CLI, install with consent, load, spawn under production limits, and pass the conformance kit — capability-honesty claims demonstrated, error paths typed.
- D-10 four questions: MEASURE — 65/65 tests, 5/5 gate ladder green, walkthrough outputs captured from real runs. NOT TESTED — panel UI composition beyond the host-side PanelRecord (app card); a user-facing install dialog flow (the kit performs install programmatically); signature verification (no crypto dep, doc 06 §10 Q5 open). WHAT BREAKS — any sibling-worktree cargo build of `commands` (e.g. w23b's catalog work) silently poisons this worktree's playback-engine compile until `touch crates/commands/src/lib.rs && cargo check -p commands` is re-run; the examples read sidecar text only (no embedded tag formats). WHO HITS IT FIRST — the next agent on any w23 worktree running cargo test while w23b is active (the E0004 signature is the tell); the first real extension author needing a Rust guest toolchain (recorded [PROPOSAL]).
- Open for the Lead: the shared-CARGO_TARGET_DIR same-name-package clobber is a standing hazard for all concurrent worktrees (w23b↔w23d observed; w23e produced a distinct hash so only same-dep-hash worktree pairs collide); an orchestrator-level fix (per-worktree metadata salt or serialized builds with source-path-keyed artifacts) is worth a decision.
Task ID: 7.w23e
Agent: w23-e (final pass, issue #48 — web H-pillars)
Task: Finish cards 7.H2 (render surface) + 7.H3 (AudioWorklet sink/clock/autoplay) in apps/web from a killed predecessor's dirty state; verify green; commit.

Work Log:
- Inherited 3055 dirty lines (5 modified + untracked render/ and audio/ trees). Audited each module; code was structurally sound (the "#ust_use]" seen in diffs is a display artifact — bytes are `#[must_use]`).
- Fixed the shared-target clobber: w23b's concurrent build overwrote libcommands in the shared CARGO_TARGET_DIR (their branch has VideoBackendChanged etc.), failing my playback-engine compile. `cargo clean -p commands` + back-to-back chained verification runs restored consistency (the documented hazard, hit for real).
- Fixed 2 failing tests (both pacing bugs in the predecessor's new tests, not engine bugs): the ~2 s fixture reaches Ended within 16x pump(4) — restructured `surface_loss_falls_then_degrades_to_audio_only` to trigger the loss BEFORE the first present and pump-to-first-present on the fallback lane; `the_autoplay_gate_parks_freezes_and_resumes` now parks from session start (position frozen at 0 exactly — the never-fake-playing proof) and asserts the engine still reaches Playing while parked. No test was weakened: both now assert MORE (frozen == 0, reached_playing).
- Fixed 1 clippy -D warnings error (`manual_is_multiple_of` in sink.rs).
- JS glue (hand-written, zero npm deps): NEW apps/web/audio-worklet.js — the audited transcription of the tested WorkletQueue model (bounded queue typed refusals, 128 quantum, silence-on-underrun, played-counter bump, cursor pops across chunk boundaries); extended apps/web/worker.js with the raw frame lane (reads lumen_web_frame_addr/len before any further wasm call, transfers the RGBA copy) + header docs for the page-driven boot-probe/surfaceLost/audioContext reports. node --check both; node harness (not committed — /home/z/audioworklet-harness) executed the REAL processor file against the Rust tests' own invariants: 25/25 checks PASS (10-s filled sim counter == 480000, underrun discipline, typed bound refusals, chunk-boundary continuity, flush-keeps-counter, report cadence).
- lint-literals was FAILING at the inherited H4 commit (33 violations, real exit 1) — annotated every site with `// lumen-allow-literal` (the sanctioned marker, reasons mirroring the apps/cli precedent: reserved example host test vectors + the wire-protocol scheme prefixes in validate_http_url). PASS now: 0 violations, 440 allowlisted.
- README.md updated (layout table incl. render/audio/io + audio-worklet.js, honest-limits rows for H2/H3/H4 with [VERIFY] boundaries, page-probes-then-boots serving note).
- wasm evidence: cargo build -p lumen-web --target wasm32-unknown-unknown --locked OK; parsed the export section of the real 5,989,631-byte lumen_web.wasm — all 6 function exports present (write_u32, dispatch, output_addr, output_len, NEW frame_addr, NEW frame_len) + memory (kind 2, index 0).

Stage Summary:
- ACTUAL: cargo test -p lumen-web = 111 passed / 0 failed (render 20, audio 15, io 35 kept green, host 14, abi 6, protocol 6, capabilities 5, open 5, source 5); fmt --check CLEAN; clippy --all-targets -D warnings CLEAN; wasm32 --locked build OK; check-architecture PASS 5/5 (38 members, 94 internal edges); lint-literals PASS 0 violations.
- Per-pillar status: H2 render — typed seam landed (negotiation ladder over simulated capability reports, canvas DPR arithmetic, RGBA upload with colorimetry, latest-wins mailbox, loss ladder with audio-only degrade); H3 audio — sink model + exact-integer clock (drift/wallclock/resample) + autoplay gate landed, JS processor transcribed and node-harness-verified; H4 io — kept green + lint-literals fixed. Browser halves (real navigator.gpu probe, canvas draw, AudioContext, fetch) all [VERIFY] card 7.I1 — v1 output lanes remain the headless presenter + null device, stated in the capability snapshot.
- D-10: MEASURE — 111/111 tests, 25/25 node harness checks, 6+1 wasm exports verified by section parse, fmt/clippy/architecture/lint-literals all green. NOT TESTED — anything in a real browser (no browser in this sandbox); the AudioWorklet real-time thread, autoplay policy per browser, outputLatency availability. WHAT BREAKS — the JS transcription could drift from the Rust model if either changes without the other (mitigated: mirrored constants + the header contract + harness recorded); report cadence (8 quanta) may need tuning on real devices. WHO HITS IT FIRST — card 7.I1's browser matrix.
- Commits: 5dbd7ec (H2+H3+JS glue+lint-literals fix, 4324 insertions) on w23/e; never pushed; main untouched.
---
Task ID: 7.w23f
Agent: w23-f
Task: Issue #52 final deliverable — std-only CycloneDX 1.5 SBOM generator (xtask subcommand) + committed artifact + CI/gate wiring, after sweeps 1-4 (a05516c parsers, 0d91b37 plugins, 784c6e6 control, 64d5753 supply-chain; all four reports verified present under docs/security/).

Work Log:
- tools/xtask/src/cmd/sbom.rs (new, std-only, zero deps — xtask C5 kept): parses Cargo.lock via the shared toml_lite reader (check_architecture's own reader rejects multi-line dep arrays, toml_lite does not — reused toml_lite, issue #20's shared module); product.toml [identity] (name/display_version/publisher) as the BOM subject; emits CycloneDX 1.5 JSON through a minimal deterministic writer (2-space pretty, fixed key order, RFC 8259 escaping).
- Component model: one per [[package]] row, sorted (name, version) for byte-stability across lockfile reorders; registry packages get purl pkg:cargo/<n>@<v> + crates.io distribution externalReference + SHA-256 hashes from the lockfile checksum; path packages get the opaque <n>@<v> bom-ref (no fabricated purl); every component carries the raw lockfile source in lumen:cargo-lock-source; unknown lockfile keys or unresolvable/ambiguous dependency descriptors are loud exit-2 tool errors (never silent graph holes).
- Determinism: serialNumber is a content-derived UUIDv8 (two-seed FNV-1a 128-bit fingerprint, version nibble 8 + variant 10xx per RFC 9562), no timestamp — same inputs → byte-identical SBOM, which the drift gate requires; licenses honestly omitted (Cargo.lock carries none; G3 cargo-deny owns them, recorded in metadata.properties).
- --check drift mode mirrors fixtures/schema-gen: byte-compare vs the committed file, exit 0/1/2, actionable remediation string. cargo-cyclonedx NOT used (clean-room AGENTS §3.2 + doc 11 §3.4 deviation documented in-module and in the README; [PROPOSAL] swap-in later).
- 10 unit tests: lock parse/classification, loud-failure shapes, identity read, dep resolution incl. "name version" suffixes + ambiguity, determinism + reorder-stability, CycloneDX field presence, purl-absence for path packages, UUID shape/sensitivity, JSON escaping, writer pretty-print/empty containers, non-crates-io source opacity.
- Registered in cmd/mod.rs under 7.w23f's granted tools/xtask/src/** ownership (check-panics/7-e precedent; frozen-registry files edited only to add the one entry + table row).
- Artifact: docs/security/sbom/cyclonedx.json (121166 bytes, 156 components = 118 registry + 38 workspace/path, 157 dependency entries) + docs/security/sbom/README.md (regen/drift contract, deviation, honest limits). Machine-checked with python json.load: valid JSON, 156 unique bom-refs, every dependsOn target resolves, subject dependsOn = the 38 path packages, serialNumber matches the urn:uuid pattern.
- Drift negative path proven by hand: tampered file → "result: DRIFT" + exit 1; regenerated → exit 0.
- CI: .github/workflows/ci.yml cargo-deny job only — "SBOM — generate + drift check" step AFTER the deny check (cheap-first, doc 11 §3.5 order), same cargo run -p xtask --locked -- <cmd> style as the other drift gates. No other workflow logic touched. python yaml.safe_load OK; actionlint not present in this sandbox [VERIFY].
- scripts/full_gate.sh: step 9 "sbom --check" following the if/tail/fail=1 style (bash -n OK; cargo xtask alias path exercised, exit 0).
- Verification battery (ACTUAL): cargo fmt -p xtask -- --check CLEAN; cargo clippy -p xtask --all-targets -- -D warnings CLEAN; cargo test -p xtask 206 passed / 0 failed (146 bin incl. 10 new sbom + 60 integration); cargo run -p xtask --locked -- sbom --check no drift (121166 bytes); cargo run -p xtask --locked -- check-architecture PASS 5/5. Also re-run green: lint-literals PASS, check-identity PASS (committed SBOM contains identity strings but both gates scan crates/apps only, as designed).
- Commits on w23/f: 3c04dee (generator + artifact + README), cb0da89 (CI + full_gate + fmt), this worklog entry. Never pushed; main untouched.

Stage Summary:
- Issue #52 SBOM deliverable closed: std-only CycloneDX 1.5 generator (no cargo-cyclonedx — deviation documented), committed docs/security/sbom/cyclonedx.json + README, drift-gated in CI (cargo-deny job) and scripts/full_gate.sh step 9. With sweeps 1-4 already committed, issue #52's deliverable set is complete on this branch.
- D-10: MEASURE — 121166-byte SBOM, 156 components (118 registry w/ SHA-256 + purl, 38 path), 157 dependency entries, all machine-verified resolvable; 206 xtask tests green; full 5-command battery green. NOT TESTED — official bom-1.5.schema.json validation with a JSON-Schema validator (structural self-checks only, README [VERIFY]); actionlint on the edited workflow (absent in sandbox); the CI job itself never executed (billing-blocked, repo-wide standing caveat). WHAT BREAKS — a Cargo.lock or product.toml identity change without regenerating makes the CI/full_gate step fail (intended: exit 1 + remediation); a future lockfile dialect shape (new [[package]] key, inline-table deps) turns into a loud exit 2 until the reader is extended. WHO HITS IT FIRST — the next agent bumping any dependency (drift gate message tells them to run cargo xtask sbom and commit).
- Limitations (honest): no licenses/authors/CPE in the SBOM (not in Cargo.lock; G3 owns policy); Rust graph only (wasm + npm UI SBOMs are separate future cards, doc 11 §3.4; per-platform finalization + cargo-auditable embedding = 8.E3); subject dependsOn = all 38 path packages (shipped-app vs dev-tool split not encoded in the lockfile, documented); serial fingerprint is non-cryptographic by design (uniqueness only).

---
Task ID: W24-B
Agent: W24-B (soak & stress lane — entry appended retroactively by the W24-M r2 review-fix round, see the provenance bullets)
Task: Issue #54, cards 8.C1 / P8-3 — soak & stress completion: the bounded five-scenario measurement suite, the 5 MB/h FREEZE decision on measured numbers, the nightly 4 h tier + LSAN wiring.

Work Log:
- 8bcaf62 (17:13 UTC): the scenario suite — config plane (tests/soak/*.toml manifests + scenarios.schema.json contract), five scenario runners, markdown report generator (crates/playback-engine/tests/soak_scenarios/).
- 8780e50 (17:44): scripts/soak-scenarios.sh sandbox wrapper + report-name fixups (the SOAK_SCENARIOS_ONLY chunking knob for wall-capped sandboxes; CI and the nightly never set it).
- 9055384 (18:30): raw sandbox evidence, scenarios 1–3 of 5 (play-loop, seek-storm, playlist-cycle — release runs, verbatim reports committed under docs/soak/raw/).
- 09ad85a (19:10): bounded instrument sampling — the count-scenario stride keeps each series ≤ 4,096 samples (the ~48 MB per-cycle-sampling regression found in-session, fixed, unit-pinned) + count recalibration + the 2M-cycle churn evidence.
- e272507 (19:14): raw sandbox evidence, scenario 5 of 5 — reopen-stress 150k cycles + its calibration report.
- 80fe6bc (19:24): the forced-threshold enforcement self-proof — pinched budgets in a 1M-cycle profile FAIL with 2 violations, exit ≠ 0 (a red that proves the guard bites).
- 0024041 (19:27): extend .github/workflows/soak-nightly.yml — the scenario-suite 4 h tier + the LSAN job; nightly cycle counts derived from the measured rates (both billing-blocked, stated in the report's §8).
- c7c5df7 + 0acd8c4 (19:49): clippy manual_is_multiple_of fix; the synthesis report docs/soak/soak-report-20261110.md (measured numbers, the 5 MB/h FREEZE, §6.2 regression-guard wiring gap, audits).
- Verification battery, ACTUAL results — re-run green by the independent W24-R4 audit in a fresh worktree on 2026-11-10 (the lane's own battery runs were never recorded; the audit's re-run is the committed record): `cargo test -p playback-engine --test soak_scenarios` (CI-short default profile) 29/29 PASS in 39.4 s, all five scenarios inside budgets; forced-threshold profile re-run (`--release`) FAIL — 2 violations, exit ≠ 0, the expected enforcement proof (measured span 849,815 cycles reproduced exactly; live bytes +84 B vs the recorded +82 B — run-to-run variance, i.e. real measurement); `cargo test -p playback-engine` (whole crate) green incl. the pre-existing soak loop (12/12, 120.05 s); `cargo clippy -p playback-engine --all-targets --locked -- -D warnings` CLEAN; `cargo fmt --check` (workspace) CLEAN; `cargo run -p xtask -- lint-literals` PASS (0 violations / 452 allowlisted).
- Honest gap: the full-workspace `cargo test --workspace --all-features` run and `check-architecture` that the report's §10 originally listed were never recorded on this branch and were not re-run by the audit — no committed result exists for those two items (§10 now states this; the scoped battery above is the committed evidence).
- Provenance: the original W24-B session ended without appending this entry, so the synthesis report's §10 "recorded verbatim in the W24-B worklog entry" citation dangled (review W24-R4 finding P1-1). This entry was appended 2026-11-10 23:02 UTC by the W24-M r2 review-fix round from the lane's commit chain and the W24-R4 audit's recorded re-run results, labeled as such above. Nothing else about the lane's record was rewritten — the report's numbers check out against the raw evidence.

Stage Summary:
- The five-scenario bounded soak RAN for real in-sandbox (release, linux/x86_64, 30.9 min of measured walls: 3 × 480.0 s + 216.4 s + 200.1 s); the 5 MB/h design target is FROZEN on the measured numbers — worst duration-scenario live slope 0.060 MB/h (83× under), worst RSS slope 0.403 MB/h (12× under), count scenarios +0.0 B/cycle live drift. Raw evidence: seven verbatim reports under docs/soak/raw/.
- Lane: 9 commits on w24/b (8bcaf62..0acd8c4), 19 files, +4,529/−0; nightly 4 h tier + LSAN job wired and billing-blocked (§8 of the report).
- This entry is the record the synthesis report's §10 points at; its battery results are the W24-R4 audit's independent re-run (green), with the two never-recorded battery items named honestly.

---
Task ID: W24-C
Agent: w24-c (rounds 1-4, issue #56 — cards 8.E1-8.E3, doc 11 §9.3; branch w24/c)
Task: Cross-platform packaging: Windows MSI/WiX + Authenticode driver, macOS dmg + codesign/notarize driver, Linux deb/AppImage/rpm lanes, tar.gz/zip archives, per-artifact SBOMs + checksums, release.yml 3-OS matrix, packaging runbook, full_gate step 11; honest BLOCKED labels for user-owned credentials.

Work Log:
- Landed across 5 prior commits (60e837d r1: packaging.toml + schema + packaging-env.py bridge + MSI/dmg/rpm/signing drivers; 44bd866 r2: deb+AppImage lanes + shared stagers; 1668fb1: gate.sh + ps1-sanity.py + two real bug fixes; 6eeb3d5 r3a: Linux lane PROVEN locally — real .deb + AppImage + tar.gz/zip, packaged-binary launch tests, SBOMs, checksum manifest, evidence; f048753 r3b: archive lanes + kind-glob bug fix caught by dry-run).
- r4 resumed a killed context with 4 dirty files + 3 untracked. Reviewed every one: the release.yml rewiring onto scripts/packaging/* drivers (old inline literal-laden tar.gz steps + scripts/release/* calls replaced by config-driven lanes) was complete and coherent — kept, finished, validated.
- r4 additions: (1) UNIVERSAL-BINARY DECISION (card 8.E2 deliverable, previously missing): TWO per-arch dmg/app sets, NOT one lipo'd universal .app — rationale (10x-runner build cost, per-artifact self-containment incl. per-slice signing per docs/inventory/platform-macos.md, arch-named downloads, reversibility in release.yml alone) recorded in the macos-arm64 job header + docs/packaging/README.md. (2) .gitignore: the local pinned tools (appimagetool.AppImage, type2-runtime) + __pycache__/ (pyc was accidentally committed r1, removed r3b — recurrence now impossible). Verified both binaries are NOT in the commit.
- gen-sboms.sh (new, untracked until now): the ONE per-artifact CycloneDX SBOM writer — wraps xtask sbom, suffix + specVersion from packaging.toml with a hard drift check; called by every release.yml job and by verify-linux-artifacts.sh step 8 (one writer, two callers). Negative test (forced specVersion mismatch → exit 1) in the r3 evidence log.
- Validation battery ACTUAL at r4 HEAD: python yaml.safe_load on release.yml = 6 jobs / 63 steps / 41 run-blocks, needs-graph correct (release needs all five build jobs); bash -n + sh -n over all 14 scripts/packaging/*.sh + scripts/full_gate.sh = clean; per-run-block bash -n with ${{ }} neutralized = 41/41; full_gate.sh STEP 11 executed via its exact invocation body → "gate.sh: PASS ... packaging: PASS" (51 env vars, 14 scripts, stagers executed against a throwaway tree, both .UNSIGNED.txt fallback contracts executed for real); xtask lint-literals PASS (0 violations, 452 allowlisted); xtask check-architecture PASS 5/5; xtask sbom --check no drift. No Rust files touched this branch (delta vs origin/main = packaging/scripts/docs/workflows only), so fmt/clippy were not owed.
- docs/packaging/README.md (runbook): two config planes table, artifact matrix (per-OS/arch × artifact × builder × local-proof status), per-platform runbook incl. the sandbox-provable Linux lane commands, the gate, the release-workflow walkthrough, signing-posture table, blockers table (R-35 Apple account, Windows Authenticode cert, CI billing — owner user, next step each; AppStream metadata; arm64 runner availability), evidence links. All referenced paths verified to exist.
- Commits: 60e837d, 44bd866, 1668fb1, 6eeb3d5, f048753 (r1-r3), this r4 commit (release.yml universal-binary decision + gitignore + gen-sboms.sh + runbook + evidence + full_gate step 11 + this entry). Pushed origin w24/c at r4 close. main untouched.

Stage Summary:
- Issue #56 packaging deliverable complete on w24/c: config-driven (ZERO product/packaging literals in scripts or workflow — lint-literals green) MSI/zip/dmg/deb/AppImage/rpm/tar.gz lanes with real signing drivers and explicit honest-unsigned fallbacks, per-artifact CycloneDX SBOMs, per-set + global SHA-256 manifests, tag-triggered 6-job release workflow with pinned+SHA-256-verified tooling, runbook + evidence, gate wired as full_gate step 11.
- D-10: MEASURE — step-11 gate PASS via the exact full_gate invocation; yaml.safe_load 6 jobs/63 steps/41 run-blocks; 41/41 run-block bash -n; lint-literals 0 violations; architecture 5/5; sbom no drift; plus the committed local Linux-lane proof (real .deb 2,073,444 B, AppImage 1,276,408 B, tar.gz 3,054,325 B, spec 1,038 B, 4× SBOM 121,900 B, per-set SHA256SUMS — docs/packaging/evidence/). NOT TESTED — the workflow has NEVER executed (CI billing blocked, issues #53/#56): wix build, signtool, codesign/notarytool/stapler, hdiutil, ditto, msiexec, rpmbuild, gh release create are all [VERIFY at first live run]; the binary .rpm exists only in CI (this host has no rpmbuild — the spec was validated instead); minisign detached signatures + cargo-auditable embedding from doc 11 §9.3 are [PROPOSAL], not shipped (checksums-only posture, documented). WHAT BREAKS — packaging.toml/product.toml drift fails step 1 of the gate loudly (validated schema, 51 :? asserts); renaming convention changes would silently neuter the release-job kind glob (the r3-caught bug class — now dash-form, dry-run-verified); a billing lift without a workflow_dispatch dry-run first cuts a possibly-broken release. WHO HITS IT FIRST — the user lifting the spending limit (runbook blockers table says: dispatch dry-run FIRST); the next agent editing packaging.toml without the schema; the first live Windows run (WiX net6.0 rollForward on the windows-latest .NET — [VERIFY]).
- Blockers handed to the user (all tracked in the runbook): R-35 Apple Developer account (macOS signing/notarization), Windows Authenticode certificate budget, Actions spending limit.

---

Task ID: W24-D
Agent: W24-D (compat-matrix lane, issue #53, cards 8.B1–8.B2, branch w24/d). Entry appended 2026-11-10 by W24-P (the wave-24 review-fix lane) per W24-L finding P1-1 — the lane's 9 commits shipped without the mandatory §4 append; content below is reconstructed from the commit messages and cross-checked against W24-L's r2 audit (docs/reviews/w24-d-compat-findings.md, every number re-verified there against the raw rows).
Task: The full-corpus compatibility matrix (issue #53, cards 8.B1–8.B2): the standalone runner (tests/compat), the real 166-file run, the parity-tool transitions writer + its application, the gap triage, and the CI job.

Work Log:
- 08eee64: round-1 skeleton (recovery commit) — config/corpus/decode/oracle/proc/sha256 modules, standalone-workspace wiring per the bench-vs-vlc precedent; report.rs, the config JSON and the regen manifest still TODO at that point.
- 3dfe1c0: runner finished — compiles and clippy-clean under the mirrored deny lints (no unwrap/expect/panic/dbg, unsafe forbidden); config schema, regen-corpus manifest, version-asserted live-oracle wiring.
- 7223106: THE REAL RUN — 166 corpus files (46 committed + 54 malformed + 66 regenerable, manifest-anchored) decoded through the player's real open path (open_source → container registry → HostDecoderProvider → codec_api::Decoder): 84 decoded / 22 decoded-partial / 6 refused / 0 demux-fatal / 54 malformed all-typed (zero crashes). Lanes: linux-native live; web-wasm via recorded build+test steps (both exit 0); windows/macos blocked-by-external citations. Oracle differential over the lossless-comparable family: 29 bit-exact (MD5-prefix) + 6 f32 tolerance-FAIL. Reports + raw rows + transitions outbox committed under docs/corpus/compat-*.
- 932d461: the parity tool's transitions writer — cargo xtask compat-matrix apply-transitions (tools/xtask/src/cmd/compat_matrix.rs): re-checks every from-status against the live feature-matrix, rejects verified/stale/unknown rows, coalesces same-row+platform proposals, appends the machine-written ledger; 10 unit tests.
- 3fdbf6d: applied the 5 run-emitted transitions + 1 triage-authored addendum (the f32 oracle-tolerance family, which the decode-level rules do not cover) through the tool — 6 machine-written ledger lines, zero hand-edited statuses; §6 of the md re-rendered post-apply as the gap triage; post-apply cargo xtask feature-audit PASS (252 rows, implemented 192→187 / partial 142→147 — the five linux cells; no inference for platforms that did not run).
- 3d90424: a configured-but-absent VLC binary is a recorded absence (rows cite the committed 2026-11-09 record, labeled), never a tool error — CI-wiring prep for hosts without the pinned Debian build.
- ecc615e: gitignore target-xtask/ (the ad-hoc xtask build dir in worktrees; the c339ee8 precedent).
- 4c3f6b8: config/2 — the native-decode lane's platform is REQUIRED and explicit, never host-inferred (a windows CI sweep must attribute its evidence to the windows column; old configs refused, not defaulted).
- 2e01a2c: the CI job — .github/workflows/compat-matrix.yml (workflow_dispatch only, permissions: read, fail-fast: false, 3-OS matrix, SHA-pinned actions) + tests/compat/ci/derive_config.py (the per-OS config deriver: re-attributes the native lane, drops this-OS blocked lanes, rewrites sibling-lane citations, refreshes claims from the live matrix, redirects reports).

Stage Summary:
- Issue #53 cards 8.B1–8.B2 delivered on w24/d (9 commits, 25 files +14,485/−15): the honest evidence engine, its first real run, and the closed transitions loop (runner → outbox → rule-checked apply → feature-audit). Verification, per W24-L's r2 audit re-run on a fresh worktree: matrix counts cross-checked md↔json↔jsonl — all green; the dev's three smoke re-runs (committed-config linux, windows-config, CI-derived linux) reproduce the 166 raw rows with zero non-timing differences, and the CI-derived re-run with refreshed claims emits 0 transitions (the loop closes); 10/10 xtask compat-matrix tests; cargo fmt clean (both workspaces); clippy --all-targets clean under the mirrored deny lints; lint-literals PASS (0 violations). Residuals the audit flagged (P2: in-row refusal surfacing, wasm-cell derivation; P3: unit-test coverage, wording/labeling) are being fixed by W24-P on w24/p.

---

Task ID: W24-E (r4 closeout — issue #58, cards 8.G1+8.G2, wave 24 lane E)
Agent: w24-e
Task: Docs completeness sweep + release checklist + RC smoke (issue #58). Rounds 1–3 landed the record/ADR/guides/README work (commits 492db70, d915188, 3a7fd06, 559b897, 9a0a380, ff001e5, 9cd090f); this round: the audit report, the executable release checklist, the release-notes skeleton, the RELEASE.md reality fix, and push.

Work Log:
- Verified the version-stamping mechanism before documenting it (8.G2 card says "from product.toml" — reality is two-track): `scripts/release/product-env.py --print` ran green (PRODUCT_VERSION=0.1.0-dev, PRODUCT_VERSION_CODE=1, MSI_VERSION=0.1.0, BIN_CLI=lumen, BIN_DESKTOP=lumen-desktop); binary `--version` = `env!("CARGO_PKG_VERSION")` from apps/cli/Cargo.toml (help.rs [PROPOSAL] — identity wiring pending), so product.toml and the two crate versions must be bumped together. Documented in release-checklist §0 + RELEASE.md RC short form.
- Format-record completeness re-audit against feature-matrix.toml (73 format rows = 21 container + 16 video-codec + 19 audio-codec + 17 subtitle-format; 37 claimed desktop = 14/6/9/8; 0 web — recomputed with a parse script, method recorded). Found ONE gap: network-dash (partial; 6.C1a-c + 6.C2 landed) had no docs/formats record while sibling lanes (hls, http) did — closed it by authoring docs/formats/streaming-dash.md from the matrix row's recorded evidence (27 dash #[test] fns re-counted on disk: 12 resolve + 4 fuzz + 4 abr + 7 sched); statuses untouched. Getting-started §3 now names the 37th claimed row (subtitle encoding detection) so the enumeration is exact.
- Wrote docs/audit/docs-sweep-20261110.md (the 8.G1 deliverable): all-green audit — §1 format records complete (37/37 claimed rows covered + the no-record-owed register re-verified per row), §2 ADR register current (12/12 files, numbering gaps + ADR-010/013 drift flags), §3 user guides verified against sources of truth (150 keymap bindings counted; /keyboard-shortcuts route + bindings.ts + hotkeys:check exist; schemas/settings.toml 32 [[setting]] incl. the five extensions.crash-* keys; guide commands vs help.rs surface), §4 README overclaim audit (greps re-run — zero live hits; the three 559b897 fixes listed: false "prebuilt binaries published" claim, stale Phase-6 status, unbounded capability prose → 37/73/0), §5 findings F-1..F-5 (F-1 App.tsx merge markers + F-2 stale settings screen = main code defects, fix verified on LOCAL w24/a 70ed69c/771259c — origin/w24/a NOT pushed at sweep time, honest record + wave-merge re-run note; F-3/F-4/F-5 fixed in-sweep), §7 every re-run command with recorded output.
- Wrote docs/release-checklist.md (the 8.G2 deliverable): end-to-end RC runbook — §0 verified two-track version stamping, §1 pre-flight (with the honest RED row: config-validator fails 4 violations, all missing assets/brand/* files — and full_gate.sh does NOT run it, so RELEASE.md's old "or the gate still passes" line was doubly stale), §2 rc-smoke section (recorded engine/wasm green, web red on main F-1/F-2, RC-day rule: 8/8 must pass on the RC commit), §3 build matrix (release.yml 6 jobs; lane C's scripts/packaging/* on w24/c as execute-at-wave-merge rows — origin/w24/c not pushed), per-OS launch/install/uninstall table, §3.4 update-URL wiring documented as the honest reality (no updater ships; MSI UpgradeCode replace-on-upgrade is the whole story), §4 SBOM+checksums, §5 notes, §6 the RC sequence, §7 blocked rows + owners (CI billing, 3-OS machines, R-35 Apple, Windows cert, ADR-010 patent sign-off, wave-merge, sandbox disk).
- Wrote docs/release-notes/skeleton-v0.1.0.md: first-release notes skeleton — product.toml version (0.1.0-dev → v0.1.0 expected pair), honest scope statement (37/73 desktop rows, zero web, ADR-005 null audio, ADR-004 desktop skeleton, no updater/telemetry), known-limits references (KNOWN-LIMITS.md + RELEASE.md register), changes via release-notes.sh (dispatch dry-run executed and recorded).
- Fixed docs/RELEASE.md RC-checklist section to match reality (config-validator RED note + gate-doesn't-run-it note + crate-version bump + rc-smoke row + pointer to the full checklist).
- Verification battery this round (all recorded in audit §7): bash -n on scripts/rc-smoke.sh + 8 release scripts OK; node --check rc-smoke-web.mjs OK; py_compile product-env.py/gen-icon.py OK; product-env.py --print exit 0; release-notes.sh dry-run exit 0; release.yml yaml.safe_load OK (6 jobs); xtask via the shared cached target/debug/xtask binary (sandbox disk hit 100% — cargo run unavailable, same binary, recorded for honesty): lint-literals PASS 0/452, check-identity PASS 0/453, sbom --check no drift (157 components, subject Lumen 0.1.0-dev), config-validator FAILURE 4 (missing brand assets — carried as checklist row). rc-smoke NOT re-run this round (disk full; re-run is a blocking RC-day row and would reproduce the same F-1/F-2 red until wave merge).
- Commits this round: bc97962 (DASH record + guide tweak), 8348f17 (audit), be4fedb (checklist + skeleton + RELEASE fix), + this entry; then push origin w24/e. Total lane: 11 commits.

Stage Summary:
- Issue #58 (P8-7, cards 8.G1+8.G2) deliverables complete on w24/e: docs/audit/docs-sweep-20261110.md (all green), docs/release-checklist.md (executable end-to-end, every row = command/artifact, blocked rows carry owner + next step), docs/release-notes/skeleton-v0.1.0.md, scripts/rc-smoke.sh + rc-smoke-web.mjs (ff001e5), plus the r1–r3 record/ADR/guides/README work.
- Blocked rows (owner user): CI billing (pipeline never executed), 3-OS runner machines (Windows/macOS install/launch/uninstall + MSI upgrade test), Apple Developer account R-35 (notarization), Windows code-signing certificate, ADR-010 patent sign-off before any binary distribution. Blocked rows (Lead): wave-24 merge (lane A F-1/F-2 fixes unmerged — fix verified on local w24/a; lane C packaging drivers unpushed). Local blocker recorded: sandbox disk 100% (full_gate + rc-smoke re-run deferred to RC day; xtask gates ran via the cached binary).
- D-10: MEASURE — 73/37/0 matrix recompute, 150 bindings, 32 settings, 5 crash keys, 27 dash tests re-counted, 11 scripts syntax-clean, lint-literals 0 violations, check-identity 0 violations, sbom no-drift, config-validator 4 violations (all missing brand assets), product-env + release-notes dry-runs exit 0. NOT TESTED — the CI pipeline (billing), rc-smoke full re-run (disk + would-be-same-red), anything on Windows/macOS. WHAT BREAKS — release-checklist §0's two-track versioning: bumping product.toml without the crate versions ships artifacts whose names disagree with `lumen --version`; the config-validator red silently persists if relied on full_gate (documented in both files). WHO HITS IT FIRST — the Lead at wave-24 merge → RC sequence (checklist §6 step 1–4).

---

Task ID: 7.w24f (W24-F r2, issue #51 — Phase 7 exit gate [7.L1-V])
Agent: w24-f
Task: Run the Phase-7 exit-gate battery, gather evidence, write docs/gates/phase7-exit.md (verification lane; no feature implementation).

Work Log:
- Resumed with round-1 logs uncommitted; reviewed tails (clippy exit 0; evidence suite 3 passed/1 ignored; fmt empty; workspace tests exit 101, 4 failing targets), committed them FIRST as a4e35e7 (context-kill protection).
- Completed the battery with per-run headers + exit codes in docs/gates/logs/phase7-exit-*.log: fmt (0), clippy r2 (0; r1 body kept as -r1.log), xtask gates 7/7 PASS (architecture 5/5, codegen/schema-gen/fixtures no-drift, lint-literals 0 violations, check-identity 0 violations, sbom no-drift), wasm32 lumen-web + theme-tokens canary (exit 0, plain + --locked), cargo-deny 0.20.2 (pinned tarball downloaded, SHA-256 == ci.yml pin; licenses/bans/sources/advisories ok), UI battery (typecheck/lint/test FAIL, l10n PASS), parity-report + feature-audit (PASS). Committed 2bf5b66.
- Diagnosed all 4 workspace failures + UI failures to root causes with in-source evidence: OF-1 committed merge markers ui/src/App.tsx:15-24 (merge 45c5c6f; fix in flight w24/a 70ed69c); OF-2 stale pins after c71cd5a grew settings seed 27→32 (settings-schema lib.rs:155, integration.rs:117, cli help.rs areas 6→7); OF-3 ui generated schema.ts stale (settings:check exit 1); OF-4 rebrand reverse-control test roots at the shared target dir (manual repro: rebrand no-op exit 0, README byte-identical) — test-infra, not product; OF-5 feature-matrix.toml:416 note claims VT/MF/VA-API landed but hw/mod.rs:53-58 retracts MF (NOT IMPLEMENTED, no mediafoundation.rs ever existed).
- Verified per-card substance: platform adapters (linux 28/0 on-host incl. real sysfs/pulse probes; mac/win 16/0 seam each; unsafe audit module-scoped + SAFETY invariants), hw-decode truth-table honesty in hw/mod.rs (VT compile-verified; VA-API failure-path runtime-tested; MF retracted), hotplug/discovery (#42), transcode/recording/remux (#43/#44), crash isolation incl. playback-unaffected proof (#46: plugin_crash_isolation 6/0, examples kit 5/0), web pillars (#48: lumen-web 111/0 + wasm build), control-protocol first-pass-secured (#50: loopback-refused + token/lockout tests green; sweep-control.md); #49 honestly in-flight on w24/a.
- Wrote docs/gates/phase7-exit.md: verdict NOT PASSED (5 findings, all bounded, no design work), evidence ledger (22 checks), per-card table, web parity honesty (web 0/252 conservative-honest), Phase-8 launch list + external blockers (CI billing, 3-OS machines, Apple R-35, Windows cert — user-owned), Lead re-run battery.

Stage Summary:
- Verdict: Phase-7 exit gate NOT PASSED at c339ee8 — 18/22 battery checks green; 5 open findings (P0: App.tsx markers; P1×3: pins/generated/matrix-note; P2: test-infra root discovery). Exit is achievable at wave-24 merge: land w24/a 70ed69c, bump 3 pins + npm settings:generate, fix matrix note, then re-run the §8 battery. Per-card Phase-7 substance verified in-tree and green; hardware-dependent rows honestly blocked. All evidence in docs/gates/logs/phase7-exit-*.log, report docs/gates/phase7-exit.md. Commits a4e35e7, 2bf5b66, + this entry on w24/f; pushed origin w24/f; main untouched.

---

Task ID: 7.w24g
Agent: w24-g (gate-findings fixer lane; this §4 entry appended retroactively by w24-n under the Lead's W24-N assignment — the g lane shipped its commits without it, R5-2)
Task: Phase-7 exit-gate honesty fixes OF-2..OF-5 (docs/gates/phase7-exit.md on w24/f): derive the stranded seed-count pins, regenerate the stale UI artifact, root the rebrand reverse-control test hermetically, retract the feature-matrix Media-Foundation over-claim. 7 commits on w24/g @ 8121cbf == origin/main c339ee8 + 1c48902/64757b1/ba9659f/de9b341/9b15003/56b59c1/8121cbf.

Work Log:
- OF-2 (1c48902 fix(settings)): the three stale 27/14-class pins (crates/settings-schema/src/lib.rs, crates/settings-schema/tests/integration.rs, apps/cli/src/help.rs) now DERIVE every count from schemas/settings.toml via independent raw-toml::Table parses (the crate under test is never asked what its own input contains); help.rs's areas test renamed areas_block_lists_the_six_areas -> areas_block_lists_every_screen_the_seed_declares and upgraded to FULL Areas-block string equality (screens, per-screen counts, engine-internal annotation). Added the cross-artifact shape guard seed_shape_is_what_every_committed_artifact_self_reports: the seed's own shape (32 entries / 7 screens / 14 areas) must be exactly what G1's d.ts header and G2's every per-screen help row self-report, failure messages naming the two regen commands (cargo xtask schema-gen; npm run settings:generate in ui/).
- OF-3 (64757b1 fix(ui)): regenerated ui/src/screens/settings/generated/schema.ts after w23d's seed growth — pure addition of the 5 SETTING_EXTENSIONS_CRASH_* constants + 5 matching SETTINGS_MSGIDS; the ui=false engine-internal extensions rows correctly stay out of SETTINGS_ENTRIES.
- OF-4 (ba9659f test(rebrand)): tests/rebrand/rebrand_skeleton.rs repo_root() now derives from compile-time env!("CARGO_MANIFEST_DIR")/../.. (always the compiling worktree) instead of walking up from CARGO_BIN_EXE_xtask (which under the shared CARGO_TARGET_DIR resolved a sibling lane's stale checkout), and run_rebrand pins .current_dir(repo_root()) to match the tool's cwd-based workspace discovery — tool and test provably operate on the same tree. The no-op proof's assertions unchanged (real product.toml, exit 0 + "result: no-op", byte-compare of all 5 surfaces).
- OF-5 (de9b341 docs(matrix)): docs/feature-matrix.toml codec-video-h264 note now states Media Foundation (Windows) NOT IMPLEMENTED — no MF lane exists (matches the hw/mod.rs truth table: only vaapi/ + videotoolbox/ shipped), with the retraction attributed and gate-OF-5 provenance recorded. Status cells unchanged (partial, already honest).
- Hygiene: 9b15003 fmt of the OF-2 test changes; 56b59c1 gitignore target-xtask/ (the isolated build dir OF-4 verification needed under the shared-target regime); 8121cbf clippy redundant_closure style fix in help.rs + the settings-schema test.

Stage Summary:
- All four in-scope findings fixed without weakening any test (removed asserts = exactly the 4 stale pins, each replaced by a derived equal-or-stronger one). Verification source: R5's independent audit of this lane, docs/reviews/w24-g-fixes-findings.md on w24/r5 (verdict SHIP) — 4/4 previously-failing suites green from provenance-verified binaries (settings-schema lib 43/0, integration 8/0 incl. the new shape guard; lumen-cli lib 125/0; xtask --test cli 12/0/3); fmt/clippy -D warnings clean on the three touched crates; lint-literals PASS (0 violations, 452 allowlisted); npm run settings:check PASS (byte-exact). OF-1 (App.tsx markers) remains lane A's finding, out of this lane's scope.

---
Task ID: W24-H
Agent: w24-h (rounds 1-7, issue #56 — W24-R1 packaging review fixes; branch w24/h)
Task: Fix all 10 W24-R1 findings on the w24/c packaging lane (2 P1, 4 P2, 4 P3): sign-macos subshell masking, release.yml secrets wiring, wxs docs-from-config, WIX_EXE POSIX handoff, evidence digests, payload claims, signtool SDK walk, runbook sandbox path, ps1 temp-prefix literals, app-zip sidecar.

Work Log:
- 6 commits on w24/h (base origin/w24/c @ fb97822): 1f19159 P1-1; 1337ca0 P1-2; e4725d0 P2-1/P2-2/P3-1; 00821d3 P2-3; f6ebd7e P2-4; 7f2f894 P3-2/P3-3/P3-4. 9 files: release.yml, docs/packaging/{README,evidence/README}.md, scripts/packaging/{sign-macos,sign-windows,build-msi}.sh + {build-msi,test-msi-upgrade}.ps1, scripts/release/gen-wxs.sh.
- P1-1: resolve_identity now runs in the MAIN shell behind an explicit rc contract (0 resolved / 2 no-credentials / anything else already died); die() exits the script itself so the EXIT trap cleans the scratch keychain state. Proven for real: creds absent -> sidecar + exit 0 (gate step 6); creds present + step failing (fake codesign/security) -> exit 1, NO sidecar, TMPDIR cleaned. Pre-fix script under the same scenario left the scratch tmp.* dir behind (subshell assignment died with the subshell). Honest note recorded in the worklog file: the review's literal "|| true swallows die()" mechanism did not reproduce in dash/bash (set -e caught the failed assignment, old script also exited 1 there) — the fix replaces the accidental set -e reliance + dead cleanup with an explicit, locally-proven contract.
- P1-2: all 8 [signing.*] secret names mapped into the steps that sign (windows MSI step, both macOS dmg steps) + a release-job visibility step (unset = ::error:: hard fail, empty = ::warning:: absent credential). Cross-checked TOML names == workflow mappings 8/8; guard logic simulated both ways under bash (the workflow pins defaults.run.shell: bash, so the ${!cfg} indirection is legal on every runner); 42/42 run-blocks parse.
- P2-1: gen-wxs.sh + build-msi.ps1 + build-msi.sh all generate/stage the Docs component from PACKAGING_DOCS (first doc = KeyPath, ids sanitized, duplicate basenames + empty list refused loudly). Functional proof: 2-doc config -> 2 rows; 4-doc list -> 4 rows; duplicate/empty -> hard errors.
- P2-2: build-msi.ps1 converts git-bash WIX_EXE (/d/... -> D:\...) before Test-Path [VERIFY at first live run; no pwsh in sandbox — ps1-sanity PASS]. P2-3: evidence table regenerated from the committed log (5/5 digests grep-found in it; the 2 stale ones confirmed absent; unsupported byte sizes dropped). P2-4: per-artifact payload table replaces the false "every artifact packages BOTH binaries + docs" claim (AppImage/.app are desktop-only channels, stated). P3-1: PRODUCT_LOWER-driven temp prefixes (only sanctioned cargo package ids remain). P3-2: signtool walks ALL SDK version dirs newest-first — side-by-side replica test: old logic gives up on a tree where uap10.0.*/x86 sort "newest", new logic finds 10.0.22621.0/x86/signtool.exe. P3-3: runbook marks the cargo-env.sh line as sandbox-only. P3-4: both macOS zip steps ship bundle's .UNSIGNED.txt as dist/<base>.zip.UNSIGNED.txt.
- r7 verification battery: bash -n + sh -n on all touched scripts + 14-script sweep clean; yaml.safe_load OK; 42/42 run-block bash -n; lint-literals PASS (0 violations, 452 allowlisted); gate.sh PASS (= full_gate step 11: 51 vars, stagers, both unsigned-fallback contracts executed); ps1-sanity PASS on both .ps1.
- Detailed per-finding before/after evidence: docs/worklog/w24-h-review-fixes.md (docs/worklog/ placement per this task's explicit scope, W24-R1 review-scout precedent; this root entry is the AGENTS.md §4 record).

Stage Summary:
- All 10 W24-R1 findings fixed and verified on w24/h; tree clean; pushed origin w24/h. Honest boundaries unchanged: the release workflow has still never executed (CI billing blocked); wix/signtool/codesign/notarytool/msiexec paths remain [VERIFY at first live run], labeled in-file. No Rust code touched (fmt/clippy not owed). main untouched.

---

Task ID: W24-I (review fixer 2 — wave 24 lane I, issue #58 review fixes)
Agent: w24-i
Task: Fix the W24-R2 review findings against w24/e (docs-sweep audit, release checklist, format records, rc-smoke portability). All 3 P2s fixed, 5 of 6 P3s fixed, P3-9 (worklog split) deferred to Lead per the reviewer's own disposition.

Work Log:
- Read findings: `git show origin/w24/r2:docs/reviews/w24-e-docs-findings.md` (9 findings: P2-1..3, P3-4..9). Worktree wt/w24i off origin/w24/e @ 3596918.
- P2-1 FIXED (docs/audit/docs-sweep-20261110.md §1.1 + §1.2): before — video-codec row's claimed-rows cell listed "h264-software ★" (matrix says codec-video-h264-software = blocked ×4) and omitted vp9. After — claimed cell reads "mjpeg, av1, h264, hevc, prores-dnxhd, vp9", records cell lists `video-vp8-vp9.md` (the dual-row record the §1.1 preamble already asserted); h264-software moved to the §1.2 register (blocked row; its lane-authored gap record `video-h264-software.md` @ 3a7fd06 named). Re-verified post-fix with a toml parse: claimed video-codec rows = av1/h264/hevc/mjpeg/prores-dnxhd/vp9, h264-software blocked ×4, verdict 37/73 + 14/6/9/8 unchanged (GREEN intact).
- P2-2 FIXED (docs/formats/container-avi.md:68, audio-pcm.md:58): before — both cited `docs/audit/rc-smoke-2026-11-10.md` "smoke row 3" (file never committed on any branch; harness numbers headless-decode check 4). After — both cite the real evidence: the `ff001e5` commit message's recorded run (check 4 green on the same fixture, 60/60+60/60 frames) + `docs/release-checklist.md` §2 for harness/check numbering. Scoped rg for the phantom filename across docs/scripts/worklog/README: zero remaining hits.
- P2-3 FIXED (docs/release-checklist.md row 1.2 + docs-sweep §7): before — `bash -n scripts/rc-smoke.sh scripts/release/*.sh` (only the FIRST file is syntax-checked; rest become positional params, silently unchecked — pitfall re-demonstrated via /dev/shm: `bash -n good.sh bad.sh` → exit 0 with bad.sh containing a syntax error). After — both files record the loop `for f in scripts/rc-smoke.sh scripts/release/*.sh; do bash -n "$f" || exit 1; done`; loop RE-RUN with all-pass output recorded in audit §7 (rc-smoke.sh + gen-wxs, make-app, make-appimage, make-dmg, package-deb, package-windows, release-notes). py re-verified ENOSPC-safe (ast.parse both — py_compile's __pycache__ write fails at 100% disk; recorded honestly in §7).
- P3-6 FIXED (docs/formats/streaming-dash.md): before — "Test count on disk at authoring: 27 dash fns inside the crate's 191 lib tests (2 ignored)" (191+2 was true at 93b7614 = 193 fns; disk has 206). After — dash 27 re-counted current (12+4+4+7), the 191 figure pinned explicitly to the 6.C2 flip commit 93b7614, today's 206 stated. Counts re-verified: dash 27; crate 206 today; 193 at 93b7614 (git grep).
- P3-7 FIXED (scripts/rc-smoke.sh check 6): before — GNU-only `stat -c %s "$WASM"` (macOS/BSD stat reads invalid → probe 0 → check always FAILs on a Mac). After — `wc -c < "$WASM"` (POSIX; card-mandated cfg/perl-free form) in both the size test and the PASS byte-count echo, with an explanatory comment. Verified: identical output to stat on Linux (9339 == 9339 on a real file); full check-6 condition simulated on a fake 1.5 MB \0asm module in /dev/shm prints the same PASS line; missing-file path short-circuits at [ -f ] with no stray stderr; `bash -n` OK.
- P3-4 FIXED (docs-sweep §7 + §3): before — recorded `rg -c '\[\[setting\]\]'` → 32 (actually prints 33; line 13 of the schema is a comment containing the literal). After — anchored `rg -c '^\[\[setting\]\]'` → 32 recorded with the correction note; §3's settings-guide row cites the anchored command. Re-run: unanchored 33, anchored 32.
- P3-5 FIXED (folded into the P2-3 commits, both locations): before — "8 scripts" for `scripts/release/*.sh` (glob is 7: gen-wxs, make-app, make-appimage, make-dmg, package-deb, package-windows, release-notes). After — both checklist row 1.2 and audit §7 say 7 release scripts; 8 = rc-smoke.sh + 7.
- P3-8 FIXED (docs/release-checklist.md §3.2 + §7 table): before — "`origin/w24/c` was not pushed at authoring time" (stale: ls-remote → fb97822; evidence log + all scripts/packaging/* drivers verified on the remote branch via git ls-tree). After — clause struck per the file's own §8 rule with the push evidence recorded; rows stay execute-at-wave-merge (branch unmerged into main is the remaining condition); §7 blocked-rows row updated (lane A origin/w24/a still absent — re-verified, that half remains true as written).
- P3-9 DEFERRED (worklog split): the consolidated W24-E entry is another lane's append-only history; AGENTS.md forbids editing other agents' entries, and the reviewer's disposition is "none required; optionally split at wave merge if the Lead wants". Left as-is, deferral recorded here.
- Verification battery post-fix: bash -n loop all 8 sh OK; node --check mjs OK; `cargo run -p xtask --locked -- lint-literals` → **PASS, 0 violations, 452 allowlisted** (matches the audit's recorded allowlist count); matrix recompute 73/37/0; scoped phantom-file grep zero hits; `git status` clean before each commit. Sandbox disk hit 100% mid-lane (git commit ENOSPC) — freed 415 MB by deleting `target/debug/incremental` (pure cargo cache; builds were already ENOSPC-dead, no running build could be harmed) → 96% used.
- Commits: f00385c (P2-1), 88c680d (P2-2), d6c3920 (P2-3+P3-5), eddd1db (P3-6), 61fe4f3 (P3-7), cda3d8a (P3-4+P3-8), + this entry; push origin w24/i.

Stage Summary:
- All W24-R2 P2 findings fixed and re-verified (P2-1 table enumeration, P2-2 dangling evidence pointer + row-number, P2-3 defective syntax command re-run as a loop with true output). P3s: P3-4/P3-5/P3-6/P3-7/P3-8 fixed; P3-9 deferred to Lead (append-only worklog protocol). No new overclaims: every correction is annotated in-place as a post-review correction naming W24-R2's finding ID; verdicts (37/37 docs coverage, all-green scope) unchanged and re-confirmed.
- Files touched (ownership respected — findings-named files + worklog only): docs/audit/docs-sweep-20261110.md, docs/release-checklist.md, docs/formats/container-avi.md, docs/formats/audio-pcm.md, docs/formats/streaming-dash.md, scripts/rc-smoke.sh, worklog.md.
- D-10: MEASURE — matrix recompute 73/37/0 (14/6/9/8), anchored settings 32 (unanchored 33), dash 27 / crate 206 today / 193 @ 93b7614, 8 sh + 1 mjs + 2 py syntax-clean (per-file), lint-literals 0 violations/452 allowlisted, ls-remote w24/c = fb97822, wc-vs-stat byte equality on a real file, check-6 condition simulated PASS on a fake wasm module. NOT TESTED — full rc-smoke execution (disk + would reproduce the recorded F-1/F-2 web red until wave merge; engine/wasm halves already recorded green at ff001e5), anything on actual macOS/BSD hardware (portability fix verified by POSIX semantics + Linux-identical behavior, not on a Mac), py_compile literal form (ENOSPC — ast.parse equivalent recorded). WHAT BREAKS — nothing downstream: all fixes are accuracy corrections whose conclusions were independently true (reviewer re-verified each before filing); the one behavior change (rc-smoke check 6 size probe) is provably identical on Linux and now correct instead of always-failing on macOS. WHO HITS IT FIRST — the Lead at wave-24 merge reading the audit §1.1 table (now names the true 6th row) and the RC-day executor of checklist row 1.2 (now a real all-scripts syntax gate).

---
Task ID: W24-J (review fixer 3 — wave 24 lane J, R7 findings on w24/i)
Agent: w24-j
Task: Fix W24-R7's two findings against origin/w24/i (NEW-1 checklist table rendering; NEW-2 stale format-record status front-matter). Both fixed.

Work Log:
- Read findings: `git show origin/w24/r7:docs/reviews/w24-i-fixes-findings.md`. Worktree wt/w24j off origin/w24/i @ ca42190.
- NEW-1 (P2) FIXED (docs/release-checklist.md row 1.2): before — the P2-3 loop `bash -n "$f" || exit 1` sat in the 4-column table cell with two raw unescaped pipes; GFM splits cells on raw pipes before inline parsing (even inside code spans), so GitHub rendered the command truncated at `"$f"` and an EMPTY Status cell. After — both pipes escaped as `\|\|` (2 characters, the reviewer's verified form), plus a parenthetical in the cell noting the escape so a future editor doesn't "fix" it back (W24-R7 NEW-1 named). Verified with markdown-it-py 4.0.0 (commonmark + table enabled): row 1.2 now renders exactly 4 cells; the command cell's first code span contains the FULL loop `for f in scripts/rc-smoke.sh scripts/release/*.sh; do bash -n "$f" || exit 1; done` (escape resolves to `|` inside the code span, correct GFM behavior); the Status cell is intact ("[verified locally] all OK (8 sh + 1 mjs + 2 py …)"). Whole-file re-audit: all 8 tables, 0 rows with cell-count mismatch vs header; zero raw unescaped pipes remaining inside any table-row code span. The audit §7 copy untouched (list item, not a table — safe per R7).
- NEW-2 (P3) FIXED (docs/formats/video-vp8-vp9.md + docs/formats/video-prores-dnxhd.md): both records authored 2026-10-24/25, matrix flipped 2026-11-09 via the issue #7 admissions (W16-EE). Statuses independently re-verified against docs/feature-matrix.toml by tomllib parse BEFORE writing: codec-video-vp9 = implemented ×3 (web not_started); codec-video-vp8 = blocked ×4 (unchanged); codec-video-prores-dnxhd = partial ×3 (web not_started). Pins + code re-verified on disk: `rusty_vp9 = "=0.1.3"` and `filmcraft-dnx = { version = "=0.9.0", default-features = false }` in crates/codecs/Cargo.toml; crates/codecs/src/video/vp9/** and .../dnx/** exist; `rg -li prores crates/codecs/` → zero hits (ProRes not implemented). Appended dated addenda (2026-11-10) to both records — original dated content byte-untouched (diff is +67/−0): vp8-vp9 gets the VP9 implemented-×3 flip (6.M-vp9, vp9_differential 4/4, dependency record + its 2026-11-09 admission addendum) + VP8 stands; prores-dnxhd gets the partial-×3 split (DNxHD/HR implemented via filmcraft-dnx, 6.M-dnx + 6.VTA-DNX-RATIFY, dnx_differential 6/6; ProRes deferred verdict stands) + the §Limits "not_started ×4" line marked superseded.
- Both format records + the checklist re-parsed with markdown-it post-edit (clean); no other files touched (ownership: the 3 findings-named docs + this worklog entry).
- Commits: 60e3db8 (NEW-1), c5aade9 (NEW-2), + this entry; push origin w24/j.

Stage Summary:
- W24-R7's findings on w24/i both closed on w24/j: the RC checklist's row 1.2 now renders the full loop + status on GitHub (rendering defect, not data defect — raw markdown was always copy-pasteable), and the two video records now carry dated addenda that reconcile their front-matter with the post-issue-#7 matrix without rewriting history. R7's Target-1 verdict (all 8 fixed-and-verified items) is otherwise untouched by this lane.
- D-10: MEASURE — markdown-it row-1.2 render: 4 cells, full loop in one code span, status intact; 8 tables / 0 bad rows / 0 raw pipes in table code spans; tomllib matrix parse: vp9 implemented ×3, vp8 blocked ×4, prores-dnxhd partial ×3; prores repo grep zero hits; addenda diff +67/−0 (append-only). NOT TESTED — rendering on github.com itself (markdown-it with table enabled is the same rule set GitHub documents for pipe escaping). WHAT BREAKS — nothing: both fixes are docs-accuracy/rendering corrections; no code, no matrix rows, no records' original content changed. WHO HITS IT FIRST — the Lead at wave-24 merge reading release-checklist row 1.2 on GitHub (now complete) and the docs-formats owner if a future admission flips vp8 or ProRes (same addendum pattern to follow).

---
Task ID: W24-K (wave 24 lane K — small fixer; issue #51 follow-up)
Agent: w24-k
Task: Fix review scout W24-R3's findings on the Phase-7 exit-gate report (branch w24/f): 1 P2 (log-header convention) + 6 P3 accuracy items + 2 manual-run evidence annotations. Docs-only lane; every corrected fact re-verified against logs/git/risk-register before editing.

Work Log:
- P2-1 (log headers): prepended plain-text headers (prepend-only; zero body lines altered — git diff shows 29 insertions, 0 deletions) to the three round-1 logs that lacked them: phase7-exit-test.log (worktree w24/f, source tree == c339ee8, committed in a4e35e7, 2026-11-10 before 16:51:45Z, TEST_EXIT=101 recorded in body), phase7-exit-clippy-r1.log (same provenance, CLIPPY_EXIT=0 recorded in body), phase7-exit-evidence-rerun.log (same provenance; exit code honestly marked "not recorded at capture time" — body complete, 3 passed / 0 failed / 1 ignored, exit-0 inference labeled as inference). Amended §2's convention sentence + the round-1 provenance paragraph in docs/gates/phase7-exit.md to describe reality (r2 capture-time headers; parity date+cmd header; r1 headers retroactively annotated by W24-K per review W24-R3 P2-1).
- P3-1: recounted wildcard warnings in phase7-exit-deny.log — exactly 30 `warning[wildcard]:` lines (27 plural + 3 singular per-crate summaries); fixed §2 #15 "32" → "30".
- P3-2: `git rev-parse d33886d` fails; real wave-22 plugin-host merge is d33886e — fixed §4 7.F1 citation.
- P3-3: docs/14-risk-register.md R-20 = documentation drift (:111); the runner/spend row is R-16 (:107, :166 — "self-hosted runners… spend decisions go to the user") — fixed §6 blocker table parenthetical to R-16 with a one-line gloss.
- P3-4: verified 771259c (w24/a, 2026-11-10 16:33:59Z — before the report commit 833944e at 17:16:47Z) regenerates ui/src/screens/settings/generated/schema.ts (+105 lines); reworded OF-3's owner/next step to fix-in-flight-on-w24/a with the commit reference; aligned §1's closure path (lane A in-flight fixes OF-1 70ed69c + OF-3 771259c, then OF-2 pin bumps).
- P3-5: re-measured both ranges in-source — hw/mod.rs MF-retraction bullet spans :51-56 (was cited :53-58); help.rs areas_block_lists_the_six_areas spans :303-324 (was :302-321; panic site :310:9 inside both) — fixed the two citations.
- Manual-run assertions (R3 obs.): no committed logs are trivially derivable for either (rebrand control was a /tmp shell run; raw sha256sum never committed) — annotated §2 #22 and §2 #15/§7 cargo-deny bullet as "independently re-verified by review W24-R3" citing docs/reviews/w24-f-gate-findings.md on w24/r3 (tip 27f1330).
- Not touched (out of brief / no action demanded): R3 P3-6 (7.I1 branch-tip snapshot aging — was correct at report time), O-1/O-2.

Stage Summary:
- All 7 R3 findings fixed + 2 observations annotated; the gate report's NOT-PASSED verdict, findings, and evidence chain are unchanged — only accuracy/convention defects repaired. Ownership respected: docs/gates/phase7-exit.md + the three log files (header lines only) + this worklog entry (repo-root worklog.md per docs/worklog/README.md live-log convention). Verification greps: 30 wildcards, d33886e, R-16, :51-56, :303-324 all present; report remains valid markdown (table rows single-line, headers intact). Commit on w24/k, pushed origin w24/k; main untouched.

---

Task ID: W24-L r2
Agent: W24-L (wave 24 review scout, round 2 — resumed)
Task: Compat-matrix lane audit of origin/w24/d (issue #53, cards 8.B1-8.B2): matrix integrity, runner quality, parity discipline, gap-triage honesty, config-driven + CI, scope.

Work Log:
- Fresh worktree /home/z/my-project/wt/w24l (branch w24/l == origin/w24/d @ 2e01a2c); audited git diff origin/main...origin/w24/d (25 files, +14485).
- Read every runner module (main/config/corpus/decode/oracle/proc/sha256/report), the regen manifest, the xtask compat_matrix writer + mod.rs registration, compat-matrix.yml, derive_config.py, all six docs/corpus artifacts, and the feature-matrix.toml diff.
- Counts cross-check (scripted): 166 = 46+54+66 (manifest 67 − 1 FAILED regen); 84+22+6+0+54 = 166; raw-rows 166 lines; oracle 35 = 29 bit-exact + 6 f32 tolerance-FAIL; md ↔ json ↔ jsonl oracle tables ALL MATCH; prefix arithmetic + f32 threshold verdicts recomputed exactly; 35/35 lumen_md5 == raw-row pcm_md5; committed-record cross-check 20 same-realization matches / 15 fresh — identical split to the run's own lumen_fresh_md5 applicability logic; 2026-10-23 prior-art md5s agree on same-realization rows.
- Evidence reproduction: the dev's 3 smoke re-runs (linux, windows-config, CI-derived) reproduce all 166 raw rows with ZERO non-timing diffs; CI-derived re-run with refreshed claims emits 0 transitions (loop closed). 31/31 manifest sha256 pins re-verified with python hashlib.
- Parity discipline: feature-matrix.toml diff = exactly the 5 ledger rows (linux-only, machine-written notes/evidence; aac-lc coalesced 2→1); ledger 6 lines = outbox 5 + addendum 1; zero hand-edited statuses; applier rejection paths re-proven by running the 10 xtask unit tests (rejects stale/verified/unknown, coalescing, idempotency, dry-run) — 10/10 green.
- Gates re-run: fmt (root + tests/compat standalone) CLEAN; clippy --locked --all-targets on tests/compat CLEAN under the mirrored deny lints (unwrap/expect/panic/unsafe machine-proven); xtask feature-audit PASS (252 rows, 187 implemented / 147 partial — exactly 192→142 + 5 flips, pre-count verified on origin/main); lint-literals PASS; compat-matrix.yml valid YAML, dispatch-only, read perms, fail-fast false, SHA pins == ci-full.yml; derive_config.py re-run for linux+windows (claims refreshed from live matrix) and web refused; runner refuses native platform "web" (exit 2, re-run).
- Findings written: docs/reviews/w24-d-compat-findings.md — 0 P0 / 1 P1 (mandatory worklog append missing for the whole W24-D lane) / 2 P2 (4 valid-class refusals inside claimed-implemented avi/wav rows counted but never surfaced — §6.2 "all decoded (43 files)" overstates vs the 41/41 cell; web-wasm cell text hard-coded, not derived from recorded step outcomes) / 10 P3 (verify_diff defeats the R6 notes-insert branch; runner verdict logic untested; "(linux)" hard-code + dead refusal_count; "VLC VLC" duplication; LumenRefused rows mislabeled committed-record; fatal-limb dump feeds oracle; 0385 manifest entry can never regenerate; stale committed claims snapshot post-apply; read_capped unbounded read; hand-edited md §6/§8 provenance note).
- Read-only discipline held: only docs/reviews/w24-d-compat-findings.md + this worklog entry written; all builds/re-runs went to /tmp (CARGO_TARGET_DIR) or pre-existing smoke artifacts.

Stage Summary:
- W24-D lane verdict: SHIP-WORTHY EVIDENCE, process gap. The compat-matrix deliverable is genuine and reproducible (3× bit-identical re-runs, oracle method re-verified to the byte, transitions provably tool-applied, feature-audit PASS), but the lane skipped the mandatory worklog append (P1) and the report surface under-explains 4 in-row refusals (P2) plus a latent steps-cell honesty gap (P2). 0 P0. Full severity-ordered findings with file:line + evidence + suggested fixes in docs/reviews/w24-d-compat-findings.md; pushed w24/l for the Lead's fix lane.

---

Task ID: W24-M r2
Agent: W24-M (soak-review fixer, round 2 — resumed session)
Task: Close the remaining W24-R4 soak-lane review findings on w24/m (issue #54, card 8.C1 evidence) — the round-2 fixer session finishing docs/reviews/w24-b-soak-findings.md.

Work Log:
- Resumed with P3-1/P3-2/P3-3 already landed this session and runner.rs dirty mid-P3-4; finished from there. All nine findings are now closed on w24/m:
- P2-2 aa04202: report.rs worst-slope fold initializer f64::INFINITY -> f64::NEG_INFINITY (the 'Worst measured live-bytes slope' headline was dead code — max(+inf, x) is always +inf) + regression test worst_slope_headline_prints_the_largest_finite_slope.
- P2-3 3295b1b: the synthesis churn row's post-warmup span 849,815 (the 1M forced run's) -> 1,699,754 (the 2M run's); runner records the measured span; count_baseline_and_span unit-pinned against all three committed series shapes.
- P2-4 a25a427: the schema $comment's wiring-gap reference repointed from the nonexistent docs/soak/regression-guards.md to docs/soak/soak-report-20261110.md §6.2.
- P3-1 3aa4b62: run_seek_storm folds sink payload bytes — the D-6 line no longer reads '0 payload bytes' next to millions of frames.
- P3-2 0b34933: the report's profile total 29.4 -> 30.9 min (the measured walls: 3 × 480.0 s + 216.4 s + 200.1 s).
- P3-3 7f6ed4d: provenance note on the calibration report (generated by the pre-09ad85a per-cycle sampling code; the off-by-one a recomputing reviewer would hit).
- P1-1 56d74fe: appended the missing W24-B worklog entry (the entry above) — the lane's commit chain plus the W24-R4 audit's re-run battery results, provenance labeled; soak-report §10 repointed truthfully, the never-recorded full-workspace --all-features + check-architecture items named as an honest gap instead of papered over.
- P3-4 dde8674: the count scenarios now carry the absent-VmRSS honest-absence note, mirroring the fd instrument's (off-Linux symmetry; the duration scenarios already printed 'VmRSS slope: n/a').
- P3-5 3fd6f5f: seeks_per_reopen promoted from a runner.rs constant to an optional [[scenario]] key (default 16, range [1, 1024], loud refusals; loader + schema + tests; behavior-identical for shipped manifests — none sets it); the schema $comment's manifest list now names scenarios-forced-threshold.toml.
- Verification (every commit message carries its own run; the code-touching ones all re-ran the battery): cargo fmt -p playback-engine -- --check CLEAN at each code commit; cargo clippy -p playback-engine --all-targets --locked -- -D warnings CLEAN at each code commit; scoped suite cargo test -p playback-engine --test soak_scenarios green after each code change — final 32/32 PASS in 39.27 s (29 inherited + the P2-2, P2-3 and P3-5 regression tests); lint-literals PASS (0 violations / 452 allowlisted); schema JSON machine-validated (python json.load) after both schema edits.

Stage Summary:
- All 9 W24-R4 findings closed (P1=1, P2=3, P3=5): the branch's central verification citation is real (worklog entry + truthful §10), the synthesis numbers match the raw evidence, the report's headline slope line renders, and the honest-absence notes are symmetric across platforms.
- No test weakened, no behavior change for shipped manifests; the seek epoch length is now config-plane with the old constant as its default.
- D-10: MEASURE — the scoped battery above, re-run per commit. NOT TESTED — a non-Linux run of the suite (the P3-4 note's trigger; unreachable on this Linux sandbox, same as the fd note it mirrors) and the full-workspace --all-features run the lane never recorded (left as the named honest gap, not re-run here — out of the round's ownership). WHAT BREAKS — nothing new; a manifest setting seeks_per_reopen > the default changes only the storm's epoch shape. WHO HITS IT FIRST — the first nightly author wanting a stormier profile (the schema + loader now take the key).
- w24/m pushed after this entry; main untouched; w24/b not modified — the fixes ride the m lane for the Lead's merge decision.

---

Task ID: 7.w24n
Agent: w24-n
Task: W24-N small fixer lane — close review scout R5's two actionable findings on w24/g (R5-2 missing worklog entry; R5-3 tautological placement.screen assert) on branch w24/n.

Work Log:
- R5-3 (crates/settings-schema/tests/integration.rs, commit a9258aa): replaced the tautological screens.len() == entries.len() assert in seed_shape_is_what_every_committed_artifact_self_reports (screens is collected 1:1 from entries via map(screen_of), so it always held — a row with a missing screen contributed "" and passed; the drift only surfaced one step later as a confusing G1-header/G2-row shape mismatch) with per-row validation at collection time: each row's placement.screen must be non-empty AND present in the seed's [areas] known registry, failing with the row index + setting key. The later G1/G2 asserts left intact.
- R5-3 negative-path proof (scoped run, seed restored after): deleting screen from engine.log-level → panics at the new assert: "every [[setting]] row must name a placement.screen — row #22 (`engine.log-level`) does not"; renaming it to unregistered "enginex" → "placement.screen `enginex` of `engine.log-level` (row #22) is not in the seed's [areas] known registry". Both fire at the strengthened assert with row-accurate messages instead of a downstream shape mismatch.
- R5-2 (this commit): appended the g lane's missing AGENTS.md §4 entry (7.w24g, the entry above) — truthful retroactive summary of its 7 commits with verification credited to R5's independent audit (docs/reviews/w24-g-fixes-findings.md on w24/r5), per the Lead's W24-N assignment.
- Verification (all from the lane-isolated CARGO_TARGET_DIR=target-w24n, binary provenance per R5-1's lesson; dir gitignored by the target-w*/ rule and deleted after): cargo test -p settings-schema = 43 lib + 8 integration passed / 0 failed; cargo clippy -p settings-schema --all-targets -- -D warnings clean; cargo fmt -p settings-schema -- --check clean; cargo run -p xtask --locked -- lint-literals PASS (0 violations, 452 allowlisted).

Stage Summary:
- R5-2 and R5-3 both closed on w24/n; no test weakened — the shape guard's diagnostics strictly improved, all suites green from provenance-clean binaries. Ownership held: worklog.md (append-only, two entries) + the one test file.
- D-10: MEASURE — 51/51 settings-schema tests green (43 lib + 8 integration), clippy -D warnings + fmt clean, lint-literals 0 violations, both negative-path failure messages captured verbatim from real runs. NOT TESTED — nothing outside settings-schema scope (no source beyond the one test file changed; the worklog is prose; R5's own broader battery stands as the lane-g verification). WHAT BREAKS — nothing new; a seed edit that drops placement.screen or invents a screen name now fails earlier with a row-accurate message instead of a misleading G1/G2 mismatch. WHO HITS IT FIRST — the next seed-growing commit (extensions of the extensions pattern).

---

Task ID: W24-P
Agent: W24-P (compat review-fix lane, issue #53, branch w24/p off w24/d). Entry appended 2026-11-11 by W24-X (wave-24 lane X, small fixer, branch w24/x) per W24-R14 finding R14-1 — the lane's 11 commits shipped without the mandatory §4 append; content below is reconstructed from the commit messages and cross-checked against R14's final-state audit (docs/reviews/w24-p-final-findings.md on w24/r14 — that audit's per-finding tables are the source for the fixed-vs-deferred accounting and every verification number below).
Task: The review-fix pass over the compat matrix (issue #53): W24-L findings P1-1, P2-1, P2-2, P3-2…P3-5, P3-7, P3-10; the IN-8 aac-lc evidence double-append (both halves); and W24-R10's R10-1/R10-2/R10-3 — 11 commits on w24/p.

Work Log:
- a776c24: the W24-D worklog entry (W24-L finding P1-1) — the mandatory §4 append reconstructed from the 9 commit messages, cross-checked against the W24-L audit.
- 16d64b0: P2-1 — in-row refusals surfaced in the native container cells (renderer + the committed md/JSON); the dead refusal_count removed with it (P3-3's second limb).
- 022d679: P2-2 — the web-wasm cells derived from the recorded step outcomes instead of asserted.
- 74bda46: P3-2 / R10-2 — unit tests for the verdict/config logic: 10 oracle (prefix_verdict ×4, f32_verdict ×3 with power-of-two offsets, committed_record_applies, compare_row_committed integer + f32-echo) + 5 config (schema refusal, web-as-native refusal, two native lanes, structural rejections, minimal-valid control); the applicability rule factored into oracle::committed_record_applies with unchanged semantics.
- 2a43b78: P3-3 — the transition bullet's platform rendered from the outbox row (transition.platform), not a hard-coded "(linux)".
- 8f72b30: P3-4 — the doubled "VLC" dropped from the oracle source line (renderer format string + the committed md header line, provenance recorded).
- b30a925: P3-5 — a VlcSource::None variant for rows where no VLC side was consulted ("none" in the md table); no committed artifact affected (all 35 oracle rows are live_run).
- cd7d5c5: P3-7 — the 0385 regen gap recorded as documented debt: the manifest purpose and report §7 name the machine-local concat intermediate and the re-authoring next step, deferring to the corpus owner; argvs/sha256/deterministic untouched, manifest still parses (67 entries).
- b5aff90: IN-8 / R10-1 limb 1 — the applier's evidence append is idempotent: within a coalesced group each DISTINCT citation appends once, and a citation already a "; "-segment of the row's current evidence (unescaped via the new string_value_of parse) is never re-appended; two unit tests.
- 6a1677c: IN-8 / R10-1 limb 2 — the duplicated aac-lc evidence segment removed from feature-matrix.toml with a 7-line provenance comment (finding/cause/fix-commit/untouched fields); statuses/notes untouched.
- c48dc76: R10-3 — the steps-lane failure detail is a fourth steps_cell parameter (both production call sites pass their own wording, the format_cell clause byte-identical to the old text); new test steps_failure_detail_is_parameterized_not_wasm_prose.

Stage Summary:
- Fixed (R14-verified on the pushed tip; fix commit in parens): P1-1 the W24-D worklog entry (a776c24); P2-1 in-row refusals (16d64b0); P2-2 derived wasm cells (022d679); P3-2 / R10-2 oracle+config unit tests (74bda46, plus b30a925's echo tests); P3-3 platform from the outbox row and the dead refusal_count gone (2a43b78, 16d64b0); P3-4 "VLC VLC" (8f72b30); P3-5 VlcSource::None (b30a925); P3-10 hand-edit provenance beyond the ask (the provenance blockquote + JSON editorial note, extended by each editorial commit); IN-8 both halves — applier idempotence guard (b5aff90) + committed-data dedupe (6a1677c); R10-3 parameterized steps-cell (c48dc76). P3-7 (the 0385 regen entry) closed as documented debt, not a fix (cd7d5c5).
- Deferred — open, untouched on this lane (each re-verified untouched at the tip by R14; recorded here per R14-1 so the deferral is no longer unrecorded): P3-1 — verify_diff defeats notes-insertion (the notes_line: None arm inserts a notes key, tools/xtask/src/cmd/compat_matrix.rs:671-682, while verify_diff rejects any key-count change, :793-798); P3-6 — the first-audio limb dump-vs-fatal (tests/compat/src/decode.rs untouched in the range); P3-8 — the stale claims snapshot (tests/compat/compat-matrix.json untouched in the range); P3-9 — read_capped's unbounded read (tests/compat/src/proc.rs untouched in the range). All four remain open for the compat-matrix owner. Known bound on the new IN-8 guard (R14-2, P3, not merge-blocking): the dedupe compares whole "; "-separated segments, so a citation containing "; " or a reworded variant would re-append — theoretical on the current path-like citations.
- Verification (R14's runs on the pushed tip origin/w24/p @ c48dc76 — cited from the audit, not re-run by W24-X): tests/compat cargo test 27 passed / 0 failed (4 sha256 + 10 oracle + 5 config + 8 report), clippy --all-targets --locked -D warnings = 0 warnings; xtask cargo test -p xtask compat_matrix 12 passed / 0 failed (incl. the two IN-8 tests), clippy = 0 warnings; fmt --check clean in both; lint-literals PASS (0 violations, 452 allowlisted); matrix counts unchanged from w24/d — files_total 166 = 84 decoded + 22 decoded_partial + 6 refused (2 refused_probe + 4 refused_demux) + 0 demux_fatal + 54 malformed_typed; oracle 29 bit_exact_prefix + 6 tolerance_fail; raw-rows blob identical on d and p; tests additions-only; scope = the 9 findings-named files, +1,114/−55.
- Reconciliation of the W24-D entry's closing line (per §4's never-edit rule that text stays untouched; this entry is the correction): it said the residuals (P2: in-row refusal surfacing, wasm-cell derivation; P3: unit-test coverage, wording/labeling) "are being fixed by W24-P on w24/p". What actually landed: both P2s fixed; "unit-test coverage" = P3-2 fixed; "wording/labeling" = P3-3/P3-4/P3-5 fixed; additionally IN-8 fixed in both halves, P3-7 documented as debt, P3-10 provenance maintained beyond the ask; P3-1/P3-6/P3-8/P3-9 deferred (above), not fixed.
- Commits: 11 on w24/d..w24/p (a776c24 → c48dc76), pushed to origin/w24/p; main untouched. R14's bottom line after this append: the lane is merge-ready from that audit's perspective (R14-2 remains a theoretical robustness note).


---

Task ID: W24-Q
Agent: w24-q (wave 24 lane Q — packaging P4 fixer; branch w24/q, base origin/w24/h @ 45c7136)
Task: Fix W24-R6's 4 new-P4 observations on the W24-H packaging lane (docs/reviews/w24-h-fixes-findings.md §"New observations"): wiring-guard future-proofing, .wxs doc-basename XML escaping, runbook run-block count, PACKAGING_DOCS whitespace constraint.

Work Log:
- 4 commits on w24/q: a6626fa P4-1; f1d45c8 P4-2; b0901a0 P4-3; 35b3b7e P4-4; 5c9795b P4-2 runbook follow-through. Files: .github/workflows/release.yml (4 guard steps + header comment), scripts/release/gen-wxs.sh, docs/packaging/README.md, packaging/packaging.toml + packaging/packaging.schema.json (comments/hint only) — exactly the findings-named set, no .rs files (fmt/clippy not owed).
- P4-1 (guards enumerated only the 8 known names): all four fail-loud wiring guards (windows MSI step, both macOS dmg steps, release-job visibility step) now DERIVE their expected secret-name set from packaging/packaging.toml at guard time — an inline python (tomllib, same interpreter the parse step already requires) prints one PACKAGING_SIGNING_<SECTION>_<KEY>_ENV bridge var per [signing.*] key ending in _env; the loop then checks bridge-exported AND mapped-in-env:. New second branch: a TOML key with no exported bridge var gets an explicit out-of-sync ::error:: instead of a bare indirect-expansion failure. Executed the exact YAML-delivered run blocks for proof: happy path passes; unmapped secret -> wiring-gap ::error:: + rc 1; a synthetic 9th [signing.windows] key (foo_env) placed in a TOML copy -> ::error:: naming $WINDOWS_SIGNING_FUTURE_FOO + rc 1 (the P4-1 scenario, previously invisible); bridge drift -> sync ::error::; visibility step still warns (rc 0) when all 8 mapped credentials are empty (honest-unsigned semantics unchanged).
- P4-2 (doc basenames unescaped in File/@Source): xml_escape extended to also escape ' as &apos; (full set & < > " '), File/@Source now carries xml_escape(basename). Regression test = new `gen-wxs.sh --self-test` mode: runs the REAL generation body (refactored into generate_wxs — no copied logic) with hostile identity + docs "a&b<c>\"d\".md it's.md", then proves via python xml.etree that the output PARSES, the hostile basenames round-trip, File/@Ids stay sanitized + unique, Package/@Name round-trips. Proofs: self-test PASS; pre-fix script (origin/w24/h) under the same fixtures exits 0 but ET.parse dies "not well-formed (invalid token): line 23" (bug reproduced exactly as W24-R6 described); output for the REAL config (LICENSE+README.md via the bridge) byte-identical old-vs-new; duplicate-basename/empty-list/missing-env refusals unchanged (same messages, rc 1/2).
- P4-3: runbook said "bash -n over all 41 steps" — re-counted myself (yaml.safe_load; windows 8 + macos-arm64 6 + macos-x64 6 + linux-x64 7 + linux-arm64 7 + release 8 = 42) and re-ran per-run-block bash -n with ${{ }} neutralized: 42/42. Count fixed with provenance note.
- P4-4 (space-separated PACKAGING_DOCS can't represent spaced paths): enforcement already existed (schema pattern ^[A-Za-z0-9][A-Za-z0-9._/-]*$ excludes all whitespace) but the die message was an opaque regex mismatch. Added x-lumen-hint on shipped.docs items (shown in packaging-env.py's error: names PACKAGING_DOCS, the word-split hazard, "rename the file", runbook pointer), extended the schema $comment (why the charset excludes whitespace; validation time = enforcement point; richer list format = deliberate non-goal), documented the constraint in packaging.toml [shipped] comment + runbook config-planes section. Negative tests with a space ('my doc.md') and a tab doc path: packaging-env.py rc 1 with the new actionable message; positive path unchanged (51 vars).
- P4-2 follow-through (honesty): runbook claimed ".wxs generation + env asserts run in gate.sh" — gate.sh never called gen-wxs.sh on this branch. Corrected to the true (now stronger) claim: gen-wxs carries its own executable self-test; wix build stays CI-only.
- Verification battery (ACTUAL, at HEAD 5c9795b): bash -n + sh -n on all 14 scripts/packaging/*.sh + gen-wxs.sh clean; yaml.safe_load release.yml OK; 42/42 run-block bash -n; schema JSON valid; gen-wxs.sh --self-test PASS; `source /home/z/my-project/scripts/cargo-env.sh && bash scripts/packaging/gate.sh` PASS (51 vars, stagers, both unsigned-fallback contracts); cargo run -p xtask --locked -- lint-literals PASS (0 violations, 452 allowlisted).

Stage Summary:
- All 4 W24-R6 P4s fixed on w24/q and pushed; each with a negative-path or round-trip proof executed for real (not asserted). The wiring guards are now closed under config growth (any new [signing.*] *_env key fails loudly until mapped), gen-wxs is closed under hostile filenames (self-test would have caught the original bug), the runbook count is current, and the docs-list whitespace contract is documented at all three sites (schema hint shown in the error, schema $comment, packaging.toml comment, runbook) with no new list format.
- Known follow-ups handed to the Lead (outside this lane's file ownership): (1) build-msi.ps1 has the same unescaped $base in its mirrored File/@Source rows (W24-R6 noted "same property in the .ps1 twin") — one-line XmlEscape + quoting change when a lane owns that file; (2) wiring `gen-wxs.sh --self-test` into gate.sh step 5/6 would make the runbook's local-proof story fully gate-enforced (gate.sh is not in this lane's ownership).
- D-10: MEASURE — 4 fix commits, 5 files, +289/-100; guard behavior proven on 5 scenarios (incl. the synthetic 9th-secret case); self-test red/green vs pre-fix script; gate + lint-literals + 42/42 run-block parse green. NOT TESTED — anything inside GitHub Actions (workflow never executed, billing blocked, repo-wide standing caveat): the guard's tomllib parse on actual runners [VERIFY at first live run], wix build on the escaped Source values. WHAT BREAKS — a future [signing.*] secret without a release.yml env: mapping now fails the job loudly at the guard (intended); a doc path with whitespace now fails at the parse step with the rename message (intended). WHO HITS IT FIRST — whoever adds the 9th signing secret or a spaced doc path (both get an error naming the exact fix).

---

Task ID: W24-R2 (wave 24 review scout 2 — READ-ONLY docs-lane auditor)
Agent: w24-r2
Task: Audit origin/w24/e (issue #58 release-docs lane) for honesty and rule violations. Read-only; sole deliverable docs/reviews/w24-e-docs-findings.md on branch w24/r2.

Work Log:
- Fresh worktree wt/w24r2 @ 3596918 (w24/e tip, base c339ee8 = origin/main). 26 verification groups re-run (findings file §"What was sampled"): product-env.py --print (exact match incl. exit 0); two-track versioning confirmed in help.rs:101 + Cargo.tomls; release.yml YAML parse + 6 jobs/runners/continue-on-error/needs; all four xtask gates re-run via the cached binary — lint-literals 0/452, check-identity 0/453, sbom no-drift 157, config-validator 4 (all brand assets) — byte-for-byte matches; full_gate 9 steps (no config-validator) confirmed; matrix recompute 73/37/0 + 14/6/9/8 reproduced via tomllib; 53 format records; 8 matrix-row statuses vs records; 150 keymap bindings + 17 spot-checks; 14 areas / 21 ui / 32 settings / 5 crash keys; product.toml defaults; fixture 425516 bytes = BENCH example; CLI enum vs guide verbs; BENCH/state-changed/engine-info formats vs rc-smoke parsers; rc-smoke stub/mock audit (real, exit-gated, BLOCKED=fail, env-driven — no fakes found); F-1/F-2 probes (main 2 markers @15/100, w24/a 0 + 70ed69c/771259c exist, origin/w24/a absent); playwright 1.64.1 vs chromium-1200; overclaim greps zero live hits; 0 tags; phase6-exit PENDING; ADR register 12 files/11 rows + statuses verbatim + vaapi drift commits real; release-notes dry-run exit 0 (-20 cap; -50 first-release rule); RELEASE.md known-limit 1/2/5/6 numbering; PRODUCT_* env (no literals); proper per-file bash -n battery all green; routes/bindings.ts/npm scripts exist; no TODO markers; English throughout.
- Findings: 0 P0, 0 P1, 3 P2 (audit §1.1 video-codec claimed-rows table lists blocked h264-software instead of implemented vp9; container-avi.md:68 + audio-pcm.md:58 cite never-committed docs/audit/rc-smoke-2026-11-10.md; release-checklist:51 bash -n multi-file command only checks the first of 8 scripts — conclusion independently re-verified true), 6 P3 (rg -c setting 33-vs-32 recording; "8 scripts" vs 7; streaming-dash 191-lib-tests stale by 13; GNU-only stat -c in rc-smoke check 6; w24/c now pushed; single consolidated worklog entry). None invalidate the lane's all-green docs verdict.
- Not re-run: cargo-test gate rows inside format records + fresh rc-smoke execution (disk 97%/ENOSPC — matches the lane's own recorded limitation); "10 vCPU" environment claim unverifiable from this container (2 cores).
- Output: docs/reviews/w24-e-docs-findings.md committed on w24/r2, pushed. No code touched; no other lanes modified.

Stage Summary:
- W24-E's docs are substantially honest: every load-bearing claim I could re-run reproduced exactly; the 3 P2s are accuracy defects inside the audit/checklist documents themselves (wrong row classification, dangling evidence pointer, defective verification command), each with a one-line fix, none changing the 8.G1 GREEN verdict or the 8.G2 runbook's conclusions.

---

Task ID: 7.w24r5-r2
Agent: w24-r5
Task: W24-R5 r2 (resume, review scout 5 round 2) — audit the gate-findings fixer lane (git diff origin/main...origin/w24/g) against phase7-exit.md OF-2..OF-5, run the previously-failing tests + battery, SHIP w24/r5.

Work Log:
- Scope: origin/w24/g @ 8121cbf == origin/main c339ee8 + 7 commits (OF-2 1c48902, OF-3 64757b1, OF-4 ba9659f, OF-5 de9b341, fmt 9b15003, gitignore 56b59c1, clippy 8121cbf). No source writes by this lane; only docs/reviews/w24-g-fixes-findings.md + this entry.
- OF-2 verified FIXED: all three pin sites (settings-schema/src/lib.rs:155-185, tests/integration.rs:117-141, apps/cli/src/help.rs:303-375) now DERIVE counts from schemas/settings.toml via independent raw-toml::Table reads — zero literals in assertions (27/32/6/7 appear only in comments). help.rs test renamed areas_block_lists_the_six_areas -> areas_block_lists_every_screen_the_seed_declares and upgraded to FULL Areas-block string equality (screens, per-screen counts, engine-internal annotation mirroring emit.rs:291-302) — strictly stronger than the old name-vec + 2 spot-checks.
- Shape guard traced REAL: seed_shape_is_what_every_committed_artifact_self_reports cross-checks seed shape (32 entries/7 screens/14 areas) vs G1 header "// 32 entries across 7 screen(s) (14 known areas)." and every G2 per-screen row (6+5+1+7+3+6+4=32) — a 27/6-stale artifact fails both with regen commands named. Residual (accepted design): UI schema.ts stays guarded by npm settings:check + vitest, not cargo.
- OF-3 verified FIXED: schema.ts diff is pure addition of the 5 extensions.crash-* constants + 5 MSGIDS, correctly excluded from SETTINGS_ENTRIES (ui=false engine-internal rows; 21 ui=true count matches). npm run settings:check PASS (byte-compares schema.ts AND Screens.tsx vs fresh regeneration; "5 ui areas, 21 ui entries, 32 total", exit 0) — run without node_modules symlink (zero-dep node script; sandbox blocks symlinks anyway).
- OF-4 verified FIXED + NOT WEAKENED: repo_root() now env!("CARGO_MANIFEST_DIR")/../.. (compile-time worktree-correct; the gate doc's [PROPOSAL]) and run_rebrand pins .current_dir(repo_root()) matching the tool's cwd-based root discovery (rebrand.rs:718-719) — hermetic. reverse_control_real_product_toml_is_a_noop assertions unchanged: real product.toml, exit 0 + "result: no-op", byte-compare of all 5 surfaces — now against the same tree the tool read. The 3 #[ignore] boot-tier placeholders pre-exist identically on origin/main. No #[ignore] added anywhere in the diff; removed asserts = exactly the 4 stale pins, all replaced by equal-or-stronger derivations.
- OF-5 verified FIXED: matrix note now says VT landed (compile-verified, runtime [VERIFY]), VA-API landed (compile-verified + failure-path runtime-tested, activation [PROPOSAL] ADR-013), "Media Foundation (Windows) NOT IMPLEMENTED — no MF lane exists", retraction attributed to hw/mod.rs truth table + gate OF-5 provenance. Code reality re-grepped: hw/ has only vaapi/ + videotoolbox/; remaining MediaFoundation hits are the HwBackend capability-tag enum (negotiation vocabulary answering the typed required/no-bridge truth) and NOT-IMPLEMENTED doc comments. Note == code.
- Incident (R5-1, P2 infra): the shared-target binary clobber hit THIS audit twice — first lumen-cli --lib run executed a stale binary compiled from wt/w24d (old test name, pin 27, FAILED); xtask --test cli executed one from wt/w24l (pre-OF-4 code, "workspace root not found" panic). Remedies: cargo clean -p lumen-cli + immediate rerun (125/0, dep-info = my worktree) and a lane-isolated CARGO_TARGET_DIR=target-xtask for xtask/settings-schema/lumen-cli final runs (12/0/3, 43/0+8/0, 125/0 — all provenance-verified via the executed binary's .d CARGO_MANIFEST_DIR). Proves OF-4's disease is live and g's isolated-dir + gitignore pattern (56b59c1) is the right mitigation; [PROPOSAL] per-lane target dirs in cargo-env.sh. Isolated cache (1.4G) deleted after verification to relieve the rootfs (59% now).
- Battery on w24/r5 (all provenance-clean): fmt --check PASS; clippy -p settings-schema -p lumen-cli -p xtask --all-targets -D warnings PASS; the 4 previously-failing suites all green (settings-schema lib 43/0, integration 8/0 incl. the new guard, lumen-cli lib 125/0, xtask cli 12/0/3); lint-literals PASS 0 violations/452 allowlisted; check-architecture PASS 5/5; npm settings:check PASS.
- Scope discipline: diff = 7 files, 6 finding-related + .gitignore chore (itself finding-related infra per the incident above). Gap recorded (R5-2, P3): the g lane never appended its AGENTS.md §4 worklog entry. Full report: docs/reviews/w24-g-fixes-findings.md (incl. findings register R5-1..R5-4).

Stage Summary:
- VERDICT: OF-2/OF-3/OF-4/OF-5 all FIXED PROPERLY, none weakened — SHIP w24/r5 @ 8121cbf toward wave-24 integration; re-run the phase-7 §8 battery at merge (OF-1 closes via lane A; App.tsx still carries 2 markers on main as expected).
- D-10: MEASURE — 4/4 previously-failing tests green from provenance-verified binaries; fmt/clippy -D warnings clean on all 3 touched crates; lint-literals 0 violations; settings:check byte-exact. NOT TESTED — the UI vitest suite (node_modules symlink blocked in this sandbox; settings:check itself is the drift gate and passes); UI-side typecheck/lint (OF-1 still open on main, lane A owns). WHAT BREAKS — future seed growth without regen now fails schema-gen --check AND the new shape guard AND the derived pins coherently (cannot be stranded stale); the tautological screen-name assert (R5-3) mislabels a missing-placement.screen as a G1/G2 mismatch message. WHO HITS IT FIRST — the next seed-growing commit (extensions of the extensions pattern) and any reviewer trusting unprovenanced shared-dir test binaries (R5-1).

---

Task ID: W24-R7 (review scout 7 — dual-scope read-only audit)
Agent: w24/r7
Task: Audit origin/w24/i (W24-R2 fix lane, primary) + origin/w24/b soak-nightly.yml (secondary). Read-only; findings in docs/reviews/w24-i-fixes-findings.md.

Work Log:
- Worktree wt/w24r7 off origin/w24/i @ ca42190. Diff origin/w24/e...origin/w24/i = 7 commits (task card said 8 — description discrepancy, no lane fault), 7 files, all findings-scoped + worklog.
- Read R2's findings doc from origin/w24/r2 for per-fix dispositions.
- P2-1 verified by own tomllib parse of feature-matrix.toml: claimed video rows = av1/h264/hevc/mjpeg/prores-dnxhd/vp9, h264-software blocked x4; matrix recompute 73/37/0 (14/6/9/8, both counting rules); all 7 record files exist; getting-started agrees.
- P2-2 verified: phantom rc-smoke-2026-11-10 filename → zero hits outside worklog's before-narrative; ff001e5 commit message records the 60/60+60/60 run; rc-smoke check 4 IS headless-decode on the avi_mjpeg_pcm.avi default fixture; checklist §2 numbering matches.
- P2-3 verified: loop present in BOTH places; loop re-run → all 8 sh pass, exit 0; pitfall re-demonstrated (one-shot bash -n with bad 2nd file → exit 0). 7 release scripts confirmed.
- P3-4/P3-5/P3-6/P3-7/P3-8 all independently re-verified (33-vs-32 anchored counts, 27/206/193 test counts @ 93b7614 via git grep, wc-vs-stat 9339==9339 + fake-wasm PASS simulation + missing-file path, ls-remote w24/c=fb97822 + ls-tree drivers + not-merged-into-main + w24/a absent).
- P3-9 deferral matches R2's own "none required" disposition — accepted.
- Gates re-run: lint-literals PASS 0/452 (matches worklog claim); node --check OK; literal py_compile now OK too (disk recovered to 54% — moots the ENOSPC note).
- NEW-1 (P2): checklist row 1.2's `bash -n "$f" || exit 1` inside a GFM TABLE cell uses raw pipes → cell splits early; proven with markdown-it table render: command truncated at "$f", Status cell renders EMPTY, trailing cells dropped. Fix: escape as `\|\|` (verified renders correctly). 2-character change.
- NEW-2 (P3, pre-existing, surfaced by verification): video-vp8-vp9.md + video-prores-dnxhd.md status front-matter stale vs the 2026-11-09 issue-#7 admission flips (vp9 implemented x3, prores partial x3) — the fixed §1.1 records cell leans on both; suggest dated addenda.
- TARGET 2 (origin/w24/b soak-nightly.yml): yaml.safe_load valid (3 jobs) + strict duplicate-key parse clean; diff vs main = additions only (0 deleted lines — soak job untouched, no renamed/deleted steps); schedule+dispatch-only trigger shape unchanged, no continue-on-error, [PROPOSAL]/[VERIFY]/NEVER-EXECUTED/R-16 billing-blocked labels present and correct (R-16 verified); all referenced files exist on w24/b (scenarios-nightly.toml = 5x48min = 240min = 4h exactly, wrapper bash -n OK, env contract SOAK_SCENARIOS/SOAK_SCENARIOS_REPORT matches main.rs, 4 fixtures, schema json, soak report). PASS, no findings.
- Wrote docs/reviews/w24-i-fixes-findings.md; commit + push origin w24/r7. No other writes (read-only audit).

Stage Summary:
- Target 1 verdict: all 3 P2s + 5 fixable P3s correctly fixed and independently re-verified; 1 NEW P2 (GFM pipe escaping in row 1.2 — renders truncated command + empty status) + 1 pre-existing P3 observation (stale record front-matter). P3-9 deferral accepted. No overclaims, no ownership violations, per-commit scoping verified.
- Target 2 verdict: PASS on all four checks (YAML valid, purely additive, conditional/billing labels correct, references exist).
- D-10: MEASURE — everything in the findings doc's verification-runs block (loop 8/8, 33/32, 27/206/193, 9339==9339, 73/37/0, 150/5, lint-literals 0/452, ls-remote/ls-tree, markdown-it renders). NOT TESTED — actionlint (not installed in sandbox; strict YAML parse + manual schema checks instead), actual rc-smoke execution, real macOS. WHO HITS NEW-1 FIRST — anyone reading the rendered release-checklist on GitHub (Lead at wave merge); fix is a 2-character edit before merge.

---

Task ID: W24-R8 (wave 24 review scout — audit lane A's committed work, read-only)
Agent: w24-r8
Task: Audit lane A (browser matrix, issue #49, worktree wt/w24a — never entered) from a separate worktree: P0 App.tsx merge-marker fix 70ed69c, settings regen 771259c, harness stack ff6e6aa+231598f+d1cba99+9a13c39. Write docs/reviews/w24-a-web-findings.md, push w24/r8.

Work Log:
- Reachability: origin/w24/a does NOT exist (fetch → "couldn't find remote ref"); local w24/a @ 9a13c39 IS reachable — audited the full committed delta c339ee8..9a13c39 (6 commits, +1803/−10, 15 files) from worktree wt/w24r8 (branch w24/r8 based at 9a13c39). Lane A still running: uncommitted work not audited.
- P0 fix 70ed69c VERIFIED CLOSED: App.tsx 0 merge markers (repo-wide ui/src scan clean), /discovery (App.tsx:97) + /convert (App.tsx:98) both preserved with imports (merge 45c5c6f intent); tsc --noEmit exit 0; eslint 0 errors (28 warnings, all pre-existing files untouched by the lane); npm run build exit 0; vitest 718/718/80 files — exactly the commit's coherence claim.
- Settings regen 771259c VERIFIED: settings:check OK (5 ui areas, 21 ui entries, 32 total); toml cross-check 32 [[setting]] / 14 known areas / 7 screens-with-entries; the 5 extensions.crash-* rows match toml values (spot-checked initial-ms: 5000, 0-600000, extensions/crash-isolation/order 10).
- Harness verdict SOUND: matrix.config.json single knob source with loud shape validation (mjs:43-99); runtime facts env-injected; real Worker/wasm/fetch in-page suite (harness.js 735 lines, no stubs/TODOs); BCD lanes honestly "not-runnable-in-sandbox"; fragments merged only when actually written; PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 with env→config→pinned executable resolution recorded into tooling block; playwright.web.config.ts fails fast outside the driver (verified live both with and without env); smoke lane still lists 38 tests/3 files with e2e/web excluded.
- d1cba99 server-lifecycle fix correct: async spawn replaces spawnSync (event-loop starvation → net::ERR_ABORTED root cause documented in-code); exit code propagated.
- 9a13c39 wasm32 throttle fix sound: cfg-split default_throttle_source — native InstantSource unchanged, wasm32 static ZeroMonotonic (std Instant::now panics on wasm32; SessionConfig::new builds the default transiently before host injection); Arc<dyn MonotonicSource> field coerces both arms; no other reachable InstantSource on the wasm path; regression test the_default_policy_is_constructible_and_readable passes.
- Rust strictness on lane A's only Rust file (playback-engine/src/events.rs): fmt --check CLEAN; clippy --all-targets CLEAN; clippy -p lumen-web --target wasm32-unknown-unknown CLEAN; cargo check wasm32 OK; xtask lint-literals PASS 0 violations.
- Scope discipline PASS: 15 files all within ownership (ui/, docs/web, playback-engine events.rs = the sanctioned wasm throttle fix). feature-matrix web column / xtask dispatch / full_gate step 10 / ci.yml / worklog NOT yet in the committed delta (lane still running — closer must check they land).
- Findings written to docs/reviews/w24-a-web-findings.md: 0×P0, 0×P1, 2×P2 (empty chromium lane in the committed matrix record + stale git stamp — final run evidence pending; report.markdownName dead knob — validated but never read), 5×P3 (HARNESS_TIMEOUT_MS literal harness.js:33; timeout 240_000 literal playwright.web.config.ts:44; spawn-error path skips server.close() mjs:342-349; parseRange swallows multi-range specs mjs:177-202; worklog entry not yet committed), plus environment notes.
- NOT TESTED: the browser matrix end-to-end — the recorded chromium-1200 executable no longer exists in this sandbox (~/.cache/ms-playwright holds only ffmpeg-1011), so the run could not be reproduced; verdicts rest on code review + committed record + live-run failure narratives in commit messages.
- Commits on w24/r8: findings doc + this worklog entry. w24/a untouched; wt/w24a never entered.

Stage Summary:
- Lane A audit complete: P0 verified fixed, settings regen in sync, harness real and honest, Rust green on native+wasm32, scope clean. The lane's finish-line items (final passing-run record, markdownName decision, gate/CI/feature-matrix wiring, worklog) are tracked as P2/P3 in docs/reviews/w24-a-web-findings.md for the wave-24 closer.
- D-10: MEASURE — 6 commits audited (+1803/−10); full verification battery ACTUAL (tsc 0, eslint 0 errors, build 0, vitest 718/718, settings:check OK, fmt/clippy/lint-literals clean, wasm32 check+clippy clean, 1 regression test green, 2 live Playwright fail-fast probes). NOT TESTED — browser-matrix end-to-end re-run (no chromium binary in this sandbox). WHAT BREAKS — nothing in committed code; the risk is process-level (lane closing without the final evidence record). WHO HITS IT FIRST — the wave-24 merge closer reading docs/reviews/w24-a-web-findings.md.

---

Task ID: W24-R9 (wave 24 review scout — INTEGRATION audit of the rehearsal tree)
Agent: w24/r9
Task: Read-only cross-lane-seam audit of w24-integration-rehearsal @ 286bacd (17 merges: lanes b–k + r1–r7), diff origin/main...rehearsal. Findings in docs/reviews/w24-integration-findings.md.

Work Log:
- Worktree wt/w24r9, branch w24/r9 off the rehearsal tip at setup (286bacd). NOTE: the rehearsal moved during the audit to 3f2b96c (+ merges of w24/l, w24/n, w24/r8 = 20); every finding re-verified against the live tip, deltas recorded.
- Seam 1 (feature-matrix): tomllib parse 252 rows; lane D's 5 linux flips == 6-line applied-transitions ledger (aac-lc hit twice); linux implemented 64→59 / partial 46→51; lane G's OF-5 note verbatim; integrated 34-line diff decomposes exactly into d(32)+g(4) — zero hand-edits outside ledger+OF-5. Web column pass: absent everywhere (lane A never touched the matrix; web partial=4 on main and integrated).
- Seam 2 (full_gate): bash -n PASS; steps 1–9 then 11 — step 10 does not exist; comment reserves it for lane B which never landed it (empty diff on w24/b); "web-budget" = 0 hits repo-wide/all refs. Ran scripts/packaging/gate.sh live: exit 0 PASS.
- Seam 3 (workflows): yaml.safe_load all 6; no dup job ids/names; action pins byte-identical across files (checkout 3d3c42e, toolchain d124f86, rust-cache 6323deb, upload cf430e0, download 9000827); cargo-deny pinned once (ci.yml 0.20.2 SHA-256) + prose-consistent in release.yml; ci.yml identical to main (no budget gate).
- Seam 4 (worklog): every rehearsal merge's worklog diff = additions only (0 deletions, 17/17 merges); 10 root entries each exactly once in merge order; pure append vs main. Coverage: lanes B/D/G entry-less (fixes on unmerged w24/m + w24/p; 7.w24g landed at tip via w24/n); R1/R3/R4/R6 used docs/worklog side files (AGENTS.md §4 deviation).
- Seam 5 (collisions): docs/reviews 7 distinct at snapshot (9 at tip); .gitignore union sane but target-xtask/ duplicated (lanes d+g); tests/compat standalone workspace, root Cargo.lock/toml unchanged (blob-identical).
- Seam 6 (xrefs): 238 paths across 76 wave docs resolved; exceptions enumerated (browser-matrix.md name drift, regression-guards.md R4 P2-4, lane-a paths at snapshot only); rc-smoke phantom fix verified closed; release-checklist script refs all exist.
- Seam 7 (greps): merge markers only in ui/src/App.tsx (pre-existing OF-1 from main; 0 markers at live tip); TODO/FIXME/unimplemented!/todo! diff-scoped: zero new in non-test code (2 pre-existing allowlisted hits unchanged).
- Wrote docs/reviews/w24-integration-findings.md (1 P0 / 3 P1 / 3 P2 / 3 P3); commit + push origin w24/r9. No other writes.

Stage Summary:
- Verdict: NOT READY at 286bacd (UI un-compilable OF-1; step-10 hole; lanes B/D/G worklog gaps; w24/m + w24/p unmerged); live tip 3f2b96c closes OF-1 and adds the a/d review reports + 4 more worklog entries, but still misses w24/m, w24/p, lane-A tip 389bfcc, step 10, and the web-column/budget-gate deliverables (which exist on no branch — wave-plan vs reality mismatch flagged to the Lead). Merge machinery itself introduced zero regressions: every defect is inherited from a lane's own state or from lanes still outside the rehearsal.
- D-10: MEASURE — tomllib/yaml parses, per-merge worklog add/del counts, diff decomposition d(32)+g(4)=integrated(34), live gate.sh run (exit 0), 238-path resolution sweep, marker/TODO greps with main-baseline comparison. NOT TESTED — full UI typecheck/vitest on the snapshot (markers = guaranteed TS parse failure, per gate log), actionlint, 3-OS runners. WHO HITS IT FIRST — the Lead at wave-24 merge: run the phase7-exit §8 battery + UI suite on the post-m/p/389bfcc rehearsal.

---

Task ID: W24-R10
Agent: W24-R10 (wave 24 review scout, round 10 — audit of the compat-fix lane W24-P)
Task: Read-only audit of lane P's fixes for W24-L's findings on the compat-matrix lane (w24/d, issue #53) — findings doc docs/reviews/w24-p-fixes-findings.md.

Work Log:
- Branch state: `origin/w24/p` does NOT exist (fetch fails; lane unpushed at audit time) — audited the LOCAL ref `w24/p` @ 022d679 (3 commits on origin/w24/d: a776c24 worklog/P1-1, 16d64b0 refusals/P2-1, 022d679 wasm cells/P2-2) via a fresh worktree wt/w24r10 (branch w24/r10); never entered lane P's live worktree wt/w24p (snapshot audit of the committed state only).
- P1-1 VERIFIED FIXED: the appended W24-D worklog entry matches AGENTS.md §4 exactly, is append-only, discloses the W24-P reconstruction, and is truthful — all 9 commit hashes/descriptions match git log, and every cited verification result matches W24-L's green list item-by-item.
- P2-1 VERIFIED FIXED: mixed native rows now render valid/(valid+refused) denominators + a typed-refusal clause (41/43 avi, 8/10 wav — string-identical to the new renderer, re-derived from the raw rows); §6.2's "all decoded (43 files)" corrected to 41/43 + 8/10 with all four files (0257/0146/0370/0241) named and attributed (facts cross-checked against the regen manifest, §6.3 and crates/demux/src/wav/format.rs); JSON cells + editorial note aligned; unit test mixed_row_surfaces_its_refusals genuinely exercises the path (read it, ran it).
- P2-2 VERIFIED FIXED: Steps-lane cells derive from the recorded StepRecords via steps_cell/lane_steps (failure/timeout → steps-failed quoting the step; pass → byte-identical old text, asserted); reachability confirmed (run_bounded records nonzero/timeout outcomes as Ok); tests cover failed/timeout/pass/no-records for both row kinds.
- No regressions: raw .jsonl blob identical (469f246…); JSON counts + md headline + §3 table byte-identical (166 rows; 84/22/6/0/54; oracle 29/6) — reporting-only confirmed; tests additions-only (11/11 pass, 0 ignored, no deleted asserts); clippy --all-targets 0 warnings under the mirrored deny lints; fmt clean; scope = 4 findings-named files + worklog.
- Scout findings on the fix lane: 1×P2 (IN-8 aac-lc evidence double-append not addressed at this tip + no fixed-vs-deferred accounting anywhere on the branch; the W24-D entry's "P3 … are being fixed" sentence now overstates the tip) and 2×P3 (P3-2 oracle vectors still the sharpest gap; steps_cell failure detail hard-codes wasm prose). Snapshot caveat recorded: lane P may still be running.

Stage Summary:
- docs/reviews/w24-p-fixes-findings.md committed on w24/r10 and pushed: verdict 0 P0 / 0 P1 / 1 P2 / 2 P3 on the fix lane; per-L-finding table: P1-1 FIXED, P2-1 FIXED, P2-2 FIXED, P3-3 half-fixed, P3-10 fixed beyond the ask, P3-1/2/4/5/6/7/8/9 + IN-8 not addressed at 022d679; matrix counts unchanged (PASS). Lead action before wave close: confirm with lane P the IN-8 dedupe + applier idempotence guard and an explicit deferred list, then re-audit the final pushed tip.

---

Task ID: W24-R11
Agent: w24-r11 (review scout, read-only)
Task: Audit lane A's NEW commits on w24/a (delta 9a13c39..b22bff5 — three commits: 389bfcc first real chromium capture + fragment-before-verdict, ca21cb5 human report + honest stamp regen + COOP/COEP knob, b22bff5 five R8 P3 fixes + range-blind staging fix) from a separate worktree; lane A's wt/w24a never entered.

Work Log:
- Read-only worktree wt/w24r11 (branch w24/r11) off local w24/a @ b22bff5 (tip re-verified unchanged at audit end).
- Determinism proof of the human report: cloned b22bff5 to /tmp, ran scripts/web-matrix-run.mjs --restamp, diffed the regenerated docs/web/browser-matrix-20261110.md vs committed — byte-identical except the three provenance lines --restamp regenerates. Every table row re-derived independently from the committed JSON (9/17 per mode, 12 pump-timeouts, 2 decoders_absent, 2 range-blind 404s, 14×200 wasm fetches, Playwright 1.64.1, UA, BCD lanes).
- Stamp-regen honesty verified by diff: 389bfcc→ca21cb5 touches only git.commit + appended capture/recordRegenerated; lanes/tooling/artifact untouched; b22bff5 touches no docs/web file.
- P1 found: the honesty note claims the sandbox Chromium was deleted / matrix NOT re-runnable — false: the recorded executable /home/z/.agent-browser/browsers/chrome-157.0.8103.50/chrome exists (mtime 06:34, never reinstalled) and runs (--version exit 0); the wasm artifact still exists with the exact recorded sha256; "only ffmpeg remains" is also stale (ms-playwright cache holds only .links since ~22:52). The final-tree re-run that would close R8 P2-1 was skipped on a false premise.
- R8 P3 verification: P3-1 waits.timeoutMs CLOSED (spec threads it; runSuite asserts + overwrites), P3-2 runner.timeoutMs CLOSED (playwright.web.config.ts validates + uses), P3-3 spawn-error server close CLOSED (.finally), P3-4 multi-range CLOSED (warn + 200 full body, RFC 9110 §14.2), P3-5 worklog OPEN.
- Range-blind staging fix code-reviewed sound: fixture now also staged under rangeBlindPath; server's rangeBlind branch (Range ignored, no accept-ranges, 200 full body) matches the scenario's assertions; range-aware route untouched; no browser evidence yet (ties to the P1/P2 re-run).
- COOP/COEP knob verified: serving.coopCoepDefault=false, boolean-validated, rendered into every report, doc 09 §9.5 decision record consistent with §12 Q8/§2.5 item 6/§2.3/§3.3.
- Strictness: npx tsc --noEmit exit 0; npm run lint exit 0 (0 errors / 28 pre-existing warnings, none in touched files); no .rs in delta → cargo gates N/A. node_modules supplied by hardlink-copy (symlink creation blocked in this sandbox; gitignored).
- Findings committed to docs/reviews/w24-a-newcommits-findings.md (1×P1 false non-runnability claim, 1×P2 final-tree re-run outstanding, 4×P3) + this entry; pushed w24/r11.

Stage Summary:
- Verdict: R8 P2-2 CLOSED (markdownName now the real .md writer), P2-1 HALF-closed (capture committed + provenance honest, but predates b22bff5's fixes and the claimed impossibility of re-running is false), 4/5 P3s closed, range-blind fix sound on code review, COOP/COEP knob correct, scope clean, UI gates green.
- D-10: MEASURE — renderer determinism proven by byte-diff; all report numbers re-derived from the JSON; chrome binary + wasm artifact existence/execution verified live; tsc/eslint green. NOT TESTED — the browser matrix itself (read-only mandate; would rewrite the record). WHO HITS IT FIRST — lane A at close-out: fix the honesty note, re-run the matrix on the final tree while the browser exists, commit the fresh record, append the worklog entry (R8 P3-5), refresh the stale chromium comment + "735 lines" literal.

---

Task ID: W24-R12 (wave 24 review scout 12 — gate-document chain coherence pass)
Agent: w24-r12
Task: Read-only coherence audit of the Phase-7 exit-gate evidence chain w24/f (833944e) → w24/k (ef15bcc, R3 fixes) → w24/s (faeca6b, R9 IN-6/IN-5 fixes): internal coherence of docs/gates/phase7-exit.md end to end, log-header truth + prepend-only proof, cross-reference resolution, numeric recounts, honesty-rule compliance.

Work Log:
- Worktree wt/w24r12 off origin/w24/s; read the 280-line report end to end; cross-checked §1↔§2↔§3↔§4↔§5↔§6↔§8 (18/22 green recounted exact; 4 FAIL rows map to OF-1/2/3/4; fix-in-flight pointers OF-1 70ed69c + OF-3 771259c verified on w24/a with pre-report timestamps; S's 7.I1/7.K1 rows still match §1's "honestly in-flight on w24/a" characterization — no pass claimed anywhere; §5/§8 consistent with the honesty-pass-still-pending status).
- Logs: all 11 phase7-exit-*.log carry headers; byte-diffed the 3 retrofitted r1 logs (test, clippy-r1, evidence-rerun) against origin/w24/f — bodies byte-identical (K's diff = +29/−0, prepend-only confirmed); the other 15 logs byte-identical f→s; header facts verified TRUE (wt/w24f paths in bodies; a4e35e7 = c339ee8 + exactly the 4 r1 logs; r1 "before 16:51:45Z" and r2 16:56:26→17:12:23 all bracketed by a4e35e7/2bf5b66 commit times; evidence-rerun exit honestly marked unrecorded/inferred).
- Cross-refs: 26/26 in-repo cited paths exist at tip; 14/14 spot-checked file:line cites exact (incl. K's re-measured hw/mod.rs:51-56 and help.rs:303-324); docs/reviews/w24-f-gate-findings.md exists on origin/w24/r3 as cited by all 3 R3 re-verification pointers; docs/web/browser-matrix-20261110.{md,json} exist on local w24/a (b22bff5) — unpushed branch caveat recorded; undated browser-matrix.md confirmed absent from every branch as claimed.
- Numbers recounted exact: 4418/4/49 over 306 test-result blocks (exit 101); wasm 5,998,961 B + 4 exit-0 markers; l10n 611-0-0-0; deny 30 wildcards; UI 705/706 (2 load-fail suites + 1 drift); clippy 1m14s/21.80s; all 8 xtask sub-claims; parity 64/252×3 + web 0/252 + strict 0/252 + cells 133/192/505/142/36; per-card spot counts (codecs 506, lumen-web 111, control-protocol 47+25, control-server 11, platform-linux 13+15+16+16, device-discovery 37, playback 81, plugin-host 44, crash-isolation 6, examples 5, Discovery/Convert 8/8).
- Findings (docs/reviews/w24-s-chain-findings.md): P2-1 — the chain tip's fix-status narrative is stale w.r.t. fixes lane w24/g (OF-2 1c48902, OF-3 64757b1 byte-identical to lane A's, OF-4 ba9659f, OF-5 de9b341; landed 17:56–18:00Z, before K's 21:52 and S's 23:42 edits; already merged into the rehearsal) — the doc nowhere mentions w24/g and §3 OF-2's "not owned by any wave-24 lane" is false as read at tip time (conservative direction; recommend a ~10-line fix-lane addendum or conscious Lead acceptance). P3-1 — §1's "all with single-commit root causes in wave-23 commits" is false for OF-4 (environmental root cause; test last touched wave 22). P3-2 — §1's closure path omits OF-4. P3-3 — 7.I1/7.K1 deliverable citations resolve only against unpushed local w24/a. 4 informational observations (clippy-r1 filename rename at 2bf5b66; mixed time anchors; wasm artifact mtime 16:16 vs 16:56:57 header = cache reuse; doc 09 §12 "Q5" numbering style).

Stage Summary:
- Verdict: the f→k→s chain is internally coherent and evidentially sound (numbers exact, citations 45/45 resolved, logs prepend-only, honesty rules held; NOT-PASSED verdict + closure path explicit); one merge-relevant gap (P2-1, w24/g fix-lane acknowledgment missing from the doc) for the Lead to patch or consciously accept before merge. Read-only audit: only docs/reviews/w24-s-chain-findings.md + this worklog entry written; no source or gate-doc edits. Commit on w24/r12, pushed origin w24/r12; main untouched.

---

Task ID: W24-R13
Agent: w24-r13 (wave 24 review scout — audit fixer lane W24-U; branch w24/r13 off origin/w24/u @ 1244e82)
Task: READ-ONLY audit of w24/u (2 commits on origin/w24/q): T's F-1 fix in build-msi.ps1 (unescaped Source basenames + missing &apos;), the discovered pre-fix parse failure (bash-style \" — 4 ParserErrors), the New-MsiWxs extraction, and the new -SelfTest.

Work Log:
- Setup: worktree wt/w24r13; remote refspec only fetches main, so branched w24/r13 from the fetched w24/u commit 1244e82 directly (identical to origin/w24/u head; worktree wt/w24u checked out at the same sha).
- Audit executed with U's still-present /tmp/pwsh-test/pwsh (PowerShell 7.7.0; auditor reuses it under the same verification-tooling-only, nothing-committed disclosure): (1) REAL parse both revisions — pre-fix 4 ParserErrors at lines 161/164 (exactly U's report), post-fix 0; consumer test-msi-upgrade.ps1 also 0; (2) -SelfTest run: PASS rc=0; red/green mutants in /tmp copies — dropping &apos; fails exactly the raw-text assertion, reverting to unescaped $base fails with ET.ParseError line 25 column 51 (the original failure mode); (3) fake-root full-script e2e (fake exes + fake wix, T's methodology): real config rc=0 (.msi + .wxs written, parses), -VersionOverride 0.0.9 -> Package/@Version="0.0.9", hostile docs a&b<c>"d".md + it's.md rc=0 with round-trip + raw pins green, all three refusals (duplicate/empty/missing) rc=1 with unchanged messages; (4) cross-twin byte-diff vs gen-wxs.sh under the same env (hostile identity values included): only the 4 provenance-comment lines + the .ps1's pre-existing missing trailing newline after </Wix> — U's claim confirmed exactly; (5) template verbatim check: old vs new here-string byte-identical after normalizing the two env->param interpolations ($msiVersion->$Version, $($env:PRODUCT_MSI_UPGRADE_CODE)->$UpgradeCode); (6) XmlEscape order/ set audited: & first (no double-escape possible), full set identical to gen-wxs.sh xml_escape, applied to pure [IO.Path]::GetFileName basenames — Source carries no path prefix in either twin, exe rows unescaped symmetrically.
- Battery (ACTUAL): gate.sh PASS (51 vars, 14 scripts bash -n + sh -n, python tools, ps1 structural, stagers, both .UNSIGNED fallbacks); cargo run -p xtask --locked -- lint-literals PASS (0 violations / 452 allowlisted); ps1-sanity.py PASS on both .ps1. Scope verified: diffstat = build-msi.ps1 + worklog.md only.
- U's three handed-over follow-ups verified REAL: (a) gen-wxs.sh --self-test lacks the raw-text pin — PROVEN by mutant: gen-wxs.sh with s/'/\&apos;/g deleted still passes --self-test rc=0 (the .ps1 twin's equivalent mutant fails); (b) docs/packaging/README.md .msi proof row (line 64) + local-proof paragraph (line 141) credit only gen-wxs.sh --self-test, predating build-msi.ps1 -SelfTest; (c) gate.sh wires neither self-test (no self-test/SelfTest/gen-wxs hits in gate.sh).
- Writes (the only two allowed): docs/reviews/w24-u-fixes-findings.md (full per-item verdicts + evidence) + this entry. No other file touched.

Stage Summary:
- VERDICT: PASS — merge-ready. All 7 audit items green; zero blocking/high/medium findings; U's worklog accurate on every spot-checked claim (4->0 parse errors, line 25 column 51, rc values, messages, cross-twin diff shape). 3 LOW follow-ups confirmed for the Lead: gen-wxs.sh raw-text escape-set pin (proven gap via mutant), README build-msi.ps1 proof row predating -SelfTest, gate.sh self-test wiring (both Q's and U's hand-overs converge here). 3 INFO notes: symmetric dead escaped-description in both twins, pre-existing trailing-newline divergence (XML-insignificant), honest [VERIFY at first live run] boundaries (wix build on escaped Sources, Windows-runner pwsh, 5.1) correctly kept open — this audit's evidence is pwsh 7.7.0 on Linux, same class as U's own proof.

---

Task ID: W24-R14
Agent: W24-R14 (wave 24 review scout — final-state audit of lane P, compat-matrix issue #53).
Task: Audit origin/w24/p @ c48dc76 (the pushed final tip, 11 commits on origin/w24/d) — the 8 commits R10 did not see (022d679..c48dc76); findings in docs/reviews/w24-p-final-findings.md (this worktree/branch w24/r14).

Work Log:
- Worktree wt/w24r14 at c48dc76; remote ref confirmed via git ls-remote (local w24/p identical). Diff w24/d...w24/p: 9 files +1,114/−55; unaudited sub-range 74bda46..c48dc76: 7 files +710/−29, none touching worklog.md (verified per-commit).
- IN-8 (R10-1) verified closed: b5aff90 — two idempotence limbs in edit_row's evidence append (distinct-citation dedupe within a coalesced group; never re-append a citation already a "; "-segment of the row's current unescaped value, via the new string_value_of), + 2 unit tests (coalesced_evidence_is_deduped, recited_evidence_is_not_reappended — both run green here); 6a1677c — the aac-lc duplicate removed with a 7-line provenance comment; my independent full-file segment scan: zero duplicated evidence segments on any of the 252 rows.
- R10-2 (P3-2) verified closed: 74bda46 — 10 oracle tests (prefix_verdict ×4, f32_verdict ×3 with power-of-two offsets for exact RMS/peak asserts, committed_record_applies, compare_row_committed ×2) + 5 config tests; asserted messages match the implementation's real error strings; the applicability refactor (closure → oracle::committed_record_applies) is semantics-preserving (compared verbatim).
- R10-3 verified closed: c48dc76 — steps_cell's decode-refusal clause is a 4th parameter; both call sites pass their own wording (format_cell's byte-identical to the old text); new test proves a non-web steps lane never renders "wasm32".
- P3-7 (cd7d5c5): honest documented next-step, not a fix — manifest purpose + report §7 name the permanent 0385 gap and the re-authoring path, argvs/sha256/deterministic untouched, manifest parses (67 entries). Deferred to the corpus owner, as the entry itself states.
- FINDING R14-1 (P1): no W24-P worklog entry exists on the final pushed state (only a776c24's W24-D entry touches worklog.md); the fixed-vs-deferred list R10 asked for is absent, and P3-1/P3-6/P3-8/P3-9 are deferred with the deferral recorded nowhere (each re-verified untouched at the tip — P3-1's notes-insertion-vs-verify_diff conflict still present at compat_matrix.rs:671-682/793-798). Same §4-violation shape as L's P1-1. FINDING R14-2 (P3): the IN-8 re-cite guard matches exact "; "-segments only (theoretical over-append edge; no data loss).
- Verification battery (all run in this worktree): tests/compat cargo clippy --all-targets --locked -- -D warnings CLEAN; cargo test 27/27 (0 ignored). xtask clippy CLEAN; compat_matrix tests 12/12. fmt clean (both workspaces). lint-literals PASS (0 violations, 452 allowlisted).
- No-regression checks: JSON counts block byte-identical to w24/d — 166 files; 84 decoded / 22 partial / 6 refused (2 probe + 4 demux) / 0 demux-fatal / 54 malformed; oracle 29 bit-exact + 6 tolerance-fail (recounted from oracle.rows; all 35 rows live_run). raw-rows.jsonl blob 469f246… identical both refs; no .jsonl in the diff. Tests additions-only (no deleted asserts/#[test], no #[ignore]/should_panic anywhere; sha256 4 vectors intact; test counts d→p: oracle 0→10, config 0→5, report 0→8, xtask 10→12). Scope: findings-named files + worklog only.

Stage Summary:
- Verdict on lane P's final pushed state: 1 P1 / 0 P2 / 1 P3. All technical asks CLOSED and verified (IN-8 both halves, R10-2, R10-3, P3-3/P3-4/P3-5 fixed, P3-7 closed-as-documented-debt); counts/raw-rows/tests/scope all clean. The P1: the mandatory W24-P worklog entry never landed — the branch's only accounting is the W24-D entry's now-unreconcilable "P3s being fixed by W24-P" closing line. Remediation is one append-only commit (the fixed-vs-deferred tables in docs/reviews/w24-p-final-findings.md are ready-made content). R14-2 is a theoretical robustness note on the new guard. READ-ONLY audit honored: only docs/reviews/w24-p-final-findings.md + this entry written; pushed as branch w24/r14.

Task ID: W24-R16 (wave 24 review scout 16 — audit of fixer lanes W24-V and W24-W)
Agent: w24-r16
Task: Read-only audit of lane V (origin/w24/v @ 978ed91 — R12 P2-1/P3-1/P3-2 fix, the Phase-7 gate report's w24/g fix-lane addendum) and lane W (origin/w24/w @ 8e97c56 — plan-doc 7.I1 deliverable-name drift fix), both off origin/w24/s (faeca6b); findings in docs/reviews/w24-vw-fixes-findings.md; branch w24/r16 off origin/w24/w.

Work Log:
- Setup: fetched w24/v + w24/w; worktree wt/w24r16 on new branch w24/r16 off origin/w24/w (worktrees busy — 14 existing); read both lane diffs in full (V: phase7-exit.md 53/8 + worklog 21; W: plan doc 1/1 + worklog 16 — scope clean, nothing else touched).
- Lane V, hashes + timeline: all 9 cited commits rev-parse (1c48902/64757b1/ba9659f/de9b341/70ed69c/771259c/8121cbf/286bacd/c339ee8); the 5 g-side commits are ancestors of w24/g; w24/g == origin/w24/g @ 8121cbf, merge-base vs origin/main = c339ee8, 7 commits. Timestamps re-read: g fixes 17:56:10–18:00:29Z, K 21:52:12Z, S 23:42:25Z, report 833944e 17:16:47Z — the addendum's 17:56–18:00Z-before-K/S timeline is exact.
- Lane V, citations: docs/reviews/w24-g-fixes-findings.md exists on origin/w24/r5 — quoted SHIP verdict verbatim, suite counts 43/0, 8/0, 125/0, 12/0+3i and settings:check/fmt/clippy/no-weakening all match; docs/reviews/w24-integration-findings.md exists on w24/r9 == origin/w24/r9 @ d47d578 — 17 merges incl. g confirmed and re-verified myself (merge-base --is-ancestor w24/g 286bacd → true); seam row 1 (OF-5 retraction verbatim, D+G decomposition) quoted accurately. W24-R10's report grepped: 0 hits for w24/g or OF-2..5 — it is the lane P audit, so V's R5+R9-for-R10 deviation was the right call (brief miscite).
- Lane V, substance: OF-3 byte-identity re-proven (git diff w24/a w24/g -- ui/.../schema.ts → empty; both commits pure +105); ba9659f implements the [PROPOSAL]'s first option (CARGO_MANIFEST_DIR) as the bullet states; de9b341 lands the OF-5 retraction with status cells untouched; §1 closure path accurate — g NOT an ancestor of origin/main (Lead merge genuinely pending), OF-1 still lane A's (70ed69c on unpushed w24/a, main still carries 2 <<<<<<< blocks, g's diff never touches App.tsx); P3-1/P3-2 fixed exactly as R12 prescribed; verdict header honestly frozen at report time. Tables re-parsed programmatically on both S and V versions: 3 blocks 24/16/6 rows, uniform pipes 5/5/4, separators intact; §2 and §4–§8 byte-identical to origin/w24/s.
- Lane W, substance: 7.I1 row (line 764) is single-line, 8 pipes / 7 columns matching the header; both dated files cat-file -e on w24/a (--diff-filter=A: .md ca21cb5 23:18:51Z, .json 231598f 19:06:05Z — "delivered 2026-11-10 on w24/a" exact); undated docs/web/browser-matrix.md swept over EVERY local + remote ref → 0 hits (never existed anywhere); repo-wide re-grep of the undated form → only the gate report's explanatory clause (phase7-exit.md:192) + worklog historical entries — W's sweep claim confirmed; no new undated references introduced.
- One P3 on W (worklog accuracy only, landed doc unaffected): the side note "docs/web/ is empty on every other branch including this one" was already false at W's commit time (00:59:16Z) — w24-integration-rehearsal has carried the dated .json since the r8 merge 3f2b96c (23:06:48Z), and w24/r11 (eaf7970, 00:31:34Z, built on lane A's b22bff5) carries both files. Provenance attribution "delivered on w24/a" remains correct (downstream copies derive from lane A). Recorded in the findings doc; no action demanded (append-only worklog). Also 3 INFO notes (V's "exactly as prescribed" understates OF-5's richer landed note; W's R12 section attribution; W's base-selection note verified consistent with V's 01:04:56Z push).
- Worklogs vs AGENTS.md §4: both entries exact-format, pure appends (diff hunks touch only the tail; zero prior lines modified); V's verified truthful claim-by-claim (incl. the 280-line count at the S tip); W's truthful except the P3 side note.
- Wrote docs/reviews/w24-vw-fixes-findings.md: verdict PASS/PASS merge-ready (0 P0/P1/P2; 1 P3 worklog nit on W; 3 INFO), full verification-run ledger. Noted both lanes already merged unchanged into the rehearsal (W e19d4b8 01:05:14Z, V 6b0b5d8 01:06:56Z) — no objection from this audit.

Stage Summary:
- W24-V and W24-W audited on w24/r16 (off origin/w24/w 8e97c56): BOTH PASS, merge-ready. Every substantive claim in V's §3 fix-lane addendum verified true (hashes, containment, timeline, R5/R9 citations verbatim, OF-3 byte-identity re-proven, closure-path accuracy, P3-1/P3-2 fixed as prescribed, tables intact, §2/§4–§8 untouched) and V's R10→R5+R9 deviation confirmed correct. W's plan-doc fix verified true (dated files + provenance exact, undated name never existed on any ref, row structure intact, sweep claim re-confirmed). One P3: W's worklog "docs/web/ is empty on every other branch" side note was false at commit time for the rehearsal + w24/r11 (copies derived from lane A; landed doc unaffected). Files touched: docs/reviews/w24-vw-fixes-findings.md + this worklog entry; branch w24/r16; pushed origin w24/r16; main untouched.

---

Task ID: W24-S (wave 24 lane S — small integration fixer)
Agent: w24-s
Task: Fix integration audit W24-R9 finding IN-6 — dangling browser-matrix deliverable name in the Phase-7 exit gate report (docs/gates/phase7-exit.md, §4 row 7.I1); docs-only lane.

Work Log:
- Read the finding (docs/reviews/w24-integration-findings.md IN-6 on w24/r9): phase7-exit.md:192 promised `docs/web/browser-matrix.md` as issue #49 / card 7.I1's deliverable; the file exists on no branch — name drift, dangling reference in the merged tree.
- Verified the REAL deliverable names against the shared repo's local branch state (w24/a is unpushed — checked out in lane A's busy worktree; `git ls-tree w24/a -- docs/web/`): `docs/web/browser-matrix-20261110.json` (machine record; real chromium capture since 389bfcc) + `docs/web/browser-matrix-20261110.md` (human report; ca21cb5, rendered FROM the committed JSON by renderMatrixMarkdown(); exists ONLY on w24/a — rehearsal/r8 carried lane A only up to 9a13c39). Lane tip at fix time: b22bff5. Undated `docs/web/browser-matrix.md`: ls-tree sweep over every local head + remote ref → 0 hits (never existed anywhere), matching R9.
- Fixed phase7-exit.md §4 row 7.I1: deliverable citation repointed at the dated `.md` + `.json` with a W24-R9 IN-6 provenance note; lane-A tip citation refreshed 771259c → b22bff5, time-anchored "at W24-S fix time" (required for internal truth: docs/web/ is EMPTY at 771259c, so the row would otherwise cite deliverables that don't exist at its own cited tip); row semantics preserved — #49 in-flight on w24/a, no pass claimed, Lead re-verifies at wave merge, §5 web snapshot pointer kept. The card's "web feature-matrix honesty pass" half is now marked still-pending per IN-5, re-verified TRUE at current branch state: web-column value histogram identical to main on every branch (blocked=27, not_started=206, partial=4, unsupported=15, implemented=0 — the only feature-matrix.toml changes anywhere are lane D's linux implemented→partial flips).
- Applied the IN-5 budgets nuance to §4 row 7.K1 (evidence cell, previously "—"): budgets pending — no budget-gate work exists on any branch (verified: `git log --all -S web-budget` → only R9's own review commit; no branch's diff touches .github/workflows/ci.yml; scripts/full_gate.sh carries no web-budget step on any branch — this tree runs steps 1–9), and ci.yml's size job records sizes only ("SIZE BUDGETS: NONE enforced", budgets [PROPOSAL] pending doc 09 §12 Q5); covered by the Lead's §6 #49 re-verification at wave merge.
- Markdown verification: §4 table rows all single-line with exactly 5 pipes (4 columns) including both edited rows; no headings or other sections touched; diff = 2 lines changed in 1 file.
- Deliberately NOT touched (outside this lane's ownership — docs/gates/phase7-exit.md + worklog only): docs/13-agent-wave-program.md:764, the plan doc's 7.I1 Deliverables cell, still names the undated `docs/web/browser-matrix.md`. It is the ORIGIN of the promised name (plan-vs-delivered drift, not a citation of an existing file), and R9 IN-5's sanctioned action ("the Lead should re-baseline the wave-24 deliverable list for the web lane") owns that doc. Flagged here for the Lead. Note: phase7-exit.md:192 does mention the undated path once more, inside the explanatory clause "(the undated … exists on no branch)" — an intentional historical record of the drift, not a dangling citation.

Stage Summary:
- W24-R9 IN-6 closed on w24/s (commit 4277355): the Phase-7 exit report now cites lane A's real deliverables `docs/web/browser-matrix-20261110.{md,json}` instead of the never-existent undated `browser-matrix.md`; the lane-A tip citation is current and time-anchored; the honesty-pass half is honestly pending (IN-5); the 7.K1 row carries the budgets-pending reality (IN-5). Remaining undated reference: docs/13-agent-wave-program.md:764 (plan doc, Lead/IN-5 rebaseline — outside lane ownership). Files touched: docs/gates/phase7-exit.md (2 lines) + this worklog entry; branch w24/s off origin/w24/k (gate-doc fixer chain tip ef15bcc); pushed origin w24/s; main untouched.

---

Task ID: W24-T
Agent: W24-T (wave 24 review scout — audit of the W24-M and W24-Q fixer lanes; branch w24/t, base origin/w24/q @ 99e80c6; M audited via git in a disposable detached worktree)
Task: Read-only audit of origin/w24/m (soak-review fix round, 10 commits on w24/b, closing all 9 W24-R4 findings) and origin/w24/q (packaging P4 fixes, 6 commits on w24/h, closing W24-R6 P4-1..P4-4). Findings: docs/reviews/w24-mq-fixes-findings.md.

Work Log:
- W24-M, all 9 R4 findings re-verified: P1-1 worklog entry truthful (battery numbers match R4's recorded re-run exactly — 29/29 @ 39.4 s, forced-threshold 2-violation FAIL span 849,815, crate green, clippy/fmt/lint-literals clean; lane +4,529/−0 over 9 commits with matching timestamps; provenance-labeled, honest gap named). P2-2 NEG_INFINITY fix REAL — reverted the one line in a scratch worktree and the regression test FAILED, restored and 32/32 pass. P2-3 span 1,699,754 recomputed from the raw evidence (stride 489, 4,090 samples, 613 warmup-excluded, tag 300,246) + forced 849,815 spot-checked in the raw violations; unit test pins all three shapes. P2-4 $comment repointed at the real §6.2 (no stale refs). P3-1..P3-5 all verified (payload fold live-renders ~61.5 KB/frame; 30.9 min exact; provenance note correct on 224-vs-225; VmRSS honest-absence note; seeks_per_reopen default 16 / [1,1024] / schema+loader+tests, no shipped manifest sets it).
- W24-M battery (my runs at origin/w24/m): cargo test -p playback-engine --test soak_scenarios 32/32 in 39.32–39.38 s; clippy -p playback-engine --all-targets --locked -- -D warnings CLEAN (cold cache); cargo fmt --check CLEAN; lint-literals PASS 0/452; forced-threshold --release re-run FAIL as designed (exit ≠ 0, span 849,815 recorded — my run 1 violation vs R4/M's 2, run-to-run variance in the pinched-budget proof). Zero removed asserts in the lane diff (17 added) — no test weakening.
- W24-Q, all 4 P4s re-verified empirically: P4-1 guards derive from packaging.toml at guard time (tomllib inline python) — synthetic 9th key foo_env inside [signing.windows] → ::error:: naming $WINDOWS_SIGNING_FUTURE_FOO rc 1, while the ORIGIN/W24/H guard passes the same TOML silently (fix real, not hardcoded to 8); bridge-drift and unmapped-secret branches also proven; honest-unsigned warning semantics unchanged. P4-2 self-test PASS (ran it), pre-fix script reproduced (exit 0 + malformed XML line 23 col 51), real-config output byte-identical, refusal paths unchanged. P4-3 recounted 42 run blocks (8+6+6+7+7+8), 42/42 bash -n with ${{ }} neutralized. P4-4 space/tab doc paths refused rc 1 with the actionable rename message (tested with /tmp fixtures).
- W24-Q battery: gate.sh PASS (51 vars, 14 scripts, stagers, both unsigned-fallback contracts); bash -n + sh -n all scripts; release.yml yaml OK; schema JSON OK; lint-literals PASS 0/452.
- CONFIRMED follow-up for the Lead (F-1): build-msi.ps1:161,164 still emits Source="$base" unescaped in the mirrored File rows (its XmlEscape is applied to identity fields only; escape set also lacks &apos;) — defect live at w24/q HEAD, outside Q's file ownership exactly as Q scoped it. Q's second handed-over item (self-test not wired into gate.sh) also still open.
- Nits (non-blocking): OBS-1 the M report's parenthetical "warmup-excluded fit windows — 27.3 min" mixes duration measured-windows with full count walls (cycle-proportional would be ~26.2 min; the headline 30.9 is exact). OBS-2 Q worklog says "4 commits" but lists 5 SHAs (+ worklog = 6 on branch; SHAs/claims accurate).
- Verdict: both lanes PASS — every claimed fix real, no new P0/P1/P2, no test weakening. Scratch worktree and /tmp fixtures removed; no audited branch written.

Stage Summary:
- w24/t = origin/w24/q + this audit's findings doc + worklog entry; pushed origin w24/t. M and Q lanes recommended for merge from the audit's side; the ps1 unescaped-$base defect (F-1) needs a new owner before the packaging lane's .wxs story is fully symmetric.
- D-10: MEASURE — everything in the findings doc's honest-sampling statement. NOT TESTED — nightly 4 h profile, LSAN, GitHub workflow execution (billing-blocked), non-Linux suite run, wix build on escaped Sources ([VERIFY at first live run], as labeled). WHAT BREAKS — nothing new from this audit (read-only). WHO HITS IT FIRST — the Lead merging the lanes reads docs/reviews/w24-mq-fixes-findings.md; whoever picks up F-1 hits the ps1 defect at its named lines.

---

Task ID: W24-U
Agent: w24-u (wave 24 lane U — small fixer; branch w24/u, base origin/w24/q @ 99e80c6)
Task: Close W24-T's confirmed follow-up F-1: scripts/packaging/build-msi.ps1 emits File/@Source basenames UNESCAPED (lines 161/164) and its XmlEscape lacks &apos; — the PowerShell mirror of Q's gen-wxs.sh fix (W24-R6 P4-2).

Work Log:
- 2 commits on w24/u: the build-msi.ps1 fix + this entry. Files: scripts/packaging/build-msi.ps1 + worklog.md only (the findings-named ownership; no .rs, no .sh, no .py touched — fmt/clippy not owed, gate's 14-script bash -n sweep still green).
- FINDING EXTENDED while fixing (evidence below): F-1's two lines were not merely unescaped — they used bash-style \" inside PowerShell double-quoted strings, which PowerShell does NOT treat as an escape. The pre-fix script did not PARSE AT ALL: [System.Management.Automation.Language.Parser]::ParseFile over origin/w24/q HEAD's build-msi.ps1 reports 4 parse errors, all on lines 161/164 ("Unexpected token ... in expression or statement"). No prior wave could see this (no pwsh anywhere; ps1-sanity is structural by design and models \" as a string close). The Windows lane's real consumer — test-msi-upgrade.ps1, which invokes build-msi.ps1 — would have died at parse time on its first live run. The fix therefore rewrites those two literals with PowerShell-legal backtick-quote escaping (the idiom test-msi-upgrade.ps1 line 114 already uses) as part of the F-1 change; same lines, same ownership.
- The fix (mirrors Q's f1d45c8 semantics for the .ps1): (a) XmlEscape extended to the FULL set — & < > " and ' as &apos;, & replaced first — byte-for-byte the same contract as gen-wxs.sh's xml_escape (change both together); (b) File/@Source now carries XmlEscape applied to the BASENAME ($escBase; $base is a pure [IO.Path]::GetFileName basename, so there is no staged-path prefix to preserve — the staging dir is the wix working directory); (c) the generation body refactored into ONE function New-MsiWxs (identity + exe names + doc paths in, full .wxs text out; duplicate-basename and empty-list refusals inside it) so the self-test exercises the REAL path, never a copy — the same discipline as Q's generate_wxs refactor; the main flow's docs loop is now staging-only, matching the bash twins' division (build-msi.sh stages, gen-wxs.sh refuses). The .wxs template moved verbatim into the function; interpolation points and order unchanged.
- Self-test: new -SelfTest switch (no config env, no wix, no binaries; needs python3/python — the same interpreter CI provides for packaging-env.py). Runs the REAL New-MsiWxs with the SAME hostile fixtures as gen-wxs.sh --self-test (Name 'A&B "C" <D>', Manufacturer 'P&Q', Description "it's <a> & \"b\" > c", docs 'a&b<c>"d".md' + "it's.md"), then a python xml.etree proof: output PARSES, 4 File rows, hostile basenames + Package/@Name + @Manufacturer round-trip, File/@Ids sanitized + unique. Deliberately STRONGER than the bash twin's proof: two raw-text assertions require the escaped forms 'Source="a&amp;b&lt;c&gt;&quot;d&quot;.md"' and 'Source="it&apos;s.md"' in the raw .wxs — parse + round-trip alone cannot catch a reverted &apos; (a bare ' parses fine inside a double-quoted attribute), the raw pin can and does.
- Verification tooling disclosure: pwsh is not provisioned in this sandbox (gate step 4's standing assumption, unchanged). I downloaded the official PowerShell 7.7.0 linux-x64 tarball to /tmp and used it for this lane's proofs — verification tooling only, not installed system-wide, not on PATH, nothing committed, no repo dependency added (AGENTS.md §3.2 not implicated).
- Proofs EXECUTED (this sandbox, pwsh 7.7.0 unless noted): (1) real parse: pre-fix 4 errors at 161/164 -> post-fix 0; (2) -SelfTest: PASS rc 0; (3) red/green discrimination on /tmp copies: reverting &apos; alone -> rc 1 failing exactly the raw-text &apos; assertion; reverting to unescaped $base -> rc 1 with ET.ParseError "not well-formed (invalid token): line 25, column 51" (the exact W24-R6 failure mode, reproduced through the .ps1); (4) FULL-SCRIPT end-to-end in a /tmp fake root (fake target-dir exes + a fake wix executable that writes the -o artifact; T's fake-root methodology; the script's own OS_ID gate satisfied by env): real config (Lumen, docs LICENSE+README.md) -> rc 0, dist/<base>.msi + .wxs written, .wxs parses; (5) cross-twin diff vs gen-wxs.sh under the same env: the ONLY differences are the 4 provenance-comment lines (each generator names itself) and the .ps1 template's pre-existing missing trailing newline after </Wix> (a PowerShell here-string excludes the newline before the terminator; the bash heredoc includes it — XML-insignificant, pre-existing shape, left as-is per minimal-diff); (6) hostile-docs end-to-end: rc 0, the emitted .wxs parses, both hostile basenames round-trip, both raw escaped forms present; (7) refusal paths through the real script: duplicate basenames (README.md + sub/README.md) -> rc 1 with the same-basename message; empty PACKAGING_DOCS -> rc 1 with the docs-is-empty message; missing doc -> rc 1 with the not-found message (all three messages unchanged from pre-fix W24-H wording); (8) -VersionOverride 0.0.9 -> Package/@Version="0.0.9" (the upgrade-path test's predecessor-build contract intact).
- Battery (ACTUAL): python3 ps1-sanity.py on both .ps1 PASS (ASCII-only, delimiters balanced, param first); source scripts/cargo-env.sh && bash scripts/packaging/gate.sh PASS (51 vars, 14 scripts bash -n + sh -n, python tools parse, ps1 structural, stagers, both .UNSIGNED.txt fallback contracts) — identical to the w24/q baseline; cargo run -p xtask --locked -- lint-literals PASS (0 violations / 452 allowlisted).

Stage Summary:
- F-1 closed on w24/u and pushed: the .ps1 twin now matches gen-wxs.sh exactly on the escaping contract (full set incl. &apos;, escaped basenames, one REAL generation path with an executable hostile-fixture self-test) — and, beyond what F-1 described, build-msi.ps1 now actually parses (its File-row strings were a hard PowerShell parse error invisible to every no-pwsh review wave). Honest boundary: `wix build` on the escaped Source values, the real Windows-runner parse (release.yml runs the .ps1 via pwsh 7; my verification is pwsh 7.7.0 on Linux), and Windows PowerShell 5.1 remain [VERIFY at first live run].
- D-10: MEASURE — parse errors 4 -> 0; self-test PASS with a red/green proof on both halves of the fix (escape set + escaping itself); 3 full-script e2e runs rc 0 (real config / hostile docs / -VersionOverride) with the emitted .wxs machine-parsed each time; 3 refusal paths rc 1 with unchanged messages; gate + lint-literals green; cross-twin output diff = comments + trailing newline only. NOT TESTED — real WiX/Windows execution, 5.1 parse, the release workflow itself (billing-blocked, repo-wide standing caveat). WHAT BREAKS — pre-fix, the first live run of the MSI upgrade-path test died at PowerShell parse time (and, had it parsed, a doc named a&b.md would have produced malformed .wxs); post-fix, a future escape-set or quoting regression fails -SelfTest loudly before any runner is reached. WHO HITS IT FIRST — the user lifting the CI spend limit and dispatching the release dry-run (runbook blockers table); until then, anyone running `pwsh scripts/packaging/build-msi.ps1 -SelfTest` after touching the generator.
- Notes handed to the Lead (all outside this lane's file ownership): (1) gen-wxs.sh --self-test would take the same raw-text escape-set pin (its round-trip-only proof still passes with &apos; reverted — the gap my .ps1 proof closes); (2) docs/packaging/README.md's build-msi.ps1 local-proof row predates -SelfTest (the script's own header + usage block now document it); (3) wiring both self-tests into gate.sh remains open (Q handed it over first; gate.sh untouched here).

---

Task ID: W24-V (wave 24 lane V — small fixer)
Agent: w24-v
Task: Fix audit W24-R12's P2-1 (+ P3-1/P3-2) on the Phase-7 exit-gate report: the gate-doc chain tip never mentions fix lane w24/g, which landed OF-2/3/4/5 before the K/S edits. Docs-only lane (docs/gates/phase7-exit.md + this worklog); branch w24/v off origin/w24/s.

Work Log:
- Read the finding (docs/reviews/w24-s-chain-findings.md P2-1/P3-1/P3-2 on origin/w24/r12) + the full 280-line report (§1 verdict/closure path, §3 OF rows, §6/§8 criteria). Verified every fact before editing, in the shared repo:
  - w24/g == origin/w24/g @ 8121cbf, off c339ee8 (origin/main), 7 commits; the four fixes rev-parse'd + branch-containment-checked (merge-base --is-ancestor vs w24/g): OF-2 1c48902 (17:56:10Z, pins derived from the seed + shape guard), OF-3 64757b1 (17:56:43Z, regenerates ui schema.ts +105), OF-4 ba9659f (17:59:31Z, roots rebrand proof in the compiling worktree — the report's [PROPOSAL]), OF-5 de9b341 (18:00:29Z, MF retraction note) — all before W24-K's ef15bcc (21:52:12Z) and W24-S's 4277355 (23:42:25Z), and after the report's 833944e (17:16:47Z): the staleness is real, exactly as R12 states.
  - OF-3 duplication: git diff w24/a w24/g -- ui/src/screens/settings/generated/schema.ts → empty (byte-identical to lane A's 771259c output).
  - W24-R5 verification (docs/reviews/w24-g-fixes-findings.md on origin/w24/r5): "ALL FOUR IN-SCOPE FINDINGS (OF-2…OF-5) FIXED PROPERLY — none weakened. SHIP."; the 4 previously-failing suites re-run green (settings-schema lib 43/0, integration 8/0, lumen-cli lib 125/0, xtask cli 12/0+3i) + settings:check/fmt/clippy clean; no #[ignore] added, no assertion weakened.
  - W24-R9 integration audit (docs/reviews/w24-integration-findings.md on origin/w24/r9, tip d47d578): w24/g is an ancestor of the audited rehearsal snapshot 286bacd (verified myself); seam row 1: lane G's OF-5 retraction present verbatim, merged feature-matrix diff decomposes exactly into lanes D + G, zero overlap. NOTE: the task brief said "+ W24-R10's related audit", but W24-R10 audited lane P (compat-matrix, docs/reviews/w24-p-fixes-findings.md) — zero w24/g/OF content (grepped); the audit R12's own finding cites is W24-R9's, so the addendum cites R5 + R9. Every citation in the landed addendum is verified true.
  - OF-1: fix 70ed69c on w24/a (verified); w24/a unpushed (ls-remote empty — R12 P3-3); w24/g does not touch ui/src/App.tsx (0-file diff) and main still carries the 2 conflict markers — OF-1 remains lane A's, as the report says.
- Edited docs/gates/phase7-exit.md (53 insertions / 8 deletions):
  - §1 verdict (P3-1): "all with single-commit root causes in already-merged wave-23 commits" → "four have single-commit root causes … while OF-4's root cause is environmental (test-infra root discovery under the shared target dir — §3 OF-4)" — now matches §3's own taxonomy.
  - §1 closure path (P2-1 + P3-2): rewritten — OF-2/3/4/5 LANDED on w24/g (4 commits, R5-verified), OF-1 on w24/a; Lead's remaining step = merge the lanes + §8 battery re-run; OF-4 now enumerated (was omitted entirely).
  - §3 OF-2/OF-3/OF-4/OF-5 rows: one "Post-report status (W24-V)" bullet appended each — the false-at-tip "not owned by any wave-24 lane" (OF-2) and the stale "[PROPOSAL]" (OF-4) now point at the landed fixes; OF-3's row records the byte-identical two-lane duplication.
  - §3 end (after the Minor paragraph): new subsection "Fix-lane addendum (W24-V, 2026-11-11) — OF-2/3/4/5 landed on w24/g" — the four commits with one-line descriptions, the timeline (17:56–18:00Z, before K 21:52Z / S 23:42Z), independent verification by W24-R5 (suites green, no weakening) + W24-R9 (rehearsal merge, seam check), OF-1 on lane A (local-only), remaining Lead step. §6/§8 criteria untouched (R12: they "remain correct as a criterion").
- Verification: all 3 markdown tables re-checked programmatically (24+16+6 rows; uniform pipe counts per block; separator rows intact — no table row edited); every hash cited in the addendum resolves (1c48902/64757b1/ba9659f/de9b341/70ed69c/771259c/8121cbf/286bacd/c339ee8); both cited review paths resolve on their named branches (origin/w24/r5, origin/w24/r9); no contradiction remains between §1, §3 rows, and the addendum (rows carry local pointers; addendum frames §3 owner rows as the report-time snapshot).

Stage Summary:
- W24-R12 P2-1 closed on w24/v (off the gate-doc chain tip faeca6b): the Phase-7 exit report now tells the one coherent merge story — verdict/battery frozen at report time, §3 owner rows carrying post-report status pointers, and the new §3 fix-lane addendum recording that w24/g landed OF-2/3/4/5 (1c48902/64757b1/ba9659f/de9b341; R5-verified 4/4 green, no weakening; R9-verified merged into rehearsal 286bacd) with OF-1 still on lane A (70ed69c, unpushed). §1's closure path is now "merge lane A + w24/g, re-run §8" — no longer "apply the OF-2 pin bumps / correct the OF-5 note". P3-1 (root-cause over-generalization) + P3-2 (OF-4 omitted from the closure path) fixed in the same edit. Files touched: docs/gates/phase7-exit.md + this worklog entry (repo-root worklog.md, append-only per AGENTS.md §4). Commit on w24/v, pushed origin w24/v; main untouched.

---

Task ID: W24-W (wave 24 lane W — small fixer)
Agent: w24-w
Task: Fix the plan-doc deliverable-name drift flagged by lanes S and R12 (informational): docs/13-agent-wave-program.md:764, the Phase-7 card table's 7.I1 Deliverables cell, still promised the undated `docs/web/browser-matrix.md`; docs-only lane.

Work Log:
- Base selection: `origin/w24/v` does not exist as a remote ref (local `w24/v` is locked at `faeca6b`, the same commit as `origin/w24/s`); fell back per instructions — branch `w24/w` off `origin/w24/s` (`faeca6b`, the gate-doc chain tip with S's IN-6 fix).
- Read the finding: S's worklog ("Deliberately NOT touched … docs/13-agent-wave-program.md:764 … plan-vs-delivered drift … Lead/IN-5 rebaseline") + W24-R12's chain-findings informational notes (the dated deliverables exist only on local `w24/a`; the undated name exists on no branch). The 7.I1 row's Owns/Deliverables cell read `docs/web/browser-matrix.md`, matrix updates.
- Verified the real deliverables against the shared repo's local `w24/a` (unpushed, lane A's busy worktree): `git cat-file -e` succeeds for both `docs/web/browser-matrix-20261110.md` and `docs/web/browser-matrix-20261110.json`; added on w24/a at `ca21cb5` (2026-11-10T23:18:51Z, the .md) and `231598f` (2026-11-10T19:06:05Z, the .json) — both delivered 2026-11-10 on w24/a. Note: docs/web/ is empty on every other branch including this one; the cell is a plan citation of lane A's deliverables, consistent with the gate report's already-corrected row.
- Fixed the cell: `docs/web/browser-matrix-20261110.md`, matrix updates → `docs/web/browser-matrix-20261110.{md,json}` (delivered 2026-11-10 on w24/a; plan name drift corrected per W24-R9 IN-6/S), matrix updates. Scope/AC text and every other column untouched; the "matrix updates" intent is preserved.
- Swept the whole plan doc (`grep -n browser-matrix docs/13-agent-wave-program.md`): line 764 was the ONLY reference — no other instances to fix.
- Swept docs/ repo-wide for the undated form (`grep -rn "browser-matrix.md" docs/ --include="*.md"` plus a repo-wide extension sweep): remaining mentions are all intentional historical records, none dangling deliverable promises — docs/gates/phase7-exit.md:192 (S/V-owned; the undated name appears only inside the explanatory clause "the undated … exists on no branch"; untouched per ownership rules) and S's append-only worklog entries at worklog.md:814/815/819/822 (never edit other agents' entries per AGENTS.md §4). No other file in the repo carries a stale undated reference.
- Markdown verification: the 7.I1 row is still a single line with exactly 8 pipes (7 columns), matching the wave-table header; diff = 1 line changed in 1 file.

Stage Summary:
- Plan-vs-delivered name drift closed on w24/w: the plan doc's 7.I1 Deliverables cell now names lane A's real dated deliverables `docs/web/browser-matrix-20261110.{md,json}` with the provenance note (delivered 2026-11-10 on w24/a; corrected per W24-R9 IN-6/S); ", matrix updates" intent kept; card scope/AC unchanged; the 7.I1 honesty-pass half remains pending exactly as the gate report records (no status change made or implied). Files touched: docs/13-agent-wave-program.md (1 line) + this worklog entry; branch w24/w off origin/w24/s (faeca6b); pushed origin w24/w; main untouched. Remaining undated mentions in the tree are the two intentional historical records noted above (gate report's drift-explanation clause + prior worklog entries).

---

Task ID: W24-X
Agent: W24-X (wave-24 lane X — small fixer, branch w24/x off origin/w24/p @ c48dc76)
Task: Fix W24-R14's P1 (R14-1): append the missing W24-P worklog entry (AGENTS.md §4) with the explicit fixed-vs-deferred accounting R10-1 limb 3 asked for, and reconcile the W24-D entry's closing line append-only.

Work Log:
- Read R14's final-state audit (docs/reviews/w24-p-final-findings.md on w24/r14); its per-R10-finding and per-W24-L-finding tables are the source material for the W24-P entry above.
- Reproduced R14-1's evidence on this tree (HEAD == origin/w24/p == c48dc76): git log w24/d..w24/p -- worklog.md shows only a776c24 (the W24-D entry), and the last Task ID: in worklog.md was W24-D.
- Verified every cited hash resolves (git show <sha> --stat for all 11: a776c24, 16d64b0, 022d679, 74bda46, 2a43b78, 8f72b30, b30a925, cd7d5c5, b5aff90, 6a1677c, c48dc76) and that the one-liners above match the commit subjects.
- Verified the deferrals at the tip: empty diff w24/d..w24/p for tests/compat/src/decode.rs (P3-6), tests/compat/compat-matrix.json (P3-8), tests/compat/src/proc.rs (P3-9); the P3-1 notes-insertion arm (compat_matrix.rs:671) and verify_diff's key-count rejection (:791) both present; the three named new tests present (coalesced_evidence_is_deduped, recited_evidence_is_not_reappended in xtask; steps_failure_detail_is_parameterized_not_wasm_prose in tests/compat/src/report.rs).
- Appended the W24-P entry above and this entry to repo-root worklog.md — append-only, no prior entry edited, ownership worklog.md only (no code, docs, or committed artifacts touched).

Stage Summary:
- R14-1 remediated in one append-only commit: the W24-P §4 entry now exists, carrying the fixed-vs-deferred accounting exactly as R14's audit tables state it (fixed: P1-1, P2-1, P2-2, P3-2/R10-2, P3-3, P3-4, P3-5, P3-10, IN-8 both halves, R10-3; documented debt: P3-7; deferred/open: P3-1, P3-6, P3-8, P3-9), the verification battery attributed to R14's runs on the pushed tip, and the reconciliation of the W24-D closing line (that text left untouched per the never-edit rule).
- Honest bound: the test/clippy/fmt/lint numbers and the count recount in the P entry are R14's audit runs on the identical commit c48dc76, not fresh W24-X runs (this lane owns only worklog.md; re-running the battery was outside its minimal scope).
- One commit on w24/x (worklog.md only), pushed to origin w24/x; main untouched.

---

Task ID: W24-Y
Agent: w24-y (wave 24 lane Y — small fixer; branch w24/y, base origin/w24/u @ 1244e82)
Task: Fix audit R13's three confirmed follow-ups on the W24-U packaging lane (docs/reviews/w24-u-fixes-findings.md, all LOW, all handed over as outside U's ownership): F-U1 gen-wxs.sh raw-text escape-set pin, F-U2 runbook credits for both .wxs self-tests, F-U3 gate wiring of both self-tests.

Work Log:
- 3 fix commits + this entry. Files: scripts/release/gen-wxs.sh, docs/packaging/README.md, scripts/packaging/gate.sh + worklog.md only (the findings-named ownership; no .rs, no .py touched — fmt/clippy not owed; gate's 14-script bash -n sweep still green, now 15 self-test-exercising steps).
- F-U1 (gen-wxs.sh --self-test lacked the raw-text pin; R13 proved an &apos;-deleted mutant still passed rc-0): ported the two raw-text asserts from build-msi.ps1's -SelfTest into the python proof heredoc — the emitted .wxs must literally contain Source="a&amp;b&lt;c&gt;&quot;d&quot;.md" and Source="it&apos;s.md". RED (reproduced first, pre-fix mutant in /tmp): deleting the s/'/\&apos;/g sed expression -> --self-test PASS rc=0 (round-trip cannot see a reverted &apos; — a bare ' parses fine in a double-quoted attribute). GREEN (post-fix): self-test PASS rc=0 with "escape set pinned on raw text" in the PASS line. RED (post-fix mutant, same sed deletion against the fixed script): rc=1, AssertionError "raw .wxs does not escape ' as &apos; -- the escape set must mirror build-msi.ps1's full set" — exactly the discriminating assert. Real-config generation path smoke-tested (LICENSE+README.md via env): output parses; header/usage comments + self_test doc block updated; twins stay change-both-together (the assert message names the mirror).
- F-U2 (runbook predates -SelfTest): the .msi artifact-matrix proof row and the Windows/macOS local-proof paragraph now credit BOTH twins' self-tests (gen-wxs.sh --self-test, python3-only; pwsh build-msi.ps1 -SelfTest, added W24-U), including their honest requirements and (since W24-Y) the raw-text escape-set pin, with the why (a reverted &apos; parses fine, so parse+round-trip alone cannot catch it).
- F-U3 (gate wired NEITHER self-test; step 4 was ps1-structural-only): new gate.sh step 7 runs both. gen-wxs.sh --self-test runs UNCONDITIONALLY (needs only python3 — the interpreter steps 1/3/4 already require; no config env, so it runs even if earlier steps failed — more evidence, never less). build-msi.ps1 -SelfTest is pwsh-conditional (checked how U's -SelfTest runs: it needs pwsh; the -SelfTest itself needs no config env/wix/binaries, and pwsh exists on every OS): `command -v pwsh` -> run `pwsh scripts/packaging/build-msi.ps1 -SelfTest` for real; no pwsh (this sandbox's standing assumption) -> honest skip label naming step 4's structural check as that twin's local floor + [VERIFY at first live run] — the repo's existing honest-label pattern, never a fake PASS. Header comment documents step 7; PASS line now says "self-tests". Runbook gate paragraph + local-proof paragraph updated to state the wiring.
- F-U3 proofs (executed): (1) gate PASS rc=0 as-is in this pwsh-less sandbox (gen-wxs self-test executes in step 7; .ps1 twin honestly skipped, label printed); (2) gate PASS rc=0 with the /tmp verification-only pwsh 7.7.0 (U's/R13's disclosed tool, nothing committed) prepended to PATH — step 7 then executes build-msi.ps1 -SelfTest for real under pwsh (PASS line printed); (3) RED: a /tmp copy of gate.sh with the gen-wxs.sh path pointed at the &apos;-deleted mutant -> gate FAIL rc=1 at step 7 (failure propagates: bad -> fail=1 -> exit 1).
- Battery (ACTUAL, at HEAD 216adab): bash -n + sh -n on gate.sh + gen-wxs.sh clean; source /home/z/my-project/scripts/cargo-env.sh && bash scripts/packaging/gate.sh PASS (51 vars, 14 scripts bash -n + sh -n, python tools parse, ps1 structural, stagers, both .UNSIGNED.txt fallback contracts, step 7 self-tests); cargo run -p xtask --locked -- lint-literals PASS (0 violations, 452 allowlisted); git worktree add from origin/w24/u, no merge to main, pushed origin w24/y.

Stage Summary:
- All three R13 follow-ups closed on w24/y and pushed. The two .wxs twins now carry the IDENTICAL self-test contract (same fixtures, same parse + round-trip + raw-text escape-set pins — change both together) and BOTH are gate-enforced: the bash twin on every gate run anywhere, the .ps1 twin on every gate run where pwsh exists (Windows runner included, once CI billing lifts), honest-skip otherwise. The runbook's .msi proof story now matches reality.
- D-10: MEASURE — F-U1 red/green triple (pre-fix mutant passes rc-0 / fix green rc-0 / post-fix mutant fails rc-1 on the exact assert); gate green both ways (pwsh-less: honest skip; pwsh 7.7.0: -SelfTest executed) + gate-red rc-1 with a broken generator wired in; lint-literals 0 violations. NOT TESTED — anything inside GitHub Actions (workflow never executed, billing blocked, repo-wide standing caveat); the -SelfTest under the real Windows-runner pwsh and Windows PowerShell 5.1 remain [VERIFY at first live run]. WHAT BREAKS — pre-fix, a reverted &apos; in either twin's escape set passed its own self-test silently (gen-wxs side now impossible: raw pin fails rc-1, and the gate fails with it); the gate's .ps1 branch silently degrades to structural-only on hosts without pwsh (by design, labeled). WHO HITS IT FIRST — anyone editing either generator's escape set or template now fails step 7 of the gate before any runner is reached; the user lifting the CI spend limit gets the -SelfTest executed for real on windows-latest at the same dispatch.
---
Task ID: W24-AC (wave 24 lane AC — consolidated follow-up ledger)
Agent: w24-ac
Task: Produce the single consolidated, deduplicated, prioritized ledger of every wave-24 finding (R1–R16 + IN-* + L + T + the gate's OF-*), tracking each lifecycle FILED → FIXED (lane/commit) → VERIFIED (audit) | DEFERRED (reason/owner) | OPEN (owner/next step), with fixer-chain cross-check, summary counts, Lead-decision items, and user-owned blockers; deliverable docs/reviews/wave24-followup-ledger.md.

Work Log:
- Setup: fetched all w24/* remote refs; worktree wt/w24ac on new branch w24/ac off origin/w24/r16 (702ad89). Ownership: only the ledger + this worklog entry; main untouched.
- Sources inventory: read ALL 17 wave-24 review reports in full — 16 collected at the integration-rehearsal tip 88db8a1 (c/e/f/b/d/g/h/i/a-web/integration/mq/p-fixes/a-newcommits/s-chain/u-fixes/p-final) + w24-vw-fixes-findings.md on this branch; every report's branch tip recorded for citation (r1 cfc196c … r16 702ad89, L 54840f4, T 8a29077).
- Not-filed check (no invention): origin/w24/{r15,aa,ab} do not exist; local w24/r15 == w24/aa == rehearsal tip 88db8a1 and w24/ab == lane A tip c1c37b2, all with zero own commits and clean worktrees — R15's battery findings and AB's size audit were never filed/pushed; the phase7-exit §8 re-run battery remains unexecuted (owner: Lead at wave merge).
- Fixer-chain cross-check via git log/rev-parse/ls-tree/show on origin/w24/{h,i,j,k,m,n,p,q,s,u,v,w,x,y} + local w24/{a,z}: verified r1→h(→q→u→y), r2→i(→j), r3→k(→s→v→w), r4→m, l→p(→x→z), r5→n, r6→q, r8→a, r10/r14→p, r11→a, r12→s/v, r13→u/y hop-by-hop against the audits (R6, R7, T, R12, R16, R10, R14, R13, R11, R5, R9). Gaps found and recorded: J's NEW-1/NEW-2 merged into rehearsal (3b2c79e) but never audited; X's R14-1 fix (3117bb7 on origin/w24/x) and Y's F-U1..3 (a6d6cb7 on origin/w24/y) pushed but unmerged + unaudited; w24/z sits idle at 3117bb7; lane A has NOT fixed any R11 finding (P1 honesty note still false at web-matrix-run.mjs:168-170 on tip c1c37b2; "735 lines" still at mjs:315/.md:171; config comment still stale; no lane-A worklog entry; worktree dirty with in-flight web_budget.rs).
- Rehearsal-state re-verification (2026-11-11): full_gate.sh step-10 hole + stale reserving comment still present at 88db8a1 (IN-2 OPEN); .gitignore target-xtask/ dedupe landed (369c2b4, IN-9 fixed); the four docs/worklog/ side files still present (IN-10 OPEN); lane A remains unpushed (no origin/w24/a) with 3 post-b22bff5 commits (30153fa/b85966f/c1c37b2, the 7.K1 size work) outside the rehearsal; ci.yml/full_gate.sh untouched by every branch (IN-5's budget-gate half still open).
- Lifecycle tables: 101 numbered findings (P0×2, P1×12, P2×24, P3×53, P4/LOW/nit×10) + 13 informational notes; disposition 71 FIXED+VERIFIED, 7 FIXED-verification-pending, 5 DEFERRED (recorded, owned), 15 OPEN (13 unique after deduping R8 P2-1≡R11 P2-1 and R8 P3-5≡R11 P3-4), 3 NO-ACTION; P1/P2 closure rule checked — every P1/P2 ends FIXED+VERIFIED or OPEN-with-owner (the 2 exceptions-in-kind, R14-1 and R7 NEW-1, are FIXED with pending verification and named as such).
- Wrote docs/reviews/wave24-followup-ledger.md: sources inventory, per-report lifecycle tables with citations, chain cross-check, counts, top-open prioritization, 17 Lead-decision items (merge order, IN-2/IN-5 re-baselines, R5-4/IN-10/W-P3-1 ratifications, assignments), 5 user-owned blockers (CI billing, 3-OS machines, Apple R-35, Windows cert, ADR-010 patent sign-off — each with issues/risks + concrete next step), ambiguities/method limits. Committed 44c67be before this worklog append.

Stage Summary:
- Wave-24 ledger delivered on w24/ac (off origin/w24/r16): 101 numbered findings across 17 reports + the gate's OF-*; 71 fixed+verified, 7 fixed-pending-verification (J/X/Y + N's R5-3), 5 deferred with recorded owners (L P3-1/6/8/9 + R2 P3-9), 15 open (13 unique) — top open: R11 P1 (false non-runnability claim, lane A), IN-2 (gate step-10 hole, Lead), IN-5 (web-lane deliverable re-baseline, Lead), R5-1 (shared-target clobber, Lead/tooling), the unreviewed fix lanes J/X/Y, lane A close-out. 17 Lead-decision items; 5 user-owned blockers (ADR-010 patent sign-off binds any public binary/artifact incl. the 7.K1 web bundle). Rehearsal still lacks w24/x, w24/y, and lane A's final tip; the §8 battery re-run is unexecuted. Files touched: docs/reviews/wave24-followup-ledger.md + this worklog entry; pushed origin w24/ac; main untouched.
## 7.w24a — browser matrix, web conformance & packaging budgets (issue #49, cards 7.I1 + 7.K1)

Lane A close-out (rounds r1–r16, Lead-finished). Deliverables on `w24/a`:
- P0 fix: App.tsx committed merge markers resolved (70ed69c) — UI typecheck/lint/vitest green again (718/718, R8-verified).
- OF-3 fix: settings screens regenerated after w23d's seed growth (771259c).
- Browser-matrix harness: config-driven Playwright-Chromium conformance driver (ui/e2e/web/ + ui/scripts/web-matrix-run.mjs + playwright.web.config.ts) — real engine Worker, real wasm artifact, per-mode servers (Range-aware + range-blind + COOP/COEP pair).
- The matrix run: real Chromium captures (chrome-157.0.8103.50 via WEB_MATRIX_CHROMIUM; earlier capture chromium-1200/143, recorded honestly). Final-tree re-run: 10/17 per isolation mode — the 7 fails are the typed web.decoders_absent refusal + downstream pumps; Firefox/WebKit/Safari/iOS lanes = BCD-cited expectations + not-runnable-in-sandbox + next step, never fabricated. Record+report: docs/web/browser-matrix-20261110.{json,md}.
- 7.K1: five real wasm builds (floor/builtin-inhouse/flac/mp3/external-all) with per-config size table + symbol data (docs/web/size-report.md, size-record.toml); budgets as CONFIG in product.toml [web.size_budgets] (+ schema + tests); xtask web-budget --check gate (rebuild rows via real cargo build, integer-exact >5% growth rule, breach exits 1) wired as full_gate step 10 + ci.yml wasm-build step; COOP/COEP posture knob (off-by-default, doc 09 §9) + decision recorded in the matrix report.
- Web-column honesty pass: docs/web/web-column-pass-20261110.md (252 rows, zero unexplained cells).
- Review findings fixed en route: R8 P2-1/P2-2 + all five P3s; R11 P1 (false not-re-runnable claim — matrix re-executed on the final tree) + P3s (stale comment, dirty-tree flag, derived line count).
Known-open on this lane: none. Cross-lane: the 4 known-failing workspace tests (rebrand ×2, gate_dry_run target-path, settings seed pins) are fixed on lane g / merge — verified at wave close on the integrated tree.

---
Task ID: w26.a
Agent: Lead (wave-26 orchestration)
Task: Wave A — foundation, components, player, onboarding (20 lanes, relaunch-heavy)

Work Log:
- State at start: W25 spec committed but redesign unimplemented; Solstice skin live; UI builds clean; 849 tests green.
- Wrote the wave-26 contract (docs/waves/w26/CONTRACT.md): mission flow, design language, per-surface frame map, parallel-work hard rules (file ownership, frozen APIs, i18n keys pattern, no-git/no-install), comparison loop, quality bar. Program: docs/waves/w26/PROGRAM.md (13 waves x 20 agents = 260).
- Wave A ran with three sandbox recycles/server-saturation rounds; the Lead salvaged interrupted lanes' on-disk work each time (salvage commits 5e5f5b0, 4ae6d8e) and fixed cross-lane type/test breaks directly (SwatchGroup readonly+disabled, AppShell callback void, tokens test chaining, onboarding NumberField value coercion, CustomizePanels Icon guard, player sr-only h1 contract resolving the a11/a13 heading conflict).
- Lanes delivered: tokens re-pinned to W25 spec (orange default, Inter, eq band accents, Solstice groups removed); lumen.css replaces solstice.css (backdrop tiers dead); button/slider/menu/form/icon/surface/list families restyled to Figma grammar (frozen APIs kept); appshell (title bar + rail + sidebar); app flow gate (?onboarding=1 bypass); onboarding 11-frame rebuild; player core (single-sidebar composition, hero empty state, picture-frame canvas), control bar single-row anatomy (30 tests re-anchored), states (hide-UI fade), variants (music + miniplayer), customize 9-state, dialogs + ToolsDialogShell for wave-c; two analysis docs (menus-control-parts 641 lines, tools-settings 725 lines with the 13-area mapping: 0 EXISTS / 10 NEEDS-MERGE / 4 NEW).
- Vision loop executed per lane (MATCH verdicts for canvas-hero, loading, bookmarks, button, list rows; documented residues: scrim alpha, 15px font token, #969696/#1B1B1B primitives).

Stage Summary:
- Wave A merged at 0d81605: tsc clean, 894/894 vitest green, stylelint/eslint clean per lane, zero console errors on core routes, all Solstice remnants removed from component + player layers. Residuals owned downstream: settings/playlist/working screens (wave B), audio/video tools + menus-in-screens (wave C), i18n keys registration (b19), 13px/15px font tokens (b20), scrim visibility (wave C fixer).

---
Task ID: w26.b+w26.c
Agent: Lead (wave-26 orchestration, waves B and C)
Task: Settings 13-area reshape, playlists/options/subtitle flows, working screens, i18n registration, token completion; audio/video tools (8) + integration

Work Log:
- Wave B (20 lanes, salvage-heavy due to infra instability — the Lead polled disks, verified, and Lead-closed five interrupted lanes): settings shell to the measured floating-window grammar (b01); the 13-area registry + seed with folds hotkeys→controls, codecs→video, playlist-services→library, about→global, advanced→in-flow disclosure, effects→audio (b02 — settings:check OK); controls/recording/extensions/library areas (b07/b08); playlist core + create-playlist dialog (b09); sorting/view/thumbnail/media options (b10); subtitle flows ×7 frames (b11); discovery (b12); convert/media-info/404/open-media (b13 combined lane, 54/54); network-stream (b15); shortcuts (b16); i18n registration — 216 lane keys merged into messages.ts with a repeatable script, lane-seam precedence flipped catalog-first so pseudo/l10n flow, library retitled per b02 handoff (b19); font tokens sidebar 13px + section 15px through the real cargo pipeline — 80 semantic tokens, pins updated deliberately, stylelint+eslint clean (b20).
- Wave C: EQ suite (frame-exact bands/presets, 46 tests), dynamics tools ×3 (29), visualizer (5-mode selector, honest levels-driven display, 18), video tools ×4 (colors/crop/essential/HDR); all mounted via ToolsDialog (Audio/Video tabs) + Effects-area embed (c5), ToolsDialogHost + gear trigger + lumen:open-tools seam (Lead wiring); context menus wired into playlist/discovery/player (c6).
- Cross-lane Lead fixes: sr-only h1 contract; area-fold test retargets (advanced→performance redirect, codecs badge grouping, audio real area); App 404 copy; tokens sha/length pins; ControlBar gear→tools; icon guards.

Stage Summary:
- Merged green: tsc 0 errors, 1199/1199 vitest, l10n 948/948 keys, stylelint clean, eslint 0 errors. All screens now Figma-composed; 8 tools live; flows wired (onboarding gate → player home → playlists/settings/tools/dialogs). Residuals filed for wave-e: vision proofs owed on ~10 lanes, scrim strength, EQ vertical-fader anatomy (frame A1 vs mounted card), network-stream proportions, r5/#1B1B1B chip tokens.

---
Task ID: w26.lead-pre-g
Agent: Lead (wave-26 orchestration)
Task: Pre-wave-g CI-red cleanup — first real CI executions on main exposed stale pins + a11y regressions (issue #68)

Work Log:
- Diagnosed ui.yml failures from run 43189096326 (Actions billing unblocked — workflows now execute): 18 e2e fails (6 theme matrix, 11 axe, 1 keyboard) + 30/30 visual PR-lite drift.
- Root causes: (1) e2e/theme.spec.ts pins were pre-W25 Solstice values (yellow default #f2c41d, cream surface #e8dbbd, orange light-bright #e8851a, yellow/gray ramps) — CI never ran e2e before, so the W25/f15 repins never reached this file; (2) axe nested-interactive: AppShell sidebar playlists rows render <Link> inside role="option" (every shell route) + kebab/IconButton rows in PlaylistScreen/PlaylistPanel/MediaList (wave f06/f08 additions); (3) settings keyboard audit lacked modal focus-trap semantics (the w26.f03 trap wraps Tab to the window's first focusable — correct behavior, unencoded in the test).
- Fix (1): re-pinned theme.spec to the committed pipeline truth (themes/generated/tokens.css): dark+orange default, surface-raised light #ffffff, orange light-bright #b35f0d (f15 WCAG 4.2:1), yellow #c49e23/#daaf24, gray #afafaf/#bababa; comments document the W25 re-pin authority.
- Fix (2): VirtualList gains semantics="listbox"|"grid" (grid = role=grid/row/gridcell — the ARIA pattern for rows embedding links/buttons); switched the four interactive consumers (AppShell sidebar, PlaylistScreen library, PlaylistPanel queue, MediaList); .lumen-vrow__cell CSS; five test files re-queried option→row. MiniPlayer queue stays listbox (no interactive descendants — verified).
- Fix (3): a11y tab audit now encodes trap wrap semantics: a backward DOM jump is legal ONLY as an exact wrap to the trapped window's first focusable (new assertion — anything else still fails).
- Visual: re-seeded Linux PR-lite baselines after the fixes (runner-rendered truth, program §4 re-seed rules — waves e/f changed visuals since the last seed); PR lane 32/32 green.
- Verified full battery: vitest 1317/1317, e2e a11y+theme+smoke 38/38, visual 32/32, tsc 0, eslint 0 errors, stylelint clean, tokens:check OK (sha 6edc89df), l10n 1139/1139.

Stage Summary:
- main is CI-green again (issue #68 fix complete pending push); GitHub issues #60–#69 filed for waves g–m + releases + Tauri admission + infra. Next: wave g (20 verifiers).
