# AUDIT: crates/demux/src/avi/riff.rs#part1 (lines 1–691)
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-21 |
| Lines / functions | 691 audited (1-691, all production code; `#[cfg(test)] mod tests` starts :693 — that module is part2's assignment) / 27 items |
| File verdict | REAL (chunk) |
| Reachable from shipped entry point? | YES. `SourceReader`/`PayloadSlots` are the shared byte/pool layer of the whole demux crate (rg users: avi, ivf, adts, asf, wav, ts, ps, es, mkv). Shipped lane: `lumen open` on the AVI first-vertical fixture → `apps/cli/src/containers/avi.rs` → `AviDemuxer` → this reader/pool on every packet. Executed: AVI first-vertical open-session full lifecycle + `decode-bench` `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60` (INDEX.md §contrast) · `avi::riff::tests` 18/18 (orchrun-test-workspace-nofailfast.log.txt:1596-1613) · `tests/zero_alloc_read_chain.rs` 2/2 — `header_peek_loop_allocates_nothing_when_warm`, `pooled_movi_walk_allocates_nothing_when_warm` (orchrun :2237-2245, the D-1 zero-alloc claim executed). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Full read of :1-202, :203-412, :412-491, :490-691 (production code complete) · outline rg (27 items) · `rg -n "read_exact_into\|peek_chunk_header\|read_chunk_header_at"` (callers: AVI family only — mod.rs:485/1136, header.rs:475/620/865, index.rs:688) · `rg -ln "SourceReader" crates/demux/src` (9 families) · Read `crates/demux/src/error.rs:29-75` (`crate::error::truncated` mints `demux.avi.truncated`) · Read `crates/demux/src/avi/mod.rs:510-594, 668-712` (caller discipline: MAX_CHUNK_PAYLOAD=32 MiB ceiling before pool fill :517-529; short-read damage accounting :588-594; RecList arm :675-693) · `rg -n "u32le\("` + Read header.rs:565-581, index.rs:360-371 (LE-reader caller bounds-contracts honored: 56-byte guarded read; `got/16` stride) · `rg -n "read_exact_or_end"` (users: asf/wav with file-controlled sizes) · baseline greps cited above. |

## 1. One-paragraph verdict
This is the demux crate's load-bearing foundation and it is built to a measurably higher standard than the demuxers sitting on it: every read primitive is bounds-clamped or typed-truncating, the RIFF vocabulary (`FourCC`, `ChunkHeader`, `pad_for`) is exact, the `PayloadSlots` arena pool's cursor/recycle/exclusivity logic (`Arc::get_mut` as the pin probe, doubling growth capped at `slot_cap`, spill beyond) is correct under every interleaving I traced, and the D-1 zero-alloc claim is not prose — it is executed by `tests/zero_alloc_read_chain.rs` in the baseline. The three defects are all trust-boundary debt: (1) the reader trusts `stream.size()` verbatim and its prefix primitives return silently-short data on a size-lying source, delegating damage accounting to callers — a delegation the AVI walk honors (:588-594) but the IVF walk does not (cross-ref ivf/mod.rs F-001, S2); (2) `classify_movi_chunk` brands every `LIST` inside `movi` a `rec ` grouping without reading the form type its own comment names; (3) the shared reader's typed-truncation path mints an AVI-family error code (`demux.avi.truncated`), latent namespace debt the tree itself acknowledges. Nothing here is fake, nothing panics on any in-tree path I could construct, and the hot-path performance story (W5-14 non-restoring chunk-header read, 7-c single-ask prefix read) is backed by committed measurements.

## 2. What actually works (verified)
— `SourceReader::new` capability gate (:219-237): unknown-length, unseekable, empty sources refused typed at stage Open — executed by `trait_calls_before_start_refuse_typed` (ivf) and AVI equivalents in baseline.
— `read_exact_into` (:324-336): READ_STEP_MAX-bounded steps, `Ok(0)` → typed `Corrupt::Truncated` with structure numbers — pinned by `read_exact_into_fills_or_errors_typed` (orchrun :1610).
— `read_prefix_at`/`read_prefix_into` clamp semantics (:307-312, :357-369): `to.min(length)` clamp, one full-ask borrow read, short-read re-ask — pinned by `read_prefix_at_clamps_to_the_stream` / `read_prefix_into_clamps_like_read_prefix_at` (orchrun :1612-1613).
— `peek_chunk_header_into` (:383-397): save/seek/read/restore with the original error winning over restore failure — pinned by `peek_header_into_restores_the_position`, `peek_header_past_end_is_typed_truncation` (orchrun :1607-1608).
— `PayloadSlots::fill` (:588-671): traced all four regimes (pack-at-cursor, recycle-drained, grow-by-doubling `next_fit` ≤ `slot_cap`, spill-when-pinned-or-over-cap). No path writes through a pinned arena (`Arc::get_mut` gate :617/:639), no path slices out of bounds (`at + want ≤ cap` established before every `read`), a failed `read` leaves no half-generation (cursor advances only on success). Pinned by `payload_slots_recycle_exact_size_buffers`, `payload_slots_spill_varying_sizes_and_stay_bounded`, `payload_slots_short_reads_deliver_exact_lengths`, `arena_growth_stays_under_the_declared_bound`, `payload_slots_over_a_real_reader_recycle` — 5/5, orchrun :1596-1605.
— D-1 zero-allocation: `zero_alloc_read_chain.rs` 2/2 executed (orchrun :2237-2245) — the header-peek loop and the pooled movi walk allocate nothing when warm; the `pool_bytes()` ≤ `max_slots × slot_cap` bound asserted by `arena_growth_stays_under_the_declared_bound`.
— LE readers' caller-bounds contract honored at the sites I sampled (header.rs:570-581 behind a `declared_size < 56` refusal + exact 56-byte read; index.rs:366-371 behind `got / 16` stride arithmetic) — the helpers cannot panic on those paths.
— End-to-end: the AVI first-vertical lane (the shipped feature) runs every packet through `read_chunk_header_at` + `fill` and decodes 60/60 video + 60/60 audio frames (INDEX.md §contrast, decode-bench on the avi fixture).

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/avi/riff.rs:357-369
Code:
```
    pub fn read_prefix_into(&mut self, out: &mut [u8], to: u64) -> Result<usize, EngineError> {
        let to = to.min(self.length);
        self.seek_to(to)?;
        let mut filled = 0;
        while filled < out.len() {
            let got = self.stream.read_into(&mut out[filled..])?;
            if got == 0 {
                break;
            }
            filled += got;
        }
        Ok(filled)
    }
```
What is wrong: `self.length` is `stream.size()` captured once at `new` (:227) and never cross-checked. If a `SourceStream` impl over-reports its size and then serves `Ok(0)` early (the trait makes short reads legal data, access.rs:36-38; nothing binds `size()` to the readable extent), this returns a silently-short `filled` — no error, no damage signal. The design intent ("the caller counts the difference as damage", :301-305) is honored by the AVI walk (avi/mod.rs:588-594 compares `delivered` vs `want`) but the reader itself has the means to detect the lie and doesn't use it: `stream.size()` is re-queryable without I/O, which is exactly how input's `BufferedReader::confirm_eof` types the same condition (buffered.rs:407-416).
Why it matters: This is the shared root of the IVF S2 (ivf/mod.rs F-001: `frame_header_at` slices the short result unchecked and panics; the IVF walk pre-validates extent against the trusted `length` and assumes exact reads). Every future demuxer card that copies the IVF/ADTS idiom inherits either the panic or the silent-short payload. One defense here (re-query `size()` on the `Ok(0)` path, or an explicit documented contract that callers must treat short prefix reads as damage) closes the class for all nine families.
What correct looks like: On `got == 0` with `filled < out.len()`, re-check `self.stream.size()`: if it now reports less than `self.length`, mint the typed truncation (input's `confirm_eof` pattern); otherwise the short read is a truthful EOF and `Ok(filled)` stands.
Confidence: HIGH — Verification: read; in-tree defenses traced (input buffered.rs confirm_eof, streaming ranged.rs fetch_window shortfall typing); IVF victim verified in the ivf/mod.rs audit; no in-tree impl currently triggers it (execution UNVERIFIED — would need a contract-violating source).

### F-002 [S4]
Where: crates/demux/src/avi/riff.rs:117-122
Code:
```
pub fn classify_movi_chunk(fourcc: &FourCC) -> MoviChunkKind {
    if *fourcc == LIST {
        // A nested list inside `movi` is a `rec ` record grouping —
        // transparent descent (the v0 scope note in the demuxer docs).
        return MoviChunkKind::RecList;
    }
```
What is wrong: The classification brands *any* `LIST` chunk inside `movi` a record grouping. The form type (the 4 bytes after the LIST header — `rec ` for genuine record groups) is never inspected: the consumer arm skips `pos + 8 + 4` blind (avi/mod.rs:692) and its comment asserts "A `rec ` list" (:679) as if it had been verified. A `LIST` of any other form type inside `movi` (hostile or merely unusual) is descended into and its payload re-parsed as chunk headers.
Why it matters: A mis-descended list's bytes become garbage chunk headers; the walk's damage tolerance (Other → skip + damage note) contains it, so the blast radius is mislabeled damage and lost payload — not a panic, not a hang. But the comment claims a specificity the code does not have, and the four form bytes are sitting right there at `pos + 8` for the cost of one already-buffered read.
What correct looks like: Read the form type in the RecList arm (or extend `classify_movi_chunk` to take it) and treat non-`rec ` lists as `Other` (skip + damage), matching the comment.
Confidence: HIGH — Verification: read; consumer arm traced (avi/mod.rs:675-693).

### F-003 [S4]
Where: crates/demux/src/avi/riff.rs:327-335 (via `crate::error`, demux/src/error.rs:62-69)
Code:
```
        while filled < out.len() {
            let step = (out.len() - filled).min(READ_STEP_MAX);
            let got = self.stream.read_into(&mut out[filled..filled + step])?;
            if got == 0 {
                return Err(error::truncated(start + out.len() as u64, self.length));
            }
            filled += got;
        }
```
What is wrong: The shared reader's typed-truncation path mints `crate::error::truncated`, whose stable code is `"demux.avi.truncated"` (error.rs:68) — an AVI-family namespace baked into the layer every family uses. Today only the AVI family calls `read_exact_into`/`peek_chunk_header`/`read_chunk_header_at` (rg-verified: avi mod/header/index only), so no wrong-namespace error can currently surface; the debt is latent and the tree knows it ("the crate-root `error.rs` is the AVI card's file — scope discipline, R3 consolidates when the registry merges", ivf/error.rs:5-7).
Why it matters: The first non-AVI family to adopt exact reads (an obvious next step after F-001's hardening) starts emitting `demux.avi.truncated` for e.g. Ogg truncations — a diagnostic lie at the user boundary.
What correct looks like: A neutral code at this layer (`demux.source.truncated`-style) or a family parameter; the R3 consolidation the tree already scheduled.
Confidence: HIGH — Verification: read; callers enumerated by rg.

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (two-layer contract) | 1-20 | REAL | — | The "nothing trusts a declared size before it has been ceiling-checked by the caller" rule verified at the AVI call site (mod.rs:517); pad-byte exclusion claim pinned by the fixture AC (odd JPEG payloads hash-match). |
| `FourCC::new/tag/as_bytes` | 34-46 | REAL | — | const fns, no validation (open fourcc space — documented). |
| `FourCC::render` | 51-58 | REAL | — | Printable-or-hex, log-only (doc says never for parsing — honored: no parser call site). |
| `RIFF`…`FORM_REC` constants | 62-76 | REAL | — | Byte-exact tags incl. the trailing-space `AVI `/`rec `. |
| `ChunkHeader::from_le_bytes` | 91-96 | REAL | — | const fn over `[u8; 8]` — structurally panic-free. |
| `MoviChunkKind` enum | 101-109 | REAL | — | Packet{stream}/RecList/Other. |
| `classify_movi_chunk` | 117-139 | REAL | S4 | F-002 (LIST form type never checked). Digit math checked (`checked_mul/add`), 0-99 stream space per AVI 1.0, `dc`/`wb` only, `pc`/`tx` → Other (documented PROPOSAL scope). |
| `u16le` / `u32le` / `i32le` | 143-166 | REAL | — | Panic-if-caller-lied helpers with a documented caller-bounds contract; contract honored at sampled sites (header.rs:570-581, index.rs:366-371). Not a defect — but every caller must be checked in their own audits (open question below). |
| `pad_for` | 170-172 | REAL | — | `size & 1` — RIFF-exact. |
| `READ_STEP_MAX` = 64 KiB | 177 | REAL | — | Governs Vec-destination reads only; borrow-read full-ask is the documented 7-c exception (:345-356). |
| `ReadExact` enum | 181-186 | REAL | — | |
| `SourceReader::new` | 219-237 | REAL | S3 | F-001 root: trusts `size()` with no cross-check (refusal set itself is correct and tested). |
| `length` / `position` / `seek_to` | 240-254 | REAL | — | `seek_to` does not clamp to length (source's own typed out-of-range) — consistent with read paths clamping themselves. |
| `read_exact_or_end` | 267-280 | REAL | — | Step-bounded; `Ok(0)` → `Ended{partial}` — the partial is honest data. |
| `read_exact_or_end_at` | 285-295 | REAL | — | `Ok(None)` discards the partial bytes — fine for header parsing (callers all constant-size or pre-validated); noted, no finding. |
| `read_prefix_at` | 307-312 | REAL | S3 | Vec twin of the clamp primitive; allocation of `len` is caller-ceilinged (AVI: 32 MiB gate; IVF: 12/32-byte constants). F-001 applies. |
| `read_exact_into` | 324-336 | REAL | S4 | F-003 namespace. Typed truncation with structure numbers — correct otherwise. |
| `read_prefix_into` | 357-369 | REAL | S3 | F-001. The single-full-ask + re-ask loop matches its doc; the W5-14 rationale (no window defeat) is sound. |
| `peek_chunk_header_into` | 383-397 | REAL | — | Original-error-wins restore discipline — the pattern IVF's `seek` violates (cross-ref). |
| `read_chunk_header_at` | 426-431 | REAL | — | Non-restoring by design (W5-14, measured 3.80→2.07 ms/walk); error leaves position at short-read end — documented, walk owns it. |
| `peek_chunk_header` (free fn) | 451-455 | REAL | — | Thin wrapper; used by header.rs/index.rs/seek paths. |
| `PayloadSlots` type docs (D-1/D-3/7-c policy) | 457-512 | REAL | — | Every concrete claim in the doc verified against `fill`'s code below (bounds, pin probe, spill, growth). |
| `PayloadSlots` struct + `Arena` | 514-536 | REAL | — | `buf.len()` = capacity invariant maintained by `resize`-only growth. |
| `new` / `max_slots` / `resident_slots` / `pool_bytes` / `slot_cap` | 542-575 | REAL | — | Accessors; `pool_bytes` sums real arena lengths (≤ max×cap). |
| `fill` | 588-671 | REAL | — | All four regimes traced correct (see §2); `want == 0` static-empty early-out; spill-below-bound adoption into the pool; no unsound aliasing (get_mut gate before every write). |
| `next_fit` | 678-681 | REAL | — | `want.next_power_of_two().max(len)`, exact-fit under ceiling — `grown ≥ want` always (want==0 unreachable). |
| `Default for PayloadSlots` | 683-691 | REAL | — | 2 × 16 MiB; matches AVI's MAX_CHUNK_PAYLOAD=32 MiB spill ceiling coherently. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fourcc_render_printable_and_hex` | YES | Both render branches asserted. |
| `little_endian_readers` / `pad_is_one_only_for_odd_payloads` | YES | Value-exact. |
| `movi_chunk_classification` | YES | dc/wb ordinals, `pc`/`tx`/garbage → Other, LIST → RecList. (Would NOT catch F-002 — no non-rec LIST case exists; that is why it survived.) |
| `read_exact_and_end_detection` / `read_exact_or_end_agrees_with_the_borrow_twin` | YES | Complete/Ended boundary + twin parity — kills step-bounding drift. |
| `read_prefix_at_clamps_to_the_stream` / `read_prefix_into_clamps_like_read_prefix_at` | YES | Past-end clamp pinned for both twins. |
| `peek_header_does_not_move_the_position` / `peek_header_into_restores_the_position` / `peek_header_past_end_is_typed_truncation` | YES | The pure-peek contract + typed error — the strongest pins in the module. |
| `read_exact_into_fills_or_errors_typed` | YES | Short stream → typed truncation with expected/found numbers. |
| `header_parses_from_stack_bytes` | YES | const parse round-trip. |
| `payload_slots_recycle_exact_size_buffers` / `payload_slots_spill_varying_sizes_and_stay_bounded` / `payload_slots_short_reads_deliver_exact_lengths` / `arena_growth_stays_under_the_declared_bound` / `payload_slots_over_a_real_reader_recycle` | YES | Cursor/pin/spill/growth regimes each pinned; short-read views deliver exact `got` lengths; pool byte bound asserted. Executed 5/5 (orchrun :1596-1605). |
| `tests/zero_alloc_read_chain.rs` (integration) | YES | Counting-guard asserts zero allocations warm for the header-peek loop and pooled movi walk — would fail if fill allocated in steady state. Executed (orchrun :2237-2245). |
| AVI first-vertical lanes (mod.rs tests + fixtures) | YES | End-to-end through this reader/pool; 60/60+60/60 decode in bench (INDEX.md). |
All 18 unit + 2 integration executed, 0 failed in baseline. (Detailed test-module audit is part2's assignment.)

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| Bounded per-source-call reads (`READ_STEP_MAX`), prefix path single full ask (7-c) | module docs :7-14, :345-356 | REAL | Code :271, :328, :362; twin-parity + zero-alloc runs executed. |
| Pad byte excluded from chunk size, skipped positionally, never leaks into packets | module docs :15-20 | REAL | `pad_for` + AVI walk's positional resume; fixture AC hash-match (xtask fixtures --check PASS, INDEX.md #12). |
| Pool: `max_slots × slot_cap`, lazily grown, never exceeded; larger payloads spill | :488-499 | REAL | `fill` traced; `arena_growth_stays_under_the_declared_bound` executed. |
| Zero allocation per payload in steady state (7-c, V-1 fix) | :501-512 | REAL | `zero_alloc_read_chain` 2/2 executed (orchrun :2237-2245). |
| W5-14 measured walk speedup (3.80→2.07 ms; 2.43→0.23 ms) | :399-419 | REAL-UNPROVEN | Code shape verified (non-restoring read, position lands at pos+8); the measurement is a committed claim I cannot re-run (no cargo); regression test named in the doc; numbers not independently executed in baseline logs. |

## 7. Dead / duplicated / unreachable code
- None dead in part1. `read_exact_or_end`'s Vec-returning shape has few remaining callers (asf/wav `read_exact_or_end_at`) — alive.
- `peek_chunk_header` (free fn) vs `peek_chunk_header_into` (method): twin pair by design (Vec vs borrow), not duplication.

## 8. Dependencies & flags
- `media_types::packet::SharedBytes` (pool views), `crate::error` (AVI-namespaced — F-003), no feature gates; compiled unconditionally.
- No unsafe anywhere in part1 (Arc::get_mut exclusivity instead) — verified by reading every line of the range.

## 9. Open questions for the next agent
- part2 (tests, :693-1222): does any test cover a non-`rec ` LIST inside `movi` (F-002)? My rg of test names says no — confirm while auditing the module.
- wav/asf `read_exact_or_end_at(value_size/packet_size, …)` call sites pass file-controlled sizes to a `vec![0u8; len]` allocator — verify their ceiling checks in the wav/asf audits (wav mod.rs:809 `value_size`, asf mod.rs:467/608/746 `packet_size`).
- Every `u16le/u32le/i32le` call site in avi/header.rs and avi/index.rs relies on the caller-bounds contract — those files' auditors should confirm each read is preceded by a length guarantee (I sampled two sites, both sound).
