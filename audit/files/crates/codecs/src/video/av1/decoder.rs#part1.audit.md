# AUDIT: crates/codecs/src/video/av1/decoder.rs#part1 (lines 1-521)
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2026-11-21 |
| Lines / functions | 521 (of 1043) / 14 (+2 consts blocks; chunk boundary: `emit_picture` starts at 506 and continues into part2 at 522 — geometry guard in my range, pixel-copy/pool in A08's) |
| File verdict | REAL (implemented, reachable, executed via baseline differential + conformance suites) |
| Reachable from shipped entry point? | YES — `apps/cli/src/pipeline_host.rs:314-317` `"av1" \| "av01" => { let mut decoder = Av1Decoder::new(); … }`; `codec-av1` is in codecs' default feature set (crates/codecs/Cargo.toml:50, :66 `codec-av1 = ["dep:rav1d"]`), release.yml builds `lumen-cli` with default features (release.yml:193). Executed: baseline `test-workspace.log.txt:891-897` av1_differential 6/6 ok (incl. `differential_pixel_md5_against_ffmpeg_dump`, `midstream_corruption_resyncs_and_recovers`), :885 av1_conformance 1/1 ok (14.05 s of real decode). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | Read 1-521 + test tail 971-1043; grep fn/test map; baseline log av1 lines (519-534, 873-897); ls ~/.cargo/registry/src → empty (rav1d source unavailable → ABI-trust items UNVERIFIED). |

## 1. One-paragraph verdict
This is the strongest file audited so far: a carefully-scoped C-ABI seam over rav1d with per-call SAFETY comments, a documented containment gate for a known upstream abort bug, bounded bookkeeping (PENDING_CAP with oldest-drop aging, SEND_RETRY_CAP, pooled slots), and — decisive — real execution evidence: the baseline differential suite decoded fixtures and compared pixel MD5s against ffmpeg dumps, exercised mid-stream corruption resync, and pinned the 10-bit typed refusal, all passing. The one structural weakness: `drain_pictures`' error branch loops without any iteration cap, trusting that rav1d latches each error exactly once — the very engine whose port this module exists to contain has a documented history of violating such invariants.

## 2. What actually works (verified)
— Baseline `av1_differential.rs` 6/6 ok (`test-workspace.log.txt:891-897`): `differential_pixel_md5_against_ffmpeg_dump` (decode actually ran), `midstream_corruption_resyncs_and_recovers` (the reference gate + drain error path executed; `gate_skips == 2` asserted at decoder-adjacent av1_differential.rs:460-461), `tenbit_fixture_is_a_typed_refusal_not_a_misdecode`, `error_paths_are_typed_and_per_packet`. `av1_conformance.rs` 1/1 ok (:885, 14.05 s).
— Unit tests for my range's pure functions all real and passing (:519-522): `descriptor_gate_accepts_the_delivery_vocabulary`, `eagain_classification_is_platform_correct` (discovers the platform's WouldBlock errno via std's own mapping — would catch sign/mapping bugs), `damage_folds_into_recovered` (exact outcome equality incl. accumulation and zero-damage no-op), `pending_map_bounded_and_lifo_tolerant` (fills to PENDING_CAP, asserts cap, out-of-order pop, double-pop → Missing).
— `feed_loop`'s EAGAIN discipline and `SEND_RETRY_CAP=64` bound: correct dav1d send/get choreography (send EAGAIN → drain → retry; cap → typed `av1.feed.stalled`).
— `feed_tu` ownership cleanup: unref of the data buffer only when `data.r#ref.is_some()` after the loop (line 387) — leak-free on both success and failure paths, per the documented dav1d ownership-transfer semantics (UNVERIFIED against source, see F-002).

## 3. Findings
### F-001 [S2]
Where: crates/codecs/src/video/av1/decoder.rs:455-501
Code:
```rust
            // Bitstream damage on one frame (EGeneric/EIO/ERANGE/…):
            // rav1d latches the error for exactly one get call; the
            // next TU resyncs. Count and keep draining.
            *damage += 1;
        }
```
What is wrong: `drain_pictures` is an unbounded `loop {}` whose only exits are success, EAGAIN, EINVAL, −ENOMEM, or an emit error. The fall-through damage branch (`*damage += 1`) assumes `dav1d_get_picture` latches each error exactly once and returns EAGAIN on the next call. If the engine ever repeats a non-{EAGAIN,EINVAL,ENOMEM} error without consuming the errored picture (a persistent context error, or another port bug — this same port has the documented abort-class bug the module's reference gate exists to contain, module docs lines 44-61), the loop spins forever: a hang, with `damage` wrapping in release builds (u32 overflow) or panicking in debug after 2^32 iterations.
Why it matters: the module's own damage philosophy promises "typed, counted, bounded, never an abort" (line 58) — this loop is the one unbounded structure in the file; `feed_loop` 30 lines up caps the identical ping-pong concern at `SEND_RETRY_CAP = 64`. The asymmetry is the tell.
What correct looks like: a `DRAIN_RETRY_CAP` (e.g. 64 pictures + errors per call) after which return Ok/typed-stalled, mirroring feed_loop.
Confidence: MEDIUM — Verification: read; the "latches exactly once" invariant is UNVERIFIED against rav1d source (registry cache empty in sandbox); the hang is conditional on that invariant failing, but the code's job is to not trust this engine.
### F-002 [S3]
Where: crates/codecs/src/video/av1/decoder.rs:167-173
Code:
```rust
// SAFETY: `Dav1dContext` is `RawArc<Rav1dContext>` — an Arc handle
// that is `!Send` only because raw pointers are. rav1d upstream
// asserts `unsafe impl Send/Sync for Rav1dContext` (1.1.0's
// src/internal.rs: the context is Arc/Mutex/Atomic plumbing), so
// moving the single owned handle across threads is sound for exactly
// the same reason `Arc<Rav1dContext>: Send` would be.
unsafe impl Send for Rav1dHandle {}
```
What is wrong: a load-bearing soundness claim (`Box<dyn Decoder>` crosses threads in the engine) justified by a citation into rav1d 1.1.0's source that cannot be verified in this sandbox (`~/.cargo/registry/src` is empty; no network; cargo forbidden). Same class of unverifiable ABI-trust points in my range: all-zero `Dav1dSettings` validity (181-188), the zeroed-logger `None`-niche claim (199-203), and `dav1d_send_data` nulling `data.r#ref` on consumption (387-397).
Why it matters: if any citation is wrong, the result is unsound Send (data race), UB from an invalid settings struct, or a double-unref UAF — none of which any test would catch deterministically before shipping.
What correct looks like: vendored or CI-side verification (a `cargo test` asserting the rav1d source contains the impl, or a doc-linked blob), or pinning the claims in a checked-in copy of the relevant rav1d source excerpts.
Confidence: MEDIUM — Verification: UNVERIFIED (blocked: dependency source unavailable); SAFETY comments themselves are thorough and internally consistent.
### F-003 [S4]
Where: crates/codecs/src/video/av1/decoder.rs:135-138, 486
Code:
```rust
/// `ENOMEM` on every platform we ship (Linux, Windows, macOS, the
/// BSDs — errno 12 everywhere; rav1d returns it negated on picture
/// allocation failure).
const ERRNO_ENOMEM: c_int = 12;
```
What is wrong: EAGAIN and EINVAL are classified platform-correctly through `std::io::Error`'s own mapping (lines 145-159, deliberately "without a libc edge"), but ENOMEM is a hard literal compared against `-12`. Mixed disciplines in one file; on any platform where the claim is wrong, memory exhaustion silently falls into the bitstream-damage branch (counted corruption, drain continues) instead of the typed ResourceLimit error.
Why it matters: the claim is true for all four shipped platforms (checked against known errno tables), so this is drift-risk, not a live bug.
What correct looks like: `errno_kind(code) == OutOfMemory` (Rust 1.83+ maps ENOMEM to `ErrorKind::OutOfMemory`) — same trick as the other two.
Confidence: HIGH — Verification: read + Rust ErrorKind knowledge.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `errno_kind` / `is_eagain` / `is_einval` | 145-159 | REAL | — | `unsigned_abs` total (no i32::MIN panic); i32::MIN classifies Uncategorized as documented; tested |
| `Rav1dHandle` + `unsafe impl Send` | 163-173 | REAL | S3 | F-002 (unverifiable citation) |
| `Rav1dHandle::open` | 180-234 | REAL | — | zeroed settings + explicit null-logger both SAFETY-documented (UNVERIFIED claims, F-002); ctx_out-None ABI-violation arm typed (222-232) |
| `Rav1dHandle::get` | 240-242 | REAL | — | Copy handle; None only post-Drop |
| `Drop for Rav1dHandle` | 245-256 | REAL | — | close-exactly-once via Option::take; SAFETY documented |
| `Av1Decoder::new` / `accepts` | 287-295 | REAL | — | `av1`\|`av01`; tested |
| `take_damage` | 299-310 | REAL | — | saturating fold; tested incl. accumulation + no-op |
| `push_pending` | 324-333 | REAL | — | PENDING_CAP oldest-drop; tested at cap |
| `pop_pending` | 324-333 | REAL | — | O(n) search bounded 1024; double-pop → Missing; tested |
| `feed_tu` | 339-399 | REAL | — | null-ptr alloc check typed; non-overlapping copy; unref-if-unsent cleanup (387-397); executed via differential suite |
| `feed_loop` | 403-436 | REAL | — | SEND_RETRY_CAP bound; EAGAIN→drain→retry; non-EAGAIN → classify (part2) |
| `drain_pictures` | 440-502 | REAL | S2 | F-001 unbounded loop; EINVAL/−ENOMEM typed; emit-then-unref-before-`?` ordering correct (no picture leak on unsupported layout) |
| `emit_picture` (head; body in part2) | 506-521 | REAL | — | w/h ≤ 0 typed Corrupt; boundary at 522 |

## 5. Test honesty (unit tests live at 971-1043, part2's range; they cover MY functions)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| descriptor_gate_accepts_the_delivery_vocabulary | YES | positive+negative asserts |
| eagain_classification_is_platform_correct | YES | discovers platform errno via std, asserts negated classification both ways |
| damage_folds_into_recovered | YES | exact outcome Eq ×3 (fold, accumulate, no-op) |
| pending_map_bounded_and_lifo_tolerant | YES | cap enforced, out-of-order pop, re-pop Missing |
| (integration) av1_differential 6 + av1_conformance 1 | YES | real decode + MD5-vs-ffmpeg + corruption resync + 10-bit refusal (baseline :885-897, all ok) — these execute feed_tu/feed_loop/drain_pictures for real |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the repo's second sanctioned unsafe exception … for this file only" | decoder.rs:10-13, 93 | REAL | `#![allow(unsafe_code)]` at :93; workspace deny elsewhere (root Cargo.toml:53-54); idct_simd the first (verified by grep of allow sites) |
| rav1d admitted asm-off, default-features off | decoder.rs:22-28 | REAL | crates/codecs/Cargo.toml:125 `rav1d = { … default-features = false, features = ["bitdepth_8","bitdepth_16"] }` |
| "confirmed process abort" in rav1d on broken reference state | decoder.rs:46-50 | REAL-UNPROVEN | UNVERIFIABLE (no rav1d source); the CONTAINMENT is proven by `midstream_corruption_resyncs_and_recovers` (baseline ok, gate_skips==2) |
| "typed, counted, bounded, never an abort" (gate) | decoder.rs:56-58 | REAL | differential corruption test; but "bounded" is weakened file-wide by F-001's uncapped drain loop |
| "8-bit 4:2:0 I420 only; typed refusal otherwise" | decoder.rs:73-80 | REAL | `tenbit_fixture_is_a_typed_refusal_not_a_misdecode` (baseline ok) |
| "One packet = one temporal unit … validated before the engine sees bytes" | decoder.rs:30-35 | REAL | obu tests :524-534 all ok (A08's obu.rs audit) |

## 7. Dead / duplicated / unreachable code
None found in my range. `accepts` is called from configure (part2, 779+); all helpers have callers.

## 8. Dependencies & flags
rav1d =1.1.0, `default-features = false`, bitdepth_8/16 (codecs/Cargo.toml:125) — matches the module's admission claims. `#![allow(unsafe_code)]` module-level (:93). Unsafe sites in my range: zeroed settings ×2 (188, 203), dav1d_open (207), dav1d_close (253), dav1d_data_create (363), copy_nonoverlapping (378), dav1d_data_unref (396), dav1d_send_data (416), dav1d_get_picture (460), dav1d_picture_unref (469) — every one carries a SAFETY comment; invariants are locally plausible; the upstream-behavior claims are the UNVERIFIED residue (F-002).

## 9. Open questions for the next agent
- A08 (part2): does `configure` map ThreadBudget to n_threads defensively (0/negative)? Does decode()'s gate re-check `refs_valid` before EVERY feed, including after flush?
- A08 (part2): `pooled_frame_slot`/`copy_plane` — check the pooled-slot reuse vs a sink that retains frames longer than FRAME_SLOTS=4 (overwrite-under-reader hazard).
- REQUEST-ORCH-RUN: `rg -n "unsafe impl Send" ~/.cargo/registry/src/*/rav1d-1.1.0/src/internal.rs` (or vendor the excerpt) to close F-002's verification gap.
