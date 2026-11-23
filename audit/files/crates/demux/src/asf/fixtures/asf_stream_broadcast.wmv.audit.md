# AUDIT: crates/demux/src/asf/fixtures/asf_stream_broadcast.wmv (binary asset)
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-02-14 |
| Size / type | 10 467 bytes; `file` → "data" (correct: the 12-byte MMS `$H` prefix defeats file(1)'s offset-0 ASF magic) |
| Asset verdict | REAL — a genuine ffmpeg `-f asf_stream` broadcast product, sha-pinned, decodes exactly as the parser's docs describe |
| Reachable from shipped entry point? | NO (consumed only by crates/demux/src/asf/tests.rs:1433; the asf demuxer itself is unreachable from lumen CLI — see the asf/header.rs audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`, `md5sum` (317a7f3da04804c86dbaa230c3390732), `sha256sum | cut -c1-16` → `a9db9175da47fe6c`, `od -t x1z` head, python struct decode, `ffprobe` → "Invalid data found when processing input" (exit: refuses), rg for citations |

## 1. Verdict
The bytes are exactly what the ASF module's documentation claims for the §2.5 broadcast lane,
verified by direct decode (python struct over the committed file):
- bytes 0..12: the MMS `$H`-family prefix `24 48 2f 03 00 00 00 00 00 0c 2f 03` — matches
  header.rs's "broadcast `$H`-family prefix (bytes `24 48 2f 03 …` measured, plan §2.5)";
- bytes 12..28: ASF Header Object GUID `30 26 b2 75 8e 66 cf 11 a6 d9 00 aa 00 62 ce 6c` —
  byte-identical to `guid::HEADER` (guid.rs:110-112);
- header size QWORD = 757, child count = 6, reserved `01 02` — matching header.rs:502-504's
  documented 30-byte header layout and "observed `01 02`";
- first child at 42: File Properties GUID (matches `guid::FILE_PROPERTIES`), size 104 =
  24 + the 80-byte FPO payload — matching header.rs:20's "FPO payload (80 B, closes 24+80=104)";
- FPO flags DWORD = 0x3: broadcast bit 0 SET — matching header.rs:137-139 ("flags 3 vs the
  normal files' 2") and the module's `is_broadcast()` gate.

Live ffprobe 7.1.5 refuses the file ("Invalid data found") — exactly the behavior the citing
test pins (`broadcast_fixture_refuses_typed_where_ffprobe_refuses`, tests.rs:1429), where the
demuxer must refuse typed (`demux.asf.broadcast-unsupported`) wherever ffprobe refuses.

## 2. Evidence chain (who cites it, does it prove the claim)
- crates/demux/src/asf/tests.rs:1069 lists it among the committed fixtures;
  :1433 walks it in the broadcast test; :1496 pins its truncated sha256
  (`a9db9175da47fe6c`) — my `sha256sum | cut -c1-16` reproduces it exactly, so the committed
  bytes are the pinned bytes.
- The test executed green in this session's 36-pass asf run (fixture present — no regeneration
  fallback needed; the fallback regenerates via `ffmpeg -f asf_stream`, tests.rs:1435-1449).
- It is NOT cited by any doc/matrix row outside the demux crate (rg over crates/, apps/, docs/).

## 3. Findings
None for the asset itself. (Context finding recorded on the asf/header.rs audit: the demuxer
this fixture exercises is unreachable from shipped entry points.)

## 4. Notes for the next agent
- The other 10 committed ASF fixtures carry sha pins in the same `expected_sha` table
  (tests.rs:1484-1497); spot-verification of the remaining pins is one `sha256sum` loop.
- The `$H` prefix means media-probe (offset-0-anchored signatures) can never recognize this
  family — consistent with the demuxer's own `probe` doc ("the honest probe-miss").
