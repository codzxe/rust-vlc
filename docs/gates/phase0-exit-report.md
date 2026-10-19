# Phase 0 Exit Gate Report (card 0.D4-V)

Lead gate review, 2026-10-19. Checklist source: `docs/02-phase0-audit-plan.md`
§1.2. Verdict per row with evidence links. **Gate verdict: PASS** (with
[VERIFY-blocked-on-hardware] rows explicitly carried, not hidden — they are
work-assignments for Phase 2+ hardware runs, not failures).

| # | Exit criterion | Status | Evidence |
|---|----------------|--------|----------|
| 1 | VLC 3.0.23 pinned per platform (hashes, sources, signature results) | ✅ complete (Win/macOS run-dumps blocked-on-hardware, runbooks ready) | `docs/baselines/vlc-3.0.23.md` + `vlc-3.0.23.json`; per-platform records + raw/; DR-01..DR-16 discrepancy register |
| 2 | Feature inventory from plugin categories | ✅ complete (Linux measured; Win/mac skeletons cited) | `docs/inventory/modules-{linux,windows,macos}.json`, `modules-summary.md` |
| 3 | Options/preferences inventory | ✅ complete (1,476 records, 100% coverage, 252/252 default agreement) | `docs/inventory/options-*.json`, `options-summary.md` |
| 4 | Menus/dialogs/hotkeys inventory | ✅ complete (8 menus/58 items/23 dialogs; 111 actions → keymap seed) | `docs/inventory/ui-inventory.md`, `keymaps/default.toml` |
| 5 | Formats/access inventory with probe evidence | ✅ complete (82 families; 59 probed; G5 statement) | `docs/inventory/formats.{json,md}`, `raw/linux/17-format-probes.txt` |
| 6 | feature-matrix.toml schema frozen + populated | ✅ complete (252 rows / 21 categories; schema_version 1 frozen; change-control per doc 02 §4.7) | `docs/feature-matrix.toml`, `matrix-seed-notes.md` |
| 7 | Feasibility report written | ✅ complete | `docs/feasibility-report.md` + `feasibility/sections/{video-codecs,audio-codecs,subtitles,subsystems}.md` |
| 8 | ADR-001 hardware decoding decided | ✅ accepted | `docs/adr/ADR-001-hardware-decode.md` (hw-first H.264/HEVC; conformance-gated software track; VA-API E2-conditional) |
| 9 | ADR-002 WASM runtime decided | ✅ accepted | `docs/adr/ADR-002-wasm-runtime-wasmi.md` (wasmi; no-WASI; limits; nested-wasm spike) |
| 10 | ADR-003 video-in-webview decided | ✅ accepted | `docs/adr/ADR-003-video-in-webview.md` (per-OS: A-macOS, A-or-B-Windows, B-Linux; prototype gates 2.D3/2.E2-V) |
| 11 | Dependency pre-audit + allowlist format | ✅ complete | `docs/dependencies/{audit-method,allowlist-format,preaudit-foundation}.md`, `records/wasmi.md`, `docs/adr/ADR-002-evidence.md` |
| 12 | Platform & hardware capability records | ✅ complete | `docs/inventory/platform-{windows,linux,macos}.md` + `platform-synthesis.md` |
| 13 | Security threat-model draft + corpus plan | ✅ complete | `docs/inventory/security-phase0.md`, `corpus-plan.md` |

## Lead ratifications recorded at this gate

- **R-1 (0.A4)**: Debian-trixie as the Linux baseline package (3.0.23 exact;
  Ubuntu 24.04–25.10 don't carry it — Launchpad-verified). Accepted.
- **R-2 (0.A4)**: GPG signature predates the 2026-08-01 key expiry; two
  keyserver anchors agree on the fingerprint. Accepted as pin-sufficient;
  key re-anchor listed for the next release pin.
- **R-3 (0.A4)**: hardware-run owners → assigned to Phase 2 wave 2.E
  (Windows/macOS smoke dumps) and Phase 3 differential harness bring-up.
- **R-4**: `docs/baselines/` plural spelling stands (D-007); doc 02/13 path
  references annotated, no edit churn.
- **R-5**: G1 ticked "complete-with-hardware-pending" as recorded in
  `baselines/vlc-3.0.23.md` §G1 — the gate's spirit (auditable pin) is met.

## New decisions + register updates at this gate

- **D-011**: ADR-011 (TLS/HTTPS) opened as *proposed*; decision due ≤ Phase 5
  gate; no HTTPS dep merges until then.
- **D-012**: doc 13 amendment — filter-implementation cards to be added to
  Phase 5 waves (gap found by 0.C3-I). Owner: Lead, before Phase 5 launch.
- **D-013**: doc 04 §4 "no pure-Rust H.264/HEVC decoder" annotated as
  superseded-by 0.C1-I finding (young 2026 wave; conformance-gated track).
- **D-014**: doc 02 §7's "0.A9 license-policy work" satisfied early by
  ADR-006 = MIT (user decision, pre-gate).

## Carried risks (top-5 at gate)

R-01/R-02 (H.264/HEVC software — mitigated by ADR-001), R-03 (patents,
ADR-010 — user+counsel, pre-distribution), R-06 (Opus — self-build track),
R-04 (WebKitGTK — bounded by ADR-003 B-path), R-44-class (TLS — ADR-011).

## Phase 1 launch list (doc 13 §4, waves 1.A–1.F, 20 cards)

Ready to launch in dependency order: 1.A workspace + lints + CI skeleton →
1.B product.toml + schema + validator → 1.C settings-schema + localization +
commands crates (incl. ts_ns envelope decision) → 1.D media-types + plugin-api
(Lead-owned) → 1.E theme-tokens + keymap wiring + xtask suite → 1.F testkit v0
+ boundary check + wasm CI job. ADR-005 (audio backend) decides at 1.C per
D-008.

**Phase 0 is complete. Tag: phase-0-gate.**
