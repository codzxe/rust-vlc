# AUDIT: crates/subtitles/src/ass/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-23 |
| Lines / functions | 52 / 0 functions (module hub: 5 module decls + 2 re-export lines) |
| File verdict | REAL (library) / UNREACHABLE (product) — accurate header docs, one stale self-contradicting merge note |
| Reachable from shipped entry point? | NO — `ass::parse` re-exported at crate root; no in-crate production caller outside ass/ (fuzz harness + tests only); crate is a dep of no app (only fuzz/Cargo.toml:50) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | ls src/ass/ (6 files, all declared modules exist, exit 0); rg in-crate callers of ass::parse (exit 0); sed quote verification |

## 1. One-paragraph verdict
A 52-line module hub that does its job: declares parser/tags/layout/render/fuzz (all five files exist, no undeclared strays), re-exports the public ASS vocabulary (`parse`, `AssDocument`, `AssStats`, `AssStyle`, `AssDamage`, `StyleFlavour`, `PositionOverride`), and carries a module header whose claims I verified against the audited code: infallible parse with typed damage, G-2 tag minimum applied natively, beyond-minimum tags dropped-and-counted, `\N`/`\n`/`\h` folding — all confirmed by the 10/10 tags tests and the parser read. The defect is the fossilized "[LEAD SEAM]" block: it asserts this file "declares ONLY the render-path modules" while `pub mod parser; pub mod tags;` sit twenty lines above it, and describes the parser half as living in another worktree when parser.rs is in this tree — the merge it describes as future already happened. Plus the standing domain blocker: none of it ships.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-70f89de07410626f ass::tags` -> 10 passed (exit 0; the engine this hub wires), ls of src/ass/ (every declared module present), and rg confirming no orphan files. The header's damage-model description matches parser.rs behavior read during the tags.rs audit (typed AssDamage, per-cue skip on bad times).

## 3. Findings
F-001 [S2]
Where: this file + apps/cli/Cargo.toml (absence); fuzz/Cargo.toml:50
Code: `subtitles = { path = "../crates/subtitles" }` (fuzz/Cargo.toml:50; no such row in apps/cli/Cargo.toml)
What is wrong: the crate's ASS entry (`ass::parse`) is a dependency of nothing shipped (tags.rs F-001). Confidence: HIGH — Verification: ran (rg).
F-002 [S4]
Where: crates/subtitles/src/ass/mod.rs:39-48 vs 22-26
Code:
```
// # [LEAD SEAM]
//
// This `mod.rs` is the 5.M2b half: it declares ONLY the render-path
// modules, and the 5.M2a worktree owns `parser.rs` plus its own
// `mod.rs` declaring `pub mod parser;`. At merge the Lead unions the
// declarations (append `pub mod parser;` + any re-exports) — the
```
vs
```
pub mod parser;
pub mod tags;
```
What is wrong: the merge-note contradicts the file it sits in: parser+tags ARE declared above (lines 22-23), parser.rs IS in this tree (read during this audit), yet the comment claims the opposite ("declares ONLY the render-path modules", parser "NOT in this worktree"). Post-merge fossil that misleads any reader reconstructing the architecture. What correct looks like: delete or rewrite the block as history. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/subtitles/src/ass/mod.rs:27-37
Code:
```
// ASS (Advanced SubStation Alpha) support — split across two
// file-disjoint cards (wave 6):
//
// * **5.M2a (parallel worktree):** the parser (`ass/parser.rs`) —
//   ASS event/style text → the cue model. NOT in this worktree.
```
What is wrong: same fossil as F-002, front half — "NOT in this worktree" is false in the merged tree. Kept separate because it is the prose a reader hits first. Confidence: HIGH — Verification: read.

## 4. Per-function table (file < 100 LOC — no functions; inventory instead)
| Item | Lines | Verdict | Notes |
|---|---|---|---|
| module docs | 1-20 | REAL | claims verified against parser/tags behavior + tests |
| `pub mod parser` / `pub mod tags` | 22-23 | REAL | files exist, tested |
| re-exports | 25-26 | REAL | public vocabulary; PositionOverride used by parser.rs:711 |
| stale merge commentary | 27-48 | STALE | F-002/F-003 |
| `pub mod layout` / `pub mod render` / `pub mod fuzz` | 49-52 | REAL | files exist; render audited via pipeline.rs cross-refs; fuzz target fuzz_sub_ass.rs EXISTS in fuzz/ (unlike pgs) |

## 5. Test honesty
No tests in this file. The modules it wires carry the tests audited elsewhere (tags 10/10 executed).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The parser is infallible: parse always returns a document" | mod.rs:16 | REAL | parser.rs read; fuzz harness swallows all outcomes |
| "G-2 override-tag minimum natively (issue #17)" | mod.rs:6-8 | REAL | tags.rs audit, 10/10 tests |
| "karaoke, rotation, animation, fades ... stripped and counted per tag name, never a parse failure" | mod.rs:9-11 | REAL | unsupported_tags_stay_dropped_and_counted passes |
| "This mod.rs ... declares ONLY the render-path modules" | mod.rs:41-42 | FALSE (stale) | F-002 |
| "5.M2a ... NOT in this worktree" | mod.rs:30-31 | FALSE (stale) | F-003 |

## 7. Dead / duplicated / unreachable code
No dead items. The whole module unreachable from shipped builds (F-001).

## 8. Dependencies & flags
std-only crate; no features; no unsafe. Module set matches the directory exactly.

## 9. Open questions for the next agent
- ass/layout.rs and ass/render.rs belong to another agent — the render path's inline block rasterization (pipeline.rs F-002) is where the font-seam story breaks; coordinate verdicts.
- If the LEAD SEAM comment is deleted, check docs referencing "file-disjoint cards" for matching staleness.
