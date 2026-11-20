# DEVIL AUDIT — EXECUTIVE REPORT

Audit branch: `audit/devil-20261116` (pinned to audited commit `727ad6b2`, w31.6).
Audit date: 2026-11-16 (host: Linux, 2 CPU / 4 GB / no display; all claims below are
executed-run outputs or sed-verified citations; raw logs under `audit/runs/`).

## The one-sentence truth

**A 3-second H.264+AAC MP4 opens, demuxes, decodes and plays to 100% in the shipped CLI on
this host — with no sound on any OS, an unproven GUI display path, a broken MP4 seek, and a
documentation/matrix layer that is one to seven waves stale in both directions.**

## The headline numbers

- **REAL (executed end to end by this audit): 4 / 252 matrix rows = 1.6%**
  (MP4 container, H.264, H.264-software, AAC-LC — all on linux; Windows/macOS cells of the
  same rows: REAL-UNPROVEN).
- REAL-UNPROVEN (suite-passed, not individually executed): 11 rows (6.0% cumulative).
- SHALLOW (implemented cells backed only by path-existence evidence): 52 rows (20.6%).
- Repo's own parity figure: 66/252 windows (26.2% implemented∪verified) — a tier that this
  audit proved is certified to a standard that passes a **0-byte file** as 4-platform evidence.
- verified-only parity: **0/252 on every platform** (executed) — zero rows anywhere carry
  `verified`; v0.1.0–v0.1.2 shipped on the weaker tier.
- Docs layer (8 key docs, 180 checkable claims): REAL 91 · REAL-UNPROVEN 35 · SHALLOW 8 ·
  PAPER-ONLY 6 · FALSE 23 · UNVERIFIED 17 — **12% of doc claims are FALSE, all in the
  status stratum; zero fabricated results; seventeen stale ones.**

## Wave 0 MP4 smoke, verbatim

```
$ printf 'advance 512\ntime\nquit\n' | ./target/release/lumen open audit/runs/baseline/sample-h264-aac.mp4
...
event: tracks 2 ready
event: TimingDurationKnown { duration: Timestamp(3000000) }
event: MediaInfoReady { ... h264 (Video), aac (Audio) ... seekable: true }
event: state-changed Buffering → Playing
event: end-of-media
event: state-changed Playing → Ended
ok: time 3.041814s (100.0%), 1 buffered range(s)
EXIT=0
```
Sample: ffmpeg testsrc 320x240 H.264 + sine AAC, 3.000s, 45,682 bytes.
(First-pass note, recorded for honesty: an initial smoke that fed only `quit` produced a
quit-triggered `Stopped`, and the stale `decode-bench` allowlist printed "no decoder in
this build" — both artifacts are audit findings S2/S3, not evidence of decode failure. The
conformance-protocol rerun above is the executed truth.)

## Top 25 findings (full detail with verbatim quotes in the wave summaries)

1. **[S0] Evidence standard = path existence** — `feature_audit.rs:489-501`; empty-file experiment PASS (exit 0). All 193 "implemented" cells are certified to this.
2. **[S1] No audible audio anywhere** — null-only, discard-mode (`audio-output lib.rs:152-157`, `pipeline_host.rs:1555-1556`, cpal admission closed).
3. **[S1] engine-info lies about the build** — static 2-decoder vec (`host.rs:1452-1482`) while the binary decodes H.264+AAC; **pinned as the expected test answer** (`command_surface.rs:125-133`).
4. **[S1] stts/ctts unclamped loops** — crafted-moov CPU DoS (`mp4/mod.rs:1253-1266`, `:1314-1332`).
5. **[S1] Track-id space split** — chapter-trak files deliver packets on unlisted TrackIds (`mod.rs:833-839` vs `:1882`).
6. **[S1] Backward seek serves a stale cursor** — `order_cursor` never reset (`mod.rs:1571-1577`, `:2129-2141`).
7. **[S2] decode-bench cannot bench the shipped codecs** — stale allowlist + false "no decoder in this build" message + `extra_data: None` seeding gap (`bench.rs:300-334`, `:427-446`).
8. **[S2] elst empty-edit units bug, pinned by its own test** — movie-timescale stored in a media-ticks field (`mod.rs:1090-1094`); `tests.rs:196-197` asserts the wrong value — fails-if-fixed.
9. **[S2] I420 window-full hold stashes unconverted frames** — surface freezes on early window frames (`pipeline_host.rs:1314-1341`, self-admitted).
10. **[S2] Mixed progressive+fragmented MP4 silently drops progressive samples** (`mod.rs:779-813`).
11. **[S2] Depth guard is dead code while its doc claims it exists** (`error.rs:81-90`, `MAX_BOX_DEPTH` never referenced).
12. **[S2] GUI runtime display unproven** — no runtime pixel evidence in the tree; GUI frame fixtures MJPEG-era only; `[VERIFY]` marker unresolved (`tauri-transport.ts:92-98`).
13. **[S2] PARITY-REPORT "result: PASS"** vs executed feature-audit FAILURE (2 dangling evidence paths, rows 23/163).
14. **[S2] check_architecture duplicate-package scan misses non-adjacent pairs** (`check_architecture.rs:990-993`).
15. **[S3] CI-billing schism** — release.yml:26 present-tense "NEVER EXECUTED" vs RELEASE.md "unblocked 2026-11-12"; one side false either way (undecidable in-tree).
16. **[S3] KNOWN-LIMITS "no canvas exists in ui/src"** — FALSE at HEAD (`MediaSurface.tsx:174`).
17. **[S3] Fabricated snapshot fields** — `seekable: true` and `buffered=[0..duration]` hard-coded (`host.rs:1128`, `:1172-1178`); MP4 seek actually typed-refuses.
18. **[S3] help.rs:57 seek line false** — describes reopen semantics that do not exist.
19. **[S3] PlayerScreen.tsx:36-42 stale block** — "DECODE/FRAMES half is engine-wired nowhere" while mounting the canvas two blocks below.
20. **[S3] ROADMAP pins 61 commands / phantom commit `2d06607`** — live catalog 65; the revision doesn't exist.
21. **[S3] Yanked crates in the shipped lockfile** — wasmi 2.0.0 / wasmi_core 2.0.0 (cargo deny advisories FAILED, executed).
22. **[S3] Two workspace tests RED** — control-server catalog 65 vs pinned 64 (executed).
23. **[S3] gate.rs's 10-command gate is 9** — `WASM_BINDING_CRATE` points at a nonexistent crate (`gate.rs:107-110`).
24. **[S3] 32.65 fps frame-ship ceiling** — one-frame-per-tick newest-wins decimation, uncounted (desktop-tauri).
25. **[S3] Config-validator doesn't enforce patterns** — "validated" weaker than the name (`config_validator.rs:16-18`).

## Process failure analysis — how the paperwork outran the product

The program that produced this repo had 15 planning docs, a 252-row matrix, a risk register,
wave discipline, and an auditor (`feature-audit`) that checks that evidence strings name
existing paths. What it did not have was **any mechanism that forces a status surface to be
true**: the same wave that lands code does not refresh the docs that describe it (17 stale
claims across 8 docs, each lagging a specific wave), the self-describing surfaces
(engine-info, decode-bench) froze at the first vertical and were then **pinned by tests**
(two separate tests now assert stale values as expected answers — the test suite defends
the lie), and the one honest strict metric (verified-only 0/252) never gated a release:
v0.1.0–v0.1.2 shipped anyway. The result is a repo where the code is roughly one wave ahead
of the paperwork in the direction that flatters (capabilities it has stopped describing) and
the paperwork one wave ahead of the code where it flatters (statuses the evidence standard
cannot certify). The owner's complaint — "an MP4 cannot be opened properly" — was true of
the era the docs describe; at HEAD the CLI plays the MP4 and **nobody updated the claim**, so
the owner has no way to know. A paperwork program that never re-derives its own numbers is a
rumor mill with lint gates.

## What to stop doing

1. Stop shipping releases on implemented-tier evidence while the verified counter sits at
   0/252 and calling the result "parity".
2. Stop pinning self-descriptions in tests (two tests now defend stale facts).
3. Stop letting any wave land code without the matching status-surface refresh in the same
   commit (the change-control rule the matrix header already declares — enforce it).
4. Stop generating new planning documents; the existing 537-doc tree is 7 days stale at best.

## Minimum credible next goal

One automated run on a real OS (Windows, the owner's platform): open the H.264+AAC MP4 in
lumen-tauri, assert rendered frames (pixel hash), assert an audible sine on the default audio
device, publish the run. Everything needed is enumerated in
`audit/final/REMEDIATION-MAP.md` (B1 audio backend, B2 display proof, B4 self-description
refresh, B5 demux hardening). That single green run is worth more than the next fifty
planning docs.

## Coverage statement (this run vs the 200-agent spec — read this before using the numbers)

Executed this run: Wave 0 (full pre-flight: manifest 3,226 files, baseline runs, MP4 smoke),
Wave 1 (9 xtask files + 2 hostile experiments), Wave 5 (6 MP4 demux files, the priority
wave), Wave 13 (19 CLI/GUI/audio files, full argv→canvas traces), Wave 18 (252-row matrix
claim ledger), Wave 19 (8 key docs, 180 claims), Wave 20-lite (citation sweep 17/17 faithful,
5 final deliverables). **42 source/doc files audited at depth + the full matrix + the full
doc-status layer.**

NOT executed at spec depth this run (assignments ready in `audit/waves/wave-00/assignments.tsv`):
waves 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17 domains (engine/pipeline internals,
other demuxers/codec crates, subtitles, streaming, settings, plugins, remaining UI screens,
the full test corpus, CI/packaging scripts) — ~3,180 of 3,226 files not yet file-by-file
audited. The 1.6% REAL number is therefore a **lower bound under this run's coverage**;
51 SHALLOW rows may upgrade on audit. Continuation: re-run §9 agents from assignments.tsv;
the gates, templates and evidence standards are all in the committed audit/ tree.

## Appendix: all executed commands' raw outputs
`audit/runs/baseline/*` (build, test 1001/2/20, clippy, fmt, deny, feature-audit, parity,
config-validator, architecture, UI typecheck/vitest/build, rc-smoke, both MP4 smokes, the
empty-file experiment), per-wave summaries under `audit/waves/`, verification under
`audit/verify/`.
