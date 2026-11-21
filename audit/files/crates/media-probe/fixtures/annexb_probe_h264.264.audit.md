# AUDIT: crates/media-probe/fixtures/annexb_probe_h264.264 (binary fixture)
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-21 |
| Size / kind | 2,393 B · raw H.264 Annex-B elementary stream (x264-encoded) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture (crate-relative read, `registry_roundtrip.rs:192`). The `AnnexbSignature` it exercises IS shipped-reachable (with_builtins → `lumen open` probe path; the ES container host dispatches `h264` at containers/mod.rs:64). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `od -A d -t x1` (SPS `00 00 00 01 67 …`, PPS `…68 …`, SEI `…06 …` with the x264 string); python3 NAL parse (26 NALs: 7/8/6 then type-1 slices; SEI string at offset 62); `grep -n annexb_probe tests/registry_roundtrip.rs` (:192 row, expected `h264`, extension `264`); discrimination logic re-read at `signature/es.rs:216-265`; `git log --diff-filter=A` (commit 061a8b5d, 6.OPW-es); baseline `orchrun-test-workspace-nofailfast.log.txt:3427` (round-trip ok) |

## 1. One-paragraph verdict
Genuine x264 product with embedded provenance — the SEI at offset 62 carries `x264 - core 164 r3108 31e19f9 - H.264/MPEG-4 AVC codec - Copyleft 2003-2023` — and its NAL layout drives the *other* decisive arm of the Annex-B discrimination, complementing its HEVC sibling: the first NAL byte `0x67` is SPS under H.264 (type 7 → `h264_param = true`, alive) and fatal under HEVC (`0x67 >> 1 = 51 > 40` → `hevc_alive = false`), so the registry round-trip proves the `(h264_param, hevc-dead)` → `FormatId::h264()` path on real encoder bytes; it ran green in the baseline. The one defect is the same as the sibling's: no SHA pin and no recorded generation command — the round-trip's format-id assertion is a content check, but any HEVC-looking replacement that still probes `h264` would pass unnoticed.

## 2. What actually works (verified)
— NAL structure (python3 parse of the actual bytes): SPS (7) at start code 0, PPS (8) at 29, SEI (6) at 36, then 23 type-1 (non-IDR) VCL NALs — 26 NALs in 2,393 B; header bytes `67 f4`, `68 eb`, `06 05` od-verified; emulation-prevention sequences present (`00 00 03`).
— Discrimination path verified by hand against `signature/es.rs:216-265`: `0x67` → h264_type 7 (param, alive) / hevc_type 51 (> 40 → dead) → decisive arm `(true, false) if h264_param` mints `h264`.
— Executed, passing: `every_landed_container_probes_to_its_format_id ... ok` (baseline :3427) — this fixture's row asserts id `h264` + `confidence ≥ 60` through the full builtin registry (the same registry `lumen open` constructs at demux_host.rs:444).
— Provenance: commit 061a8b5d (card 6.OPW-es) + the self-identifying x264 core-164 SEI string.

## 3. Findings
### F-001 [S4]
Where: crates/media-probe/tests/registry_roundtrip.rs:215-236 (assertion set), fixture at crates/media-probe/fixtures/annexb_probe_h264.264
Code:
```rust
        let report = registry.probe(&bytes, &meta, &config).must();
        assert_eq!(
            report.format.as_str(),
            row.expected,
            "{}: wrong format id",
            path.display()
        );
```
What is wrong: no SHA-256 pin, no committed generation command. The assertion set (non-empty, format id, confidence ≥ 60) fails only if the bytes stop probing as `h264` — a different x264 file, a tail truncation, or a leading-bytes-preserving edit passes silently. The embedded SEI provenance string is asserted by nothing.
Why it matters: identical gap to the HEVC sibling (my file-12 audit F-001) and the same inconsistency with the demux crate's ASF family (per-file SHA pins, asf/tests.rs:1484-1499). This fixture and its sibling are the only committed real-codec Annex-B inputs the probe suite has.
What correct looks like: per-fixture SHA pins in the round-trip table + a generation command in a fixtures README (one fix covers both annexb fixtures).
Confidence: HIGH — Verification: read, parse, exhaustive grep (no pin exists).

## 4. Per-function table
Not applicable (binary asset). Consumer: the round-trip row (:192) via `window()` (leading 4096 B — the whole 2,393 B file).

## 5. Test honesty
| Test consuming this fixture | Would it fail if the fixture were broken? | Reason |
|---|---|---|
| every_landed_container_probes_to_its_format_id | PARTIALLY | Fails on missing/empty or loss of the `h264` verdict; passes through any byte change that keeps an SPS-led H.264 shape (F-001). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| Annex-B codec discrimination: h264 vs hevc by NAL-header shape with param-set proof | signature/es.rs:208-265 | REAL | Both decisive arms proven on real encoder bytes (this file + sibling); baseline :3427 ok. |
| `h264` probe id for `.264` raw streams | registry row :192-195 | REAL | Round-trip executed ok. |
| es.rs unit tests (`h264_start_codes_probe_h264`, `ambiguous_nal_shapes_name_the_family_sub_threshold`) use synthetic bytes | signature/es.rs tests | REAL (cross-ref) | The committed real-codec coverage is exactly these two fixture files — the synthetic tests pin the edge shapes, these pin the common real shape. |

## 7. Dead / duplicated / unreachable code
Not applicable. Single consumer; no orphans.

## 8. Dependencies & flags
None (test data). Same crate-relative fixture convention as the sibling.

## 9. Open questions for the next agent
- This stream's first VCL NAL is type 1 (non-IDR) — no IDR (type 5) anywhere in the parsed list: the fixture is a mid-stream-shaped sample for probe purposes. If a later lane needs IDR-present Annex-B bytes (e.g., the demux es/annexb reanchor path), this fixture must not be reused blindly.
- The `1..=15` H.264 type filter in es.rs (noted in my file-12 audit §9) rejects NAL 16..=31 — real encoders that emit NAL 19/20 would be misclassified as "not H.264"; this fixture (x264's common output: 1/5/6/7/8/9 only) cannot catch that. The es.rs auditor owns the risk assessment.
