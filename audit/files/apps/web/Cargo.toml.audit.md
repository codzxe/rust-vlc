# AUDIT: apps/web/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A10 |
| Date | 2026-11-21 |
| Lines / config keys | 126 / 30 keys (package 5, lints 1, lib 1, deps 13, target-dep tables 2, features 10, dev-deps 1) |
| File verdict | UNREACHABLE (manifest internally consistent; the crate it defines is compiled but loaded by nothing shipped) |
| Reachable from shipped entry point? | NO — the wasm module is built by `scripts/rc-smoke.sh` check 6 (baseline #20 PASS, compile-only) but no shipped surface instantiates it (proof in F-001) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n "serde_json" apps/web/src/` (dep used); `rg -n "^use ..." apps/web/src/*.rs` (all 11 path deps used); sed of `crates/codecs/Cargo.toml [features]` (all forwarded features exist); `rg -n "lumen-web" boundaries.toml` + sed 600-625 (edges match deps); `rg -n 'name = "lumen-web"' Cargo.lock` (line 434); rg over ui/src, apps/desktop, apps/desktop-tauri, .github (no wasm consumer); Read scripts/rc-smoke.sh, ui/playwright.config.ts, ui/src/client/app-transport.ts; sed of tools/xtask toml_lite.rs 240-262, 564-576 and check_architecture.rs 625-700 (multi-line array support) |

## 1. One-paragraph verdict
The manifest is internally consistent and unusually honest: every declared dependency is really imported by `apps/web/src`, every `codec-*` feature it forwards exists in `crates/codecs/Cargo.toml`, the `boundaries.toml` edge set matches the dependency tables one-for-one, and the "shipped module stays decoder-less" claim matches the code. But the crate this manifest defines — the "engine-in-Worker wasm host" — is dead weight in every shipped artifact: the web UI's own transport selection falls back to a simulated DemoTransport in the browser, no UI code ever constructs a Worker for it, and the only harness that loads the module in a real browser is excluded from CI and driven by a script no workflow invokes. One comment in the file is now factually wrong (the "rejects multi-line arrays" parser claim, F-003).

## 2. What actually works (verified)
- Dependency honesty: all 13 [dependencies] entries (commands, media-types, plugin-api, codec-api, media-probe, demux, media-clock, playback-engine, audio-engine, audio-output, serde, serde_json) + dev testkit are imported by `apps/web/src/*.rs` — verified by `rg -n "^use " apps/web/src/{lib,host,capabilities,open,protocol,source,abi}.rs` (e.g. `host.rs:74 use audio_engine::…`, `open.rs:25 use demux::avi::AviDemuxer`, `protocol.rs:47 use serde::{Deserialize, Serialize}`).
- Feature targets exist: `codec-aac/alac/dnx/flac/mp3/mp2/opus/vorbis/vp9` all defined in `crates/codecs/Cargo.toml [features]`; `codec-av1`/rav1d correctly absent from `builtin-codecs-external` (line 121), matching the "rav1d does not compile for wasm32" comment.
- Boundary conformance: `boundaries.toml:613 [crates.lumen-web]` internal_deps = exactly the 11 path deps, dev_deps=[testkit], external_allowlist=[serde, serde_json] — and baseline #11 (check-architecture) reports zero violations for lumen-web.
- The crate compiles for wasm32: baseline #20 `RC_SMOKE_SKIP_WEB=1 bash scripts/rc-smoke.sh` → PASS 6/6, which includes check 6 `cargo build -p lumen-web --target wasm32-unknown-unknown --locked` (scripts/rc-smoke.sh:130). Membership resolves: `Cargo.lock:434 name = "lumen-web"`.

## 3. Findings

### F-001 [S2]
Where: apps/web/Cargo.toml:1-2 (claim) vs ui/src/client/app-transport.ts:73-79 and ui/playwright.config.ts:21 (reality)
Code (the claim):
```
# apps/web — the engine-in-Worker wasm host (card 7.H1, issue #47;
# doc 09 §1/§2/§3/§7).
```
Code (the reality — the shipped browser UI never loads it; ui/src/client/app-transport.ts:73-79):
```
  // Browser context: the demo simulation (an honest simulated engine —
  // real wire protocol, no decoded media; badged in the app chrome).
  const transport = options.browserFallback?.() ?? new DemoTransport();
```
What is wrong: The manifest defines "the wasm32 composition root" as if it were the product's web lane, but no shipped entry point reaches it. Verified: (a) `ui/src` contains zero `new Worker(` / `lumen_web` references — the only "wasm" mention is a comment in `ui/src/client/transport.ts:37`; (b) `selectAppTransport()` (ui/src/client/app-transport.ts:58-80) picks TauriTransport or DemoTransport — never a wasm worker; (c) `apps/desktop` and `apps/desktop-tauri` contain no reference to lumen-web (rg over both trees: no matches); (d) the only real consumer, the browser-matrix harness (`ui/e2e/web/site/harness.js`), is excluded from CI's e2e (`ui/playwright.config.ts:21 testIgnore: ["**/visual/**", "**/web/**"]`) and its runner `npm run webmatrix` appears in no `.github/workflows/*.yml` (rg `webmatrix|web-matrix|playwright\.web` over .github: no matches); (e) no workflow builds wasm at all (rg `apps/web|lumen-web` over .github: no matches).
Why it matters: The entire crate (this manifest, 7+ source files, worker.js, audio-worklet.js) is UNREACHABLE from shipped entry points. The web platform column (parity web=0/252, baseline #9) is backed by a demux-only, decoder-less module that nothing shipped loads. A user opening the web UI in a browser gets the DemoTransport simulation — "no decoded media" by the app's own debug line.
What correct looks like: Either the product page wires a `new Worker` transport over this module (and a CI lane runs `npm run webmatrix`), or the manifest/docs mark the crate as a measurement/harness artifact rather than the product's web host.
Confidence: HIGH — Verification: read (rg traces above) + baseline #9 (parity web=0/252), #20 (rc-smoke compile-only PASS).

### F-002 [S3]
Where: apps/web/Cargo.toml:97-121
Code:
```
builtin-codecs = ["dep:codecs"]
```
(and lines 103-112, 121)
```
builtin-codecs-external = ["codec-aac", "codec-alac", "codec-dnx", "codec-flac", "codec-mp3", "codec-opus", "codec-vorbis", "codec-vp9"]
```
What is wrong: Ten feature gates (`builtin-codecs`, 8 `codec-*`, `builtin-codecs-external`) are enabled by no shipped build and no CI lane. The only automated wasm build, rc-smoke.sh:130, passes no `--features`; `.github/workflows/*` never builds wasm. Their sole consumer is the manually-run measurement documented in `docs/web/size-report.md:37`.
Why it matters: Feature surface no automated lane ever exercises can silently rot — a `codec-*` feature break would be invisible to every gate.
What correct looks like: A CI lane (or rc-smoke check) that builds `-p lumen-web --target wasm32-unknown-unknown --features builtin-codecs-external` at least once, so the tree-shake lanes are compile-verified.
Confidence: HIGH — Verification: read (rg over scripts/ + .github; rc-smoke.sh:130 read; size-report.md:37 read).

### F-003 [S4]
Where: apps/web/Cargo.toml:118-120
Code:
```
# Single-line array (the xtask TOML reader parses member manifests and
# rejects multi-line arrays — same convention as the root Cargo.toml
# members list; W25 fix for the CI check-architecture failure).
```
What is wrong: The claim is FALSE against current code. Both xtask TOML readers accept multi-line arrays: (1) `toml_lite.rs:246-257` implements "Multi-line continuation for arrays `[` and inline tables `{`" with a passing unit test `parses_multiline_array_and_nested_tables` (toml_lite.rs:564-575); (2) check_architecture's own parser handles newlines inside arrays (`check_architecture.rs:625-646` `parse_array` — `skip_blank` "spans newlines + comments inside the array"; `parse_opaque_array` at 658-670 explicitly documents "multi-line arrays, W25").
Why it matters: A stale constraint stated as present-tense fact in a config file makes every future editor flatten arrays for no reason and misattributes why the W25 fix worked (the fix landed in the parser, not only the manifest).
What correct looks like: Reword to past tense ("until W25 the reader rejected multi-line arrays; it no longer does — single-line kept as convention") or delete.
Confidence: HIGH — Verification: read (sed of both parsers + unit test).

### F-004 [S4]
Where: apps/web/Cargo.toml:20-22
Code:
```
# all build on wasm32-unknown-unknown (rav1d 1.1.0 fails: 39 errors,
# measured 2026-11-10 — asm-gated core code does not compile for the
# target), so the wasm module SHIPS decoder-less (the honest
```
What is wrong: The "39 errors, measured 2026-11-10" figure is not reproducible from any committed log I could find under `docs/web/` (size-report.md restates it; the raw failing cargo output is not committed), and rule 8 forbids re-running the build.
Why it matters: A load-bearing exclusion (no AV1 on wasm) rests on an unarchived manual measurement.
What correct looks like: Commit the failing build log (as `docs/gates/logs/phase7-exit-wasm.log` does for the pass) or a CI lane that demonstrates the failure.
Confidence: MEDIUM — Verification: UNVERIFIED (cannot run cargo; no committed failure log found).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| [package] name/version/edition/description/publish | 36-41 | REAL | — | publish=false, edition 2024; description matches crate contents |
| [lints] workspace = true | 43-44 | REAL | — | Root workspace.lints exists (Cargo.toml:53-66); clippy clean per baseline #5 |
| [lib] crate-type cdylib+rlib | 49-50 | REAL | — | cdylib → wasm module; rlib → native lane tests (testkit imports verified) |
| commands path dep | 54 | REAL | — | used: host.rs:82-90, open.rs:23-24, protocol.rs:45-46 |
| media-types | 60 | REAL | — | used: source.rs:22-23, open.rs:29-31, host.rs:92-94 |
| plugin-api | 61 | REAL | — | used: source.rs:24, open.rs:32 |
| codec-api | 62 | REAL | — | used: host.rs:76,81 |
| media-probe | 63 | REAL | — | used: open.rs:28 |
| demux | 64 | REAL | — | used: open.rs:25-26 — AVI demuxer only; the web open path is AVI-wired |
| media-clock | 65 | REAL | — | used: host.rs:91, open.rs:27 |
| playback-engine | 66 | REAL | — | used: host.rs:95-96 |
| audio-engine | 67 | REAL | — | used: host.rs:74 |
| audio-output | 68 | REAL | — | used: host.rs:75-76 — Null* devices only |
| serde / serde_json | 70-71 | REAL | — | used: protocol.rs:47,282; capabilities.rs:436; host.rs:1825 |
| target(not wasm32) codecs | 78-79 | REAL | — | unconditional native codecs (default features); consistent with boundaries.toml:613 |
| target(wasm32) codecs optional | 89-90 | REAL | — | default-features=false, optional; backs builtin-codecs |
| features: builtin-codecs | 97 | REAL-UNPROVEN | S3 | never enabled in any automated lane (F-002); measured only by manual size-report runs |
| features: codec-flac/mp3/vorbis/aac/alac/opus/vp9/dnx | 103-112 | REAL-UNPROVEN | S3 | same; all forward to existing codecs features (verified) |
| features: builtin-codecs-external | 121 | REAL-UNPROVEN | S3 | aggregation of the 8; single-line per stale convention (F-003) |
| [dev-dependencies] testkit | 123-126 | REAL | — | used across apps/web/src test modules (rg: 10+ files) |

## 5. Test honesty
No tests in this file (manifest). The crate's own tests ran in baseline #3/#4 (`cargo test --workspace`) — no lumen-web-specific failures reported there.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the engine-in-Worker wasm host (card 7.H1)" | apps/web/Cargo.toml:1 | UNREACHABLE | F-001: no shipped consumer; DemoTransport is the browser fallback (app-transport.ts:75) |
| "native builds … get the real MJPEG + PCM decoders" | Cargo.toml:13-16 | REAL | baseline #20 rc-smoke 4: v_decoded==v_frames on avi_mjpeg_pcm fixture |
| "the SHIPPED module enables none of them" (decoder-less wasm) | Cargo.toml:86-88 | REAL | ui/scripts/web-matrix-run.mjs:278: open fails with typed refusal `web.decoders_absent`; no feature flags in rc-smoke.sh:130 |
| "rav1d 1.1.0 fails: 39 errors, measured 2026-11-10" | Cargo.toml:20-22 | REAL-UNPROVEN | no committed build log; cannot re-run (rule 8) — F-004 |
| "check-architecture reads both sections as one normal internal edge (dedups by (name, table-kind))" | Cargo.toml:75-77 | REAL | baseline #11: only violation is lumen-cli→streaming; lumen-web clean |
| "the xtask TOML reader … rejects multi-line arrays" | Cargo.toml:118-120 | FALSE | F-003: toml_lite.rs:246-257 + test 564-575 and check_architecture.rs:625-670 both accept multi-line arrays |

## 7. Dead / duplicated / unreachable code
The whole crate is unreachable from shipped entry points (F-001). No duplicate manifest keys. The dual `codecs` target-table entries (78-79, 89-90) are an intentional target-split, deduped by the arch checker (baseline #11 clean).

## 8. Dependencies & flags
13 runtime deps (11 internal path + serde/serde_json), 1 target-gated internal ×2, 1 dev. No externals beyond the two admitted serde-family crates — matches boundaries.toml external_allowlist exactly. No unused deps found (all imports verified). No default-enabled features.

## 9. Open questions for the next agent
- Does `docs/feature-matrix.toml` claim a working web play column that this decoder-less, un-wired module contradicts? (parity web=0/252 suggests not; matrix rows belong to another agent.)
- worker.js / audio-worklet.js (the JS half) — the harness in ui/e2e/web is the only consumer that executes them; that lane's last recorded run is 2026-11-10 (browser-matrix docs), 11 days stale at audit date.
