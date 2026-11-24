# AUDIT: tests/fixtures/generated/valid/avi_mjpeg_pcm.avi
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 19364 (binary; 425,516 bytes) / n-a |
| File verdict | REAL (data fixture, the canonical first-vertical fixture) |
| Reachable from shipped entry point? | NO (test/demo fixture; referenced in user-facing docs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 batch (sha256 1f56202d7b113d38…, md5 2b3c5fe5a3841191e222dce2a58a66bc); head bytes; rg "avi_mjpeg_pcm" docs/ tools/ crates/ |

## 1. One-paragraph verdict
Class-A valid fixture: genuine RIFF/AVI container (`RIFF…AVI LIST…hdrl` magic
verified in the first 24 bytes; RIFF size 0x00067e24 = 425,508 = filesize-8,
self-consistent) with MJPEG video + PCM audio. It is the fixture the shipped
smoke lane actually decodes: rc-smoke headless-decode PASS and the MP4 smoke
litmus both cite it (baseline logs below). Heavily consumed and honest. The
one defect is a doc ambiguity: vlc-diff-report prints a bare hex digest next
to the filename that is NOT the file's md5.

## 2. What actually works (verified)
— python3: first 24 bytes `52 49 46 46 24 7e 06 00 41 56 49 20 4c 49 53 54 24 01 00 00 68 64 72 6c` = "RIFF", size 0x67e24, "AVI ","LIST", 0x124, "hdrl".
— Size claim in docs/guides/getting-started.md:81 (`bytes=425516`) matches the
real file size exactly (verified: 425,516 B).
— Consumers: tests/integration/first-vertical (harness), demux/codecs/
playback-engine corpora, tools/bench-vs-vlc; docs/release-checklist.md:73
names it as the default headless-decode fixture.
— Baseline execution evidence: audit/runs/baseline/rc-smoke.txt:131
"headless-decode: PASS (v_decoded==v_frames, a_decoded==a_frames)" and
audit/runs/baseline/mp4-smoke.md AVI/MJPEG/PCM fixture pass.

## 3. Findings
F-001 [S3]
Where: docs/corpus/vlc-diff-report-20261023.md:313 (claim touching this file)
Code: `avi_mjpeg_pcm.avi 08db7ccc163dc478709627c1db9044b5 (352,800 B, 60 packets)`
What is wrong: the actual file md5 is 2b3c5fe5a3841191e222dce2a58a66bc and its
size is 425,516 B. The printed digest/size evidently describe the decoded
first-audio PCM payload, not the fixture named in the sentence.
Why it matters: an evidence line that names a file and prints an unrelated
digest invites false verification; a reader cannot tell which object the
digest pins. What correct looks like: label it `pcm_payload_md5=…`.
Confidence: HIGH — Verification: ran md5sum on the real file (python3).

## 4. Matrix evidence
Used as the demo fixture across docs/guides/getting-started.md:42-81 and
soak scenario logs (docs/soak/raw/*). No feature-matrix.toml row names it
(rg over feature-matrix.toml: no hit) — release-checklist row 4
("headless-decode") is its operational citation.
