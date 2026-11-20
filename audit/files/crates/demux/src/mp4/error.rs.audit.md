# Audit — crates/demux/src/mp4/error.rs

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W05-A02 |
| Date | 2026-11-16 |
| Lines | 170 (verified `wc -l`) |
| Functions | 12 constructors (`not_mp4`, `truncated`, `bad_box`, `box_overruns_parent`, `depth_exceeded`, `oversized_box`, `no_moov`, `no_tracks`, `no_sample_table`, `not_started`, `seek_out_of_range`, `scan_budget_exhausted`) + private `mint` |
| File verdict | **REAL** — a typed error taxonomy riding `media_types::ErrorKind` + structured `CorruptDetail`/`LimitKind` payloads with stable `demux.mp4.*` codes; one constructor is dead-on-arrival (`depth_exceeded`, zero call sites repo-wide) |
| Reachable | YES — `pub mod error;` (mod.rs:99); 11 of 12 constructors have production call sites in mod.rs (grep-verified; `depth_exceeded` has none — see E-001) |
| Findings | S0:0 S1:0 **S2:1** **S3:1** **S4:1** (3 total) |
| Commands run | `wc -l`; full `Read`; `rg -n "error::(bad_box|box_overruns_parent|depth_exceeded|oversized_box|no_moov|no_tracks|no_sample_table|not_started|seek_out_of_range|scan_budget_exhausted|not_mp4|truncated)" crates/demux/src/mp4/mod.rs` (call-site map, 53 hits); `rg -n "depth_exceeded|depth-exceeded|scan-budget-exhausted|oversized-box|oversized_box"` repo-wide (dead-code proof); `sed -n '78,90p'`, `'92,104p'` (quote verification) |

## 1 One-paragraph verdict

This is a real error taxonomy, not stringly-typed mush: every constructor mints an `EngineError` at `PipelineStage::Demux` carrying a structured `ErrorKind` (Unsupported/Corrupt/ResourceLimit) with typed payloads — `CorruptDetail::Truncated { expected, found }`, `InvalidStructure { offset }`, `LimitKind::Memory / RecoveryAttempts` — plus a stable scannable code (`demux.mp4.not-mp4`, `demux.mp4.truncated`, …) that tests.rs pins by exact string equality. The taxonomy is deliberately NOT redefined locally (it delegates to media-types' seven kinds, matching the wav/ogg constructor pattern), and the "No secrets" doc constraint (byte counts/offsets/box kinds only, never paths/URIs/payload) is honored by every signature except one that takes a box kind and then throws it away. Two honesty defects: `depth_exceeded` is a phantom — its doc advertises a box-descent depth guard, `mod.rs:31` claims the walk runs "with a depth guard ([`MAX_BOX_DEPTH`])", the constant `MAX_BOX_DEPTH` sits at mod.rs:126, and not one line of code in the repository calls either — a documented safety property that does not exist; and three of the twelve codes (`oversized-box`, `scan-budget-exhausted`, `depth-exceeded`) are never asserted by any test in the file's own test suite (tests.rs), leaving the bounded-recovery claims resting on the swallow-everything fuzz harness.

## 2 What actually works (verified)

- Typed payloads with real values: `truncated(expected, found)` (41-48) carries the numbers the compat-matrix raw rows then surface verbatim (`"truncated data: expected 425516 bytes, found 12 (demux.avi.truncated)"` shape for the avi twin — the mp4 codes surface the same way through `EngineError` Display).
- `not_mp4` embeds `FormatInfo { name, sniffed }` with capped sniffed bytes (27-37) — probe diagnostics, not stringly mush.
- `scan_budget_exhausted` (161-170) reports `LimitKind::RecoveryAttempts` with budget=`MAX_FRAGMENT_SCAN_BOXES` — a real resource-limit refusal wired at mod.rs:718/1856/2210.
- Code stability: the `demux.mp4.*` code strings are pinned by exact `assert_eq!` in tests.rs for 9 of the codes (not-mp4, no-moov, truncated, box-overruns-parent, no-tracks|no-sample-table via disjunction, not-started, seek.out-of-range, plus Recovered reasons distinct from error codes).
- Stage discipline: single `const STAGE: PipelineStage = PipelineStage::Demux` (19) applied by every constructor — no stage forgery.
- The `_kind` discipline claim in the module doc ("Constructors take byte counts, offsets and box kinds only") holds for 11/12 signatures (the exception is E-002).

## 3 Findings

### E-001 [S2] `depth_exceeded` is a phantom constructor: the advertised depth guard does not exist anywhere in the codebase
Where: `crates/demux/src/mp4/error.rs:78-90` (+ mod.rs:31, mod.rs:124-126)
Code (error.rs, verified `sed -n '78,90p'`):
```rust
/// The box-descent depth exceeded the walk's guard (`moov → trak →
/// mdia → minf → stbl → …` is six; anything deeper than the guard is
/// a cycle-shaped lie, not a file).
pub fn depth_exceeded(offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure {
                offset: Some(offset),
            },
        },
        "demux.mp4.depth-exceeded",
    )
}
```
Code (mod.rs:31, verified `sed -n '29,33p'`):
```rust
//! - **Structural boxes are read whole, ceiling-checked** — `moov` ≤
//!   [`MAX_MOOV_BYTES`], each `moof` ≤ [`MAX_MOOF_BYTES`] — then
//!   walked in memory with a depth guard ([`MAX_BOX_DEPTH`]) and a
//!   child>parent check on every level. Sample payloads are *never*
```
Code (mod.rs:124-126):
```rust
/// Box-descent depth guard (`moov→trak→mdia→minf→stbl` is five;
/// `moov→udta→meta→ilst→©nam→data` is six).
pub const MAX_BOX_DEPTH: u32 = 8;
```
What's wrong: repo-wide `rg "depth_exceeded|depth-exceeded"` finds the constructor, its code string, and UNRELATED hits in `crates/playlists/src/mpd.rs` (a different demuxer, which really has a depth guard and tests it: `playlists.mpd.depth-exceeded`). ZERO call sites exist in `crates/demux/src/mp4/`. `MAX_BOX_DEPTH` has exactly two mentions in the whole repo — its doc and its declaration (mod.rs:31, 126). `iter_boxes` (mod.rs:345-384) takes no depth parameter; the walk cannot detect or refuse deep descent. The module doc at mod.rs:31 tells the reader the walk runs "with a depth guard ([MAX_BOX_DEPTH])" — a documented protection that does not exist.
Why it matters: this is a false safety claim in the primary module documentation plus dead API. The container-matrix narrative "hostile class never exceeds bounded memory/time" (docs/feasibility/sections/subsystems.md:123) cites probe-depth bounding as the pass criterion; a reader auditing protection finds a constructor and a constant and no enforcement. (Material risk is low: the descent only recurses into a fixed set of known box names — moov→trak→mdia→minf→stbl and udta→meta→ilst — so unbounded recursion is not reachable through the implemented vocabulary; the guard's absence matters as a false claim, not as a live overflow.)
What correct looks like: either thread a `depth: u32` through `iter_boxes`/parse_trak and return `error::depth_exceeded(pos)` past `MAX_BOX_DEPTH`, or delete the constructor, the constant, and the mod.rs:31 sentence. Half-shipped state (doc + dead fn) is the worst of the three options.
Confidence: HIGH (repo-wide grep zero call sites; both doc quotes sed-verified).

### E-002 [S4] `oversized_box` accepts the box kind and drops it — the error cannot name the box
Where: `crates/demux/src/mp4/error.rs:95-104`
Code (verified `sed -n '92,104p'`):
```rust
/// A structural payload (`moov`, `moof`) exceeds the demuxer's
/// in-memory ceiling — refused as a typed resource limit rather than
/// trusted into an allocation.
pub fn oversized_box(_kind: &[u8; 4], budget: u64, attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget,
            attempted,
        },
        "demux.mp4.oversized-box",
    )
}
```
What's wrong: `_kind` is underscore-dropped. The module doc advertises "Constructors take byte counts, offsets and box kinds only" — the kind is taken then discarded, so the minted error cannot say whether the `moov` or a `moof` blew the 16 MiB/1 MiB ceilings (the single call site mod.rs:768 hardcodes `b"moov"` into the argument for nothing).
What correct looks like: fold the kind into `FormatInfo`/detail or drop the parameter and the doc's "box kinds" clause.
Confidence: HIGH.

### E-003 [S3] Three of twelve taxonomy codes have zero assertions in the unit suite that owns this file's semantics
Where: `crates/demux/src/mp4/tests.rs` (absence) vs error.rs:95-104, 161-170, 81-90
What's wrong (verified by grep over tests.rs): `demux.mp4.oversized-box`, `demux.mp4.scan-budget-exhausted`, `demux.mp4.depth-exceeded` never appear in any `assert`. The bounded-recovery story ("a pathological moof ladder is a typed failure, never a hang", error.rs:158-160) is therefore pinned nowhere at unit level; the malformed-derivative integration suite runs small mutated fixtures whose moov cannot reach 16 MiB and whose moof ladders cannot exhaust the scan budget, and the fuzz harness swallows errors by contract (fuzz.rs:7-9: "malformed input is *expected* to produce typed errors, which are swallowed here").
Why it matters: the typed-refusal narrative is this crate's core quality claim; untested error paths are exactly where regressions hide (a budget variable accidentally made enormous, or the ceiling check removed, would pass every gate).
What correct looks like: one unit test per guard: patch a fixture's moov size over `MAX_MOOV_BYTES` → assert `oversized-box`; synthesize >`MAX_FRAGMENT_SCAN_BOXES` moofs → assert `scan-budget-exhausted`; after E-001 is fixed, a >8-deep nesting → assert `depth-exceeded`.
Confidence: HIGH for the absence (grep); MEDIUM for the integration-suite characterization (mp4_fixtures.rs read at header level only).

## 4 Per-function/per-test table

No tests live in this file (unlike `wav/error.rs` and `ogg/error.rs`, which carry inline `#[cfg(test)]` modules — mp4/error.rs does not; grep `#[cfg(test)]` over the file: zero hits).

| Constructor | Lines | Production call sites (mod.rs) | Pinned by tests.rs? |
|---|---|---|---|
| `not_mp4` | 27-37 | 701 | YES (exact, tests.rs:478) |
| `truncated` | 41-48 | 351, 357, 398, 405, 743, 773, 1593, 1605, 1787, 1800, 1828 | YES (exact, tests.rs:512, 684, 687) |
| `bad_box` | 53-62 | 359, 369, 375, 411-426, 879, 1019, 1141, 1143, 1156, 1163, 1183 | indirectly (no direct pin; fires via stsd/entry paths) |
| `box_overruns_parent` | 67-76 | 372 | YES (exact, tests.rs:618) |
| `depth_exceeded` | 81-90 | **NONE — dead** | NO (E-001) |
| `oversized_box` | 95-104 | 768 | NO (E-003) |
| `no_moov` | 107-114 | 761 | YES (exact, tests.rs:490) |
| `no_tracks` | 119-126 | 777 | YES (via disjunction, tests.rs:592/647) |
| `no_sample_table` | 130-139 | 795, 1146, 1152 | YES (via disjunction, tests.rs:647) |
| `not_started` | 142-147 | 2011, 2021, 2052 | YES (exact, tests.rs:696-698) |
| `seek_out_of_range` | 151-156 | 2062-2075, 2116, 2213 | YES (exact, tests.rs:444, 759) |
| `scan_budget_exhausted` | 161-170 | 718, 1856, 2210 | NO (E-003) |
| `mint` (private) | 21-23 | all | — |

## 5 Doc claims touching this file

- error.rs:3-7 — "The error *taxonomy* is media-types' seven kinds — never redefined here … mirroring the `wav`/`ogg` constructor pattern." VERDICT: TRUE (all 12 constructors delegate to `media_types` kinds; no local error enum).
- error.rs:9-13 — "# No secrets … Constructors take byte counts, offsets and box kinds only — never paths, URIs or payload content." VERDICT: TRUE for 11/12, FALSE in spirit for `oversized_box` (E-002: takes a box kind and discards it).
- mod.rs:31 — "walked in memory with a depth guard ([`MAX_BOX_DEPTH`])" — VERDICT: **FALSE** (E-001; guard and constructor have zero enforcement).
- `docs/formats/container-mp4.md:66-67` — "`demux.mp4.*` vocabulary; … no panics (fuzz-lite + in-crate fuzz_mutations green)". VERDICT: vocabulary real (12 codes); "no panics" supported at the fuzz-contract level (see fuzz.rs audit).
- `docs/formats/container-mp4.md:59` — 0128 lane "refuses typed `no-tracks`" — consistent with the `no_tracks` constructor existing and the stsd vocabulary gap recorded; UNVERIFIED (corpus lane, env-dependent).

## 6 Dead/duplicated code

- `depth_exceeded` (81-90): DEAD (E-001) — delete or wire it.
- `MAX_BOX_DEPTH` (mod.rs:126): DEAD constant (E-001 companion).
- `oversized_box`'s `_kind` parameter (95): dead parameter (E-002).
- No duplication: 12 constructors are one `mint` call each; nothing copy-pasted.

## 7 Open questions

1. Did the depth guard ever exist and get dropped (git history), or was mod.rs:31 written aspirationally against a never-implemented guard? The repo's own mpd.rs shows the team knows how to implement + test a depth refusal — the mp4 version's absence looks like an unfinished port of that pattern. UNVERIFIED (git archaeology out of scope).
2. Does any integration/hostile suite assert `scan-budget-exhausted` (e.g. apps/cli hostile lanes or soak scenarios with pathological moof ladders)? Grep over docs/soak showed avi/wav/mkv scenarios only — none for mp4 moof ladders found. UNVERIFIED beyond those greps.
