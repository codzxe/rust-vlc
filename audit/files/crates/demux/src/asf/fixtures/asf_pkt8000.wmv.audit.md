# AUDIT: crates/demux/src/asf/fixtures/asf_pkt8000.wmv (binary asset)
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-02-14 |
| Type / size | `file` -> "Microsoft ASF"; 16,899 bytes |
| File verdict | REAL (valid, byte-pinned, differentially exercised fixture) |
| Reachable from shipped entry point? | N/A (test asset) — consumed by the asf differential suite (crates/demux/src/asf/tests.rs:1021 lane, :1493 sha pin) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `wc -c` -> 16899; `ffprobe` -> asf, 1.128 s, wmv2 video + wmav2 audio; python3 sha256 -> `e9ef0bc063ccc02f…` == the committed pin; baseline asf::tests -> 36 passed |

NOTE ON THE ASSIGNED PATH: the assignment lists
`crates/demux/fixtures/asf/asf_pkt8000.wmv`; that path does not exist —
`crates/demux/fixtures/` contains only `adts/` and `ps/`. The file lives at
`crates/demux/src/asf/fixtures/asf_pkt8000.wmv` (committed in-tree by card
ASF-d; the tests.rs comment at :1071-1076 explains why: ffmpeg 6 vs 7 emit
different lossy bytes, so the family is committed for byte-stability). This
audit covers the real file.

## 1. One-paragraph verdict
A genuine ASF/WMV fixture with a purpose: it is the `-packet_size 8000`
variant of the wmv2+wmav2 lane (its FixtureTruth row at asf/tests.rs:1021
carries the exact ffmpeg command), committed in-tree and pinned three ways —
declared file size 16,899 (matches `wc -c` exactly), a sha256 prefix pin
`e9ef0bc063ccc02f` that I reproduced independently with python3, and a full
structural truth row (header 757 B, data object 16,050 B / 2 packets, simple
index at 16,807, wfx tag 0x0161 / 8 kHz / mono / 16-bit, 64x48 WMV2). ffprobe
confirms the container (asf, 1.128 s, wmv2 + wmav2 — exactly the lane's
`ffprobe_codecs`). No defect in the asset.

## 2. What actually works (verified)
— verified by: `ffprobe -show_entries format,stream` -> "Microsoft ASF",
wmv2 + wmav2, duration 1.128000 (exit 0); `python3 hashlib.sha256` first 16
hex == the pin at tests.rs:1493; file size == the lane's `fpo` figure
16,899; baseline cargo-test-workspace.txt shows 36 asf::tests passed
(differential over this family).

## 3. Findings
None for the asset. Checked: type, size (pinned), sha256 (pinned, verified),
stream composition vs the truth row (matches), citation surface
(asf/tests.rs:1021, 1493 — both live), generation provenance (the lane
records the exact ffmpeg invocation).

## 4-9. (short binary-asset audit; sections 4-9 not applicable)
Note for the next agent: the lane's `simple_index: Some((16_807, 92))` and
`data: (757, 16_050, 2)` figures are asserted by the differential — audit
the assertions' semantics when asf/tests.rs is assigned; the fixture itself
makes no claims.
