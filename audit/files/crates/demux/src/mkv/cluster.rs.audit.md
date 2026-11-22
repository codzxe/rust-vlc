# AUDIT: crates/demux/src/mkv/cluster.rs

| Field | Value |
|---|---|
| Auditor | W05-A03 |
| Date | 2026-11-12 |
| Lines / functions | 541 / 11 production + 8 tests |
| File verdict | REAL (lacing math is genuine and bounds-checked; one S2 cross-file contract lie on BlockGroup keyframes) |
| Reachable from shipped entry point? | YES — `cluster::parse_timecode` ← mod.rs:515 (`load_next`) and mod.rs:874 (`landing`); `cluster::parse_block_with` ← mod.rs:706/740 (`packets_from_element`); `cluster::absolute_ticks` ← mod.rs:782 (`emit_block`); chain to CLI as in the mod.rs audit (`containers/mod.rs:57` → `containers/mkv.rs:147`). `parse_block` (non-scratch wrapper, line 155) is test-only (grep: no production caller). |
| Findings | S0:0 S1:0 S2:1 S3:2 S4:2 |

Commands run (adaptation: no cargo; static analysis + `sed -n` quote verification):
- `sed -n` verification of every quote below (done, verbatim)
- Full read of this file; cross-read of `ebml.rs` (vint grammar), `mod.rs` (callers), W00 baseline for run-dependent context

## 1. One-paragraph verdict

The cluster/block/lacing layer is real, careful parsing code: the three lacing codings are implemented with per-size bounds guards, 255-continuation accumulation caps, checked delta arithmetic, and typed mismatch refusals; frame tables are bounded (≤ 255 frames/block) and the scratch-recycling design is actually wired. The unit tests assert concrete frame tables and typed error codes, so they fail if the math breaks. The two real defects: the documented BlockGroup keyframe derivation ("ReferenceBlock absence") is claimed in two places and implemented nowhere, so every BlockGroup packet is emitted `keyframe=false`; and the `Timecode` element's u64→i64 `as` cast silently wraps hostile 8-byte values negative. A 32-bit-only `sum()` wrap in `build_frames` and duplicated vint-width logic are latent lower-severity issues.

## 2. What actually works (verified)

- Xiph lacing: count byte (≤ 255), 255-continuation sizes bounded by `if size > body.len() { return Err(at); }` (lines 302-306) — no overflow, no unbounded loop (a size-only body terminates at `body.get(at) == None`). Remainder-as-last-frame rule enforced via `declared > rest` refusal in `build_frames:399-403`.
- EBML lacing: first size `usize::try_from(v + 1)` with typed refusal (line 332) — no `as` truncation; deltas use `checked_add` + negative refusal + per-size `current > body.len()` bound (lines 346-354); the delta bias `2^(7n−1)−1` matches libmatroska/ffmpeg (test at 496-510 pins it with concrete bytes).
- Fixed lacing: exact-division refusal (line 264), ≤ 255 frames, no division by zero (`frame_count == 0` refused at 260-262).
- `absolute_ticks` (428-436): `checked_add` + negative-sum refusal — the doc's timeline check is real, and mod.rs's `emit_block` adds the per-laced-frame step with `checked_add` + `saturating_mul` (mod.rs:816-818), so no timestamp overflow panic.
- Lacing frame counts are structurally bounded at 255 (count byte) — no unbounded frame-table allocation; `reserve` calls are ≤ 255 entries.
- Unit tests: would fail if the parse logic broke (concrete lengths/starts/error codes asserted) — see §5.

## 3. Findings

### C-001 [S2] BlockGroup keyframe derivation is documented in this file and in mod.rs, but implemented nowhere — every BlockGroup packet is emitted `keyframe: false`
Where: crates/demux/src/mkv/cluster.rs:58-59 (the contract), crates/demux/src/mkv/mod.rs:751-753 (the claim), crates/demux/src/mkv/mod.rs:739-749 (the missing derivation)
Code (cluster.rs:58-59):
```rust
    /// The keyframe flag (SimpleBlock bit 0x80; BlockGroup: derived
    /// from `ReferenceBlock` absence by the caller).
    pub keyframe: bool,
```
Code (mod.rs:751-753):
```rust
                    // BlockDuration/ReferenceBlock: the keyframe
                    // derivation is ReferenceBlock-absence; duration
                    // rides DefaultDuration.
```
Code (mod.rs:739-749 — the whole Block handling; `ID_REFERENCE_BLOCK` never inspected):
```rust
                    if cid == ebml::ID_BLOCK {
                        let mut block = cluster::parse_block_with(
                            &payload[cstart..cend],
                            payload_offset + cstart as u64,
                            false,
                            &mut self.block_scratch,
                        )?;
```
What is wrong: `parse_block_with(..., simple=false, ...)` sets `keyframe: simple && flags & FLAG_KEYFRAME != 0` (cluster.rs:217) — always `false` for BlockGroup. The "caller" that the doc says derives the flag walks the BlockGroup children (mod.rs:722-755) and never reads `ebml::ID_REFERENCE_BLOCK` (constant exists at ebml.rs:140, grep confirms zero read sites). So a keyframe stored as a BlockGroup **without** ReferenceBlock — the pre-SimpleBlock muxer convention still produced by older mkvmerge and several hardware transcoders — is demuxed as a non-keyframe.
Why it matters: `Packet.keyframe` (mod.rs:825) is the flag downstream seek/decode logic uses to place a landing safely. For BlockGroup-carrying files every packet is a non-keyframe: engines that skip to the next keyframe after `reanchor` find none (stall or decode-from-non-keyframe corruption), and any keyframe-based UI/trick-play logic sees an all-non-keyframe stream. The comments assert behavior the code does not have — the exact doc-vs-code lie this audit hunts.
What correct looks like: in mod.rs's BlockGroup walk, track `saw_reference = cid == ebml::ID_REFERENCE_BLOCK`, then after the Block parse set `block.keyframe = !saw_reference` before `emit_block` (and a regression test with a keyframe BlockGroup + a referenced one).
Confidence: HIGH — Verification: read (grep: `ID_REFERENCE_BLOCK` has no read site; both doc locations quoted verbatim).

### C-002 [S3] `parse_timecode` wraps a hostile u64 timecode to negative via `as i64` — silent misclassification instead of a typed refusal
Where: crates/demux/src/mkv/cluster.rs:131-137
Code:
```rust
            let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
            let value =
                ebml::uint_from_be(&bytes).ok_or_else(|| error::bad_structure(child.data_pos))?;
            return Ok(ClusterTimecode {
                ticks: value as i64,
```
What is wrong: `uint_from_be` accepts up to 8 bytes → values up to u64::MAX; `value as i64` wraps anything above i64::MAX to a negative `ticks`. The demuxer then fails every block of that cluster in `absolute_ticks` (negative-sum refusal, cluster.rs:432-434) and damage-skips it — graceful, but the error the user sees is "timecode overflow" per block instead of one typed "bad timecode" at the source, and moderately-large-but-valid timecodes (e.g. 2^63 + small) silently masquerade as negatives.
Why it matters: `parse_info` and `parse_tracks` in header.rs use range-checked parses (`scale <= u32::MAX` etc.); this is the one unchecked narrowing in the mkv value path. Same wrap pattern exists in cues.rs:131 (`v as i64` on CueTime — filed as an open question in the mod.rs audit).
What correct looks like: `i64::try_from(value).map_err(|_| error::bad_structure(child.data_pos))?`.
Confidence: HIGH — Verification: read (cast semantics; downstream negative check verified at cluster.rs:432-434).

### C-003 [S3] `build_frames`' `sizes.iter().sum()` can wrap on 32-bit targets (first EBML size is pushed unbounded)
Where: crates/demux/src/mkv/cluster.rs:332-333 (unbounded push), 351-353 (bound applies only to delta sizes), 396 (the summing site)
Code:
```rust
    let mut current = usize::try_from(v.0.checked_add(1).ok_or(at)?).map_err(|_| at)?;
    sizes.push(current);
```
(the bound `if current > body.len()` at 352-354 sits inside the delta loop, which never revisits `sizes[0]`), and:
```rust
    let declared: usize = sizes.iter().sum();
```
What is wrong: On 64-bit, `declared` cannot overflow (sizes[0] ≤ 2^56, others ≤ body.len() ≤ 32 MiB, ≤ 254 of them) and the `declared > rest` refusal at 399-403 catches the oversized first size — correct typed refusal. On 32-bit usize (wasm32 — `apps/web/Cargo.toml:64` builds this crate for wasm32-unknown-unknown), `sizes[0]` can be up to 2^32−1 and the sum of it plus the body-bounded sizes can wrap u32: a wrapped-small `declared` passes the `declared > rest` check and produces a frame table with `start`/`len` beyond the block body.
Why it matters: No panic and no OOM results (mod.rs:802 damage-skips frames > 16 MiB; positional reads stay file-bounded), but the parse silently fabricates garbage frames instead of refusing — a correctness hole on the web target the moment MKV is wired there (today `apps/web/src/open.rs` wires AVI only, so UNREACHABLE from shipped entry points).
What correct looks like: bound the first size like the deltas (`if current > body.len() { return Err(at); }` right after line 333), or use `usize::checked_add` folding in `build_frames`.
Confidence: MEDIUM — Verification: read (arithmetic derivation; 32-bit lane not executed).

### C-004 [S4] Duplicated vint-width walk in `parse_block_with` with no independent cap
Where: crates/demux/src/mkv/cluster.rs:183-194
Code:
```rust
        let mut len = 1usize;
        let mut mask = 0x80u8;
        while data[0] & mask == 0 {
            mask >>= 1;
            len += 1;
        }
```
What is wrong: This re-implements `ebml::parse_vint_size`'s marker walk (ebml.rs:217-226) minus its `len > 8` refusal. It is safe today only because line 181 first ran `parse_vint_id` (which rejects zero lead bytes and >4-octet patterns, guaranteeing this loop stops at len ≤ 4). The safety is inherited, not local — any future edit that removes or reorders the `parse_vint_id` call turns this into an unbounded loop and then a slice panic at `data[1..len]`.
Why it matters: Copy-paste divergence hazard in the one function every packet flows through.
What correct looks like: A `vint_value(data)` helper in ebml.rs (value-with-marker-stripped) reused by both `ebml_sizes`' `parse_vint_value` and this site.
Confidence: HIGH — Verification: read (guard chain derived: parse_vint_id at 181 precedes the loop).

### C-005 [S4] Zero-length final frame accepted; `frame_count == 0` check dead in the Ebml arm
Where: crates/demux/src/mkv/cluster.rs:404-418 and 247-251
Code:
```rust
    let last = rest - declared;
```
and
```rust
        Lacing::Ebml => {
            let (frame_count, consumed) = ebml_sizes(body, &mut scratch.sizes)?;
            if frame_count == 0 {
                return Err(consumed);
            }
```
What is wrong: (a) When `rest == declared`, `last == 0` and a zero-byte frame is pushed downstream (emit_block mints a zero-length packet via the pool's `want == 0` static-empty path, riff.rs:592-596) — degenerate input tolerated silently rather than refused; harmless but undocumented. (b) `ebml_sizes` already refuses `frame_count == 0` at its own line 321-323, so the re-check at 249-251 is unreachable in the Ebml arm (it is live only for Xiph, whose `xiph_sizes` does not reject count 0).
Why it matters: Dead branch suggests a state that cannot occur; zero-length packets reach the packet queue unflagged.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts `FLAG_KEYFRAME`/`FLAG_LACING_MASK` | 26-28 | REAL | — | Spec values (0x80 / 0x06); verified against lacing decode at 200. |
| `struct BlockFrame` | 31-36 | REAL | — | Body-relative offsets; consumed as `payload_offset + body_offset + start` (mod.rs:807) — verified consistent. |
| `struct BlockScratch` | 43-49 | REAL | — | Scratch recycling verified end-to-end: mod.rs:715/748 hand frames back. |
| `struct ParsedBlock` | 52-75 | REAL (doc defect) | S2 | `keyframe` doc (58-59) claims caller-side derivation that does not exist — C-001. |
| `enum Lacing` | 78-88 | REAL | — | Spec bit mapping verified at 200-205. |
| `struct ClusterTimecode` | 91-97 | REAL | — | Data holder. |
| `parse_timecode` | 108-143 | REAL (defect) | S3 | C-002: `value as i64` wrap. Unknown-size child → typed refusal (119-124). len ≤ 8 enforced (128-130). Non-Timecode children skipped by declared end — no recursion. Absence → typed refusal (142). |
| `parse_block` | 155-161 | REAL | — | Test-only convenience wrapper (grep: no production caller) — thin, honest. |
| `parse_block_with` | 167-224 | REAL (defect) | S4 | Header bounds checked (174-176, 195-197); C-004 duplicated width walk; unreachable lacing arm is documented defensive code (206-209). Track vint value decode correct (marker stripped). |
| `lace_frames` | 229-278 | REAL | S4 | Dispatch per coding; Fixed path division-safe (frame_count ≥ 1 enforced 260-262); C-005b dead zero-check in Ebml arm. |
| `xiph_sizes` | 283-311 | REAL | — | 255-continuation guarded (302-306); sizes ≤ 255 entries; `body.get` bounds every read. |
| `ebml_sizes` | 316-357 | REAL (defect) | S3 | C-003: first size unbounded before `build_frames` (32-bit wrap); delta math checked (`checked_add`, negative refusal, per-size bound); bias formula verified against test 496-510. |
| `parse_vint_value` | 360-384 | REAL | — | len ≤ 8 cap, `at + len > data.len()` bound — no panic, no truncating cast (returns u64; callers use try_from). |
| `build_frames` | 389-420 | REAL (defect) | S3/S4 | C-003 sum site; C-005a zero-length last frame; 64-bit `declared > rest` refusal correct. |
| `absolute_ticks` | 428-436 | REAL | — | checked_add + negative refusal; consumed at mod.rs:782 with per-frame `checked_add` on top. |
| tests (8) | 438-541 | REAL | — | See §5. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `no_lacing_single_frame` | YES | Asserts track/relative/keyframe/lacing + exact frame byte range. |
| `xiph_lacing_splits_and_remainder_last` | YES | Asserts 3 concrete lengths (5/300/160) + start offset — pins the 255-continuation decode. |
| `fixed_lacing_divides_exactly` | YES | Asserts 4 frames × len 2. |
| `ebml_lacing_sizes_decode` | YES | Asserts 3/3/2 + remainder — pins the −1 offset and delta bias. |
| `lacing_overrun_is_typed_mismatch` | YES | Declared sizes exceed body → asserts `demux.mkv.bad-structure`. |
| `fixed_lacing_remainder_is_refused` | YES | Non-multiple body → asserts typed refusal. |
| `short_block_header_is_typed` | YES | 2-byte block → typed refusal. |
| `absolute_ticks_checks_the_timeline` | YES | Negative sum and i64::MAX overflow both asserted Err. |

Coverage gaps (verified by reading): no test for Xiph count=0; no test for the 255-continuation runaway guard (302-306); no test for a negative EBML delta (`next < 0`); no test for BlockGroup keyframe semantics (C-001 — untestable today, the feature does not exist); no test for a first-EBML-size larger than the body (the C-003 32-bit hazard). Mutation reasoning (not run — no cargo): flipping `declared > rest` to `>=` would fail `xiph_lacing_splits_and_remainder_last` only when last==0 — the boundary is otherwise untested; flipping the delta bias would fail `ebml_lacing_sizes_decode`.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A block whose header/payload crosses the cluster end … is the typed truncation … never a partial frame handed downstream" | cluster.rs doc, lines 6-8 | REAL (in this file) | Bounds enforced here per-block; the truncation classification lives in mod.rs — but see mod.rs F-001: mod.rs misfires that refusal on valid straddling headers. This file's half of the contract holds. |
| "Relative timecodes are i16 … checked against the timeline (negative or overflowing = typed timecode_overflow)" | cluster.rs doc, lines 15-18 | REAL | `absolute_ticks` 428-436 verified. |
| "Lacing mismatches … are typed bad-structure at the block offset" | cluster.rs doc, lines 11-14 | REAL | build_frames 399-403, fixed 264-267, xiph/ebml Err paths — all typed with offset context (`bad(at)` at 173). |
| "BlockGroup: derived from ReferenceBlock absence by the caller" | cluster.rs:58-59 | FALSE | C-001 — the caller (mod.rs:722-755) never reads ID_REFERENCE_BLOCK. |

## 7. Dead / duplicated / unreachable code

- C-005b: dead `frame_count == 0` check in the Ebml arm of `lace_frames` (247-251).
- `parse_block` wrapper: production-dead (tests only) — kept as public convenience; not hidden, not fraud.
- C-004: duplicated vint width walk.
- The `_ => return Err(bad(...))` lacing arm (206-209) is unreachable but honestly documented as such — acceptable defensive code, not a finding.

## 8. Dependencies & flags

No feature flags; compiled unconditionally with the `mkv` module (lib.rs:50). Depends only on `ebml.rs` primitives + `error.rs` constructors + `media_types::EngineError`. No unsafe, no I/O (pure in-memory parse — the reader is passed only to `parse_timecode`).

## 9. Open questions for the next agent

- Who consumes `Packet.keyframe` downstream (engine seek flow, apps/desktop)? C-001's user-visible blast radius (stall vs. corrupt frame after seek) depends on that consumer's policy — files in the engine/pipeline crates.
- cues.rs:131 `v as i64` CueTime wrap (same family as C-002) — cues.rs belongs to another agent; recommend they file it.
- Does any muxer-side or fixtures generator in this repo emit BlockGroups with keyframes (to build the C-001 regression test)? Check `tools/` and `crates/demux/fixtures/`.
