# CLAIM AUDIT: docs/inventory/ui-inventory.md
| Field | Value |
|---|---|
| Auditor | W18-A05 |
| Claims extracted | 14 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 6 / 0 / 0 / 0 / 1 / 1 / 0 (6 claims UNVERIFIED — external oracle) |
| Internal contradictions found | 1 |

## 1. Blunt verdict
This is a Phase-0 VLC *inventory* (upstream documentation), not a capability claim — so its
assertions about VLC's own UI are UNVERIFIED here (no VLC binary in the sandbox to re-run the
oracle). Everything checkable *downstream* of the doc is real: the keymap seed, the 111-row
hotkey table, the 23-dialog catalog, the 9+3 conflict ledger, and every referenced design SVG
and companion file exists. One numeric self-count is false: the doc claims "58 leaf items (15
[VERIFY])"; mechanical count of its own §1 tables is 60 data rows / 16 [VERIFY] rows. Its most
consequential downstream artifact — `keymaps/default.toml` — is accurately described (141+9
bindings, 4 scalars, 100% source provenance), but W11 proved only 7/150 bindings are wired to
anything, so the seed this doc advertises is a paper product in the shipped build.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "complete transcription of … the Qt menu bar — 8 menus, 58 leaf items (15 partial-[VERIFY])" | ui-inventory.md:428 | FALSE (count) | `rg '^\|'` over lines 26-132: 76 table lines − 8 headers − 8 separators = **60** data rows; rows carrying `[VERIFY` = **16** (awk/rg counts, exit 0). Same drift repeated at :132 "58 transcribed rows above, of which 15 carry [VERIFY]". |
| 2 | "(b) 23 dialogs (20 doc-sourced, 3 [VERIFY]-heavy)" | ui-inventory.md:428 | REAL | `sed -n '138,163p'` + `rg -c '^\| [0-9]+ \|'` → 23 numbered dialog rows (#1–#23). |
| 3 | "(d) all 111 VLC 3.0.23 hotkey actions with binary-observed Linux defaults" | ui-inventory.md:428 | UNVERIFIED (oracle) / REAL (table shape) | 111 rows counted in §5.2 (`sed -n '211,324p' | rg -c '^\| \`key-'` → 111, exit 0). "Binary-observed" cannot be re-run: no VLC binary in repo; evidence scratch "tmp/b3/" is outside the repo (line 22). |
| 4 | "(f) keymaps/default.toml seed — 141 bindings + 4 jump-size scalars + 9 macOS overrides, every entry carrying source provenance" | ui-inventory.md:428 | REAL | keymaps/default.toml: `rg -c '^[a-z0-9-]+ = \{ key = '` → **150** entries; `[platform_overrides.macos]` section (line 267) holds **9** overrides (`rg -c` → 9); `rg -c '^jump-size'` → **4**; `rg -c 'source = "'` → **150/150**. 141+9=150 checks out; all carry `source`. |
| 5 | "Companions: keymaps/default.toml …, docs/baselines/vlc-3.0.23-linux.md …, design/README.md + design/figma/" | ui-inventory.md:5 | REAL | `ls keymaps/` (default.toml, 44137 bytes), `ls docs/baselines/` (vlc-3.0.23-linux.md present), `ls design/figma/` → 102 files incl. menu.svg, right-click menu.svg, Audio Visualizer.svg, Video - Crop.svg, Video - Essential.svg. |
| 6 | "(g) 9 design-vs-VLC conflicts + 3 design-internal inconsistencies" | ui-inventory.md:428 | REAL | `rg -c '^\| C-0'` → 9; `rg -n '^\| D-[0-9]'` → 3 (D-1/D-2/D-3). |
| 7 | "The seed carries [platform_overrides.macos] with 9 [VERIFY] entries" | ui-inventory.md:335 | REAL | 9 `-macos` entries in keymaps/default.toml:268-276; every note begins `[VERIFY]`. Cross-source note: W11 SUMMARY says "all **8** macOS overrides dead" — the seed has 9 (W11 miscount). |
| 8 | "No screenshots were produced or consulted. Text transcription only" | ui-inventory.md:9 | UNVERIFIED | Process claim about a past agent run; no screenshot artifacts exist in repo to contradict it. |
| 9 | "all web evidence fetched with curl (recorded in tmp/b3/ scratch, outside repo deliverables)" | ui-inventory.md:22 | UNVERIFIED | tmp/b3 is outside the repo; nothing verifiable remains. |
| 10 | Crosswalk: "Audio → Audio Device ▸ (Shift+A) … audio.list-devices / audio.set-device | adopted" | ui-inventory.md:367 | REAL (catalog) / PAPER-ONLY (capability) | Both ids exist in crates/commands/src/catalog/ids.rs:121-122; **no host implements them** — apps/control-server/src/dispatch.rs:66-69 `_ => Err(…NotReady)`, apps/cli/src/dispatch.rs handles only session/playback/media/tracks/config/system commands. W8: shipped sink is NullBackend, device crates are orphans. |
| 11 | Crosswalk: "Video → Zoom ▸ / Aspect ▸ / Crop ▸ … video.set-zoom / video.set-aspect-ratio / video.set-crop | adopted" | ui-inventory.md:374 | REAL (catalog) / PAPER-ONLY (capability) | ids.rs:126-128 defines all three; no dispatcher arm anywhere (rg over apps/). Renderer math exists but is dead (see matrix row audit: W06-A04 — nothing calls `Renderer::set_params`). |
| 12 | "VLC 3.0.23 ships with no global hotkeys bound" (all 111 global-key-* empty) | ui-inventory.md:205 | UNVERIFIED | Upstream-oracle claim; no VLC binary here. |
| 13 | "key-subtitle-text-scale-up/-down help strings … swapped relative to the option names" | ui-inventory.md:325 | UNVERIFIED | Upstream-oracle claim. |
| 14 | "Design-internal inconsistencies … D-3: Stereo Widener mock 0.2 vs VLC default 0.3" | ui-inventory.md:422 | REAL (recorded) | Matrix row afeat-stereowiden (feature-matrix.toml:1726) repeats it consistently. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "58 leaf items (15 partial-[VERIFY])" (line 428) and "58 transcribed rows above, of which 15 carry [VERIFY]" (line 132) | The doc's own §1.1–§1.8 tables contain 60 data rows, 16 carrying [VERIFY] | The tables are true; the doc's self-count is false (mechanical count above). S4 count drift. |
| Doc's crosswalk marks stereo-mode "not-adopted-v1 (audio filter/channel remap later phase)" (line 368) | crates/audio-engine/src/channel_map.rs already contains `swap_stereo`/`mono_to_stereo`/`stereo_to_mono` (W07-A07: production-unreachable, zero non-test callers) | Both are accurate at different layers: the vocabulary exists as dead code; nothing user-visible ships. |

## 4. Fabrication / inflation indicators
- The [B]/[Q]/[UG]/[MAC]/[D] evidence-marker scheme is unusually honest: every unverifiable
  statement is self-tagged [VERIFY] with 10 ledgered follow-ups (V-01..V-10, lines 432-443).
- No inflation found in the doc itself. The inflation risk is *downstream*: this inventory is
  the population source for feature-matrix rows (meta line 26-31), and the matrix/seed pipeline
  advertises a 150-binding keymap of which 143 are wired to nothing (W11 SUMMARY:22-23) — the
  inventory's product exists, the consumption of it does not.
- One count error (58/15 vs actual 60/16) — sloppy self-audit, not fraud.

## 5. Matrix rows
N/A for this file — matrix chunk audited separately in
audit/docs/feature-matrix.toml.rows-118-130.audit.md. This file is cited as `inventory_ref`
by rows afeat-stereo-mode (§1.3), afeat-visualizations (§1.3), afeat-audio-device-switch
(§1.3) — all three anchors exist (### 1.3 Audio at line 65).
