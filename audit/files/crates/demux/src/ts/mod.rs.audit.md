# AUDIT: crates/demux/src/ts/mod.rs
| Field | Value |
|---|---|
| Auditor | W05-A04 |
| Date | 2025-07-11 |
| Lines / functions | 1206 / 40 (9 module fns, 31 methods) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: `lumen open/play` → `open_source_with` (apps/cli/src/demux_host.rs:378,446) → `containers::open("mpegts")` (apps/cli/src/containers/mod.rs:72) → `ts::open` (apps/cli/src/containers/ts.rs:110 `TsDemuxer::new`, :111 `start`, :153 `set_pes_consumer`) → `TsDemuxer::{start,next_event}` (this file). Executed end-to-end (see §2). |
| Findings | S0: 0 S1: 1 S2: 1 S3: 5 S4: 5 |
| Commands run | `sed -n` quotes (exit 0, all findings); `rg` callers (exit 0); `./target/debug/deps/demux-5e0c7194d741938b probe_requires_synced_stride` → 1 passed, exit 0; `./target/debug/deps/demux-036dd705535b2fb3 probe` → 9 passed, exit 0; `./target/debug/lumen open audit/scratch/ts188.bin` → **panic, exit 101**; `.../ts187.bin` → exit 65 (`demux.ts.no-pat`); `.../ts189.bin` → exit 65 (`probe.no-demuxer`) |

## 1. One-paragraph verdict
The transport-packet layer, PSI plumbing and program tracking in this file are genuinely implemented and reachable: PAT/PMT/SDT application, bounded resync, section reassembly with a 12-bit ceiling, a tolerant+counted continuity policy, and a typed-refusal seek all do what the code says, and the shipped CLI wires a real PES consumer at the seam. The KNOWN Wave-4 probe finding is CONFIRMED and EXECUTED: `probe` (315-322) indexes `header[188]` when `len == 188` exactly — but the production crash site is its shipped twin `TsSignature::probe` (crates/media-probe/src/signature/phase5.rs:161), which `lumen open` reaches before this demuxer ever runs; `lumen open` on a 188-byte file starting `0x47` panics with exit 101 (a valid single-packet TS per the code's own acceptance comment). Beyond that: the demuxer's per-packet damage notes grow an UNCAPPED Vec for the whole walk while ts/fuzz.rs claims a "damage-log cap" exists (false), and the walk allocates one Vec + seek + read per 188 bytes ignoring the crate's own zero-alloc borrow seam.

## 2. What actually works (verified)
- `./target/debug/lumen open audit/scratch/ts187.bin` → `error: [demux.corrupt]: demux.ts.no-pat`, exit 65 — verifies `open_sync` single-packet acceptance (mod.rs:708-709), the bounded PSI scan, and the typed `no_pat` refusal on a sub-packet file.
- `./target/debug/deps/demux-5e0c7194d741938b probe_requires_synced_stride` → 1 passed — the demux-side probe executes (and, per F-001, never sees the 188-byte shape).
- `./target/debug/lumen open audit/scratch/ts189.bin` → `probe.no-demuxer`, exit 65 — the 188-stride second-sync check works when a byte exists at [188].
- Baseline: `cargo test` per-package 4744 passed / 2 failed (audit/runs/baseline/, AGENT-SPEC §13); TS corpus/integration suites (ts_corpus.rs, ts_fuzz_survival.rs, streaming hls_vod_e2e.rs) exercise this file's walk with real fixtures.

## 3. Findings

F-001 [S1] — KNOWN Wave-4 finding CONFIRMED, EXECUTED; production site is the media-probe twin
Where: crates/demux/src/ts/mod.rs:315-322 (this copy test-only); live copy crates/media-probe/src/signature/phase5.rs:160-164
Code (mod.rs, verbatim):
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
What is wrong: when `header.len() == 188` exactly, `header.len() < PACKET_BYTES` is false, so `header[188]` is evaluated on a 188-element slice (valid indices 0..=187) → index-out-of-bounds panic. `probe` itself has NO production caller (rg `ts::probe(` → only ts/tests.rs:250-254); the shipped open path probes through `media_probe::probe_stream` (demux_host.rs:446) whose `TsSignature::probe` contains the same off-by-one with variable `prefix` (`if prefix.len() >= TS_PACKET_BYTES { if prefix[TS_PACKET_BYTES] != 0x47`), matching the Wave-4 wording `prefix[188] under len>=188`.
Why it matters: `lumen open` on ANY 188-byte file starting with 0x47 crashes the process. EXECUTED: `./target/debug/lumen open audit/scratch/ts188.bin` → `thread 'main' panicked at crates/media-probe/src/signature/phase5.rs:161:16: index out of bounds: the len is 188 but the index is 188`, exit 101. Controls: 187 B → typed `demux.ts.no-pat` (exit 65); 189 B → `probe.no-demuxer` (exit 65). A single-packet TS file is valid input by the code's own comment ("a whole 188-byte file is also accepted") and by `open_sync`'s explicit single-packet acceptance — the comment states acceptance, the code panics. Also reachable over the HTTP lane (`open_http_source` → same probe tail): a server returning exactly 188 bytes starting 0x47 crashes the CLI remotely. The guard test `builtin_signatures_never_panic_on_arbitrary_windows` (media-probe/src/signature/mod.rs:470-492) sweeps a 64-byte RIFF template only and cannot reach a 188-byte 0x47 window — its name is a false promise.
What correct looks like: `header.len() <= PACKET_BYTES || header[188] == 0x47` (both copies), or `get(188)`.
Confidence: HIGH — Verification: ran (panic reproduced; controls run; quotes sed-verified).

F-002 [S2] Uncapped damage log grows per damaged packet; ts/fuzz.rs claims a cap exists
Where: crates/demux/src/ts/mod.rs:845-848, 860-863, 841, 884 (note sites); crates/demux/src/avi/header.rs:397-400 (the log); ts/fuzz.rs:22-24 (the false claim)
Code (mod.rs verbatim):
```rust
        if parsed.transport_error {
            self.stats.tei_packets += 1;
            self.note(0, "ts: transport error indicator");
            return;
        }
```
Code (header.rs verbatim):
```rust
    pub fn note(&mut self, skipped: u64, reason: &'static str) {
        self.total = self.total.saturating_add(skipped);
        self.events.push((skipped, reason));
    }
```
What is wrong: every TEI packet, CC jump, bad header, discontinuity-indicator packet and SI skip pushes one `(u64, &'static str)` (24 bytes) into `DamageLog::events` with no ceiling. `process_packet` is called once per 188-byte packet for the whole `next_event` walk; `start` resets the log (mod.rs:1092) but the delivered walk never trims. ts/fuzz.rs:22-24 states "Memory: the demuxer's own ceilings (section assembler caps, damage-log cap) apply" — there is no damage-log cap; the claim is false.
Why it matters: a crafted damage-dense TS (e.g. every packet TEI, or every packet with the discontinuity indicator) allocates ~24 B/packet for the entire file: a 1 GiB crafted file ≈ 5.6 M entries ≈ 135 MB; larger inputs scale linearly. The fuzz harness's no-unbounded-allocation contract is violated by exactly the inputs it exists for.
What correct looks like: a bounded ring/count in `DamageLog` (count + first N events), as the AVI side's budgets do.
Confidence: HIGH — Verification: read (both sites sed-verified; no cap anywhere in DamageLog).

F-003 [S3] `TsPacket::parse` panics on slices shorter than 188 (public API hazard)
Where: crates/demux/src/ts/mod.rs:251-257, 287-291
Code (verbatim):
```rust
        if afc & 0b10 != 0 {
            let length = h[4];
            let end = 5 + usize::from(length);
            if end > PACKET_BYTES {
                return Err(bad_packet(0));
            }
```
What is wrong: the function documents "Parse one 188-byte packet" but never checks `bytes.len()`. `h[4]` panics for len < 5; `&h[at..]` (line 288, `at` up to 188) panics for any `bytes.len() < at` — e.g. a 100-byte slice with AF length 150 passes the `end > 188` check (155 ≤ 188) then dies slicing `&h[155..]`. In-crate callers are safe (`read_packet` gates ≥ 188; tests pass 188-byte packets), so this is a public-API trap, not a shipped-path crash.
What correct looks like: `if bytes.len() < PACKET_BYTES { return Err(bad_packet(0)); }` at entry.
Confidence: HIGH — Verification: read (callers enumerated with rg: mod.rs:836 and tests.rs only).

F-004 [S3] Duplicate-CC packets are counted, then delivered to the PES seam anyway — and the seam cannot dedup
Where: crates/demux/src/ts/mod.rs:858-859 (count, no suppress), 930-938 (delivery); trait at 457
Code (verbatim):
```rust
                if parsed.continuity_counter == last {
                    self.stats.cc_duplicates += 1;
                } else if parsed.continuity_counter != (last + 1) & 0x0F {
```
What is wrong: a retransmitted packet (same CC, same payload — legal broadcast splice behavior, explicitly "tolerated" by the module policy) is counted and then handed to `pes_payload` a second time. The `PesConsumer` signature is `(pid, pusi, payload)` — the continuity counter is not forwarded, so the PES half structurally cannot detect or drop the duplicate; a duplicated PUSI PES start mints a duplicate PES packet downstream. PSI re-delivery is idempotent (re-parse overwrites), the ES lane is not.
Why it matters: duplicated ES data on realistic retransmission; downstream damage (duplicate frame/AU) UNVERIFIED (pes.rs is out of this file's scope).
What correct looks like: suppress duplicate-CC payload delivery (count remains), or forward CC in the seam.
Confidence: MEDIUM — Verification: read (seam contract verified; downstream effect UNVERIFIED).

F-005 [S3] One heap Vec + one seek + one read per 188-byte packet; the crate's zero-alloc borrow seam unused
Where: crates/demux/src/ts/mod.rs:995-1001; crates/demux/src/avi/riff.rs:307-312
Code (verbatim):
```rust
    fn read_packet(&mut self) -> Result<Option<Vec<u8>>, EngineError> {
        let bytes = self.reader.read_prefix_at(PACKET_BYTES, self.walk)?;
        if bytes.len() < PACKET_BYTES {
            return Ok(None);
        }
```
What is wrong: `read_prefix_at` does `vec![0u8; 188]` + seek + read per packet. `start`'s PSI scan is up to 4 MiB ≈ 22.3k packets ≈ 22.3k allocations; the delivered walk of a multi-GB file does millions. `SourceReader::read_prefix_into` (riff.rs:357, "the borrow twin") exists precisely to avoid this and is not used by the TS walk (the AVI side uses stack buffers per riff.rs:89).
Why it matters: hot-path allocation churn proportional to file size; measurable on large captures; bounded, so S3 not S2.
What correct looks like: one reusable 188-byte (or chunked) buffer via `read_prefix_into`.
Confidence: HIGH — Verification: read (both files).

F-006 [S3] `psi_complete` demands an SDT while the module doc says "Missing SDT is legal" — SDT-less files always scan the full 4 MiB
Where: crates/demux/src/ts/mod.rs:35-36 (doc), 945-953 (check), 1058 (loop)
Code (verbatim):
```rust
    fn psi_complete(&self) -> bool {
        self.transport_stream_id.is_some()
            && self.original_network_id.is_some()
```
What is wrong: `original_network_id` is set only by an SDT (line 798). The doc (lines 35-36: "Missing SDT is legal (names stay `None`)") describes the OUTCOME as legal, but the scan-termination condition can never be satisfied without an SDT, so every SDT-less TS (SDT-less captures exist) walks the entire `MAX_PSI_SCAN_BYTES` = 4 MiB window (~22k packet reads, per F-005 each an alloc+seek+read) before `start` returns. Correct results, bounded but avoidable cost; outcome/doc are reconcilable only by reading "legal" as "not an error".
What correct looks like: terminate when PAT + all PAT-declared PMTs are seen; treat SDT as best-effort (it already is — the stash handles order).
Confidence: HIGH — Verification: read.

F-007 [S3] A PES-consumer error surfaces one call late and is permanently replaced by EndOfStream if it fires on the last packet
Where: crates/demux/src/ts/mod.rs:1106-1112 (entry-only sticky check), 933-938 (fail inside process_packet), 1117-1142 (loop never re-checks)
Code (verbatim):
```rust
    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if let Some(err) = self.sticky.as_ref() {
            return Err(err.clone().mint());
        }
```
What is wrong: `fail()` at line 937 stores the sticky shape, but the `loop` below only checks `sticky` at function entry. After a consumer error the loop keeps reading and re-invoking the failing consumer every subsequent PES packet (each `fail` overwrites the shape); queued events are returned first. If the error fires on the final packet, the EOS branch returns `EndOfStream` and the error never surfaces unless the engine calls `next_event` again after EOS (engines stop).
What correct looks like: check `sticky` at the top of the loop (or break on consumer error).
Confidence: MEDIUM — Verification: read (control flow traced; how often BridgeConsumer errors mid-walk is UNVERIFIED — pes.rs out of scope).

F-008 [S4] Dead ceiling check in `SectionAssembler::feed`
Where: crates/demux/src/ts/mod.rs:527-532
Code (verbatim):
```rust
            let declared = 3 + ((usize::from(self.buf[1] & 0x0F) << 8) | usize::from(self.buf[2]));
            if declared > MAX_SECTION_BYTES {
                // Over the 12-bit ceiling: a corrupt length, drop it.
```
What is wrong: `section_length` is 12 bits (max 4095), so `declared` max = 4098 = `MAX_SECTION_BYTES` (psi.rs:53) — `>` can never fire. Harmless defensive dead code; the comment implies a reachable guard.
Confidence: HIGH — Verification: read (psi.rs:53 sed-checked).

F-009 [S4] `EXTENSIONS` is dead in production — the live registry row is the media-probe twin
Where: crates/demux/src/ts/mod.rs:93-94
Code (verbatim):
```rust
/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["ts", "m2t", "mts"];
```
What is wrong: no caller outside this definition (rg across repo); the shipped extension list is phase5.rs:172-174's own copy. Mirror drift is only caught by the round-trip suite.
Confidence: HIGH — Verification: read (rg).

F-010 [S4] Two `unreachable!` behind `CodecDescriptor::new` in `TsStream::descriptor`
Where: crates/demux/src/ts/mod.rs:369, 372
Code (verbatim):
```rust
                return CodecDescriptor::new(&format!("ts-stream-{other:02x}"))
                    .unwrap_or_else(|_| unreachable!("formatted names are valid"));
```
What is wrong: baseline tree-wide pattern (4126 unreachable!); construction-provable but still panic-on-parsed-input shape if `CodecDescriptor::new` ever gains stricter rules.
Confidence: HIGH — Verification: read.

F-011 [S4] Stringly-typed error-code seam for CRC failures
Where: crates/demux/src/ts/mod.rs:919
Code (verbatim):
```rust
                    Err(err) if err.context.code == "demux.ts.psi.crc-mismatch" => {
```
What is wrong: psi.rs mints the string, mod.rs compares it — a rename in either file silently reclassifies CRC failures as "unparseable". Works today (psi.rs:159 verified).
Confidence: HIGH — Verification: read.

F-012 [S4] PMT-before-PAT is not stashed (SDT is); `language_of` doc/code mismatch
Where: crates/demux/src/ts/mod.rs:784-795 (PMT-without-PAT fallback), 1171-1174 vs 1186-1188
Code (verbatim, language_of):
```rust
/// The ISO 639 language descriptor (0x0A) of an ES descriptor loop —
/// the first well-formed one wins.
```
```rust
            return None;
        }
        at += 2 + len;
```
What is wrong: (a) A PMT arriving before its PAT is counted as unknown-PID and lost until its next repetition; recovery leans on the sub-second broadcast repeat — a single-shot PMT-before-PAT leaves the program trackless (damage-noted at 1082). SDT gets a `pending_services` stash; PMT gets none — asymmetric. (b) `language_of`'s doc says "the first well-formed one wins" but the code returns `None` on the FIRST 0x0A descriptor with len ≥ 3 that fails to parse — later well-formed descriptors are never considered.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint | 108-114 | REAL | — | error helper |
| not_ts | 118-128 | REAL | — | sniffed clamp safe |
| no_pat / bad_packet / resync_failed / not_started | 132-174 | REAL | — | typed errors, all reachable |
| TsPacket::parse | 245-303 | REAL | S3 | AF bounds correct (251-257); panics on short slices (F-003); PCR 33+9 bit math correct |
| TsPacket::has_payload | 307-309 | REAL | — | |
| probe | 315-322 | BUG | S1 | F-001; test-only reachability; twin live in media-probe |
| TsStream::descriptor | 349-373 | REAL | S4 | F-010 |
| TsStream::kind | 378-385 | REAL | — | |
| PesConsumer (trait) | 451-458 | REAL | S3 | no CC in signature (F-004) |
| StickyShape::of / mint | 470-481 | REAL | — | |
| SectionAssembler::feed | 501-546 | REAL | S4 | pointer-field + header bounds correct; tail-after-complete dropped (documented 493-500); F-008 dead check |
| TsDemuxer::new | 594-619 | REAL | — | |
| set_pes_consumer | 622-624 | REAL | — | called by CLI ts.rs:153 |
| accessors (programs…track_of_pid) | 629-675 | REAL | — | used by CLI harvest |
| fail / note | 679-686 | REAL | S2 | note → uncapped log (F-002) |
| is_psi_pid | 690-692 | REAL | — | |
| open_sync | 696-720 | REAL | — | bounds-safe; single-packet acceptance |
| apply_section | 723-831 | REAL | S4 | PAT/PMT/SDT tracking correct; NIT pid 0 skipped; F-012 asymmetry; PMT-without-PAT pmt_pid=0 |
| process_packet | 834-942 | REAL | S2/S3 | CC policy matches doc; TEI/null/scramble/PSI routing correct; F-002/F-004/F-007 |
| psi_complete | 945-953 | REAL | S3 | F-006 |
| build_tracks | 957-992 | REAL | — | TimeBase 1/90000 direct; ids sequential |
| read_packet | 995-1001 | REAL | S3 | F-005 |
| resync | 1005-1028 | REAL | — | bounds correct; hard error near EOF when <2 packets remain (typed, acceptable) |
| start | 1032-1104 | REAL | S3 | scan + reset + rewind logic verified; F-005/F-006 costs |
| next_event | 1106-1152 | REAL | S3 | F-007; EOS partial-tail note correct |
| seek / seekability | 1154-1168 | REAL | — | typed refusal, matches docs; CLI `--start-time` on TS always refuses (ts.rs:125-131) |
| language_of | 1173-1192 | REAL | S4 | F-012b |
| is_eit | 1196-1198 | REAL | — | |

## 5. Test honesty (tests in ts/tests.rs — this file's `mod tests`)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| probe_requires_synced_stride (249-255) | NO for the S1 | feeds len 1 and len 1128 (single_program_ts = 6×188); never len 188 — the panic shape is untested (executed proof in F-001) |
| packet_fields_parse (198-223) | YES | asserts header bit fields on a real packet |
| tei_and_scrambling_and_af_overrun (225-247) | YES | AF length 200 → typed `demux.ts.bad-packet` |
| continuity_jump_and_duplicate_are_tolerant_and_counted (513-533) | PARTIAL | pins cc_duplicates/cc_jumps stats; does NOT assert duplicate payload non-delivery (F-004 invisible to it) |
| no_pat_is_typed / not_ts_is_typed (386-412) | YES | typed refusal pins |
| single/multi_program_surface (281-355) | YES | PAT/PMT/SDT surface incl. service names |
| pes_seam_receives_payloads_and_errors_propagate (625-663) | YES | collector consumer incl. error propagation |
| partial_tail_packet_is_a_note_not_an_error (600-624) | YES | EOS note path |
Coverage gaps: no 188-byte probe boundary test; no damage-log growth test; no duplicate-payload-delivery test.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A CC that repeats is a tolerated duplicate (counted); a CC that jumps … raises one Discontinuity" | mod.rs:23-27 | TRUE | 858-874 verified; test 513 |
| "Missing SDT is legal (names stay None)" | mod.rs:35-36 | TRUE-but-costly | legal outcome, but psi_complete (946-947) can't terminate without SDT → full 4 MiB scan (F-006) |
| "a whole 188-byte file is also accepted — the walk will tell" | mod.rs:320-321 | FALSE | the probe panics on exactly-188 (F-001, executed); the demuxer's open_sync WOULD accept it (708-709) |
| "No fake packets are ever emitted" | mod.rs:47-48 | TRUE | TsDemuxer emits only Discontinuity/Recovered/EOS; packets come from the CLI-side bridge |
| "damage-log cap" | ts/fuzz.rs:22-24 | FALSE | DamageLog has no cap (header.rs:397-400, F-002) |
| "Seeking: … SeekSupport::None and a typed refusal, never a fake landing" | mod.rs:51-54 | TRUE | 1154-1168; test seek_refuses_typed (664) |

## 7. Dead / duplicated / unreachable code
- `probe` (315-322): unreachable in production (test-only) — the live twin is media-probe phase5.rs.
- `EXTENSIONS` (93-94): dead in production (F-009).
- `declared > MAX_SECTION_BYTES` (528): dead branch (F-008).
- The probe logic is triplicated: mod.rs:probe, phase5.rs TsSignature, and open_sync's own stride check (696-720) — three hand-synced copies of the same magic walk.

## 8. Dependencies & flags
No feature gates on this file's code (plain `pub mod` in lib tree; release lane builds it). Cross-crate deps: media_types, plugin_api; intra-crate: avi::riff::SourceReader, avi::header::DamageLog, ts::psi, ts::pes (consumer half). The DamageLog dependency lives in the AVI module — a demuxer-family layering smell (TS depends on AVI's header module for both SourceReader and DamageLog).

## 9. Open questions for the next agent
- pes.rs (the consumer half): does PesAssembler emit a duplicate PES on a re-sent PUSI packet (F-004 downstream)? Does BridgeConsumer error mid-walk in practice (F-007)?
- media-probe phase5.rs + signature/mod.rs are in another agent's scope: the LIVE S1 (phase5.rs:161) and the false-promise no-panic test (mod.rs:470-492) need findings there; my execution output above is the evidence.
- psi.rs: does parse_section reject PAT entries with reserved PID bits / multi-section PATs (section_number > 0)? Tracking in mod.rs trusts entries blindly.
