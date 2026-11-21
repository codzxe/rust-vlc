# AUDIT: crates/plugin-host-wasm/src/crc32.rs
| Field | Value |
|---|---|
| Auditor | W12-A04 |
| Date | 2026-11-21 |
| Lines / functions | 57 / 5 items (1 fn, 1 static, 1 const fn, 1 test, module docs) |
| File verdict | UNREACHABLE (correct, canonical-vector-tested, consumed in-crate by the .lumenext package verify path — but nothing shipped links plugin-host-wasm) |
| Reachable from shipped entry point? | NO. In-crate chain: `package.rs:328` (`extract` path: `crc32(data)` compared against the entry's stored CRC, `PackageError::CrcMismatch` on mismatch) and the packaging CLI `bin/lumenext.rs:254,272` (write side). Crate-level: no shipped binary depends on plugin-host-wasm (established by audit/files/crates/plugin-host-wasm/Cargo.toml.audit.md: not in lumen, lumen-web, or lumen-tauri graphs). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | Read crc32.rs 1–57 (full); rg `crc32` in crates/plugin-host-wasm (callers mapped: package.rs:31/175/276/328, bin/lumenext.rs:17/254/272); sed package.rs 310–335 (the extract-path CRC comparison); rg `EDB88320\|fn crc32` repo-wide (three CRC-32 variants exist — demux/ogg/page.rs and demux/ts/psi.rs use the non-reflected 0x04C11DB7 family with different parameters; not copy-paste duplicates). |

## 1. One-paragraph verdict
A textbook-correct, dependency-free CRC-32 (ISO-HDLC/ZIP, reflected 0xEDB88320, init/xor 0xFFFFFFFF), const-evaluated table, pinned by the four canonical test vectors — including the 0xCBF43926 check value and the all-ones fixed point. It does exactly what its doc says inside the plugin-host-wasm crate: the `.lumenext` extract path computes it per entry and refuses typed on mismatch. The only thing wrong with this file is where it lives: a correct integrity primitive inside a WASM plugin host that no shipped binary links, verifying packages for a plugin system with no real plugins — polished masonry in an unbuilt house.

## 2. What actually works (verified)
- Algorithm verified line-by-line against the standard byte-wise reflected form: `crc = (crc >> 8) ^ TABLE[(crc ^ byte) & 0xFF]`, init 0xFFFFFFFF, final xor 0xFFFFFFFF (crc32.rs:9–15) — correct.
- Table generation (19–37) is the standard 8-round 0xEDB88320 shift, `const fn` → compile-time `static TABLE` (17) — no runtime init cost.
- All four test vectors are the published canonical values: `crc32("") = 0`, `crc32("123456789") = 0xCBF43926` (the ISO-HDLC check value), `crc32("The quick brown fox…") = 0x414FA339`, `crc32([0xFF;4]) = 0xFFFFFFFF` (the reflected polynomial's fixed point). Tests executed in baseline #3 (`cargo test --workspace` — plugin-host-wasm suite; worklog records plugin-host 44 tests).
- The doc claim "used to integrity-check every `.lumenext` ZIP entry we extract" is REAL: package.rs:327–333 computes and compares on the extract path, refusing `PackageError::CrcMismatch { name }` typed.

## 3. Findings
None. What was checked, and how:
- **Fakeness:** no todo!/unimplemented!/constant returns — output depends on every input byte (verified by reading the loop; the four vectors exercise distinct outputs).
- **Correctness:** table construction, byte loop, init/final-xor conventions, index masking `& 0xFF`, no endianness dependence, no overflow paths, no panics possible (indexing is 0–255 by construction). Checked by line-read; vectors would fail on any of these being wrong.
- **Performance:** byte-at-a-time table CRC over package entries (small archives, one-shot at install) — appropriate; no hot path.
- **Tests:** would they fail if the code were broken? YES — any deviation in polynomial, init, reflection, or final xor changes at least one of the four published values; the fixed-point vector additionally catches a wrong table build.
- **Dead code / duplication:** in-crate callers exist (package read + write sides); the two other CRC-32s in the tree (demux/ogg/page.rs:205, demux/ts/psi.rs:91) are different CRC variants (non-reflected 0x04C11DB7 family, different check values — Ogg's 0x89A1897F) — not duplicates of this one.
- **Docs:** header claims match verified reality, with one process claim noted UNVERIFIED below.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-5 | REAL | — | "cross-checked against zlib" is a process claim I cannot re-run — UNVERIFIED, but the four vectors are the published canonical values independent of that claim. |
| crc32 | 9-15 | REAL | — | Standard reflected form; correct. |
| TABLE (static) | 17 | REAL | — | Const-evaluated; private. |
| build_table (const fn) | 19-37 | REAL | — | Standard 8-round generation; no panic paths. |
| tests: canonical_vectors | 43-56 | REAL | — | Four published values incl. check value + fixed point; mutation-sensitive. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| canonical_vectors (44) | YES | Any error in polynomial/reflection/init/final-xor/table changes at least one published constant; e.g. dropping the final xor makes `crc32(b"")` return 0xFFFFFFFF, not 0. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "CRC-32 (ISO-HDLC / IEEE 802.3, the ZIP check value)" | crc32.rs:1 | REAL | Algorithm + vectors (0xCBF43926 is that variant's check value). |
| "used to integrity-check every `.lumenext` ZIP entry we extract" | crc32.rs:4 | REAL | package.rs:327–333 compute-compare-refuse on the extract path. |
| "No dependency; verified against the canonical vectors in tests" | crc32.rs:5 | REAL | std-only; tests present with canonical values. |
| "cross-checked against zlib" | crc32.rs:46 | REAL-UNPROVEN | The vectors are the published canonical values (verified by knowledge of the standard constants); the zlib cross-check run itself is not reproducible from the repo. |

## 7. Dead / duplicated / unreachable code
Unreachable at product level (crate linked by no shipped binary — see header). Not dead in-crate. Not duplicated (the tree's other two CRC-32s are different variants for Ogg and MPEG-PSI). The honest structural observation: this is one more polished component of the plugin packaging pipeline (crc32 + sha256 + fsjail + wasmi host + supervisor) whose consumer story is the wave's headline question — nothing ships it, no real plugin exists to package.

## 8. Dependencies & flags
std-only, no features, no unsafe, no deps — exactly as documented. Compiles under the workspace deny-lints (baseline #5 clippy CLEAN).

## 9. Open questions for the next agent
- Whoever audits `install.rs` (A09): the extract path that consumes this CRC — confirm every entry type actually goes through `extract_entry`'s check (the doc says "every entry"; I verified the one extract function, not all call sites of it).
- Wave gate: this file is the counterpoint to filter.rs in the same domain — here the code is complete and correct, but the subsystem it serves is unshipped and guestless; the defect is strategic (dead weight), not local.
