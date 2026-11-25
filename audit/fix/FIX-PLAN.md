# LUMEN FIX PLAN — from 19.44% REAL to a real video player

| Field | Value |
|---|---|
| Document | Remediation program plan (follow-up to Devil Audit §8.1) |
| Date | 2026-11-25 |
| Inputs | audit/final/{DEVIL-AUDIT-REPORT, REALITY-SCORE, REMEDIATION-MAP, CONTRADICTIONS, MP4-REALITY-TRACE}.md + wave summaries 01–20 |
| Ground truth | REAL% = 49/252 = 19.44%; MP4 decodes into null sinks; no user has ever seen a pixel or heard a sound; 46 registered contradictions |
| Rule | This is the FIX plan the remediation map refused to be: sequenced, sized, acceptance-tested |

---

## 0. Where we stand (non-negotiable facts)

1. **The engine is real.** H.264 (rust_h264 0.6.1) + AAC (symphonia) decode to 100%
   end-of-media in the shipped release binary; MP4 demux (moov-at-end and
   faststart), avcC/esds harvest, I420→RGBA all execute.
2. **The product is not.** The shipped GUI cannot compile on any host (E0425,
   `app.rs:301` calls `spawn_session_pump`, defined nowhere; `spawn_event_pump`
   is at `:190`); every shipped host renders audio into `NullBackend`
   (`pipeline_host.rs:1598-1599`); no window exists anywhere in the shipped lane.
3. **The instruments lie.** Four decoder registries in one binary answer 2 / 16 /
   12 / "none" (engine-info `host.rs:1643-1658`, bench `bench.rs:299-334`,
   landed-string `pipeline_host.rs:372-378`, dispatch `pipeline_host.rs:242-380`).
   `decode-bench` reported zero frames for the file the same binary just played.
4. **The paperwork lies both directions.** 14 rows overstate (asf "implemented",
   seek "partial"), 24 rows call shipped, tested code `not_started`
   (rows 084/095/099/100/101, ~10 UI rows), 18 rows hold real code no shipped
   binary can reach (the entire 25k-LOC subtitles crate), 46 contradictions.
5. **The gates cannot fail.** alloc-budget's `sys.exit(1)` is swallowed by
   trailing `say`s (executed repro, exit 0); clippy runs `continue-on-error:
   true`; the complexity ratchet baseline was never committed.

The goal hierarchy for this plan:

- **G1 (the only goal that matters now):** an MP4 plays with picture and sound
  on Windows, seek included.
- **G2:** the repo's status layer can be trusted again (one status layer,
  gates that can fail, evidence that regenerates).
- **G3:** the stranded inventory ships (subtitles, playlists, audio controls).

Open no new feature lane until G1 closes. This is the audit's rule and this
plan keeps it.

---

## 1. Principles (from DEVIL-AUDIT-REPORT §5 — how we work while fixing)

1. **One status layer, generated from the code**, refreshed by release CI.
   Delete or demote the other nine (matrix hand-edits, compat-matrix,
   parity-report, engine-info strings, landed-string, INFRA-BLOCKED,
   risk-register...). The status layer that drifted in both directions carries
   zero information; it gets replaced, not patched row by row forever.
2. **One decoder registry.** The dispatch table in `pipeline_host.rs` IS the
   registry; `engine-info`, `decode-bench`, and the refusal-string derive from
   it by code, not by hand-maintained lists.
3. **Every gate can fail or gets deleted.** Kill `continue-on-error`, swallowed
   exits, uncommitted baselines, gates that skip their subject.
4. **Every fix lands with three things:** the code, the test that would have
   caught the audit finding, and the regenerated status artifact. A fix
   without the test is a claim; this repo has enough claims.
5. **No doc claims a capability without a committed, regenerable command** that
   demonstrates it. Path-existence is not evidence (feature_audit R4 lesson).
6. **REAL% is re-scored at every phase exit** using the same verdict buckets
   (REAL / ACCURATE-ABSENT / SHALLOW / UNDERSTATED / UNREACHABLE / FALSE /
   REAL-UNPROVEN). The trajectory is a first-class project metric.

---

## 2. PHASE 0 — Stop the bleeding (Days 1–2, one engineer, ~1–2 days)

Small, independent, unblocks everything downstream. Every item is verified
already-static (the audit executed or re-derived each one).

| ID | Fix | Files | Size | Acceptance |
|---|---|---|---|---|
| F0.1 | E0425: rename `spawn_session_pump` → `spawn_event_pump` at the call site (or define the missing pump); remove the workspace `exclude` OR add a CI lane that actually compiles the crate; re-point INFRA-BLOCKED's "compiles" claim | `apps/desktop-tauri/src/app.rs:301`, root `Cargo.toml:51`, `.github/workflows/ci.yml:730-731` | 1 LOC + CI yaml | `cargo build --manifest-path apps/desktop-tauri/Cargo.toml` succeeds on a webview host; the CI lane compiles the app, not just the lib |
| F0.2 | Registry single-sourcing: `engine-info`, `decode-bench`, and the `landed:` refusal string derive from the dispatch table (16 arms) at runtime | `apps/cli/src/host.rs:1643-1658`, `bench.rs:299-334`, `pipeline_host.rs:372-378` | ~50–150 LOC | `engine-info` lists all 14 shipped families; `decode-bench sample.mp4` reports h264 v_frames > 0 for the file it plays |
| F0.3 | CLI `info` seekable: read the engine's `SeekSupport`, delete the `seekable: true` constant | `apps/cli/src/host.rs:1307` | <10 LOC | FLAC-in-WAV run: engine false → CLI prints false (W13-A02 disproof flips) |
| F0.4 | Panic-guard cluster (each 1–4 LOC + a crafted-input test): (a) TS-188 probe `prefix[TS_PACKET_BYTES]` OOB — `media-probe/src/signature/phase5.rs:159-161` and twin `demux/src/ts/mod.rs:315-322`; (b) MP4 stts/ctts unclamped loops — `demux/src/mp4/mod.rs:1253-1254, 1316-1317` (clamp like sibling stsz/stco); (c) MKV lacing inversion `frame_count = body[0] as usize` → `+1` and fix the 3 tests that pin the bug — `demux/src/mkv/cluster.rs:258`; (d) MKV window-slide livelock exit condition — `demux/src/mkv/mod.rs:644-663`; (e) adts `frame_length == 0` never advances — `apps/cli/src/containers/ts.rs:354-376`; (f) argv non-UTF8 panic — `apps/cli/src/main.rs:14` (`args_os`); (g) dash resolve div-by-zero x2 — `streaming/src/dash/resolve.rs:410,735`; (h) hls master usize underflow; timestamp_map i64 overflow; (i) ASS parser PlayResX+\pos overflow — `subtitles/src/ass/parser.rs:722`; (j) asf header.rs:558 child-size check; (k) mp3 xing VBRI `rest[14..21]`; (l) ogg flac second-header 38→34 bytes; (m) PS payload_len u64 underflow; (n) PGS 4 GiB retention cap + DVB stale-CLUT OOB | see column | ~20–60 LOC + tests | Every crafted input from the audit re-runs: 188-byte TS file → typed refusal (exit 65/0), no exit 101; stts 12-byte box parse bounded; `HELLO WORLD!` CC renders correctly after PAC range fix (0x10–0x1F) |
| F0.5 | Gates that can fail: (a) `23_alloc_budget.sh` — move `sys.exit(1)` after the `say`s / single exit path; (b) `rust-clippy.yml` — remove `continue-on-error: true`, add `-D warnings`; (c) commit the `26_complexity.sh` ratchet baseline; (d) wire `oracle_check.sh` into CI (6 unpinned fixtures red today); (e) drop `\|\| true` from miri/asan/ubsan lanes; (f) fix `dependabot.yml` stub + root `package-lock.json` invalid JSON | `scripts/quality/*`, `.github/workflows/*` | yaml/text | Each gate, fed a deliberately broken tree, goes red (prove it once in the PR) |
| F0.6 | Kill the six user-facing doc lies (C-001…C-006): video-h264.md "no software H.264 decode"; INFRA-BLOCKED "compiles"; PARITY-REPORT "billing-blocked / dies at 0 steps" vs six run IDs; network-http.md "CLI does not dispatch yet"; seek triple-lie (matrix + MediaInfo + CLI); the four-registry story | `docs/formats/video-h264.md:8-12`, `docs/PARITY-REPORT.md:122-125`, `docs/network-http.md`, matrix notes | text | `sed`-checkable: no shipped doc contradicts the binary; the seek row says "not implemented" until F1.3 lands |

Phase 0 exit: CI compiles the GUI crate; `decode-bench` and `engine-info`
tell the truth; every panic input from the audit is typed-refused; every
quality gate demonstrably fails on a broken tree; the six lies are gone.
REAL% does not move much (the rows' substance did not change) — but every
subsequent measurement becomes trustworthy. That is the point.

---

## 3. PHASE 1 — Picture and sound on Windows (Weeks 1–6)

The audit's minimum credible goal. Critical path: audio dominates.

### F1.1 — Windows audio output (the "sound" half) [L]

Two routes; **decision required on day 1**:

- **Route A — admit cpal (recommended):** flip `AUDIO_ADR005_ADMITTED`
  (`crates/audio-output/src/cpal.rs:43`), delete the compile-time assert,
  write the real cpal adapter (~200–400 LOC), land the ADR-005 amendment +
  dependency-allowlist row, replace `NullBackend::open_stream` at
  `apps/cli/src/pipeline_host.rs:1598-1599` under `cfg(windows)` (the
  desktop and tauri hosts chain through `CliHost` and inherit the fix).
  Days of work, not weeks — IF the Lead accepts the dependency.
- **Route B — finish WASAPI render:** the 787-LOC `platform-windows/src/windows/wasapi.rs`
  is enumeration/notification only (0 `IAudioClient` hits). Writing the
  render path (IAudioClient + IAudioRenderClient + event-driven mode +
  format negotiation + `AudioBackend` adapter) is +800–1,500 LOC of COM FFI
  plus hardware cycles. Choose only if cpal is policy-blocked.

Prerequisite ABI fixes either way (platform-windows is an orphan crate with
latent memory bugs; fix BEFORE wiring):
- PropVariant pinned at 16 bytes vs 24 on x64 (`com.rs`, W08-A10; executed
  `size_of=16`) — stack under-write class.
- dxgi vtables missing the 4 IDXGIObject slots; `get_desc1` actually calls
  `EnumOutputs` (`dxgi.rs`).

Also the "sound experience" knobs — part of the acceptance bar:
- `audio.set-volume` refused by every host; EQ math wired to nothing; mute and
  balance dead (W07-A02/A06). Wire volume + 3-band EQ (engine truth) into the
  pipeline; the UI's 10-band EQ must be reconciled (either extend the engine
  or cut the UI to 3 bands and say so).
- CoreAudio `ELEMENT_MAIN=1` vs master=0 property-address bug (W08-A02) —
  fix on the way past for the macOS lane.

**Acceptance:** on Windows hardware, `lumen open sample.mp4` produces audible
AAC audio; volume and mute work in-session; a crafted zero-length ADTS packet
does not wedge the stream (F0.4e).

### F1.2 — Verify + harden the frame lane on real hardware [M]

After F0.1, the lane exists end-to-end: engine → FrameTap →
`apps/desktop/src/mailbox.rs` (capacity 2 — the only backpressure) → 10 ms
ticker (`app.rs:109`) shipping `records.last()` only (`app.rs:223-229`) →
tauri Channel (raw bytes, ~3.7 MB per 1280×720 RGBA frame) →
`MediaSurface.tsx:227` `putImageData`. This is verification + backpressure
hardening, not new architecture:

- Instrument: frames produced vs frames dropped at the mailbox; end-to-end
  latency; channel stall behavior (a stalled webview must not OOM — the
  W14-A07 unbounded-queue class).
- Fix what the hardware run exposes: raise mailbox capacity or add a
  drop-policy + timestamp passthrough (no more `records.last()` silently
  dropping intermediate frames); confirm 10 ms cadence does not starve the
  webview on 4K.

**Acceptance:** user watches sample.mp4 end-to-end in the GUI on Windows with
a frame counter that matches the decode count (F0.2 makes `decode-bench` the
oracle).

### F1.3 — MP4 reanchor: make in-session seek true [M]

`Mp4Demuxer` has a fully-tested `seek()` (`mp4/mod.rs:2050`) and
`SeekSupport::Fast` (`:2039`); the engine's only seek entry calls the trait's
`reanchor` (`plugin-api/src/demux.rs:113-120`), which no demuxer implements —
every MP4 seek returns `plugin.demux.reanchor.unsupported` (executed:
`TimingSeekFailed`, position 0.0%, W20-A05 F-001). Implement `reanchor` on
`Mp4Demuxer`: commit-position + pending-flush + event re-priming on the
existing index machinery (~150–400 LOC + tests). WAV seek already completes —
it is the reference implementation of the contract.

**Acceptance:** `seek 1s` on sample.mp4 → `TimingSeekCompleted`, position
~1.0 s, playback continues; the C-003 triple-lie closes from the code side
(F0.3 + F0.6 closed the reporting side).

### F1.4 — Phase gate

On a Windows machine: open sample.mp4 in the shipped GUI → user sees frames,
hears audio, seeks, plays to end. `cargo test` green (now including the
desktop-tauri lane), every Phase 0 acceptance still green. This is G1 closed.
Estimated band: **~1–2 days to a compiled GUI (Phase 0 + F0.1); ~2–6
engineer-weeks to picture+sound**, dominated by F1.1.

---

## 4. PHASE 2 — Trust restoration (parallel with Phase 1, ~2 weeks)

Mostly independent of the audio work; run it concurrently.

- **F2.1 One status layer.** Replace hand-edited statuses with a generator
  that reads the code (dispatch table, wired verbs, plugin registry) and emits
  the matrix; CI regenerates and diffs — a drifted status fails the build.
  Sweep the 24 UNDERSTATED rows to truth (084 file, 095 http, 099/100/101
  reconnect/auth/remote-playlists ~4,900 LOC, 142 autostart, ~10 UI rows,
  148-151 config areas); the 14 FALSE rows get either code (asf → implement
  or mark absent honestly; seek closed by F1.3) or corrected status.
- **F2.2 Test evidence honesty.** Refresh the stale first-vertical
  transcripts (baseline: `committed_transcripts_match_fresh_runs` FAILED);
  run the journey/recovery_matrix packages the exit-101 abort skipped;
  pin the 88 unpinned ffmpeg-authored fixtures (CORPUS.md "authored in-tree"
  is false for ogg/HLS families); actually run the registered-but-never-run
  fuzz campaign; move the visual lane into the per-PR set (regression
  currently ships ≤24 h undetected); resolve the 3 shipped WCAG-AA contrast
  exclusions or document them as accepted.
- **F2.3 CI gate overhaul.** Every quality lane fail-closed (Phase 0 did the
  top 6; finish the tail: `|| true` leftovers, J-reporters whose exit codes
  never propagate, `25_fuzz_campaign` fail-open). Evidence logs are
  regenerated by committed commands, not snapshotted; delete the stale-green
  wave3-merge/phase-exit logs or regenerate them truthfully; re-sign or remove
  the backdated phase3-exit signature; remove the `resolve_w2_scheduler.py`
  booby trap (re-run silently deletes 9 code lines).
- **F2.4 Evidence culture.** w26 verification captures live in /tmp and
  .gitignore — commit them or reproduce their generation in CI. The repo's
  strongest paperwork (VTA vp9 bit-exact verdicts) becomes reproducible.

**Exit:** re-run the Devil Audit's verdict methodology on the matrix;
target: no FALSE rows remain except honest-blocked ones; UNDERSTATED → 0;
REAL% re-scored and published in-tree.

---

## 5. PHASE 3 — Land the stranded inventory (after G1, 3–6+ weeks)

Ordered by user value per LOC already written (the code exists and is
tested — the wiring is missing).

- **F3.1 Subtitles (the single biggest win).** 25,416 LOC, 267 tests,
  zero shipped consumers (rows 057–068: srt/vtt/ass/pgs/dvb/cc/vobsub +
  rasterizer + encoding-detection all UNREACHABLE). Wire: CLI verbs +
  Frame::Subtitle render path + GUI subtitle track picker; **font loading is
  the hidden cost** (today all glyphs render as solid blocks — no font
  backend); CC PAC range fix ships with F0.4n's cluster. Estimate: weeks,
  dominated by the rasterizer/font lane, not the parsers.
- **F3.2 Audio controls.** Volume/EQ/mute/balance end-to-end in UI + engine
  (F1.1 did the pipeline seam; this is the UI polish + persistence).
- **F3.3 Playlists and hotkeys.** m3u/pls parsing exists (rows 077/078
  UNREACHABLE); wire to open + GUI playlist panel. Hotkeys: 143 of 150 are
  unbound (W11) — bind the top 20 (Space/P/S/N/M/ arrows/seek).
- **F3.4 Restore the false-marked playback features.** frame-step
  (UNREACHABLE, W3), bookmarks (mock-only), snapshot (row 110), chapter-title
  — each is an engine feature with no UI/verb.
- **F3.5 HEVC decision.** sample-hevc exits 65 (no decoder, honestly
  refused). Either implement (large: new codec lane) or keep refusing and
  fix the matrix row's expectations (it currently implies more than the
  binary does — decide and document).

---

## 6. PHASE 4 — Beyond first playable (roadmap, not scheduled)

HLS/DASH real wiring (m3u8 currently dies silently in probe; the streaming
crate has 4,900 LOC + 212 passing tests and two S1 panics already guarded by
F0.4) · ASF probe or delete the row · web lane honesty (browser wasm has zero
decoders — label it a demo or port codecs) · hardware H.264 decode path
(row 022) · renderer-wgpu rename or actually use wgpu (it is pure software
compositing today — a name lie) · f32 tolerance failures vs VLC (6/6) from
the perf lane.

---

## 7. Sequencing, effort, trajectory

```
Week 0          : Phase 0 (1-2 days)  ── stop the bleeding
Week 1-6        : Phase 1  ┌ F1.1 audio (dominant, start day 1)
                            ├ F1.2 frame lane (needs F0.1)
                            └ F1.3 reanchor (independent)
  parallel      : Phase 2  └ trust work (independent of audio)
After G1        : Phase 3  └ stranded inventory (subtitles first)
Later           : Phase 4  └ roadmap
```

| Phase | Effort band | Gate |
|---|---|---|
| 0 | 1–2 engineer-days | all Phase 0 acceptance items green |
| 1 | 2–6 engineer-weeks | G1: MP4 picture+sound+seek on Windows |
| 2 | ~2 engineer-weeks (parallel) | 0 FALSE / 0 UNDERSTATED rows; gates provably fail |
| 3 | 3–6+ engineer-weeks | subtitles render; controls work; REAL% ≥ 55% |
| 4 | unscheduled | roadmap |

Functional LOC estimate (from REMEDIATION-MAP §3, audit-derived):
~500–2,400 LOC + mandatory Windows hardware cycles; audio dominates.

**REAL% trajectory (same audit methodology, re-scored at each exit):**

| Milestone | Expected REAL% | Why |
|---|---|---|
| Today | 19.44% | measured (49/252) |
| Phase 0 exit | ~20–22% | rows unchanged in substance; instruments now honest; a few FALSE rows (registry story) resolved |
| 2026-11-26 interim — Phase-1 code-side exit, post-FX4/FX5/FX6 | **20.63% — REAL 52/252, floor 20%** (the re-score's home: `audit/fix/REALITY-RESCORE.md`, FX7-A03, Devil-Fix wave FX7) | measured with the same verdict buckets over the same 252-row denominator; 4 rows flip — mp4-seek SHALLOW→REAL (FX5-A01/A04/A06/A07), volume FALSE→REAL and EQ FALSE→REAL (FX3-A03 + FX4-A01/A02/A05 + FX5-A02/A03 + FX4-A08/FX5-A05), resampler ACCURATE-ABSENT→UNDERSTATED (FX3-A04 + FX4-A01, the conservative direction); the frame lane owns no matrix row (FX6-A06's guard) and W1-06's closure was not in the tree at re-score time, so nothing of the frame lane is counted as landed; row recorded 2026-11-26 by the docs pass FX7-A04 (same wave) |
| Phase 1 exit (G1) | ~20–22% — **re-projected 2026-11-26** (FX7-A03's REALITY-RESCORE §7, carried to FX7-A04) from the original ~30–35% estimate; G1-closed projection ~21.0% (53/252 — row 169 becomes a REAL candidate) | the original band's own listed clauses never supported it: the "several SHALLOW playback rows" are F3.4/Phase-3 scope deferred by the G1 sequencing rule, and hardware-dependent rows count REAL-UNPROVEN until the Windows run (this interim is the code-side exit, pre-G1); the first two clauses matched by name and landed — mp4-seek flips TRUE→REAL (FX5), volume/EQ rows become REAL (FX3/FX4/FX5) |
| Phase 2 exit | ~40–45% | 24 UNDERSTATED rows re-verified as REAL; FALSE rows resolved to truth |
| Phase 3 exit | ~55–60% | 18 UNREACHABLE rows gain shipped consumers (12+ subtitle/playlist rows flip to REAL) |

These are estimates with the same honesty rule as the audit: floor, never
round up, and re-measured — not promised — at each gate.

---

## 8. Governance — how this never happens again

1. **Definition of done for every fix** (per Principles §1): code + the test
   that would have caught the audit finding + regenerated status artifact.
2. **REAL% is a release gate**: the status generator runs in release CI; a
   matrix row whose status disagrees with the generated truth fails the build.
3. **The contradiction register stays alive**: any PR that introduces a
   doc-vs-code contradiction (auditable by the same sed/rg checks the audit
   used) is rejected in review.
4. **No parallel status layers** (Principle 1) — the failure mode this audit
   proved is drift, and drift is what nine layers guarantee.
5. **Token hygiene**: the GitHub token used for this audit's pushes was
   exposed in chat during the engagement. Revoke it and issue a scoped one
   per integration. Treat chat history as public.

---

## Appendix — work-item register (ordered, with dependency)

| # | Item | Depends on | Files | LOC |
|---|---|---|---|---|
| 1 | F0.1 E0425 + CI lane | — | app.rs:301, Cargo.toml:51, ci.yml | 1+yaml |
| 2 | F0.2 registry single-source | — | host.rs, bench.rs, pipeline_host.rs | 50–150 |
| 3 | F0.3 seekable truth | — | host.rs:1307 | <10 |
| 4 | F0.4 panic guards ×14 | — | see §2 table | 20–60 |
| 5 | F0.5 fail-closed gates ×6 | — | scripts/quality, workflows | yaml |
| 6 | F0.6 six doc lies | — | docs/* | text |
| 7 | F1.1 audio route decision + adapter | F0.1 | cpal.rs / wasapi.rs, pipeline_host.rs:1599 | 200–1,500 |
| 8 | F1.1b PropVariant 24B + dxgi slots | (with 7) | com.rs, dxgi.rs | small |
| 9 | F1.1c volume/EQ wiring | 7 | pipeline_host, UI | M |
| 10 | F1.2 frame lane hardening | F0.1 | mailbox.rs, app.rs, MediaSurface.tsx | 50–300 |
| 11 | F1.3 MP4 reanchor | — | mp4/mod.rs, plugin-api | 150–400 |
| 12 | F2.1 status generator | F0.2 | xtask, docs/feature-matrix.toml | M |
| 13 | F2.2 test evidence | — | tests/*, fixtures | M |
| 14 | F2.3/F2.4 CI + evidence | F0.5 | workflows, docs | M |
| 15 | F3.1 subtitles + fonts | G1 | subtitles/*, CLI verbs, GUI | L |
| 16 | F3.2–F3.4 controls/playlists/hotkeys | G1, 9 | — | M each |
| 17 | F3.5 HEVC decision | — | — | decide |

Total functional: ~500–2,400 LOC before G3; the plan's cost is dominated by
audio (F1.1) and subtitles (F3.1), exactly as the remediation map sized them.
