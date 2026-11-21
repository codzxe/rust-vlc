# AUDIT: crates/codecs/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2026-11-21 |
| Lines / functions | 59 / 2 (two module declarations; everything else is crate-charter documentation) |
| File verdict | REAL (functioning crate root; charter docs contain two FALSE claims — see F-001/F-002) |
| Reachable from shipped entry point? | YES — the crate root of `codecs`, linked into the shipped `lumen` CLI (apps/cli/Cargo.toml:51) and the tauri shell chain (desktop-tauri Cargo.lock:406); its `video`/`audio` trees are dispatched by `apps/cli/src/pipeline_host.rs:236-378`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | Read full file; ls crates/codecs/src/audio (10 modules); sed quotes (:1-10, :47-59); baseline hygiene-perfile.txt (codecs files with unsafe: vaapi/session 41, videotoolbox/session 45, av1/decoder 23, opus/decoder 26, …); Cargo.toml default features (quoted in wave log). |

## 1. One-paragraph verdict
Two lines of code (both correct) under 57 lines of crate charter whose two headline safety/scoping claims are false against the crate they introduce: "zero external dependencies" (the default feature set now pulls eleven external decode engines) and "No `unsafe`" (the tree carries hundreds of unsafe blocks in its hw/av1/SIMD/FFI modules, concentrated exactly where the baseline hygiene audit says). Both claims were true in the 2.B4/2.B5 era the docs describe and were never updated as engines were admitted; Cargo.toml's own comments tell the true story, so this is doc drift at the crate's front door — the first thing a reader trusts.

## 2. What actually works (verified)
— `pub mod audio; pub mod video;` — both trees exist and compile (baseline build #1 clean; test log compiles codecs at :31).
— The crate-wide behavioral convention the docs state IS upheld by the code this wave audited: mjpeg concealment (`Recovered { ConcealedFrame }`, contract suite baseline :917), av1 damage folding (differential suite :891-897), typed refusals everywhere (mjpeg/av1/hw lanes).
— Baseline fuzz suites for the crate's codecs all pass (`fuzz_codec_mjpeg`-family survival tests ok), supporting the "no panics on any input" half of the conventions claim for the in-house decoders.

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/lib.rs:8
Code:
```rust
//! Layer L3 (doc 01 §2 registry row: `codec-api`, `plugin-api`,
//! `media-types` — this card's decoder surface needs the first and last;
//! `plugin-api` joins with the registry/`register()` surface of doc 01
//! §2.5(b), a later card). Std-only by C5: **zero external dependencies**.
```
What is wrong: "Std-only by C5: zero external dependencies" is false for the crate as shipped. `crates/codecs/Cargo.toml:47-59` puts eleven external decode engines in the DEFAULT feature set (claxon, symphonia-bundle-mp3, symphonia-core, symphonia-codec-{vorbis,aac,alac}, rav1d, ruopus, rusty_vp9, filmcraft-dnx, rust_h264 — deps at :86-174), and the shipped CLI builds with defaults (release.yml:193). Only a `--no-default-features` subset build is std-only (and even that keeps no external deps only because every engine is optional — which the doc does not say).
Why it matters: the crate-root charter is the claim a security/license reviewer reads first ("zero external dependencies" implies zero supply-chain surface — false: rav1d/ruopus/rust_h264 et al. are exactly the supply-chain surface the dependency records exist for). Cargo.toml's feature comments are honest; the root doc was never updated after the admissions.
What correct looks like: "Std-only at 2.B4; today the default feature set carries the Lead-admitted external engines (see Cargo.toml [features] and docs/dependencies/records/); `--no-default-features` is the std-only lane."
Confidence: HIGH — Verification: read of both files (Cargo.toml quoted in this wave's log).
### F-002 [S3]
Where: crates/codecs/src/lib.rs:49-52
Code:
```rust
//! - No `unsafe`, no panics on any input: every decode path is
//!   bounds-checked and overflow-checked (the fuzz targets
//!   `fuzz_codec_mjpeg` / `fuzz_codec_pcm`, doc 04 §8.5, hammer
//!   exactly this guarantee).
```
What is wrong: the "No `unsafe`" half is false for the crate as shipped. The tree contains module-level `#![allow(unsafe_code)]` in eight files (vaapi/{session,dl,library,ffi}.rs, videotoolbox/{session,ffi}.rs, av1/decoder.rs:93, mjpeg/idct_simd.rs), overriding the workspace `unsafe_code = "deny"`; baseline hygiene-perfile records 41 unsafe-mentions in vaapi/session.rs, 45 in videotoolbox/session.rs, 23 in av1/decoder.rs, 26 in audio/opus/decoder.rs, and more. The "no panics on any input" half is supported for the in-house fuzzed decoders (fuzz suites pass) but the sentence as written claims both for every decode path.
Why it matters: a false safety charter at the crate root — anyone scoping a security audit by reading lib.rs would exempt the crate's unsafe surface, which is precisely where this wave found the sharpest defects (session.rs#part3 F-002 shift overflow; ffi layout questions).
What correct looks like: "No unsafe in the in-house software decoders; the sanctioned exceptions (ADR-001 E2: hw lanes, rav1d seam, MJPEG SIMD kernel) carry per-site SAFETY invariants — see the module docs of each."
Confidence: HIGH — Verification: read + baseline hygiene-perfile.txt + this wave's own allow-site inventory (log, cross-cutting facts).
### F-003 [S4]
Where: crates/codecs/src/lib.rs:35-46
Code:
```rust
//! # Module map
//!
//! - [`video`] — video decoders; [`video::mjpeg`] is the Motion-JPEG /
//!   baseline-JPEG decoder (card 2.B4, doc 04 §4 row "Motion-JPEG").
//! - [`audio`] — audio decoders; [`audio::pcm`] is the unpacked-PCM
```
What is wrong: the module map lists three decoders (mjpeg, pcm, flac); the crate ships ten audio modules (aac, adpcm, alac, flac, mp2, mp3, opus, pcm, vorbis — ls verified) and eight video families (mjpeg, h264, hevc, av1, vp9, dnx, mpeg12, + color/idct plumbing). Same staleness class as F-001.
Why it matters: the map is the navigation entry point; it describes a crate two admission waves out of date.
What correct looks like: regenerate the map when a module lands, or replace it with a pointer to `video/mod.rs` / `audio/mod.rs` (which carry their own current maps).
Confidence: HIGH — Verification: read + ls.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pub mod audio;` | 58 | REAL | — | 10 submodules behind it |
| `pub mod video;` | 59 | REAL | — | 8 families behind it; per-lane reachability established in this wave's log (mjpeg/av1/h264-sw reachable; hevc/mpeg12/hw-lanes unreachable) |
| crate docs (charter) | 1-57 | PARTLY FALSE | S3/S4 | F-001 (deps claim), F-002 (unsafe claim), F-003 (module map); the damage-model and in-house-decision narratives are accurate |

## 5. Test honesty
No tests in this file. The crate's test surface is the largest in the repo (baseline: codecs suites across :240-:776 plus integration benches/differentials); nothing here to assert.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std-only by C5: zero external dependencies" | :8 | FALSE | Cargo.toml:47-59 default features (11 external engines) |
| "No `unsafe`, no panics on any input" | :49-52 | FALSE (unsafe half) | 8 module-level allow(unsafe_code) sites; baseline hygiene counts |
| "Corruption is either concealed-and-reported … never a silent partial frame, never a crash" | :53-57 | REAL | verified in this wave: mjpeg concealment + contract suite (:917), av1 damage folding (differential :891-897) |
| "in-house [MJPEG] decision … zero new deps to vet" (2.B4 narrative) | :10-25 | REAL (historically) | accurate for its era; superseded by F-001's story |
| module map (mjpeg/pcm/flac) | :35-46 | SHALLOW | F-003 — 3 of ~13 modules |

## 7. Dead / duplicated / unreachable code
None in this file. (Unreachable subtrees are catalogued per-file in this wave's other audits.)

## 8. Dependencies & flags
The root declares none; `[lints] workspace = true` (Cargo.toml:39-40) inherits the deny set that the eight unsafe modules must — and do — override at module level. The charter's job is to reconcile that tension in prose; today it pretends the tension doesn't exist (F-002).

## 9. Open questions for the next agent
- Wave-7 audio agents: `audio/opus/decoder.rs` shows 26 unsafe-mentions in hygiene-perfile — check whether that in-house glue (or ruopus's seam) has its own allow(unsafe_code) and SAFETY discipline; the crate-root claim (F-002) is wrong there too.
- Whoever owns docs/11 (dependency security): the crate-root "zero external dependencies" phrasing should be swept repo-wide — grep for other copies of the claim.
