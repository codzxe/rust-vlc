# AUDIT: product.toml
| Field | Value |
|---|---|
| Auditor | W01-A07 |
| Date | 2026-11-23 |
| Lines / functions | 244 / 14 sections, 51 leaf keys |
| File verdict | SHALLOW (parses, validates, and several sections are genuinely consumed; headline enforcement + exposure claims are FALSE — see F-001..F-004) |
| Reachable from shipped entry point? | PARTIAL: [window] read at runtime by apps/desktop (window_config.rs); [features].control-server read by apps/control-server:150; lumen-cli reads NOTHING (no product-config dep; queue caps are mirrored constants). Validators: config-validator (run only by hand — not in full_gate/CI), web-budget ([web.size_budgets], full_gate step 10 + CI) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 1 |
| Commands run | rg cross-checks (consumers of every key; exit 0); sed quote verification (exit 0); baseline citations: xtask-config-validator.txt, full_gate.txt. No builds (spec §2) |

## 1. One-paragraph verdict
The file is structurally honest — every range-checked value sits inside its schema bound (verified against product.schema.json), telemetry is genuinely all-off, [web.size_budgets] is really enforced by the web-budget gate, and [window] really flows into the desktop shell. But its three headline promises are false: (1) `audio-output = true` claims "default audio device output" while no OS audio backend exists anywhere — the audio-output crate ships a null sink plus typed not-supported stubs and `AUDIO_ADR005_ADMITTED: bool = false` (crates/audio-output/src/cpal.rs:43), the desktop app contains zero audio code, only the web host has a real AudioContext/worklet path — a direct violation of the MP §5 rule the file itself quotes ("Feature exposure lists only implementations that exist"); (2) "a violation is a build-breaking error" is false — config-validator is wired into neither scripts/full_gate.sh nor .github/workflows/ci.yml, and the baseline shows 4 live unenforced violations (missing brand assets); (3) rule F1/F2 ("a feature id not in the engine's static feature registry is a VALIDATION ERROR"; "the validator cross-checks exposure against the registry") is paper-only — the checks exist in product-config/src/semantic.rs:404-425 but the only callers passing a registry are product-config's own tests, and config_validator.rs:19-20 admits v0 only checks booleanness. Additionally the "every surface that shows a name reads from here at runtime" claim is false for the most visible surface: `lumen --version` prints `env!("CARGO_PKG_VERSION")` (apps/cli/src/help.rs:101) from Cargo.toml, not display_version, and the CLI hardcodes mirrored queue-cap constants (demux_host.rs:63-78) instead of reading [limits].

## 2. What actually works (verified)
— [web.size_budgets] consumed and enforced with no code-side default (web_budget.rs:315-340; verified in the web_budget audit; full_gate step 10 + ci.yml:659).
— [window] all 8 keys parsed at runtime by apps/desktop's own std-only reader (apps/desktop/src/window_config.rs:93-141, parse_window_table) — a real consumer.
— [features].control-server is a REAL runtime gate: apps/control-server/src/config.rs:150-152 `match product.features.get("control-server") { Some(true) => Ok(()), Some(false) => Err(AppError::FeatureDisabled ...`.
— config-validator really validates when run: baseline xtask-config-validator.txt:6-18 (16 tables walked, 4 real brand-asset violations found).
— streaming-http = true is real: apps/cli/src/demux_host.rs:461-488 opens http(s) via streaming::ranged::RangedResource with real-socket tests (mockhttp, demux_host.rs:373).
— All [limits] values inside schema ranges; telemetry all-off; identity ids = reverse_dns + "." + PascalCase(name) consistent across windows/macos/linux/web; schema_version 1 == empty [migrations] (validator rule, baseline clean).
— verified by rg: range cross-check vs product.schema.json:217/223 etc.; consumers enumerated per-key (§4).

## 3. Findings
F-001 [S2]
Where: product.toml:150 (and 131-133, 142-145) vs crates/audio-output/src/cpal.rs:43, stub.rs:1-9, apps/desktop/src/* (no audio code)
Code:
```
audio-output = true             # default audio device output
```
```
pub const AUDIO_ADR005_ADMITTED: bool = false;
```
What is wrong: the feature table exposes "default audio device output" as existing. Reality: no OS audio library is linked anywhere ("no OS audio library is imported anywhere in the workspace; the only backends are the null backend and the typed not-supported stubs" — cpal.rs module docs); stub.rs is the "typed not-supported stub for real-device backends"; rg finds zero audio code in apps/desktop; the CLI lane renders to the null sink. Only apps/web has a real device path (AudioContext + audio-worklet, apps/web/src/audio/). Why it matters: the file's own quoted hard rule (L132-133 "Feature exposure lists only implementations that exist. Config can never enable a missing codec, protocol or hardware capability") is violated by the file itself; every consumer that trusts [features] for capability reporting is lied to; and the F2 validator that would reject exactly this (`feature is exposed as enabled but its implementation does not exist (rule F2)`, semantic.rs:417-425) is never invoked (F-003). What correct looks like: `audio-output = false # web host only; native device backends land with ADR-005` or per-host exposure. Confidence: HIGH — Verification: read (native stub verified; desktop build UNVERIFIED — no GTK in sandbox — but no audio code path exists to build).
F-002 [S2]
Where: product.toml:4-7 (claim) vs scripts/full_gate.sh (steps 1-11), .github/workflows/ci.yml
Code:
```
# Single source of truth for product decisions (MP §5). Validated by
# product.schema.json via `cargo xtask config-validator`; a violation is a
# build-breaking error with an actionable diagnostic (MP §5), never a silent
```
What is wrong: `rg -n "config-validator" scripts/full_gate.sh .github/workflows/ci.yml` returns ZERO hits — the validator is not wired into the full gate or any CI job. A product.toml violation breaks no build. The baseline proves it: 4 violations (missing brand assets) exist in the committed tree (xtask-config-validator.txt:14-17, "result: FAILURE (4 violation(s))") and no gate would have flagged them — the orchestrator had to run config-validator by hand. Why it matters: the "never a silent fallback" guarantee is itself a silent fallback — range violations, unknown keys, or a broken identity can be committed with a green CI. What correct looks like: add `cargo xtask config-validator` as a full_gate step + CI job (like sbom/web-budget). Confidence: HIGH — Verification: rg (exit 0, no hits) + baseline.
F-003 [S3]
Where: product.toml:137-139 and 169-171 (claims) vs tools/xtask/src/cmd/config_validator.rs:19-20, crates/product-config/src/semantic.rs:404-425
Code:
```
# A feature id that is not in the engine's static feature registry is a
# VALIDATION ERROR here (rule F1) — you cannot even name an unimplemented
```
```
//! * Rule F1/F2 (feature ids vs the engine feature registry) needs the
//!   registry surface; v0 checks that every `features.*` value is boolean.
```
What is wrong: the promised validation does not run. The F1/F2 machinery exists (`feature_checks` in semantic.rs pushes "unknown feature id (rule F1...)" and "exposed as enabled but its implementation does not exist (rule F2)" diagnostics), but the only callers passing `feature_registry` are product-config's own tests (rg: semantic.rs tests only). The shipped validator checks only that values are booleans. Why it matters: product.toml could name `features.foo-bar = true` for a feature that exists nowhere and nothing fires — this is precisely the guard F-001 slipped through. What correct looks like: config-validator builds a FeatureRegistry from the engine's static registry and calls validate_semantics. Confidence: HIGH — Verification: read + rg callers.
F-004 [S3]
Where: product.toml:13-15 (claim) vs apps/cli/src/help.rs:101, apps/cli/src/demux_host.rs:63-73
Code:
```
# values (§8). Every surface that shows a name reads from here at runtime;
# packaging metadata is regenerated from here at build time.
```
```
    format!("lumen {}\n", env!("CARGO_PKG_VERSION")) // lumen-allow-literal: CLI argv0/usage prose — identity wiring pending (see help.rs [PROPOSAL])
```
```
pub const PRODUCT_QUEUE_PACKETS_AUDIO: u64 = 1024;
/// The video half of the same mirror.
pub const PRODUCT_QUEUE_PACKETS_VIDEO: u64 = 512;
```
What is wrong: the CLI — the product's most visible surface — reads neither display_version nor [limits]: `lumen --version` prints the Cargo.toml version (a duplicate "0.1.3" that can drift; version_code=4 lives only here), and the queue caps are hardcoded mirrors (PRODUCT_QUEUE_PACKETS_AUDIO/VIDEO at demux_host.rs:71-73) whose own comment admits "The CLI loads no product-config yet". lumen-cli has no product-config dependency at all (rg of apps/cli/Cargo.toml: no hit). Why it matters: "single source of truth" is false for the CLI lane; changing cache/queue limits in product.toml changes nothing in the shipped CLI behavior until the promised join lands. What correct looks like: either the join or an honest header ("CLI mirrors [limits] until card X"). Confidence: HIGH — Verification: read + rg.
F-005 [S3]
Where: product.toml:225-231 ([urls]), 179-180, 193 ([playback]), 200-208 ([limits])
Code:
```
[urls]
# Rendered in the Help menu and About (doc 05 §8.3); schema enforces https://.
```
What is wrong: declared-but-never-read keys. `update_feed` has zero consumers repo-wide (rg over apps/ crates/ ui/); help/support/privacy are consumed only by the validator's telemetry→privacy cross-check — no Help-menu reader exists in ui/ or apps/ (the URL literals appear nowhere else; rg "lumen-player.example.org" → product-config/src/patterns.rs only). preferred_audio_languages / preferred_subtitle_languages / default_rate: product-config tests only. probe_max_bytes / max_open_sessions / max_playlist_items / max_wasm_extension_memory_mib / cache_network_mib: no runtime reader (probe has no bound from here; sessions/playlists unbounded by it). Why it matters: config that is validated but never applied invites silent drift — the schema guarantees shape, nothing guarantees effect. What correct looks like: land the consumers or mark the keys [PROPOSAL]/parked like defaults.theme is. Confidence: HIGH — Verification: rg per key (consumer lists in §4).
F-006 [S4]
Where: product.toml:108, 113
Code:
```
theme = "dark"
...
profile = "default"
```
What is wrong: [defaults] points at files that do not exist — themes/ has no *.toml and profiles/ does not exist (baseline xtask-config-validator.txt:10-11, "enum injection skipped ... parked plane"). The validator is honest about it, but the committed default references a nonexistent artifact. Confidence: HIGH — Verification: baseline + LS themes/ (no *.toml found by validator).

## 4. Per-key consumption table (declared vs consumed)
| Key (section.leaf) | Parsed/validated | Runtime consumer | Verdict |
|---|---|---|---|
| schema_version | yes | validator (== max migrations.to) | REAL |
| identity.name/short_name/publisher/display_version/version_code/description_key | yes | control-server (identity), rebrand xtask; CLI version does NOT read display_version (F-004) | SHALLOW |
| identity.ids.* (7 keys) | yes | validator derivation check; packaging (build-time, UNVERIFIED) | REAL-UNPROVEN |
| window.* (8 keys) | yes | apps/desktop window_config.rs (runtime) | REAL |
| brand.accent_color | yes | desktop window_config (hit); theme-token join [VERIFY] pending | REAL-UNPROVEN |
| brand.icon_ico/icns/png_512/logo_svg | yes | validator existence check — 4 ASSETS MISSING (baseline) | FALSE (files absent) |
| defaults.theme/locale/profile | yes | locale → validator cross-check (en-US exists); theme/profile → parked planes (F-006) | SHALLOW |
| telemetry.enabled/crash_reports/usage_stats/update_checks | yes | validator (enabled⇒privacy); no runtime consumer | SHALLOW |
| features.* (20 keys) | booleanness only | control-server reads `control-server` (real gate); NOTHING else reads the table at runtime; F1/F2 unwired (F-003) | SHALLOW |
| playback.preferred_audio_languages / preferred_subtitle_languages | yes | tests only | PAPER-ONLY (runtime) |
| playback.subtitle_mode | yes | settings-schema codegen | REAL-UNPROVEN |
| playback.hw_decode_policy | yes | codec-api/src/capability.rs | REAL |
| playback.start_paused | yes | CLI has its own flag (not read from here); apps/web host hit | SHALLOW |
| playback.default_rate | yes | tests only | PAPER-ONLY (runtime) |
| limits.queue_packets_audio/video | yes | MIRRORED constants at apps/cli/src/demux_host.rs:71-73 (F-004) — not read | FALSE (as "source of truth") |
| limits.cache_demux_mib | yes | crates/input comments/tests (ceiling semantics); not read from product.toml at runtime | SHALLOW |
| limits.cache_network_mib / probe_max_bytes / max_playlist_items / max_open_sessions / max_wasm_extension_memory_mib | yes | no runtime reader | PAPER-ONLY (runtime) |
| web.size_budgets.* (2) | yes | xtask web-budget (verified, file 1) | REAL |
| urls.help/support/privacy/update_feed | yes | validator only; no Help-menu reader; update_feed zero consumers | PAPER-ONLY (runtime) |
| migrations.entries | (empty) | validator rule | REAL (honest empty) |

## 5. Feature exposure vs implementation (the requested cross-check)
| feature id | exposed | implementation found | verdict |
|---|---|---|---|
| core-playback | true | rc-smoke: command-layer-play PASS, headless-decode PASS, mp4-smoke state walk exit 0 (baseline) | REAL |
| local-file | true | open path + MP4 smoke (baseline mp4-smoke.md) | REAL |
| audio-output | true | native: null sink + typed stubs, ADR-005 NOT admitted (cpal.rs:43); desktop: no audio code; web: real worklet | FALSE (native lanes) — F-001 |
| video-render | true | desktop frame_tap→mailbox→CanvasBridge→LUF1→webview canvas (frame_tap.rs:1-30); desktop build UNVERIFIED (no GTK); web honestly lacks it (lumen-web: "headless counting presenter") | REAL-UNPROVEN |
| track-selection | true | tracks.set-video/audio/subtitle-track in command catalog (commands/src/catalog/command.rs:153-167) | REAL-UNPROVEN (catalog verified, no live run) |
| config-surface | true | config.get/set/reset-setting (command.rs:407-426) + settings-schema crate + ui/src/screens/settings | REAL-UNPROVEN |
| streaming-http | true | demux_host.rs:461-488 ranged resource + real-socket tests | REAL |
| playlists/library/streaming-hls/dash/rtsp/optical-disc/device-capture/audio-device-enumeration/recording/transcode/wasm-extensions/control-server/hw-decode | false | consistent with parked planes; control-server app reads its false flag and refuses to start (config.rs:150) | REAL (honest) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a violation is a build-breaking error" | product.toml:5-6 | FALSE | config-validator absent from full_gate.sh + ci.yml (rg zero hits); baseline 4 unenforced violations (F-002) |
| "Feature exposure lists only implementations that exist" | product.toml:132-133 | FALSE | audio-output=true vs stub-only native backends (F-001) |
| "A feature id ... not in the engine's static feature registry is a VALIDATION ERROR here (rule F1)" | product.toml:137-139 | FALSE (unwired) | config_validator.rs:19-20 admission; semantic.rs registry only called in tests (F-003) |
| "the validator cross-checks exposure against the registry (§3.4)" | product.toml:169-171 | FALSE | same proof as F-003 |
| "Every surface that shows a name reads from here at runtime" | product.toml:14 | FALSE | help.rs:101 CARGO_PKG_VERSION; demux_host.rs:71-73 mirrored constants (F-004) |
| "UIs grey out non-exposed commands from data (system.list-commands + get-engine-info), never from hardcoded lists" | product.toml:139-140 | REAL-UNPROVEN | commands catalog exists; UI greying behavior not executed by me (UNVERIFIED — UI audit owns it) |
| "[web.size_budgets] ... web-budget --check reads THESE values" | product.toml:213-216 | REAL | web_budget.rs:315-340 (file-1 audit) |
| "schema 480..=3840 / 320..=2160 / 4..=512 ..." range comments | product.toml:64-71, 201-208 | REAL | product.schema.json ranges match; all values in range (read) |
| "Opt-in only, default OFF" telemetry | product.toml:119-129 | REAL | all four values off (read) |

## 7. Dead / duplicated / unreachable code
- Duplicated truth: apps/cli/Cargo.toml version "0.1.3" duplicates identity.display_version; PRODUCT_QUEUE_PACKETS_AUDIO/VIDEO duplicate limits.queue_packets_* (drift risk, F-004).
- Dead (never read at runtime): urls.update_feed; playback.default_rate; playback.preferred_*; limits.probe_max_bytes / max_open_sessions / max_playlist_items / max_wasm_extension_memory_mib / cache_network_mib (F-005).
- Unreachable validation: product-config semantic.rs feature_checks (F1/F2) — no non-test caller passes a registry.

## 8. Dependencies & flags
- Read by: xtask config-validator + web-budget (toml_lite); apps/desktop window_config.rs (own parser); apps/control-server (product-config crate); xtask sbom (subject "Lumen 0.1.3" per baseline sbom output); settings-schema codegen.
- NOT read by: apps/cli (no product-config dependency — F-004), apps/web, ui/ (no hits).
- product.schema.json ranges honored (verified per key).

## 9. Open questions for the next agent
- Who owns wiring config-validator into full_gate/CI (F-002) and building the engine FeatureRegistry for F1/F2 (F-003)? Likely the config-validator owner + a Lead decision on where the static registry lives.
- Desktop audio/video execution is UNVERIFIED in this sandbox (gdk-sys build failure — environment limit, not a repo defect). The desktop video path code exists (frame_tap.rs); its live behavior needs a GTK-capable machine.
- ui/src contains screens for playlist/convert/discovery/network-stream while features.playlists/transcode/etc. are false — check whether the UI ships unreachable screens (UI-domain agent).
- Does the engine-info "features" list (apps/cli/src/run.rs:633) come from a static registry that could feed FeatureRegistry::from_status? (Candidate wiring point for F1/F2 — UNVERIFIED.)
