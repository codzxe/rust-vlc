# AUDIT: crates/subtitles/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A10 |
| Date | 2026-11-21 |
| Lines / config keys | 53 / 8 (package 5, lints 1, deps 1, dev-deps 2) |
| File verdict | UNREACHABLE (manifest consistent; the crate it defines has zero dependents — 21,334 LOC / 55 files nothing ships) |
| Reachable from shipped entry point? | NO — rg over every workspace member manifest: no path-dep on `subtitles`; rg `use subtitles|subtitles::` over crates/ + apps/ *.rs: matches only inside crates/subtitles' own tests |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg -l "subtitles" over all member Cargo.tomls (no dependents); rg -ln "use subtitles|subtitles::" crates/ apps/ (self-only); rg --files crates/subtitles/src + wc (55 files / 21,334 lines); sed boundaries.toml:245-265 (edge set match); ls crates/subtitles/{src,tests} (10 format modules, 15 test files); rg media_types/testkit/media_clock usage in src vs tests |

## 1. One-paragraph verdict
The manifest is small, std-only as claimed, and every declared edge is real and boundary-conformant — but the crate it defines is an orphan at product scale: nothing in the workspace depends on it, so 21K lines of ASS/CC/DVB/PGS/SRT/VobSub/VTT parsing plus a compositing contract are unreachable from both shipped entry points. The header also carries merge residue: two scaffold-note blocks from the two wave-5 worktrees, the second of which asserts "No dev-dependencies either" three lines above an actual `[dev-dependencies]` table with two entries.

## 2. What actually works (verified)
- Dep usage split is exactly as declared: `media-types` used in src (rg: compositing/{bitmap,target,compose}.rs, vobsub/mod.rs, dvb/mod.rs, …); `testkit` + `media-clock` used only in tests/ (rg: compositing_ac.rs, timing_clock.rs, vtt_fuzz.rs, …) — the "dev edge cannot leak into the public API" claim holds structurally.
- Boundary conformance: boundaries.toml:259-263 `[crates.subtitles]` internal_deps=["media-types"], dev_deps=["testkit","media-clock"], external_allowlist=[] — matches the manifest key-for-key; baseline #11 check-architecture: no subtitles violation.
- Std-only claim: zero external dependencies in the manifest (only 3 path deps) — "Std-only by C5" holds.
- The crate compiles and its 15 test files ran: baseline #3 `cargo test --workspace` (1681 passed / 1 failed — the failure is apps/desktop transport, not subtitles).
- KNOWN-LIMITS.md:44 honestly says "PGS/DVB captions stay missing until compositing is proven" — no doc claims wired subtitle rendering.

## 3. Findings

### F-001 [S3]
Where: crates/subtitles/Cargo.toml:43-53 (the dependency tables) — evidence of orphanhood
Code:
```
[dependencies]
media-types = { path = "../media-types" }

[dev-dependencies]
```
What is wrong: The crate has zero consumers. Verified: (a) no member manifest contains a path dependency on `subtitles` (rg -l "subtitles" over all member Cargo.tomls → only crates/subtitles/Cargo.toml itself and a comment mention in crates/media-types/Cargo.toml:11); (b) `use subtitles`/`subtitles::` appears only in crates/subtitles/tests/*; (c) lumen-cli, playback-engine, video-engine, renderer-wgpu — none link it. 55 source files / 21,334 lines of format parsers + compositing are dead weight in every shipped binary.
Why it matters: The product's subtitle story (doc 04 §5, the "compositing contract" in this very manifest's description) exists only as an unlinked library. Every gate (build/test/clippy/arch) pays for it; no user ever receives it. Drift risk: parsers rot invisibly because no integration path exercises them against real demux output.
What correct looks like: Either playback-engine/video-engine gain the subtitle edge (and the feature matrix marks subtitles unwired until then), or the crate is marked experimental/deferred in its manifest header so its orphan status is stated, not implied.
Confidence: HIGH — Verification: read (both rg passes above) + baseline #3 (tests execute) + baseline #9 (parity does not credit it).

### F-002 [S4]
Where: crates/subtitles/Cargo.toml:27-30 vs 46-53
Code (the stale claim):
```
# Std-only by design (doc 11: zero external dependencies — the model is
# pure data + pure functions; any future dependency needs the doc-11
# audit). No dev-dependencies either: the roundtrip/fuzz suites are
# std-only and deterministic.
```
(The file, 12 lines later:)
```
[dev-dependencies]
# testkit is dev-only by contract (MP §3.7): the counting allocator for
```
What is wrong: Self-contradiction inside one file: "No dev-dependencies either" vs two dev-dependencies (testkit, media-clock). The second header block (lines 19-30) is the pre-merge 5.L2 worktree's scaffold note, left in place when the Lead unified the two scaffolds; the first block (lines 1-17, the 5.L1 note) correctly documents the media-clock dev edge. Two overlapping scaffold headers is itself merge residue.
Why it matters: A reader reconciling the manifest against its comments gets opposite answers about its dev edges; the stale block also misdescribes the test suites ("std-only and deterministic" — they use testkit's counting allocator and MediaClock).
What correct looks like: Delete the 5.L2 scaffold block (lines 19-30); keep the 5.L1 block that matches the actual tables.
Confidence: HIGH — Verification: read (both blocks + the tables in the same file).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| [package] name/version/edition/description/publish | 32-38 | REAL | — | description describes the compositing contract — accurate for the code, but nothing consumes it (F-001) |
| [lints] workspace = true | 40-41 | REAL | — | inherited; clippy clean per baseline #5 |
| [dependencies] media-types | 43-44 | REAL | — | used in src (rg-verified); matches boundaries internal_deps |
| [dev-dependencies] testkit | 46-49 | REAL | — | used in tests/ (rg-verified) |
| [dev-dependencies] media-clock | 50-53 | REAL | — | used in tests/ (timing suites); dev-only edge documented in block 1 and boundaries.toml:256-258 |

## 5. Test honesty
No tests in the manifest. The crate's own test files executed in baseline #3 (workspace test run). Whether those tests would fail on broken parsers is the src-auditor's call; from the manifest side nothing is faked.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Deps (5.L1 half): media-types only … missing allowed-edges are fine" | Cargo.toml:11-13 | REAL | [dependencies] has exactly media-types; boundaries allowlist matches |
| "media-clock is DEV-ONLY … a dev edge cannot leak into the public API" | Cargo.toml:14-17 | REAL | media-clock only under [dev-dependencies]; rg confirms no src import |
| "Std-only by design … zero external dependencies" | Cargo.toml:27-28 | REAL | no external deps in any table |
| "No dev-dependencies either: the roundtrip/fuzz suites are std-only and deterministic" | Cargo.toml:29-30 | FALSE | two dev-dependencies at 46-53 (F-002) |
| "this worktree (wip/p5-l2, card 5.L2) owns src/model/**" | Cargo.toml:24 | STALE | worktree-scaffold residue in a merged main-line manifest |

## 7. Dead / duplicated / unreachable code
The entire crate is unreachable from shipped entry points (F-001). The manifest's duplicated scaffold headers (1-17 vs 19-30) are the duplication in this file.

## 8. Dependencies & flags
1 internal dep, 2 dev deps, 0 externals. No features. All edges boundary-declared. Nothing to cut at manifest level; the crate-level orphan problem is F-001.

## 9. Open questions for the next agent
- The src auditor for crates/subtitles should check whether the 15 test files assert real parser output or tautologies — from the manifest side I only verified they run.
- Which matrix rows (if any) credit subtitle formats as implemented? Feature-audit cells say 147 partial / 129 blocked — if any "partial" cites this crate while nothing links it, that row overstates.
