# CLAIM AUDIT: docs/feature-matrix.toml (rows 183–195, TOML lines 2537–2712)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Rows audited | 13 (platform-feature ×4, services-discovery ×6, ui-screen ×3) |
| Row verdicts | REAL: 3 (rows 193–195 with defects noted) · PAPER-ONLY-accurate: 8 (183–186, 190–192… see table; row 189 = FALSE) · FALSE: 1 (row 189) · SHALLOW: 1 (row 187 stale) |
| Claims extracted | 18 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 8 / 0 / 1 / 0 / 8 / 1 / 0 |
| Internal contradictions found | 2 (row 189 vs crates/device-discovery/src/ssdp.rs; row 187 vs same crate) |

## 1. Blunt verdict
Rows 183–186, 190–192 are honest empty planning rows (no code, no evidence claimed) — nothing to inflate. Row 189 (sd-upnp) is FALSE as a status: it stamps "blocked" on all four platforms while the tree contains a 584-line SSDP/UPnP discovery core with real-UDP round-trip tests — the row understates in-tree work and misuses the frozen "blocked" vocabulary (internal scope choice, not external blocker). Row 193's evidence string cites `KeyHintStrip.tsx`, which exists NOWHERE in the repo — a fabricated evidence path the xtask R4 check missed because the token is embedded in prose, not a bare path. Row 194's evidence cites AppShell.tsx for shuffle/repeat consumption; AppShell.tsx contains zero shuffle/repeat references — wrong file, real functionality elsewhere. Row 195's description "13 generated area screens" is inaccurate: 13 area screens exist, only 10 are schema-generated.

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | status blocked ×3 + web unsupported, "crates.io bluray NOT_FOUND; menus blocked" | :2546-2548 | PAPER-ONLY (accurate) | docs/feasibility/sections/subsystems.md:560-562 says exactly this; `rg -i bluray crates/` -> no code. |
| 2 | "BDMV/STREAM/*.m2ts opens as MPEG-TS (5.I1) with an honest no-menus state" [PROPOSAL] | :2548 | PAPER-ONLY (proposal, labelled) | No m2ts/BDMV code: `rg -i 'bdmv\|m2ts' crates/` -> empty. Labelled [PROPOSAL], not claimed as built. |
| 3 | status not_started ×4 (disc-cdda) | :2560 | PAPER-ONLY (accurate) | `rg -i cdda crates/` -> no code. |
| 4 | status not_started ×4 (zero-copy-interop) | :2572 | PAPER-ONLY (accurate) | No zero-copy code; CLI Cargo.toml comment: "renderer-wgpu … software compositor — no GPU in this sandbox". |
| 5 | status not_started ×4 (desktop-integration) | :2585 | PAPER-ONLY (accurate) | `rg -i 'systemtray\|inhibit\|screensaver' crates/` -> only WASAPI audio-device notifications (platform-windows/src/capability.rs:134), not desktop integration. |
| 6 | sd-directory-watch not_started ×4, evidence "" | :2603 | SHALLOW/stale (understates) | crates/device-discovery/ EXISTS: 2510 LOC (volume.rs 382, monitor.rs 436, events.rs 306) — the seam landed (card 7.D1); only platform adapters are the typed UnavailableVolumes fallback (volume.rs:181). Row's not_started is stale w.r.t. the crate but no platform functionality exists. |
| 7 | sd-upnp "blocked" ×4 incl. linux/windows/macos | :2629 | FALSE (status) | crates/device-discovery/src/ssdp.rs:1-30 implements the UPnP discovery core ("M-SEARCH codec … response parser … proven with real UDP round-trips"); std-only UDP declared PROVEN FEASIBLE — not an external blocker. Browse half (ContentDirectory) absent, which is the row substance; status vocabulary misapplied (internal scope choice ≠ "blocked = external blocker", doc :59-60). |
| 8 | sd-mdns blocked ×4, "mdns-sd 0.21.6 is pure" | :2642-2644 | PAPER-ONLY (accurate) | subsystems.md:726 E22 confirms mdns-sd 0.21.6 (2026-10-19); no mdns code in crates/. |
| 9 | sd-sap blocked ×4, "paired with blocked udp/rtp anyway" | :2655-2657 | PAPER-ONLY (accurate) | No SAP code in crates/. (udp/rtp row statuses are outside this chunk — UNVERIFIED here.) |
| 10 | sd-census-tail not_started ×4; "lua SD replaced by WASM model" | :2668-2670 | PAPER-ONLY (accurate) | plugin-host-wasm crate exists; no pulselist/mtp/xcb_apps code. |
| 11 | ui-screen-player implemented ×3, web not_started | :2685 | REAL (evidence defect below) | PlayerScreen.tsx (1553 LOC) + 1262-LOC test + states/ + controls/ + useChromeAutoHide.ts + useFullscreen.ts all exist; ui baseline: 136 files/1705 tests passed (audit/runs/baseline/ui-test.txt:178-179); ui build exit 0 (ui-build.txt tail). Not executed by me (npm budget) — REAL-UNPROVEN on execution, but paths + baseline runs verified. |
| 12 | evidence: "…states/, controls/, KeyHintStrip.tsx, useChromeAutoHide.ts…" | :2686 | FALSE (evidence path) | `rg --files ui/src \| rg -i keyhint` -> NOTHING; `rg -i KeyHintStrip` repo-wide -> nothing. KeyHintStrip.tsx does not exist anywhere. |
| 13 | ui-screen-playlist-library implemented ×3 | :2697 | REAL | PlaylistScreen/Panel/Provider (609/456/320 LOC) + tests exist. |
| 14 | evidence: "engine-reported shuffle/repeat consumed by the menubar checkboxes (ui/src/components/appshell/AppShell.tsx)" | :2698 | FALSE (citation) / claim substance REAL | `rg -i shuffle ui/src/components/appshell/` -> 0 hits. Real consumers: ui/src/screens/player/controls/ControlBar.tsx:533,547 (checkbox onChange), PlaylistPanel.tsx:192,200,419, PlayerScreen.tsx:377,385. Cited file wrong; functionality real. |
| 15 | ui-screen-settings implemented ×3 | :2709 | REAL | SettingsScreen.tsx (496 LOC), SettingsProvider (159), search.ts, persistence.ts, SettingField.tsx, generated/Screens.tsx, areas/index.tsx all exist; ui-test baseline passes. |
| 16 | description: "13 generated area screens" | :2705 | FALSE (wording) | generated/Screens.tsx:318 `GENERATED_AREA_IDS = [global, interface, privacy, controls, playback, video, audio, performance, recording, extensions]` = 10 generated; 13 total area screens (3+ hand-authored merges, areas/index.tsx:60-63). "generated" is wrong for 3 of 13. |
| 17 | notes: "generated-screen snapshot test detects schema drift" | :2711 | REAL-UNPROVEN | generated/Screens.tsx + areas tests exist (areas.test.tsx, codecs.test.tsx…); snapshot behavior not individually executed by me. |
| 18 | notes: "mock persistence live; real config.set-setting write path is the open handoff" | :2711 | REAL (honest disclosure) | persistence.ts:3 "// MOCK persistence seam (card 4.E1)… REAL write path is the app layer's — `config.set-setting`" — the row admits settings writes go to localStorage, not the engine. Status "implemented" stands on framework code only. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| row 189 sd-upnp: status = blocked on windows/linux/macos (external blocker) | crates/device-discovery/src/ssdp.rs implements M-SEARCH + parser + searcher, "PROVEN FEASIBLE, std-only", real UDP round-trips in tests | The code exists; "blocked" is a misclassification. True state: partial discovery core in-tree, browse not implemented. |
| row 187 sd-directory-watch: not_started, evidence = "" | crates/device-discovery/src/lib.rs: "the services-discovery layer (card 7.D1…)" with VolumeSource trait + monitor + events (2510 LOC, tests) | Work HAS started (seam + engine side); platform adapters absent (volume.rs:181 UnavailableVolumes). Row is stale in the honest direction (understates). |
| row 193 evidence: KeyHintStrip.tsx exists | `rg --files` repo-wide: no such file | The file does not exist; evidence string cites a fabricated path. |

## 4. Fabrication / inflation indicators
1. Row 193 (:2686) — evidence string contains `KeyHintStrip.tsx`; no file by that name exists anywhere in the repo (`rg --files | rg -i keyhint` -> empty). The xtask feature-audit (audit/runs/baseline/xtask-feature-audit.txt) reports only 2 R4 violations (rows 23, 163) — it did NOT catch this one because bare filenames embedded in prose are not resolved as paths. The tool's R4 rule under-detects; this row is silent evidence inflation on a P0 row.
2. Row 194 (:2698) — evidence points at AppShell.tsx for shuffle/repeat checkboxes; that file has zero such references. Real code lives in ControlBar.tsx:533. Evidence strings are prose, not verified pointers.
3. Row 195 (:2705) — "13 generated area screens": 13 screens, 10 generated. Number inflation of the "generated" adjective.

DISPLAY-PIPELINE CORRECTION (2026-11-24): an earlier draft of this audit reported 259 files/1044 sites of `#ust_use` "corruption" — that was an artifact of the auditor's own output display (the rendering pipeline eats `[m` byte pairs; hex-dump proof: surface.rs:59 bytes `23 5b 6d 75 73 74 5f 75 73 65` = `#[must_use`, monitor.rs:55 = `#[must_use = "constructs the config"]`, baseline.py:134 = `ordered[middle]`). The tree is byte-clean; the corruption claim is RETRACTED.

## 5. Matrix rows (per template §5)
| Row | id | Statuses claimed (w/l/m/web) | Evidence string | Code actually found | Executed? | Verdict |
|---|---|---|---|---|---|---|
| 183 | platform-disc-bluray | blocked/blocked/blocked/unsupported | "" | none (rg bluray -> 0 in crates/) | no (nothing to run) | PAPER-ONLY, accurate; blocked_reason verified vs subsystems.md:560-562 |
| 184 | platform-disc-cdda | not_started ×4 | "" | none | no | PAPER-ONLY, accurate |
| 185 | platform-zero-copy-interop | not_started ×4 | "" | none | no | PAPER-ONLY, accurate |
| 186 | platform-desktop-integration | not_started ×4 | "" | none (only WASAPI device notifications) | no | PAPER-ONLY, accurate |
| 187 | sd-directory-watch | not_started ×4 | "" | crates/device-discovery 2510 LOC (seam+monitor, no platform adapters) | no | SHALLOW/stale — understates in-tree work |
| 188 | sd-podcast | not_started ×4 | "" | none | no | PAPER-ONLY, accurate |
| 189 | sd-upnp | blocked ×4 | "" | ssdp.rs 584 LOC: M-SEARCH + parser + searcher + real-UDP tests | no | FALSE status (see ledger #7) |
| 190 | sd-mdns-renderers | blocked ×4 | "" | none | no | PAPER-ONLY, accurate |
| 191 | sd-sap | blocked ×4 | "" | none | no | PAPER-ONLY, accurate |
| 192 | sd-census-tail | not_started ×4 | "" | none (plugin-host-wasm exists per "WASM model") | no | PAPER-ONLY, accurate |
| 193 | ui-screen-player | implemented ×3 / web not_started | :2686 (cites nonexistent KeyHintStrip.tsx) | PlayerScreen 1553 LOC + 1262-LOC test + states/controls/autohide/fullscreen | not by me; baseline ui-test 1705 passed (ui-test.txt:178-179), ui-build exit 0 | REAL with S2 evidence defect |
| 194 | ui-screen-playlist-library | implemented ×3 / web not_started | :2698 (cites AppShell.tsx wrongly) | Playlist trio + ControlBar/PlayerScreen/PlaylistPanel shuffle/repeat wiring | not by me; same baseline | REAL with S3 citation defect |
| 195 | ui-screen-settings | implemented ×3 / web not_started | :2710 (paths all exist) | 496-LOC screen + provider + search + mock persistence + 10 generated areas | not by me; same baseline | REAL; description wording defect (S4); persistence is mock by the row's own admission |

Findings (file-level): S0: 0 · S1: 0 · S2: 2 (F-001 fabricated evidence path row 193; F-002 row 189 FALSE status) · S3: 2 (F-003 wrong-file citation row 194; F-004 stale row 187) · S4: 1 (F-005 "13 generated" wording).
Commands run (exit 0 unless noted): rg counts/paths as cited; sed -n '2530,2728p' docs/feature-matrix.toml; ls ui/src/screens/player/{states,controls,dialogs}; wc -l on all cited UI files; rg shuffle/repeat across ui/src; rg keyhint repo-wide (0 hits); python3 hex-dump byte verification of suspect lines (see RETRACTION in section 4).
