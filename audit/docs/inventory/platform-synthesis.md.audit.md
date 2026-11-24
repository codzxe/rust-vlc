# CLAIM AUDIT: docs/inventory/platform-synthesis.md
| Field | Value |
|---|---|
| Auditor | W18-A05 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 4 / 0 / 0 / 0 / 1 / 0 / 0 (7 claims UNVERIFIED — external ecosystem/hardware facts) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A Phase-0 consolidation of three platform-research records feeding ADR-001/003/005 — a
decision-input document, and it says so ("Decision inputs, not decisions", line 9). Its
citable artifacts all exist, and the three ADRs it feeds are real and accepted. Its one
checkable "substrate" claim is materially stale/misleading: it lists "wry 0.57.0 / tauri
2.12.4 / tao 0.37.1 / raw-window-handle 0.6.2 / wgpu 30.0.1 / cpal 0.18.2" as the common
substrate — but NONE of those crates is compiled into the shipped workspace (root Cargo.lock,
158 packages, contains none of them; renderer-wgpu is std-only with the wgpu import
explicitly deferred; cpal exists only as a deferral record), and the only lockfile that does
pin them (apps/desktop-tauri/Cargo.lock — an excluded, compile-broken crate per W14 S0) pins
newer versions than the doc states. The external-ecosystem claims (WebCodecs floors, Windows
HEVC store ids, VA-API coverage) are not re-verifiable in this sandbox — marked UNVERIFIED,
not counted as findings without proof.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Common substrate (all three records): wry 0.57.0 / tauri 2.12.4 / tao 0.37.1 / raw-window-handle 0.6.2 / wgpu 30.0.1 / cpal 0.18.2" | :11-12 | PAPER-ONLY (as project deps) + stale numbers | `rg -n 'name = "(wry\|tao\|raw-window-handle\|wgpu\|cpal\|tauri)"' Cargo.lock` → 0 hits (root lock, 158 pkgs, none of them). apps/desktop-tauri/Cargo.lock: wry **0.57.2** (:4961-4962), tauri **2.12.5** (:3688-3689), tao **0.37.2** (:3631-3632), raw-window-handle 0.6.2 (:2746-2747) — three of four drifted. renderer-wgpu/Cargo.toml:4-13: "v0 is std-only by C5: the wgpu import is a recorded Lead decision … the crate ships the renderer architecture behind an internal backend seam". |
| 2 | "Feeds ADR-001 (hw decode), ADR-003 (video-in-webview), ADR-005 (audio backend)" | :9 | REAL | docs/adr/ADR-001-hardware-decode.md:2 "Status: accepted (2026-10-19, Phase 0 gate)"; ADR-003-video-in-webview.md:2 accepted; ADR-005-audio-output.md:2-3 "accepted for the adapter-seam architecture (2026-10-22…". |
| 3 | Sources "[L] platform-linux.md (0.C5-a) · [W] platform-windows.md · [M] platform-macos.md" | :4-5 | REAL | `ls docs/inventory/platform-*.md` → platform-linux.md, platform-macos.md, platform-synthesis.md, platform-windows.md. |
| 4 | "Supporting, where cited: modules-summary.md (0.B2), 13-agent-wave-program.md §5, ROADMAP.md D-008, MASTER-PROMPT.md §2/§9" | :6-7 | REAL | All four exist (docs/13-agent-wave-program.md, docs/ROADMAP.md, docs/MASTER-PROMPT.md present in docs/ listing). |
| 5 | "consolidation only — no new research performed" | :3 | UNVERIFIED | Process claim about the producing agent; no artifact distinguishes consolidated vs new research. |
| 6 | "All underlying evidence dated 2026-10-19" | :10 | UNVERIFIED | Dates live in the three platform records; spot-check not performed against all of them. |
| 7 | "Windows HEVC $0.99 … legacy `9N4WGH0Z6VHM` returns HTTP 410; new id `9nmzlz57r3t7`" | :37 | UNVERIFIED | External Microsoft Store facts; no network/hardware in sandbox. Carried forward into SV-38. |
| 8 | "Video-only WebCodecs from macOS ≥13.3 … full API from macOS ≥26" | :30 | UNVERIFIED | External WebKit facts. |
| 9 | "WebCodecs proven compiled-in to Debian-stable WebKitGTK 2.52.6 … Default-enabled state unverified" | :31 | UNVERIFIED | External; the doc itself carries the honest [VERIFY] (SV-1). |
| 10 | "no pure-Rust VA-API is possible (the API is a C ABI)" | :39 | UNVERIFIED (external), consistent with ADR-013 | docs/adr/ADR-013-va-api-hw-decoder.md:3-4 confirms the VA-API lane is still a proposed draft ("this file does NOT decide anything"). |
| 11 | "Oracle corroboration: VLC 3.0.23's own module scoring prefers hw decode (ffmpeghw score 10001, dav1d 10000)" | :77 | UNVERIFIED (oracle numbers) | modules-summary.md exists as the cited source; the scores are upstream-VLC facts not re-derivable here. |
| 12 | "No question is left unanswered in principle." | :81 | REAL (structurally) | §3 tables: 8+8+7 questions each carrying answered/[VERIFY] status; SV-1..SV-42 ledger exists at :157-200 (42 items). |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Common substrate wry 0.57.0 / tauri 2.12.4 / tao 0.37.1 (:11-12) | apps/desktop-tauri/Cargo.lock pins wry 0.57.2 / tauri 2.12.5 / tao 0.37.2 | The lockfile is true; the doc's snapshot (dated 2026-10-19) is stale. S4 staleness — and the deeper fact is that none of the substrate is in the shipped workspace lock at all. |
| ADR-005 ranks cpal as acceptable path (:47, :55, :150-152) | W8-A04: the cpal seam in crates/audio-output is a STUB ("zero cpal code behind the seam") and the shipped sink is NullBackend (W8 SUMMARY) | No contradiction — the doc is a decision input predating implementation; recorded here because the decision still has zero shipped implementation behind it. |

## 4. Fabrication / inflation indicators
- Honest by construction: every hardware-dependent statement routes to the SV-1..SV-42
  [VERIFY] ledger with named vehicles/owners; §6 "Gaps observed during consolidation
  (listed, not filled)" enumerates what could not be checked (:202-209).
- The substrate line is the only inflation-adjacent item: presented as "common substrate"
  for the whole project, it describes crates no shipped lane compiles; a reader would
  believe the repo builds on wgpu 30.0.1/cpal 0.18.2 — it does not (root Cargo.lock: none).
- Cross-wave consistency: W14's S0 (desktop-tauri cannot compile on any host) means even the
  excluded crate that DOES pin these deps cannot build; the substrate is lockfile-paper.

## 5. Matrix rows
N/A — no feature-matrix rows cite this file as inventory_ref (matrix inventory_refs are
options-summary/modules-summary/ui-inventory/options-linux.json/formats.json).
