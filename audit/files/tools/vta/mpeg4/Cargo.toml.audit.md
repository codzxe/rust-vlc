# AUDIT: tools/vta/mpeg4/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W17-A02 |
| Date | 2026-11-24 |
| Lines / functions | 19 / manifest only (no functions) |
| File verdict | REAL (manifest of a real, executed, archived VTA harness) |
| Reachable from shipped entry point? | NO by design — standalone SPIKE harness, NOT a workspace member, NOT shipped (deliberate isolation) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read; `grep -c oxideav Cargo.lock` -> 0 (not a workspace dep); grep [lints] of the three path-dep crates (all `workspace = true`); `ls src` -> main.rs 332 lines with `fn main()` at :278, 0 todo!/unimplemented!; mpeg4 Cargo.lock parsed (30 pkgs incl. demux/plugin-api/media-types/codec-api/oxideav-*) |

## 1. One-paragraph verdict
A clean manifest that does exactly what its comments claim: the empty
`[workspace]` table pins this crate to its own workspace root (verified: it
matches no member glob of the root workspace and carries its own 30-package
Cargo.lock), `publish = false`, exact pin `oxideav-mpeg4video = "=0.1.8"`
matching the dependency record, and path dependencies into the three
workspace crates the harness differentially exercises. The harness is real:
src/main.rs is 332 lines of actual demux-and-decode code, and the sibling
README carries a full 8-lane results snapshot from a 2026-10-26 run. The
only defect is a stale membership comment that under-represents the root
workspace members. This is an ARCHIVED BLOCKED spike by its own verdict —
nothing here is wired into the product.

## 2. What actually works (verified)
- Standalone-workspace isolation verified: not matched by root members
  (`crates/*`, `apps/*`, `tools/xtask`, `tools/mem-audit`, `tools/sweep`,
  `tests/integration/first-vertical`); own Cargo.lock present.
- `oxideav` appears 0 times in the root Cargo.lock — the README's "NOT a
  dependency of any workspace crate" claim is TRUE.
- Path-dep targets exist and resolve (lock lists demux, plugin-api,
  media-types, codec-api, oxideav-core, oxideav-mpeg4video).
- The `[lints] workspace = true` tables on the path deps
  (crates/demux/Cargo.toml:25-26, plugin-api:20-21, media-types:20-21)
  resolve against those crates' own root workspace regardless of the
  dependent — evidenced by the committed lock and the executed results
  snapshot (README:64-81). No build attempted here (load discipline:
  cargo-build-budget).

## 3. Findings
F-001 [S4] — Stale membership comment
Where: tools/vta/mpeg4/Cargo.toml:2-3
Code:
```
# Standalone VTA differential harness for card 6.VTA-oxideav-mpeg4video.
# NOT a workspace member (root members = crates/*, apps/*, tools/xtask) —
```
What is wrong: the root workspace membership is listed as 3 patterns, but
root Cargo.toml declares 6 (`crates/*`, `apps/*`, `tools/xtask`,
`tools/mem-audit`, `tools/sweep`, `tests/integration/first-vertical`). The
CONCLUSION ("NOT a workspace member") remains true for tools/vta/mpeg4 —
nothing matches it — but the supporting fact is out of date and would
mislead anyone reasoning about member globs from this comment.
Confidence: HIGH — Verification: read both manifests.

## 4. Per-function table
N/A — manifest, no functions (19 lines).

## 5. Test honesty
Not a test file. The harness's honesty lives in the README/results snapshot
(tolerance-class divergences honestly reported; verdict BLOCKED recorded in
docs/dependencies/records/oxideav-mpeg4video-0.1.8.md — cross-referenced,
not re-audited here: that record is another agent's file).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NOT a workspace member ... pins it to its own workspace root" (L2-3) | manifest comment | REAL | root members grep; own Cargo.lock (30 pkgs) |
| "oxideav-mpeg4video = "=0.1.8"" exact pin | L13 | REAL | record title oxideav-mpeg4video-0.1.8; mpeg4 lock line 76 |
| "The crate is NOT a dependency of any workspace crate" (README:7-8) | tools/vta/mpeg4/README.md | REAL | grep oxideav Cargo.lock -> 0 |
| Root members list (L2-3) | manifest comment | FALSE (stale list, true conclusion) | F-001 |

## 7. Dead / duplicated / unreachable code
The harness is archived-but-present by decision ("BLOCKED ... unblock paths
recorded"). Not dead code in the shipped sense — UNREACHABLE from the
product by design; kept as reproducible VTA evidence.

## 8. Dependencies & flags
oxideav-mpeg4video =0.1.8 (+oxideav-core transitively), path deps
demux/plugin-api/media-types (the workspace's own crates — the point of the
differential). Edition 2021, publish=false, [profile.dev] opt-level 1.
Corpus referenced by README lives outside the repo (/tmp/my-project/
corpus-v4/media) — reproduction needs external corpus (noted in README; no
in-repo corpus for these lanes).

## 9. Open questions for the next agent
- README's reproduce block hardcodes `/home/z/my-project/worktrees/
  p6-vta-mpeg4/tools/vta/mpeg4/diff.py` — a vanished worktree path; the
  README auditor should decide whether that block is repair-instructions
  or noise.
- Record oxideav-mpeg4video-0.1.8.md owns the BLOCKED verdict details; if
  its "±1..3 on 7/8 lanes" tolerance numbers disagree with the README
  snapshot table, that contradiction belongs to that record's audit.
