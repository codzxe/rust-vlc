# ROADMAP — Phase Gates, Wave Program & Decision Register

Authority chain: `docs/MASTER-PROMPT.md` (the brief) → this roadmap (program-level
plan & decisions) → domain docs 01–15 (detail). Conflicts resolve upward.
Statuses per MASTER-PROMPT §17: not_started, in_progress, implemented, verified,
blocked.

**Status — phase states** (supersedes the §8 historical snapshot):

- **Phase 0 — COMPLETE** (`docs/gates/phase0-exit-report.md`).
- **Phase 1 — COMPLETE** — PASS 2026-10-20, local-evidence; 3-OS CI green-run
  links PENDING first push (`docs/gates/phase1-exit.md`).
- **Phase 2 — COMPLETE** — PASS 2026-10-21, local-evidence
  (`docs/gates/phase2-exit.md`).
- **Phase 3 — complete in conduct** — gate evidence record
  `docs/gates/phase3-exit.md` (2026-10-24, card 3.F2-V); its verdict line
  still reads **PENDING LEAD SIGNATURE** while Phases 4–6 proceeded on its
  evidence — the signature line is owed a Lead square-up, not a re-run.
- **Phase 4 — complete in conduct** — same shape:
  `docs/gates/phase4-exit.md` (2026-10-24, card 4.I1-V), verdict line
  PENDING LEAD SIGNATURE.
- **Phase 5 — COMPLETE** — **PASS WITH REGISTERED CAVEATS**, Lead sign-off
  2026-10-24 (`docs/gates/phase5-exit.md`; 47/47 parent cards resolved;
  "Phase 5 is closed. The Lead authorizes Phase 6.").
- **Phase 6 — Network & streaming — IN PROGRESS** (worklog 6.* through
  6.W4-STATUS, 6.FIX-pcm-stall, 6.FIX-ogg-reanchor; `crates/streaming`
  landed — HTTP(S) client via ureq 3.5.0 + ring exception per 6.A1b-RATIFY,
  ranged seekable resource, scheme registry, HLS VOD + live window, DASH MPD
  parser core, open-path container wiring; buffering/reconnect and the
  6.D/6.E–6.I tails remain).
- **Phases 7–8 — NOT STARTED** (no worklog task IDs, no gate records).

Header last refreshed: 2026-11-09 (issue #21).

## 1. Phase gates (MASTER-PROMPT §16)

Each gate passes only with recorded evidence on every claimed platform
(Windows, Linux, macOS; web column tracked per §10). Wave/card references are
from `docs/13-agent-wave-program.md` (200 cards / 77 waves).

| Phase | Objective | Waves | Cards | Exit-gate evidence (summary) | Owning docs |
|-------|-----------|-------|-------|------------------------------|-------------|
| 0 — Audit | Pin VLC 3.0.23, full feature inventory, feasibility report, ADR-001/002/003 decided | 0.A–0.D | 17 | `docs/baseline/` pinning records; `docs/feature-matrix.toml` populated with schema frozen; `docs/feasibility-report.md`; ADR-001/002/003 status=accepted; [VERIFY-P0] ledger cleared or converted to risks | 02, 04, 11, 14 |
| 1 — Foundation | Workspace, config system, schemas, lints, CI, command layer, plugin API, settings generator, token pipeline | 1.A–1.F | 20 | **PASS 2026-10-20 — local-evidence; 3-OS CI PENDING (workflows-disabled, PAT repo-scope only; phase1-exit.md:6-8)**: local gate green (exact commands, doc 10 §2; 529/0/3); CI merge-blocking jobs written + locally validated, first push owed; rebrand acceptance test passes; boundary check active; `cargo xtask gate` exists | 01, 05, 06, 07, 10, 15 |
| 2 — First vertical | One real file open→decode→audio+video on screen via command layer, from CLI and UI; video-in-webview prototype; Figma token proof | 2.A–2.E | 17 | **PASS 2026-10-21 — local-evidence; 3-OS CI PENDING (same PAT cause; phase2-exit.md §4.1)**: demo fixture plays end-to-end through the command layer, CLI-hosted, with byte-stable mock-labeled evidence (real-hardware tier + per-OS runs pending); webview prototype delivered — variant-B protocol core proven at virtual-time level, PROVISIONAL-B, per-OS matrix pending desktop runtimes; token pipeline renders Player screen from tokens (landed 1.B4) | 03, 04, 05, 08 |
| 3 — Reliable engine | Clocks, queues, generations, seeking, source replacement, A/V sync, recovery | 3.A–3.F | 19 | All 16 integration scenarios (doc 10 §6) green incl. error paths; property suites green; soak 8h no-growth; differential-vs-VLC harness v1 reporting — **gate record: `docs/gates/phase3-exit.md` (2026-10-24, card 3.F2-V); verdict line reads PENDING LEAD SIGNATURE (see Status block above)** | 03, 10, 12 |
| 4 — Desktop product | Figma-exact UI, layouts from config, themes, localization, playlists, settings, plugins, rebrand test | 4.A–4.I | 25 | Screenshot diffs within tolerance vs design exports at defined sizes/DPI; all 13 settings areas driven by settings-schema; i18n lint zero violations; conformance suite green on Tauri host — **gate record: `docs/gates/phase4-exit.md` (2026-10-24, card 4.I1-V); verdict line reads PENDING LEAD SIGNATURE (see Status block above)** | 07, 08, 05 |
| 5 — Breadth | Containers, audio+video codecs, subtitles, filters — one verified increment at a time | 5.A–5.R | 45 | Per-format records complete for every shipped format; conformance kit + fuzz target per parser plugin; corpus coverage; feature-matrix statuses updated by evidence only — **PASS WITH REGISTERED CAVEATS, Lead sign-off 2026-10-24 (`docs/gates/phase5-exit.md`; 47/47 parent cards incl. delta cards 5.X1/5.X2; caveats honestly registered there §4)** | 04, 06, 10 |
| 6 — Network & streaming | HTTP(S), HLS, DASH, RTSP-where-possible, buffering, reconnect, auth, fuzzing | 6.A–6.I | 17 | Network scenarios (interrupted/redirect/auth) green; protocol fuzzers in CI; blocked items (raw UDP/RTSP etc.) recorded with reasons — **IN PROGRESS (2026-11-09, see Status block)** | 03, 04, 11 |
| 7 — Platform & advanced | Devices, hw decode (per ADR-001), recording, conversion, stream output, WASM extensions, web parity | 7.A–7.L | 24 | Hw-decode active-decoder truthfulness test green per OS; WASM sandbox conformance + crash isolation green; web build plays real media via same command layer on Chrome/Firefox/Safari (recorded matrix) | 09, 06, 03, 11 |
| 8 — Hardening & release | Compatibility, security, soak, perf vs VLC, packaging, honest parity report | 8.A–8.H | 16 | Language-policy audit pass; SBOM + advisories clean; final perf report vs VLC 3.0.23 on reference hardware; installers signed/launch-tested; `parity-report` states verified/partial/unsupported/blocked per §17 | 11, 12, 10, 15 |

## 2. Wave program summary

- **200 task cards in 77 waves** (per-phase: 0:17, 1:20, 2:17, 3:19, 4:25, 5:45, 6:17, 7:24, 8:16). All cards S/M sized — L is a forbidden pre-launch state (split instead).
- Parallelism 2–6 agents per wave, bounded by conflict rules R1–R7 (doc 13 §1): no two agents on the same crate/interface; Lead owns shared surfaces (`media-types`, `codec-api`, `commands`, `plugin-api`, `product.toml`, schemas).
- Wave cadence: launch → verify outputs → Lead resolves conflicts → commit with attribution → update worklog → next wave. 77 nominal waves, ~96–115 with rework factor.
- Timeout/failure protocol proven in Wave 1 (1.A2 precedent: agent times out after writing its file → Lead verifies partial output, appends worklog on its behalf, re-scopes any retry).

## 3. Decision register (Lead decisions — binding unless revisited via ADR)

| ID | Decision | Rationale | Supersedes / affects |
|----|----------|-----------|---------------------|
| D-001 | **First vertical (Phase 2) = AVI + MJPEG + PCM** | Minimal external dependencies (priority #2: auditable deps); simplest full-control container (RIFF) exercising our own demux infrastructure and first fuzz target; doc 03's state machine designed around it. MP4 lands in Phase 5 via audited `mp4parse`. | Resolves doc 03 (AVI) vs doc 04 (MP4) conflict; doc 13's dual card 2.x marked — execute AVI card, keep MP4 card for Phase 5. |
| D-002 | **Settings areas = 13** (global, interface, privacy, playback, video, audio, subtitles, controls, library, network, performance, recording, extensions) | Actual distinct areas in the design package; `-1` SVG variants are states of the same area, not extra areas. | Resolves doc 07 Q1; `design/README.md` corrected (was mislabeled 14). |
| D-003 | **Keymaps: adopt VLC 3.0.23 default hotkeys as baseline where clean-room-observable, overridden by `design/figma/Keyboard Shortcuts.svg` where it specifies**; paths `keymaps/*.toml`, binding-id indirection per doc 05 §8 | Clean-room: hotkey maps are observable behavior, not code. Design package is the visual source of truth. | Resolves doc 05 §8 vs doc 08 keymap conflict; concrete seed table = Phase 1 card 1.E. |
| D-004 | **PluginKind = 16 kinds** (doc 06 split) | Finer taxonomy aligns traits with pipeline positions; doc 01's 14-variant enum folds SubtitleRenderer/PlaylistExporter ambiguities. | doc 01 §2 plugin-api updated at Phase 1 implementation time. |
| D-005 | **Task-ID grammar ratified: `<phase>.<wave><seq>` + `-I` (investigation) / `-V` (verification) suffixes; `not-selected` is a valid card status** | Operationalizes AGENTS.md §5 at scale; needed for decision-pair cards (e.g. webview A/B). | doc 13 §13 Q1/Q2 answered. |
| D-006 | **Working name "Lumen"** stays a placeholder; trademark collision search pending [VERIFY, risk R-36-class]; fully rebrandable via `product.toml` | No identity claims before the check; rebrand acceptance test (doc 07 §8) proves configurability. | README.md, product.toml draft. |
| D-007 | **Baseline artifacts live in `docs/baselines/`** (plural) | Standardizes doc 02 (`docs/baseline/`) vs doc 12 naming. | docs 02, 12 path references. |
| D-008 | **ADR timing: ADR-001 (hw decode), ADR-002 (WASM runtime), ADR-003 (video-in-webview) all decided inside Phase 0** per §16; ADR-004 (OS webview) accepted-by-brief, **file formalized 2026-10-22: accepted with per-OS verdicts PENDING** (`docs/adr/ADR-004-os-webview.md`; seam implemented in 2.D3); ADR-005 (audio backend) promised at Phase 1 wave 1.C, **file formalized 2026-10-22 from the 2.C3 draft: adapter-seam accepted, real-backend admission PENDING** (`docs/adr/ADR-005-audio-output.md`); ADR-006 (repo license) **decided 2026-10-19: MIT — see D-009** | Follows MASTER-PROMPT §16 Phase 0 gate. The ADR-005 delay (1.C → 2.C3 draft → 3.DOC-1 file) is honestly recorded: the seam was decided-by-implementation in 2.C3 and carried the 2.E2-V gate as "ADR-005 draft". | doc 01 §7 ADR register. |
| D-009 | **Repository license = MIT** (user decision 2026-10-19; `LICENSE`; `docs/adr/ADR-006-license-mit.md`; closes risk R-05) | Maximal permissiveness, GPL-compatible ecosystem fit, matches the permissive-only dependency rule already enforced. Trade-off noted: no explicit patent grant (Apache-2.0 alternative considered); revisit only if patent exposure grows (ADR-010). | doc 11 §3 (allowlist license column now MIT-compat), §8.2; doc 14 R-05. |
| D-010 | **R-07 closed: design package cleared per user attestation 2026-10-19** — open-source Figma Community design, no terms restriction | User statement; `design/figma/` remains the pinned visual source of truth for the fidelity program. Residuals: font identities remain [VERIFY] (exports are outlined); trademark wording rules (R-38-class) still govern shipped strings. | doc 14 R-07 closure log; doc 08 §2; `design/README.md`. |
| D-011 | **ADR-011 (TLS/HTTPS) opened as proposed** — no production pure-Rust HTTPS stack exists (rustls→aws-lc/ring = C; native-tls = OpenSSL); decision due ≤ Phase 5 gate; no HTTPS dependency merges until decided. **Update (Phase 6, 6.A1b-RATIFY): decision EXECUTED** — ureq 3.5.0 admitted with ring 0.17.14 as an ADR-001-class recorded exception = the streaming layer's HTTP(S) foundation (worklog 6.A1b-RATIFY); **decision record refreshed 2026-11-09, pending Lead admission** (issue #8: fresh rustls provider audit + typed https seam; `docs/adr/ADR-011-tls-https.md`) | Phase 0 blind-spot found by 0.C3-I; affects Phase 6+ only. Options + interim posture in the ADR brief. | `docs/adr/ADR-011-tls-https.md`; doc 11 §6 (SEC-07); risk register R-44-class. |
| D-012 | **doc 13 amendment due before Phase 5: add filter-implementation cards** (EQ/compressor/spatializer/stereowiden audio panels + colors/crop/essential video panels — all self-build S/M) | Gap found by 0.C3-I: doc 13 §7 Phase-5 waves contain no filter cards despite the design package requiring the panels. | doc 13 §7; doc 03 §4 filters; feasibility/sections/subsystems.md §4. |
| D-013 | **doc 04 §4 "no pure-Rust H.264/HEVC decoder" is superseded by 0.C1-I finding** — a young 2026 wave exists (rusty_h264-decoder 0.16.1, rust_h264 0.4.0, rusty_h265, rusty_vp9; tarball-scanned pure, all self-validated) | Feeds ADR-001's conformance-gated software-adoption track; no status upgrade without passing our corpus+fuzz+differential kit. | `docs/feasibility/sections/video-codecs.md`; `docs/adr/ADR-001-hardware-decode.md` §2. |
| D-014 | **ADR-001/002/003 accepted at Phase 0 gate** (hw-first H.264/HEVC + gated software track; wasmi WASM runtime; per-OS video-in-webview strategy) + Lead ratifications R-1..R-5 (Debian baseline package; expired-key GPG accepted as pin-sufficient; hardware-run owners assigned) | Per MASTER-PROMPT §16 Phase 0 mandate; evidence packs cited in each ADR. | `docs/adr/ADR-{001,002,003}*.md`; `docs/gates/phase0-exit-report.md`. |
| D-015 | **Branch-commit rights granted to implementation agents on WIP branches** (user directive + sandbox-instability evidence: reboots reset the working tree to main; commits survive). Merge to main stays Lead-only. | AGENTS.md §3.8 deviation recorded in the worklog (1.SESSION-3-START); proven across Phases 2–3 (salvage protocol 11-for-11). | AGENTS.md §3.8; `docs/gates/phase1-exit.md` §4. |
| D-016 | **xtask registry graduation** — all six wave-1 subcommands flagged `implemented: true` (Lead batch-flip at the 1.D4 merge, commit `2d06607`); the v0 "only gate" invariant test graduated with them | The frozen-registry discipline (1.E1 flag b) resolved by Lead batch-flip rather than per-command edits. | `tools/xtask` registry; `docs/gates/phase1-exit.md` §4. |
| D-017 | **The command catalog count is 61 commands / 23 events** — doc 05's §2.3–§2.12 + §3.2 tables are the authority (the "~65/~24" in the card text was an approximation); `catalog_shape.rs` pins the exact counts | Kills a recurring approximation drift between card text and the implemented catalog. | `crates/commands` catalog_shape; `docs/gates/phase1-exit.md` §4; doc 05. |
| D-018 | **First-vertical container = AVI+MJPEG+PCM, all in-house, zero new dependencies** (decided at the 2.B1+2.A3 close-out, 2026-10-21; executes D-001's Phase-0 pick) | D-018's recorded logic: zero new deps for the first vertical; simplest full-control container exercising our own demux infrastructure; MP4 deferred to Phase 5 via the audited mp4parse path (card 2.B3 closed `not-selected`). Closes risk R-28. | doc 03 §7; doc 04 §8.1; `docs/gates/phase2-exit.md` §2/§3; feature-matrix rows container-avi / container-mp4-mov-3gp. |

## 4. Cross-doc reconciliation log (Wave 1)

| Conflict | Docs | Resolution |
|----------|------|------------|
| First vertical container | 03 vs 04 | D-001 (AVI) |
| Settings area count | 07 vs design/README | D-002 (13) |
| Default keymap seed | 05 §8 vs 08 | D-003 |
| PluginKind count | 01 vs 06 | D-004 |
| Baseline dir naming | 02 vs 12 | D-007 |
| `ts_ns` event timestamp proposal (doc 12 needs it for metrics) | 12 vs 05 | Accepted as [PROPOSAL] → Phase 1 command-layer card must include `ts_ns` in the envelope + `system.get-stats` decision (card 1.C scope note). |
| Doc 09 written before doc 08 landed | 09 §12 flagged | Non-blocking; webview A/B matrix already cross-referenced; Phase 2 cards carry both. |
| doc 04 "no pure-Rust H.264/HEVC" stale vs 0.C1-I finding | 04 §4 vs feasibility/video-codecs | D-013: superseded by 0.C1-I (young 2026 wave, conformance-gated track per ADR-001). |
| Phase 5 has no filter-implementation cards | 13 §7 vs design package | D-012: Lead adds cards before Phase 5 launch. |
| No pure-Rust HTTPS (planning blind-spot) | 11 §6 vs 0.C3-I finding | D-011: ADR-011 proposed; decision ≤ Phase 5 gate; interim no-merge posture. |
| Symphonia WavPack crate is GPL-3.0 in MIT repo | 0.C2-I vs ADR-006 | License flag on the dependency allowlist; excluded or dual-compat reviewed at Phase 5 pin time. |

## 5. Open decisions for the user (escalation per MASTER-PROMPT §15)

**Resolved 2026-10-19:**
- **ADR-006 → MIT** (D-009; `docs/adr/ADR-006-license-mit.md`; R-05 closed).
- **R-07 design-package terms → cleared** (D-010; open-source Figma Community
  design, no terms restriction; R-07 closed — residuals: font identity check at
  Phase 4, trademark wording rules on shipped strings).

**Still open:**

1. **Patent posture (ADR-010, doc 11 §8.4)** — shipping H.264/HEVC/AC-3/DTS
decode (even self-written or OS-mediated) has patent considerations separate
   from source licenses; jurisdiction-dependent; needs legal review before
   distribution. Not blocking Phases 1–5; hard trigger is any
   binary-distribution plan.

*(Resolved at Phase 0 gate: hardware-decode appetite → ADR-001 accepted per
§2 default, hw-first with truthful reporting; see D-014.)*

## 6. ADR index (register: doc 01 §7)

ADR-001 hw decode — **accepted 2026-10-19** (`docs/adr/ADR-001-hardware-decode.md`; hw-first H.264/HEVC + conformance-gated software track) · ADR-002 WASM runtime — **accepted: wasmi** (`docs/adr/ADR-002-wasm-runtime-wasmi.md`) ·
ADR-003 video-in-webview — **accepted: per-OS strategy** (`docs/adr/ADR-003-video-in-webview.md`; prototype gates 2.D3/2.E2-V — PROVISIONAL-B, per-OS matrix pending desktop runtimes) · ADR-004 OS webview — **accepted 2026-10-22** (`docs/adr/ADR-004-os-webview.md`; seam landed 2.D3; per-OS verdicts + Phase-4 security review PENDING) ·
ADR-005 audio output backend — **accepted 2026-10-22 for the adapter seam** (`docs/adr/ADR-005-audio-output.md`; real-backend admission PENDING, cpal NOT admitted) · ADR-006 repo license — **accepted: MIT,
2026-10-19** (`docs/adr/ADR-006-license-mit.md`) · ADR-007 **VLC 3.0.23 as
external test oracle [PROPOSAL]** (identity per doc 01 §7, the register
authority — Lead decision 3.DOC-1; the former ROADMAP text "first-vertical
formats" was the error, reconciled 2026-11-09 issue #21; **no ADR-007 file
exists in `docs/adr/` yet**) · ADR-008 **optical disc support scope
[PROPOSAL]** (DVD/Blu-ray menus and copy protection, each decided by ADR;
per doc 01 §7 — the former ROADMAP identities "WebCodecs-as-hw-analog" /
"DVD/Blu-ray per-title" were unregistered; the WebCodecs proposal is
tracked un-numbered at doc 09 §10/§12 Q6) · ADR-009 **WASM threading &
COOP/COEP [PROPOSAL]** (per doc 01 §7) · ADR-010 **codec patent &
redistribution policy — drafted, pending counsel sign-off (2026-11-09,
issue #24)** (`docs/adr/ADR-010-codec-patent-shipping.md`; decision record
complete: per-codec evidence matrix, options a–d, recommended posture (a)
default-build = patent-clean tier + optional/plugin encumbered builds
[PROPOSAL]; 24-item counsel ledger Q1–Q24 open; status *proposed* — user +
counsel decide; hard trigger = Phase-8 packaging 8.E1–8.E3, no public binary
before it per R-03) ·
ADR-011 TLS/HTTPS — **decision EXECUTED in Phase 6** (6.A1b-RATIFY: ureq
3.5.0 + ring 0.17.14 as an ADR-001-class exception, the streaming layer's
HTTP(S) foundation; **decision record refreshed 2026-11-09, pending Lead
admission** — fresh rustls provider audit + typed `https` seam, issue #8) · ADR-012 bitmap + closed-caption subtitle
strategy — **accepted 2026-10-24** (Lead signature;
`docs/adr/ADR-012-bitmap-subtitles.md`; self-build on the 5.L1 CueBitmap
compositor contract; wave-9 N-cards 5.N1–5.N3 executed per phase5-exit.md
§2).

## 7. Document index

| Doc | Title | Status |
|-----|-------|--------|
| — | MASTER-PROMPT.md (authoritative brief) | committed |
| 01 | Architecture & crate registry | committed (Wave A) |
| 02 | Phase 0 audit plan | committed (Wave A) |
| 03 | Media engine plan | committed (Wave A) |
| 04 | Codec & format roadmap | committed (Wave A) |
| 05 | Command layer contract | committed (Wave A) |
| 06 | Plugin architecture | committed (Wave B) |
| 07 | Config-driven system | committed (Wave B) |
| 08 | UI & Figma fidelity | committed (Wave B) |
| 09 | Web/WASM target | committed (Wave B) |
| 10 | Testing, CI & quality | committed (Wave B) |
| 11 | Dependency, security & compliance | committed (Wave C) |
| 12 | Performance & benchmarking | committed (Wave C) |
| 13 | Agent wave program (200 cards / 77 waves) | committed (Wave C) |
| 14 | Risk register (44 risks) | committed (Wave C) |
| 15 | Git workflow | committed (Wave C) |
| — | design/ (102 pinned SVG exports + inventory) | committed |
| — | worklog.md — live log at the **repository root** (append-only); prior entries archived under `docs/worklog/` (see `docs/worklog/README.md`) | live |

## 8. Current state (2026-10-22) — HISTORICAL SNAPSHOT

> **[HISTORICAL SNAPSHOT — do not update in place.]** The block below is the
> program-state snapshot written 2026-10-22 (card 3.DOC-1), kept verbatim for
> the record. Its "Phase 3 IN PROGRESS" state is **superseded by the Status
> block in the header** (refreshed 2026-11-09, issue #21): Phases 3–5 have
> since closed out (gate records cited there; the phase-3/4 verdict lines
> still await Lead signature) and Phase 6 is in progress.

**Phase 1 COMPLETE** (all six waves 1.A–1.F; gate report
`docs/gates/phase1-exit.md`, 2026-10-20 — PASS, 529/0/3, CI green-run links
honestly PENDING first push). **Phase 2 COMPLETE** (all five waves 2.A–2.E;
gate verdict **PASS (local-evidence)**, `docs/gates/phase2-exit.md`,
2026-10-21): 17-card ledger green (2.B3 not-selected per D-018), test
trajectory 529 → 1329 (+800, per-merge reconciliations in the worklog), the
first vertical (AVI+MJPEG+PCM per D-018) running end-to-end through the
command layer with byte-stable, mock-labeled evidence (2.E1). The two
PENDING labels are infrastructure-environment facts, not code: 3-OS CI green
(workflow-scoped token owed — 3.HARD-AUDIT decision D-b) and the ADR-003
per-OS webview verdicts (desktop runtimes; PROVISIONAL-B stands).

**Phase 3 IN PROGRESS — wave 3.A first pair in flight** (3.LAUNCH-1,
2026-10-21): **3.A1 seek** on `wip/3a1-seek` (plugin-api seek hooks + AVI
demuxer reanchor landed on-branch; the playback-engine half is mid-
continuation after a transport cut — compile blockers and the ordered
continuation steps are enumerated in 3.HARD-AUDIT) and **3.B4 event
completeness** (events.rs draft salvaged; Lead-ratified continuation plan
sequences it behind the 3.A1 merge). Queued per the 2.E2-V launch list:
3.A2+3.A3 → the 3.B chain → 3.C state machine/recovery → 3.D testkit
v1/properties → 3.E scenarios/differential/soak → 3.F gate.

Program hygiene: 3.HARD-AUDIT (2026-10-22) verified the full program state
honest — no fabrications found; every CRITICAL/MAJOR finding was
documentation-governance debt, dispatched to card 3.DOC-1 (this refresh is
part of it). The git-always + salvage protocol has recovered 100% of
cut work across sandbox incidents (11-for-11).
