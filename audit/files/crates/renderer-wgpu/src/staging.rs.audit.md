# AUDIT: crates/renderer-wgpu/src/staging.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-21 |
| Lines / functions | 589 / 19 (11 prod incl. 2 trait impls + pool, 7 test fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` bin → apps/cli/src/pipeline_host.rs:1290 `accept_video` → `Renderer::new_headless` (:1322) → renderer.rs:189 `render()` → renderer.rs:205 `staging.stage(...)`; executed end-to-end by the baseline AVI first-vertical smoke: 60/60 frames decoded/presented, `audit/runs/baseline/INDEX.md` §"contrast: AVI first-vertical") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "StagingUploader"` → renderer.rs (field :46, ctor :75, use :205) + tests; `rg -n "fn rgba8_bytes" params.rs` → checked_pixels + saturating_mul(4) (overflow-safe); `sed -n 185,215p renderer.rs` → render() pipeline confirmed; baseline: `sed -n 4810,4816p orchrun log` → all 7 staging tests ok; `sed -n 4865,4870p` → zero_alloc_present `present_path_is_zero_allocation_after_warm_up ... ok`; `sed -n` verification of every quote below |

## 1. One-paragraph verdict
The CPU→"texture" upload lane of the v0 software renderer: real, reachable,
executed (AVI first-vertical smoke presented 60 frames through exactly this
`stage()`), and its bounded-pool discipline is genuine — 7 unit tests plus the
counting-allocator proof `zero_alloc_present` all passed in the no-fail-fast
baseline. The error taxonomy (zero extent / unsupported YUV / short buffer) is
typed and tested. The defects are a per-pixel extend loop in the BGRA swizzle
(per-frame hot path, constant-factor but real), a timing metric that quietly
includes pool acquisition while claiming to measure only the copy, an
asymmetric size check that silently accepts over-long buffers, and an unguarded
`u64 as usize` cast. Note the contrast: the sibling zero-alloc claim in the
desktop app (`zero_alloc_transport` mailbox path) FAILED in the same baseline
(608 bytes) — this file's own zero-alloc proof passed, so the renderer lane's
claim stands while the desktop lane's does not.

## 2. What actually works (verified)
— All 7 `staging::tests` ran ok: baseline
`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4810-4816`.
— Zero-alloc proof binary exists and passed: `tests/zero_alloc_present.rs` →
`present_path_is_zero_allocation_after_warm_up ... ok` (orchrun log :4865-4868)
— the W2-06 phantom-evidence correction (test comment :490-495) is now real.
— Pool boundedness verified by reading: `release` caps at `RECYCLE_SLOTS` (92-97),
`acquire` best-fit over ≤3 slots (:73-83), Drop moves the Vec out before the
pool handle dies (:120-128); clone-shares-pool behavior bounded identically.
— Mutation check: making `release` unbounded still passes
`staged_buffers_return_to_the_pool_and_are_reused` (it never asserts pool size),
but breaking drop-return-to-pool fails the zero-alloc proof binary — the
cross-file pin holds the contract.

## 3. Findings
### F-001 [S3]
Where: crates/renderer-wgpu/src/staging.rs:293-298
Code:
```rust
            PixelFormat::Bgra8 => {
                buffer.reserve(needed as usize);
                for pixel in source[..needed as usize].as_chunks::<4>().0 {
                    buffer.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
                }
            }
```
What is wrong: The swizzle loop performs one 4-byte `extend_from_slice` per
pixel — for 1080p that is ~2.07M calls per frame, each building a stack array
and re-checking spare capacity. The module's own claim "the Bgra8 swizzle is
the same single pass" (:16-17) is honest about copy *count* but the pass is
per-pixel, not bulk.
Why it matters: `stage()` runs once per presented frame on the shipped video
path (pipeline_host → render); Bgra8 is the CLI's configured surface format
(pipeline_host.rs:1326 `SurfaceFormat::Bgra8Srgb`), so every MJPEG/AVI frame
presented by the shipped binary pays this loop. A chunked write into
`spare_capacity_mut` (or copy-then-swap-in-place) removes the per-pixel
overhead at identical copy count.
What correct looks like: `buffer.extend_from_slice(source[..needed])` then
an in-place 4-byte stride swap over `spare_capacity`/`chunks_exact_mut::<4>`.
Confidence: MEDIUM (cost is a constant factor, unmeasured here — the stats
fields record it but no baseline asserts timing) — Verification: read; format
confirmed the shipped surface format at pipeline_host.rs:1326.

### F-002 [S4]
Where: crates/renderer-wgpu/src/staging.rs:285-286 (with 302)
Code:
```rust
        let started = Instant::now();
        let mut buffer = self.pool.acquire(needed as usize);
        buffer.clear();
```
What is wrong: `stage_ns_total`/`last_stage_ns` are documented as "wall time
of the staging copies" (:18-20, :203-206), but the stopwatch starts before the
pool acquire (Mutex lock + best-fit scan) and the `clear()`, so the recorded
evidence includes synchronization and pool bookkeeping, not just the copy.
Why it matters: These fields are the card-2.D1 "measured staging" evidence;
conflating lock+copy inflates the copy figure the card asserts on.
What correct looks like: Start the clock after acquire/clear (copy-only), or
document the span as "acquire+copy".
Confidence: HIGH — Verification: read of 285-302.

### F-003 [S4]
Where: crates/renderer-wgpu/src/staging.rs:278-283
Code:
```rust
        if (bytes.len() as u64) < needed {
            return Err(RendererError::UploadSizeMismatch {
                expected: needed,
                found: bytes.len() as u64,
            });
        }
```
What is wrong: Only the short side is checked. A buffer LONGER than
`width × height × 4` is silently accepted and truncated to `needed` by the
`source[..needed]` slices (:291, :295) — no typed error, no diagnostic. The
frame contract says "no padding" (:114), but the upper-side breach of that
contract is invisible, where the lower-side breach is a typed
`UploadSizeMismatch`.
Why it matters: A decoder emitting strided or over-allocated frames would
present silently-wrong pixels (rows read at the wrong stride) instead of
failing loudly — the exact class of corruption the typed-error discipline
exists to catch.
What correct looks like: `!=` instead of `<` (both sides typed), or at least
an upper-bound check with a distinct diagnostic.
Confidence: HIGH — Verification: read of 278-301.

### F-004 [S4]
Where: crates/renderer-wgpu/src/staging.rs:286, 291, 294-295
Code:
```rust
        let mut buffer = self.pool.acquire(needed as usize);
```
What is wrong: `needed` is a `u64` (from `rgba8_bytes`, overflow-checked at
params.rs:45-58) cast to `usize` with `as` at four sites. On a 32-bit target
a frame extent with `width×height×4 > 4 GiB` truncates and the subsequent
slices could panic (slice out of range) or corrupt accounting. `extent.
rgba8_bytes()` did the work of producing a checked u64; the caller discards
the check at the cast.
Why it matters: Unguarded `as` truncation is on the audit checklist; the
practical reach (4 GiB RGBA frame) is nil on 64-bit, but the crate makes no
explicit 64-bit-only claim.
What correct looks like: `usize::try_from(needed)` mapped to a typed
`ResourceLimit`-style error, or a documented 64-bit invariant.
Confidence: HIGH — Verification: read; `rgba8_bytes` read at params.rs:45-47.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `RECYCLE_SLOTS` const | 42 | REAL | — | 3; rationale documented |
| `RecyclePool::new` | 56-60 | REAL | — | Mutex<Vec<Vec<u8>>> |
| `RecyclePool::acquire` | 66-84 | REAL | — | Best-fit scan O(≤3); poisoned lock recovered via into_inner; fallback `pop()` takes the last slot (arbitrary, not smallest — negligible at cap 3) |
| `RecyclePool::release` | 87-97 | REAL | — | Bounded push; full pool drops returnee (documented) |
| `StagedFrame` (struct) | 110-118 | REAL | — | Clone shares pool Arc — bounded, tested |
| `Drop for StagedFrame` | 120-128 | REAL | — | `mem::take` moves buffer to pool before Vec drop — correct |
| `StagedFrame::detached` | 134-141 | REAL | — | `#[cfg(test)]` only — no production pollution |
| `Debug for StagedFrame` | 144-153 | REAL | — | Matches old derive rendering |
| `PartialEq/Eq for StagedFrame` | 155-163 | REAL | — | Pixel equality, pool handle excluded — matches derive semantics |
| `StagedFrame::pixel` | 170-185 | REAL | — | Saturating index math, OOB → opaque black, never panics (tested :460-482) |
| `StagingStats` (struct) | 194-207 | REAL | — | Counts exact; timings recorded-not-asserted (honest) |
| `StagingUploader::default/new` | 219-243 | REAL | — | Fresh pool per uploader |
| `StagingUploader::stats` | 247-249 | REAL | — | Copy out |
| `Debug for StagingUploader` | 228-236 | REAL | — | Stats only |
| `StagingUploader::stage` | 262-315 | REAL | S3 (F-001), S4 (F-002, F-003, F-004) | Validation order zero→format→size; Rgba8 bulk copy; Bgra8 per-pixel swizzle (F-001); stats u64 saturating on ns only |
| test `rgba_upload_measures_exact_sizes_and_copies` | 328-373 | REAL | — | Ran ok (:4813); exact byte/copy counts |
| test `bgra_upload_swizzles_in_the_same_single_pass` | 376-395 | REAL | — | Ran ok (:4810); exact swizzle vector [3,2,1,4,7,6,5,8] |
| test `yuv_formats_are_typed_rejections...` | 398-413 | REAL | — | Ran ok (:4816); Nv12/I420/Yuyv422 all typed-refused |
| test `short_buffers_and_zero_extents_are_typed_errors` | 416-457 | REAL | — | Ran ok (:4814) |
| test `pixel_access_never_panics_on_degenerate_input` | 460-482 | REAL | — | Ran ok (:4812); OOB + empty-buffer cases |
| test `staged_buffers_return_to_the_pool_and_are_reused` | 485-538 | REAL | — | Ran ok (:4815); overlapping frames + steady-state reuse |
| test `hand_built_frames_and_clones_behave_like_plain_owners` | 541-587 | REAL | — | Ran ok (:4811); W2-06 corrected comment preserved |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| rgba_upload_measures_exact_sizes_and_copies | YES | Exact frames/bytes/copies counts and pixel values at corners |
| bgra_upload_swizzles_in_the_same_single_pass | YES | Exact swizzled byte vector; staging_copies == 1 |
| yuv_formats_are_typed_rejections... | YES | Exact error variant for 3 formats |
| short_buffers_and_zero_extents... | YES | Exact UploadSizeMismatch{64,16} and ZeroExtent payloads |
| pixel_access_never_panics... | YES | Panics would fail; exact OOB black |
| staged_buffers_return_to_the_pool... | PARTIAL | Asserts reuse-by-value and pooled-ness (`recycle.is_some()`), never pool size/bound — unbounded `release` passes here; the bound is pinned only indirectly via the (separate, passing) zero-alloc binary |
| hand_built_frames_and_clones... | YES | Equality semantics + clone ownership |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "measured staging: one copy per frame, swizzle same pass" (card 2.D1) | module docs :13-20 | REAL | staging_copies contract tested (:390-394); cost caveat F-001 |
| "v0 always stages through exactly one buffer copy" (zero-copy honesty) | :22-27 | REAL | Single extend per frame; pool reuses buffers; zero_alloc_present passed (orchrun :4867) |
| Zero-alloc across 128 steady-state presents | test comment :490-495 (W2-06 correction) | REAL | `tests/zero_alloc_present.rs::present_path_is_zero_allocation_after_warm_up ... ok` (orchrun :4865-4868); contrast: the DESKTOP mailbox zero-alloc claim FAILED the same baseline (608 bytes, summary item 1) — different lane, no contradiction here |
| YUV upload refused, shader path is the GPU plan (doc 03 §4.11 e) | :25-26, :398-413 | REAL (refusal) / the GPU shader path itself UNVERIFIED here | Typed refusal tested; no GPU backend in shipped build (features: video.software-compositor) |
| `[VERIFY: GPU-side, where a device exists]` | :10-11 | Honest marker | No device path exists in shipped build; marker not yet discharged |

## 7. Dead / duplicated / unreachable code
None found. `detached` is cfg(test)-gated; the `_ =>` arm at :300 is the
type-exhaustiveness sink documented as unreachable (guard at :275 proves it).

## 8. Dependencies & flags
`media_types` (PixelFormat, SharedBytes), `std` (Arc/Mutex/Instant). No
feature gates, no unsafe, no I/O. Pool is process-internal; no cross-thread
contract beyond the Mutex. renderer-wgpu Cargo.toml has no staging-specific
deps.

## 9. Open questions for the next agent
- renderer.rs / compositor auditors: does `composite` read via `pixel()` per
  sample (making F-001's per-pixel cost compound), or bulk?
- Whoever audits apps/desktop transport: the mailbox zero-alloc failure
  (baseline item 1, 608 bytes at zero_alloc_transport.rs:98) — is the desktop
  path bypassing this pool (fresh `StagedFrame` per mailbox hop) or leaking
  elsewhere?
