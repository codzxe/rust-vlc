# AUDIT: crates/demux/src/avi/riff.rs#part2 (lines 612-1223)
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-21 |
| Chunk boundaries | part2 = lines 612-1223 of 1223 (part1 = 1-611, assigned to W04-A04). The boundary cuts through `PayloadSlots::fill` (588-672); I read and traced the whole of `fill` for coherence, findings scoped to my range. Part2 = fill's pass-2 tail + spill path, `next_fit`, `Default`, and the entire `#[cfg(test)] mod tests` (693-1223). |
| Lines / functions | 612 / 3 production (fill-tail, next_fit, Default) + 18 tests + 2 test helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (`PayloadSlots` is the shared payload arena of 10 demuxers — `rg -n "PayloadSlots::new|default"` → avi/mod.rs:377 (default, the first-vertical AVI lane exercised by baseline rc-smoke), wav, aiff, mp4, mkv, ogg-family ps, adts, ivf, es/{mp3,annexb}) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `sed -n` on every quoted range; `rg -n "PayloadSlots::new|PayloadSlots::default" crates/demux/src/ apps/` (10 users); `rg -n "payload_too_large|MAX_CHUNK_PAYLOAD" crates/demux/src/avi/` (caller ceiling checks); `rg -n "from_pool_range" -A 30 crates/media-types/src/packet.rs` (assert semantics); `rg -n "fn read_into" -A 5 crates/plugin-api/src/access.rs` (default borrow seam); `rg -n "riff::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (18/18 ok, lines 1596-1613) |

## 1. One-paragraph verdict
The payload-arena pool's second half (spill path, growth policy, default bounds) and the
file's entire test module. The pool logic is correct under hostile tracing: pin
detection via `Arc::get_mut` before every write, cursor advances only after a
successful read, growth is doubling-bounded under `slot_cap`, spill never blocks and
never exceeds one exact-size buffer, and `SharedBytes::from_pool_range`'s release-mode
`assert!` is unreachable from here (`at + got ≤ at + want ≤ cap = buf.len()` by
construction). All 18 tests pass in the committed baseline plus the 10/10
`zero_alloc_read_chain` integration suite that pins allocation counts end-to-end.
Only hygiene findings: the test module re-implements the crate's own public
`test_support::BytesStream`, and the AVI default `slot_cap` (16 MiB) sits below the
AVI walk's own `MAX_CHUNK_PAYLOAD` (32 MiB), silently voiding the "zero allocations
per packet" claim for the 16-32 MiB payload class.

## 2. What actually works (verified)
— verified by `rg -n "riff::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → lines 1596-1613: all 18 tests `... ok` (incl. `arena_growth_stays_under_the_declared_bound`, `payload_slots_over_a_real_reader_recycle`).
— verified by same log lines ~2229-2254 (`tests/zero_alloc_read_chain.rs`): `varying_size_payloads_pack_into_the_arena_zero_alloc`, `over_ceiling_spill_is_exactly_two_allocs_per_payload`, `pooled_movi_walk_allocates_nothing_when_warm` — 10/10 ok: the pool's allocation claims are pinned by measurement, not assertion-free prose.
— verified by reading `SharedBytes::from_pool_range` (crates/media-types/src/packet.rs:144-154): the `assert!` boundary can never fire from `fill` (offset+len bounded by cap).

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/avi/riff.rs:699-715 (vs crates/demux/src/avi/test_support.rs:121-124)
Code:
```rust
    /// In-memory seekable source (the plugin-api test pattern, trimmed
    /// to what the reader needs).
    struct MemStream {
        uri: SourceUri,
        data: Vec<u8>,
        pos: usize,
    }
```
What is wrong: The test module hand-rolls a private seekable in-memory
`SourceStream` while the **same crate** ships a public one
(`avi/test_support.rs` `BytesStream`/`mem_stream`, whose doc says it exists "for
driving the demuxer in doctests, the fuzz harness and downstream engine tests").
Two parallel implementations of the same test double in one crate, with subtly
different seek-past-end semantics (MemStream maps `Current` overflow to `u64::MAX`
then errors; BytesStream has its own typed path).
Why it matters: Test-double drift: a fix to one (e.g. read-into efficiency, EOF
semantics) silently misses the other; the tests then pin the reader against a source
no production path uses.
What correct looks like: `use crate::avi::test_support::mem_stream` in the test
module, or move `MemStream` into `test_support` and delete one.
Confidence: HIGH — Verification: read both files

### F-002 [S4]
Where: crates/demux/src/avi/riff.rs:683-691 (with crates/demux/src/avi/mod.rs:126,377)
Code:
```rust
impl Default for PayloadSlots {
    /// 2 arenas × 16 MiB ceiling (the measured corpus frame classes'
    /// scale — see the type docs; the input crate's `MAX_BUFFER_BYTES`
    /// precedent): at most 32 MiB of lazily-grown payload memory per
    /// demuxer.
    fn default() -> PayloadSlots {
        PayloadSlots::new(2, 16 * 1024 * 1024)
    }
}
```
What is wrong: The AVI demuxer is the one user of `Default`
(avi/mod.rs:377) while the same module's walk accepts chunk payloads up to
`MAX_CHUNK_PAYLOAD = 32 * 1024 * 1024` (avi/mod.rs:126). Every AVI payload in
(16 MiB, 32 MiB] passes the walk's ceiling check, then unconditionally takes the
spill path (`want > slot_cap` → `vec![0u8; want]` per packet, never pooled) — the
type doc's "both at zero allocations per packet" (lines 510-512) silently stops
holding exactly for the payload class the AVI constants permit. The two constants
are not tied together anywhere (no assert, no shared const).
Why it matters: A 4K-MJPEG-class file with frames >16 MiB would allocate-and-drop
~20-32 MiB per frame on the demux thread — bounded (no OOM) but a per-packet
churn the docs say does not happen. Blast radius is small because real MJPEG
frames measured ≤12.5 MB (type doc, line 495).
What correct looks like: Either derive the pool bound from
`MAX_CHUNK_PAYLOAD` (`PayloadSlots::new(2, MAX_CHUNK_PAYLOAD.min(16 MiB))` with the
walk ceiling lowered), or document the >slot_cap spill class in the AVI module.
Confidence: HIGH — Verification: read (avi/mod.rs:126, 377; fill's `want <=
self.slot_cap` gates at lines 597/664)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PayloadSlots::fill` (spill tail; whole fn 588-672 spans the part1/part2 boundary) | 654-670 | REAL | — | one exact-size buffer; pushed as a new arena only while `want ≤ slot_cap && len < max_slots` — at the bound every arena is pinned (a free small one would have been grown in pass 2), verified by tracing passes 1/2; error path commits nothing |
| `next_fit` | 678-681 | REAL | — | `.max(len)` is dead in every reachable call (pass 2 invokes only when `want > cap == len`, so `pow2(want) > len` always) — harmless defense; `want == 0` impossible (early return at 592) |
| `Default for PayloadSlots` | 683-691 | REAL | F-002 | 2×16 MiB; only AVI uses it; see F-002 for the ceiling mismatch |
| `MemStream` + `SourceStream` impl (test helper) | 699-762 | REAL | F-001 | duplicate of test_support::BytesStream; `Current` overflow → `u64::MAX` → typed error — test-only code |
| `reader` (test helper) | 764-766 | REAL | — | — |
| `fourcc_render_printable_and_hex` | 768-772 | REAL | — | covers part1 `FourCC::render`; would fail if render broke |
| `little_endian_readers` | 774-780 | REAL | — | covers part1 u16le/u32le/i32le incl. -1 |
| `pad_is_one_only_for_odd_payloads` | 782-788 | REAL | — | covers part1 `pad_for` — the RIFF odd-payload invariant |
| `movi_chunk_classification` | 790-820 | REAL | — | covers part1 `classify_movi_chunk`: `00dc`/`01wb`/`10dc`/`JUNK`/`00pc`/`##dc`/`LIST` — `##dc` non-digit pin matters (corrupt ids → Other, no panic) |
| `read_exact_and_end_detection` | 822-834 | REAL | — | Complete vs Ended{partial} semantics |
| `read_prefix_at_clamps_to_the_stream` | 836-859 | REAL | — | clamp semantics: past-end ask → real prefix, at-end → empty-not-error |
| `peek_header_does_not_move_the_position` | 861-869 | REAL | — | position restore contract of `Demuxer::seek`'s dry-run |
| `peek_header_past_end_is_typed_truncation` | 871-887 | REAL | — | pins expected=pos+8/found=length exactly |
| `header_parses_from_stack_bytes` | 889-899 | REAL | — | `[u8;8]` const parse + round-trip vs peek |
| `peek_header_into_restores_the_position` | 901-924 | REAL | — | borrow twin + same truncation shape |
| `read_exact_into_fills_or_errors_typed` | 926-949 | REAL | — | expected=3+4/found=5 truncation pin |
| `read_prefix_into_clamps_like_read_prefix_at` | 951-968 | REAL | — | Vec/borrow twin agreement |
| `read_exact_or_end_agrees_with_the_borrow_twin` | 970-995 | REAL | — | 300-byte corpus, both twins byte-equal |
| `payload_slots_recycle_exact_size_buffers` | 1000-1062 | REAL | — | pointer-identity assertions: pinned slot never handed out twice; pool grows to bound only; steady state = exactly 2 allocations |
| `payload_slots_spill_varying_sizes_and_stay_bounded` | 1067-1103 | REAL | — | pinned+full → unpooled spill; oversized never pooled; empty want static |
| `payload_slots_short_reads_deliver_exact_lengths` | 1108-1138 | REAL | — | pins `got` (not `want`) as view length — the clamp-correctness contract downstream demuxers rely on |
| `arena_growth_stays_under_the_declared_bound` | 1147-1175 | REAL | — | doubling chain 100→1024→16384→65536 verified against pool_bytes; bound asserted per step |
| `payload_slots_over_a_real_reader_recycle` | 1179-1221 | REAL | — | end-to-end over SourceReader; 6 single-handle fills served by exactly the 2 pooled pointers (a third would be a spill) |

## 5. Test honesty
All 18 tests in the module (my range) carry stateful, mutation-sensitive assertions.
Mutation reasoning (no edits made): changing `from_pool_range(..., at, got)` to use
`want` fails `payload_slots_short_reads_deliver_exact_lengths` (256 vs 100); removing
the `Arc::get_mut` pin probe breaks compilation or the `assert_ne!` pointer check in
`payload_slots_recycle_exact_size_buffers`; a growth policy that allocates per size
fails `arena_growth_stays_under_the_declared_bound`'s `pool_bytes() == 64 * 1024`
terminal assertion. Baseline: 18/18 ok (nofailfast log 1596-1613) plus the
allocation-count integration suite `zero_alloc_read_chain` 10/10 (log ~2229+).
No tautologies, no ignored tests, no mocks of the subject under test (MemStream is
the *source* double, the pool/reader are the real code under test).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "at most `max_slots × slot_cap` bytes, lazily grown and never exceeded" | type doc 490-492 | REAL | pass structure + `arena_growth_stays_under_the_declared_bound` (baseline:1596) |
| "A pinned arena … is never written through; … when every arena is pinned it spills" | type doc 481-486 | REAL | `Arc::get_mut` probes at 617/639; `payload_slots_recycle_exact_size_buffers` (baseline:1603) |
| "the cursor only advances on success, and spill buffers are only committed on success" | fill doc 583-587 | REAL | code order: `read(...)?` precedes every cursor/used/push mutation (620-627, 644-651, 661-669) |
| "both at zero allocations per packet" (slot-cap policy) | type doc 510-512 | REAL for ≤16 MiB payloads / silently FALSE for AVI's 16-32 MiB class | F-002 |
| "one copy (the read itself) per payload" | type doc 467-468 | REAL | single `read_into` into the arena; zero_alloc suite pins it |

## 7. Dead / duplicated / unreachable code
- `next_fit`'s `.max(len)` (line 679): dead in every reachable invocation (pass 2
  only runs when `want > cap`), kept as defense — noted, not counted.
- `MemStream` (F-001): duplicated test double vs `avi/test_support::BytesStream`.
- No dead production code in the part2 range; `Default` has exactly one user (AVI).

## 8. Dependencies & flags
`SharedBytes` (media-types) is the only cross-crate type in part2; `Arc` from std.
No features, no unsafe, no unwrap/expect in production paths (test module uses
`unwrap_or_else(|e| unreachable!(...))` per house style — acceptable in tests).
Caller-side ceiling contract ("nothing trusts a declared size before it has been
ceiling-checked by the caller", line 13-14) verified honored at both audited call
sites: AVI (`MAX_CHUNK_PAYLOAD` check, avi/mod.rs:517) and AIFF (`MAX_PACKET_BYTES`
cap in step(), aiff/mod.rs:515).

## 9. Open questions for the next agent
- W04-A04 (riff.rs part1): the `read_prefix_into` single-full-ask design (lines
  344-365) bypasses `READ_STEP_MAX` by contract — confirm every `fill` caller's
  `want` is ceiling-checked before the call (I verified AVI and AIFF; check
  mp4/mkv/ogg/ps/es/adts/ivf users).
- Does any demuxer construct `PayloadSlots::new(0, …)`? (Degenerate: pure spill
  pool. `rg` shows none today.)
