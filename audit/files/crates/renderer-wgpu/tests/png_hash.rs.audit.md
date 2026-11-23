# AUDIT: crates/renderer-wgpu/tests/png_hash.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| Lines / functions | 261 / 4 helpers + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A test suite — exercises the crate's shipped public path (`Renderer::new_headless` → present → readback → `screenshot::encode_rgba`); renderer-wgpu is the shipped software compositor (engine-info feature line) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `./target/debug/deps/png_hash-cee1d8862e16caad` → 5 passed / 0 failed (exit 0); rg census of synthetic.rs (4 in-crate pattern tests + documented per-pixel spec) |

## 1. One-paragraph verdict
This is what an acceptance-proof suite should look like. The committed SHA-256 constant is not observed-and-blessed: the test forces a derivation chain — vector-pinned hasher (asserted in-test, and I independently reproduced all six vectors with python hashlib in the common/mod.rs audit), renderer value-transparency (`readback == expected_pattern`), measured staging (exactly one copy of exactly 12288 bytes), structural PNG verification (chunk census IHDR/sRGB/IDAT/IEND in order, per-chunk CRCs, Adler-32, filter-0 rows), defiltered-pixels equality with the pure generator, and a non-vacuity proof (one flipped pixel and a different extent each change the hash). The generator itself is unit-pinned in synthetic.rs (4 tests, per-pixel spec documented). All 5 tests pass under my execution.

## 2. What actually works (verified)
— verified by running the prebuilt suite → "5 passed; 0 failed" (ac hash, double-run determinism, verified-image pinning, content+geometry sensitivity, BGRA/RGBA swizzle involutivity). Trust-chain dependencies verified separately: SHA-256 vectors reproduced with python hashlib (common/mod.rs audit); synthetic pattern's 4 unit tests exist (rg census).

## 3. Findings
None. What was checked and how: (a) tautology risk — the hash constant is cross-checked at two independent layers (readback-vs-generator equality, defiltered-PNG-vs-generator equality) before the hash compare, so a consistently-broken renderer cannot pass; (b) sensitivity — test 4 proves the hash is not vacuous (content and geometry mutations both change it); (c) swizzle — test 5 proves byte order is transport (both sRGB orders hash identically); (d) the provenance doc's one-time out-of-tree Python oracle is honestly labeled as a recording-time step, not an in-test guarantee — acceptable since the in-test chain re-derives the constant from the generator; (e) no ignored tests, no cfg gates, no mocks of the system under test (the renderer is real, headless).

## 4. Per-function table (file < 100 LOC of logic; table for completeness)
| Item | Lines | Verdict | Notes |
|---|---|---|---|
| AC_EXTENT / EXPECTED_PNG_SHA256 | 49-73 | REAL | constant carries full provenance note (sha256sum cross-check, zlib oracle, drift contract) |
| ac_renderer / render_png | 77-96 | REAL | the exact shipped public path |
| ac_synthetic_frame_headless_render_produces_the_expected_png_hash | 102-132 | REAL | backend assert "software" + value truth + staging measurement + hash + sRGB intent |
| double_run_renders_are_byte_identical | 138-161 | REAL | fresh-renderer and re-present determinism |
| the_committed_hash_pins_a_verified_image | 167-190 | REAL | structural decode + chunk census + pixel equality |
| the_hash_is_sensitive_to_content_and_geometry | 196-229 | REAL | non-vacuity |
| both_srgb_surface_orders_read_back_the_same_image | 235-261 | REAL | swizzle involutivity + same measured transfer |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 5 | YES | each asserts concrete equality against independently pinned values (generator bytes, chunk census, vector-hashed digests); mutation of swizzle/placement/encoder/hash/constant each trips a specific assert (traced per layer) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the composite constant is *derived*, not observed-and-blessed" | png_hash.rs:11-14 | REAL | the chain is enforced in-test (see §1); generator unit-pinned (synthetic.rs 4 tests) |
| "any drift in generator, placement, swizzle, encoder or hasher fails this suite" | png_hash.rs:33-34 | REAL | mutation analysis per layer confirms a specific assert fires |
| sha256sum agreement `1466e74a…eb30bf` at recording time | png_hash.rs:61-62 | REAL-UNPROVEN (recording-time step) | not repeatable in-test; in-test chain compensates |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
renderer-wgpu public API + common/mod.rs machinery (audited: REAL) + testkit. No feature gates, no ignores.

## 9. Open questions for the next agent
None.
