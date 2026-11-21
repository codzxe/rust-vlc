# AUDIT: crates/media-types/src/packet.rs
| Field | Value |
|---|---|
| Auditor | W02-A04 |
| Date | 2026-11-21 |
| Lines / functions | 589 / 32 (9 SharedBytes methods, 6 trait impls, 1 Packet method, 16 test entries) |
| File verdict | REAL-UNPROVEN (code traced sound; its own test suite has NO executed baseline evidence — see F-004) |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → apps/cli/src/pipeline_host.rs → apps/cli/src/demux_host.rs:581 `DemuxEvent::Packet(packet)` → crates/codec-api/src/decode.rs:204 `packet.is_stale(current)`; SharedBytes views minted on the executed AVI lane by crates/demux/src/avi/riff.rs:623/647/663 `SharedBytes::from_pool_range`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `rg -l "SharedBytes" crates apps` (40+ consumer files); `sed -n '580,670p' crates/demux/src/avi/riff.rs` (pool.fill bounds: `at + want > cap` guard before every from_pool_range); `sed -n '820,895p' crates/demux/src/ogg/mod.rs` + `sed -n '55,140p' crates/demux/src/ogg/page.rs` (slice caller bounds via CRC+stream_len gate); `rg -n "is_stale"` (codec-api/src/decode.rs:204); `rg -n "SharedBytes::build"` (dnx/decoder.rs:583); `rg -n "from_static"` (production callers: riff.rs:595, ogg/mod.rs:640, both `b""`); `rg -n "get_mut"` (audio-engine/src/pipeline.rs:424,433; av1/decoder.rs:747; dnx/decoder.rs:575; apps/cli/src/pipeline_host.rs:1761–1817); `rg -n "Running" audit/runs/baseline/test-workspace*.log.txt` (media-types test binaries absent from both runs); `sed -n '1217,1250p' docs/01-architecture.md` |

## 1. One-paragraph verdict
This is the real thing: a carefully engineered `(Arc<Vec<u8>>, Range<usize>)` zero-copy view type with documented invariants, checked arithmetic at both range constructors, content-based PartialEq/Hash kept mutually consistent, a Debug impl capped at 8 bytes, and an honest cost story (from_vec moves, from_static copies and admits it, build zero-inits then fills). I traced every method for overflow/soundness and found none: `from_pool_range`'s `checked_add` guard precedes the `offset..offset+len` construction, `slice`'s additions are bounded by `self.range.end <= buf.len()`, and `get_mut` is `Arc::get_mut`-gated. The production callers I traced (riff.rs pool.fill, ogg page assembly, codec-api staleness check, audio-engine slot recycling) uphold the panic contracts. What keeps this from REAL: (a) not one of its 16 tests was executed in either baseline run — `cargo test --workspace` fail-fast stopped at package `desktop`/`first-vertical`, and `media-types` sorts after both, so INDEX.md's "1681 passed" headline silently covers only the packages before the stop; (b) the doc-comment layer has three small lies/staleness issues (F-002, F-003, F-005).

## 2. What actually works (verified)
— SharedBytes view minting executed on the shipped AVI first-vertical lane: baseline `audit/runs/baseline/mp4-smoke-bench-avi-mjpeg-pcm.txt` → `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60`; payloads reach Packet.data via riff.rs `pool.fill` → `from_pool_range` (verified by reading crates/demux/src/avi/riff.rs:588-668). — `lumen open` MP4 session pushes Packet objects through the pipeline (baseline `mp4-smoke-open-session.txt`: `event: tracks 2 ready`, decode errors prove packets flowed to the decoder layer). — Verified invariants by reading: no overflow, no unwrap/panic path reachable with in-bounds callers.

## 3. Findings

### F-001 [S3]
Where: crates/media-types/src/packet.rs:83-89
Code:
```rust
    pub fn from_static(bytes: &'static [u8]) -> SharedBytes {
        let len = bytes.len();
        SharedBytes {
            buf: Arc::new(bytes.to_vec()),
            range: 0..len,
        }
    }
```
What is wrong: The name `from_static` promises the `bytes`-crate semantics (adopt a `&'static [u8]` with no allocation and no copy); this always allocates a fresh heap block and copies every byte.
Why it matters: An API consumer writing a hot path will read the name, not the 12-line honesty note, and pay a hidden payload copy per call. Production callers currently dodge this only because both (riff.rs:595, ogg/mod.rs:640) pass `b""` — verified by `rg -n "from_static"` across crates/ and apps/.
What correct looks like: Rename to `from_static_copy`/`copy_from_static`, or provide an `Arc::from(&'static [u8])`-style true-zero-copy constructor documented as the exception.
Confidence: HIGH — Verification: read + rg caller census.

### F-002 [S4]
Where: crates/media-types/src/packet.rs:272
Code:
```rust
    /// Decode timestamp (absent when equal-or-unknown).
    pub dts: Stamp,
```
What is wrong: The parenthetical states a convention ("absent when equal") the shipped demuxers do not follow: crates/demux/src/avi/mod.rs:651-653 sets `dts: Stamp::Present(pts)` with dts == pts (comment: "DTS mirrors PTS"), and mp4/mod.rs:1884 sets `dts: Stamp::Present(...)` when known.
Why it matters: Downstream code that trusts the comment to infer "dts == Absent ⇒ dts == pts" will mis-order packets for demuxers that always emit Present.
What correct looks like: State the actual convention: "Missing only when the container has no decode-order information; demuxers may emit Present with dts == pts."
Confidence: HIGH — Verification: read of avi/mod.rs:640-660, mp4/mod.rs:1884, ogg/mod.rs:1057.

### F-003 [S4]
Where: crates/media-types/src/packet.rs:378-385
Code:
```rust
    fn size_of_recorded_before_and_after() {
        assert_eq!(
            std::mem::size_of::<SharedBytes>(),
            std::mem::size_of::<Arc<Vec<u8>>>() + 2 * std::mem::size_of::<usize>(),
            "thin Arc + Range<usize> (24 on 64-bit; was 16 pre-W6-05)"
        );
        assert_eq!(std::mem::size_of::<SharedBytes>(), 24); // 64-bit pin
    }
```
What is wrong: The second assertion hard-pins a 64-bit layout. On any 32-bit host (or wasm32 test run — the repo ships an apps/web column in the feature matrix), size_of is 12, and the crate's own tests fail.
Why it matters: The first assertion is already self-describing and portable; the pinned duplicate adds no proof and breaks portability of the "most-reviewed, extremely stable" crate (its Cargo.toml words).
What correct looks like: Delete the `== 24` pin or gate it behind `#[cfg(target_pointer_width = "64")]`.
Confidence: HIGH — Verification: read; pointer-width arithmetic.

### F-004 [S3]
Where: crates/media-types/src/packet.rs:297-589 (the entire `mod tests`) + audit/runs/baseline/test-workspace.log.txt (evidence gap)
Code:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::TrackId;
```
What is wrong: None of this file's 16 tests (nor tests/shared_bytes_alloc.rs, nor testkit_wiring.rs) has executed baseline evidence. Both baseline runs fail-fast-stopped at packages sorting before `media-types`: `cargo test --workspace` halted at `desktop::zero_alloc_transport` (log line 2391, last "Running" entry; exit 101) and `--all-features` halted at `first_vertical::evidence` (last "Running" entry, line 2468). `rg -n "Running" … | rg -i media` returns nothing in either log. INDEX.md's headline "1681 passed / 1 failed / 24 ignored" does not disclose that every package after the stop (input, lumen-cli, localization, media-clock, media-probe, media-types, packet-queue, playback-engine, playlists, plugin-*, product-config, renderer-wgpu, settings-schema, streaming, subtitles, testkit, theme-tokens, transcode, video-engine, xtask) never ran.
Why it matters: The counting-allocator proofs that this file's doc-comments lean on ("proved by the counting allocator in tests/shared_bytes_alloc.rs", lines 32-33) are exactly the evidence that was never produced. A future regression in the zero-copy contract would be invisible to the baseline.
What correct looks like: Baseline rerun with `--no-fail-fast` (and/or per-crate runs), INDEX.md amended with the true coverage set. REQUEST-ORCH-RUN filed in W02-A04.log.md: `cargo test -p media-types --locked`.
Confidence: HIGH — Verification: read of both baseline logs (line-level census of 108/116 "Running" entries).

### F-005 [S4]
Where: crates/media-types/src/packet.rs:47-51 vs docs/01-architecture.md:1221
Code:
```rust
    buf: Arc<Vec<u8>>,
    /// The exposed byte range of `buf` (the view). Invariant, asserted
    /// at every construction site: `0 <= start <= end <= buf.len()`.
    range: Range<usize>,
}
```
(docs/01-architecture.md:1221 declares: `pub struct SharedBytes(std::sync::Arc<[u8]>);`)
What is wrong: docs/01-architecture.md §3.3 still declares the pre-W6-05 representation this very file's doc-comment says was replaced (packet.rs:17-26).
Why it matters: The architecture doc is the registry the project points auditors and new agents at; it describes a type shape that no longer exists (no view/range, different copy semantics).
What correct looks like: Update doc 01 §3.3 to the `(Arc<Vec<u8>>, Range<usize>)` view shape (or add a "superseded by W6-05" pointer).
Confidence: HIGH — Verification: read of docs/01-architecture.md:1217-1250 and packet.rs:39-51.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedBytes::from_vec | 63-69 | REAL | — | Moves Vec behind Arc; no copy (traced). Callers: demux asf/wav/mp4/ogg/aiff, transcode. |
| SharedBytes::from_static | 83-89 | REAL | S3 | Copies; misleading name (F-001). Production callers pass only `b""`. |
| SharedBytes::build | 109-119 | REAL | — | Zero-init memset + fill; 2 alloc events. Caller: codecs dnx/decoder.rs:583. |
| SharedBytes::from_pool_range | 144-154 | REAL | — | checked_add guard before range construction — no overflow. Executed on AVI lane (riff.rs:623/647/663). Panics documented, callers bounds-checked (riff.rs fill guards `at + want > cap`). |
| SharedBytes::slice | 170-182 | REAL | — | Assert then add; additions bounded by `range.end <= buf.len()` — no overflow. Executed via ogg/mod.rs:860 (bounds guaranteed by parse_header stream_len check + CRC gate; a source whose `length()` lies could still panic — see §9). |
| SharedBytes::as_slice | 185-187 | REAL | — | Trivial, correct. |
| SharedBytes::len | 190-192 | REAL | — | No underflow (invariant start <= end). |
| SharedBytes::is_empty | 195-197 | REAL | — | Trivial. |
| SharedBytes::get_mut | 213-216 | REAL | — | Arc::get_mut gate; sound. Callers: audio-engine/pipeline.rs:424,433; av1/decoder.rs:747; dnx/decoder.rs:575; apps/cli/pipeline_host.rs:1761-1817. |
| Deref::deref | 219-225 | REAL | — | Range semantics, matches as_slice. |
| PartialEq::eq | 230-234 | REAL | — | Content equality, consistent with Hash. |
| Eq | 236 | REAL | — | Derived marker; eq is total. |
| Hash::hash | 240-244 | REAL | — | Hashes as_slice; consistent with eq. |
| Debug::fmt | 246-255 | REAL | — | Caps at 8 bytes of the VIEW; no panic (min guard); allocates per call (Debug-only, acceptable). |
| AsRef::as_ref | 257-261 | REAL | — | Alias of as_slice. |
| Packet (struct) | 265-287 | REAL | S4 | Doc drift on dts (F-002). Fields executed on shipped lanes. |
| Packet::is_stale | 292-294 | REAL | — | `self.generation < current`; GenerationId is checked-add monotonic u64 (ids.rs:464-478) — no wrap. Caller: codec-api/src/decode.rs:204. |
| tests::sample_packet (helper) | 304-319 | REAL-UNPROVEN | — | Test fixture; never executed in baseline (F-004). |
| tests::shared_bytes_clone_shares_without_copying | 321-329 | REAL-UNPROVEN | — | Asserts pointer equality — would fail on a copying clone. Not executed in baseline. |
| tests::shared_bytes_debug_is_capped | 331-337 | REAL-UNPROVEN | — | Would fail if cap removed (4096 "ab" matches vs 8). Not executed. |
| tests::packet_staleness_follows_generation_order | 339-353 | REAL-UNPROVEN | — | Covers equal/next/new(0); would catch inverted comparison. Not executed. |
| tests::packet_fields_roundtrip_through_clone | 355-363 | REAL-UNPROVEN | — | Clone roundtrip; weak (only catches Clone/PartialEq breakage). Not executed. |
| tests::size_of_recorded_before_and_after | 377-385 | REAL-UNPROVEN | S4 | Layout pin; F-003. Not executed. |
| tests::range_views_expose_only_their_range | 387-412 | REAL-UNPROVEN | — | Strong: deref/index/AsRef/empty-view semantics. Not executed. |
| tests::subslicing_composes_relative_to_the_view | 414-429 | REAL-UNPROVEN | — | Pointer-offset assert; catches offset-composition bugs. Not executed. |
| tests::views_keep_the_allocation_alive_past_the_original_handle | 431-459 | REAL-UNPROVEN | — | strong_count accounting + drop(owner) survival + get_mut recycle. Not executed. |
| tests::get_mut_refuses_to_write_behind_live_views | 461-472 | REAL-UNPROVEN | — | Would catch a get_mut that ignores live views (soundness regression). Not executed. |
| tests::out_of_range_construction_panics_never_clamps | 474-504 | REAL-UNPROVEN | — | catch_unwind ×4 cases + no-clamp residue check. Not executed. |
| tests::equality_and_hash_are_content_not_identity | 506-534 | REAL-UNPROVEN | — | Hash/eq consistency incl. unequal-over-same-allocation. Not executed. |
| tests::build_fills_in_place | 536-550 | REAL-UNPROVEN | — | Content + zero-len edge. Not executed. |
| tests::from_static_copies_content_faithfully | 552-559 | REAL-UNPROVEN | — | Roundtrip + empty. Not executed. |
| tests::debug_shows_the_view_not_the_pool | 561-569 | REAL-UNPROVEN | — | First-byte offset 0x18 assert — catches pool-relative (wrong) Debug. Not executed. |
| tests::pool_owner_can_reclaim_through_try_unwrap | 571-588 | REAL-UNPROVEN | — | Recycle story end-to-end. Not executed. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 16 in-file tests | YES (by construction) | Each asserts observable behavior (pointer identity, strong_count, catch_unwind panics, hash equality), not mocks of the subject. Mutation check (reasoned, not run): making `slice` clamp instead of assert fails `out_of_range_construction_panics_never_clamps`; making `clone` copy fails the pointer-equality assert; making Debug dump the pool fails `debug_shows_the_view_not_the_pool`. |
| tests/shared_bytes_alloc.rs (5 tests, referenced by doc-comments) | YES | Counting-allocator deltas assert exact event/byte counts (e.g. build == 2 events, N + header bytes). BUT: never executed in baseline (F-004) — the doc-comment's "proved by" cites a proof that has no run evidence. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Zero-copy-friendly shared byte buffer (std-only)" | docs/01-architecture.md §3.3 header / packet.rs:9 | REAL | from_vec/build move; slice/from_pool_range are refcount bumps (traced); AVI bench lane decoded 60/60 frames through pooled views (baseline mp4-smoke-bench-avi-mjpeg-pcm.txt). |
| `pub struct SharedBytes(std::sync::Arc<[u8]>);` | docs/01-architecture.md:1221 | FALSE (stale) | packet.rs:47-51 is `(Arc<Vec<u8>>, Range<usize>)`; F-005. |
| Packet field set incl. `generation: GenerationId // stale ⇒ dropped at earliest checkpoint` | docs/01-architecture.md:1223-1231 | REAL | Fields match 1:1; drop-at-checkpoint implemented at codec-api/src/decode.rs:204 (`packet.is_stale`). |
| "clone is the refcount bump only (proved by the counting allocator in tests/shared_bytes_alloc.rs)" | packet.rs:32-33 | REAL-UNPROVEN | Test exists and is rigorous, but no baseline run executed it (F-004). |
| "Empty slices — the only production shape (demux riff.rs zero-size payloads)" | packet.rs:81-82 | REAL | rg census: production from_static callers are riff.rs:595 and ogg/mod.rs:640, both `b""`. |
| "the caller is engine code that has already bounds-checked the chunk it read" (panic contract) | packet.rs:140-142 | REAL (with one caveat) | riff.rs fill bounds-checks (`at + want > cap`, spill `got <= want`); ogg relies on parse_header's stream_len check + CRC — a source reporting a wrong `length()` could reach `slice` with a short body (see §9). |

## 7. Dead / duplicated / unreachable code
None in this file. Every public constructor has at least one production caller (from_vec: demux×6+transcode; from_static: riff/ogg empty; build: dnx decoder; from_pool_range: riff pool; slice: ogg; get_mut: audio-engine/av1/dnx/cli). `Packet::is_stale` called from codec-api decode.rs:204.

## 8. Dependencies & flags
std-only (verified: Cargo.toml `[dependencies]` absent, comment "No dependencies. Deliberately boring"). dev-dependency `testkit` (path) — dev-only, contract documented. No feature gates on any code in this file. `[lints] workspace = true` inherited.

## 9. Open questions for the next agent
1. OGG panic depth: if a source's `length()` over-reports (HTTP content-length mismatch, shrinking file), `pool.fill` returns a short `got`, CRC-32 is linear (attacker-solvable), and `body.slice(*start..*start+*run_len)` at crates/demux/src/ogg/mod.rs:860 could exceed the view → panic. Needs the demux/ogg auditor to confirm `reader.length()` semantics for non-local sources.
2. Did `cargo test -p media-types` ever pass on this toolchain? REQUEST-ORCH-RUN filed; wave gate should paste output.
3. apps/cli/src/pipeline_host.rs:1761 `SharedBytes::get_mut(bytes).must()` — what does `.must()` do on None (panic?) — CLI auditor should check.
