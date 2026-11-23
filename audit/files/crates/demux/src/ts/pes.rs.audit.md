# AUDIT: crates/demux/src/ts/pes.rs
| Field | Value |
|---|---|
| Auditor | W05-A02 |
| Date | 2026-11-23 |
| Lines / functions | 1235 / 27 prod fns+methods, 19 unit tests + 1 doctest + 2 test helpers |
| File verdict | REAL (with a FALSE seam doc and spec-wrong field parse) |
| Reachable from shipped entry point? | YES: lumen-cli `containers::ts::open` (apps/cli/src/containers/ts.rs:110) -> `set_pes_consumer(BridgeConsumer)` (:153) -> `BridgeConsumer::pes_payload` (:422) -> `PesBridge::push` (:254) -> `PesAssembler::push_packet` (pes.rs:524). `observe_pcr` and the `discontinuity` argument: NOT reachable (see F-001). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 5 S4: 3 |
| Commands run | `./target/debug/deps/demux-036dd705535b2fb3 ts::pes` -> 20 passed, 0 failed (binary newer than source, 2026-11-23 20:05 vs 17:35); sed/rg verifications (exit 0) |

## 1. One-paragraph verdict
The PES assembler is genuinely implemented and genuinely wired into the shipped TS
open path; the 33-bit PTS/DTS reassembly math is correct, the memory ceiling is a
checked typed refusal, and every production index/slice is bounds-guarded — the
Wave-4 `TsSignature::probe` off-by-one panic has NO twin here (all of pes.rs's
indexing audited, see F-NC-1). But the module doc's central seam claim is false in
the shipped build (the transport never hands this layer the discontinuity bit or a
single PCR observation — two of five `TimingEvent` variants are unreachable and
`PesPacket.discontinuity` is a constant `false` in production), `data_alignment` is
parsed from the wrong byte (it returns ESCR_flag), and the per-packet discontinuity
flag is clobbered by the next start's flag before the pending packet completes.

## 2. What actually works (verified)
- Ran `./target/debug/deps/demux-036dd705535b2fb3 ts::pes` -> "test result: ok.
  20 passed; 0 failed" (19 `ts::pes::tests` + `ts::tests::pes_seam_receives_
  payloads_and_errors_propagate`). Orchestrator baseline agrees:
  audit/runs/baseline/cargo-test-workspace.txt:2076-2084 (all ts::pes tests ok).
- decode_ts5 bit math manually re-derived against §2.4.3.7 field layout:
  `(b0&0x0E)<<29 | b1<<22 | (b2&0xFE)<<14 | b3<<7 | (b4&0xFE)>>1` is exact for all
  2^33 values (roundtrip test covers 0, 1, 126_000, 1<<32, 2^33-1).
- `MAX_PES_BYTES` ceiling: `checked_add` + `filter(total > MAX)` at pes.rs:567-577 —
  no unbounded allocation; typed `ResourceLimit` refusal (test executed).
- Bounded-PES completion, overrun truncation, EOS typed truncation, dormancy on
  non-PES PIDs: all executed by the suite above.

## 3. Findings
F-001 [S2]
Where: crates/demux/src/ts/pes.rs:60-62 (claim), :531-534, :592-613 (dead-in-production code); apps/cli/src/containers/ts.rs:255
Code:
```
//! parses the adaptation field and hands this layer the
//! `discontinuity_indicator` bit and PCR observations
//! ([`PesAssembler::observe_pcr`]). The deliberately small
```
```
    fn push(&mut self, pid: u16, pusi: bool, payload: &[u8]) -> Result<(), EngineError> {
        self.assembler.push_packet(pid, pusi, false, payload)?;
```
What is wrong: The doc claims the transport layer hands this layer the
discontinuity bit and PCR observations. It does not, anywhere in the shipped build.
The only production consumer (the CLI bridge, the sole `set_pes_consumer` caller
outside tests) hardcodes `discontinuity = false`; `observe_pcr` has zero production
callers (rg over crates/ + apps/: only pes.rs tests and tests/ts_pes_corpus.rs).
The `PesConsumer` trait (ts/mod.rs:457) carries only `(pid, pusi, payload)`.
Consequences: `PesPacket.discontinuity` is a constant `false` in the shipped lane
(so `mint_packet`'s `discontinuity: pes.discontinuity` and the ADTS
`discontinuity && aus == 0` at apps/cli ts.rs:370/452 never fire), and
`TimingEvent::DiscontinuityIndicator` (pes.rs:531-534) and `TimingEvent::PcrRegressed`
(:607-612) are UNREACHABLE from any shipped entry point. The engine still hears
transport-level discontinuities via ts/mod.rs:883, so the impact is a false module
doc plus dead timing machinery, not total silence. What correct looks like: either
extend the `PesConsumer` seam to carry the adaptation-field bits (and call
`observe_pcr` from the walk), or rewrite the doc to say the shipped seam passes
payload only. Confidence: HIGH — Verification: read + rg (callers enumerated).

F-002 [S3]
Where: crates/demux/src/ts/pes.rs:535-547
Code:
```
        let had_pending = {
            let state = self.states.entry(pid).or_insert_with(Assembly::new);
            if start {
                state.dormant = false;
                state.disc = discontinuity;
                !state.buf.is_empty()
```
What is wrong: On a new start, `state.disc = discontinuity` overwrites the flag
accumulated from continuation packets of the PENDING PES before `complete_one`
(:546-548) reads it. A discontinuity set on a continuation packet (line 579-581)
is dropped from the completed packet, and the completed packet instead inherits
the NEXT packet's start bit. This contradicts the field's own contract
(Assembly doc, :458: "Discontinuity seen since the last PES start on this PID").
The unit test `discontinuity_surfaces_as_event_and_packet_flag` only covers the
start-boundary case where the re-set at :579-581 re-establishes the flag — it
passes while the continuation case is broken. Blast radius: zero in the shipped
lane (F-001: always false), latent for any future honest wiring.
What correct looks like: snapshot `state.disc` for the pending packet before
assigning the new start's flag. Confidence: HIGH — Verification: read + manual
trace (mutation reasoning; no run).

F-003 [S3]
Where: crates/demux/src/ts/pes.rs:339-342
Code:
```
        let flags2 = bytes[7];
        let data_len = usize::from(bytes[8]);
        header.header_len = 9 + data_len;
        header.data_alignment = flags2 & 0x04 != 0;
```
What is wrong: Per ISO/IEC 13818-1 Table 2-16, `data_alignment_indicator` is bit 2
of the FIRST flags byte (offset 6); bit 2 of offset 7 (`flags2 & 0x04`) is
`ESCR_flag`. Byte 6 is never read at all, so the parsed `data_alignment` is
actually ESCR_flag and PES_scrambling_control is silently ignored. The sibling
parser in the same crate confirms the layout: ps/mod.rs:528 checks
`if buf[0] >> 6 != 0b10` (the '10' marker of byte 6). The test factory enshrines the
same misreading — pes.rs:839 `let mut flags2 = 0x04u8; // data_alignment_indicator
set` sets ESCR_flag, and the test asserts `pkt.data_alignment` (line 925) —
parser and vector agree with each other and both contradict the spec.
No production consumer reads `PesPacket.data_alignment` (the CLI bridge
destructures `data_alignment: _`), limiting blast radius.
What correct looks like: `header.data_alignment = bytes[6] & 0x04 != 0;`.
Confidence: HIGH — Verification: read (spec layout cross-checked against ps/mod.rs).

F-004 [S3]
Where: crates/demux/src/ts/pes.rs:346-353
Code:
```
        match flags2 >> 6 {
            0b10 => {
                header.pts = Some(decode_ts5(&bytes[9..], 0b0010)?);
            }
            0b11 => {
                header.pts = Some(decode_ts5(&bytes[9..], 0b0011)?);
                header.dts = Some(decode_ts5(&bytes[14..], 0b0001)?);
```
What is wrong: No validation that `data_len` can hold the declared stamps
(spec: >=5 for PTS-only, >=10 for PTS+DTS). The `bytes.len() < header.header_len`
guard bounds the read to the BUFFER, not to the declared optional header, so a
damaged PES with flags `10`/`11` and a small `data_len` gets its PTS/DTS decoded
out of PAYLOAD bytes past the declared header (accepted as real stamps when the
marker bits happen to match) instead of being typed as `short_header` damage.
No panic (decode_ts5 re-checks length). What correct looks like: reject when
`data_len < 5` (flags 10) / `data_len < 10` (flags 11) before decoding.
Confidence: HIGH — Verification: read + manual trace.

F-005 [S3]
Where: crates/demux/src/ts/pes.rs:624-677 (complete_one) vs :770-787 (finish)
Code:
```
        let buf = std::mem::take(&mut state.buf);
        let disc = std::mem::replace(&mut state.disc, false);
        match PesHeader::parse(&buf) {
            Ok(header) => {
                if header.class == PesStreamClass::Padding {
```
What is wrong: A bounded PES cut short mid-stream by the next `payload_unit_start`
completes through `complete_one` with no comparison of `buf.len()` against
`header.total_len` — the short packet is emitted silently (payload truncated, no
`Recovered` record, no `damage_bytes` entry), even though the same condition at
EOS is the typed sticky `demux.ts.pes.truncated` (finish, :772-786). This also
covers the PUSI-trust angle: a spurious PUSI on a bounded PID silently truncates
the pending packet with zero damage accounting. The module doc's damage list
(header lines 37-42) enumerates overrun but not underrun, so the doc is not
technically false — the silent short emit is nonetheless inconsistent with the
"recovered-and-counted" policy the file claims as its damage model.
What correct looks like: in complete_one, when `total_len` is Some and
`buf.len() < total`, either emit the truncation as a Recovered record or type it.
Confidence: HIGH — Verification: read + manual trace.

F-006 [S3]
Where: crates/demux/src/ts/fuzz.rs:4-7 (claim) + :37-46 (harness)
Code:
```
//! One call = one full untrusted-transport pass: construct, `start`,
//! drain every event to end-of-stream (budgeted) — the 188-byte
//! packet walk, PSI assembly (PAT/PMT/SDT), resync-on-garbage and the
//! PES timing model all ride that drain.
```
What is wrong: The fuzz harness constructs `TsDemuxer` and never calls
`set_pes_consumer` (rg: `pes: None` from `TsDemuxer::new`, ts/mod.rs:616), so
`pes_payload` is never delivered (ts/mod.rs:933 `if let Some(consumer)`) and NONE
of pes.rs rides the fuzz drain. The claimed fuzz coverage of "the PES timing
model" is false; pes.rs's panic-freedom under hostile input rests solely on the
synthetic unit vectors. (File is fuzz.rs's, filed here because it is pes.rs's
only hostile-input evidence story.) Confidence: HIGH — Verification: read + rg.

F-007 [S4]
Where: crates/demux/src/ts/pes.rs:592-613
Code:
```
        if let Some(prev) = self.last_pcr.replace(pcr).filter(|prev| pcr < *prev) {
            self.timing.push_back(TimingEvent::PcrRegressed {
```
What is wrong: No PCR wraparound handling: a legitimate 33-bit PCR wrap
(~26.5 h of stream) is flagged `PcrRegressed`. Event-only impact (and unreachable
in the shipped lane per F-001). Confidence: MEDIUM — Verification: read.

F-008 [S4]
Where: crates/demux/src/ts/pes.rs:137-147
Code:
```
pub fn short_header() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated {
                expected: 0,
                found: 0,
            },
```
What is wrong: `expected`/`found` are hard-zero placeholders (the function has no
inputs), so every `short-header` error carries fake truncation arithmetic; also
overloaded for the forbidden `PTS_DTS_flags == 01` case (:356) where "truncated"
is the wrong noun. Confidence: HIGH — Verification: read.

F-009 [S4]
Where: crates/demux/src/ts/pes.rs:415-420 vs crates/demux/src/ps/mod.rs:543-550
Code:
```
    pub fn presentation(&self) -> Result<Stamp, EngineError> {
        match self.pts {
            Some(t) => pts_stamp(t),
```
What is wrong: (a) `presentation()` is pub with exactly one caller — its own unit
test (pes.rs:1213); the shipped bridge uses free `pts_stamp` instead. Dead public
API. (b) Marker-bit policy fork inside one crate: pes.rs's `decode_ts5` REQUIRES
the three marker bits (drops the PES as `bad-pts` damage) while ps/mod.rs
deliberately does not (:544-546: "The trailing marker bit is NOT
enforced (ffmpeg writes both 0 and 1 — the pinned convention note)"). Same
container family, same stamp
fields, opposite strictness: a TS file with zero marker bits drops every PES; the
identical bytes in a PS file play. Confidence: HIGH — Verification: read + rg.

F-NC-1 [NO-TWIN — probe pattern check, hostile-focus item]
Where: crates/demux/src/ts/pes.rs:253-257, 313, 339-340 (all production indexing)
The Wave-4 twin (`ts/mod.rs:315-322`: `header.len() < PACKET_BYTES ||
header[PACKET_BYTES] == 0x47` — panics on an exactly-188-byte 0x47 file because
index 188 is out of bounds) has NO counterpart in pes.rs. Exhaustive indexing
scan: `bytes[0..3]` guarded by `len() < 6` short-circuit (:313); `bytes[7]/[8]`
guarded by `len() < 9` (:336); `bytes[9..]/[14..]` re-guarded inside decode_ts5
(:254); payload extraction uses non-panicking `buf.get(header_len..)` (:662-665).
Verification: read + rg index-pattern sweep. pes.rs is clean of the pattern.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint (private) | 119-121 | REAL | — | error ctor helper |
| bad_start_code | 125-132 | REAL | — | typed ctor, used |
| short_header | 137-147 | REAL | S4 | zeroed detail (F-008) |
| bad_pts | 151-158 | REAL | — | |
| truncated | 163-170 | REAL | — | honest expected/found |
| payload_over_ceiling | 173-182 | REAL | — | |
| time_base_unavailable | 186-193 | REAL | — | honest unreachable arm |
| PesStreamClass::has_optional_header | 226-228 | REAL | — | Video/Audio/Private1 per Table 2-16 |
| classify_stream_id | 234-247 | REAL | — | verified vs Table 2-18 incl. 0x00..=0xBB -> Reserved |
| decode_ts5 | 253-266 | REAL | — | 33-bit math exact; markers enforced (F-009b policy fork) |
| encode_ts5 (cfg test) | 271-280 | REAL | — | test-only inverse |
| PesHeader::parse | 312-369 | REAL (spec-defective) | S3 | F-003 (alignment byte), F-004 (data_len trust); bounds clean |
| mpeg_time_base | 376-378 | REAL | — | pub; no external caller (ts/mod.rs builds TimeBase directly) |
| pts_stamp | 381-385 | REAL | — | u64->i64 checked; used by shipped bridge |
| PesPacket::presentation | 415-420 | REAL-UNPROVEN (dead pub API) | S4 | only self-test calls it (F-009a) |
| Assembly::new | 475-484 | REAL | — | |
| PesAssembler::new | 502-511 | REAL | — | |
| push_packet | 524-584 | REAL | S3 | ceiling OK; F-002 disc overwrite; PUSI trust -> F-005 |
| observe_pcr | 592-613 | REAL-UNPROVEN (unwired) | S4 | no production caller (F-001); wrap (F-007) |
| complete_one | 618-692 | REAL | S3 | F-005 silent underrun; padding/damage paths OK |
| try_complete | 695-751 | REAL | — | overrun truncate + dormancy correct |
| finish | 760-791 | REAL | — | typed sticky truncation; short buffer stays put |
| next_pes / take_recovered / take_timing_events / damage_bytes | 794-813 | REAL | — | trivial accessors |
| Default | 816-820 | REAL | — | |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| stream_id_classification_table | YES | table pins classify_stream_id |
| ts5_roundtrip_and_marker_guard | YES | covers full 33-bit range + markers + wrong prefix + short slice |
| unbounded_video_assembles_across_packets | PARTIAL | assembly/hold path pinned; `data_alignment` assert enshrines F-003 |
| bounded_audio_completes_at_declared_length | YES | |
| tiny_bounded_pes_without_optional_header_completes | YES | 7-byte no-optional-header shape |
| length_overrun_is_recovered_damage | YES | |
| stray_continuation_before_any_start_is_counted | YES | |
| non_pes_pid_goes_dormant_instead_of_accumulating | YES | |
| truncated_bounded_pes_at_eos_is_typed | YES | pins expected 38 / found 12 |
| bad_pts_marker_is_typed_damage_midstream | YES | |
| forbidden_pts_dts_flags_refuse_typed | YES | flags 01 |
| header_swallowing_the_declared_packet_is_refused | YES | |
| discontinuity_surfaces_as_event_and_packet_flag | NO (for F-002) | only start-boundary case; continuation-disc clobber untested |
| dts_regression_and_jump_are_events | YES | pins both events + thresholds |
| pts_regression_without_dts_is_an_event... | YES | reorder-vs-damage distinction pinned |
| pcr_regression_is_an_event_and_discontinuity_resets | YES (but unwired) | tests a path no shipped code reaches (F-001) |
| payload_ceiling_is_the_typed_refusal | YES | feeds 1 MiB chunks until refusal |
| pts_stamp_maps_to_the_ninety_kilohertz_base | YES | base + ticks + nanos |
| error_kinds_are_stable_and_staged | YES | codes/stage/display pinned |
| doctest (lines 69-94) | YES | end-to-end assemble->finish->assert (run: see baseline; doctest binary not re-run — UNVERIFIED-executed, baseline 4744-passed includes demux doctests) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "transport layer ... hands this layer the discontinuity_indicator bit and PCR observations" | pes.rs:60-62 | FALSE (shipped build) | apps/cli ts.rs:255 passes `false`; no observe_pcr caller (rg) — F-001 |
| "PES_packet_length (0 = unbounded ...)" semantics | pes.rs:9-11 | REAL | try_complete/finish behavior + tests |
| "bounded PES complete at 6 + PES_packet_length" | pes.rs:16-17 | REAL | bounded_audio + overrun tests |
| "Accumulation is ceiling-checked ... typed ResourceLimit refusal" | pes.rs:48-50 | REAL | push_packet:567-577 + ceiling test (executed) |
| "PTS/DTS value is ticks of the MPEG clock ... TimeBase::new(1, 90_000)" | pes.rs:22-26 | REAL | pts_stamp test + apps/cli debug_assert (ts.rs:192-196) |
| "B-frame PTS reordering with a monotone DTS is not damage" | pes.rs:31-33 | REAL | pts_regression_without_dts test |
| "never silent" damage surfacing | pes.rs:30-31 | PARTLY FALSE | mid-stream bounded underrun is silent (F-005) |
| "errors with code demux.ts.pes.short-header mean 'hold' or 'damage'" | pes.rs:309-311 | REAL | try_complete hold vs complete_one drop |

## 7. Dead / duplicated / unreachable code
- `observe_pcr` + `TimingEvent::PcrRegressed` + `TimingEvent::DiscontinuityIndicator`
  emission at push_packet:531-534: UNREACHABLE in shipped build (F-001).
- `PesPacket::presentation`: pub, only self-test caller (F-009a).
- `PesHeader` here vs the separate `PesHeader` in ps/mod.rs:221: two independent
  PES parsers in one crate with divergent marker policy (F-009b) — duplication
  with contradictory behavior, flagged for the ps/mod.rs owner.

## 8. Dependencies & flags
- Uses media_types (error taxonomy, time), std collections. No feature gates; the
  module is unconditionally compiled (`pub mod pes;` ts/mod.rs:1206). No unsafe.
  No Cargo.toml changes needed. Reached via lumen-cli release lane (baseline
  cargo-build-release-lumen-cli.txt exit 0).

## 9. Open questions for the next agent
- ts/mod.rs owner: confirm whether `TsDemuxer` ever forwards adaptation-field
  discontinuity/PCR to the PesConsumer seam in any lane (I found none —
  F-001). The Wave-4 `probe` panic twin (mod.rs:315-322) is yours: it panics on an
  exactly-188-byte 0x47-led header; pes.rs has no twin (F-NC-1).
- fuzz.rs owner: harness doc claims the PES timing model rides the drain (F-006).
- apps/cli ts.rs owner: `mint_adts_aus` ignores `data_alignment` and
  `discontinuity` is constant-false in `mint_packet` — the per-packet
  discontinuity plumbing end-to-end is dead in production.
