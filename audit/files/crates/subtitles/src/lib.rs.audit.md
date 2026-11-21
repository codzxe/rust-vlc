# AUDIT: crates/subtitles/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-21 |
| Lines / functions | 145 / 0 functions — 8 `pub mod` declarations + the crate-contract doc comments (the claims ARE the file) |
| File verdict | FALSE (as a system-contract narrative: the promised consumers/seams do not exist) / the declared modules themselves REAL-UNPROVEN-in-shipped (all UNREACHABLE) |
| Reachable from shipped entry point? | NO — the whole crate: `rg` over every Cargo.toml → `subtitles` is a dependency ONLY of `fuzz/Cargo.toml`; `rg -i "subtitle" apps/` → zero code identifiers. `lumen` CLI, desktop-tauri, web never link it. |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 2 |
| Commands run | `rg "subtitles" --glob **/Cargo.toml` (fuzz only); `rg -i "subtitle" apps/ crates/playback-engine/src crates/video-engine/src crates/renderer-wgpu/src` (labels/commands/queue-lane only — see F-001); `rg "ComposeTarget" crates/ apps/` (impl ONLY at compositing/target.rs:213 — the in-crate reference surface); `rg "SubtitleParser" crates/ apps/` (trait in plugin-api/src/subtitle.rs:26, mock impl LineParser in-plugin, ZERO subtitles-crate impls, no dep edge); `sed -n '354,366p' crates/demux/src/ts/mod.rs` (0x06→"ts-private-pes", 0x90→"pgssub" — string labels); `sed -n '74,75p' crates/renderer-wgpu/src/lib.rs` ("blending joins with the subtitles crate (Phase 5)" — a comment, not code); baseline INDEX #3/#4 (crate compiled + 267/267 tests via nofailfast log). `sed -n` quote verifies on all findings. |

## 1. One-paragraph verdict
The crate root is the subtitles wave's claim headquarters, and its central claims describe a system that does not exist: the "compositing contract" whose "renderer adapter (a later card) forwards its rows" has no adapter (renderer-wgpu contains exactly one Phase-5 comment and zero code references; the only `ComposeTarget` implementation in the entire tree is the crate's own reference surface), no app links the crate, and the four "carriage landed earlier" stories are demuxer track-name strings with no decoder on the other end. Inside the island the modules are real and heavily tested (267/267 baseline), but the root also overclaims the island's own discipline — "no panics on any input" is refuted by three dev-profile panic sites in vobsub/bitmap — and carries stale worktree-scaffold narrative into the shipped tree. Verdict for the narrative: FALSE; for the code: unreachable.

## 2. What actually works (verified)
— Module declarations match the tree exactly (ass, compositing, model, srt, font, vtt, dvb, vobsub, cc, pgs — all present, all compile: baseline build #1 CLEAN, test #3/#4 include the crate; `orchrun-test-workspace-nofailfast.log.txt:5269-5271` 267/267).
— Honesty note "The AC tests are pixel-exact by construction" (:40) — TRUE: `tests/compositing_ac.rs` 8/8 (baseline :5569-5581) pins exact seams/z-order/scaling/rounding.
— The registry/doc anchors (doc 04 §5, doc 01 §2.2) are consistent with docs found in the tree (not re-audited here — doc wave's scope).

## 3. Findings

### F-001 [S2]
Where: crates/subtitles/src/lib.rs:19-25 (with :1-17)
Code:
```
//! [`ComposeTarget`] is the one trait a renderer implements to receive
//! composited subtitle pixels: `extent()` + one `row_rgba()` call per
//! touched destination row. The blend math lives HERE, once — no
//! video-engine or renderer-wgpu edit is needed or allowed by this
//! card; the renderer adapter (a later card) forwards its rows.
```
What is wrong: the "later card" never came. `rg "ComposeTarget"` over the whole tree: the only implementation is `impl ComposeTarget for Rgba8Surface` inside the crate itself (compositing/target.rs:213). renderer-wgpu's entire relationship with subtitles is one doc comment — "blending joins with the subtitles crate (Phase 5)" (renderer-wgpu/src/lib.rs:74-75). No app links the crate (Cargo.toml census: fuzz only). The playback engine has a subtitle queue LANE and a `SelectSubtitle` session command that its own docs classify as "class S: it never reaches the table" (session.rs:147-152) — attribute bookkeeping, no pipeline leg.
Why it matters: this crate's stated purpose is to be the CONTRACT between format decoders and the renderer; both sides of the contract are absent from every shipped binary. Every `CompositedCue` produced by pgs/dvb/vobsub/cc/ass/srt has no destination — the wave's hostile question ("is any subtitle path reachable and test-proven in the shipped build?") is answered NO at the root.
What correct looks like: either a renderer-wgpu adapter (`impl ComposeTarget` over its frame rows) + pipeline wiring of one format, or the crate root stating "library island; integration card owed" the way sub-vobsub.md does for its own gaps.
Confidence: HIGH — Verification: read + rg censuses (header).

### F-002 [S2]
Where: crates/subtitles/src/lib.rs:70-72
Code:
```
//! Everything is pure data + pure functions: no I/O, no threads, no
//! unsafe (workspace lint: `deny`), no panics on any input — failure is
//! typed ([`model::ModelError`]/[`model::WireError`]), per MP §8.
```
What is wrong: "no panics on any input" is false. Three dev-profile (overflow-checks on — the default `cargo test`/`cargo run` profile; baseline ran `--release`) panic sites on parseable input: vobsub `0x20u8 + stream.index` with idx `index:` ≥ 224 (vobsub/mod.rs:159), vobsub `start_ms * 1_000_000` with ≥8-digit-hour idx timestamps (:267-268), bitmap `solid`'s unchecked `w·h·4` (:151). In release, the first two degrade silently (wrap → wrong-channel match / all-cues-dropped), which the "failure is typed" half of the sentence also disowns.
Why it matters: this is the crate's MP §8 discipline claim, repeated per module; it holds for the fuzzed hot paths and fails exactly where no fuzz input reaches (proven by the survival suites' alphabets — vobsub mutation chars cap index ≤ 9).
What correct looks like: fix the three sites (checked arithmetic / typed damage) or scope the claim ("no panics on any input the in-crate fuzz corpora can synthesize").
Confidence: HIGH — Verification: read (cross-file findings vobsub F-002/F-003, bitmap F-001, each with traced arithmetic).

### F-003 [S3]
Where: crates/subtitles/src/lib.rs:104-112 (pattern repeated :114-121, :123-133, :134-145)
Code:
```
//! [`compositing::CompositedCue`]s. Carriage landed earlier: TS
//! stream type `0x06` → private PES (`demux/src/ts`), the demuxer
//! hands the decoder the payload after the PES header.
pub mod dvb;
```
What is wrong: "the demuxer hands the decoder the payload" — there is no decoder to hand to. The demuxer's whole involvement is two track-name strings: `0x06 => "ts-private-pes"` (demux/src/ts/mod.rs:355) and `0x90 => "pgssub"` (:365); the PS demuxer labels `PrivateKind::Spu => "dvd_subtitle"` (ps/mod.rs:171). No subtitle decoder exists in the codecs registry, no pipeline leg consumes a Subtitle-kind track, and the CC "H.264 user-data seam" discards its bytes (my cc/mod.rs F-001: `SeiMessage::Unparsed{payload_type,size}` — sei.rs:148-156).
Why it matters: four modules' docs assert an integration ("carriage landed", "hands the decoder") that is a string label; a reader concludes captions/PGS/DVB/VobSub flow from container to decode, when nothing flows anywhere.
What correct looks like: "the demuxer LABELS these streams; payload consumption is unwired (integration card owed)".
Confidence: HIGH — Verification: read + rg (ts/mod.rs:354-366, ps/mod.rs:171, sei.rs:148-156, codecs registry per baseline engine-info).

### F-004 [S3]
Where: crates/subtitles/src/lib.rs:99-101 (with :50-54, ass/mod.rs:39-48)
Code:
```
//! ledger that never panics. The only fatal is a missing `WEBVTT`
//! header. Future 5.M1 unification: the `SubtitleParser` seam.
```
What is wrong: the unification seams enumerated here and in the [LEAD SEAM] note were never executed: (a) the ASS parser↔render union ("the Lead wires parser output into these types at merge", render.rs:9-12) — no adapter exists; `AssRenderCue` has zero non-test constructors (my render.rs#part2 F-001); (b) the `SubtitleParser` seam — the trait exists in plugin-api (plugin-api/src/subtitle.rs:26) with a mock `LineParser`, but the subtitles crate does not depend on plugin-api (boundaries.toml allowlist: media-types only) and no srt/vtt/ass parser implements it. Both "future" items are still future; the doc reads as a plan already executed ("unification" as a named seam).
Why it matters: the crate root normalizes un-executed integration debt as if it were structure; three separate seams (parser↔render, SubtitleParser, ComposeTarget) all point the same way — an archipelago, not a pipeline.
What correct looks like: a single honest "Integration debt" section listing the three unwired seams with their owning cards.
Confidence: HIGH — Verification: read + rg (plugin-api trait, boundaries.toml:259-264, render.rs census).

### F-005 [S4]
Where: crates/subtitles/src/lib.rs:33-37
Code:
```
//! * Blending composites straight-alpha RGBA8 over an OPAQUE video
//!   plane, in byte space, rounding to nearest. This is associative
//!   for cue-over-cue-over-opaque-video (z stacking) and is what
//!   subtitle overlays do in practice; linear-light blending is the
//!   GPU path's concern.
```
What is wrong: byte-space blend with round-to-nearest is NOT exactly associative — `a over round(b over v)` and `round((a over b) over v)` can differ by 1 ulp; associativity holds only for the exact-rational blend the byte rounding approximates. The compositor's sequential z-fold (compose.rs) makes the order well-defined regardless, so the claim is decorative — and unqualified.
Why it matters: a crate that advertises "exact integer arithmetic … pixel-exact by construction" (:38-40) should not round its math claims.
What correct looks like: "associative up to the ±1 rounding of each byte-space step".
Confidence: MEDIUM — Verification: read (rounding placement argument; no counterexample computed — the blend's exact rounding path is target.rs, A03's file, cross-ref to check).
### F-006 [S4]
Where: crates/subtitles/src/lib.rs:48-54
Code:
```
//! **Scaffold note (Lead-ratification required):** the parallel wave-5
//! cards 5.L1 (text-format parsers: SRT/WebVTT-family) and 5.L2 (cue
//! model) each create this crate in their own worktree; the Lead unifies
//! the scaffold at wave close. This branch (`wip/p5-l2`) lands card
```
What is wrong: worktree/branch/process narrative ("this branch (wip/p5-l2)") fossilized in the shipped crate root — plus a second, duplicated scaffold block in Cargo.toml (:1-30). The described "parallel worktrees" no longer exist; the note describes merge-time mechanics for a merge that already happened.
Why it matters: crate-root docs should describe the code, not the git topology of its birth; a reader trying to find `wip/p5-l2` or the "other worktree" finds nothing.
What correct looks like: delete or move to the worklog/docs history.
Confidence: HIGH — Verification: read.

## 4. Per-function table (module declarations — the file's only code)
| Declaration | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pub mod ass` | 74 | REAL / UNREACHABLE | S2/S3 (docs) | render half fixture-driven (render F-001); parser half real |
| `pub mod compositing` | 75 | REAL / UNREACHABLE | S2 | contract with no implementor (F-001) |
| `pub mod model` | 76 | REAL / UNREACHABLE | — | 5.L2 data layer (other agents' files) |
| `pub mod srt` | 77 | REAL / UNREACHABLE | — | audited by A02/A06/A07 |
| `pub mod font` | 87 | REAL / UNREACHABLE | — | seam + placeholder pipeline (A03/A04) |
| `pub mod vtt` | 102 | REAL / UNREACHABLE | S4 (doc) | "SubtitleParser seam" future-work marker (F-004) |
| `pub mod dvb` | 112 | REAL / UNREACHABLE | S3 (doc) | "hands the decoder" false (F-003) |
| `pub mod vobsub` | 121 | REAL / UNREACHABLE | S1/S2 (code) | carries the crate's worst code defects (my vobsub audit) |
| `pub mod cc` | 133 | REAL / UNREACHABLE | S2 (doc+code) | H.264 seam false (cc F-001) |
| `pub mod pgs` | 145 | REAL / UNREACHABLE | S3 (doc) | 0x90 label-only (F-003 pattern) |

## 5. Test honesty
No tests in this file. Crate-level: 267/267 baseline (cited in §2); per-module honesty covered in my four sibling audit files and the other agents' files.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the renderer adapter (a later card) forwards its rows" | lib.rs:24-25 | FALSE (never landed) | F-001 census |
| "no panics on any input — failure is typed" | lib.rs:70-72 | FALSE | F-002 + cross-refs |
| "The AC tests are pixel-exact by construction" | lib.rs:40 | REAL | compositing_ac 8/8, baseline :5569-5581 |
| "Carriage landed earlier … the demuxer hands the decoder the payload" (×4 modules) | lib.rs:110-111, 116-117, 128-130, 143-144 | FALSE (label-only) | F-003 |
| "Future 5.M1 unification: the SubtitleParser seam" | lib.rs:101 | PAPER-ONLY | F-004 (trait exists in plugin-api, unwired) |
| "Scaling is point-sampled nearest-neighbor … same honesty as renderer-wgpu's software compositor" | lib.rs:29-32 | REAL-UNPROVEN | compose.rs xmap (A05); parity with renderer-wgpu untestable — no shared test |
| matrix rows subtitle-{srt,vtt,pgs,dvb-subs,cc-608-708,vobsub} = implemented win/linux/macos | feature-matrix.toml:887-1046 | UNREACHABLE (code real, tests real, no shipped path) | this file's header census |

## 7. Dead / duplicated / unreachable code
The unreachable set is the crate itself (F-001). Duplicated: the scaffold note appears twice (lib.rs:48-54 and Cargo.toml:1-30, in different words). The `#[non_exhaustive]` damage enums and re-exports are consistent.

## 8. Dependencies & flags
`media-types` only (matches boundaries.toml:259-264 — the allowlist's missing plugin-api edge is exactly what makes F-004's "seam" impossible without a boundaries change; worth noting the boundary file and the doc disagree in spirit: the doc names a seam the boundaries forbid).

## 9. Open questions for the next agent
- Wave gate: the three unwired seams (ComposeTarget adapter, parser→render adapter, SubtitleParser) + the missing subtitle pipeline leg are one integration card's worth of work; the orchestrator should decide whether any matrix row may stay "implemented" while it is owed.
- A03 (target.rs): compute or cite a ±1 counterexample (or proof of absence) for F-005's associativity wording.
