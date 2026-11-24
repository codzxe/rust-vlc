# AUDIT: apps/web/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W12-A02 |
| Date | 2026-11-24 |
| Lines / functions | 126 / 1 package, 1 lib, 12 deps, 2 target tables, 10 features, 1 dev-dep |
| File verdict | REAL — declarations verified against boundaries.toml, codecs features, and an executed default wasm32 build |
| Reachable from shipped entry point? | YES — lumen-web is the wasm32 composition root (card 7.H1); shipped as lumen_web.wasm loaded by the UI worker. Baseline rc-smoke lane 6 builds it; I rebuilt it (exit 0). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo build -q -p lumen-web --target wasm32-unknown-unknown --locked` -> exit 0 (default features = decoder-less); `rustup target list --installed` -> wasm32-unknown-unknown present; rg over boundaries.toml / crates/codecs/Cargo.toml / audit/runs/baseline/{full_gate,rc-smoke}.txt |

## 1. One-paragraph verdict
This manifest's unusually long comment block is unusually accurate: every checkable claim checks out. plugin-api is a dep and the web app really uses its demux/access contracts (apps/web/src/open.rs:32, staged_source.rs:63); video-engine is absent as stated; the target-split codecs edge matches the boundaries.toml row exactly (`[crates.lumen-web] internal_deps = [audio-engine, audio-output, codec-api, codecs, commands, demux, media-clock, media-probe, media-types, playback-engine, plugin-api]`, external_allowlist `[serde, serde_json]`); the shipped wasm module is built with NO codec features (baseline lane 6 command has no `--features`, and I rebuilt the same command successfully, exit 0). Two nits: the "native gets MJPEG + PCM" comment understates that native default links ALL external decode engines (rav1d, rust_h264, symphonia family…), and the rav1d "39 errors" record is cited but not re-verifiable cheaply. Critically for my domain: NO dependency on plugin-host-wasm — the browser app structurally cannot load `.lumenext` extensions.

## 2. What actually works (verified)
— verified by `cargo build -q -p lumen-web --target wasm32-unknown-unknown --locked` -> exit 0, artifact target/wasm32-unknown-unknown/debug/lumen_web.wasm (proves the "every crate below is proven wasm32-clean" claim for the default/decoder-less floor).
— verified by rg: boundaries.toml:613-617 matches this manifest's dep set 1:1; the baseline arch-gate failure (full_gate.txt:43) is lumen-cli→streaming, NOT this crate — lumen-web's edges pass.
— verified by read: no `default` in [features] (L92-121) ⇒ shipped module enables zero codec features, exactly as claimed L85-88; `codec-mp2 = ["codec-mp3"]` exists at crates/codecs/Cargo.toml:77, matching the "one lane, both decoders" comment (L104-106); rav1d absent from the feature list, matching L113-117.

## 3. Findings
F-001 [S4]
Where: apps/web/Cargo.toml:13-15
Code:
```
# builds link the codecs crate unconditionally and get the real MJPEG
# + PCM decoders (the first vertical) so the headless lane proof
# decodes for real.
```
What is wrong: Understatement presented as the story: native `codecs` links with DEFAULT features = the full external engine set (crates/codecs/Cargo.toml:47-59: aac, alac, av1/rav1d, dnx, flac, h264-sw/rust_h264, mp2, mp3, opus, vorbis, vp9), not merely "MJPEG + PCM". A reader concludes the native web lane is lean when it drags in every external decoder. Why it matters: misleads size/security review of the native lane; the wasm lane's careful tree-shaking has no native counterpart documented. What correct looks like: state that native default = full codec set, mjpeg/pcm/adpcm unconditional. Confidence: HIGH — Verification: read codecs/Cargo.toml default table.

F-002 [S4]
Where: apps/web/Cargo.toml:19-21
Code:
```
# all build on wasm32-unknown-unknown (rav1d 1.1.0 fails: 39 errors,
# measured 2026-11-10 — asm-gated core code does not compile for the
# target), so the wasm module SHIPS decoder-less
```
What is wrong: The measured-record claim is not re-verifiable within audit budget (would require attempting a wasm32 rav1d build). The cited record exists (docs/web/size-report.md present in tree) — UNVERIFIED, not disputed. The SHIPS-decoder-less half IS verified (default build, no features, exit 0; baseline lane 6 command identical). Confidence: HIGH on the claim's documentation, UNVERIFIED on the 39-error measurement. Verification: read + build.

## 4. Per-section table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| package/lints | 36-44 | REAL | – | workspace lints inherited |
| lib crate-type cdylib+rlib | 49-50 | REAL | – | rlib feeds native lane tests (staged_source.rs:826 test uses plugin_api::Demuxer) |
| [dependencies] (12) | 52-71 | REAL | – | 10 path + serde/serde_json only; matches boundaries.toml row; plugin-api used for Demuxer/SourceStream contracts |
| target not-wasm32 codecs | 78-79 | REAL | S4 | F-001: default = full engine set |
| target wasm32 codecs optional | 89-90 | REAL | – | default-features=false as documented |
| [features] (10) | 92-121 | REAL | – | no default ⇒ decoder-less floor; codec-mp3 lane wires mp2+mp3 correctly; rav1d absent |
| [dev-dependencies] testkit | 123-126 | REAL | – | dev-only per ARCH-7 |

## 5. Test honesty
No tests in this file. Downstream verification: native lane tests (apps/web/src/*) link the rlib; wasm floor build executed by me (exit 0).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the wasm module SHIPS decoder-less" | L22-23, 85-88 | REAL | default build (no --features) exit 0; baseline rc-smoke lane 6 uses the same bare command |
| "video-engine is deliberately ABSENT" | L57-59 | REAL | absent from [dependencies] (L52-71) and boundaries row |
| "this crate adds none beyond the command layer's own serde wire family" | L9-11, 31-34 | REAL | only serde+serde_json external; allowlist matches |
| "it stays boundary-checked and allowlisted in boundaries.toml" | L76-77 | REAL | boundaries.toml:613-617 row matches; arch gate's only edge failure is lumen-cli→streaming (full_gate.txt:43) |
| "rav1d 1.1.0 fails: 39 errors, measured 2026-11-10" | L20-21 | UNVERIFIED | cited record docs/web/size-report.md exists; re-measuring exceeds budget |
| "the wasm32 rust-std component installed" builds pass | AGENT-SPEC §13 / rc-smoke lane 6 | REAL | my rebuild: exit 0 (baseline log shows FAILED only because the sandbox lacked the target) |

## 7. Dead / duplicated / unreachable code
None. (Domain note: plugin-api is a live dep — its access/demux traits — but its manifest/plugin machinery is compiled into the wasm module without ever being invoked there; see manifest.rs audit F-002.)

## 8. Dependencies & flags
All path deps L1-L6 per boundaries.toml; externals serde 1 + serde_json 1 (command-layer wire family); codecs target-split (native unconditional / wasm32 optional, default-features=false); no default features; testkit dev-only. `[lints] workspace = true`.

## 9. Open questions for the next agent
- Which artifact does the SHIPPED UI actually load — the decoder-less default lumen_web.wasm or a feature-enabled measurement build? ui/scripts/web-matrix-run.mjs and scripts/rc-smoke-web.mjs reference it; the browser-matrix docs (docs/web/browser-matrix-20261110.md) should settle whether end users ever get a codec-carrying module.
- The extensions settings UI (ui/src/screens/settings/areas/extensions.tsx) — what does it manage if no shipped app can load extensions? Recommend the UI-domain auditor trace its data source.
