# AUDIT: tools/vta/mpeg4/Cargo.lock
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 262 / 30 package entries (25 registry + 5 path) |
| File verdict | REAL-UNPROVEN (structurally valid Cargo v4 lock; checksums not verifiable offline) |
| Reachable from shipped entry point? | NO — standalone VTA harness, explicitly NOT a workspace member (Cargo.toml `[workspace]` empty-table pin; README: "The crate is NOT a dependency of any workspace crate"). Reachable only by a human running `cargo build` inside tools/vta/mpeg4. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | grep counts (30 names, 25 checksums, 25 registry sources); root-lock cross-greps (zmij, serde_json, oxideav, rust_h264); read of Cargo.toml/README |

## 1. One-paragraph verdict
A well-formed Cargo v4 lockfile for the archived VTA differential harness. Every registry package carries a checksum; the five local packages (vta-mpeg4, demux, plugin-api, media-types, codec-api) correctly carry none. The root package's dependency set in the lock exactly matches Cargo.toml, and the `=0.1.8` exact pin on `oxideav-mpeg4video` is honored (lock entry 0.1.8, checksummed). The serde_json 1.0.154 entry (including its `zmij 1.0.23` dependency) is byte-consistent with the root workspace Cargo.lock — same version, same checksum — so both locks came out of the same resolver. Nothing tampered-looking. Cannot verify checksums against crates.io (no network, no cargo on this shell PATH).

## 2. What actually works (verified)
— verified by grep: `grep -c '^name = '` -> 30 packages; `grep -c 'checksum'` -> 25 (all registry entries); path deps (demux, plugin-api, media-types, codec-api, vta-mpeg4) have `source` absent — correct.
— verified by grep + read: lock's vta-mpeg4 deps (demux, media-types, oxideav-mpeg4video, plugin-api) == Cargo.toml [dependencies]; `oxideav-mpeg4video = "=0.1.8"` pin -> lock `version = "0.1.8"`.
— verified by cross-grep: root Cargo.lock:903/1438 also lists `zmij` under serde_json 1.0.154 (checksum `e7e9cc8b...04bd6` identical in both locks) — consistent resolver output, not a hand-edit of one file.

## 3. Findings
F-001 [S4]
Where: tools/vta/mpeg4/Cargo.lock:143-154 (and Cargo.toml pin context)
Code:
```
name = "serde_json"
version = "1.0.154"
...
dependencies = [
 "itoa",
 "memchr",
 "serde",
 "serde_core",
 "zmij",
]
```
What is wrong: nothing provable — but two entries cannot be verified from this sandbox: (a) registry checksums (would need crates.io or `cargo --locked`), (b) the crate name `zmij` (a serde_json dependency in this 2026-era tree) is unrecognizable to the auditor and unverifiable offline. Why it matters: a hostile lock could smuggle a lookalike crate; consistency with the root lock is the only evidence available. What correct looks like: CI cargo-deny/bans gate (root lane) covering this tool's lock too — it does NOT: this standalone workspace's lock is outside the audited supply chain of the main repo (see README "NOT a dependency of any workspace crate"). Confidence: LOW that anything is wrong — Verification: read only; checksums/crates.io UNVERIFIED (no network, no cargo binary on auditor PATH).

## 4. Per-function table
Not applicable (lockfile, no functions). Entries inventoried: 30 packages — bytemuck 1.25.2, codec-api (path), demux (path), equivalent 1.0.3, hashbrown 0.17.1, indexmap 2.14.2, itoa, media-types (path), memchr, oxideav-core 0.1.42, oxideav-mpeg4video 0.1.8, plugin-api (path), proc-macro2 1.0.107, quote 1.0.47, serde 1.0.229, serde_core 1.0.229, serde_derive 1.0.229, serde_json 1.0.154, serde_spanned 1.1.2, syn 3.0.9, thiserror 2.0.21, thiserror-impl 2.0.21, toml 1.1.8+spec-1.1.0, toml_datetime 1.1.2, toml_parser 1.1.5, toml_writer 1.1.3, unicode-ident 1.0.27, vta-mpeg4 (path), winnow 1.0.4, zmij 1.0.23.

## 5. Test honesty
No tests in a lockfile.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Standalone VTA differential harness ... `oxideav-mpeg4video` 0.1.8 ... NOT a dependency of any workspace crate" | tools/vta/mpeg4/README.md:1-8 | REAL | Cargo.toml `[workspace]` empty table + path deps only into crates/demux/plugin-api/media-types; root Cargo.lock has NO oxideav entries (grep -> only rust_h264 at :769) |
| "Verdict: BLOCKED — see docs/dependencies/records/oxideav-mpeg4video-0.1.8.md" | README:3-5 | REAL | record file exists (checked in sibling assignment); crate absent from root lock |

## 7. Dead / duplicated / unreachable code
The whole tool is intentionally quarantined (blocked-crate evaluation harness, archived). The lock faithfully pins the evaluated crate version — exactly what an evidence tool should do.

## 8. Dependencies & flags
Lock v4 format. No `patch`/`replace` sections — no tampering seams. Registry-only sources (`registry+https://github.com/rust-lang/crates.io-index`), single registry — no git dependencies, no weird sources.

## 9. Open questions for the next agent
- Verify `zmij 1.0.23` and the oxideav checksums against crates.io when network is available. UNVERIFIED (offline).
- Confirm cargo can regenerate this lock byte-stably (`cargo metadata --frozen` in tools/vta/mpeg4) — blocked: no cargo binary on auditor PATH (exit 127). UNVERIFIED.
