# AUDIT: crates/plugin-host-wasm/src/crc32.rs
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-24 |
| Lines / functions | 57 / 3 (crc32, build_table, 1 test) |
| File verdict | REAL — hand-rolled CRC-32 is byte-exact against zlib |
| Reachable from shipped entry point? | NO (inherited): sole production caller is package.rs:328 (`extract_entry` CRC gate of the `.lumenext` reader), inside plugin-host-wasm, which no release-lane crate depends on. Reached by the lumenext dev CLI + tests. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | cargo test -p plugin-host-wasm --offline --lib crc32 → 1 passed, exit 0; python3 zlib cross-check (2 scripts, 17 vectors total) → ALL MATCH; rg callers (exit 0); sed quote verification (exit 0) |

## 1. One-paragraph verdict
Correct, minimal, dependency-free CRC-32 (ISO-HDLC/IEEE 802.3, reflected poly 0xEDB88320, init/final-xor 0xFFFFFFFF — the ZIP convention), with a compile-time const-built table and one production consumer (the `.lumenext` ZIP entry CRC gate at package.rs:328, typed `CrcMismatch` on disagreement). Hostile focus discharged: I ported the Rust implementation byte-for-byte to Python and diffed it against `zlib.crc32` on 13 vectors — the four canonical check values plus random inputs up to 65537 bytes, all byte values 0-255, and repeated patterns — all match. The in-crate test runs the real code and passes (my run + baseline line "crc32::tests::canonical_vectors ... ok"). The only defect is inherited unreachability from the release lane and a thin 4-vector in-crate test.

## 2. What actually works (verified)
— `cargo test -p plugin-host-wasm --offline --lib crc32` (my run): `canonical_vectors` ok, 1 passed, exit 0. Baseline: cargo-test-workspace.txt (plugin-host-wasm unit block, "test crc32::tests::canonical_vectors ... ok").
— python3: zlib.crc32 == {0x0, 0xCBF43926, 0x414FA339, 0xFFFFFFFF} for the four test inputs — the asserted constants are genuine zlib values, not inventions.
— python3 port of the exact algorithm (init 0xFFFFFFFF; `crc = (crc>>8) ^ TABLE[(crc^b)&0xFF]`; final xor) vs zlib on 13 vectors incl. os.urandom data: ALL MATCH — the implementation, not just its constants, is correct.
— rg: production caller package.rs:328 compares against the entry's declared CRC and returns typed `PackageError::CrcMismatch` (package.rs:329-332).

## 3. Findings
F-001 [S4]
Where: crates/plugin-host-wasm/src/crc32.rs:43-56
Code:
```
    #[test]
    fn canonical_vectors() {
        // Check values from the ISO-HDLC / PNG / ZIP literature,
        // cross-checked against zlib (crc32 of the same inputs).
```
What is wrong: the in-crate test covers exactly 4 fixed vectors (empty, "123456789", the pangram, [0xFF;4]) — no long-input, no random, no all-256-bytes case; "cross-checked against zlib" describes a one-time human provenance, not an executable check (impossible std-only, and honest as a comment). The gap is real but closed by my external zlib diff. What correct looks like: add a 6-7 vector deterministic set (e.g. bytes 0..=255, a 64 KiB pattern) with precomputed constants. Confidence: HIGH — Verification: ran (python3 vs zlib, 13 vectors).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| crc32 | 9-15 | REAL | — | textbook reflected CRC-32; verified byte-exact vs zlib (13 vectors) |
| TABLE / build_table | 17-37 | REAL | — | const fn, compile-time table, standard 0xEDB88320 recurrence — port matches zlib's table by construction of the match |
| tests::canonical_vectors | 43-56 | REAL | S4(F-001) | 4 genuine zlib values incl. the all-ones fixed point; would fail on wrong init/poly/final-xor |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| canonical_vectors | YES | wrong polynomial, init, or final xor changes at least one of the four constants (e.g. init 0 would break the [0xFF;4] and "" cases); executed green in my run |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "std-only CRC-32 (ISO-HDLC / IEEE 802.3, the ZIP check value)" | crc32.rs:1 | REAL | zlib cross-match incl. ZIP convention init/xor |
| "used to integrity-check every `.lumenext` ZIP entry we extract" | crc32.rs:3-5 | REAL | package.rs:328-332 (sole caller; typed CrcMismatch) |
| "verified against the canonical vectors in tests" | crc32.rs:5 | REAL | my run + baseline |
| "initial value 0xFFFF_FFFF, final xor 0xFFFF_FFFF (the ZIP central-directory convention)" | crc32.rs:7-8 | REAL | code lines 10/14 match; zlib equality confirms |

## 7. Dead / duplicated / unreachable code
No dead code in-file. The module's only consumer (package.rs ZIP reader) is itself outside the release lane (see install.rs audit: lumen-cli/desktop do not depend on plugin-host-wasm).

## 8. Dependencies & flags
None — std-only, no features, `pub mod crc32` (lib.rs:78). Table built at compile time (no lazy init, no runtime cost).

## 9. Open questions for the next agent
- package.rs (other agent's file): does the reader also verify the CENTRAL-directory CRC field itself, or only the local-header copy? The fixture writer (tests/common) writes both, but the reader path at 328 reads `entry.crc32` — whose copy that is decides what a corrupting proxy must lie about.
