# AUDIT: crates/media-probe/fixtures/annexb_probe_hevc.265 (binary asset)
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-13 |
| Type / size / sha256 | `file`: "data" (generic — no HEVC magic in libmagic's table; structure verified by parse below); 3,792 B; f73b30c41fd5c403b18d9adcdea820edc88b9219564bd01ffcd6155fe5f1b4de |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture; consumed by crates/media-probe/tests/registry_roundtrip.rs:198 (every_landed_container_probes_to_its_format_id) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `sha256sum`; python3 Annex-B NAL parse (29 NALs); `cargo test -p media-probe --test registry_roundtrip` → 3 passed (exit 0) |

## 1. One-paragraph verdict
Genuine HEVC Annex-B elementary stream whose structure I re-parsed independently:
29 NAL units, led by VPS (hdr 40 01, type 32) → SPS (42 01, type 33) → PPS (44 01,
type 34) → prefix SEI (4e 01, type 39) → IDR_W_RADL (28 01, type 20) → TRAIL_R
slices — exactly the "VPS-led HEVC … decisive parameter-set shape" the consuming
test claims (registry_roundtrip.rs:170-174). The registry round-trip that probes it
(with the "265" extension hint) to format id "hevc" at confidence ≥ 60 passes
(3/3 tests, run by me). Note `file(1)` calls it plain "data" — expected, libmagic
lacks an HEVC-Annex-B recognizer; the byte-level parse above is the verification.

## 2. What actually works (verified)
- NAL census (python3): NAL0 VPS type 32 @0 (4-byte code), NAL1 SPS type 33 @27,
  NAL2 PPS type 34 @74, NAL3 SEI type 39, NAL4 IDR_W_RADL type 20, then type-0/1
  TRAIL slices; 29 NALs; 3- and 4-byte start codes both present.
- `cargo test -p media-probe --test registry_roundtrip` → 3 passed, including the
  row citing this fixture — executed here; matches baseline
  (cargo-test-workspace.txt:3691-3695).

## 3. Findings
None. Checked: structure (VPS-led Annex-B order), size (3,792 B matches
manifest.tsv:650), sha256 stability, sole consumer (registry_roundtrip.rs:198 —
repo-wide rg), provenance (comment: "committed ffmpeg-authored fixtures",
registry_roundtrip.rs:172-173; authoring command not in-repo — UNVERIFIED, bytes
self-authenticate as valid HEVC Annex-B per my parse).

## 4-9. Per-function table / test honesty / doc claims / dead code / deps / open questions
N/A (binary asset). Test honesty note: the consuming row asserts concrete format id
"hevc" and ≥60 confidence — fails on fixture swap or signature regression.
Cross-reference: audit/files/crates/media-probe/src/signature/es.rs.audit.md cites
this fixture's first-24-bytes in the annexb signature's hevc lane.
