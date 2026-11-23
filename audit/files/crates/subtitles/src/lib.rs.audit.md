# AUDIT: crates/subtitles/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W09-A03 |
| Date | 2026-11-15 |
| Lines / functions | 145 / 0 functions — 10 `pub mod` declarations + crate docs |
| File verdict | UNREACHABLE (the crate root exports a full subtitle engine that NO shipped entry point compiles) |
| Reachable from shipped entry point? | NO — proof below; sole external consumer is `fuzz/Cargo.toml:50` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Grep `subtitles` over all Cargo.tomls (1 hit: fuzz/); Grep `use subtitles::|subtitles::` tree-wide (zero non-test/non-fuzz consumers); Grep `impl.*ComposeTarget` (sole hit in-crate, corroborated by audit/files/.../target.rs.audit.md); rg `unsafe` in crates/subtitles/src (doc comments only); rg `0x90|pgssub|0x06` in demux/src/ts (hits at ts/mod.rs:355,365,382) |

## 1. One-paragraph verdict
Hostile-focus answer: the crate exports ten public modules — `ass`, `compositing`, `model`,
`srt`, `font`, `vtt`, `dvb`, `vobsub`, `cc`, `pgs` — a complete subtitle parsing/compositing
engine, and NOTHING shipped consumes it. The only workspace dependency on `subtitles` is the
fuzz harness; `use subtitles::` appears nowhere outside the crate's own tests, fuzz, and
audit probes. Every architecture promise that would connect this crate to a user — "the
renderer adapter (a later card) forwards its rows", "Future 5.M1 unification: the
`SubtitleParser` seam" — refers to code that does not exist in the tree (grep-proven; the
sole `impl ComposeTarget` is the in-crate test surface, see
audit/files/crates/subtitles/src/compositing/target.rs.audit.md). The docs are carefully
honest about scope ("the CONTRACT, not the rendering"), which keeps this short of fraud —
but a 10-module public API with zero shipped callers is a library-shaped ghost.

## 2. What actually works (verified)
— Nothing to execute in this file (declarations only). Verified claims: no `unsafe` blocks
in the crate (rg: only doc-comment mentions; consistent with workspace `unsafe_code=deny`
and the clean baseline build); the TS-carriage claims check out at the recognition level
(demux/src/ts/mod.rs:355 `0x06 => "ts-private-pes"`, :365 `0x90 => "pgssub"`,
:382 `0x90 => MediaKind::Subtitle`).
— The exported modules themselves contain real, tested code (this wave: ass::render 14/14,
cc acceptance 11/11, srt::encoding 9/9, vtt_fuzz 3/3 — all executed from prebuilt binaries).

## 3. Findings
F-001 [S3]
Where: crates/subtitles/src/lib.rs:21-25 and :100-101
Code:
```
//! [`ComposeTarget`] is the one trait a renderer implements to receive
//! composited subtitle pixels: `extent()` + one `row_rgba()` call per
//! touched destination row. The blend math lives HERE, once — no
//! video-engine or renderer-wgpu edit is needed or allowed by this
//! card; the renderer adapter (a later card) forwards its rows.
```
What is wrong: the "later card" never landed. Tree-wide grep: no `impl ComposeTarget`
outside the crate's own test surface; renderer-wgpu never reads its `subtitle_layer`
field; video-engine has zero subtitle code. Same for "Future 5.M1 unification: the
`SubtitleParser` seam" (:100-101) — no such seam exists. The crate root's entire reason
for being (a contract someone plugs into) is unfulfilled: ten public modules, zero
shipped importers. Why it matters: the crate compiles in every `cargo test --workspace`
run and inflates the "implemented" surface while a user of the shipped binaries cannot
display one subtitle pixel. What correct looks like: either wire one host (CLI overlay
flag -> ComposeTarget adapter) or mark the crate `experimental` in the registry/docs so
the matrix stops counting it. Confidence: HIGH — Verification: read + ran greps (listed
in header).
F-002 [S4]
Where: crates/subtitles/src/lib.rs:1-46 vs :48-72
Code:
```
//! Lumen subtitles: the compositing contract (card 5.L1, doc 04 §5
...
//! THE LUMEN SUBTITLES CRATE (cards 5.L1 + 5.L2, doc 04 §5).
```
What is wrong: two stacked crate-level header blocks from the two pre-merge worktrees
(5.L1's "What this crate owns in v0" and 5.L2's scaffold note) both survive, each
describing the crate's scope differently ("owns the CONTRACT, not the rendering" vs
"Card 5.L2 scope — the shared cue model"). Merge residue; a reader cannot tell which
charter is current. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/subtitles/src/lib.rs:70-72
Code:
```
//! Everything is pure data + pure functions: no I/O, no threads, no
//! unsafe (workspace lint: `deny`), no panics on any input — failure is
//! typed ([`model::ModelError`]/[`model::WireError`]), per MP §8.
```
What is wrong: the crate-wide "no panics on any input" contract is violated inside the
crate's own render path (see audit/files/crates/subtitles/src/ass/render.rs.audit.md
F-001: unchecked `u64::from(width) * u64::from(height) * 4` at ass/render.rs:393/520 —
overflow-panics in debug with ~1.7MB valid input). The other three clauses hold
(verified: media-types-only deps, no threads, no unsafe). Confidence: HIGH —
Verification: read + python3 arithmetic repro in the render.rs audit.

## 4. Per-function table
No functions in this file (declarations + docs only). Module inventory instead:
| Module | Line | Decl | Notes |
|---|---|---|---|
| ass | 74 | pub mod | parser+tags (5.M2a) + layout+render (5.M2b); audited in this wave |
| compositing | 75 | pub mod | contract + reference blend; test-only consumer |
| model | 76 | pub mod | cue model (5.L2) |
| srt | 77 | pub mod | parser + encoding (audited) |
| font | 87 | pub mod | seam + placeholder pipeline (no font stack; [PROPOSAL] record) |
| vtt | 102 | pub mod | parser (5.M3) |
| dvb | 112 | pub mod | ETSI EN 300 743 (5.N2) |
| vobsub | 121 | pub mod | IDX/SUB (5.N2) |
| cc | 133 | pub mod | CEA-608/708 (5.N3) |
| pgs | 145 | pub mod | HDMV PGS (5.N1) |

## 5. Test honesty
No tests in this file.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Text shaping, fonts and the GPU path stay in renderer-wgpu" | :10-12 | PAPER-ONLY (deferral) | no font stack exists anywhere; font record is "[PROPOSAL — Lead approval pending]" (font/mod.rs docs); renderer-wgpu has no glyph code |
| "the renderer adapter (a later card) forwards its rows" | :24-25 | FALSE today (promissory) | grep: zero adapters; sole ComposeTarget impl is the in-crate test surface |
| "Carriage landed earlier: TS stream type 0x06 → private PES ... 0x90 → pgssub (demux/src/ts)" | :109-111, :143-144 | REAL (recognition only) | demux/src/ts/mod.rs:355,365,382; but demux does not depend on subtitles — no decoder receives these payloads |
| "no I/O, no threads, no unsafe, no panics on any input" | :70-72 | FALSE (panics clause) | render.rs F-001 (S2); other clauses verified |
| "Everything is pure data + pure functions" | :70 | REAL for model/compositing; FALSE for render paths that allocate bitmaps | read (ass/render.rs allocates up to 64MiB vecs) |

## 7. Dead / duplicated / unreachable code
The entire public surface is unreachable from shipped hosts (F-001). Duplicated header
blocks (F-002). No dead `use`/items in-file.

## 8. Dependencies & flags
`media-types` (path) only; dev: `testkit`, `media-clock`. No features, no cfg gates —
everything exports unconditionally, which is exactly why the island is fully compiled in
workspace test runs.

## 9. Open questions for the next agent
- Is there an unlanded card/issue that wires `ComposeTarget` into renderer-wgpu (the
  "later card")? If the plan was dropped, F-001 escalates: the crate has no future host.
- Should the feature-matrix subtitle rows (srt/vtt/ass/dvb/pgs/vobsub/cc) carry an
  "engine not wired to any host" caveat? Current matrix evidence cites this crate's files;
  without the caveat the rows overstate user-facing support (matrix-chunk owners: verify).
