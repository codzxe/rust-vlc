# AUDIT: crates/media-probe/fixtures/annexb_probe_hevc.265 (binary fixture)
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-21 |
| Size / kind | 3,792 B · raw HEVC Annex-B elementary stream (x265-encoded) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture (crate-relative read at test time, `registry_roundtrip.rs:198` + `CARGO_MANIFEST_DIR`). The `AnnexbSignature` it exercises IS shipped-reachable (`with_builtins` → `lumen open` probe path). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `od -A d -t x1` (start codes + NAL headers + embedded x265 SEI string); python3 NAL parse (27 NALs: VPS/SPS/PPS/SEI/IDR/TRAIL enumerated); `grep -n annexb_probe tests/registry_roundtrip.rs` (:198 row, expected `hevc`, extension `265`); `sed -n '206,265p' crates/media-probe/src/signature/es.rs` (the discrimination the fixture drives); `git log --diff-filter=A` (commit 061a8b5d, card 6.OPW-es); baseline `orchrun-test-workspace-nofailfast.log.txt:3427` (`every_landed_container_probes_to_its_format_id ... ok`) |

## 1. One-paragraph verdict
A genuine, structurally-verified x265 product whose bytes carry their own provenance: the SEI at offset ~120 embeds the encoder string `x265 (build 215) - 4.1+1-1d117be:[Linux][GCC 14.2.0][64 bit] 8bit+10bit+12bit`. A hand-parse of the NAL structure confirms exactly the shape the `AnnexbSignature` needs for a decisive HEVC verdict: 4-byte start code + VPS (`0x40` → type 32), SPS (`0x42` → 33), PPS (`0x44` → 34), SEI, IDR_W_RADL (`0x4E` → 39), then TRAIL slices — 27 NALs total, with emulation-prevention bytes present; under the H.264 hypothesis the very first NAL byte (`0x40 & 0x1F = 0`) kills `h264_alive`, so the fixture drives the codec-discrimination branch `(false, true) with hevc_param` end-to-end through the full builtin registry, and that round-trip ran green in the baseline. The single defect is integrity discipline: no SHA pin (the round-trip's format-id + confidence assertions are a content check, but a mutation that keeps the leading NAL shapes would pass unnoticed).

## 2. What actually works (verified)
— NAL structure (python3 parse of the actual bytes): start codes at 0/28/75/… with NAL types [32 (VPS), 33 (SPS), 34 (PPS), then VCL/SEI NALs] — 27 start-code NALs in 3,792 B; the header bytes `00 00 00 01 40 01`, `…42 01`, `…44 01` are od-verified.
— Discrimination path verified by hand against `signature/es.rs:216-265`: first NAL byte `0x40` → h264_type 0 (not 1..=15) → `h264_alive = false`; hevc_type 32 → `hevc_param = true`; IDR `0x4E` → type 39 ≤ 40 alive → decisive `(true-false-h264, hevc_param)` arm mints `FormatId::hevc()`.
— Executed, passing: `media_probe::tests::registry_roundtrip::every_landed_container_probes_to_its_format_id ... ok` (baseline :3427) — this fixture's row asserts format id `hevc` AND `confidence ≥ 60` through `ProbeRegistry::with_builtins()` (the registry the shipped CLI constructs).
— Provenance: commit 061a8b5d (card 6.OPW-es, raw-ES signatures) + the embedded x265 build string (stronger than a manifest: the bytes self-identify).

## 3. Findings
### F-001 [S4]
Where: crates/media-probe/tests/registry_roundtrip.rs:215-236 (the assertion set over this fixture), fixture at crates/media-probe/fixtures/annexb_probe_hevc.265
Code:
```rust
        let bytes = window(&path);
        assert!(!bytes.is_empty(), "{}: empty probe window", path.display());
        let meta = SourceMeta::new(None, Some(row.extension));
        let report = registry.probe(&bytes, &meta, &config).must();
        assert_eq!(
            report.format.as_str(),
            row.expected,
```
What is wrong: the fixture has no SHA-256 pin and no recorded generation command. The round-trip's assertions (non-empty, format id, confidence) are a real content check — a mutation that breaks the leading NAL shapes fails the row — but any edit that preserves "looks like HEVC with a VPS" (e.g., re-encoding, truncating the tail, or swapping in a different x265 file) passes identically. Provenance currently rests entirely on the embedded SEI string, which nothing asserts.
Why it matters: the demux-crate ASF family pins every fixture's SHA; this probe-crate family (same repo, same era) pins none. The fixture is the only committed real-codec HEVC input the probe suite has (es.rs's unit tests use synthetic bytes), so silent replacement would quietly change what "HEVC probes hevc" is proven with.
What correct looks like: an `expected_sha` pin per crate fixture (the asf/tests.rs:1484 pattern) in the round-trip table, plus a one-line generation command in a fixtures README.
Confidence: HIGH — Verification: read (test body), NAL parse, git log; no pin found by exhaustive grep of the crate.

## 4. Per-function table
Not applicable (binary asset). Consumers: `every_landed_container_probes_to_its_format_id` row (:198) via the `window()` helper (leading 4096 B — the whole 3,792 B file).

## 5. Test honesty
| Test consuming this fixture | Would it fail if the fixture were broken? | Reason |
|---|---|---|
| every_landed_container_probes_to_its_format_id | PARTIALLY | Fails if the file is missing/empty or stops probing as `hevc` ≥ 60 through the full registry; passes through any byte change that keeps a HEVC-looking prefix (F-001). The row also feeds `extension_lists_cover_their_fixtures`-adjacent coverage by naming extension `265`. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| Annex-B discrimination: "start-code anchored with whole-window NAL-header-shape discrimination" | signature/es.rs:208-210 | REAL | Hand-parse above; the fixture drives the decisive arm; baseline :3427 ok. |
| "hevc" probe id for `.265` raw streams | registry row (:198-201) + es.rs | REAL | Round-trip executed ok (baseline :3427). |
| Registry ids incl. `h264 hevc` in the CLI's no-demuxer error list | apps/cli/src/containers/mod.rs:106 | REAL (cross-ref) | with_builtins registers AnnexbSignature (signature/mod.rs:142); the containers dispatch has `hevc`/`h264` arms (:64-65) — the ES lane is the one raw family fully wired end-to-end. |

## 7. Dead / duplicated / unreachable code
Not applicable. Single consumer, no orphans (`grep annexb_probe` over the crate returns the one row + sibling).

## 8. Dependencies & flags
None (test data). Located in `crates/media-probe/fixtures/` with crate-relative resolution — consistent with the crate's `testkit` fixture conventions; no feature gates involved.

## 9. Open questions for the next agent
- The es.rs signature's H.264 type filter (`1..=15`) rejects NAL types 16..=31 — real-world H.264 with NAL 19/20 (e.g., some SVC/edge-encoder output) would flip `h264_alive` off; the h264 sibling fixture (my next file) only proves the common x264 shapes. Whoever audits `signature/es.rs` should weigh that against the "both survive → sub-threshold annexb" fallback.
- If the fixtures-wave adds SHA pins (F-001), the two annexb probe fixtures and the codecs-crate fixtures they sit beside (`../codecs/tests/fixtures/…` rows) should be pinned together.
