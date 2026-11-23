# AUDIT: crates/demux/src/avi/riff.rs
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-02-13 |
| Lines / functions | 1223 / 30 prod (18 tests + MemStream harness) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/containers/avi.rs:36 `AviDemuxer::new(stream)` → avi/mod.rs (SourceReader::new, PayloadSlots, read_chunk_header_at:485, fill+read_prefix_into:577, read_prefix_at:836/1113, peek_chunk_header:1136) → this file; also apps/web/src/open.rs. Baseline proof: audit/runs/baseline/rc-smoke.txt:130 `BENCH file=...avi_mjpeg_pcm.avi ... v_frames=60 v_decoded=60 a_frames=60 a_decoded=60` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `cargo test -p demux --lib avi::riff` → 18 passed, 0 failed (exit 0); rg for callers across demux/apps; sed quote verification |

## 1. One-paragraph verdict
This is real, executed, hostile-input-aware code: the RIFF primitives do saturating
position math in the callers, typed truncation errors, per-source-call bounds on the
Vec/exact paths, and a genuinely bounded payload arena pool whose recycling is pinned
by pointer-identity tests. I ran all 18 unit tests — pass. The chunk-size arithmetic
itself lives in avi/mod.rs (saturating_add everywhere, MAX_CHUNK_PAYLOAD=32 MiB gate
before any fill), so zero-size chunks advance 8 bytes and no overflow loop is possible
through these primitives. Defects found are documentation-level: one doc comment on
`read_prefix_at` claims READ_STEP_MAX bounding that its own implementation (by later,
documented design) does not apply, and the raw little-endian readers are panic-on-misuse
helpers with no debug guard.

## 2. What actually works (verified)
— verified by `cargo test -p demux --lib avi::riff` -> "18 passed; 0 failed" including
pointer-identity recycling (`payload_slots_recycle_exact_size_buffers`,
`payload_slots_over_a_real_reader_recycle`) and the exact 64 KiB doubling chain
(`arena_growth_stays_under_the_declared_bound`).
— verified by rg: every in-tree caller of `read_prefix_at`/`read_exact_or_end_at`
passes either a constant (4/12/16/40/56), a READ_STEP_MAX-capped scan window
(mod.rs:836 `((limit - at) as usize).min(riff::READ_STEP_MAX)`), a
MAX_STRN_BYTES-clamped size (header.rs:644), or a MAX_CHUNK_PAYLOAD-gated size
(mod.rs:517-536). No unbounded attacker-sized allocation reaches these Vec APIs.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/avi/riff.rs:304-311
Code:
```
    /// the difference as damage. Reads are bounded per source call by
    /// [`READ_STEP_MAX`] as everywhere else. One exact-sized allocation
    /// (the borrow seam fills it directly, card 3.B3).
    pub fn read_prefix_at(&mut self, len: usize, to: u64) -> Result<Vec<u8>, EngineError> {
        let mut out = vec![0u8; len];
        let got = self.read_prefix_into(&mut out, to)?;
```
What is wrong: The doc on `read_prefix_at` claims per-source-call READ_STEP_MAX
bounding; the function delegates to `read_prefix_into`, which issues one full-size
`read_into(&mut out[filled..])` ask (line 362) — up to `len` bytes per call — by the
deliberate 7-c/V-1 design documented at lines 345-356 and in the module header
("the prefix borrow read — the payload path — makes **one** full `read_into` ask").
The comment is stale and states a bounding contract the code does not honor.
Why it matters: an I/O-bounding claim is a safety/perf contract a reviewer or caller
relies on; this one is false for the largest reads in the file. What correct looks
like: delete or correct the sentence to match read_prefix_into's doc. Confidence:
HIGH — Verification: read (sed 297-312, 357-369 both quoted above).

F-002 [S3]
Where: crates/demux/src/avi/riff.rs:143-155
Code:
```
pub fn u16le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}
```
What is wrong: Unchecked indexing — `data[offset]` panics on an out-of-range offset.
The contract is "bounds-checked by the caller" (line 141) with no `debug_assert!` to
catch misuse; any future caller that computes an offset from a parsed field turns a
malformed AVI into a remotely-triggerable panic. Current in-tree callers
(header.rs:576-587 offsets 0..44 into a length-verified 56-byte buffer) are safe.
Why it matters: convention, not type, is the only guard on a demux hot path.
What correct looks like: `debug_assert!(offset + 2 <= data.len())` or get()-based
typed errors. Confidence: MEDIUM (exploit requires a caller bug; none found today) —
Verification: read + rg of all call sites in avi/*.

F-003 [S4]
Where: crates/demux/src/avi/riff.rs:678-681
Code:
```
fn next_fit(len: usize, want: usize, cap: usize) -> usize {
    let target = want.next_power_of_two().max(len);
    if target > cap { want } else { target }
}
```
What is wrong: `want.next_power_of_two()` overflows (wraps to 0 in release, panics in
debug) for `want > 2^63`; a wrapped 0 target would `resize(0)` then panic slicing
`block[..want]`. Unreachable through the shipped default (slot_cap 16 MiB, callers
gated at MAX_CHUNK_PAYLOAD 32 MiB) but `PayloadSlots::new` is pub and unguarded.
Confidence: LOW (theoretical config) — Verification: read.

F-004 [S4]
Where: crates/demux/src/avi/riff.rs:597/660
Code:
```
        if want <= self.slot_cap {
```
What is wrong: payloads in (slot_cap, MAX_CHUNK_PAYLOAD] = (16 MiB, 32 MiB] always
take the spill path — one fresh `vec![0u8; want]` (line 660) per packet, never pooled.
Documented ("Payloads larger than `slot_cap` always spill"), bounded per packet, but a
file of 17-32 MiB MJPEG frames pays a 17-32 MiB alloc+copy per frame.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FourCC::new/tag/as_bytes | 34-46 | REAL | — | const, trivial, tested |
| FourCC::render | 51-58 | REAL | — | hex fallback tested |
| ChunkHeader::from_le_bytes | 91-97 | REAL | — | const LE parse, tested round-trip |
| classify_movi_chunk | 117-139 | REAL | — | checked digit math; only dc/wb pass, LIST→RecList, tested incl. `##dc`, `00pc` |
| u16le / u32le / i32le | 143-166 | REAL | S3 | F-002: unchecked index, caller-contract only |
| pad_for | 170-172 | REAL | — | `size & 1`, tested |
| SourceReader::new | 219-237 | REAL | — | typed Unsupported on no-size/no-seek/empty |
| SourceReader::length/position | 240-247 | REAL | — | accessors |
| SourceReader::seek_to | 250-254 | REAL | — | maps to SeekOrigin::Start |
| read_exact_or_end | 267-280 | REAL | — | READ_STEP_MAX-capped loop, partial on EOF; alloc size is caller-bound (verified all callers) |
| read_exact_or_end_at | 285-295 | REAL | — | seek + discard-partial twin |
| read_prefix_at | 307-312 | REAL | S3 | F-001: doc lies about READ_STEP_MAX; alloc len bounded by callers (≤64 KiB in-tree) |
| read_exact_into | 324-336 | REAL | — | typed truncated with expected/found; tested |
| read_prefix_into | 357-369 | REAL | — | clamps `to` to length; full-ask loop, strict progress; tested incl. past-end→empty |
| peek_chunk_header_into | 383-397 | REAL | — | save/restore, best-effort restore on error, tested |
| read_chunk_header_at | 426-431 | REAL | — | walk-cursor twin, no restore (W5-14) |
| peek_chunk_header (free) | 451-455 | REAL | — | wrapper, tested |
| PayloadSlots::new/max_slots/resident_slots/pool_bytes/slot_cap | 542-575 | REAL | — | accessors |
| PayloadSlots::fill | 588-671 | REAL | S4 | F-003/F-004; 2-pass scan + growth + spill; state consistent on read error (cursor/used advance only after success — verified read) |
| next_fit | 678-681 | REAL | S4 | F-003 theoretical overflow |
| Default for PayloadSlots | 688-690 | REAL | — | 2×16 MiB lazy |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fourcc_render…/little_endian…/pad_is_one… | YES | exact-value asserts on the real functions |
| movi_chunk_classification | YES | covers dc/wb/10dc/JUNK/00pc/##dc/LIST arms |
| read_exact_and_end / read_prefix_at_clamps / peek_header_* (4) / header_parses / read_exact_into / read_prefix_into / borrow_twin | YES | boundary positions, typed-error shapes, position-restore asserts — mutation-sensitive |
| payload_slots_recycle_exact_size_buffers | YES | pointer identity proves real Arc recycling, not fresh allocs |
| payload_slots_spill_varying_sizes_and_stay_bounded | YES | bound + content + oversized + empty-want asserts |
| payload_slots_short_reads_deliver_exact_lengths | YES | delivered count ≠ slot len — catches length confusion |
| arena_growth_stays_under_the_declared_bound | YES | asserts exact 64 KiB terminal residency (100→1024→16384→65536) |
| payload_slots_over_a_real_reader_recycle | YES | exactly-2-distinct-pointers over 6 fills catches any spill |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Vec-returning and exact borrow reads are chunk-bounded (READ_STEP_MAX per source call); the prefix borrow read makes one full read_into ask" | module doc :8-14 | REAL | lines 271, 328 step-capped; 362 full ask |
| "Reads are bounded per source call by READ_STEP_MAX as everywhere else" (read_prefix_at) | :304-305 | FALSE | F-001 — read_prefix_into makes full-size asks |
| "a failed read leaves no half-filled generation behind" | :586-587 | REAL | cursor/used assigned only after `read(...)?` returns (lines 620-622, 644-646) |
| "at most max_slots × slot_cap bytes, lazily grown and never exceeded" | :491-492 | REAL | arena_growth test + push-guard line 664 |

## 7. Dead / duplicated / unreachable code
None found: every pub item has an in-tree caller (rg over demux/apps) except
`FourCC::render` (log/debug rendering — acceptable) and `PayloadSlots::max_slots`
(test-only observable). No dead code.

## 8. Dependencies & flags
No new deps; plugin-api (SourceStream), media-types (SharedBytes/EngineError), std Arc.
No feature gates on this file — compiled in the plain release lane (rc-smoke AVI bench
proves it).

## 9. Open questions for the next agent
- avi/mod.rs walk (not mine): verify `resync_after_clamp` cannot re-enter
  `scan_for_landing` without progress (the `at`-advance math at mod.rs:850 is correct
  for odd/even block lens — I verified on paper).
- SharedBytes::from_pool_range asserts (media-types/src/packet.rs:145-149) — a
  misbehaving fill closure returning got>want would panic in release; in-tree closures
  are read_prefix_into (returns ≤ out.len()).
