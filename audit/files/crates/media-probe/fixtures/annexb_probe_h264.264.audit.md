# AUDIT: crates/media-probe/fixtures/annexb_probe_h264.264 (binary asset)
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-13 |
| Type / size / sha256 | `file`: "JVT NAL sequence, H.264 video @ L 10"; 2,393 B; 7ba2558002ac914505d51d22f93ceea770ed7a1938423bbc64d1384656c31094 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture; consumed by crates/media-probe/tests/registry_roundtrip.rs:192 (every_landed_container_probes_to_its_format_id) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `sha256sum`; python3 Annex-B NAL parse (28 NALs); `cargo test -p media-probe --test registry_roundtrip` → 3 passed (exit 0) |

## 1. One-paragraph verdict
Genuine ffmpeg-authored H.264 Annex-B elementary stream whose byte structure I
re-parsed independently: 28 NAL units, led by SPS (hdr 0x67, type 7) → PPS (0x68)
→ SEI → IDR slice (0x65) → non-IDR slices — exactly the "SPS-led … decisive
parameter-set shape" the consuming test's comment claims (registry_roundtrip.rs:
170-174). The fixture does its job by execution: the registry round-trip that
probes it (with the "264" extension hint) to format id "h264" at confidence ≥ 60
passes (3/3 tests, run by me).

## 2. What actually works (verified)
- NAL census (python3): NAL0 SPS type 7 @0 (4-byte code), NAL1 PPS type 8 @28,
  NAL2 SEI type 6, NAL3 IDR type 5, then type-1 slices; 28 NALs; both 3- and
  4-byte start codes present — a realistic stream, not a synthetic minimum.
- `cargo test -p media-probe --test registry_roundtrip` → 3 passed, including
  every_landed_container_probes_to_its_format_id (the row citing this fixture) —
  executed in this sandbox, matching the baseline (cargo-test-workspace.txt:
  3691-3695).

## 3. Findings
None. Checked: magic/structure (Annex-B start codes + SPS-led NAL order), size
(2,393 B matches manifest.tsv:649), sha256 stability, sole consumer
(registry_roundtrip.rs:192 — repo-wide rg), provenance (comment: "committed
ffmpeg-authored fixtures", registry_roundtrip.rs:172-173; no generator script in
media-probe — UNVERIFIED how it was authored, but bytes self-authenticate as a
valid H.264 Annex-B stream per `file` + my parse).

## 4-9. Per-function table / test honesty / doc claims / dead code / deps / open questions
N/A (binary asset). Test honesty note: the consuming row asserts a concrete format
id and a ≥60 confidence — it would fail if the fixture were swapped or the annexb
signature regressed. Cross-reference: audit/files/crates/media-probe/src/
signature/es.rs.audit.md (prior agent) already cites this fixture as the h264
lane's evidence.
