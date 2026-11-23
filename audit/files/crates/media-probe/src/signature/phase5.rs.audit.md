# AUDIT: crates/media-probe/src/signature/phase5.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-23 |
| Lines / functions | 208 / 17 (1 helper + 8 signature structs x probe+extensions) |
| File verdict | REAL (with an S1 bounds bug in the TS signature) |
| Reachable from shipped entry point? | YES — apps/cli/src/demux_host.rs:443-448: every opened file (file:// and http(s) lanes) runs `probe_stream` over `ProbeRegistry::with_builtins()` -> this file's signatures, no extension pre-filter, no catch_unwind. |
| Findings | S0: 0 S1: 1 S2: 0 S3: 0 S4: 2 |
| Commands run | rustc standalone repro (EXECUTED — output below), rg (callers, roundtrip tests, catch_unwind), sed quote checks, baseline citation. |

## 1. One-paragraph verdict
Eight magic-anchored signatures, each bounds-guarded — except the TS one, which has a fatal off-by-one: `TsSignature::probe` indexes `prefix[188]` under the guard `prefix.len() >= 188`, so a probe window of EXACTLY 188 bytes starting with 0x47 panics with "index out of bounds: the len is 188 but the index is 188". I reproduced and executed the exact logic with a standalone rustc binary (both this function and its `demux::ts::probe` twin, which carries the identical bug). The shipped CLI probes every opened file with a 64 KiB window, and fill_window truncates the window to file size — so a 188-byte local OR REMOTE (http lane serves the same path, demux_host.rs:453-460) file whose first byte is 'G' crashes the open path. This includes a VALID single-packet TS file, which the demuxer's own comment says should be accepted. The mirror-duplication design (the file's stated rationale) is the root cause: the same off-by-one had to be maintained in two crates and is wrong in both; the round-trip suite has no 188-byte case, so nothing fails.

## 2. What actually works (verified)
— verified by EXECUTION (standalone rustc binary, audit/runs/tmp/ts_probe_repro.rs, rustc 1.99.1):
```
phase5 TsSignature on 188-byte 0x47 file: true   (panicked)
demux ts::probe on 188-byte 0x47 file: true      (panicked)
phase5 on 189 bytes: Some(189)                   (ok)
demux on 189 bytes: true                          (ok)
phase5 on 187 bytes: Some(1)                      (ok)
demux on 187 bytes: true                          (ok)
stderr: index out of bounds: the len is 188 but the index is 188
```
(the `true` values are catch_unwind results: both functions panicked on the 188-byte input; 187/189-byte controls behave.)
— read: the other seven signatures (WAV/AIFF/Ogg/MKV/ADTS/PS/IVF) are correctly length-guarded before every slice/index (61-63, 81-82, 102, 117-118, 137/140, 185-187, 202).
— baseline: media-probe 80 unit tests + 1 doctest pass — none covers a 188-byte window (verified: the TS near-miss case in tests/registry_roundtrip.rs:284-286 uses 201 bytes).

## 3. Findings
F-001 [S1]
Where: crates/media-probe/src/signature/phase5.rs:160-164
Code (verbatim):
```
        if prefix.len() >= TS_PACKET_BYTES {
            if prefix[TS_PACKET_BYTES] != 0x47 {
                return None;
            }
```
What is wrong: the guard admits `prefix.len() == 188`, then indexes `prefix[188]` — one past the end. A probe window of exactly 188 bytes starting with 0x47 panics. Why it matters: (1) it is reachable from the shipped CLI by opening ANY 188-byte file whose first byte is 0x47 ('G') — the open path probes every file with no extension filter and no panic guard (demux_host.rs:443-448), and the http(s) lane serves the identical probe path, making it remotely triggerable; (2) a legitimate single-packet TS capture is exactly 188 bytes — the demuxer's own comment (ts/mod.rs:320: "a whole 188-byte file is also accepted — the walk will tell") documents that such files are meant to be accepted, so this is a crash on valid input too; (3) the twin `demux::ts::probe` (ts/mod.rs:315-322, `header.len() < PACKET_BYTES || header[PACKET_BYTES] == 0x47`) panics identically at len==188 — same bug, two crates. Verification: EXECUTED — standalone repro compiled and run (output in §2); panic message "index out of bounds: the len is 188 but the index is 188". What correct looks like: `prefix.len() > TS_PACKET_BYTES` here, `header.len() <= PACKET_BYTES ||` in the demuxer twin, plus a regression test with a 188-byte input in BOTH crates. Confidence: HIGH — Verification: ran.
F-002 [S4]
Where: crates/media-probe/src/signature/phase5.rs:40-43 (+ the whole mirror design, doc 7-13)
Code (verbatim):
```
/// The 188-byte MPEG-TS packet stride (ISO 13818-1 §2.4.3), mirrored
/// from `demux::ts::PACKET_BYTES` (no crate edge; mirror-pinned by
/// the round-trip suite).
const TS_PACKET_BYTES: usize = 188;
```
What is wrong: the crate-edge prohibition forces every demuxer probe gate to be duplicated by hand in media-probe; F-001 is the direct product (one off-by-one, faithfully copied into both crates). The round-trip suite pins agreement between the twins but cannot pin correctness of the shared logic — and indeed both panic. Blast radius: every signature in this file is a hand-copied twin. Confidence: HIGH — Verification: read + the executed twin repro.
F-003 [S4]
Where: crates/media-probe/src/signature/phase5.rs:166-169
Code (verbatim):
```
        // Shorter than one packet: a lone sync byte is all the window
        // can see — the demuxer's own probe accepts it, so the claim
        // stays anchored (score 70, one byte consumed).
        Some(anchored(FormatId::mpegts(), 1))
```
What is wrong: a single 0x47 byte claims the `mpegts` family at SCORE_MAGIC_ANCHOR (70) — a score the ladder defines as "magic anchored — the format family is certain". One byte of 'G' (any 1..187-byte file starting with 'G') is routed to the TS container host and fails there with a demux-stage error instead of the honest probe-stage `probe.no-demuxer`. Documented and twin-consistent, but it overclaims certainty by the module's own scoring semantics. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| anchored | 46-53 | REAL | — | uniform hit shape, empty hints |
| WavSignature::probe / extensions | 60-72 | REAL | — | RIFF/RF64/BW64+WAVE, len<12 guard ✓ |
| AiffSignature::probe / extensions | 80-92 | REAL | — | FORM+AIFF/AIFC ✓ |
| OggSignature::probe / extensions | 101-107 | REAL | — | OggS, len>=4 ✓ |
| MkvSignature::probe / extensions | 116-124 | REAL | — | EBML BE magic ✓ |
| AdtsSignature::probe / extensions | 136-146 | REAL | — | ADIF + 12-bit-sync/layer-00 mask 0xF6/0xF0 — bit math verified correct (MP3 0xFB & 0xF6 = 0xF2 ≠ 0xF0, not stolen) |
| TsSignature::probe / extensions | 156-174 | REAL (buggy) | S1 | off-by-one panic at len==188 (F-001); 1-byte overclaim (F-003) |
| PsSignature::probe / extensions | 184-192 | REAL | — | 00 00 01 BA, starts_with ✓ |
| IvfSignature::probe / extensions | 201-207 | REAL | — | DKIF ✓ |

## 5. Test honesty
Tests live in crates/media-probe/tests/ (registry_roundtrip.rs etc., not this file):
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| roundtrip TS cases (committed fixtures, multi-packet) | YES for magic/stride agreement | but no 188-byte input — F-001 passes the whole suite |
| near_misses_and_garbage_stay_unprobed (201-byte TS negative) | YES | negative pinned at len=201; the fatal len=188 boundary unpinned |
| ADTS/mp3 layer-bit disjointness (roundtrip 292-301) | YES | both directions pinned |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "mirroring its demuxer's own probe(header) gate byte-for-byte" | phase5.rs:7-8 | TRUE — and that is the problem: the mirror copies the demuxer's off-by-one panic faithfully (F-001/F-002) |
| "any drift is a test failure, not a silent guess" | phase5.rs:10-13 | SHALLOW | the round-trip pins twin AGREEMENT, not correctness; identical bugs in both twins pass |
| "Every signature here awards SCORE_MAGIC_ANCHOR only" | phase5.rs:17-18 | REAL | anchored() is the only constructor used |
| "the ES demuxers are routed by extension in the open path, never by a content claim here" | phase5.rs:29-31 | REAL-UNPROVEN | consistent with demux_host/containers reading; the extension-route auditors should confirm |

## 7. Dead / duplicated / unreachable code
No dead code. Duplication is the design (F-002): TS_PACKET_BYTES + the probe logic exist in both media-probe (this file) and demux/src/ts/mod.rs.

## 8. Dependencies & flags
crate::report/signature only; std-only; no unsafe; no features. House rules hold except the S1.

## 9. Open questions for the next agent
- URGENT for the demux/src/ts/mod.rs auditor: ts/mod.rs:315-322 `probe` panics at header.len()==188 — same executed repro; the demuxer-side callers (TsDemuxer::start? container ts.rs?) determine its independent reachability. My repro proves the function panics; call-site reachability for the demuxer twin is UNVERIFIED by me.
- media-probe/tests auditor: add the 188-byte TS regression case (both a 0x47-first valid shape and the negative) to registry_roundtrip.
- Engine: should the open path wrap probe in catch_unwind as defense-in-depth? (No catch_unwind exists anywhere in apps/cli or playback-engine — rg verified.)
