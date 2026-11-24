# AUDIT: tools/vta/ruopus/Cargo.lock
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 209 / n-a (cargo-generated lockfile; 3 workspace path deps, 5 registry deps) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — standalone non-workspace VTA harness lock (its `[workspace]` empty table pins its own root). The CANDIDATE it pins IS the shipped decoder: root Cargo.lock ruopus == this lock's ruopus |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
A correct, meaningful lockfile: it pins the exact candidate bytes the VTA differential measured, and those bytes are byte-identical (version AND checksum) to what the release lane now ships. ruopus 0.1.2 checksum `fcfcd963c83fc1ada2180ccdac7f1568d87fb0f2ad7236b266fdc9fb9122fbfd` appears identically in root Cargo.lock:763-765 and here (L77-79). Every registry dependency carries a checksum. The harness's candidate became the shipped `codec-opus` default feature — so the VTA evidence chain (docs/dependencies/records/ruopus-0.1.2.md) demonstrably covers the shipped artifact.

## 2. What actually works (verified)
— `diff tools/vta/ruopus/Cargo.lock tools/vta/vp9/Cargo.lock` -> 18 diff lines: only the candidate package block (ruopus vs rusty_vp9) + harness name; everything else identical — the two harness locks are the same scaffold, different candidate.
— Checksum cross-check (ran, byte-compared): root lock ruopus == this lock ruopus (`fcfcd963...`).
— Admission verified: crates/codecs/Cargo.toml:137 `ruopus = { version = "=0.1.2", optional = true, ... }`; `codec-opus` is in codecs' default features (L56); apps/cli/Cargo.toml depends on `codecs` path dep with default features — so `cargo build --release --locked -p lumen-cli` (the release lane) compiles ruopus in.
— All 5 registry entries (equivalent, hashbrown, indexmap, ruopus, serde/proc-macro2 family) have `checksum =` lines.

## 3. Findings
None. Zero findings — checked: version-pinning (`=0.1.2` in Cargo.toml), checksum presence, path-dep integrity (demux/plugin-api point at ../../../crates — correct relative depth), lock format v4, no git deps, no patch sections, no duplicates.

## 4. Per-function table
N/A (data file). Inventory: 9 packages — vta-ruopus (harness root), demux/plugin-api/codec-api/media-types (path deps to the repo's crates), ruopus 0.1.2 (candidate), thiserror/proc-macro2/serde/indexmap+equivalent+hashbrown (transitive, checksummed).

## 5. Test honesty
N/A. The lock's "test" is the differential it enabled; the record exists (docs/dependencies/records/ruopus-0.1.2.md) and the admission is real (Cargo.toml feature wiring verified above).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Standalone (non-workspace) cargo bin ... decodes an Ogg/Opus file through OUR ogg demuxer into ruopus" | tools/vta/ruopus/README.md | REAL | Cargo.toml has `[workspace]` empty table + path deps to crates/demux, plugin-api (read) |
| "Evidence lives in docs/dependencies/records/ruopus-0.1.2.md" | README | REAL | file exists (listed in records dir) |

## 7. Dead / duplicated / unreachable code
The lock is unreachable from the shipped binary by design (spike harness). Its candidate is reachable — see above.

## 8. Dependencies & flags
ruopus 0.1.2, default-features=false, features=["std"] in the harness (Cargo.toml L14) — matching the shipped pin's posture (codecs/Cargo.toml:137 same shape).

## 9. Open questions for the next agent
1. Does the ruopus-0.1.2.md record's admission addendum date post-date this lock (i.e., was the harness re-run at admission time)? Not my file (records dir is another agent's lane).
