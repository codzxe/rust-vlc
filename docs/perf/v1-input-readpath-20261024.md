# V-1 + the input read path — card 7-c (working notes, 2026-10-24)

Agent 7-c (relaunch 2), branch `wip/w7-c`. Scope this session: **part 1**
(demux-side arena pool + one source call per payload). Part 2 — the
input crate's large-read path — is a later session (not started).

All numbers below are **test-pinned measurements** from
`cargo test -p demux` on this branch (commit range `5bc51ef..HEAD`),
toolchain rustc 1.99.1, debug profile, shared target dir
(`/home/z/my-project/target`) with up to 4 sibling Wave-7 agents
compiling concurrently (lock-blocked waits, no CPU-idle contention
during the test runs themselves — the suites are single-threaded
serialized). Raw evidence: the assertions in
`crates/demux/tests/zero_alloc_read_chain.rs` and
`crates/demux/src/avi/riff.rs` are the on-disk record; each `assert_eq!`
message carries the number and its reason.

## 1. The V-1 violation (before, main @ 3f04786)

`PayloadSlots` was an exact-size slot pool (2 × 64 KiB cap). Varying
MJPEG frame sizes never repeated an exact length and mostly exceeded
64 KiB, so **every video packet spilled**: one payload-sized
allocation + the `Arc` conversion = **2.000 alloc events per video
packet** (pinned by the pre-7-c test
`varying_size_spill_is_exactly_one_alloc_per_payload`: 128 payloads →
exactly 256 events). Memory-audit 2026-10-23 register V-1
(docs/perf/memory-audit-20261023.md:238).

## 2. The seam-call pin (predecessor 5bc51ef, red on main)

`big_payload_reads_are_one_source_call_per_frame`: 6 uniform 1.5 MiB
`00dc` frames through the real `AviDemuxer`, counting every
`read_into` at the source seam.

| build | total seam calls | payload-class asks | max ask |
|---|---|---|---|
| main @ 3f04786 | **164** | 144 (24 chopped 64 KiB steps × 6) | 64 KiB |
| 7-c (this branch) | **26** | **6** (one full 1.5 MiB ask per frame) | **1.5 MiB** |

The 26 = 14 open-phase small reads (RIFF head 12 B; `LIST hdrl` peek +
form; `avih` peek + 56 B; `LIST strl` peek + form; `strh` peek + 56 B;
`strf` peek + 40 B; `LIST movi` peek + form — all ≤ 56 B, fixed cost,
none per-payload) + 6 payload reads + 6 header reads (8 B). The
original pin asserted 12 total, assuming open-phase reads were Vec
reads; they route through the card-3.B3 borrow seam by design, and
making them Vec reads would be a regression (an allocation per open
read). The pin was amended (same file, same contract, more precise):
payload-class asks == frames, max ask == full frame, open phase
pinned by name at 14. **Not a weakening**: the chopped-payload failure
mode it exists to catch shows up in the payload-class counter at 24×
inflation.

## 3. The fix (crates/demux/src/avi/riff.rs)

* `PayloadSlots` rebuilt as a bounded **arena pool**: shared
  `Arc<Vec<u8>>` backing blocks with a fill cursor; a payload is read
  into the arena at the cursor and the packet receives a
  `SharedBytes::from_pool_range(arena, offset, got)` view — O(1), one
  refcount bump, zero allocation, zero copy (the read itself is the
  one copy), for **any** size mix.
* Recycle: `Arc::get_mut` exclusivity probe (the `&mut` twin of the
  W6-05 `Arc::try_unwrap` template in media-types
  `tests/shared_bytes_alloc.rs::pool_owner_can_reclaim_through_try_unwrap`).
  A pinned arena (views queued downstream) is never written through;
  the pool rotates; when every arena is pinned or the payload is over
  the ceiling it **spills** — never waits.
* Growth: doubling (`next_fit`), demand-driven, never past `slot_cap`;
  at most `log2(slot_cap)` resizes per arena per stream. Default
  **2 arenas × 16 MiB ceiling** — lazily grown, so a 720p-MJPEG stream
  settles at one 64 KiB–2 MiB arena; the ceiling covers the measured
  corpus frame classes (≤ 4K MJPEG ≈ 12.5 MB; memory-audit §4) and
  matches the input crate's `MAX_BUFFER_BYTES` precedent.
* `read_prefix_into`: **one `read_into` of the full caller-sized ask**
  (was `ceil(want / 64 KiB)` READ_STEP_MAX steps). The destination is
  caller-owned memory (the arena), so the step loop bounded nothing
  the caller had not already bounded. READ_STEP_MAX still governs the
  reads whose destination is the reader's own buffer.
* No `unsafe`.

## 4. After (this branch) — counting-allocator table

MJPEG-in-AVI video path (varying per-frame sizes, warm pool):

| metric | before (main @ 3f04786) | after (7-c) | pinned by |
|---|---|---|---|
| alloc events / video packet | 2.000 (128 payloads → 256) | **0** | `varying_size_payloads_pack_into_the_arena_zero_alloc` |
| payload bytes copied / packet | ~2× (spill fill + Arc copy path) | 1× (the read) | by construction; no post-fill copy exists |
| true over-ceiling spill | 2 events/payload | 2 events/payload (unchanged escape hatch) | `over_ceiling_spill_is_exactly_two_allocs_per_payload` |
| uniform PCM path | 0 (exact-size recycle) | 0 (cursor packing/recycle) | `pooled_movi_walk…`, `real_movi_walk…`, `two_stream…` |
| seam calls, 6 × 1.5 MiB frames | 164 | 26 (6 payload-class, full ask) | `big_payload_reads_are_one_source_call_per_frame` |

Bounded memory: `pool_bytes ≤ max_slots × slot_cap` at every step of a
100 B → 50 KB escalation sequence, one arena, doubling chain
100 → 1024 → 16384 → 65536 — pinned by
`riff::tests::arena_growth_stays_under_the_declared_bound`.

Full demux suite on this branch: **140 passed, 0 failed** (corpus
tests ignored — media dirs absent in this sandbox; 4 pre-existing
ignores).

## 5. Not measured / known limits

* The **real-file** syscall count (input crate's buffered reader
  serving a 1.5 MiB `read_into`) is part 2 — not started. The default
  `SourceStream::read_into` allocates one `out.len()` Vec per call, so
  sources that do not override it pay a big transient allocation per
  large ask; the input crate's override is the follow-up.
* A chunk whose declared size runs past EOF still asks for the
  declared prefix size (pre-existing walk behavior, unchanged by this
  card) — a clamp to `file_len - payload_start` would bound it; left
  for a follow-up to avoid touching damage accounting under deadline.
* Latency/throughput benches (ms/frame) not re-run this session — the
  counts above are the honest scope. The W5-14 walk benches live in
  the color_bench harness and should be re-run at merge.

## 6. D-10

* **MEASURE?** Yes — every number above is a pinned test assertion
  run on this branch (`cargo test -p demux`, 140/0), not an estimate.
* **NOT tested?** Real-file I/O path, corpus suite (media absent),
  latency benches, part 2 entirely.
* **What breaks?** Nothing in-suite; risk concentrated in the amended
  seam-call pin (open-phase count 14 is fixture-shaped — a header.rs
  change shifts it) and in consumers holding many packets live (pool
  degrades to spill, correct but allocating).
* **Who hits it first?** The merge gate (full workspace run) and the
  input-crate agent (part 2) who must make the source side honor
  large asks without a window-copy per call.
