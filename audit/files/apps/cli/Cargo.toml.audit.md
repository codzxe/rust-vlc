# AUDIT: apps/cli/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W13-A04 |
| Date | 2026-11-24 |
| Lines / functions | 71 / config keys: package(6), lib(2), bin(2), dependencies(17+2), dev-dependencies(1), lints(1) |
| File verdict | REAL (accurate manifest; one stale header comment) |
| Reachable from shipped entry point? | This IS the shipped entry point's manifest: bin `lumen` (baseline release lane `cargo build --release --locked -p lumen-cli`). Binary executed repeatedly this session (bin/lumen). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg usage count per dependency (all 19 used — §2); bin/lumen engine-info → `version: 0.1.3` matches `version = "0.1.3"`; sed verification of quoted lines. |

## 1. One-paragraph verdict
An honest manifest: exactly the 2 external deps the header names (serde_json, toml — verified present and counted in use), 17 internal workspace deps ALL genuinely used (rg counts 3–131 uses each, none unused), no `[features]` section (nothing in the CLI is feature-gated — the full decoder set compiles into the shipped `lumen` binary, confirmed behaviorally: rust_h264 decodes, symphonia AAC decodes), minimal dev-deps (testkit), workspace lints. The one defect is narrative: the header still describes the codecs dependency as "MJPEG/PCM decoders behind the Decoder contract" — the first-vertical-era story — while the same manifest's default feature graph (crates/codecs/Cargo.toml `default` includes codec-h264-sw, codec-aac, …, codec-vp9) ships twelve decoder families and the CLI dispatches all of them. Same stale-era narrative family as engine-info's hardcoded `decoders: mjpeg, pcm_s16le` (host.rs:1643) and bench.rs's "first vertical: mjpeg + pcm" (bench.rs:334).

## 2. What actually works (verified)
— Dependency-usage census (rg `crate::` per dep over apps/cli/src + tests): audio-engine 5, audio-output 3, codec-api 21, codecs 19, commands 131, demux 54, input 5, media-clock 37, media-probe 3, media-types 104, playback-engine 33, plugin-api 30, renderer-wgpu 6, settings-schema 9, streaming 10, video-engine 7, serde_json 6, toml 24. Zero unused dependencies.
— `bin/lumen engine-info` → `version: 0.1.3`, `host: cli` — matches package name/version fields.
— The shipped binary contains rust_h264-0.6.1 (strings) and decodes h264+aac MP4 end-to-end (mp4_h264_family_lane passed) — proving the default feature graph reaches the shipped artifact with no gate.

## 3. Findings
### F-001 [S4] Header describes the codecs dependency as "MJPEG/PCM decoders" — stale by twelve decoder families
Where: apps/cli/Cargo.toml:11-12
Code (sed-verified):
```toml
# media-probe + input + demux (the open path), codecs + codec-api
# (MJPEG/PCM decoders behind the Decoder contract), media-clock +
```
What is wrong: the codecs crate's default features (crates/codecs/Cargo.toml:47-59) compile codec-aac (symphonia-codec-aac), codec-h264-sw (rust_h264 0.6.1), alac, av1 (rav1d), dnx, flac, mp2, mp3, opus (ruopus), vorbis, vp9 — and apps/cli/src/pipeline_host.rs:55-66 imports and dispatches AacDecoder, AdpcmDecoder, AlacDecoder, FlacDecoder, Mp2Decoder, Mp3Decoder, OpusDecoder, VorbisDecoder, Av1Decoder, DnxDecoder, H264SwDecoder, Vp9Decoder. The manifest comment describes the pre-7.K1 first vertical. Why it matters: this comment is the first place an auditor or maintainer reads what `lumen` can decode; it undersells the artifact by a whole codec registry and reinforces the same false "mjpeg+pcm" story that engine-info and decode-bench print at runtime (my mp4.rs audit, F-002 there; bench.rs:334). Confidence: HIGH — Verification: read + rg + ran (binary decodes h264/aac).

### F-002 [S4] Version-spec style drift between the two external deps
Where: apps/cli/Cargo.toml:64-65
Code (sed-verified):
```toml
serde_json = "1.0"
toml = "1"
```
What is wrong: cosmetic inconsistency ("1.0" vs "1"); no functional impact (both resolve to latest 1.x under the workspace lock). Confidence: HIGH — Verification: read.

## 4. Config-key inventory (every key)
| Key | Verdict | Notes |
|---|---|---|
| package.name = "lumen-cli" | REAL | avoids bin-name collision, per :4-6 comment — verified (bin is `lumen`) |
| package.version = "0.1.3" | REAL | matches runtime engine-info output |
| package.edition = "2024" | REAL | compiles on the workspace toolchain (baseline builds clean) |
| package.publish = false | REAL | local app crate |
| package.description | REAL | accurate ("headless host") |
| lib (name lumen_cli, path) | REAL | consumed by desktop/frame_tap (`use lumen_cli::pipeline_host::PresentTap` — frame_tap.rs:34) |
| bin (name lumen, path main.rs) | REAL | the shipped binary, executed |
| 17 path dependencies | REAL | all used (§2 census) |
| serde_json = "1.0" | REAL | 6 uses (control-stdio/json event surfaces) |
| toml = "1" | REAL | 24 uses (settings validation seam, as the header claims) |
| dev-dependencies: testkit | REAL | used by every #[cfg(test)] suite (ran: mp4/adts/diagnostics/mailbox/pipeline/protocol all green) |
| lints workspace = true | REAL | clippy -D warnings CLEAN per baseline |

## 5. Test honesty
N/A (manifest). Note: the crate's test suites (which this manifest builds) were executed this session — lumen_cli binary: containers::mp4 11 passed, containers::adts 7 passed; conformance mp4_h264_family_lane 1 passed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "External deps (2, both already Lead-admitted …): serde_json … toml" | :20-30 | REAL | exactly 2 externals in [dependencies]; both counted in use |
| "codecs + codec-api (MJPEG/PCM decoders behind the Decoder contract)" | :11-12 | FALSE (stale) | pipeline_host.rs:55-66 + h264 arm :346; default features include codec-h264-sw/codec-aac; binary decodes h264+aac (ran) |
| "The bin target is `lumen` … package name `lumen-cli` so … no collision" | :4-6 | REAL | Cargo.toml:33/44; `cargo run -p lumen-cli` builds bin lumen |
| "audio path over the null device — no audio hardware in this sandbox" | :13-14 | REAL | engine-info `features: audio.null-device`; null ring in pipeline_host |
| "serde_json: the command layer's OWN wire format … --control-stdio … --json" | :22-26 | REAL | 6 serde_json uses in the command surfaces |

## 7. Dead / duplicated / unreachable code
None: no unused deps, no feature-gated dead lanes, no `[features]` at all.

## 8. Dependencies & flags
No feature keys anywhere in this manifest — the CLI consumes every dep with default features; codecs' `default` = the full codec set (crates/codecs/Cargo.toml:47-59), so the shipped `lumen` binary carries all decoders unconditionally. Release lane (`-p lumen-cli -p desktop`, no extra features) therefore reaches every decoder arm — no UNREACHABLE-by-flag code in the CLI.

## 9. Open questions for the next agent
- Who owns the "first vertical: mjpeg + pcm" narrative strings (host.rs engine_info hardcode, bench.rs:334)? Those are runtime lies relative to this manifest's real graph; this file's comment is the third copy of the same stale story.
