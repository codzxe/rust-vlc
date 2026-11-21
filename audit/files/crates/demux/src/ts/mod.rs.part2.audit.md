# AUDIT: crates/demux/src/ts/mod.rs#part2 (lines 604–1206)
| Field | Value |
|---|---|
| Auditor | W05-A10 |
| Date | 2026-11-21 |
| Lines / functions | 603 (of 1206; part2 = 604–1206, part1 = 1–603 ends inside `TsDemuxer::new`'s field-init) / 27 units |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/host.rs` open_session → `apps/cli/src/demux_host.rs:open_source` → `apps/cli/src/containers/mod.rs:72` `"mpegts" => ts::open` → `apps/cli/src/containers/ts.rs:110-111` `TsDemuxer::new`+`start()` → `Demuxer::{start,next_event,seek,seekability}` (this range) driven by `crates/playback-engine/src/pipeline.rs:2141` `self.demuxer.next_event()`) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 3 S4: 4 |
| Commands run | `rg -n "pub trait Demuxer" crates/` (trait contract, plugin-api/src/demux.rs:52-121); `rg -n "mpegts" apps/cli/src/containers/mod.rs` (:72 dispatch); `rg -n "seekability|demuxer_reanchor" crates/playback-engine/src/` (session.rs:2021,2784 gate seeks on `seekability() != None`; pipeline.rs:1499 reanchor — engine never calls `seek`); `rg -n "observe_pcr|last_pcr" crates/ apps/` (PesAssembler::observe_pcr called only from pes.rs's own tests; `TsDemuxer::last_pcr()` zero callers; `damage_total` zero callers); `sed -n` on every quoted range; `rg -n "DamageLog|fn note" crates/demux/src/avi/header.rs` (no cap on events Vec); `rg -n "push_packet" -A 60 crates/demux/src/ts/pes.rs` (non-start payload appended unconditionally); baseline greps below |

## 1. One-paragraph verdict
This range is the whole MPEG-TS state machine — the PSI collection scan
(`start`), the 188-byte packet walk (`next_event`/`process_packet`), the
section-assembler application, the program/track surface, and the honest
typed refusals — and the core of it is real, executed code: 49 `ts::` unit
tests and 3 `containers::ts` CLI-path tests pass in the baseline
(`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:1861-1909,2835-2837,1955`),
including a corpus lane through the real open path. The defects are in the
seams the careful design did not close: duplicate continuity-counter packets
are counted as "tolerated" but **re-delivered** to the PES seam (corrupting
PES assembly — the exact broadcast-splice scenario the policy cites), the
damage ledger grows without bound from file-controlled per-packet notes, the
trait-pure `seek` query permanently poisons the demuxer via the sticky
mechanism, the pre-`start` refusal likewise bricks the demuxer permanently,
and an SDT-less (spec-legal) stream forces every open to walk the full 4 MiB
PSI window because `psi_complete` hard-requires the SDT. None of these are
covered by the test suite: every relevant test asserts the typed error code
or a counter and never re-calls the demuxer afterwards.

## 2. What actually works (verified)
- 49 `ts::` tests ok, incl. `bad_sync_resyncs_with_recovery_event`,
  `crc_damaged_psi_is_dropped_not_fatal`, `partial_tail_packet_is_a_note_not_an_error`,
  `multi_program_surface`, `pes_seam_receives_payloads_and_errors_propagate` —
  `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:1861-1909`;
  demux crate suite `462 passed; 0 failed` (:1955).
- Shipped CLI lane: `containers::ts::tests::corpus_ts_lanes_through_the_open_path ... ok`
  and `synthetic_lane_opens_demuxes_and_refuses_seek_typed ... ok` (:2835-2837)
  — TS opens, demuxes, and refuses seek typed through the real `open_source` path.
- Bounded recovery is genuinely bounded: open scan ≤ `MAX_OPEN_SCAN_BYTES`
  (256 KiB, :91/:698), PSI scan ≤ `MAX_PSI_SCAN_BYTES` (4 MiB, :84/:1057),
  per-event resync ≤ `MAX_RESYNC_SCAN_BYTES` (2048, :88/:1007-1008) — verified by read.
- `start()`'s scan→reset→replay design is coherent: stats/damage/pending/psi/cc
  reset (:1091-1101), walk rewound to head so the PES seam (registered by the
  CLI host after `start`) sees the whole stream — traced through
  `apps/cli/src/containers/ts.rs:110-153` (start :111 → facts → `set_pes_consumer` :153 → wrap).
- The `start()` reset correctly re-notes the head resync (:1094-1098) so the
  replay's first events reproduce the scan's damage truth — verified by read.

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/ts/mod.rs:855-877 (policy) and 930-938 (delivery)
Code:
```rust
            if let Some(last) = expected {
                if parsed.continuity_counter == last {
                    self.stats.cc_duplicates += 1;
                } else if parsed.continuity_counter != (last + 1) & 0x0F {
```
```rust
        } else if self.es_pids.contains(&parsed.pid) {
            self.stats.pes_packets += 1;
            self.stats.pes_bytes += parsed.payload.len() as u64;
            if let Some(consumer) = self.pes.as_mut()
```
What is wrong: A duplicate CC (same PID, same counter, payload-bearing —
ISO/IEC 13818-1 §2.4.3.2's retransmission case) is counted and then
**execution falls through to the ES delivery branch**, which hands the same
payload slice to the PES consumer a second time. Nothing drops it: the
`if/else if` only counts, and the `cc.entry(...).last = Some(...)` update at
:876 stores the same value. Downstream, `PesAssembler::push_packet`
(crates/demux/src/ts/pes.rs:573-580) appends a non-start payload
unconditionally (`state.buf.extend_from_slice(payload)`), so a duplicated
continuation fragment is appended twice to the partial PES (declared-length
mismatch / corrupted packet), and a duplicated PUSI packet mints the same
PES packet twice (duplicate frame).
Why it matters: The module doc (:23-27) sells the policy as "tolerant +
damage-noted… broadcast streams splice and PID-tunnel" — i.e. precisely the
lossy-broadcast-capture scenario where duplicated TS packets occur. In that
realistic case the "tolerance" corrupts every downstream PES. ffmpeg drops
same-CC duplicates; this demuxer counts and re-delivers them.
What correct looks like: On `parsed.continuity_counter == last` with payload,
skip delivery of this packet (count it) — or byte-compare and skip only true
retransmissions.
Confidence: HIGH — Verification: read (fall-through traced :855→:930;
assembler append traced pes.rs:573-580); the shipped lane registers a real
consumer (`apps/cli/src/containers/ts.rs:153` `set_pes_consumer`).
Note: `ts::tests::continuity_jump_and_duplicate_are_tolerant_and_counted`
(tests.rs:513-531) asserts only `cc_duplicates >= 1` / `cc_jumps >= 1` with
no consumer registered — it cannot fail under double-delivery.

### F-002 [S2]
Where: crates/demux/src/ts/mod.rs:838-842 (and :684-686, :1018)
Code:
```rust
            Err(_) => {
                // Adaptation field overruns the packet: damage-noted,
                // the walk continues at the next packet boundary.
                self.note(PACKET_BYTES as u64, "ts: bad packet header skipped");
                return;
            }
```
What is wrong: `note()` (→ `DamageLog::note`, crates/demux/src/avi/header.rs:397-400)
pushes an entry into `damage.events` with **no cap and no drain API**
(`damage_events()` at :666 is `&self` read-only; nothing ever truncates). This
branch can fire once per 188-byte packet: a file that passes the open probe
(`0x47` at 0 and at 188) whose every packet declares an adaptation-field
length > 183 (`h[4]=0xFF`, AFC bit set — `TsPacket::parse` :251-255 refuses)
drives one `note()` per packet for the file's entire length. A 2 GiB file =
~11.3 M entries × 16 B/tuple ≈ 180 MiB of irrecoverable ledger growth —
unbounded allocation from a file-controlled count. The resync path (:1018,
one note per ≤2048 B) amplifies the same ledger on recoverable-garbage files.
Why it matters: The demuxer's entire design vocabulary is "bounded
recovery" (three MAX_* constants), but the recovery *ledger* itself is
unbounded — a resource-exhaustion vector reachable from a probe-passing
malformed file, with no ceiling anywhere in the crate (rg found no
`events.len()`/truncate cap).
What correct looks like: A cap on `DamageLog::events` (keep the first N,
count the rest into `total`), matching the crate's own bounded-recovery
doctrine.
Confidence: HIGH — Verification: read (no cap exists; per-packet fire path
traced); the OOM threshold is machine-dependent (magnitude, not mechanism).

### F-003 [S3]
Where: crates/demux/src/ts/mod.rs:1154-1164
Code:
```rust
    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        Err(self.fail(mint(
            ErrorKind::Unsupported {
```
What is wrong: `self.fail()` (:679-682) sets `self.sticky`. The trait contract
for `seek` is a **pure query** — "Does **not** move the read position: the
engine decides whether to act" (crates/plugin-api/src/demux.rs:36-38, 70-75),
and invariant D4's discipline (:100-103) is that an `Err` leaves the demuxer
in a valid state. Here one `seek()` call poisons the demuxer permanently:
every later `start()` (:1033-1035 checks sticky first) and `next_event()`
(:1107-1109) replays `demux.ts.seek.unsupported` forever. The shipped engine
never calls it (session.rs:2021/2784 gate on `seekability() != SeekSupport::None`,
and the engine calls `reanchor`, not `seek`), and the one shipped CLI caller
(`apps/cli/src/containers/ts.rs:127`, `--start-time` > 0) aborts the open on
the propagated error — so today it is latent. But the ts.rs:127 call happens
on a live, started demuxer: if that error were ever swallowed there (or any
host uses the documented "validate targets before a session exists" query),
playback is bricked by a query.
Why it matters: A documented-pure query mutating permanent state is a
contract violation that converts an honest refusal into a state-machine kill.
What correct looks like: `Err(mint(...))` without `self.fail` — matching the
AVI demuxer's pure seek query (avi/mod.rs:947-954, position restored).
Confidence: HIGH — Verification: read (sticky interplay traced :679-682,
:1033-1035, :1107-1109); callers traced (ts.rs:127; engine gating verified).
Note: `ts::tests::seek_refuses_typed` (tests.rs:664-676) asserts the error
code and never re-calls the demuxer — it passes under the poisoning.

### F-004 [S3]
Where: crates/demux/src/ts/mod.rs:1110-1112 and 1145-1148
Code:
```rust
        if !self.started {
            return Err(self.fail(not_started()));
        }
```
```rust
            if packet[0] != 0x47 {
                self.resync()?;
                continue;
            }
```
What is wrong: Two inconsistent stickiness bugs. (a) `next_event` before
`start` routes through `self.fail` → sticky → a demuxer whose `start()` was
never called is **permanently** dead; `start()` at :1033 now returns the
replayed `not_started` error forever even though starting was still possible.
(Same pattern W04-A07 flagged as AVI F-003 — the crate's policy is
inconsistent across demuxers.) (b) The inverse: mid-stream unrecoverable
sync loss returns `resync_failed` **without** `fail()` — non-sticky — and
`self.walk` never advances, so every subsequent `next_event` re-reads the
same non-sync packet, re-scans 2048 bytes, and re-errors: the demuxer can
never reach `EndOfStream` and each call pays the rescan. Unrecoverable
damage gets the *weaker* treatment than an operational ordering mistake.
Why it matters: Contract-level state-machine inconsistency; an engine retry
loop (or any error-recovery flush path) spins on (b) forever without EOS.
What correct looks like: One policy: operational-order errors non-sticky
(or sticky in every demuxer), unrecoverable container damage sticky via
`fail()` with the walk advanced to the failure point.
Confidence: HIGH — Verification: read (both paths traced; contrast :679-682).
Note: `ts::tests::next_event_before_start_is_typed` (tests.rs:679-685) and
`unresyncable_garbage_is_typed` (tests.rs:448-462) assert the codes once and
never re-call — both pass under either behavior.

### F-005 [S3]
Where: crates/demux/src/ts/mod.rs:945-953 (with :797-798, :84, :1057-1058)
Code:
```rust
    fn psi_complete(&self) -> bool {
        self.transport_stream_id.is_some()
            && self.original_network_id.is_some()
```
```rust
            PsiSection::Sdt(sdt) => {
                self.original_network_id = Some(sdt.original_network_id);
```
What is wrong: `original_network_id` is set **only** by the SDT (:798), and
`psi_complete` hard-requires it (:947). The module doc (:35, part1) says
"Missing SDT is legal (names stay `None`)" — and it is legal at exit, but
the scan's early-exit can never fire without an SDT: an SDT-less stream
(legal TS; many hardware/IP encoders omit it) walks the **entire**
`MAX_PSI_SCAN_BYTES` = 4 MiB window (:84, :1057-1058) on every open — ~22 K
packets, each a fresh 188-byte `Vec` allocation (`read_packet` :995-1001 →
`read_prefix_at`), where the documented intent is "the corpus carries all
three in the first three packets" (:34-35).
Why it matters: Every open of an SDT-less capture pays a full 4 MiB scan
(plus the reset-and-replay walk that follows), a 1000×+ deviation from the
sub-second design intent — a realistic-input performance defect.
What correct looks like: Completeness = PAT + all PAT-declared PMTs; treat
SDT as opportunistic (a time-boxed sub-scan, or accept first-SDT-if-any
within the window).
Confidence: HIGH — Verification: read (only-writer traced :798; loop bound
traced :1057-1058).

### F-006 [S4]
Where: crates/demux/src/ts/mod.rs:645-675
Code:
```rust
    /// The most recent raw PCR seen on any PID.
    #[must_use]
    pub fn last_pcr(&self) -> Option<Pcr> {
        self.last_pcr
    }
```
What is wrong: The observability surface is wired to nothing in shipped
builds: `last_pcr()` has **zero callers anywhere** (the state it exposes is
maintained at :878-881 for nothing; the shipped TS lane never forwards PCR —
the CLI host never calls `PesAssembler::observe_pcr`, whose only callers are
pes.rs's own tests), `damage_total()` (:660) has zero callers, and
`stats()`/`damage_events()`/`transport_stream_id()`/`original_network_id()`
are called only from `crates/demux/src/ts/tests.rs`. The module doc's
"surfaced with… the **raw** PCR" (:19-21) is a surface with no consumer.
Why it matters: Dead public API that reads as capability (PCR-based timing
exists in pes.rs as `PcrRegressed` events — unfed in the shipped lane); the
"damage log (observability)" is unobservable in any shipped binary.
What correct looks like: Either wire the accessors into the CLI/engine
telemetry or delete them; the PCR regression detector needs a feeder.
Confidence: HIGH — Verification: read + `rg` over crates/ apps/ (zero /
test-only callers, cited above).
Severity note: S4 because each accessor is trivially correct; the unwired
capability claim is the debt.

### F-007 [S4]
Where: crates/demux/src/ts/mod.rs:703-719
Code:
```rust
            if head[at] == 0x47 {
                let next = at + PACKET_BYTES;
                if next >= head.len() {
                    if at == 0 {
                        return Ok(Some(0)); // single-packet stream
                    }
                    break;
                }
```
What is wrong: The "single-packet stream" acceptance requires `at == 0`
(and implicitly `head.len() ≤ 188`). A file of 189–375 bytes containing one
complete valid packet plus a truncated tail (e.g. a 200-byte .ts fragment),
or a single full packet at a nonzero head offset with the next stride past
EOF, never proves the stride and returns `Ok(None)` → `not_ts` refusal —
despite a complete, walkable packet existing.
Why it matters: Tiny blast radius (degenerate fragment files), but the
refusal misclassifies a damaged-but-partially-valid TS as "not a transport
stream".
What correct looks like: Accept any head offset where one full 188-byte
packet fits before EOF (the walk's resync will tell the rest), consistent
with the probe's "a whole 188-byte file is also accepted" escape (:320).
Confidence: HIGH — Verification: read (bounds traced for 188 < len ≤ 375).

### F-008 [S4]
Where: crates/demux/src/ts/mod.rs:995-1001
Code:
```rust
    fn read_packet(&mut self) -> Result<Option<Vec<u8>>, EngineError> {
        let bytes = self.reader.read_prefix_at(PACKET_BYTES, self.walk)?;
```
What is wrong: One fresh heap `Vec<u8>` (188 B) allocated per packet for the
entire stream lifetime (≈11 M allocations for a 2 GiB file) — the
crate-wide `read_prefix_at` pattern, but TS is the most packet-dense format
in the crate.
Why it matters: Per-packet allocation in the hottest demux path; hygiene
(the AVI/WAV demuxers share the pattern, so this is a crate-level choice,
not a local one).
What correct looks like: A reusable 188-byte scratch buffer
(`read_prefix_into` exists precisely for that, riff.rs:357-369).
Confidence: HIGH — Verification: read (allocation traced through
`read_prefix_at` riff.rs:307-312: `vec![0u8; len]` per call).

### F-009 [S4]
Where: crates/demux/src/ts/mod.rs:622-624 (with :930-938, :1085-1101)
Code:
```rust
    pub fn set_pes_consumer(&mut self, consumer: Box<dyn PesConsumer>) {
        self.pes = Some(consumer);
    }
```
What is wrong: If a consumer is registered **before** `start()` (the API
permits it; the trait doc only says "until it is registered… emits
nothing"), the PSI scan delivers every post-PMT ES payload to it
(:930-938 runs inside `start`), and then the post-scan reset rewinds
`walk` to head (:1099) and `next_event` **re-delivers the same payloads**
— a double-delivery API-order hazard, undocumented. The shipped CLI host
avoids it only by ordering (ts.rs registers after `start`).
Why it matters: A public seam whose correctness depends on an unstated call
order.
What correct looks like: Document the order requirement, or suppress PES
delivery until `started` (a flag checked at :930).
Confidence: HIGH — Verification: read (scan-time delivery traced; reset
traced).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `TsDemuxer::new` (tail) | 610-619 | REAL | — | field-init tail; head/walk start 0; no panics |
| `set_pes_consumer` | 622-624 | REAL | S4 (F-009) | shipped caller ts.rs:153 (bridge registration); order hazard |
| `programs` | 629-631 | REAL | — | shipped caller ts.rs:142 (PES routing) |
| `transport_stream_id` | 635-637 | REAL | S4 (F-006) | test-only callers |
| `original_network_id` | 641-643 | REAL | S4 (F-006) | test-only callers |
| `last_pcr` | 647-649 | REAL | S4 (F-006) | zero callers; state maintained for nothing |
| `stats` | 653-655 | REAL | S4 (F-006) | test-only (drain helper tests.rs:277) |
| `damage_total` | 660-662 | REAL | S4 (F-006) | dead code, zero callers |
| `damage_events` | 666-668 | REAL | S4 (F-006) | test-only (tests.rs:570) |
| `track_of_pid` | 673-675 | REAL | — | shipped caller ts.rs:144 (bridge routing) |
| `fail` | 679-682 | REAL | S3 (F-003/F-004) | the sticky setter — poison point for both |
| `note` | 684-686 | REAL | S2 (F-002) | unbounded ledger push, no cap/drain |
| `is_psi_pid` | 690-692 | REAL | — | PAT/CAT/NIT/SDT-BAT + discovered PMT PIDs |
| `open_sync` | 696-720 | REAL | S4 (F-007) | ≤256 KiB window; single-packet edge F-007; no panics |
| `apply_section` | 723-831 | REAL | — | PAT/PMT/SDT application idempotent under replay (find-by-number); PMT-without-PAT damage-noted (:786-794); `pending_services` grows only within scan window; malicious PAT PIDs (null/PAT) harmless (earlier filters at :850/:691) |
| `process_packet` | 834-942 | REAL | S2 (F-001, F-002) | CC policy counts duplicates but re-delivers (F-001); bad-header note per packet (F-002); consumer error → sticky via fail (:937) — the documented propagation; partial-drop accounting (:893-914) traced correct incl. CC-jump clear interplay (no double count) |
| `psi_complete` | 945-953 | REAL | S3 (F-005) | SDT hard-requirement drives 4 MiB scans |
| `build_tracks` | 957-992 | REAL | S4 | shared-ES-PID routing overwrite (last program wins) — unusual input; TimeBase 1/90000 direct-construct bypasses `TimeBase::new` (documented, DVB-builder precedent); TrackId monotonic u32 |
| `read_packet` | 995-1001 | REAL | S4 (F-008) | fresh Vec per packet |
| `resync` | 1005-1028 | REAL | — | ≤2048 B window; `walk + 2048` cannot overflow; stats + `Recovered` event on success; typed failure |
| `Demuxer::start` | 1032-1104 | REAL | S3 (F-005) | sticky-first; idempotent after start (:1036-1038); scan bounded; reset+replay coherent (see §2); `no_pat` sticky via fail — correct (unrecoverable) |
| `Demuxer::next_event` | 1106-1152 | REAL | S3 (F-004) | pre-start sticky poison; resync error non-sticky + walk never advances (F-004b); EOS tail: `length - walk` cannot underflow (walk ≤ length invariant); partial-tail note + psi partial counts correct |
| `Demuxer::seek` | 1154-1164 | REAL | S3 (F-003) | typed refusal but sticky poison; engine never calls (gated); CLI ts.rs:127 aborts |
| `Demuxer::seekability` | 1166-1168 | REAL | — | `None`; engine gates on it (session.rs:2021,2784) |
| `language_of` (free fn) | 1173-1192 | REAL | — | bounds-checked descriptor walk; first 0x0A wins — doc says "first well-formed one wins" but a malformed first 0x0A returns None instead of continuing (doc/code drift, ≤S4) |
| `is_eit` (free fn) | 1196-1198 | REAL | — | 0x4E–0x6F per EN 300 468 §5.2.4 — correct |
| `pub mod pes;` + seam note | 1199-1206 | REAL | — | module wiring; I1b/I1a merge note for the Lead |

## 5. Test honesty (tests live in ts/tests.rs — A02's assignment; assessed only against this range)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `continuity_jump_and_duplicate_are_tolerant_and_counted` | NO (for F-001) | asserts only `cc_duplicates >= 1`/`cc_jumps >= 1`; no consumer registered, so double-delivery is invisible; mutation "deliver duplicates twice" leaves it green |
| `seek_refuses_typed` | NO (for F-003) | asserts error code once; never re-calls `next_event`/`start` after the refusal, so sticky poisoning is invisible |
| `next_event_before_start_is_typed` | NO (for F-004a) | asserts the code; never verifies `start()` still works afterwards (it does not) |
| `unresyncable_garbage_is_typed` | NO (for F-004b) | asserts the code once; never re-calls — the stuck non-sticky loop is invisible |
| `bad_sync_resyncs_with_recovery_event` / `partial_tail_packet_is_a_note_not_an_error` / `multi_program_surface` / `pes_seam_receives_payloads_and_errors_propagate` | YES (for their subjects) | assert concrete events/counters on constructed streams; mutation of resync stats, tail notes, program surface, or error propagation would flip them (all `ok` in baseline :1861-1909) |
| `containers::ts::tests::corpus_ts_lanes_through_the_open_path` / `synthetic_lane_opens_demuxes_and_refuses_seek_typed` / `start_time_is_refused_typed_at_open` | YES (for the open lane) | drive the real CLI `open_source` path; a broken `start`/`next_event` walk or lane refusal fails them (baseline :2835-2837) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Continuity policy: tolerant + damage-noted. A CC that repeats is a tolerated duplicate (counted)" | mod.rs:23-27 (part1 module doc) | SHALLOW | counted (true, :859) but re-delivered to the PES seam (F-001) — "tolerated" only at the counter layer |
| "Missing SDT is legal (names stay `None`)" | mod.rs:35 | REAL | exit path proceeds (:1072-1085); but the 4 MiB scan cost is unbudgeted (F-005) |
| "Seeking: TS has no container index… `SeekSupport::None` and a typed refusal, never a fake landing" | mod.rs:52-54 | REAL | refusal typed (:1154-1164), seekability None (:1166-1168), engine gating verified; the refusal poisons (F-003) |
| "the pacing math is the engine's, not the demuxer's" (raw PCR surfaced) | mod.rs:19-21 | SHALLOW | PCR parsed and stored but zero consumers in any shipped lane (F-006); pes.rs `PcrRegressed` detector unfed |
| "until it [PES consumer] is registered the demuxer counts the bytes and emits nothing (no fake packets, ever)" | mod.rs:44-48 / :445-450 | REAL | verified :930-938 counts-then-delivers; order hazard noted (F-009) |
| demux suite health | baseline INDEX #3 | REAL | 462 passed / 0 failed (no-failfast log :1955), incl. 49 `ts::` |

## 7. Dead / duplicated / unreachable code
- `damage_total()` (:660-662): zero callers anywhere — dead.
- `last_pcr()` + `last_pcr` state (:647-649, :878-881): zero callers — dead in
  shipped builds.
- `stats()`/`damage_events()`/`transport_stream_id()`/`original_network_id()`:
  test-only callers.
- `bad_packet()` (:143-152, part1): the Err variant of `TsPacket::parse` is
  consumed only to be discarded at :838 (`Err(_) =>`) — the typed error's
  offset is thrown away; near-vestigial, noted not dead.
- Duplication: `StickyShape` (:462-481) is the crate-wide WAV/Ogg/ADTS/AVI
  pattern (copy #5+); `open_sync` (:696-720) duplicates `probe`'s (:315-322)
  stride logic with slightly different edge rules.

## 8. Dependencies & flags
- No feature gates on this module (`pub mod ts;` unconditional, lib.rs:54);
  compiled into every shipped build; CLI dispatch at containers/mod.rs:72.
- Cross-crate reuse from the AVI demuxer: `crate::avi::header::DamageLog`,
  `crate::avi::riff::SourceReader` (:69-70) — layering note: every demuxer
  depends on avi's primitives (S4-pattern already recorded by W04).
- No unsafe, no unwrap/expect in this range (workspace lints inherit; clippy
  clean per baseline INDEX #5/#6).

## 9. Open questions for the next agent
- ts/pes.rs auditor (A04/A05): `PesAssembler::push_packet` appends non-start
  payloads unconditionally — confirm no PID-level de-dup exists anywhere in
  the seam (I traced none); F-001's blast radius ends there.
- ts/tests.rs auditor (A02): the `drain` helper `unreachable!`s on any walk
  error (tests.rs:274) — it structurally cannot observe F-004b (stuck
  non-sticky error loop); worth a row in that audit.
- Whoever owns avi/header.rs (W04, done): DamageLog cap (F-002) is a
  cross-demuxer fix point; my note is the TS amplification only.
- The `pes` seam note (:1199-1205) references the Lead's "I1 unification" —
  cross-check with other demuxers' merge notes for drift.
