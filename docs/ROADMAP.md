# ROADMAP — Phase Gates, Wave Program & Decision Register

Authority chain: `docs/MASTER-PROMPT.md` (the brief) → this roadmap (program-level
plan & decisions) → domain docs 01–15 (detail). Conflicts resolve upward.
Statuses per MASTER-PROMPT §17: not_started, in_progress, implemented, verified,
blocked. **Current status: planning complete — Phase 0 ready to launch.**

## 1. Phase gates (MASTER-PROMPT §16)

Each gate passes only with recorded evidence on every claimed platform
(Windows, Linux, macOS; web column tracked per §10). Wave/card references are
from `docs/13-agent-wave-program.md` (200 cards / 77 waves).

| Phase | Objective | Waves | Cards | Exit-gate evidence (summary) | Owning docs |
|-------|-----------|-------|-------|------------------------------|-------------|
| 0 — Audit | Pin VLC 3.0.23, full feature inventory, feasibility report, ADR-001/002/003 decided | 0.A–0.D | 17 | `docs/baseline/` pinning records; `docs/feature-matrix.toml` populated with schema frozen; `docs/feasibility-report.md`; ADR-001/002/003 status=accepted; [VERIFY-P0] ledger cleared or converted to risks | 02, 04, 11, 14 |
| 1 — Foundation | Workspace, config system, schemas, lints, CI, command layer, plugin API, settings generator, token pipeline | 1.A–1.F | 20 | Local gate green (exact commands, doc 10 §2); CI merge-blocking on 3 OS; rebrand acceptance test passes; boundary check active; `cargo xtask gate` exists | 01, 05, 06, 07, 10, 15 |
| 2 — First vertical | One real file open→decode→audio+video on screen via command layer, from CLI and UI; video-in-webview prototype; Figma token proof | 2.A–2.E | 17 | Demo fixture plays end-to-end on all 3 desktop OS with recorded evidence; webview prototype A/B measured with decision record; token pipeline renders Player screen from tokens | 03, 04, 05, 08 |
| 3 — Reliable engine | Clocks, queues, generations, seeking, source replacement, A/V sync, recovery | 3.A–3.F | 19 | All 16 integration scenarios (doc 10 §6) green incl. error paths; property suites green; soak 8h no-growth; differential-vs-VLC harness v1 reporting | 03, 10, 12 |
| 4 — Desktop product | Figma-exact UI, layouts from config, themes, localization, playlists, settings, plugins, rebrand test | 4.A–4.I | 25 | Screenshot diffs within tolerance vs design exports at defined sizes/DPI; all 13 settings areas driven by settings-schema; i18n lint zero violations; conformance suite green on Tauri host | 07, 08, 05 |
| 5 — Breadth | Containers, audio+video codecs, subtitles, filters — one verified increment at a time | 5.A–5.R | 45 | Per-format records complete for every shipped format; conformance kit + fuzz target per parser plugin; corpus coverage; feature-matrix statuses updated by evidence only | 04, 06, 10 |
| 6 — Network & streaming | HTTP(S), HLS, DASH, RTSP-where-possible, buffering, reconnect, auth, fuzzing | 6.A–6.I | 17 | Network scenarios (interrupted/redirect/auth) green; protocol fuzzers in CI; blocked items (raw UDP/RTSP etc.) recorded with reasons | 03, 04, 11 |
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
| D-008 | **ADR timing: ADR-001 (hw decode), ADR-002 (WASM runtime), ADR-003 (video-in-webview) all decided inside Phase 0** per §16; ADR-004 (OS webview) accepted-by-brief; ADR-005 (audio backend) at Phase 1 wave 1.C; ADR-006 (repo license) is a **user decision** — see §5 | Follows MASTER-PROMPT §16 Phase 0 gate. | doc 01 §7 ADR register. |

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

## 5. Open decisions for the user (escalation per MASTER-PROMPT §15)

1. **ADR-006 — this repo's own license** (GPL-3.0 vs MIT/Apache-2.0; brief in
   doc 11 §8.3). Blocks: ecosystem/dependency direction, long-term contribution
   model. Recommendation framing prepared; not decided.
2. **Figma Community design package terms (R-07)** — the "VLC Redesign
   (Community)" exports are committed as the visual source of truth; commercial
   use may require the author's permission. Alternatives: obtain permission,
   treat as reference-only and re-derive, or commission an original system.
3. **Patent posture (ADR-010, doc 11 §8.4)** — shipping H.264/HEVC/AC-3/DTS
   decode (even self-written) has patent considerations separate from source
   licenses; jurisdiction-dependent; needs legal review before distribution.
4. **Hardware-decode appetite (feeds ADR-001)** — default per §2 is
   software-first with OS decoders as audited exception; user may tighten
   (software-only, lower parity ceiling) or confirm the exception path.

## 6. ADR index (register: doc 01 §7)

ADR-001 hw decode (Phase 0) · ADR-002 WASM runtime (Phase 0) · ADR-003
video-in-webview (Phase 0/2 prototype) · ADR-004 OS webview (accepted-by-brief) ·
ADR-005 audio output backend (Phase 1) · ADR-006 repo license (user) · ADR-007
first-vertical formats (decided: D-001) · ADR-008 WebCodecs-as-hw-analog on web
(Phase 7, feeds ADR-001) · ADR-009 DVD/Blu-ray per-title decisions (post-v1) ·
ADR-010 patent shipping matrix (user + counsel).

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
| — | worklog.md (17 agent records) | live |

## 8. Next action

Launch **Phase 0, wave 0.A** (doc 13 §3): baseline pinning + inventory
extraction — the first 4–6 investigation agents. Phase 0 ends at its gate
(doc 02 §1) with ADR-001/002/003 decided and the feasibility report written.
