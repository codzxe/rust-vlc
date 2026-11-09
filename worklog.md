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
