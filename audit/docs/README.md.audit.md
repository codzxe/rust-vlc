# CLAIM AUDIT: README.md (repository root)

| Field | Value |
|---|---|
| Auditor | W19-A01 |
| Date | 2026-11-16 |
| Claims extracted | 26 | REAL 14 | REAL-UNPROVEN 4 | SHALLOW 1 | STUB 0 | PAPER-ONLY 0 | FALSE 2 | UNVERIFIED 5 |
| Internal contradictions found | 0 |

## 1. Blunt verdict

The README is the best-maintained doc in the set: its headline numbers check out exactly (38-of-73 format rows is a number I reproduced from the matrix to the digit), its honest-gaps block (no sound, null audio) matches the code, and it self-discloses the stale-docs problem ("some older docs still carry the pre-unblock 'billing-blocked' wording"). Its failures are freshness failures, not honesty failures: the "What plays today" block is one wave stale — the canvas path IS wired (w31.4) and http(s) URLs DO open (w31.3) — and the release-publication and CI-executed claims are GitHub-side narratives the tree cannot confirm, standing against release.yml's own "never executed" note. The README's real defect is that it was refreshed on 2026-11-15 and the repo moved the same night.

## 2. Claim ledger

| # | Claim (verbatim ≤25 words) | Location (path:line) | Verdict | Evidence you gathered |
|---|---|---|---|---|
| 1 | "Media engine: 100% Rust — playback state, probing, demuxing, decoding, … streaming, devices, playlists, subtitles" | README.md:4 | REAL | crates/ carries the full set (media-probe, demux, codecs, playback-engine, streaming, device-discovery, subtitles, playlists …); no C/C++ sources in crates. |
| 2 | "Desktop shell: Tauri … this project is never described as 'all-Rust'" | README.md:6 | REAL | apps/desktop-tauri exists; README phrase complies with MASTER-PROMPT §2 (MP:34-35). |
| 3 | "Phase 8 — … in progress (2026-11-15, wave 30 — the remediation wave after the 12-agent read-only audit)" | README.md:16-17 | SHALLOW | The 12-agent audit is real (worklog.md:2146 "Wave 1 = 12 read-only audit agents"); but HEAD is in wave 31 (w31.3/w31.4/w31.6; HEAD commit 727ad6b2 = w31.6, 2026-11-16) — the status line is one wave stale. |
| 4 | "Phases 0–6 are complete (Phase 5: PASS WITH REGISTERED CAVEATS, Lead sign-off 2026-10-24 …; the Phase-6 exit ledger is drafted with verdict PENDING LEAD DECISION" | README.md:18-21 | REAL | phase5-exit.md:3-5 ("PASS WITH REGISTERED CAVEATS", "47 parent cards, all resolved"); phase6-exit.md:3 "Status: DRAFT — PENDING LEAD DECISION". |
| 5 | "Phase 7 landed across waves 20–23 (merge evidence: `git log --oneline --grep='merge wave'`)" (+#41–#52 list) | README.md:21-28 | REAL | Merge commits verified: 9cff3c47 (hw adapters #41), d5611d13 (hotplug #42), 45c5c6f5 (recording/remux #44), 95c07433 (crash isolation #46), 67d567d6 (web pillars #48), c97a3d69 (apps/web #47), 8e8c70cd (control-server #50), f19a30ec (release pipeline #56), 14496c7c (security+SBOM #52). |
| 6 | "Releases v0.1.0, v0.1.1 and v0.1.2 are published (2026-11-13 → 2026-11-15 …)" | README.md:28-31 | UNVERIFIED | Publication is GitHub-side; local git tags v0.1.0/v0.1.1/v0.1.2 exist at exactly those dates (a64b7030 11-13, f4ef9e71/98cec32e 11-15). |
| 7 | "Since v0.1.0: H.264 software decode landed (w27 — bit-exact vs the ffmpeg oracle)" | README.md:31-32 | REAL | rust_h264-0.6.1.md §D "Classification: bit-exact"; crates/codecs/tests/h264_sw_differential.rs; matrix row codec-video-h264-software implemented; executed fact: MP4 H.264+AAC plays end to end. |
| 8 | "wave 30 … lands the Windows fixes: every local file opens by path or `file:` URL … GUI playback actually progresses (the ~30 ms pacer), installers are launch-proven …" | README.md:33-39 | REAL-UNPROVEN | Open-path code real (demux_host.rs routing; worklog w30.1/w30.2); pacer real (apps/desktop-tauri/src/app.rs:202 pump_once); "installers launch-proven" is Windows-side, unverifiable from sandbox. |
| 9 | "**38 of the 73 format rows** … are claimed (implemented or partial) on desktop (14 containers, 7 video codecs, 9 audio codecs, 8 subtitle formats) — zero on web" | README.md:41-44 | REAL | Independent matrix parse: exactly 14+7+9+8 = 38 on windows AND linux AND macos; format-row base 21+16+19+17 = 73; web = 0. Reproduced to the digit. |
| 10 | "the `lumen` CLI drives them headless (decode + sync real)" | README.md:44 | REAL | Executed: MP4 open → tracks 2 ready → Playing → Ended, time 100.0%, EXIT=0 (audit/runs/baseline/mp4-smoke-open.txt). |
| 11 | "audio output is the ADR-005 null device — no sound on any OS yet" | README.md:45 | REAL | cpal.rs:43 AUDIO_ADR005_ADMITTED=false; lib.rs NullBackend+UnsupportedBackend; worklog.md:2294-2296. |
| 12 | "the desktop app opens any local file but **displays no video yet** (the UI mounts an honest empty media surface; the canvas path is built but unwired)" | README.md:46-48 | FALSE | At HEAD the canvas is wired: ui/src/components/player/MediaSurface.tsx `<canvas>` + putImageData presenter; PlayerScreen.tsx:97/1062/1098 mounts MediaSurface; commit e983c683 (w31.4) "the GUI displays the engine's presented frames end to end". Runtime pixels UNVERIFIED (no display in sandbox) — but "empty surface / unwired" is stale by one wave. |
| 13 | "URL sources (http/hls) are refused with a clear message (the streaming engine exists; enablement is the next wave)" | README.md:48-49 | FALSE | http(s) now opens: apps/cli/src/demux_host.rs:427 `"http" | "https" => open_http_source` (w31.3, RangedResource); product.toml:157 streaming-http = true. hls/dash/rtsp refusal still true. The README claim is false for the http half. |
| 14 | "v0.1.0–v0.1.2 are published (billing was unblocked 2026-11-12 …); v0.1.3 is being cut" | README.md:59-63 | UNVERIFIED | Published/billing claims GitHub-side (contradicted by release.yml:26 in-repo); v0.1.3 drafting REAL: product.toml display_version 0.1.3 + docs/release-notes/skeleton-v0.1.3.md. |
| 15 | "Every release ships `SHA256SUMS.txt` — verify first" | README.md:64-69 | UNVERIFIED | Artifact-side; no release assets visible from the tree. |
| 16 | "Two programs ship in every artifact: `lumen` … and `lumen-tauri` … From v0.1.3 the `lumen-desktop` exit-86 stub leaves every user payload" | README.md:71-75 | REAL-UNPROVEN | Stub-drop decision recorded in release.yml header (w30.8) and worklog; actual payloads GitHub-side. |
| 17 | "Binaries are unsigned — expect Windows SmartScreen warnings and the macOS Gatekeeper right-click → Open" | README.md:76-78 | UNVERIFIED | Consistent with RELEASE.md known-limit 1; artifact-side. |
| 18 | "Build from source remains fully supported … `cargo build --release --locked -p lumen-cli -p desktop`" | README.md:80-83 | REAL | Executed fact: PASS (2026-11-16 run). |
| 19 | "the desktop app builds from `apps/desktop-tauri` (own lockfile …)" | README.md:84-85 | REAL-UNPROVEN | apps/desktop-tauri/Cargo.lock exists on disk; the tauri build itself was not executed in the audit sandbox. |
| 20 | Repository layout block (marked "target") | README.md:88-103 | REAL | Root tree matches (apps/, ui/, crates/, plugins/, tools/, tests/, fuzz/, docs/, design/, locales/, themes/, keymaps/, schemas/); divergences self-covered by the "target" label (no config/, layouts/, profiles/ dirs exist at root). |
| 21 | Documents table (MASTER-PROMPT, ROADMAP, 01–15, guides, release-checklist, design/figma) | README.md:105-115 | REAL | All cited paths exist. |
| 22 | "the workflow set (… ci.yml + ci-full.yml …, ui.yml, soak-nightly.yml, release.yml) is written and actionlint-clean" | README.md:125-128 | REAL-UNPROVEN | All five files exist (plus codeql.yml, compat-matrix.yml, rust-clippy.yml, rust.yml, tauri-launch.yml — unlisted but present); "actionlint-clean" is recorded at phase1-exit.md:6-8 (actionlint 1.7.12) but not re-runnable here. |
| 23 | "Billing was unblocked 2026-11-12 and workflows have executed since … and the CI lanes run on `main` (worklog w28.2/w28.3)" | README.md:127-131 | UNVERIFIED | GitHub-side; worklog w28.2/w28.3 entries reference CI runs (e.g. run 43733811241, "this CI run … f4ef9e71") — recorded claims, not tree-verifiable; contradicted by release.yml:26 at HEAD. |
| 24 | "verified parity is still 0/252 (… the evidence-outbox flip pass has not run; all landed evidence is local-run, `hardware_class = mock`), and some older docs still carry the pre-unblock 'billing-blocked' wording" | README.md:131-135 | REAL | verified=0 at HEAD (matrix parse + executed parity-report); the stale-wording admission is verifiable and accurate: PARITY-REPORT.md:123-126, KNOWN-LIMITS.md:86, release.yml:26, soak-nightly.yml:58/210. |
| 25 | "The earlier rustfmt-drift red (w17, fixed by `621fa20` …) is history" | README.md:136-138 | REAL | Commit 621fa203 exists: "ci: fix rustfmt drift across 47 files from wave merges". |
| 26 | "This repository's own code: MIT (decided 2026-10-19 …); … R-07 closed by user attestation" | README.md:142-147 | REAL | LICENSE file present; docs/adr/ADR-006-license-mit.md exists; ROADMAP D-009/D-010 record both decisions. |

## 3. Contradictions

| This doc says | Other source says (path:line) | Which is true (proof) |
|---|---|---|
| "URL sources (http/hls) are refused with a clear message" (README.md:48-49) | KNOWN-LIMITS.md:37 "http(s) progressive/ranged OPENS since w31.3"; product.toml:157 `streaming-http = true`; apps/cli/src/demux_host.rs:427 http arm | The code/KNOWN-LIMITS — README is one wave stale on the http half (hls refusal remains true). |
| "displays no video yet; the canvas path is built but unwired" (README.md:46-48) | worklog w31.4 / commit e983c683 "a 2D putImageData presenter in the media surface … the GUI displays the engine's presented frames end to end"; KNOWN-LIMITS.md:52 (which itself is now false the other way) | The code — MediaSurface.tsx canvas + putImageData statically wired; only runtime pixel delivery remains unverified. |
| "workflows have executed since … CI lanes run on main" (README.md:127-131) | release.yml:26 "THIS WORKFLOW HAS NEVER EXECUTED (CI billing blocked)"; PARITY-REPORT.md:123-126 | Unresolvable from the tree; the README cites worklog entries that reference CI runs, the workflow file says none happened. The README itself admits "some older docs still carry the pre-unblock wording" (README.md:134-135) — release.yml is one of them. |

## 4. Fabrication/inflation indicators

1. **Status-line freeze**: "wave 30 — in progress (2026-11-15)" while HEAD is w31.6 (2026-11-16) — the README's own freshness discipline (date-stamped status) makes the staleness measurable, which is honest, but the "What plays today" block still understates by one wave (http opens, canvas wired).
2. **Unverifiable publication claims**: "published", "workflows have executed", "CI lanes run on main" are all GitHub-side; the tree contains a directly contradicting workflow-file note. The README hedges none of these.
3. **No inflation detected in the checkable numbers**: 38-of-73, 14/7/9/8 split, zero-on-web all reproduce exactly — the quantitative headline is clean, which distinguishes this doc from PARITY-REPORT's stale totals.
