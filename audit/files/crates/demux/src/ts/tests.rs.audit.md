# AUDIT: crates/demux/src/ts/tests.rs
| Field | Value |
|---|---|
| Auditor | W05-A10 |
| Date | 2026-11-23 |
| Lines / functions | 685 / 30 (28 #[test] + 2 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES for the code under test (containers/mod.rs:72 `"mpegts" => ts::open(stream, opts)` -> apps/cli/src/containers/ts.rs:110 `TsDemuxer::new`); tests.rs itself is `#[cfg(test)]` (mod.rs:77-78), reached by `cargo test -p demux` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `./target/debug/deps/demux-036dd705535b2fb3 ts::tests::` -> 66 passed (28 ts::tests + substring-matched adts), 0 failed, exit 0; baseline cargo-test-workspace.txt:2409-2415 (ts_corpus 2 passed), :2168 (demux 462 passed); python3 CRC reference reimplementation |

## 1. One-paragraph verdict
This is a real, executing, mostly honest unit-test suite: all 28 tests run and pass (prebuilt binary run, output above), the byte builders in test_support.rs are genuine MPEG-TS constructions, and the damage-path tests (resync, TEI, CC jump/dup, CRC-damaged PSI, partial tail) assert discriminating outcomes I traced by hand against mod.rs (e.g. the 57-byte resync splice arithmetic checks out exactly). The defects are test-honesty gaps, not fakes: the CRC test cannot detect a wrong polynomial and its comment carries a factually wrong guessed check value; two test names promise coverage their bodies do not deliver (PES-consumer error propagation; multi-section payloads); and 5 of 16 TsStats counters are never pinned.

## 2. What actually works (verified)
— verified by `./target/debug/deps/demux-036dd705535b2fb3 ts::tests:: --test-threads=2` -> `test result: ok. 66 passed; 0 failed; ... finished in 0.01s` (all 28 ts::tests among them).
— verified by hand-trace: `bad_sync_resyncs_with_recovery_event` (57-byte splice, resync lands at window offset 57 -> `skipped == 57`), `truncated_psi_section_drops_and_continues` (psi_partial_drops via had_partial+PUSI path, mod.rs:905-913), `continuity_jump_and_duplicate...` (cc 0,0,1,5 -> 1 dup + 1 jump).
— verified by python3 reimplementation of psi.rs's table CRC: "123456789" -> 0x0376E6E7 (published CRC-32/MPEG-2 check value) — the implementation's polynomial is correct today, independent of the weak in-file pin.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/ts/tests.rs:29-32
Code:
```
    // The known-check-value lane (poly 0x04C11DB7, init all-ones,
    // no reflection): 32 zero bytes → 0x4E08BFB4? — the corpus's own
    // section CRCs are the authority here; pin one exactly.
    assert_eq!(crc32_mpeg(&[]), 0xFFFF_FFFF);
```
What is wrong: The comment's only concrete check value is wrong — CRC-32/MPEG-2 of 32 zero bytes is 0x4A55AF67, not 0x4E08BFB4 (verified by reimplementation; "123456789" -> 0x0376E6E7 matches the published check). The assertion pins only the init, never the polynomial. Every other CRC in this file is computed by `crc32_mpeg` itself (test_support.rs:83), so the append-to-zero property holds for ANY polynomial: mutating the poly in psi.rs:76 passes all 28 tests. Why it matters: a poly regression would make every real-world TS file's PSI fail CRC and the demuxer refuse all real streams while the suite stays green. Mitigation: tests/ts_corpus.rs:199 pins `psi_crc_failures == 0` over real ffmpeg bytes — but that suite self-skips when the corpus is absent (its own header, lines 19-20), so the pin is environment-conditional. What correct looks like: `assert_eq!(crc32_mpeg(b"123456789"), 0x0376_E6E7);` and delete the wrong guess.
Confidence: HIGH — Verification: ran (python3 reference + full test run + baseline ts_corpus citation)

F-002 [S3]
Where: crates/demux/src/ts/tests.rs:634-645
Code:
```
    impl PesConsumer for Collector {
        fn pes_payload(
            &mut self,
            _pid: u16,
            _pusi: bool,
            payload: &[u8],
        ) -> Result<(), media_types::EngineError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.bytes
                .fetch_add(payload.len() as u64, Ordering::Relaxed);
            Ok(())
        }
    }
```
What is wrong: The test is named `pes_seam_receives_payloads_and_errors_propagate` (line 625) but the Collector returns `Ok(())` unconditionally; no test anywhere in the file drives the consumer-Err path (mod.rs:933-938, where `fail(err)` makes the error sticky through `next_event`). Half the name is untested. Why it matters: the sticky-error replay contract of the PES seam — a documented behavior ("The consumer's own typed error, propagated through next_event", mod.rs:455-457) — has zero coverage; a regression that swallowed the error would pass. What correct looks like: a second consumer returning Err, asserting the typed error surfaces from `next_event` and replays.
Confidence: HIGH — Verification: read (exhaustive scan of the file; only Ok(()) consumer exists)

F-003 [S3]
Where: crates/demux/src/ts/tests.rs:356-361
Code:
```
fn sections_spanning_packets_and_multi_section_payloads() {
    // A PMT big enough to split across two packets (40 ES entries →
    // a 220-byte section): the first packet carries the pointer field
    // plus a full 183-byte payload of section bytes, the second the
    // remaining 37.
    let streams: Vec<(u8, u16, &[u8])> =
```
What is wrong: The name promises two scenarios; the body delivers only the section-spanning-packets one. No test feeds a payload carrying back-to-back sections — the case SectionAssembler::feed explicitly documents as dropped (mod.rs:495-500: "back-to-back sections in one packet are the documented gap"). The spanning half is real (40 ES entries, 2 packets, asserted track count 40); the multi-section half is a name-only claim. Why it matters: the documented data-loss gap has no test even characterizing it (e.g. asserting the tail is dropped-and-counted), so a change in that behavior — in either direction — is invisible. What correct looks like: either a test characterizing the drop, or a name that stops claiming it.
Confidence: HIGH — Verification: read (whole-body scan; only one PMT section + separate PAT/SDT packets constructed)

F-004 [S4]
Where: crates/demux/src/ts/tests.rs:530
Code:
```
    assert!(stats.cc_jumps >= 1);
    bytes.clear();
}
```
What is wrong: `bytes.clear()` is a dead leftover; `let mut bytes` (line 521) is only mutated by this no-op. Noise that implies a teardown the test does not need.
Confidence: HIGH — Verification: read
F-004b [S4]
Where: crates/demux/src/ts/tests.rs:248-255 (with mod.rs:408-443 TsStats)
Code:
```
fn probe_requires_synced_stride() {
    assert!(probe(&[0x47]));
    let mut stream = single_program_ts();
    assert!(probe(&stream));
    stream[0] = 0x00;
    assert!(!probe(&stream));
}
```
What is wrong: The name says "requires synced stride" but the discriminating case — `0x47` at byte 0 with a non-`0x47` at byte 188 (probe's stride check, mod.rs:321) — is never constructed; all three asserts pass under the mutation `probe := header[0] == 0x47`. Compounding: `probe` has no production caller outside this crate (rg over crates/ + apps/: only tests.rs:14; the shipped detection path is a mirror in media-probe/src/signature/phase5.rs:150), so the unpinned stride check protects nothing exercised in production. Also: 5 of 16 TsStats counters (`packets`, `psi_skipped_sections`, `scrambled_packets`, `null_packets`, `unknown_pid_packets`) are asserted by no test in this file.
Confidence: HIGH (probe gap) / HIGH (counter gaps, exhaustive field-by-field scan) — Verification: read + rg

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| crc32_matches_the_corpus_shape | 18-33 | SHALLOW | S3 | init-only pin; wrong guessed check value; no corpus bytes (F-001) |
| pat_parses_program_entries | 37-51 | REAL | — | field-level asserts vs psi.rs parse_pat |
| pmt_parses_pcr_and_es_loop | 53-80 | REAL | — | pcr_pid/program_info/es_info asserted |
| sdt_parses_service_names_the_5_byte_loop_header | 82-109 | REAL | — | pins the 5-byte loop regression |
| cat_and_future_and_short_sections_skip | 111-137 | REAL | — | 3 skip lanes, byte counts asserted |
| crc_mismatch_is_typed | 139-151 | REAL | — | typed code asserted |
| truncated_and_bad_structure_are_typed | 153-179 | REAL | — | 3 typed lanes; stride math hand-verified |
| dvb_text_prefixes_and_composition | 181-193 | REAL | — | 4 charset lanes incl. Latin-1 fallback |
| packet_fields_parse | 197-222 | REAL | — | PCR round-trip; layout spec-checked both sides |
| tei_and_scrambling_and_af_overrun | 224-246 | REAL | — | TEI/scrambling/AF-overrun typed |
| probe_requires_synced_stride | 248-255 | SHALLOW | S4 | stride negative case missing (F-004b) |
| demux (helper) | 259-267 | REAL | — | — |
| drain (helper) | 269-278 | REAL | — | — |
| single_program_surface | 280-311 | REAL | — | 12 asserts; stats hand-traced |
| multi_program_surface | 313-353 | REAL | — | NIT-pointer exclusion + descriptor vocabulary |
| sections_spanning_packets_and_multi_section_payloads | 355-381 | SHALLOW | S3 | name promises multi-section; absent (F-003) |
| no_pat_is_typed | 385-402 | REAL | — | sticky refusal replay asserted |
| not_ts_is_typed | 404-411 | REAL | — | — |
| bad_sync_resyncs_with_recovery_event | 413-445 | REAL | — | skipped==57 hand-verified; 0 spurious discontinuities |
| unresyncable_garbage_is_typed | 447-462 | REAL | — | bounded-scan refusal |
| truncated_psi_section_drops_and_continues | 464-489 | REAL | — | drop-then-apply path hand-traced |
| crc_damaged_psi_is_dropped_not_fatal | 491-510 | REAL | — | tolerant-scan + None names |
| continuity_jump_and_duplicate_are_tolerant_and_counted | 512-531 | REAL | S4 | asserts real; dead bytes.clear() (F-004) |
| tei_packets_are_skipped_and_counted | 533-549 | REAL | — | — |
| cat_sections_skip_with_a_note | 551-574 | REAL | — | damage-event string asserted |
| af_discontinuity_indicator_raises_the_event | 576-597 | REAL | — | — |
| partial_tail_packet_is_a_note_not_an_error | 599-622 | REAL | — | skipped==5 Recovered |
| pes_seam_receives_payloads_and_errors_propagate | 624-661 | SHALLOW | S3 | error half untested (F-002) |
| seek_refuses_typed | 663-676 | REAL | — | SeekSupport::None + typed code |
| next_event_before_start_is_typed | 678-685 | REAL | — | — |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| crc32_matches_the_corpus_shape | NO (poly swap) / YES (init or append-property swap) | Self-referential CRC (F-001) |
| pat/pmt/sdt parses_* | YES | byte-exact field asserts vs independent builder |
| cat_and_future_and_short_sections_skip | YES | enum-arm + byte-count asserts |
| crc_mismatch / truncated / bad_structure | YES | typed code strings pinned |
| dvb_text_* | YES | 4 discriminating encodings |
| packet_fields_parse | YES (field decode) / NO (bit-layout swap in BOTH builder and parser) | round-trip pair; layout spec-verified manually |
| tei_and_scrambling_and_af_overrun | YES | typed bad-packet + raw bits |
| probe_requires_synced_stride | NO (stride check removal) | F-004b |
| single/multi_program_surface | YES | 12+ asserts incl. names/PIDs/kinds |
| sections_spanning_packets... | YES (spanning) / N/A (multi-section untested) | F-003 |
| no_pat/not_ts/unresyncable | YES | sticky + typed codes |
| bad_sync_resyncs | YES | skipped==57, recovered==1, discontinuities==0, stats |
| truncated_psi / crc_damaged_psi | YES | counter + surface asserts |
| continuity_jump / tei_packets / cat_sections / af_discontinuity / partial_tail | YES | counters/events with exact values |
| pes_seam_* | NO (error path) / YES (delivery counts) | F-002 |
| seek_refuses / next_event_before_start | YES | typed codes |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "32 zero bytes → 0x4E08BFB4?" | tests.rs:30 comment | FALSE (as a value; presented with "?") | python3 reference: 0x4A55AF67 |
| "the corpus sections were checked against before any Rust was written" | tests.rs:22-23 comment | UNVERIFIED (no corpus bytes in this test; corpus suite exists at tests/ts_corpus.rs and passed in baseline) | baseline cargo-test-workspace.txt:2409-2415 |
| "errors propagate" (test name) | tests.rs:625 | FALSE (untested in body) | F-002 |
| "multi_section_payloads" (test name) | tests.rs:356 | FALSE (untested in body) | F-003 |

## 7. Dead / duplicated / unreachable code
- tests.rs:530 `bytes.clear();` dead leftover (F-004).
- `probe` (tested here) has no production caller outside the demux crate; media-probe/signature/phase5.rs:150 reimplements the stride check ("mirrors demux::ts::probe") — mirror duplication, flag for the ts/mod.rs owner.

## 8. Dependencies & flags
- Pure in-crate: depends on ts::psi, ts::test_support, plugin_api, media_types. No feature gates; `#[cfg(test)]` only (mod.rs:77-78). No unsafe, no unwrap in test bodies beyond the `unreachable!` failure arms (idiomatic for tests).

## 9. Open questions for the next agent
- ts/mod.rs + psi.rs owners: psi.rs parse_section rejects `section_length < 5` for short-form sections too (psi.rs:285-288) — legal short tables (e.g. MIP) would be typed bad-structure; no test covers it.
- TsStats counters `packets`, `psi_skipped_sections`, `scrambled_packets`, `null_packets`, `unknown_pid_packets` pinned only by the corpus suite (environment-conditional), not by unit tests.
- The PES-consumer Err path (mod.rs:933-938) has zero coverage anywhere — confirm no other suite (ts_pes_corpus.rs) covers it before assuming F-002 is closed elsewhere.
