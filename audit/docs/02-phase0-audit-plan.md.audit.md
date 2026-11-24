# CLAIM AUDIT: docs/02-phase0-audit-plan.md (L1-739)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 8 / 0 / 0 / 0 / 0 / 0 / 4 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A Phase-0 plan whose artifact promises are 90% real — every inventory/feasibility/ADR artifact it names exists in the tree — and the exit gate DID close — via docs/gates/phase0-exit-report.md, a drifted path doc 02 does not name. Minor path/name drift everywhere (docs/baseline vs docs/baselines, ADR-001-hardware-decoding vs ADR-001-hardware-decode, hotkeys.toml never created — hotkeys were folded into ui-inventory.md). None of it is fabrication; it is an unclosed process loop with sloppy artifact naming.

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | G13: "The Lead writes `docs/phase0-exit-report.md` ticking each row with links" | :39,:55 | REAL (path drift) | The report EXISTS at docs/gates/phase0-exit-report.md (ls-verified), NOT at the path doc 02 names. Correction 2026-11-24: an earlier draft of this row claimed the gate was never closed — wrong; it closed under the drifted path. |
| 2 | G1 artifacts `docs/baseline/vlc-3.0.23.md` + `.json` | :43 | REAL (path drift) | Actual: docs/baselines/vlc-3.0.23.md + vlc-3.0.23.json (+per-OS .md files) — same artifact, different directory name. |
| 3 | G2 raw inventory dumps `docs/inventory/raw/` + `raw/manifest.json` | :44 | REAL (structure drift) | docs/inventory/raw/{modules,options}/manifest.json exist (two manifests, not one). |
| 4 | G3/G4/G5: modules-{3}.json + options-{3}.json + formats.json | :45-47 | REAL | All present in docs/inventory/ (listing verified). |
| 5 | G6: `docs/inventory/ui-inventory.md` + `docs/inventory/hotkeys.toml` | :48 | PARTIAL | ui-inventory.md exists and carries the hotkey inventory ([B] binary-observed rows, :59); hotkeys.toml does NOT exist (rg --files: only ui/scripts/gen-hotkeys-data.mjs + UI hotkey files). Artifact never created as specified. |
| 6 | G7 feasibility report `docs/feasibility-report.md` | :49 | REAL | File exists in docs/ listing; sections/ dir has the 4 section docs. |
| 7 | G8/G9/G10 ADRs 001/002/003 | :50-52 | REAL (name drift) | docs/adr/ADR-001-hardware-decode.md, ADR-002-wasm-runtime-wasmi.md (+ADR-002-evidence.md), ADR-003-video-in-webview.md exist — filenames differ from the gate's names. |
| 8 | G11 matrix seeded, "Parses as TOML; §4.6 rules R1–R10 pass" | :53 | REAL | feature-matrix.toml parses (xtask feature-audit: rows 252); BUT the automated R1–R10 check FAILS today on 2 R4 violations (baseline xtask-feature-audit.txt) and misses prose-embedded fake paths (see matrix rows-183-195 audit, row 193 KeyHintStrip.tsx) — the "R4 evidence resolution" rule is weaker than the plan implies. |
| 9 | Schema frozen 2026-10-19 | §4.7 | REAL | feature-matrix.toml:18 "SCHEMA FROZEN 2026-10-19 (doc 02 §4.7)". |
| 10 | "Not code: Phase 0 produces documents and data only. No crates" | :7 | UNVERIFIED | Phase-0-era tree not inspectable (git forbidden); current tree obviously has crates (post-Phase-0). |
| 11 | Clean-room: "Never downloads VLC source, reads VLC source code, or copies tables/strings" | §2.1 | UNVERIFIED | No vendored VLC tree found (no vlc/ dir); full absence-of-copying verification not possible from a snapshot. |
| 12 | Seed rows "statuses are the expected Phase 0-exit initial state" = not_started ×4 | §4.5 | REAL | Matrix header vocabulary comment: "not_started … the honest Phase 0-exit default — nothing in this file is verified" (feature-matrix.toml:45-46). |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Exit gate G13: phase0-exit-report.md (docs/) written by Lead | The report exists at docs/gates/phase0-exit-report.md (ROADMAP: "Phase 0 — COMPLETE") | Gate closed; only the path drifted (docs/ → docs/gates/, consistent with D-007's baselines/ drift). |
| G6: hotkeys.toml as the hotkeys inventory artifact | ui-inventory.md carries hotkey rows ([B] evidence) | The data exists, the named artifact was never created. |

## 4. Fabrication / inflation indicators
None. This is a plan document; its checkable outputs exist. The unclosed G13 and the never-created hotkeys.toml are process debt, not lies.
Commands run: ls of docs/{adr,inventory,baselines}, rg for hotkeys/exit-report paths, matrix header grep. Exit 0.
Findings: S2: 0 · S3: 0 · S4: 4 (F-013 path/name drift baseline→baselines + ADR filenames + exit-report docs/→docs/gates/; F-014 hotkeys.toml never created (data lives in ui-inventory.md); F-015 raw manifest structure drift; F-012 CORRECTED: exit report exists, path drift only).
