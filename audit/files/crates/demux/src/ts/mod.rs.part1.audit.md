# AUDIT: crates/demux/src/ts/mod.rs (part1: lines 1-619 of 1207)
| Field | Value |
|---|---|
| Auditor | W05-A09 |
| Date | 2026-11-21 |
| Lines / functions | 619 / 23 items (consts ×12, fns ×7, structs ×8, trait ×1, module docs) |
| File verdict (part1) | REAL |
| Reachable from shipped entry point? | YES — `lumen` CLI → `apps/cli/src/containers/ts.rs:110` `TsDemuxer::new` → `:111` `start()` → `:153` `set_pes_consumer(BridgeConsumer)`; executed green: `containers::ts::tests::corpus_ts_lanes_through_the_open_path ... ok` (orchrun-test-workspace-nofailfast.log.txt:2835-2837). EXCEPTION: `probe()` itself has NO shipped caller (grep: tests only) — UNREACHABLE as shipped, see F-001 |
| Findings | S0: 0 S1: 0 S2: 2 S3: 4 S4: 3 |
| Commands run | full Read of all 1207 lines; `rg "TsDemuxer|ts::probe"` repo-wide → callers: apps/cli/src/containers/ts.rs (shipped), demux tests ×3, streaming tests ×2; `rg "TsPacket|set_pes_consumer|PesConsumer"` → parse's only production caller is mod.rs:836 (188-byte guaranteed); `rg "probe\("` in ts/ → only ts/tests.rs:250-254; sed -n verification of every quote; baseline log greps (ts::tests/ts::pes/ts::fuzz suites all ok, test-workspace.log.txt:1902-1941; ts corpus lanes ok, :2240-2257) |
| Chunk boundaries | part1 = 1-619 (through `TsDemuxer::new`); the assignment's nominal cut 603 falls mid-constructor — I audited through the end of `new` (619). part2 (A10) = 620-1207 (`set_pes_consumer` at 622 through the `Demuxer` impl, `language_of`, `is_eit`, the I1b seam note). `process_packet`/`next_event`/`start` internals are A10's; my findings there are doc-claims filed against part1 lines with cross-refs |

## 1. One-paragraph verdict
The packet layer is real and unusually careful: the 188-byte header parse, adaptation-field bounds check (5+length ≤ 188, typed `demux.ts.bad-packet`), PCR bit extraction (33+9 bits, verified against §2.4.4.9), the bounded section assembler (12-bit length ceiling against `MAX_SECTION_BYTES` = 4098, pointer-field guard), and the sticky-error precedent are all implemented and executed green across 3 TS test suites in the baseline. The defects are at the edges: `probe()` panics on exactly-188-byte input while its own comment claims acceptance (and the same bug ships in the live media-probe mirror, where the baseline proved a shipped-binary panic, exit 101); the `PesConsumer` trait doc promises error propagation and continued usability that the implementation does not deliver (sticky poison + error deferred until EOS); duplicate-CC packets are re-delivered instead of discarded; and the damage ledger grows without bound for the whole walk on the shipped open path while two doc comments call it bounded.

## 2. What actually works (verified)
- `TsPacket::parse` (245-303) bit math verified against ISO/IEC 13818-1 §2.4.3: PID = bits of h[1]&0x1F + h[2]; afc = h[3]>>4&3; adaptation `end = 5 + length` guarded `end > PACKET_BYTES` → typed error; PCR base `f[1]<<25|f[2]<<17|f[3]<<9|f[4]<<1|f[5]>>7` + extension `(f[5]&1)<<8|f[6]` is the correct 33+9-bit layout; flags guarded by `!f.is_empty()`. — verified by read.
- Adaptation-only packets yield empty payload (`afc & 0b01 == 0` → `&h[at..at]`), no out-of-range slice. — verified by read.
- `SectionAssembler::feed` (501-546): PUSI pointer guard `1 + ptr > payload.len()` → drop; 12-bit declared length `3 + ((buf[1]&0x0F)<<8 | buf[2])` capped at `MAX_SECTION_BYTES` (psi.rs:53 `3 + 4095`); completion via `std::mem::take` resets cleanly; growth is bounded to `declared` bytes — allocation is input-bounded by a real ceiling. — verified by read + psi.rs:53.
- Continuity arithmetic `(last + 1) & 0x0F` wraps correctly; CC only checked for payload-bearing packets (§2.4.3.2). — verified by read (855-876).
- `TsDemuxer::new` doc claim (refuses unknown-length/unseekable/empty at stage `Open`) — verified: `SourceReader::new` (avi/riff.rs:219-236) returns typed `demux.source.unknown-length` / `unseekable` / `empty`.
- Executed evidence: 40 ts unit tests ok (test-workspace.log.txt:1902-1941), `ts_fuzz_survival` ok (:2245), `corpus_ts_pes_seam_accounting` / `corpus_ts_programs_streams_and_names_match_ffprobe` / `corpus_ts_lanes_match_the_ffprobe_truth` ok (:2240-2257), shipped-lane tests `containers::ts::tests::*` ok (nofailfast:2835-2837), HLS e2e `ts_lane_streams_byte_exact_and_demuxes_over_the_session` ok (nofailfast:5215).
- "No fake packets are ever emitted" (module doc 47-48): grep of the file — the demuxer only ever queues `DemuxEvent::Discontinuity`/`Recovered`; no `DemuxEvent::Packet` construction exists in ts/mod.rs. TRUE.

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/ts/mod.rs:314-322
Code:
```rust
pub fn probe(header: &[u8]) -> bool {
    if header.is_empty() || header[0] != 0x47 {
        return false;
    }
    // A lone sync byte proves nothing; a second at the stride does
    // (a whole 188-byte file is also accepted — the walk will tell).
    header.len() < PACKET_BYTES || header[PACKET_BYTES] == 0x47
}
```
What is wrong: `probe(&[0x47; 188])` panics. `header.len() < PACKET_BYTES` is `188 < 188` = false, so `header[PACKET_BYTES]` = `header[188]` indexes out of bounds on a len-188 slice — exactly the "whole 188-byte file" the comment claims "is also accepted". The accepted case is len ≤ 187. The comment is FALSE and the function panics where it documents acceptance.
Why it matters: (a) it is a `pub fn` that panics on valid-ish input with a doc comment asserting the opposite; (b) the shipped binary demonstrates this exact defect class: the live sniff is media-probe's mirror `TsSignature` (phase5.rs:160-163), same shape, and the baseline run `orchrun-ts188-panic-repro.log.txt` shows `lumen open file:///tmp/g188.bin` on a 188-byte 0x47 file panicking (`index out of bounds: the len is 188 but the index is 188`, phase5.rs:161) with EXIT=101; (c) the false comment already misled an audit — the phase5 audit file claims "the demuxer-side TS probe accepts a single-packet file per its own design", citing this comment; it does not, it panics identically. In-tree `probe` callers are tests only (ts/tests.rs:250-254, none exactly-188), so the panic is latent here — but the twin shipped.
What correct looks like: `header.len() <= PACKET_BYTES || header[PACKET_BYTES] == 0x47` (or an explicit `get(PACKET_BYTES)`), plus the same fix in phase5.rs — and a round-trip test pinning the exactly-188 boundary both sides claim to own.
Confidence: HIGH — Verification: read + traced; panic proven for the mirror by baseline run orchrun-ts188-panic-repro.log.txt.

### F-002 [S3]
Where: crates/demux/src/ts/mod.rs:451-458 (trait doc), implementation at 933-938 (part2, cross-ref)
Code:
```rust
pub trait PesConsumer: Send {
    /// One TS packet's PES payload slice.
    ///
    /// # Errors
    /// The consumer's own typed error, propagated through
    /// `next_event` (the demuxer remains usable).
    fn pes_payload(&mut self, pid: u16, pusi: bool, payload: &[u8]) -> Result<(), EngineError>;
}
```
What is wrong: both halves of the `# Errors` promise are false. (1) "the demuxer remains usable": the consumer's error goes through `fail()` → `self.sticky = Some(...)` (679-682) — sticky is never cleared, so every later `next_event` returns that error forever: the demuxer is permanently poisoned, not usable. (2) "propagated through `next_event`": in `next_event`'s loop (1113-1151) sticky is checked only at fn entry; the error is `let _ = self.fail(err);`-discarded (937) and the loop keeps walking packets — the error cannot surface until the stream hits EOS, i.e. one `next_event()` call silently consumes the entire remaining file, feeding every subsequent packet to the already-failing consumer. The shipped lane registers a real consumer (apps/cli/src/containers/ts.rs:153) whose error paths include `pts_stamp` failures (mint_packet, ts.rs:437-443), so this is reachable in the shipped `lumen open` path, not hypothetical. Also unpinned: the test named `pes_seam_receives_payloads_and_errors_propagate` (ts/tests.rs:625-668) never returns an error from its Collector — the "and errors propagate" half of its name tests nothing.
Why it matters: an API contract doc that invert-describes its implementation is worse than none; a caller trusting "remains usable" will build recovery on a poison pill, and the deferred-to-EOS swallow turns a fail-fast demux error into a whole-file walk plus a spurious `EndOfStream` before the error ever surfaces.
What correct looks like: either honor the doc (surface the error from the current `next_event` call, leave the demuxer usable) or fix the doc to state sticky-permanent poisoning and EOS-deferred surfacing; rename or extend the test to actually drive a failing consumer.
Confidence: HIGH — Verification: read (both sites); shipped-consumer error paths read at apps/cli/src/containers/ts.rs:423-429, 436-455; test body read at ts/tests.rs:625-668.

### F-003 [S3]
Where: crates/demux/src/ts/mod.rs:245-248
Code:
```rust
    pub fn parse(bytes: &[u8]) -> Result<TsPacket<'_>, EngineError> {
        let h = bytes;
        let pid = (u16::from(h[1] & 0x1F) << 8) | u16::from(h[2]);
        let afc = h[3] >> 4 & 0x03;
```
What is wrong: `parse` is `pub` and indexes `h[1]`, `h[2]`, `h[3]` unconditionally (and `h[4]` when afc bit 1 is set, 252) with no length precondition — a slice shorter than 4 bytes panics; a 4-byte slice with the adaptation bit panics at `h[4]`. There is no `bytes.len() < PACKET_BYTES` guard and no `# Panics` doc.
Why it matters: this crate's brand is "typed errors, never panics" (see ts/fuzz.rs's absence-of-panic contract); the only in-tree caller is safe (mod.rs:836, `read_packet` guarantees ≥188, 996-1001), but any external consumer of the demux crate's public packet API gets a panic where a typed `demux.ts.bad-packet` was promised.
What correct looks like: `if bytes.len() < PACKET_BYTES { return Err(bad_packet(0)); }` at fn top (with the real offset, see F-007), or document the 188-byte precondition as a `# Panics` section.
Confidence: HIGH — Verification: read; caller guarantee traced at 995-1001.

### F-004 [S3]
Where: crates/demux/src/ts/mod.rs:23-27 (policy doc), implementation at 858-859 (part2, cross-ref)
Code:
```rust
//! **Continuity policy: tolerant + damage-noted.** A CC that repeats
//! is a tolerated duplicate (counted); a CC that jumps resets the
//! expectation, is counted, and raises one
```
What is wrong: "tolerated duplicate (counted)" hides that the duplicate packet's payload is still fully delivered: at 858-859 a repeated CC only increments `cc_duplicates` — execution falls through to the PSI assembler / PES consumer with the same payload again. ISO/IEC 13818-1 §2.4.3.2 receivers discard duplicates; here a duplicate continuation packet appends its bytes a second time into the live PES assembly (bounded-length sections silently cap the extra bytes; PES over-length becomes downstream recovered-damage).
Why it matters: on real spliced/tunneled broadcast streams (the doc's own motivating case) duplicate delivery corrupts the PES reassembly the shipped CLI performs, degrading exactly the damage tolerance the policy claims. The stats counter says "tolerated" while the payload path double-delivers.
What correct looks like: after counting a duplicate, `return` (skip delivery) — or the doc must say duplicates are counted AND re-delivered, so the PES half knows to expect it.
Confidence: HIGH — Verification: read of both sites; downstream effect traced to SectionAssembler's declared-length cap (536-539) and containers/ts.rs PesAssembler push path.

### F-005 [S2]
Where: crates/demux/src/ts/mod.rs:579 (field), doc claims at ts/mod.rs:50-57 + ts/fuzz.rs:21-23 (cross-file), ledger at avi/header.rs:384-401
Code:
```rust
    pending_services: Vec<crate::ts::psi::SdtService>,
    damage: DamageLog,
    stats: TsStats,
```
What is wrong: the damage ledger grows without bound for the entire delivered walk. `DamageLog::note` (avi/header.rs:397-400) pushes one `(u64, &'static str)` per recovery decision with no cap; TsDemuxer resets it exactly once (`self.damage = DamageLog::default()`, 1092) and NEVER drains during `next_event` — every TEI packet, bad header, CC jump, PSI CRC failure and skipped SI section mints an entry for the whole file. The shipped CLI wiring never reads `damage_events()`/`damage_total()` either (grep of apps/cli/src/containers/ts.rs: no call). An all-TEI 10 GB capture (valid damaged broadcast input, exactly what the module docs promise to survive) accumulates ~53M entries ≈ 850 MB+ RAM — unbounded allocation from file-controlled counts on the shipped `lumen open` path. Two doc comments call this bounded: avi/header.rs:382 "bounded-recovery bookkeeping (doc 03 §4.3(d))" and ts/fuzz.rs:21-23 "Memory: the demuxer's own ceilings (section assembler caps, damage-log cap) apply" — there is no damage-log cap. (W04-A05 filed the AVI-context twin as S3 — avi/header.rs.part1 F-001 — noting the movi walk drains and idx1 coalesces; TS has neither defense.)
Why it matters: the fuzz harness's core contract ("never allocates without a ceiling") is false, and the shipped open path has input-proportional memory growth in a demuxer whose docs sell bounded recovery.
What correct looks like: a `MAX_DAMAGE_EVENTS` cap with a fold-into-one `damage-log-truncated` event, or per-reason coalescing (the idx1 fix generalized — avi/index.rs precedent), or a periodic drain in `next_event`.
Confidence: HIGH — Verification: read of all cited sites; no-drain proven by grep (`damage` in ts/mod.rs: only 685, 1092, 660-668; none in apps/cli/src/containers/ts.rs).

### F-006 [S3]
Where: crates/demux/src/ts/mod.rs:32-36 (doc) + 84 (const), loop condition at 1058 + `psi_complete` at 945-953 (part2, cross-ref)
Code:
```rust
//! `start` walks a bounded head window ([`MAX_PSI_SCAN_BYTES`]) until
//! the PAT, every PAT-declared PMT and the SDT have been seen (the
//! broadcast repetition interval is sub-second; the corpus carries
//! all three in the first three packets). Missing SDT is legal (names
//! stay `None`); a missing PMT leaves its program trackless with a
```
```rust
pub const MAX_PSI_SCAN_BYTES: u64 = 4 * 1024 * 1024;
```
What is wrong: `psi_complete` (945-953) requires `original_network_id.is_some()` — i.e. an SDT — so a PAT+PMT stream without SDT (legal per the doc's own words, common in raw capture exports) never completes the scan: `start` walks the full 4 MiB window (~22,310 packets, each a `read_packet` Vec allocation + full parse) on EVERY open, then rewinds to head (1099) and re-walks the same region so the PES half sees it. The doc's "Missing SDT is legal" hides a fixed 4 MiB + double-walk cost; a missing PMT (also legal) does the same.
Why it matters: bounded-but-wasteful on every SDT-less open: ~8 MiB of I/O and ~44k packet parses before the first delivered event, on files whose head the demuxer will immediately re-traverse. This is the demuxer's hot path (open latency).
What correct looks like: a separate SDT deadline (e.g. stop after PAT+PMTs complete, keep listening for SDT opportunistically during the delivered walk), or a much smaller SDT sub-window; at minimum the doc should state the cost.
Confidence: HIGH — Verification: read of const, loop (1055-1071), psi_complete (945-953).

### F-007 [S4]
Where: crates/demux/src/ts/mod.rs:252-256
Code:
```rust
            let length = h[4];
            let end = 5 + usize::from(length);
            if end > PACKET_BYTES {
                return Err(bad_packet(0));
            }
```
What is wrong: `bad_packet` takes an offset parameter (143-152) precisely to locate the damage, but `parse`'s only call hard-codes `0` — every adaptation-field overrun anywhere in a 10 GB stream is reported as `offset: Some(0)`.
Why it matters: the error context is the observability surface for corrupt streams; an always-zero offset makes the typed error useless for diagnosis and misleads any log triage.
What correct looks like: thread the packet's file offset into `parse` (or report the packet index) so `bad_packet(offset)` names the real position.
Confidence: HIGH — Verification: read.

### F-008 [S4]
Where: crates/demux/src/ts/mod.rs:41-48 and 28
Code:
```rust
//! [`PesConsumer`] (PUSI + the packet's payload slice, TS header and
//! adaptation field already stripped) — until that half lands, the
//! default consumer counts packets and bytes. **No fake packets are
//! ever emitted.**
```
What is wrong: stale and imprecise on two counts. (1) "until that half lands, the default consumer counts packets and bytes" — the PES half HAS landed (`pes.rs` exists in this module, and the shipped CLI registers a real consumer at containers/ts.rs:153); and there is no "default consumer" object — the counting is inline in `process_packet` (930-932) when `self.pes` is `None`. (2) Line 28 "The adaptation-field discontinuity indicator behaves the same [as a CC jump]" — not the same: a CC jump also drops the PID's partial PSI section (866-873); the discontinuity indicator does not (882-885).
Why it matters: module docs are the contract the next maintainer (and this audit's sibling agents) trusts; both drift points misdescribe live behavior.
What correct looks like: update the seam paragraph to the landed state; delete or qualify "behaves the same".
Confidence: HIGH — Verification: read of all cited sites.

### F-009 [S4]
Where: crates/demux/src/ts/mod.rs:469-481
Code:
```rust
impl StickyShape {
    fn of(err: &EngineError) -> StickyShape {
        StickyShape {
            stage: err.stage,
            kind: err.kind.clone(),
            context: err.context.clone(),
        }
    }
```
What is wrong: `StickyShape` carries no `source`; `mint` (478-480) rebuilds via `EngineError::new(stage, kind, context)` — the original error's source chain (e.g. `.with_source(...)` payloads like bridge_poisoned's, containers/ts.rs:481) is silently dropped on every replay, so the second-and-later deliveries of a sticky error lose the diagnostic root cause.
Why it matters: sticky errors are replayed on every subsequent call — the shape callers see most is the one missing the source.
What correct looks like: store the source (the shape is already `Clone`); or document the drop.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-57 | REAL (claims: see F-002/F-004/F-006/F-008) | — | honesty notes (seek refusal, scrambled as-is, unknown PIDs counted) all verified against part2 code |
| consts `PACKET_BYTES`…`PID_NULL` | 80-102 | REAL | — | PID values match §2.4.4.3 Table 2-3; `EXTENSIONS` = the R3 row, consumed only by media-probe's round-trip mirror test (registry_roundtrip.rs) |
| `mint` | 108-114 | REAL | — | stage `Demux` typed-error factory |
| `not_ts` | 118-128 | REAL | — | `sniffed[..min(16)]` safe on empty |
| `no_pat` | 132-139 | REAL | — | exercised by `ts::tests::no_pat_is_typed` ok (test-workspace:1934) |
| `bad_packet` | 143-152 | REAL | S4 | offset param exists but part1's only caller passes 0 (F-007) |
| `resync_failed` | 156-165 | REAL | — | — |
| `not_started` | 169-174 | REAL | — | exercised by `next_event_before_start_is_typed` ok (test-workspace:1933) |
| `Pcr` | 182-188 | REAL | — | raw 33+9-bit passthrough, no interpolation (doc-honest) |
| `AdaptationField` | 192-210 | REAL | — | flags-only surface; OPCR/splice/private carried as flags only (documented) |
| `TsPacket::parse` | 245-303 | REAL | S3/S4 | F-003 (short-slice panic, pub API), F-007 (offset 0); bit math verified correct; reserved afc=0b00 → empty payload, unnoted |
| `TsPacket::has_payload` | 306-310 | REAL | — | trivial, correct |
| `probe` | 314-322 | DEFECTIVE | S2 | F-001 — panics on exactly-188 input, comment false; UNREACHABLE as shipped (tests only) |
| `TsStream` | 330-342 | REAL | — | es_info kept verbatim for decode seam |
| `TsStream::descriptor` | 349-373 | REAL | — | ffprobe-compatible table; unknowns → `ts-stream-0xNN` honest pass-through; 0x0F(LATM)+0x11(ADTS) both "aac" — defused downstream by the ADTS-syncword gate (containers/ts.rs:323-325), verified |
| `TsStream::kind` | 378-385 | REAL | — | consistent with descriptor's table (checked all shared stream types) |
| `TsProgram` | 390-404 | REAL | — | — |
| `TsStats` | 408-443 | REAL | — | 17 counters; Copy; exercised by corpus differential |
| `PesConsumer` (trait) | 451-458 | REAL | S3 | F-002 — doc inverts implementation (sticky poison, EOS-deferred) |
| `StickyShape::of` / `mint` | 469-481 | REAL | S4 | F-009 — drops `source` on replay |
| `SectionAssembler::feed` | 501-546 | REAL | — | bounded (4098 cap), pointer guard, documented back-to-back-sections gap; PSI re-feed on duplicates (F-004 cross-ref) |
| `CcState` | 550-553 | REAL | — | — |
| `TsDemuxer` struct | 560-586 | REAL | S2 | F-005 — `damage` field: unbounded, never drained in TS wiring |
| `TsDemuxer::new` | 594-619 | REAL | — | refusal claims verified via SourceReader (avi/riff.rs:219-236) |

## 5. Test honesty (tests for this file live in ts/tests.rs — A02's file; conclusions from reading them)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `probe_requires_synced_stride` (tests.rs:249-255) | NO for the panic case | asserts probe(&[0x47]) true, probe(1316-byte stream) true, mutated-sync false — the exactly-188 boundary (the one that panics, F-001) is never fed |
| `packet_fields_parse` / af-overrun test (tests.rs:~195-244) | YES | field-by-field asserts + typed `demux.ts.bad-packet` on AF 200 > 183 — would catch a parse regression |
| `pes_seam_receives_payloads_and_errors_propagate` (tests.rs:625-668) | NO for the error half | Collector always returns Ok; asserts 3 calls + bytes>0 only — the "and errors propagate" name half is unpinned (F-002) |
| `no_pat_is_typed` / `not_ts_is_typed` / `next_event_before_start_is_typed` | YES | exact error-code equality |
| `corpus_ts_programs_streams_and_names_match_ffprobe` (integration) | YES | pinned ffprobe truth, executed ok (test-workspace:2241) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "a whole 188-byte file is also accepted — the walk will tell" | mod.rs:320-321 comment | **FALSE** | F-001: len-188 input panics at 321; only len ≤ 187 short-circuits true |
| "The consumer's own typed error, propagated through `next_event` (the demuxer remains usable)" | mod.rs:455-456 | **FALSE** | F-002: sticky poison (679-682, never cleared) + error deferred to EOS (933-938, 1113-1151) |
| "A CC that repeats is a tolerated duplicate (counted)" | mod.rs:23-24 | SHALLOW | F-004: counted AND re-delivered — tolerance overstated |
| "Missing SDT is legal (names stay None)" | mod.rs:35 | REAL (cost unstated) | F-006: legal but costs the full 4 MiB window + rewind re-walk every open |
| "No fake packets are ever emitted" | mod.rs:47-48 | REAL | grep: no `DemuxEvent::Packet` construction in ts/mod.rs; PES packets are minted only by the registered consumer's bridge |
| "until that half lands, the default consumer counts packets and bytes" | mod.rs:46-47 | STALE/FALSE | F-008: PES half landed (pes.rs, containers/ts.rs:153); no "default consumer" object exists |
| "The adaptation-field discontinuity indicator behaves the same" | mod.rs:28 | SHALLOW | F-008: CC jump also drops PSI partials (866-873); AF indicator does not |
| "bounded head window (MAX_PSI_SCAN_BYTES)" | mod.rs:32-33 + 84 | REAL | 4 MiB constant; loop bounded (1058) — the bound exists (see F-006 for cost) |
| "Scrambled packets are delivered as-is … and counted" | mod.rs:55-56 | REAL | 887-889 counts; delivery falls through to the seam |
| "Seeking: … SeekSupport::None and a typed refusal, never a fake landing" | mod.rs:52-54 | REAL | 1154-1168; exercised by `seek_refuses_typed` + `containers::ts::tests::synthetic_lane_opens_demuxes_and_refuses_seek_typed` ok (nofailfast:2837) |

## 7. Dead / duplicated / unreachable code
- `probe` (314-322): UNREACHABLE from shipped entry points — the live format sniff is media-probe's `TsSignature` mirror (phase5.rs:155-170); repo grep finds `ts::probe` callers only in ts/tests.rs. Same pattern W04 found for `ogg::probe` (dead in production, mirror live). The registry_roundtrip test keeps the mirror honest on extensions, but nothing pins probe behavior parity — and both twins panic on the same input (F-001).
- `EXTENSIONS` (94): production value is mirror-check only (media-probe round-trip test); consistent with every other demuxer's pattern, not dead per repo convention.
- No duplicated logic within part1; `TsPacket` in crates/demux/tests/ts_pes_corpus.rs:132 is a test-local re-implementation (noted for its owner).

## 8. Dependencies & flags
- No feature gates — the whole module compiles unconditionally in the shipped `lumen` build (baseline `lumen engine-info` features list has no TS flag).
- Cross-module reuse by design: `crate::avi::riff::SourceReader` (69-70) and `crate::avi::header::DamageLog` — in-crate, documented; the DamageLog reuse imports avi's unbounded ledger into TS (F-005).
- `media_types` / `plugin_api` only; no unsafe; no new deps.

## 9. Open questions for the next agent
- A10 (part2): file the implementation-side finding for F-002 (`let _ = self.fail(err)` at 933-938 + loop-shape at 1113-1151) — my filing here is scoped to the trait doc. Confirm whether `next_event`'s single call can walk to EOS after a PES error on a real corpus file.
- A10 (part2): `process_packet`'s PSI partial-drop double-count condition (903-914) deserves its own mutation test; `psi_partial_drops` accounting has two independent increment sites (872, 913).
- A10: `language_of` (1173-1192) bails at the first 0x0A descriptor even when malformed, vs its doc "the first well-formed one wins" — I left it to you as it is in your range.
- W05-A02 (ts/tests.rs): the exactly-188 probe boundary case and a failing-consumer test are both missing (F-001/F-002) — both are one-line additions to existing tests.
