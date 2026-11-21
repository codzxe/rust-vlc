# AUDIT: crates/demux/src/ts/tests.rs
| Field | Value |
|---|---|
| Auditor | W05-A02 |
| Date | 2026-11-21 |
| Lines / functions | 685 / 30 (28 `#[test]` + 2 helpers `demux`/`drain`) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the code under test is: `lumen open file.ts` → `apps/cli/src/host.rs:904 open_source` → `demux_host.rs:458 containers::open` → `apps/cli/src/containers/mod.rs:72 "mpegts" => ts::open` → `apps/cli/src/containers/ts.rs:110 TsDemuxer::new(stream)`. The tests themselves ran in baseline: all 28 pass (`test-workspace.log.txt:1928–1945`, `test ts::tests::… ok`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `rg -n "demux::ts::tests::" audit/runs/baseline/test-workspace.log.txt` (0 — unit tests named `ts::tests::*`); per-test `rg -c "test ts::tests::<name> \.\.\. ok"` (28/28 = 1 hit each); `rg -n "TsDemuxer::new" crates apps` (production caller only `apps/cli/src/containers/ts.rs:110`); `rg -n "ts::probe"` (no non-test caller); `sed -n` quote verification for every finding. |

## 1. One-paragraph verdict
This is a genuinely strong unit-test suite: every test ran and passed in the baseline workspace run, the tests assert specific field values (not tautologies), and the damage paths (no-PAT, TEI, CC jumps, CRC damage, truncation, resync, partial tails) are each exercised with typed-error and counter assertions. Three test names promise coverage the bodies do not deliver — the CRC "known-check-value lane" (init-only pin, poly unverified in-file), "multi_section_payloads" (never constructed; a documented assembler gap), and "errors_propagate" (the collector always returns `Ok`) — which is exactly the kind of name-vs-body inflation this audit exists to catch. No S0/S1/S2: the file is test-only, cannot panic in production, and the code it exercises is real and shipped.

## 2. What actually works (verified)
— All 28 tests ran green in baseline: `rg -c "test ts::tests::<name> \.\.\. ok" audit/runs/baseline/test-workspace.log.txt` → 1 for each of 28 names (e.g. line 1928 `crc32_matches_the_corpus_shape`, 1945 `single_program_surface`). Suite context: `cargo test --workspace --locked` 1681 passed / 1 failed (the unrelated `zero_alloc_transport.rs:98`), INDEX.md row 3.
— Field-exact parsing pins: `pat_parses_program_entries` asserts tsid 0x1234, both (program,pid) pairs; `pmt_parses_pcr_and_es_loop` asserts pcr_pid 0x100, program_info, both ES entries and the verbatim ISO-639 descriptor; `packet_fields_parse` asserts PCR base 0x1_2345_6789/ext 0x1FF round-trip and payload length 176. A broken parser fails these.
— Damage accounting is pinned exactly: `bad_sync_resyncs_with_recovery_event` asserts `skipped == 57`, `sync_errors == 1`, `resync_bytes == 57`, `discontinuities == 0` — a regression in resync boundary handling fails this test.
— The CRC-32/MPEG-2 polynomial is externally validated (not by this file, see F-001): `crates/demux/tests/ts_corpus.rs:199` pins `psi_crc_failures == 0` over five real ffmpeg-authored corpus lanes, which ran in baseline (`test-workspace.log.txt:2241 corpus_ts_programs_streams_and_names_match_ffprobe ... ok`).

## 3. Findings
### F-001 [S3]
Where: crates/demux/src/ts/tests.rs:29-32
Code:
```
    // The known-check-value lane (poly 0x04C11DB7, init all-ones,
    // no reflection): 32 zero bytes → 0x4E08BFB4? — the corpus's own
    // section CRCs are the authority here; pin one exactly.
    assert_eq!(crc32_mpeg(&[]), 0xFFFF_FFFF);
```
What is wrong: The "known-check-value lane" the comment announces is never delivered. The only pin is `crc32_mpeg(&[]) == 0xFFFF_FFFF` — the init value, true for any implementation that merely loads the init register; it cannot distinguish the correct polynomial 0x04C11DB7 from a wrong one. The zero-property test above it is self-referential: `psi_section` (test_support.rs:83) computes the CRC with the same `crc32_mpeg` that `section_crc_ok` re-verifies, so a wrong polynomial passes both assertions. The comment's own "0x4E08BFB4?" question mark admits the vector was never pinned.
Why it matters: within this file the CRC implementation's interoperability with the MPEG-2 standard is unproven; a poly regression here stays green (the standard known-answer vector is `crc32_mpeg(b"123456789") == 0x0376E6E7`). The safety net is external: ts_corpus.rs:199 over real corpus bytes (verified in baseline). A unit-test-only consumer of this crate has no poly protection.
What correct looks like: one independent known-answer vector, e.g. `assert_eq!(crc32_mpeg(b"123456789"), 0x0376_E6E7);` replacing the question-mark comment.
Confidence: HIGH — Verification: read (psi.rs:76 poly table, test_support.rs:83 self-reference, baseline log line 2241).

### F-002 [S3]
Where: crates/demux/src/ts/tests.rs:355-359
Code:
```
#[test]
fn sections_spanning_packets_and_multi_section_payloads() {
    // A PMT big enough to split across two packets (40 ES entries →
    // a 220-byte section): the first packet carries the pointer field
    // plus a full 183-byte payload of section bytes, the second the
    // remaining 37.
```
What is wrong: The name's second clause — "multi_section_payloads" — is never tested. The body builds exactly one PMT section split over two packets (the spanning half). No packet anywhere in the suite carries two back-to-back sections (the `psi_packets` helper, test_support.rs:198-211, always starts a new packet per section), and the pointer-field mechanism for a mid-payload section start (`payload[1+ptr..]` skipping a completed section's tail) is never exercised with `ptr > 0`.
Why it matters: the production assembler documents this as a real gap — mod.rs:498-500: "back-to-back sections in one packet are the documented gap — the corpus and broadcast PSI repeat one section per PUSI packet". The test name launders that known functional gap into apparent coverage. A broadcast stream that packs PAT+CAT or multiple SDT sections per packet (legal and common, ISO/IEC 13818-1 §2.4.4) loses every section after the first in a PUSI packet.
What correct looks like: either rename the test to `section_spanning_packets`, or add a case: one PUSI packet whose pointer field is 0 and whose tail contains the start of a second section (and a packet with `ptr > 0` skipping a completed section) — and land the assembler support it would require.
Confidence: HIGH — Verification: read (test body, test_support.rs:198-211, mod.rs:494-501).

### F-003 [S3]
Where: crates/demux/src/ts/tests.rs:624-644
Code:
```
#[test]
fn pes_seam_receives_payloads_and_errors_propagate() {
…
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
What is wrong: The name's "errors_propagate" clause is untested. The `Collector` always returns `Ok(())`; no test ever returns `Err` from a `PesConsumer` and asserts the demuxer surfaces it. The production path exists — mod.rs:934-938 `let Err(err) = consumer.pes_payload(…) { let _ = self.fail(err); }` — but is unexercised by this suite.
Why it matters: mutation analysis: delete the `let Err(err) = … self.fail(err)` propagation in mod.rs (make it swallow the error) — every test in this file stays green. The seam's error contract, which the CLI's TS bridge (apps/cli/src/containers/ts.rs:479 "bridge-poisoned") depends on to abort a poisoned stream, has zero test protection.
What correct looks like: a second collector that returns `Err` on the Nth call, asserting `next_event()` yields that typed error and the demuxer enters its failed state.
Confidence: HIGH — Verification: read (test body, mod.rs:928-941).

### F-004 [S4]
Where: crates/demux/src/ts/tests.rs:530
Code:
```
    assert!(stats.cc_duplicates >= 1);
    assert!(stats.cc_jumps >= 1);
    bytes.clear();
}
```
What is wrong: `bytes.clear();` is a dead statement — it clears a local `Vec` that is never read again; it proves nothing and guards nothing.
Why it matters: dead statements in tests invite "did something get removed here?" archaeology and trip no lints; the two `>= 1` assertions above it are also weaker than the suite's own standard (neighboring tests pin exact counts, e.g. `tei_packets == 1`).
What correct looks like: delete the line; pin `stats.cc_duplicates == 1` and `stats.cc_jumps == 1` like `tei_packets_are_skipped_and_counted` does.
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: crates/demux/src/ts/tests.rs:248-255
Code:
```
#[test]
fn probe_requires_synced_stride() {
    assert!(probe(&[0x47]));
    let mut stream = single_program_ts();
    assert!(probe(&stream));
    stream[0] = 0x00;
    assert!(!probe(&stream));
}
```
What is wrong: Not the test's logic (it matches mod.rs:315-322 exactly) but its evidential value: `demux::ts::probe` has no non-test caller in the repo (`rg -n "ts::probe" crates apps` → only tests/docs). The shipped binary's TS detection is a hand-mirrored copy, `media-probe/src/signature/phase5.rs:151` ("mirrors `demux::ts::probe`"), which has no test of its own in phase5.rs.
Why it matters: this green test certifies a function the shipped CLI never calls, while the copy the CLI does call is untested here — the two can drift silently (e.g. the `len() < PACKET_BYTES` short-accept rule) and shipped `.ts` detection breaks with all tests green.
What correct looks like: a shared check (or a test importing both) pinning the two implementations to the same truth table.
Confidence: MEDIUM — Verification: read + rg (mirror located; whether some other test file ties the mirror is UNVERIFIED — not found in phase5.rs itself, not exhaustively searched crate-wide).
Note: the drift *risk* is HIGH-confidence; only the "no tie exists anywhere" claim is MEDIUM.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| crc32_matches_the_corpus_shape | 18-33 | REAL (partial) | S3 | F-001: zero-property self-referential; init-only pin |
| pat_parses_program_entries | 37-51 | REAL | — | field-exact; fails on parser regression |
| pmt_parses_pcr_and_es_loop | 53-80 | REAL | — | pcr_pid/ES loop/verbatim es_info pinned |
| sdt_parses_service_names_the_5_byte_loop_header | 82-109 | REAL | — | the 5-byte-loop regression guard; provider+name pinned |
| cat_and_future_and_short_sections_skip | 111-137 | REAL | — | three distinct skip lanes (CAT, cni=0, short-form) |
| crc_mismatch_is_typed | 139-151 | REAL | — | typed code + stage pinned |
| truncated_and_bad_structure_are_typed | 153-179 | REAL | — | 3 damage shapes; re-computes CRC after forging lengths |
| dvb_text_prefixes_and_composition | 181-193 | REAL | — | 5 encodings incl. ISO 6937 compose + Latin-1 fallback |
| packet_fields_parse | 197-222 | REAL | — | PCR round-trip self-consistent (spec value not externally pinned) |
| tei_and_scrambling_and_af_overrun | 224-246 | REAL | — | AF-overrun → typed bad-packet |
| probe_requires_synced_stride | 248-255 | REAL | S4 | F-005: tested probe not the shipped one |
| demux (helper) | 259-267 | REAL | — | constructs via mem_stream; unreachable! on setup failure (test-only, fine) |
| drain (helper) | 269-278 | REAL | — | walks to EndOfStream, returns stats |
| single_program_surface | 280-311 | REAL | — | 13 assertions incl. service names, track_of_pid handles, psi_sections==3 |
| multi_program_surface | 313-353 | REAL | — | NIT-pointer exclusion, mp3/hevc descriptors |
| sections_spanning_packets_and_multi_section_payloads | 355-381 | SHALLOW (half the name) | S3 | F-002: only spanning tested |
| no_pat_is_typed | 385-402 | REAL | — | incl. sticky-refusal replay |
| not_ts_is_typed | 404-411 | REAL | — | 2048 zero bytes |
| bad_sync_resyncs_with_recovery_event | 413-445 | REAL | — | exact skipped=57, counters, zero spurious discontinuity |
| unresyncable_garbage_is_typed | 447-462 | REAL | — | bounded-scan refusal |
| truncated_psi_section_drops_and_continues | 464-489 | REAL | — | partial drop + good-PAT recovery |
| crc_damaged_psi_is_dropped_not_fatal | 491-510 | REAL | — | damage-noted, names stay None |
| continuity_jump_and_duplicate_are_tolerant_and_counted | 512-531 | REAL (weak pins) | S4 | F-004: `>=1` pins + dead `bytes.clear()` |
| tei_packets_are_skipped_and_counted | 533-549 | REAL | — | exact ==1 |
| cat_sections_skip_with_a_note | 551-574 | REAL | — | damage_events reason string pinned |
| af_discontinuity_indicator_raises_the_event | 576-597 | REAL | — | >=1 event |
| partial_tail_packet_is_a_note_not_an_error | 599-622 | REAL | — | 5-byte tail → Recovered{skipped:5} |
| pes_seam_receives_payloads_and_errors_propagate | 624-661 | REAL (half the name) | S3 | F-003: no error path exercised |
| seek_refuses_typed | 663-676 | REAL | — | SeekSupport::None + typed refusal |
| next_event_before_start_is_typed | 678-685 | REAL | — | state-machine guard |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 26 REAL tests above | YES | specific values/counters/typed codes; mutation of the corresponding parser/state path breaks a pinned assertion (reasoned per-row; no code edited) |
| crc32_matches_the_corpus_shape | NO (for a wrong polynomial) | self-referential: helper CRCs with same fn (F-001); would fail only for broken init/xorout |
| sections_spanning_packets_and_multi_section_payloads | YES (spanning), NO (multi-section) | assembler spanning regression caught; multi-section payload drop never constructed (F-002) |
| pes_seam_receives_payloads_and_errors_propagate | YES (payload delivery), NO (propagation) | deleting mod.rs's `self.fail(err)` on consumer Err keeps suite green (F-003) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "1681 passed / 1 failed" workspace incl. these 28 | baseline INDEX.md row 3 | REAL | test-workspace.log.txt:1928-1945, 28× `ts::tests::… ok` |
| TS "damage paths (each tested, all typed/no panics)" | tests.rs:383 section banner | REAL | the six damage tests each pin a typed code or counter |
| (cross-ref) MP4 wave context: demux layer works for MP4 while the decoder refuses | baseline INDEX.md CORRECTION v2 | REAL | mp4-smoke-open-session-420.txt (plays) vs mp4-smoke-open-session.txt (decode.unsupported) — TS/demux tests are not the MP4 blocker |

## 7. Dead / duplicated / unreachable code
- `bytes.clear()` dead statement (F-004).
- The tested `demux::ts::probe` is production-dead in the shipped binary (the mirror in media-probe phase5.rs serves detection) — duplication risk, not this file's defect (F-005).
- No `#[ignore]`, no cfg-gated tests in this file; all 28 run in the default profile (verified per-name in baseline log).

## 8. Dependencies & flags
- Module wiring: `ts/mod.rs:77-78` — `#[cfg(test)] mod tests;` unit-test only, zero shipping weight.
- Uses `media_types`, `plugin_api` (DemuxEvent/Demuxer/SeekSupport) — consistent with crate rules (lib.rs:35-41 "Std-only, zero external dependencies").
- No feature flags touch this file (no `#[cfg(feature)]` present).

## 9. Open questions for the next agent
- mod.rs (SectionAssembler, not my assignment): confirm the back-to-back-sections drop (mod.rs:498-500) against real broadcast captures — the corpus lanes are ffmpeg-authored and repeat one-section-per-PUSI, so the gap is invisible there.
- media-probe/src/signature/phase5.rs: is TsSignature covered by any test in that crate? If not, the shipped TS probe is test-blind (F-005).
- ts_pes_corpus.rs / ts_fuzz_survival.rs (other agents' files): they consume this file's test_support; a helper regression there surfaces as their failure, not this suite's.
