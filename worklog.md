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
