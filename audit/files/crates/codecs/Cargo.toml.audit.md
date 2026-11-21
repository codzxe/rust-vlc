# AUDIT: crates/codecs/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A09 |
| Date | 2026-11-21 |
| Lines / functions | 186 / 24 manifest keys audited (package block, 11 features, 13 deps, 4 dev-deps) |
| File verdict | REAL (manifest whose comments carry 3 claim defects: S2 drift vs executed baseline, S3 stale record citation, S4 stale description) |
| Reachable from shipped entry point? | YES (call chain: `apps/cli/Cargo.toml:51` `codecs = { path = "../../crates/codecs" }` (default features ON) → `apps/cli/src/pipeline_host.rs:65` `use codecs::video::h264::H264SwDecoder;` → `:346` `"h264" \| "avc1" => self.seeded(track, config, H264SwDecoder::new)`) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n 'codec-h264-sw' crates apps tools` (gate sites: only h264/mod.rs:55,60) · `rg -l` per optional crate name in `crates/codecs/src` (all 11 engines referenced) · `sed -n` on Cargo.lock for claxon/filmcraft-dnx/rav1d/ruopus/rust_h264/rusty_vp9/symphonia-core (versions match pins) · `sed -n '305,330p' boundaries.toml` (deps mirrored) · python SPS parse of `audit/runs/baseline/samples/sample.mp4` + `sample-faststart.mp4` + `tests/fixtures/generated/valid/mp4_h264_aac.mp4` (profile proof, see F-001) · `rg -n 'h264\.sw\.unsupported'` (sw.rs:305,533 only) · `ls docs/dependencies/records/` |

## 1. One-paragraph verdict
The manifest is real and unusually honest about its dependency-admission chain: every optional engine maps to exactly one `codec-*` feature, `default` is the full set (11/11 optional deps covered), every engine is actually referenced in `src/`, every external pin matches Cargo.lock exactly, and boundaries.toml mirrors the dep list verbatim. The defects are in the narrative claims layered on top: the H.264 admission comment asserts the P0 `codec-video-h264-software` row is "unblocked" while the executed baseline proves the shipped binary refuses every baseline MP4 sample with `h264.sw.unsupported` — because (proven by my SPS parse) the baseline samples are High 4:4:4 Predictive, an engine boundary the manifest/matrix narrative never names; the filmcraft-dnx comment cites a record file (`filmcraft-dnx-0.1.0.md`) that does not exist (pin is 0.9.0, record on disk is `filmcraft-dnx-0.9.0.md`); and the package `description` enumerates 3 of the crate's 16 codec families.

## 2. What actually works (verified)
- Every external pin matches the lock exactly: `rg -n 'name = "(claxon|filmcraft-dnx|rav1d|ruopus|rust_h264|rusty_vp9)"' -A1 Cargo.lock` → claxon 0.4.3, filmcraft-dnx 0.9.0, rav1d 1.1.0, ruopus 0.1.2, rust_h264 0.6.1, rusty_vp9 0.1.3; `symphonia-core` single lock node 0.6.1 (line 1060).
- `default` covers all 11 optional deps (manifest lines 47–59 vs 60–81): aac→symphonia-codec-aac+core, alac, av1→rav1d, dnx, flac→claxon, h264-sw→rust_h264, mp2→codec-mp3, mp3→symphonia-bundle-mp3+core, opus→ruopus, vorbis, vp9→rusty_vp9. No orphan optional dep, no dead feature.
- All 11 external engines referenced in `src/` (rg -l per crate name: flac/decoder.rs, mp2+mp3/decoder.rs, aac/vorbis/alac/decoder.rs, av1/decoder.rs, opus/decoder.rs, vp9/decoder.rs, dnx/{decoder,error}.rs, h264/sw.rs).
- Boundaries claim verified: `boundaries.toml:315-320` `[crates.codecs] internal_deps = ["codec-api", "media-types"]`, `dev_deps = ["testkit", "input", "demux", "plugin-api"]`, `external_allowlist` = exactly the 11 engines → matches manifest lines 84–85, 182–185, 86–174.
- In-house-decoders-unconditional claim verified: `crates/codecs/src/video/h264/mod.rs:55,60` gate ONLY `sw`; parser/hw/decode unconditional.
- Dev-deps all used in tests: `rg -l 'demux::|input::|testkit|plugin_api' crates/codecs/tests` → dozens of files (e.g. h264_sw_differential.rs uses `plugin_api::SourceStream` at its `mem_stream`).
- Tree-shake consumer exists: `apps/web/Cargo.toml:90` `codecs = { path = "../../crates/codecs", optional = true, default-features = false }` — the comment's "wasm tree-shake builds are the first consumers" is a real consumer, not vaporware.

## 3. Findings

### F-001 [S2]
Where: crates/codecs/Cargo.toml:159-162
Code:
```
# The H.264 software-decode track (Lead admission 2026-11-15 on the
# user's explicit instruction — ADR-001's conformance-gated software
# lane, unblocking the P0 `codec-video-h264-software` matrix row):
# `rust_h264` 0.6.1 — the audited candidate of issue #2's admission
```
What is wrong: The comment asserts the software track "unblocks" the P0 row (matrix row `codec-video-h264-software`, docs/feature-matrix.toml:427, claims `windows/macos/linux = "implemented"`), but the executed baseline (audit/runs/baseline/INDEX.md §MP4 SMOKE, mp4-smoke-open-session.txt) shows the shipped binary refuses EVERY baseline MP4 variant: `event: error [decode.unsupported] unsupported in h264 (h264.sw.unsupported)`. I parsed the avcC/SPS of the baseline samples with python: `audit/runs/baseline/samples/sample.mp4` and `sample-faststart.mp4` both carry `profile_idc=244` (High 4:4:4 Predictive), `chroma_format_idc=3` (4:4:4), 8-bit — an engine boundary the admission narrative never mentions (sw.rs:56 documents 4:4:4 → typed `h264.sw.unsupported`; refusal sites sw.rs:305,533).
Why it matters: The owner's reported symptom (MP4 cannot be opened) persists in the shipped binary, while the manifest + matrix row read as "done". Anyone tracing from this manifest believes the H.264 lane is closed; the actual state is "closed for Baseline/Main/High 4:2:0 only; the project's own evidence-harness samples (4:4:4) still fail". The wave-0 INDEX compounds this with a false causal claim ("the software H.264 decoder directory `crates/codecs/src/video/h264/sw` does not exist") — the decoder is a file `sw.rs`, it exists, it is compiled in (default features, apps/cli/Cargo.toml:51), selected (pipeline_host.rs:346), and refuses 4:4:4 per its documented typed boundary.
What correct looks like: The admission comment (and the matrix row notes) must state the profile boundary explicitly — "4:4:4 refuses typed h264.sw.unsupported" — and the row's e2e evidence must not imply general MP4 playback; wave-0 must re-generate litmus samples in 4:2:0 (or record the 4:4:4 refusal as the expected result).
Confidence: HIGH — Verification: read + baseline cite (INDEX #9 smoke outputs) + direct SPS byte-parse of the committed samples (command in header).

### F-002 [S3]
Where: crates/codecs/Cargo.toml:146,158
Code:
```
# 6.M-dnx (Lead admission, docs/dependencies/records/filmcraft-dnx-0.1.0.md):
```
and
```
filmcraft-dnx = { version = "=0.9.0", optional = true, default-features = false }
```
What is wrong: The comment cites the purity/admission record as `filmcraft-dnx-0.1.0.md`, but no such file exists — `ls docs/dependencies/records/` shows `filmcraft-dnx-0.9.0.md` only, matching the actual `=0.9.0` pin. The record-chain citation (the project's own doc-11 §3.7 rule: "a bump re-triggers the record") is broken at the manifest level: a reader auditing the pin follows the comment to a nonexistent path.
Why it matters: This manifest's entire trust model is "every external dep has a validate-then-admit record at a citable path" (repeated at lines 12–15, 87–89, 95+). One wrong filename breaks traceability and suggests the comment was written for a different version than the one pinned — exactly the kind of drift an admission regime exists to prevent.
What correct looks like: `docs/dependencies/records/filmcraft-dnx-0.9.0.md` (or regenerate the comment from the pin).
Confidence: HIGH — Verification: read (`ls docs/dependencies/records/` + sed -n of both lines).

### F-003 [S4]
Where: crates/codecs/Cargo.toml:36
Code:
```
description = "Lumen builtin software codecs: baseline-JPEG/MJPEG video decoder, unpacked-PCM audio decoder and FLAC audio decoder (claxon engine) behind the codec-api contract (doc 01 §2.5, cards 2.B4/2.B5/5.A1)."
```
What is wrong: The description names 3 codec families (MJPEG, PCM, FLAC) while the crate ships 16 modules: video `{av1, dnx, h264, hevc, mjpeg, mpeg12, vp9}` (crates/codecs/src/video/mod.rs:23–42) and audio `{aac, adpcm, alac, flac, mp2, mp3, opus, pcm, vorbis}` (crates/codecs/src/audio/mod.rs:29–63).
Why it matters: Inventory tooling, SBOM generation, and human triage read `description`; it under-reports the attack/audit surface (11 external engines + 5 in-house families hidden).
What correct looks like: Enumerate the families or state "…and 13 further audio/video codec families (see [features])".
Confidence: HIGH — Verification: read + rg of `^pub mod` in both mod.rs files.

### F-004 [S4]
Where: crates/codecs/Cargo.toml:91-94
Code:
```
# Direct edge to the engine's core (same pinned version; the node is
# already in the graph transitively — the direct edge adds NO new
# packages, it only names the types our glue code touches).
```
What is wrong: "adds NO new packages" is a graph claim I cannot verify without `cargo tree` (forbidden: no cargo runs). Cargo.lock shows a single `symphonia-core` 0.6.1 node (line 1060), which is consistent but does not prove the transitivity claim (the lock is a union of all resolved feature combinations).
Why it matters: If false, the manifest quietly widened the dependency graph while asserting it did not — the exact claim the C5 admission regime is supposed to keep checkable.
What correct looks like: Verified statement, e.g. a committed `cargo tree -p codecs -e features` snapshot next to the claim.
Confidence: MEDIUM — Verification: UNVERIFIED (no cargo runs allowed; lock single-node check only).

## 4. Per-function table (manifest keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package] name/version/edition` | 32-35 | REAL | — | edition 2024, consistent with workspace |
| `[package] description` | 36 | drift | S4 | F-003: 3 of 16 families named |
| `[package] publish = false` | 37 | REAL | — | private crate, correct |
| `[lints] workspace = true` | 39-40 | REAL | — | matches baseline hygiene note "39/40 crates inherit" |
| `features.default` | 47-59 | REAL | — | full set, 11/11 optional deps covered (verified) |
| `features.codec-aac` | 60 | REAL | — | → symphonia-codec-aac + core; used in src/audio/aac |
| `features.codec-alac` | 61 | REAL | — | → symphonia-codec-alac + core |
| `features.codec-av1` | 62-66 | REAL | — | → rav1d; comment honestly records wasm32 non-support |
| `features.codec-dnx` | 67 | REAL | — | → filmcraft-dnx |
| `features.codec-flac` | 68 | REAL | — | → claxon =0.4.3 (lock matches) |
| `features.codec-h264-sw` | 69-73 | REAL | — | → rust_h264; only cfg gate in the crate; narrative drift F-001 |
| `features.codec-mp2` | 74-77 | REAL | — | alias of codec-mp3 (one engine, two decoders — verified in mp2/decoder.rs) |
| `features.codec-mp3` | 78 | REAL | — | → symphonia-bundle-mp3 + core |
| `features.codec-opus` | 79 | REAL | — | → ruopus =0.1.2, default-features=false + std |
| `features.codec-vorbis` | 80 | REAL | — | → symphonia-codec-vorbis + core |
| `features.codec-vp9` | 81 | REAL | — | → rusty_vp9 =0.1.3 |
| dep `codec-api` | 84 | REAL | — | non-optional, in boundaries allowlist |
| dep `media-types` | 85 | REAL | — | non-optional, in boundaries allowlist |
| dep `claxon` | 86 | REAL | — | =0.4.3 pin, lock match |
| deps `symphonia-*` (5) | 90-115 | REAL | — | all =0.6.1, lock match; see F-004 for the no-new-packages claim |
| dep `rav1d` | 116-125 | REAL | — | =1.1.0, default-features=false + bitdepth_8/16; asm-off claim only record-verifiable (UNVERIFIED) |
| dep `ruopus` | 126-137 | REAL | — | =0.1.2, default-features=false, features=["std"] |
| dep `rusty_vp9` | 138-145 | REAL | — | =0.1.3, no default features |
| dep `filmcraft-dnx` | 146-158 | REAL | — | =0.9.0 pin correct; comment cites wrong record filename (F-002) |
| dep `rust_h264` | 159-174 | REAL | — | =0.6.1 lock match; admission narrative drift F-001 |
| dev-deps `input/demux/plugin-api/testkit` | 182-185 | REAL | — | all used in tests/ (verified by rg); match boundaries dev_deps |

## 5. Test honesty
No tests in this file. The manifest's comments lean on `crates/codecs/tests/h264_sw_differential.rs` as binding admission evidence; read of `assert_lane` (lines 310–357) shows REAL assertions: pinned whole-dump MD5 per lane (runs unconditionally), frame-count/size/format asserts, and a live ffmpeg oracle re-check that skips loudly when ffmpeg is absent. Mutation check (reasoned, not run): if the rust_h264 feed or framing bridge broke, the MD5/frame-count asserts fail — the lane tests are not tautologies. Whether these tests passed in the baseline run is UNVERIFIABLE from my worktree: INDEX #3 cites `test-workspace.log`, which is NOT committed under audit/runs/baseline/ (only smoke/hygiene files exist there — see my log, evidence-gap note E-1).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every EXTERNAL decode engine is now optional behind its codec-* feature; default enables the full set" | manifest lines 17-30 | REAL | features table 47-81 + rg of all 11 engines in src |
| "the IN-HOUSE decoders (mjpeg, pcm, adpcm, h264, hevc, mpeg12) stay unconditional" | manifest lines 25-30 | REAL | h264/mod.rs:55,60 gates only `sw` |
| "boundaries.toml mirrors it exactly" | manifest lines 6-8 | REAL | boundaries.toml:315-320 matches all three dep lists |
| filmcraft-dnx record at `docs/dependencies/records/filmcraft-dnx-0.1.0.md` | manifest line 146 | FALSE | ls records/: only filmcraft-dnx-0.9.0.md exists (F-002) |
| rust_h264 admission "unblocking the P0 codec-video-h264-software matrix row" | manifest lines 159-162 + matrix row 419-428 | SHALLOW (drift) | baseline INDEX MP4 smoke: all variants fail h264.sw.unsupported; samples are 4:4:4 (my SPS parse) (F-001) |
| symphonia-core direct edge "adds NO new packages" | manifest lines 91-94 | REAL-UNPROVEN | lock single node 0.6.1; cargo tree forbidden (F-004) |
| package description enumerates the crate's decoders | manifest line 36 | FALSE (incomplete) | 16 modules exist (F-003) |

## 7. Dead / duplicated / unreachable code
None in the manifest: no unused feature, no orphan optional dep, no dep outside the boundaries allowlist. The one dead-looking thing — the `sw` evidence glob `crates/codecs/src/video/h264/sw/**` (matrix row 428) matching no file (real path is `h264/sw.rs`) — is a matrix-row defect (feature-audit violation row 23, INDEX #8), not a manifest key; cross-ref to the matrix-rows auditor.

## 8. Dependencies & flags
11 external engines, all pinned `=`, all with one record each on disk (verified by ls), all lock-consistent. Shipped CLI consumes `codecs` with default features (apps/cli/Cargo.toml:51), so all 11 engines compile into `lumen` — while engine-info (apps/cli/src/host.rs:1644-1659) hand-lists only mjpeg + pcm_s16le and decode-bench (apps/cli/src/bench.rs:300-338) hard-codes the mjpeg+pcm limbs, reporting other codecs as "no decoder in this build". The engines ARE reachable via `open` (pipeline_host.rs:253-346 wires aac/alac/av1/dnx/flac/h264/mp2/mp3/opus/vorbis/vp9), so the bench/engine-info under-reporting is an apps/cli honesty defect, not a codecs-manifest one. Cross-ref to the apps/cli and bench.rs auditors.

## 9. Open questions for the next agent
1. sw.rs auditor: confirm the 4:4:4 refusal path is the typed `h264.sw.unsupported` at sw.rs:305/533 and that no 4:2:0 High-profile input can hit it (my parse proves the baseline samples are 4:4:4; the differential lanes prove 4:2:0 High works in tests — the gap between the two is the owner's bug surface).
2. Wave-0 orchestrator: the INDEX's causal claim "the software H.264 decoder directory does not exist" is false (sw.rs exists and is compiled into the shipped binary); the litmus samples were generated 4:4:4 (ffmpeg RGB-source → libx264 auto yuv444p). Re-generate 4:2:0 litmus samples or re-state the litmus result.
3. apps/cli auditor: engine-info's static two-decoder list vs the 12 wired decoders in pipeline_host.rs — which is the lie?
