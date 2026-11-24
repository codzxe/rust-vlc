# CLAIM AUDIT: docs/release-notes/skeleton-v0.1.0.md
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Claims extracted | 13 |
| REAL: 7 · REAL-UNPROVEN: 2 · SHALLOW: 0 · STUB: 0 · PAPER-ONLY: 0 · FALSE (stale vs. current repo): 4 · UNREACHABLE: 0 |
| Internal contradictions found | 3 |

## 1. Blunt verdict
An honest-at-authoring-time template that has rotted: four concrete claims are now false against the repo (display_version is 0.1.3 not 0.1.0-dev; a Tauri webview shell NOW exists and ships; artifacts now carry the GUI shell; claimed format rows are 38 not 37). The honesty architecture is excellent (per-claim sources, known-limits register, "all formats" ban) — but if anyone pastes this skeleton on RC day as instructed, they will publish wrong version, wrong scope, and wrong shipped-binary claims.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "`product.toml` (`identity.display_version = "0.1.0-dev"` → tag `v0.1.0` …)" | :4-6 | **FALSE (stale)** | product.toml:27 `display_version = "0.1.3"` (:29 `version_code = 4`) — the repo has shipped past 0.1.0 (make-app.sh:13-16 even references "v0.1.2 users") |
| 2 | "**37 of 73 format rows** are claimed … on desktop: 14 containers, 6 video codecs, 9 audio codecs, 8 subtitle formats" | :30-32 | **REAL at 2026-11-10 / STALE now** | source verified: docs/audit/docs-sweep-20261110.md:20-22 "73 format rows … **37 claimed on desktop** (implemented or partial: 14 / 6 / 9 / 8)". My recount today (python3 tomllib over docs/feature-matrix.toml): 73 format rows ✓, but claimed = **38** (14 / **7** / 9 / 8) — one more video-codec row (the 2026-11-15 h264-software admission, visible in the container-mp4 row note) |
| 3 | "**Zero rows are claimed for the web build.**" | :32 | **REAL (current)** | recount: web claimed (implemented/partial) = 0 (web statuses: 55 not_started, 17 blocked, 1 unsupported); consistent with baseline "web 0/252" |
| 4 | "The `lumen` CLI is the real engine surface … headless decode, playback state, seeking, A/V sync" | :33-35 | **REAL** | baseline rc-smoke: headless-decode (AVI/MJPEG/PCM fixture) PASS; engine-info decoders mjpeg, pcm_s16le |
| 5 | "Audio output is the ADR-005 null device (real output adapters exist as seams but no backend is admitted yet)" | :35-36 | **REAL-UNPROVEN** | consistent with KNOWN-LIMITS register + baseline engine-info (no audio backend listed); ADR-005 text not re-read in this audit |
| 6 | "`lumen-desktop` is the ADR-004 std-only composition-root skeleton — it opens no window" | :37-39 | **REAL (at the time; lumen-desktop still exits 86)** | linux-lane-20261110.log:67 "desktop skeleton: exit 86 (as documented)" |
| 7 | "The Tauri webview shell is infra-blocked (issue #4, R-04)." | :39 | **FALSE (stale)** | apps/desktop-tauri exists now (allowlist.md §3a "ADR-004 admission, issue #67, 2026-11-13"; records/tauri-wry-2.x.md; make-app.sh stages the tauri binary as the bundle's executable, w30.8). The infra block was resolved after this skeleton was written. |
| 8 | "no update mechanism ships (the MSI's frozen UpgradeCode makes future MSIs replace this one rather than co-install)" | :40-42 | **REAL** | scripts/packaging/build-msi.sh:10-11 "the UpgradeCode is stable across versions"; release.yml:49 + :390 same mechanism from product.toml |
| 9 | "Binaries are unsigned. No Windows code-signing certificate, no Apple Developer ID / notarization" | :75-78 | **REAL** | make-app.sh:36-41 the same honest limit; no sign artifacts in release lanes for the local-provable halves |
| 10 | "Web build: … the wasm engine exists and is smoke-built (`scripts/rc-smoke.sh` check 6)" | :84-86 | **REAL** | baseline: "wasm build passes with the wasm32 rust-std component installed"; rc-smoke web-ui-build/browser-load PASS |
| 11 | "No updater, no telemetry (all telemetry settings default OFF)." | :87 | **REAL** | product.toml:120-124 "Shipped defaults are off; users may opt in" ([telemetry]) |
| 12 | "Linux arm64 artifacts are best-effort (hosted-runner availability; the job is continue-on-error)." | :88-89 | **REAL** | release.yml:876 `continue-on-error: true` (:951 "This whole job is continue-on-error (best-effort)") |
| 13 | "Two programs ship in every artifact: `lumen` (CLI) and `lumen-desktop` (skeleton)." | :56-58 | **FALSE (stale)** | release.yml:328 "the zip carries the CLI + the GUI shell + docs" — the tauri GUI shell (BIN_TAURI) now ships alongside; the skeleton's two-program list predates w30.8 |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "identity.display_version = "0.1.0-dev"" (:5) | product.toml:27 "0.1.3" | product.toml wins (cat output). |
| "The Tauri webview shell is infra-blocked (issue #4, R-04)" (:39) | allowlist.md §3a (apps/desktop-tauri admitted 2026-11-13), make-app.sh:8-17 (bundles the tauri bin as CFBundleExecutable) | The tree wins — the shell exists and ships. |
| "Two programs ship in every artifact" (:56-58) | release.yml:328 "the zip carries the CLI + the GUI shell + docs" | release.yml wins (current shipping contract). |

## 4. Fabrication / inflation indicators
- None in intent: every claim carries a source, the scope section says "honest — read this before installing", and the doc bans "all formats supported" claims. The failure mode is staleness (no RC-day regeneration ever happened for v0.1.0 as written — the repo moved to 0.1.3 without the skeleton being updated or deleted).
- Numeric discipline is otherwise excellent: the 37/73/0 triple matches its cited source exactly (docs-sweep:20-22), and my recount reproduces 73 rows and the per-category totals.

## 5. Matrix rows
N/A — not a matrix chunk (but the 37/73 numbers above were independently recomputed from docs/feature-matrix.toml with tomllib: 73 format rows across container(21)+video-codec(16)+audio-codec(19)+subtitle-format(17)).

## 6. What actually works (verified)
- `python3` tomllib recount of docs/feature-matrix.toml (exit 0) — outputs cited in claims 2-3.
- `rg` over product.toml, release.yml, build-msi.sh, docs-sweep-20261110.md (all exit 0) — claims 1, 8-12.
- All cross-referenced files exist: KNOWN-LIMITS.md, RELEASE.md, release-checklist.md, docs-sweep-20261110.md, scripts/release/release-notes.sh.

## 7. Open questions for next agent
- Was this skeleton ever actually used for a real v0.1.0 GitHub Release (git history needed)? If yes, the shipped notes contradict today's product.toml; if no, it is dead template weight that must be regenerated before any 0.1.x RC.
- The feature-matrix meta says `last_updated = "2026-11-09"` while row notes carry 2026-11-15 edits — meta-vs-row drift worth a dedicated finding in the matrix auditor's lane.
