# AUDIT: crates/renderer-wgpu/tests/common/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| Lines / functions | 488 / 17 |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A test-support module (integration-test `mod common;`) — consumed by exactly 2 suites: `png_hash.rs` and `presenter_seam.rs`; both executed by me (5/5 and 5/5 pass) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/png_hash-cee1d8862e16caad` → 5 passed (exit 0); `./target/debug/deps/presenter_seam-c985618d2dc2893e` → 5 passed (exit 0); `python3 hashlib` cross-check of all 6 SHA-256 vectors → exact match |

## 1. One-paragraph verdict
This is high-quality test machinery, and unusually trustworthy for test code: the in-file SHA-256 is pinned against six FIPS/NIST vectors which I independently reproduced with python hashlib — all six match byte-for-byte (abc, 448-bit message, empty, 64×'a', 128×'a', and the 56-byte padding-boundary case); the PNG "value-truth" decoder verifies signature, per-chunk CRC-32, IHDR field tuple, zlib header, stored-deflate LEN/NLEN complement, Adler-32, per-row filter bytes and exact scanline size — so a committed PNG hash provably pins a verified image, not encoder output taken on faith. The CRC-32 and Adler-32 implementations are the standard algorithms (read-verified). Both consuming suites pass under my execution. Two hygiene defects only: an unbounded slice in the stored-block walk and a blanket dead-code allow.

## 2. What actually works (verified)
— verified by running both consumer suites (png_hash 5/5 incl. `the_committed_hash_pins_a_verified_image`; presenter_seam 5/5 exercising VirtualClock set_ns/advance_ns) and by `python3 -c hashlib...` reproducing all six vectors in `assert_sha256_vectors` exactly (e.g. 56×'a' → b35439a4ac6f0948..., 128×'a' → 6836cf13bac400...).

## 3. Findings
F-001 [S4]
Where: common/mod.rs:367-373
Code:
```
        assert!(at + 5 <= idat.len(), "stored block header fits at {at}");
```
(then L373) `raw.extend_from_slice(&idat[at + 5..at + 5 + len]);`
What is wrong: the bounds assert covers only the 5-byte stored-block header; the `len`-byte body is then sliced with no check that `at + 5 + len <= idat.len()`. A truncated/corrupt IDAT produces a raw slice index-out-of-bounds panic instead of the positioned assert the module's own doc promises ("the assert messages carry the position", L285-287). Test-only blast radius (a panic still fails the suite). Confidence: HIGH — Verification: read.
F-002 [S4]
Where: common/mod.rs:22
Code:
```
#![allow(dead_code)]
```
What is wrong: blanket dead-code allow on a shared module. Today nothing significant is dead (verified: sha256 family + decode_png used by png_hash; VirtualClock + synthetic_request + expected_pattern + present used by both suites), but the allow will silently absorb future dead helpers — the classic way shared test modules rot. Confidence: HIGH — Verification: read + rg usage census.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Sha256::new | 58-70 | REAL | — | FIPS 180-4 §5.3.3 IV verified |
| Sha256::update | 72-96 | REAL | — | buffering traced through all cases (partial-fill, exact-fill, tail) — tail assignment safe because buf_len is always 0 there |
| Sha256::finish | 98-113 | REAL | — | padding loop terminates through the 64-flush; bit_len captured before padding updates; length bytes written without recounting |
| Sha256::compress | 115-162 | REAL | — | schedule + rounds verified against FIPS table; all adds wrapping |
| SHA256_K | 39-48 | REAL | — | spot-checked first/last and 10 middle constants against the standard table — all match |
| sha256 / sha256_hex | 166-180 | REAL | — | lowercase hex recording format |
| assert_sha256_vectors | 186-221 | REAL | — | 6 vectors incl. one-block/two-block/56-byte boundary; independently reproduced via python hashlib |
| crc32 | 227-237 | REAL | — | reflected 0xEDB88320, init/final XOR — standard |
| adler32 | 239-248 | REAL | — | mod 65521 — standard |
| decode_png | 288-404 | REAL | F-001 | full verification chain (signature→per-chunk CRC→IHDR tuple 8/6/0/0/0→zlib 0x78 0x01→stored blocks with NLEN check→Adler→filter-0 rows with exact size); unexpected chunk types unreachable!-panic (documented test vocabulary, L19-20) |
| WalkedChunk / DecodedPng | 254-279 | REAL | — | plain data carriers |
| FakeMonotonic | 411-417 | REAL | — | AtomicU64 Relaxed load — fine for single-test-owner clock |
| VirtualClock::new/clock/set_ns/advance_ns | 427-454 | REAL | — | fetch_max makes set_ns monotonic-safe (documented); 4 use sites in presenter_seam |
| synthetic_request | 464-476 | REAL | — | thin builder over crate synthetic |
| expected_pattern | 481-483 | REAL | — | delegates to the crate's pure generator |
| present | 486-488 | REAL | — | asserts Ok — a present failure fails the suite |

## 5. Test honesty
No tests inside this module (it IS the test machinery); its own correctness is pinned by the consumer suites: png_hash runs `assert_sha256_vectors` inside its hash tests, and `the_committed_hash_pins_a_verified_image` proves decode_png accepts the crate's real encoder output. Mutation reasoning: a wrong hasher fails the vector asserts before any committed-hash comparison; a permissive decode_png fails the CRC/Adler/filter asserts.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "vector-pinned, additionally cross-checked against the system sha256sum at recording time" | common/mod.rs:5-7 | REAL | 6 vectors re-derived with python hashlib — exact match |
| "a committed PNG hash is proven to pin a *verified image*" | common/mod.rs:8-12 | REAL | decode_png verifies CRC-32 + Adler-32 + filter bytes before png_hash hashes; suite passes |
| "exact nanoseconds, zero sleeps" | common/mod.rs:13-15 | REAL | VirtualClock is pure AtomicU64 arithmetic; no sleep in module |

## 7. Dead / duplicated / unreachable code
None significant (usage census: every public item consumed by one or both suites). The `unreachable!` at L345 is one of the tree's 4126, justified here as documented test vocabulary.

## 8. Dependencies & flags
Test-only deps: media-clock, media-types, renderer-wgpu (synthetic/params), testkit, video-engine. No unsafe. `#![allow(dead_code)]` (F-002).

## 9. Open questions for the next agent
- png_hash.rs (audited next) inherits this module's trust chain; the committed-hash constant itself is audited there.
