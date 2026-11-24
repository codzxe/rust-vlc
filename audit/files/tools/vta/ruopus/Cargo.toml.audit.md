# AUDIT: tools/vta/ruopus/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W17-A09 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines / functions | 15 / 1 manifest (empty [workspace] + [package] + 3 [dependencies]) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — standalone non-workspace harness crate; the DECODER it pins (ruopus 0.1.2) IS shipped via codecs/Cargo.toml:137 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | root Cargo.toml:41 members read; lock cross-read (ruopus entry L77-79); record:49 quote match; sed quote verification |

## 1. One-paragraph verdict
A correct standalone-spike manifest: the empty `[workspace]` table genuinely isolates the harness from the root workspace, both path deps resolve, and the committed lock pins the exact ruopus 0.1.2 bytes the differential measured (checksum byte-identical to the root lock — W17-A07's lock audit). Two rot spots: the comment under-lists the root workspace's actual members, and the ruopus requirement is caret-style where the shipped pin is exact — the harness's fidelity to its recorded evidence rests entirely on not regenerating the lock.

## 2. What actually works (verified)
— verified: `[workspace]` (empty table) at :4 pins its own workspace root — root Cargo.toml:41 `members = ["crates/*", "apps/*", "tools/xtask", "tests/integration/first-vertical", "tools/mem-audit", "tools/sweep"]` contains no tools/vta glob.
— verified: path deps `demux = { path = "../../../crates/demux" }` (:14) and `plugin-api = { path = "../../../crates/plugin-api" }` (:15) both resolve to real crates; the lock's path-packages (demux, plugin-api, media-types, codec-api — tools/vta/ruopus/Cargo.lock:5-18,42-55) are exactly those crates' transitive closure.
— verified: `ruopus = { version = "0.1.2", default-features = false, features = ["std"] }` (:13) matches record ruopus-0.1.2.md:49 verbatim; the lock pins ruopus 0.1.2 checksum `fcfcd963c83fc1ada2180ccdac7f1568d87fb0f2ad7236b266fdc9fb9122fbfd` (lock:76-79) — byte-identical to root Cargo.lock (W17-A07's audit, verified REAL) — so harness-candidate == shipped-decoder bytes.
— verified: `publish = false` (:10).

## 3. Findings
F-001 [S4]
Where: tools/vta/ruopus/Cargo.toml:2-3
Code: `# NOT a workspace member (root members = crates/*, apps/*, tools/xtask) —
# the empty [workspace] table below pins it to its own workspace root.`
What is wrong: the parenthetical under-lists the root members — actual root Cargo.toml:41 has six entries (`crates/*, apps/*, tools/xtask, tests/integration/first-vertical, tools/mem-audit, tools/sweep`); the comment names three. The substantive claim (not a member) is TRUE, but the comment is stale relative to the members list it quotes.
Confidence: HIGH — Verification: read (both lines quoted).

F-002 [S4]
Where: tools/vta/ruopus/Cargo.toml:13
Code: `ruopus = { version = "0.1.2", default-features = false, features = ["std"] }`
What is wrong: caret-style requirement — `cargo update` (or any lock regeneration) can float the harness to a future ruopus 0.1.x, silently decoupling the spike from the differential evidence recorded in docs/dependencies/records/ruopus-0.1.2.md. The shipped pin for the same crate is exact (`=0.1.2`, codecs/Cargo.toml:137 per the record audit); this manifest relies entirely on the committed Cargo.lock for fidelity.
Confidence: HIGH — Verification: read (lock cross-read).

## 4. Test honesty
No tests in this manifest; the harness binary (src/main.rs) audited REAL by W17-A04.

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NOT a workspace member" | :2 | REAL | root members list has no tools/vta |
| "the empty [workspace] table below pins it to its own workspace root" | :3 | REAL | :4 empty [workspace] + own Cargo.lock present |
| record's dependency form | ruopus-0.1.2.md:49 | REAL | byte-identical string at :13 |

## 6. Dead / duplicated / unreachable code
None. edition 2021 vs workspace crates' 2024 — era marker only, no defect.

## 7. Dependencies & flags
ruopus 0.1.2 (registry, std feature, default-features off — a leaf: no dependencies in the lock entry), demux + plugin-api (path). No optional features of its own.

## 8. Open questions for the next agent
- None.
