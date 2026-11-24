# AUDIT: tools/vta/ac3/Cargo.lock (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines | 152 (17 package entries, read in full) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — standalone archived harness, not a workspace member |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read (full); cat tools/vta/ac3/Cargo.toml + README.md (cross-ref) |

## 1. One-paragraph verdict
A genuine cargo-generated (version 4) lockfile for the ARCHIVED vta-ac3
harness: 17 packages, all `registry+https://github.com/rust-lang/crates.io-index`
with checksums, root `vta-ac3 0.0.0` -> `oxideav-ac3 0.0.11` +
`oxideav-core 0.1.42`. It is consistent with the tool's Cargo.toml and
proves the measured decoder is the crates.io crate (not a vendored copy).
This harness belongs to the BLOCKED family — the ac3 differential FAILED
(corpus lane maxΔ 316 LSB per the README and the oxideav-ac3-0.0.11.md
record) — so it is NOT a source of any "bit-exact" claim.

## 2. What actually works (verified)
— Entry cross-check: lock's `vta-ac3 0.0.0` deps (oxideav-ac3, oxideav-core)
match Cargo.toml's declared deps exactly (cat verified).
— The `oxideav-core` dependency list (bytemuck, serde_json, thiserror —
lock:37-41) corroborates the oxideav-mpeg4video record's R2 claim that
serde_json is a HARD dep of oxideav-core (same-author family fact).

## 3. Findings
None. All 17 entries carry registry sources + SHA-256 checksums; no git
deps, no path deps, no local-only entries; the harness needs no demuxer
deps (it consumes raw .ac3 ES via its own syncwalker, per the README).

## 4. Data summary
Packages: bytemuck 1.25.2, itoa 1.0.18, memchr 2.8.3, oxideav-ac3 0.0.11,
oxideav-core 0.1.42, proc-macro2 1.0.107, quote 1.0.47, serde 1.0.229,
serde_core 1.0.229, serde_derive 1.0.229, serde_json 1.0.154, syn 3.0.9,
thiserror 2.0.21, thiserror-impl 2.0.21, unicode-ident 1.0.27, vta-ac3
0.0.0, zmij 1.0.23. The oxideav closure is 2 crates + a serde/thiserror
chain — the "codec pulls a JSON stack" shape the dependency records flag.

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "oxideav-ac3 ... NOT a dependency of any Lumen workspace crate" | tools/vta/ac3/README.md:3-6 | REAL | rg over crates/*/Cargo.toml: no oxideav anywhere in workspace manifests; root Cargo.lock has no oxideav entries (rg verified) |
| "the crate's only output format interleaved s16le" | README (context) | N/A for this file | main.rs is another agent's assignment |

## 6. Open questions
Whether oxideav-ac3's 0.0.x version line ever stabilized post-audit —
registry status not checkable from this sandbox (no network): UNVERIFIED.
