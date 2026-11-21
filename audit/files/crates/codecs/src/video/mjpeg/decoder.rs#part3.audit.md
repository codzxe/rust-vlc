# AUDIT: crates/codecs/src/video/mjpeg/decoder.rs#part3
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-21 |
| Lines / functions | 1300–1948 of 1949 (chunk part3 of 3; part1 ≈1–650 W06-A05, part2 ≈650–1300 W06-A06) / 16 tests + 2 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | Test module itself: NO (`#[cfg(test)]`). Subject code: YES — `apps/cli/src/pipeline_host.rs:242-245` `"mjpeg" => { let mut decoder = codecs::video::mjpeg::MjpegDecoder::new(); … }` → `crates/codecs/src/video/mjpeg/mod.rs:276` `pub struct MjpegDecoder { … context: decoder::DecodeContext … }`; baseline `mp4-smoke-bench-avi-mjpeg-pcm.txt` `v_frames=60 v_decoded=60` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "^…struct|fn|impl" decoder.rs` (structure map); `rg -n "plane_builds|FRAME_SLOTS|pooled_frame_slot" decoder.rs` (definitions); `rg -n "context_reuse|pooled_slot|…" audit/runs/baseline/test-workspace.log.txt` (all 16 tests present, `ok`); `sed -n` on every quoted range |

## 1. One-paragraph verdict
Part3 is the back half of the `#[cfg(test)] mod tests` (starts 1201): hand-built JPEG construction (`tiny_jpeg`, `with_avi1`) plus 16 unit tests covering context reuse bit-exactness, DHT content-sensitivity, scratch-reset after header damage, concealment non-leakage, pooled-slot stale-byte safety and bounds, the staged (`ScanHooks`) seam, assemble-length contract, AVI1 interlaced metadata, restart-interval MCU-row crossings, and odd-geometry 4:2:0. Every test ran and passed in the baseline workspace run (`test-workspace.log.txt` lines 676–693). The tests are content-sensitive (they assert `assert_ne!` where needed to prove observability, compare against independent stateless references, and assert exact byte counts); nothing here is a tautology. One S4: two production state fields (`plane_builds`, `table_builds`) exist solely to be asserted by tests in this range.

## 2. What actually works (verified)
— All 16 tests executed and passed: `rg -n "video::mjpeg::decoder::tests" audit/runs/baseline/test-workspace.log.txt` → 16 matching `ok` lines (676–693), zero ignored.
— Subject path shipped: baseline `mp4-smoke-bench-avi-mjpeg-pcm.txt` → `v_frames=60 v_decoded=60` through the MJPEG decoder this module tests.

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/video/mjpeg/decoder.rs:1437-1444 (and 1494-1497)
Code:
```
        assert_eq!(
            ctx.plane_builds, 6,
            "plane rebuilds only on geometry change"
        );
```
What is wrong: The test asserts on `DecodeContext.plane_builds` / `table_builds` — two `pub(crate) u32` fields (defined at decoder.rs:400-401, part2's range) that production code only ever writes (`self.plane_builds += 1;` at 771, `self.table_builds += 1;` at 816) and nothing in shipped code reads. They are test-only observability counters carried permanently in production state.
Why it matters: Dead state in every shipped decode; a reader of the struct must check whether they matter (they don't); u32 counters that could in principle wrap after 4 billion geometry changes, silently invalidating the very regression assertion they exist for.
What correct looks like: `#[cfg(test)]`-gated counters, or expose build counts via a test-only accessor/wrapper, keeping the production struct lean.
Confidence: HIGH — Verification: read (`rg -n "plane_builds|table_builds"` → only 400/401/415/416/771/816 definitions+writes plus these test reads).

## 4. Per-function table (part3 range; boundary note: `pixel_cap_rejects_oversize_frames` 1271–1309 straddles the part2/part3 boundary — its assertion block 1303–1308 is in range and audited here; construction is W06-A06's)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pixel_cap_rejects_oversize_frames` (tail) | 1299-1309 | REAL | — | Asserts `MjpegError::LimitExceeded { attempted: 8193*8193 }`; matches `limit_error`/`MAX_PIXELS` (1193-1199, part2). Mutation: cap removed → decode attempts 67 MP allocation, test fails or OOMs — honest. |
| `plane_of` (helper) | 1209-1219 | REAL | — | Correct `div_ceil` block geometry; test-only. (In-range reference; defined just below part boundary.) |
| `tiny_jpeg` (helper) | 1318-1376 | REAL | — | Hand-builds SOI/DQT/DHT×2/SOF0/SOS/entropy/EOI; big-endian lengths correct (`payload.len()+2`); entropy budget `w*h/32+16` ≥ worst case `w*h*3/256` with 8× slack; no fixtures baked into production code. |
| `context_reuse_is_bit_exact_and_reallocates_only_on_change` | 1378-1449 | REAL | — | Cross-checks context decode against independent stateless `decode()` references; sequence exercises gray/420/444 + table swap; asserts meta + full byte equality + build counts (see F-001). |
| `dht_key_compare_is_content_sensitive_without_rebuilding_steady_state` | 1451-1498 | REAL | — | Proves observability first (`assert_ne!` on the two references, 1473) — the mark of an honest differential test; HUFFVAL-content sensitivity covered. |
| `header_error_does_not_poison_the_persistent_scratch` | 1500-1532 | REAL | — | Truncates at a computed SOF offset (120, layout documented 1509-1512), then demands bit-identity with a fresh decode. Mutation: pre-walk reset removed → stale quant/Huffman tables leak → test fails. |
| `concealment_does_not_leak_across_frames` | 1534-1561 | REAL | — | Truncated frame must decode (concealed=true), then clean frame bit-identical to fresh stateless decode. |
| `take_output_hands_off_the_buffer` | 1563-1580 | REAL | — | Verifies move-out + fresh-buffer re-decode equality. |
| `pooled_slot_refill_is_bit_exact_and_stale_byte_safe` | 1584-1663 | REAL | — | The strong one: two frames differing everywhere, warm-slot refill must equal stateless reference exactly; also pool growth (1→2→3) and release-reuse. Matches `pooled_frame_slot` (870) semantics. |
| `pooled_slots_rekey_on_geometry_and_stay_bounded` | 1665-1736 | REAL | — | 12 rounds alternating 16×16/32×32, all retained; asserts `slots.len() <= FRAME_SLOTS` (4) each round and rotation after release. |
| `staged_full_scan_is_bit_exact_vs_the_production_path` | 1738-1784 | REAL | — | Seam-equivalence test (FullStages vs production instantiation) over gray/420/422/truncated. NOTE: it proves A==B, not that either is correct — correctness rests on the corpus/integration tests in `tests/` (module doc 1205-1207); acceptable as a seam license. |
| `assemble_frame_rejects_a_wrong_length_destination` | 1786-1815 | REAL | — | Wrong-length destination must be a typed error both directions; exact length must assemble and equal the stateless reference. |
| `with_avi1` (helper) | 1819-1827 | REAL | — | APP0 length byte 0x0008 = 2+6 payload bytes, spec-correct; inserts after SOI. |
| `avi1_interleaved_frame_is_metadata_only_bit_exact` | 1829-1853 | REAL | — | Asserts `b.data == a.data` (metadata-only) AND polarity surfaces — mutation in either direction fails. |
| `avi1_single_field_decodes_at_field_height` | 1855-1878 | REAL | — | Polarity 0 and 1: field-height picture + polarity metadata; two independent decodes. |
| `restart_boundaries_crossing_mcu_rows_decode_bit_exact` | 1880-1916 | REAL | — | Hand-splices DRI (0xff 0xdd 0x00 0x04 0x00 0x02 — length 4 correct) + RST0/RST1 at both crossing classes; asserts `!concealed` and bit-exactness vs the restart-free stream; the 1896 offset sanity check panics loudly if the splice arithmetic rots. |
| `odd_geometry_420_decodes_without_panic_or_padding_leak` | 1918-1947 | REAL | — | 87×63: asserts exact `w*h + 2*ceil(w/2)*ceil(h/2)` length; the second decode through a dirty context is what actually makes stale bytes observable (comment admits the fresh-allocation case cannot) — honest construction. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 16 in range | YES | Each compares against an independently computed reference (stateless `decode` or exact byte-count formula), or asserts a typed error on constructed invalid input. The only equivalence-only test (`staged_full_scan…`) is explicitly a seam check; seam breakage is what it exists to catch. |
| Run status | All `ok` in `audit/runs/baseline/test-workspace.log.txt:676-693`; none `#[ignore]` (`rg "#\[ignore"` in file → only line 685 is `#[cfg(test)]` on a different helper). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "unit tests here target the geometry, assembly and context-reuse helpers with hand-built inputs" | decoder.rs:1205-1207 | REAL | The 16 tests do exactly that; integration corpus lives in `tests/` (W-other agents). |
| Pooling "bounded at FRAME_SLOTS" | decoder.rs:853-856 (part2) | REAL | `pooled_slots_rekey…` asserts the bound over 12 retained rounds. |

## 7. Dead / duplicated / unreachable code
- Test module is `#[cfg(test)]` (1201) — unreachable from shipped binaries by construction, which is correct for tests.
- `plane_builds`/`table_builds` production fields: write-only in production (see F-001).

## 8. Dependencies & flags
No feature flags gate this module: `crates/codecs/Cargo.toml` `[features]` has no mjpeg gate — `pub mod mjpeg` is unconditional (`crates/codecs/src/video/mod.rs:38-39`). Dev-deps (`input`, `demux`, `plugin-api`, `testkit`) are not used by this in-file test module (it is self-contained hand-built input); they serve the corpus suite in `tests/`.

## 9. Open questions for the next agent
- W06-A06 (part2): confirm `MAX_PIXELS` = 8 MiP and that `limit_error` (1193) is the only rejection site the 8193×8193 test can hit.
- Corpus/integration agent: do the `tests/` corpus JPEGs cover restart-marker *misnumbering* (concealment path), or only well-formed RST sequences? In-file tests cover only the well-formed case.
