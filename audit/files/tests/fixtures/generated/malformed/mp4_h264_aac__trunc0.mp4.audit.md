# AUDIT: tests/fixtures/generated/malformed/mp4_h264_aac__trunc0.mp4
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A05) |
| Size / sha256 | 400 B / 54a7326b82ee7f4ae7ed9a0d18462f5b3375c8976202209b811ba2a3e08460be |
| Type | ISO Media MP4 v1 — exact prefix of the valid parent (ftyp + 368 B of moov) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A data — consumed by 2 green demux tests (below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/size/prefix-compare; box header read; rg consumers/MANIFEST/CORPUS; baseline greps |

## 1. One-paragraph verdict
NOT empty — the task's hypothesis is wrong and the generator forbids it:
truncate_derivative REJECTS cut==0 ("cut offset {cut} outside 1..len", xtask
fixtures.rs:2521-2525), so trunc0 means "first (shortest) cut of the family
ladder", here 400 B = ftyp (32 B, brands isom/iso2/avc1/mp41) + the first 368 B
of moov. Verified an EXACT byte-prefix of the valid parent mp4_h264_aac.mp4
(13,703 B, sha 3acc7eba…; python3 prefix-match TRUE). No complete moov survives,
so the demuxer must refuse typed — and both committed consumers run it green in
baseline.

## 2. What actually works (verified)
— python3: 400 B; sha256 above; d == parent[:400] TRUE; first box ftyp size 32
(bytes `00 00 00 20 66 74 79 70`); parent sha256
3acc7ebae6f41c3194bab8f23f935e57e27adb81d61d8f2b99b8cd98124c7d8e.
— Consumers (both green in baseline cargo-test-workspace.txt:2368-2375):
crates/demux/tests/mp4_fixtures.rs:229 lists this file in MALFORMED[14];
:240-246 malformed_mp4_fixtures_survive_the_fuzz_harness feeds its bytes to
fuzz::fuzz_demux_mp4 (:2374 ok); :249-281 malformed_mp4_fixtures_only_ever_
refuse_or_demux_typed drives a full typed pass asserting the refusal stage is
Demux and the code starts with "demux.mp4." (:2373 ok).
— Generated, not handmade: `cargo xtask fixtures` B1 truncation
(truncate_derivative, fixtures.rs:2515-2537 — parent's first `cut` bytes).

## 3. Findings
F-001 [S3]
Where: tests/fixtures/generated/MANIFEST.toml + tests/fixtures/CORPUS.md (absences)
Code: `rg 'mp4_h264_aac' tests/fixtures/generated/MANIFEST.toml tests/fixtures/CORPUS.md -> No matches found`
What is wrong: the whole mp4_h264_aac derivative family (8 files) is unregistered
in the MANIFEST drift gate and undocumented in CORPUS.md — no sha256/size pin
exists; behavioral pins only (typed-refuse shape + fuzz survival). Same family
defect W16-RR2 flagged for mp4_h264_aac__trunc2 (S3, "unregistered family + no
expectation TOML"). A benign byte drift inside the 400-B prefix that leaves the
demuxer's behavior unchanged would pass every committed test.
Confidence: HIGH — Verification: read (rg executed, zero hits).

## 4. Matrix evidence
Class-B corpus member (mp4_h264_aac family); the tier-1 full-stack recovery
matrix covers the mp4_mjpeg_pcm family's trunc0 (28 B) row only — this family's
rows exist nowhere in recovery_matrix.rs (its MALFORMED coverage is demux-level).
Cross-family doc nit: recovery_matrix.rs:26-27 calls the mp4_mjpeg_pcm trunc0
"(empty)" — that file is 28 B (ftyp-only), not empty; the same "trunc0=empty"
reading would be even more wrong for this 400-B file.

## 5. Open questions for the next agent
- Why does the h264_aac family stop at trunc2 (6,000 B) while mjpeg_pcm runs
  trunc0-4? If deliberate (mdat-cut ladder sizes), document it in CORPUS.md.
