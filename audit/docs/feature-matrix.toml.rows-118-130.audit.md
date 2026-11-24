# CLAIM AUDIT: docs/feature-matrix.toml rows 118-130 (audio-feature tail + video-feature head)
| Field | Value |
|---|---|
| Auditor | W18-A05 |
| Claims extracted | 13 rows (8 audio-feature: rows 118-125; 5 video-feature: rows 126-130) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 0 / 0 / 0 / 0 / 0 / 0 / 1 (row 129 feature = UNREACHABLE; 12 not_started rows have ACCURATE claims — no code contradicts them) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
12 of 13 rows claim `not_started` on all four platforms with `evidence = ""`, and the claim is
accurate in every case: the engine code for these features does not exist (rg census below),
and the shipped parity report confirms audio-feature 0/11 and video-feature 0/10
(audit/runs/baseline/xtask-parity-report.txt). The schema itself admits it (feature-matrix.toml:45-46:
`not_started no work yet (the honest Phase 0-exit default; nothing in this file is verified — no CI exists yet)`).
The single non-trivial row is 129 (vfeat-colors-adjust), the only row with a non-empty
`evidence` string and `partial` status: every code-facing assertion in its evidence string
checks out (file exists, params validated, 48/0 tests — baseline line 6507), except `commit
50a7ba6` which is UNVERIFIED (no git in sandbox). But the feature it describes is UNREACHABLE
from every shipped entry point: `video.set-adjust` does not exist in the catalog (rg over
crates/, apps/, ui/src/generated/ → zero hits), and W03-A08 proved zero production callers for
`AdjustParams`/`apply_rgba8`. "partial" is disclosed-generous: nothing user-visible works.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | status all "not_started", evidence "" — spatializer | :1697-1707 | ACCURATE (no code) | `rg -il "spatializer" crates/ apps/` → 0 engine hits (only ui/src/screens/player/tools/dynamics/SpatializerTool.tsx = STUB per audit/files/ui/.../SpatializerTool.tsx.audit.md:7 "STUB (staged display ranges; interactive faders with no engine command)"). Design SVG exists (design/figma/Audio - Spatializer Settings.svg). |
| 2 | not_started, evidence "" — stereo widener | :1710-1720 | ACCURATE (no code) | Same census: only ui/.../StereoWidenerTool.tsx (STUB, W15-A09 audit :7). |
| 3 | not_started, evidence "" — resampler | :1723-1732 | ACCURATE for the claimed feature | crates/audio-engine/src/resample.rs exists but is drift-only (±1000 ppm; cannot 44.1→48); W07-A08/A10 audit resample.rs.audit.md:7 "REAL ... process() is UNREACHABLE from the shipped build"; that audit explicitly ruled "the feature-matrix afeat-resampler row is honestly not_started". |
| 4 | not_started, evidence "" — stereo mode | :1735-1744 | ACCURATE shipped; dead vocabulary exists | crates/audio-engine/src/channel_map.rs has swap_stereo/mono_to_stereo/stereo_to_mono — production-unreachable, zero non-test callers (W07-A07 channel_map.rs.audit.md:8). |
| 5 | not_started, evidence "" — visualizations | :1747-1757 | ACCURATE (no engine code) | `rg -il 'goom|projectm|vsxu' crates/ apps/` → 0. UI VisualizerTool is staged with demo levels (W15-A07 audit : "harness.tsx:84 hard-codes demo levels"). Exclusion note corroborated by modules-summary.md:237-240. |
| 6 | not_started, evidence "" — replay gain | :1760-1769 | ACCURATE (no code) | `rg -il 'replay.?gain' crates/ apps/` → 0 engine hits; only ui/src/screens/settings/areas/audio.tsx (settings form). inventory anchor options-linux.json:19194 "audio-replay-gain-mode" exists. |
| 7 | not_started, evidence "" — S/PDIF passthrough | :1772-1782 | ACCURATE (no code) | `rg -il 'spdif' crates/ apps/` → 0. formats.json:900 family "ac3-eac3" exists as inventory anchor. |
| 8 | not_started, evidence "" — audio device selection | :1785-1794 | ACCURATE (nothing works) | Catalog ids exist (crates/commands/src/catalog/ids.rs:121-122) but NO host implements them: apps/control-server/src/dispatch.rs:66-69 returns NotReady for all commands except engine-info/list-commands; apps/cli/src/dispatch.rs has no audio device arm; apps/web: rg 'AudioListDevices' → 0. W8 SUMMARY: "how does a user hear sound? THEY DON'T … NullBackend"; platform crates are orphans. keymap binding audio-device-cycle (keymaps/default.toml:148) references the two commands via an unwired recipe (W11: 143/150 bindings wired to nothing). |
| 9 | not_started, evidence "" — aspect ratio | :1801-1811 | ACCURATE shipped; library math exists dead | crates/renderer-wgpu/src/params.rs is REAL pure aspect math (W06-A04 audit :7) but "Fill/Stretch/Crop16x9 and user crop (F-001) … nothing outside renderer-wgpu ever calls Renderer::set_params"; catalog video.set-aspect-ratio (ids.rs:126) has no dispatcher arm. |
| 10 | not_started, evidence "" — crop | :1814-1824 | ACCURATE shipped; UI local-only | params.rs crop path dead (W06-A04); ui/.../CropTool.tsx sends no commands (W16-RR3 CropTool.test.tsx.audit.md: "rg useClient/send in CropTool.tsx (none)"). |
| 11 | not_started, evidence "" — zoom + autoscale | :1827-1837 | ACCURATE (command id only) | video.set-zoom exists in catalog ids.rs:231-232 (class S, 0.25..=4.0) — zero handlers anywhere (rg over apps/); no renderer zoom implementation (apps/web/src/render/canvas.rs:270 zoom mention is DPR scaling, not the feature). |
| 12 | status windows/linux/macos "partial", web "not_started" + evidence string | :1848-1849 | Evidence string: VERIFIED except commit hash; feature UNREACHABLE | See §5 row 129 below. |
| 13 | not_started, evidence "" — sharpen + gradfun | :1853-1863 | ACCURATE (no code) | `rg -il 'sharpen|gradfun' crates/ apps/` → 0 engine hits. UI EssentialTool.tsx:26 says verbatim: "Honest staging: no adjust/sharpen/gradfun/grain engine stage or catalog command exists — local state + the visible staged markers (VideoCard), controls fully interactive." |

## 3. Contradictions
None found. Cross-source check: matrix notes vs ui-inventory §7 (D-3 stereo-widener 0.2-vs-0.3),
modules-summary.md:237-240 (visualization plugin exclusion), options-summary.md §5 /
modules-summary.md §5 / ui-inventory.md §1.3 / options-linux.json (aspect-ratio, crop, zoom,
audio-replay-gain-mode at :19194, :19869, :20050) — every `inventory_ref` anchor resolves.
The only near-contradiction is definitional, not factual: row 129's "partial" vs the
vocabulary's "partial a documented subset works" (feature-matrix.toml:55) — no subset of the
color-adjust feature works in any shipped lane; the notes disclose this.

## 4. Fabrication / inflation indicators
- ZERO inflation in this chunk: 12/13 rows carry empty evidence and not_started. The
  parity-report tool's strict definition scores this chunk 0/13 verified — consistent with reality.
- Row 129's evidence is over-precise but not fabricated: "48/0 crate tests" matches baseline
  (audit/runs/baseline/cargo-test-workspace.txt:6507 "test result: ok. 48 passed; 0 failed");
  "ui pending sliders awaiting video.set-adjust" matches ui/src/screens/settings/areas/effects.tsx:37-42.
- `commit 50a7ba6` (row 129) is unverifiable without git — not treated as fabrication, marked UNVERIFIED.

## 5. Matrix rows (row id, platform statuses claimed, evidence strings, code actually found, executed?, verdict)

| Row | id | Statuses claimed (w/l/m/web) | evidence (verbatim key parts) | Code actually found | Executed? | Verdict |
|---|---|---|---|---|---|---|
| 118 | afeat-spatializer | ns/ns/ns/ns (1705) | "" (1706) | None in engine; UI SpatializerTool.tsx = staged STUB (W15-A09); design SVG exists | Nothing to execute; parity-report audio-feature 0/11 | Claim ACCURATE; feature absent (UI staging STUB) |
| 119 | afeat-stereowiden | ns/ns/ns/ns (1718) | "" (1719) | None in engine; StereoWidenerTool.tsx = staged STUB (W15-A09) | Nothing; parity 0/11 | Claim ACCURATE; feature absent (UI STUB) |
| 120 | afeat-resampler | ns/ns/ns/ns (1730) | "" (1731) | MicroResampler (drift-only ±1000ppm) — real, executed by W07-A10 (12 tests pass), but process() UNREACHABLE in shipped build; NOT the claimed general resampler | W07-A10 ran `cargo test -p audio-engine --lib resample` → 12/0; channel_map 7/7 baseline | Claim ACCURATE (honest per W07-A10); adjacent code REAL-UNPROVEN/UNREACHABLE |
| 121 | afeat-stereo-mode | ns/ns/ns/ns (1742) | "" (1743) | channel_map.rs swap_stereo/mono_to_stereo/stereo_to_mono exist, production-unreachable (W07-A07) | channel_map tests 7/7 baseline (:25-31 per W07-A07) | Claim ACCURATE shipped; vocabulary UNREACHABLE |
| 122 | afeat-visualizations | ns/ns/ns/ns (1755) | "" (1756) | No engine code; UI VisualizerTool staged with hard-coded demo levels (W15-A07) | Nothing; parity 0/11 | Claim ACCURATE; feature absent (UI staged, demo data) |
| 123 | afeat-replaygain | ns/ns/ns/ns (1767) | "" (1768) | None in engine; settings UI only | Nothing | Claim ACCURATE; feature absent |
| 124 | afeat-passthrough | ns/ns/ns/ns (1780) | "" (1781) | None (rg spdif → 0 in crates/apps) | Nothing | Claim ACCURATE; feature absent |
| 125 | afeat-audio-device-switch | ns/ns/ns/ns (1792) | "" (1793) | Catalog ids only (ids.rs:121-122); zero host handlers; NullBackend shipped (W8); enumeration lives in orphan crates (W8-A01/A03/A02) | Nothing; rc-smoke engine-info shows mjpeg/pcm decoders, no audio device path | Claim ACCURATE; feature absent — W8 corroboration |
| 126 | vfeat-aspect-ratio (P0) | ns/ns/ns/ns (1809) | "" (1810) | renderer-wgpu params.rs REAL math (Fit default live on headless present path); 3 of 4 AspectModes + user override DEAD (W06-A04 F-001); catalog id video.set-aspect-ratio has no dispatcher | W06-A04: baseline params 11/11 (:5215-5255); placement math recomputed independently | Claim ACCURATE shipped; library surface UNREACHABLE |
| 127 | vfeat-crop | ns/ns/ns/ns (1822) | "" (1823) | params.rs crop Rect path dead (W06-A04); CropTool.tsx UI local-only, no command send (W16-RR3) | W16-RR3: vitest CropTool 9 tests (ui-test.txt:145) | Claim ACCURATE shipped; UI SHALLOW-local |
| 128 | vfeat-zoom-scale | ns/ns/ns/ns (1835) | "" (1836) | Catalog id only: crates/commands/src/catalog/ids.rs:128 `VideoSetZoom => "video.set-zoom"`, command.rs:231 "display scaling, `0.25..=4.0` (class S)"; no handler, no renderer zoom | Nothing | Claim ACCURATE; command PAPER-ONLY |
| 129 | vfeat-colors-adjust | partial/partial/partial/ns (1848) | "crates/video-engine/src/adjust.rs (…byte-exact neutral identity, CPU reference apply_rgba8; commit 50a7ba6; 48/0 crate tests at that commit; ui pending sliders awaiting video.set-adjust)" (1849) | adjust.rs exists, REAL validated parameter model + CPU reference (W03-A08: 12 tests, all pass; hue rotation mathematically approximate — F-002); exported lib.rs:184; ZERO production callers; no video.set-adjust catalog id (rg → 0); UI sliders inert-with-reason (effects.tsx:37-42, EssentialTool.tsx:26) | Baseline cargo-test-workspace.txt:6458-6469 (12 adjust tests ok) and :6507 (48 passed; 0 failed) | Evidence VERIFIED (commit UNVERIFIED — no git); feature UNREACHABLE; "partial" disclosed-generous |
| 130 | vfeat-sharpen-gradfun | ns/ns/ns/ns (1861) | "" (1862) | None in engine; EssentialTool staged sliders only (sharpen-sigma :92, banding-radius :102, no commands) | Nothing | Claim ACCURATE; feature absent |

Notes: ns = not_started. Row 129's own notes (1850) admit: "Partial: parameter model + CPU
reference only — GPU/shader-uniform wiring [PROPOSAL] and the video.set-adjust catalog command
(D-012) pending … UI sliders render inert-with-reason until the command lands." That is a
correct description of an unreachable feature — the row is honest, and it is the only row in
this chunk with any substance behind it.

## 6. Row-count / boundary proof
`rg -n '^\[\[feature\]\]' docs/feature-matrix.toml` → 252 rows; 118th match at line 1696
(afeat-spatializer), 131st at 1865 (vfeat-color-fx) — chunk = lines 1696-1864, 13 rows exactly.
Per-category totals in audit/runs/baseline/xtask-feature-audit.txt (audio-feature=11,
video-feature=10) are consistent with this chunk's 8+5 split.
