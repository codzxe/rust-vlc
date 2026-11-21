# AUDIT: crates/demux/src/asf/fixtures/asf_stream_broadcast.wmv
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 10,467-byte binary / n/a (test asset) |
| File verdict | REAL (test asset: committed, refusal lane, byte-reproducible from the in-repo command) — with one dead integrity pin (F-001) |
| Reachable from shipped entry point? | N/A (test asset). Consumed by `broadcast_fixture_refuses_typed_where_ffprobe_refuses` (asf/tests.rs:1428-1481) — executed ok in baseline (log 1566). Consuming module UNREACHABLE from shipped entry points (guid.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 (+ family findings cross-referenced, filed in the asf_mjpeg.asf audit, not recounted) |
| Commands run | sha256sum (pin value matches table, but the assert is dead — F-001) · ffmpeg 7.1.5 regeneration from the tests.rs:1435-1443 command incl. `-f asf_stream` (byte-identical) · python3 audit/analysis/asf_parse.py (prefix + FPO + refusal-shape walk) · ffprobe -v error -f asf (exit=1 "Invalid data found" ✓ matches the test's oracle) · od -A d -t x1 -N 16 (prefix bytes) · rg `expected_sha\|sha256_16` (callers) |

## 1. One-paragraph verdict
The family's refusal fixture, and its best-designed member behaviorally: a genuine ffmpeg `asf_stream`-muxed file whose first 12 bytes are the MMS `$H`-family prefix (`24 48 2f 03 00 00 00 00 00 0c 2f 03` — od-verified) with the ASF Header GUID at +12, an FPO whose flags DWORD is 0x03 (broadcast bit set — od/walk-verified), zero declared file size and packet count, and ~9.6 KB of non-object-framed live-stream `$`-packets after a 50-byte empty Data Object. The test drives the demuxer's real recognition-then-refusal chain (header.rs:478-485 prefix locate → parse_at at +12 → FPO broadcast gate) and cross-checks against ffprobe's own refusal — which I reproduced live (exit=1). The bytes regenerate byte-exactly from the in-repo command under ffmpeg 7.1.5. One real defect: the fixture's sha pin exists in `expected_sha` (tests.rs:1496) but is DEAD — the only call site (tests.rs:1138) iterates `FIXTURES`, which does not include this file, so this is the one family member whose committed bytes are never integrity-asserted (F-001).

## 2. What actually works (verified)
- sha256 `a9db9175da47fe6cb03e51723afea2da2b7796c4f75b99e31d9ec42113171d7c` == the table value `a9db9175da47fe6c` (tests.rs:1496) — but see F-001: no test asserts it.
- Regeneration: the exact command at tests.rs:1435-1443 (`… -c:v wmv2 -b:v 100k -c:a wmav2 -b:a 32k -f asf_stream`) under ffmpeg 7.1.5 → identical sha256 (audit/analysis/gen/). The committed bytes ARE ffmpeg 7.1.5 asf_stream output.
- Structure (independent walk): prefix 12 B then HEADER at +12 (size 757, 6 children: FPO at 42, Header Extension 156, ECD 98, SP video 133 [WMV2, extradata 4], SP audio 114 [wmav2 WFX 0x0161/1/8000/256/16/10], Codec List 122) · FPO file_size 0, data_packets 0, preroll 3100, min=max=3200, **flags 0x03: broadcast=1, seekable=1** (matches header.rs:137-139's "flags 3 vs the normal files' 2") · Data Object at 769 size 50 (empty: count 0, reserved `01 01`) · after it, non-object-framed `$`-packet bytes (my object walker breaks there at 819 — by design of the stream format; the demuxer never walks past the broadcast gate).
- Oracle: `ffprobe -v error -f asf` refuses, exit=1, "Invalid data found when processing input" — matches the test's `!probe.status.success()` assert (tests.rs:1462-1465) and the plan §2.5 claim ("ffprobe itself refuses the broadcast form").
- Probe-miss: bytes[..16] is `24 48 2f 03 … 30 26 b2 75` — the Header GUID NOT at offset 0 → `probe` returns false (mod.rs:96-97), asserted at tests.rs:1469 and mirrored by the synthetic test at tests.rs:87-89.
- Baseline: the refusal test ok (log 1566).

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/asf/tests.rs:1496 (dead pin arm) and 1428-1481 (the consuming test, no sha assert)
Code:
```
        "asf_stream_broadcast.wmv" => "a9db9175da47fe6c",
```
What is wrong: `expected_sha` is called from exactly one place — tests.rs:1138, inside the loop over `FIXTURES` — and `asf_stream_broadcast.wmv` is not a `FIXTURES` member (the array at tests.rs:918-1065 has ten entries; this file is the eleventh committed asset, handled by its own test at 1428-1481, which asserts behavior but never the sha). The pin arm at tests.rs:1496 is dead code, and this is the only family member whose committed bytes face no integrity assert anywhere (also outside the xtask fixtures gate — 55 files under tests/fixtures only).
Why it matters: The module doc (tests.rs:1067-1071) claims "The §4.4 sha256 pins now pin the COMMITTED bytes" — for this file that is untrue: byte drift (regeneration under a different ffmpeg, truncation, editor damage) would be invisible as long as the file still carries the 12-byte prefix, a broadcast-flagged FPO, and is refused by ffprobe. The behavioral asserts are strong but are shape-level, not byte-level.
What correct looks like: Add the sha assert to `broadcast_fixture_refuses_typed_where_ffprobe_refuses` (one line: `assert_eq!(sha256_16(&bytes), expected_sha("asf_stream_broadcast.wmv"));`) or extend the pin loop to an 11th entry.
Confidence: HIGH — Verification: rg of `expected_sha`/`sha256_16` call sites (tests.rs:1137-1138 only); FIXTURES membership read; the broadcast test body read end-to-end.

Cross-references (filed and counted in the asf_mjpeg.asf audit): F-001 [S3] guid.rs EC constants (this file's two SP EC fields carry the real GUIDs, refuting both constants); F-002 [S4] xtask gate absence (compounded for this file by the dead pin, above); F-003 [S4] in-src regeneration path.

## 4. Per-function table (asset checks)
| Check | Result | Notes |
|---|---|---|
| Exists / size sane | YES / 10,467 B | == plan §4.4 row |
| Structure matches the refusal lane | YES | 12-byte MMS prefix + Header at +12 + FPO flags 0x03 (broadcast) — od/walk-verified |
| sha256 pin | DEAD | Table value correct but never asserted (F-001) |
| Recipe reproducibility | BYTE-EXACT | ffmpeg 7.1.5 + tests.rs:1435-1443 → identical sha |
| Referenced by code/docs | YES | tests.rs:1433 (path), :1496 (dead pin); plan §1.2:67, §4.4:461 |
| Determinism / drift gate | NONE | Neither the (dead) pin nor the xtask gate covers it — the only unenforced family member |
| Hostile value | REAL (refusal lane) | Drives the typed `demux.asf.broadcast-unsupported` refusal + honest probe-miss, cross-checked against ffprobe's own refusal |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `broadcast_fixture_refuses_typed_where_ffprobe_refuses` (tests.rs:1428-1481) | YES | Three independent gates: (1) live ffprobe must FAIL on the file (oracle refusal, :1462-1465 — reproduced by hand, exit=1); (2) the content probe must miss (:1469 — prefix verified in bytes); (3) our demuxer must refuse with exactly `demux.asf.broadcast-unsupported` (:1480) — if `is_broadcast()` misread the flags bit or the prefix-locate at header.rs:478-485 regressed, the assert fails (unreachable!("must refuse") or code mismatch). Mutation: flip the broadcast bit test to `flags & 0x2` and (3) fails. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "asf_stream_broadcast.wmv 10,467 a9db9175da47fe6c broadcast refusal lane" | plan §4.4:461 | REAL | sha256sum + size |
| "The §4.4 sha256 pins now pin the COMMITTED bytes" | tests.rs:1070-1071 | FALSE (for this file) | F-001: the broadcast pin is never asserted |
| "ffprobe itself refuses the broadcast form (§2.5 … re-measured here)" | tests.rs:1454-1456 | REAL | Hand re-run: exit=1, "Invalid data found when processing input" |
| "a broadcast file (FPO flags bit 0 …) refuses typed `demux.asf.broadcast-unsupported`" | mod.rs:46-48 | REAL | FPO flags 0x03 in bytes; refusal test ok in baseline (log 1566) |

## 7. Dead / duplicated / unreachable code
The pin arm at tests.rs:1496 is dead (F-001). The asset itself is an alive test input; consumer module dead in the shipped product (guid.rs audit F-001).

## 8. Dependencies & flags
None (data file). cfg(test) consumption only.

## 9. Open questions for the next agent
- If the pin assert is added per F-001's fix, note the recipe is duplicated (tests.rs:1435-1443 inline vs the FIXTURES-table convention) — folding this file into the table (with a refusal-specific truth entry) would fix both at once.
