# AUDIT: crates/demux/src/ts/test_support.rs
| Field | Value |
|---|---|
| Auditor | W05-A02 |
| Date | 2026-11-23 |
| Lines / functions | 243 / 12 (all builder fns; no tests in-file) |
| File verdict | REAL (test-support; correct spec layouts, two silent-corruption traps) |
| Reachable from shipped entry point? | NO (by design): no player-path caller. All users are `#[cfg(test)]` (ts/tests.rs, apps/cli ts.rs:558-559, adts.rs tests, tests/ts_fuzz_survival.rs) except the always-compiled pub harness ts/fuzz.rs:38 (see F-004). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/demux-036dd705535b2fb3 'ts::tests'` -> 66 passed, 0 failed; `... 'ts::fuzz'` -> 10 passed, 0 failed (binary newer than source); sed/rg verifications (exit 0) |

## 1. One-paragraph verdict
Genuine, spec-correct byte builders that the TS unit suite actually consumes; the
PSI section grammar (0xB0/0x70 length bytes, 9+body long-form accounting, version
byte 0xC0|version<<1|cni), PAT/PMT/SDT body layouts, PCR 6-byte field, and the
packet header bits (TEI/PUSI/PID/AFC/CC) all check out against ISO 13818-1 and
EN 300 468, and the 66 tests that eat these vectors pass (executed). Two silent
corruption traps exist for future test authors: the long-form section builder
does not clamp `section_length` to the 12-bit field (the short form does), and
`ts_packet_tuned` silently truncates oversized payloads while its doc only
promises padding. No panic paths: every slice is length-guarded; no probe-panic
twin (no indexing on externally-sized offsets at all).

## 2. What actually works (verified)
- Ran the test binary filter `ts::tests` -> "66 passed; 0 failed" — these are the
  tests that parse streams built by `psi_section*`/`pat_body`/`pmt_body`/
  `sdt_body`/`psi_packets`/`single_program_ts` (ts/tests.rs:11-12 imports), so
  the builders' outputs are demonstrably parseable by the demuxer under test.
- Ran filter `ts::fuzz` -> "10 passed; 0 failed" — includes ts::fuzz::tests which
  call `test_support::mem_stream` (fuzz.rs:38).
- Byte-layout audit against spec: psi_section_full long form `0xB0 |
  (section_length >> 8)`, `section_length = 9 + body.len()` (5 fixed + body + 4
  CRC) — correct; short form `0x70` — correct (syntax bit 0); version byte
  `0xC0 | ((version << 1) & 0x3E) | current` — correct; PCR bytes
  base>>25/>>17/>>9/>>1 + `((base&1)<<7)|((ext>>8)&1)|0x7E` + ext&0xFF — correct
  33-bit base + 6 reserved + 9-bit ext layout; AF flags 0x80 (discontinuity) /
  0x10 (PCR) — correct.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/ts/test_support.rs:62-77
Code:
```
    if !syntax {
        let sl = body.len().min(MAX_SECTION_BYTES - 3);
```
```
    let section_length = 9 + body.len();
    let mut out = Vec::with_capacity(3 + section_length);
    out.push(table_id);
    out.push(0xB0 | (section_length >> 8) as u8);
```
What is wrong: The short-form branch clamps `body` to `MAX_SECTION_BYTES - 3`
(psi.rs:53: `3 + 4095`), but the long-form branch clamps nothing: a `body`
longer than 4086 bytes makes `section_length` exceed 4095, the `as u8` cast
silently drops the overflow bits, and the emitted vector carries a declared
length that disagrees with its own bytes (CRC still computed over the full
output). A future test using a large PMT would pin or chase a corrupt vector
with no warning. What correct looks like: the same `.min(MAX_SECTION_BYTES -
9)` clamp + a `debug_assert!` (or an explicit panic — this is test code) on
oversized bodies. Confidence: HIGH — Verification: read + arithmetic.

F-002 [S3]
Where: crates/demux/src/ts/test_support.rs:141-148 (doc) and :189-190 (code)
Code:
```
/// - `payload` — the packet's payload bytes (padded to 188).
```
```
    let take = payload.len().min(188 - at);
    out[at..at + take].copy_from_slice(&payload[..take]);
```
What is wrong: The doc promises padding; the code also silently TRUNCATES any
payload longer than `188 - at` (176 bytes when a PCR adaptation field is
present). A test author handing a 184-byte PES payload to a PCR-carrying packet
loses 8 bytes with no signal — the test then pins the truncated behavior as if
intended. What correct looks like: `assert!(payload.len() <= 188 - at)` (test
code should refuse, not silently maim its own vector).
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/demux/src/ts/test_support.rs:105, 129-131
Code:
```
    body.extend_from_slice(&((program_info.len() & 0x0FFF) as u16 | 0xF000).to_be_bytes());
```
```
        let mut descriptor = vec![0x48, 0, 0x01, provider.len() as u8];
```
What is wrong: Silent narrowing casts: `program_info.len() & 0x0FFF` (line 105)
`provider.len() as u8` / `name.len() as u8` truncate oversized inputs instead of
refusing — same silent-vector-maiming class as F-001/F-002, lower likelihood
(descriptors and program_info are small in practice).
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/demux/src/ts/test_support.rs:5-7 (claim) vs crates/demux/src/ts/fuzz.rs:38
Code:
```
//! Test-support surface, not player surface — nothing in the demuxer
//! uses it.
```
```
    let Ok(stream) = super::test_support::mem_stream(data.to_vec()) else {
```
What is wrong: "Nothing in the demuxer uses it" is imprecise: the always-compiled
pub module `ts::fuzz` (mod.rs:73 `pub mod fuzz;`, no cfg gate) calls
`test_support::mem_stream`, so test_support is compiled and reachable inside the
shipped library crate (dead to the player, but not excluded from the build).
The operative claim — not player surface — is TRUE (verified: only cfg(test)
users plus the fuzz harness). Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mem_stream | 20-24 | REAL | — | delegates to avi::test_support::mem_stream (exists, :121) |
| psi_section | 33-35 | REAL | — | current=true delegate |
| psi_section_tuned | 40-48 | REAL | — | syntax=true delegate; used ts/tests.rs:123 |
| psi_section_full | 54-86 | REAL | S3/S4 | layouts correct; F-001 unclamped long form |
| pat_body | 90-97 | REAL | — | 0xE000 reserved bits correct |
| pmt_body | 102-114 | REAL | S4 | F-003 12-bit silent truncation |
| sdt_body | 122-139 | REAL | S4 | EN 300 468 0x48 descriptor correct; F-003 u8 casts |
| ts_packet | 148-150 | REAL | — | delegate |
| ts_packet_tuned | 155-192 | REAL | S3 | F-002 silent truncation; PCR/AF/header bits verified correct; no panics (all slices guarded) |
| psi_packets | 198-211 | REAL | — | pointer_field + chunks(184), CC held across sections — correct |
| stream_of | 215-221 | REAL | — | concat |
| single_program_ts | 226-243 | REAL | — | honest "PES-looking" naming (payload 00 00 00... is not a PES start code); used by 9 ts tests |

## 5. Test honesty
No tests in this file. Its consumers: ts/tests.rs (66 tests, executed, pass);
ts/fuzz.rs tests (executed, pass); apps/cli containers/ts.rs synthetic_ts tests
(cfg(test)); tests/ts_fuzz_survival.rs. The corpus tests (tests/ts_pes_corpus.rs,
real fixture bytes) do NOT use these builders — the builders feed only synthetic
vectors, so a builder defect would silently redefine what the synthetic tests
pin (that is F-001/F-002's risk).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "nothing in the demuxer uses it" | test_support.rs:6-7 | PARTLY FALSE | ts/fuzz.rs:38 uses it (always-compiled pub module); player path untouched (rg) |
| "section_length computed ... a correct CRC-32 appended" | test_support.rs:26-28 | REAL for clamped bodies; FALSE for >4086-byte bodies (F-001 wrap) |
| "payload — the packet's payload bytes (padded to 188)" | test_support.rs:145 | MISLEADING | also silently truncates (F-002) |
| "The service-loop header is 5 bytes (EN 300 468 §5.2.3)" | test_support.rs:119-120 | REAL | service_id(2)+flags(1)+loop_length(2) verified in code :127-135 |

## 7. Dead / duplicated / unreachable code
None dead: every builder has a live consumer (ts/tests.rs imports :11-12;
psi_section_tuned/full used at ts/tests.rs:123/129; single_program_ts at :251,
282, 416, 451, 514, 535, 578, 601). Unreachable from the player by design.

## 8. Dependencies & flags
Depends on plugin_api::SourceStream and crate::ts::psi (crc32_mpeg,
MAX_SECTION_BYTES) and avi::test_support (mem_stream). No feature gates; module
compiled unconditionally (mod.rs:75) — see F-004.

## 9. Open questions for the next agent
- ts/psi.rs owner: does the section assembler reject `section_length` that
  disagrees with actual bytes (it should, given F-001 can forge such vectors)?
- ts/tests.rs owner: the 66 passing tests validate these builders only against
  the demuxer's current strictness; the F-001/F-002 silent-maim paths are
  uncovered.
