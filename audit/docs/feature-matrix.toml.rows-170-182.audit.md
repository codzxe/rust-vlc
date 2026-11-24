# CLAIM AUDIT: docs/feature-matrix.toml rows 170-182 (platform-feature category)
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Rows audited | 13 (rows #170-#182, file lines 2364-2536; 252 total features confirmed) |
| REAL 6 / REAL-UNPROVEN 0 / SHALLOW 6 / STUB 0 / PAPER-ONLY 0 / FALSE 1 / UNREACHABLE-code-found 3 |
| Internal contradictions found | 1 (status "not_started" vs in-tree code) |

## 1. Blunt verdict
This chunk is the platform-feature tail of the matrix (aout ×3, vout, hwdecode ×5,
web ×2, capture, disc). Every "blocked" row (capture, DVD) is truthful and well-cited.
Every "not_started" row that says "no work yet" is an UNDERCLAIM for six of them: real
in-tree code exists — 787-line WASAPI FFI, 766-line CoreAudio FFI, 1161-line PulseAudio
protocol client, a shipped software-compositor renderer in the CLI, 403-line HEVC hw
packetization, and (worst) ~7,000 lines of VA-API/VideoToolbox H.264 hw adapters that
compile UNCONDITIONALLY in every codecs build and are settings-reachable via
`playback.hw-decode-policy`. The matrix undercounts progress in the safe direction — no
fraud, no fabricated evidence (all 13 evidence strings are empty, honestly unverified) —
but "not_started" is factually wrong as a status for rows 170-175 and 180. The status
vocabulary at the file top defines not_started = "no work yet"; that claim is false for
those rows.

## 2. Claim ledger (row-level)
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 170 | platform-aout-wasapi: windows="not_started" (= no work yet) | :2373 | SHALLOW | crates/platform-windows/src/windows/wasapi.rs exists, 787 lines: device enumeration + IMMNotification callbacks (client_on_device_state_changed etc., :471-544) — REAL code, UNREACHABLE (platform-windows is a dep of no shipped app: no Cargo.toml refs from apps/cli, apps/desktop, apps/web, playback-engine). The output path (shared/exclusive modes) itself has no code — half-true status |
| 171 | platform-aout-coreaudio: macos="not_started" | :2387 | SHALLOW | crates/platform-macos/src/macos/coreaudio.rs exists, 766 lines (AudioObjectPropertyAddress/Get/AddPropertyListener FFI :81-222) — enumeration adapter in-tree, unwired; output path absent |
| 172 | platform-aout-pipewire: linux="not_started" | :2400 | SHALLOW | crates/platform-linux/src/linux/pulse.rs (759) + pulse_ffi.rs (402) — a real PulseAudio protocol client (PaContext/PaMainloop/sink-info callbacks :48-231), the PipeWire-era path row 172's own blocked_reason describes; in-tree, unwired from shipped apps |
| 173 | platform-vout-wgpu: all="not_started"; "Unified video output across desktops" | :2414 | SHALLOW | apps/cli/Cargo.toml:60 `renderer-wgpu = { path = … }` and apps/cli/src/pipeline_host.rs:79-83 imports Renderer/SurfaceConfig/PresentMode/screenshot — a SHIPPED software-compositor video path ("the video path over the software compositor", apps/cli/Cargo.toml:15). Unified video output exists in software form; the wgpu GPU backend alone is pending (renderer-wgpu/src/lib.rs:57 "hardware backend joins when the Lead ratifies the import") |
| 174 | platform-hwdecode-h264: all="not_started" | :2427 | FALSE-as-stated | ~7.0k LOC exists and compiles unconditionally: h264/hw/vaapi/{mod,session,ffi,dl,device,library}.rs (3,894 lines incl. 2,134-line session), h264/hw/videotoolbox/{mod,session,ffi}.rs (1,466), capability.rs (584); h264/mod.rs:53 `pub mod hw;` — NO cfg gate. Settings-reachable: schemas/settings.toml:640 `playback.hw-decode-policy` + apps/cli/src/pipeline_host.rs:342 "The hw lane (`H264Decoder::from_setting`, `playback.hw-decode-policy`) stays the opt-in Phase-7 path"; decode.rs:310 `from_setting`. "No work yet" is contradicted by code+ADR+settings key. Execution UNVERIFIED (no VA-API/VT hardware in sandbox) |
| 175 | platform-hwdecode-hevc: all="not_started" | :2440 | SHALLOW | crates/codecs/src/video/hevc/hw.rs exists (403 lines): HevcHwVps/Sps/Pps/Vcl/AccessUnit packetization for adapters 7.B1-7.B3 (:7-38) — some work exists; no VA-API/VT session code for HEVC yet |
| 176 | platform-hwdecode-vp9: all="not_started" | :2453 | REAL (truthful negative) | crates/codecs/src/video/vp9/ contains only decoder.rs/error.rs (rusty_vp9 software wrapper) — no hw code found (rg dxva2/vaapi/videotoolbox within vp9: none) |
| 177 | platform-hwdecode-av1: all="not_started" | :2466 | REAL (truthful negative) | crates/codecs/src/video/av1/ = decoder.rs/obu.rs (rav1d software); no hw adapter code |
| 178 | platform-hwdecode-mpeg-family: all="not_started" | :2479 | REAL (truthful negative) | mpeg12/ = parser/facts/headers only; no hw adapter code for MPEG-2/ASP/VC-1/MJPEG |
| 179 | platform-web-webcodecs: web="not_started" | :2491 | REAL | apps/web/src/capabilities.rs:259-263 reports the row as typed blocked data: `BlockedCapability { capability: "hardware-decode", status: NotStarted, reason: "WebCodecs ADR pending (doc 09 §10); until it passes, software decode only" }` — status claim matches code reality |
| 180 | platform-web-target: web="not_started" | :2504 | SHALLOW | The web build exists and passes gates: baseline scripts/rc-smoke.sh §8 "web-ui-browser-load (vite preview + Playwright chromium)" PASS (:162-189), wasm build passes; apps/web is a real crate with host.rs/capabilities.rs/render/. "No work yet" is false; honest status would be in_progress (parity not reached — that part is true) |
| 181 | platform-device-capture: all="blocked" | :2518 | REAL | rg for dshow/v4l2/decklink/avcapture in crates/ + apps/: zero hits — nothing exists, blocked_reason cites hardware-gating honestly |
| 182 | platform-disc-dvd: all="blocked" | :2532 | REAL | rg for dvdnav/dvdread/dvdcss/bluray: zero hits; blocked_reason's μADR brief quote verified — docs/feasibility/sections/subsystems.md:604-612 "no E2 path exists" + "blocked v1 and absent a policy change, blocked v2+" |

## 3. Matrix rows (template §5 detail)
| Row | Statuses claimed | Evidence strings | Code actually found | Executed? | Verdict |
|---|---|---|---|---|---|
| 170 platform-aout-wasapi | win not_started; others unsupported | "" | platform-windows wasapi.rs 787 L (FFI enum+notify) | no (no Windows) | SHALLOW underclaim; UNREACHABLE code |
| 171 platform-aout-coreaudio | macos not_started | "" | platform-macos coreaudio.rs 766 L | no (no macOS) | SHALLOW underclaim; UNREACHABLE code |
| 172 platform-aout-pipewire | linux not_started | "" | platform-linux pulse.rs+ffi 1161 L | no | SHALLOW underclaim; UNREACHABLE code |
| 173 platform-vout-wgpu | all not_started | "" | renderer-wgpu SHIPPED in apps/cli (software compositor) | yes — baseline builds clean (release lane exit 0) | SHALLOW underclaim |
| 174 platform-hwdecode-h264 | all not_started | "" | vaapi 3.9k L + VT 1.5k L + capability 584 L, unconditional, settings key wired | UNVERIFIED (no hw substrate in sandbox) | FALSE-as-stated ("no work yet") |
| 175 platform-hwdecode-hevc | all not_started | "" | hevc/hw.rs 403 L packetization | no | SHALLOW underclaim |
| 176 platform-hwdecode-vp9 | all not_started | "" | none (sw rusty_vp9 only) | n/a | REAL negative |
| 177 platform-hwdecode-av1 | all not_started | "" | none (sw rav1d only) | n/a | REAL negative |
| 178 platform-hwdecode-mpeg-family | all not_started | "" | none | n/a | REAL negative |
| 179 platform-web-webcodecs | web not_started | "" | none; honest BlockedCapability data in apps/web | n/a | REAL |
| 180 platform-web-target | web not_started | "" | apps/web builds + browser-load PASS (baseline rc-smoke §8) | yes (baseline run cited) | SHALLOW underclaim |
| 181 platform-device-capture | all blocked (+reasons) | "" | none | n/a | REAL |
| 182 platform-disc-dvd | all blocked (+reasons) | "" | none; μADR brief verified | n/a | REAL |

## 4. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| rows 170-175,180: "not_started — no work yet" (status vocabulary, matrix header :33-34) | the tree contains 787/766/1161/403/7000-line adapters + a shipped software renderer | the code exists; the status field is wrong in the safe direction. Note the row NOTES are honest about design state (row 174: "Linux lane designed — ADR-013 [proposed]" — and docs/adr/ADR-013-va-api-hw-decoder.md exists with Status: proposed, verified) — the status field lags the notes |
| (supporting) row 174 note "avcodec ffmpeghw score 10001 (highest in dump)" | modules-linux.json | TRUE — python3: max score across all 56 capabilities = 10001 (avcodec); verified |
| (supporting) row 172 note "VLC 3.0.23 Debian ships alsa+pulse only" | modules-summary.md:171-173 | TRUE — "audio output (5: alsa, pulse, adummy, afile, amem)… Debian ships alsa+pulse only" |
| (supporting) row 173 note "23 vout/window/GL-provider registrations collapse to one" | modules-summary.md:176 | TRUE — "VLC's 23 display/window/GL-provider registrations are engine-internal" |
| (supporting) row 170 vlc_modules identity [VERIFY V1/V6] | modules-windows.json status=blocked-on-hardware, warning "NO COUNTS IN THIS FILE ARE MEASURED" | TRUE — Windows census is a skeleton; the row's [VERIFY] marks are honest |

## 5. Fabrication / inflation indicators
- None in the dangerous direction: all 13 evidence strings are empty (matches the
  baseline parity-report: 0/252 verified; no evidence fabricated for unverified rows).
- The inflation here is *deflation*: real work hidden behind "not_started". Whether that
  is deliberate humility (code exists but is unadmitted/unwired — the Lead-admission
  process in ADR-013 is genuinely pending) or ledger rot, the file's own vocabulary
  ("not_started = no work yet") makes the status fields factually wrong.
- Rows 170/171 vlc_modules identities are properly [VERIFY]-marked against the blocked
  Windows census — honest.

## 6. Severity-tagged findings
- F-MATRIX-174 [S2]: platform-hwdecode-h264 status "not_started" while ~7k LOC of
  unconditional VA-API/VT adapters + `playback.hw-decode-policy` settings wiring exist
  (crates/codecs/src/video/h264/hw/**, schemas/settings.toml:640). The parity ledger
  (the project's headline metric source) misstates the state of a P1 platform row.
- F-MATRIX-UNDERCLAIM [S3]: rows 170,171,172,173,175,180 same pattern (in-tree or
  shipped code vs "no work yet"). Fix: status in_progress with notes, or a
  "designed/coded-not-admitted" state via Lead change-control.
- F-MATRIX-EVIDENCE [S4]: rows with real code (170-175, 180) keep evidence = "" — even
  a pointer to the crate paths would make the ledger auditable without claiming
  verification.

## 7. Open questions for the next agent
- Does `H264Decoder::from_setting` actually open VA-API on real hardware? Needs a
  hardware run (UNVERIFIED here).
- The h264 hw code is unconditional while `sw` is feature-gated — asymmetric gating
  worth a design review (dead weight on wasm32? does apps/web compile the hw FFI?).
- Row 180's honest status: "in_progress" per the browser-load baseline run.

## 8. ADDENDUM (cross-agent discovery, from w24-f-gate-findings audit)
Intra-matrix contradiction confirmed by a sibling row: `codec-video-h264`
(docs/feature-matrix.toml:413-417) describes the SAME hw substrate
(crates/codecs/src/video/h264/hw/**) with status `windows/linux/macos = "partial"` and
notes "hw lanes landed … VideoToolbox compile-verified … VA-API compile-verified +
failure-path runtime-tested" (plus the OF-5 honesty-fix history citing the phase-7 exit
gate §3 OF-5), while row #174 platform-hwdecode-h264 (:2427) claims `not_started` on
every platform for "Per-OS hardware decode: Media Foundation (Win), VideoToolbox (mac),
VA-API (Linux)". Both rows describe the same code; the codec row (partial, note-rich,
gate-corrected) is the accurate one. This upgrades F-MATRIX-174's confidence to HIGH
and gives it a second intra-file proof: the matrix contradicts itself about the hw
adapters' status.
