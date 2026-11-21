# AUDIT: crates/demux/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A09 |
| Date | 2026-11-21 |
| Lines / functions | 39 / 8 manifest keys (package, lints, 2 deps, 2 dev-deps) + 2 header claim blocks |
| File verdict | REAL (dep table exact and boundaries-mirrored; hygiene claim verified true; description stale: names 1 of 13 format modules) |
| Reachable from shipped entry point? | YES, EXECUTED (call chain: `apps/cli/Cargo.toml:53` `demux = { path = "../../crates/demux" }` → CLI open path; baseline proof: mp4-smoke-open-session.txt `event: tracks 2 ready` + `TimingDurationKnown` (demux ran on MP4), avi-firstvertical-open-session.txt + mp4-smoke-bench-avi-mjpeg-pcm.txt `v_frames=60 a_frames=60` (demux ran on AVI, end-to-end)) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `ls crates/demux/src` (13 format modules) · `rg -n 'pub mod' crates/demux/src/lib.rs` (43-55: adts aiff asf avi es ivf mkv mp4 ogg ps ts wav) · `sed -n '197,203p' boundaries.toml` ([crates.demux] exact mirror) · precise hygiene rg: `rg -n 'unsafe\|\.unwrap\(\)\|\.expect\(\|todo!\|panic!\|unimplemented!' crates/demux/src` minus comment lines → ZERO code hits (all 67 "unwrap" substrings in mp4/mod.rs are `unwrap_or*` total functions: lines 494,920,938,942,943…) · `rg -ln 'input::\|testkit' crates/demux/tests` (both dev-deps used) · `rg -n 'demux' apps/cli/Cargo.toml` (line 53) |

## 1. One-paragraph verdict
The cleanest manifest audited so far: two internal deps mirrored exactly by boundaries.toml:197-202, both dev-deps genuinely used, zero externals — and the crate's bold hygiene claim ("No unsafe, no unwrap/expect/panic/todo") survives a precise re-check: every one of the 67 "unwrap" substrings the baseline hygiene script counted in mp4/mod.rs is an `unwrap_or`/`unwrap_or_default` total function, not a panicking `unwrap()`; there is not one real `unsafe`, `.unwrap()`, `.expect(`, `todo!` or `panic!` in non-test demux source. The single defect is inventory staleness: the description and the header's "v0 = src/avi/" framing present an AVI-only crate while lib.rs declares 13 format modules — including the MP4 demuxer the owner's litmus test runs through, and 7 formats (adts, aiff, asf, es, ivf, mp4, ps) that the header's own "later formats (wav/ogg/mkv/ts)" list doesn't even anticipate.

## 2. What actually works (verified)
- Dep table ↔ boundaries.toml exact: `boundaries.toml:197-202` `[crates.demux] layer = 2, internal_deps = ["media-types", "plugin-api"], dev_deps = ["testkit", "input"], external_allowlist = []` — identical to manifest lines 28-39.
- The "no compile-time edge to input" architecture claim is real: demux pulls bytes through `plugin_api::SourceStream` (manifest lines 3-6); `input` appears only as a dev-dependency and is used in tests (`rg -ln 'input::' crates/demux/tests` → common/mod.rs, es_corpus.rs, mkv_editions_tags.rs, corpus.rs).
- Hygiene claim VERIFIED: `rg -n 'unsafe|\.unwrap\(\)|\.expect\(|todo!|panic!|unimplemented!' crates/demux/src --glob '*.rs'` filtered for non-comment lines → empty. The baseline hygiene-perfile.txt "67 unwrap() in crates/demux/src/mp4/mod.rs" is a methodology artifact: the script's `git grep 'unwrap()'` pattern (BRE) matches the bare substring "unwrap", so it counts safe `unwrap_or` calls — other agents citing that file must not read it as 67 panicking unwraps.
- `testkit` dev-dep used: rg → mkv_extras_wiring.rs, mkv_fixtures.rs, adts_corpus.rs, …
- Crate reachable AND executed: baseline smoke proves the demux layer ran on MP4 (2 tracks, duration, seekable) and AVI (60+60 frames demuxed and decoded).

## 3. Findings

### F-001 [S4]
Where: crates/demux/Cargo.toml:8-11,22
Code:
```
# v0 (D-018 variant A, the first vertical): `src/avi/` — the RIFF/AVI 1.0
# demuxer for the MJPEG+PCM slice (doc 03 §7 candidate 2). Module layout
# leaves room for later formats (wav/ogg/mkv/ts per doc 04 §8.5; the
```
and
```
description = "Lumen container demultiplexers: AVI 1.0 RIFF walk, hdrl/strl track discovery, idx1 seek table, MJPEG+PCM packet extraction (card 2.B2, doc 03 §4.3/§7)."
```
What is wrong: The crate ships 13 format modules — `lib.rs:43-55` declares `adts, aiff, asf, avi, es, ivf, mkv, mp4, ogg, ps, ts, wav` (+ error) — but the description enumerates only the AVI demuxer, and the header frames the crate as "v0 = src/avi/" with "later formats (wav/ogg/mkv/ts)" — a list that misses 7 of the 12 non-AVI formats that actually landed (adts, aiff, asf, es, ivf, mp4, ps).
Why it matters: demux is the crate at the center of the owner's bug (the MP4 open path); a manifest that describes it as an AVI-only walker misdirects every audit triage, SBOM review, and capability inventory toward the wrong module set. The MP4 demuxer's existence is exactly what the feature-matrix container-mp4 row's older note ("No MP4 demuxer code exists") got wrong — this manifest perpetuates the same stale frame one level down.
What correct looks like: Description naming the format families ("AVI, MP4/ISOBMFF, MKV, MPEG-TS/PS, Ogg, WAV/AIFF, ADTS, ASF, IVF, ES…"); header rewritten as history ("v0 was AVI-only; now 13 formats").
Confidence: HIGH — Verification: read (lib.rs pub mods + ls src).

## 4. Per-function table (manifest keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header: registry/trait-seam claim | 1-6 | REAL | — | plugin-api SourceStream seam verified; boundaries mirror |
| header: v0-avi framing | 8-11 | drift | S4 | 13 formats exist; 7 not even in the anticipated list (F-001) |
| header: hygiene claim | 13-16 | REAL | — | precise rg: zero unsafe/unwrap()/expect/todo/panic in non-test code |
| `[package]` block | 18-23 | REAL | — | description stale (F-001) |
| `[lints] workspace = true` | 25-26 | REAL | — | inherits; clippy baseline #5/#6 CLEAN with -D warnings |
| dep `media-types` | 29 | REAL | — | L0 vocabulary; boundaries-mirrored |
| dep `plugin-api` | 30 | REAL | — | SourceStream port; boundaries-mirrored |
| dev-dep `testkit` | 32-35 | REAL | — | used across the test suite (rg verified) |
| dev-dep `input` | 36-39 | REAL | — | used (composition tests through real FileAccess) |

## 5. Test honesty
No tests in this file. The manifest's dev-deps exist precisely to make the crate's tests compositional (real `input::FileAccess`, not in-memory stand-ins — verified by rg of test imports). The crate's own test files are another agent's scope; nothing in this manifest makes claims about specific test outcomes.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "C5 discipline: std-only, zero external dependencies" | manifest lines 13-13 | REAL | no externals; boundaries external_allowlist = [] |
| "No unsafe, no unwrap/expect/panic/todo (workspace lints)" | manifest lines 13-14 | REAL | precise rg → zero code hits; baseline hygiene "67" is substring overcount of `unwrap_or` (see §2) |
| "doc 13's 'Consumes: input' satisfied at the trait seam … no compile-time edge" | manifest lines 3-6 | REAL | input is dev-only; plugin_api::SourceStream is the wire |
| description = AVI-only crate | manifest line 22 | FALSE (stale) | 13 modules in lib.rs:43-55 (F-001) |
| "Module layout leaves room for later formats (wav/ogg/mkv/ts …)" | manifest lines 10-11 | FALSE (stale) | all four landed PLUS 7 unlisted formats |

## 7. Dead / duplicated / unreachable code
None in the manifest. Crate-level: fully reachable and executed (baseline smoke). Cross-ref: the MP4 box walker duplicated TEST-SIDE in `crates/codecs/tests/common/mod.rs` (per feature-matrix container-mp4 notes) is a duplication smell owned by the codecs tests auditor.

## 8. Dependencies & flags
Two internal deps, two internal dev-deps, zero externals, zero features — the minimal graph the header promises. `mp4/test_support.rs` inside src (not tests/) is noted for the mp4-module auditor: test-support code compiled into the production target needs a cfg gate check (UNVERIFIED here — out of my file's scope).

## 9. Open questions for the next agent
1. mp4/mod.rs auditor (2,225 lines): the manifest's ResourceLimit claim ("payload sizes refused above a typed ceiling") is the crate's central safety promise — verify it on the box-size paths.
2. Whoever consumes hygiene-perfile.txt: the "unwrap()" section counts the substring "unwrap" (BRE `\(\)` = empty group), so `unwrap_or` inflates counts repo-wide; re-derive panicking-unwrap counts with a precise pattern before citing.
3. Is `crates/demux/src/mp4/test_support.rs` cfg-gated out of release builds? (If not, test code ships in the lib.)
