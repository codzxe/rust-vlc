# CLAIM AUDIT: docs/dependencies/records/rav1d-1.1.0.md
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Claims extracted | 18 |
| REAL | 9 |
| REAL-UNPROVEN | 2 |
| FALSE | 1 (wasm32-compiles, contradicted by later measurement) |
| UNVERIFIED | 6 (network / vendored-source / sabotage-build) |
| Internal contradictions found | 1 (with crates/codecs/Cargo.toml) |

## 1. Blunt verdict
Repo-checkable core is exact: the asm-off pin, the lock checksum, all 21 transitive
versions of the R2 paste, the ADR-010 AV1 row, and both AV1 test suites (differential
6/6, conformance 1/1) verified through the orchestrator baseline. One real defect:
R5 asserts "wasm32 compiles the generic paths" as fact — a later in-tree measurement
(crates/codecs/Cargo.toml:62-65, "39 errors, measured 2026-11-10") proves rav1d does
NOT compile for wasm32. The record hedged it with a VERIFY-P1 tag but still stated it
as a fact; the fact is false. Everything network-bound (OSV, sparse-index, tarball
source counts, the sabotage build) is honestly labeled and unverifiable here.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "`rav1d = { version = \"=1.1.0\", default-features = false, features = [\"bitdepth_8\", \"bitdepth_16\"] }`" | L25-26, L243-244 | REAL | crates/codecs/Cargo.toml:125 — identical, asm OFF |
| 2 | "ck 1932f060d5e7bd49dc9f8b272c1dc5e9ce0ffe141c28be900265d3989b36c9ed" | L73, L206-207 | REAL | Cargo.lock rav1d block checksum — BYTE-IDENTICAL |
| 3 | R2 graph: assert_matches 1.5.0, atomig 0.4.3, atomig-macro 0.4.0, bitflags 2.13.2, cfg-if 1.0.5, libc 0.2.191, parking_lot 0.12.5, lock_api 0.4.14, scopeguard 1.2.0, parking_lot_core 0.9.12, smallvec 1.16.4, paste 1.0.15, raw-cpuid 11.6.0, strum 0.26.3, strum_macros 0.26.4, to_method 1.1.0, zerocopy 0.7.35, zerocopy-derive 0.7.35, nasm-rs 0.3.2 | L74-97 | REAL | all 19 normal+build graph versions + jobserver/byteorder verified in Cargo.lock by rg; lock rav1d deps list carries exactly these nodes |
| 4 | "**ADR-010:** the \"Software decoders implemented\" table carries the AV1 row — `crates/codecs/src/video/av1/**` + `rav1d =1.1.0` asm-off" | L248-250 | REAL | docs/adr/ADR-010-codec-patent-shipping.md:76 (AV1 row, verbatim) |
| 5 | "`av1_differential` — **6/6 pass**" | L255-256 | REAL | baseline cargo-test-workspace.txt:989-1000: `running 6 tests` -> `ok. 6 passed; 0 failed` (incl. differential_pixel_md5_against_ffmpeg_dump) |
| 6 | "`av1_conformance` — **1/1 pass**" | L256-257 | REAL | baseline:977-988: `test conformance_matrix_vs_ffmpeg_generated_set ... ok`, `1 passed; 0 failed` |
| 7 | "`cargo test -p codecs` → 643 ... 624 pass, 18 ignored ... 0 fail" | L252-254 | REAL (for its date) / drifted | baseline now: codecs 773 passed / 18 ignored / 0 failed — 18-ignored matches exactly |
| 8 | "wasm32 compiles the generic paths" | L142-144 | **FALSE** | crates/codecs/Cargo.toml:62-65: `rav1d does not compile for wasm32-unknown-unknown (39 errors, measured 2026-11-10 — asm-gated core code ...)` — the later measurement contradicts the record's stated fact |
| 9 | "tarball **ships 174 native asm files** ... EXCLUDED from compilation by the asm-off pin" | L41-44 | UNVERIFIED | registry tarball claims; the asm-OFF half is corroborated by the lock (nasm-rs listed as build-dep but nothing links native code) + baseline release build |
| 10 | "Sabotage build (executed 2026-10-26): ... `CC=/nonexistent-cc ... cargo build` ... **build succeeded**" | L118-125 | REAL-UNPROVEN | past-sandbox execution, not re-runnable here (cargo-build-budget); the claim is coherent with asm-off pin |
| 11 | "OSV ... every node ... queries clean except `paste 1.0.15` → RUSTSEC-2024-0436 (informational ... unmaintained)" | L192-195 | UNVERIFIED | network advisory claims |
| 12 | "rav1d 1.1.0: published 2025-05-07, `rust_version` 1.79, not yanked" | L188 | UNVERIFIED | network |
| 13 | "`unsafe` surface: large (≈642 `unsafe` tokens across `src/`" | L53 | UNVERIFIED | vendored source not in sandbox |
| 14 | "the `dav1d_*` C-ABI-shaped entry points ... are the only public Rust API of 1.1.0" | L150-158 | REAL-UNPROVEN | our wrapper uses them (crates/codecs/src/video/av1/decoder.rs exists); rav1d-side API shape is registry-source |
| 15 | "film-grain application is left at rav1d's `apply_grain` default" | L159-162 | REAL-UNPROVEN | wrapper-side decision; effect only visible in differential fixtures |
| 16 | "thread pool defaults ... our default pins 1 and exposes the budget up to 256" | L162-165 | REAL-UNPROVEN | codec-api ThreadBudget wiring; not re-executed |
| 17 | "the R3 sabotage build ... `[VERIFY-P0]` closed by direct experiment" | L121-125 | REAL-UNPROVEN | executed evidence from 2026-10-26; consistent with everything in-tree |
| 18 | "No decode claim is made without our own ffmpeg-authored differential run behind it" | L230-232 | REAL | the differential tests exist and pass in baseline (#5) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "wasm32 compiles the generic paths" (R5, L142-144) | crates/codecs/Cargo.toml:62-65: "rav1d does not compile for wasm32-unknown-unknown (39 errors, measured 2026-11-10 — asm-gated core code)" | The Cargo.toml comment is the measurement-backed one (a date, an error count, and cross-refs to apps/web target gate + docs/web/size-report.md). The record stated an unmeasured fact that later proved false. Severity for THIS record: S3 — an admission record that misinforms the wasm lane decision. |

## 4. Fabrication / inflation indicators
None detected in the repo-verifiable subset (checksum + 21 versions + test counts all
exact). The wasm32 line is over-assertion rather than fabrication — the record even
carried the VERIFY-P1 tag but wrote the sentence as fact anyway.
