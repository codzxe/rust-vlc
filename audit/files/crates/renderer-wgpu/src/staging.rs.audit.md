# AUDIT: crates/renderer-wgpu/src/staging.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 588 / 13 production fns + 7 tests |
| File verdict | REAL (executed live: 7/7 unit tests + zero-alloc proof binary pass) |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:81 `use renderer_wgpu::renderer::Renderer` → Renderer::render (renderer.rs:205 `let staged = self.staging.stage(bytes, extent, frame.format)?`); apps/desktop/Cargo.toml:42 also depends on renderer-wgpu |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
A small, genuinely engineered CPU→RGBA staging path: bounded 3-slot recycle pool with best-fit reuse, one measured copy per frame (BGRA swizzle folded into the same pass), exact byte/copy counters, saturating pixel access, typed errors for zero-extent/YUV/short-buffer. Everything it claims is either test-verified (I ran the 7 staging tests and the previously-phantom `zero_alloc_present` proof binary — both green) or honestly labeled `[VERIFY: GPU-side]` for the wgpu half. Only hygiene-level notes: the BGRA swizzle is a per-pixel `extend_from_slice` loop (2M calls/frame at 1080p — fine while no production decoder emits Bgra8, a perf cliff the day one does), and the recycle pool retains stale larger buffers without a shrink policy. No correctness defects found.

## 2. What actually works (verified)
- `timeout 110 ./target/debug/deps/renderer_wgpu-cea255c0e8d5c422 staging --test-threads=2` → `7 passed; 0 failed` (exact sizes/copies, BGRA swizzle bytes, YUV typed refusals, short-buffer/zero-extent typed errors, pixel saturation, pool reuse, clone/owner semantics).
- `timeout 110 ./target/debug/deps/zero_alloc_present-a1a58fac9a1f3d5b` → `1 passed` (`present_path_is_zero_allocation_after_warm_up`) — the proof binary re-cited at L490-495 after the W1-20 phantom-evidence finding is real and passing; the citation is now honest.

## 3. Findings

F-001 [S4 — BGRA swizzle is per-pixel extend_from_slice]
Where: staging.rs:293-298
Code:
```
            PixelFormat::Bgra8 => {
                buffer.reserve(needed as usize);
                for pixel in source[..needed as usize].as_chunks::<4>().0 {
                    buffer.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
                }
            }
```
What is wrong: one `extend_from_slice` call per pixel (1920×1080 → ~2.07M calls + a 4-byte stack array per pixel) where a pre-sized `set_len`/chunked `copy_from_slice` pair or `array::map` into a reserved slice would be ~an order faster. No production decoder currently emits `PixelFormat::Bgra8` frames (rg shows only wire-mapping and tests referencing it), so today this is a cold defensive path; it becomes a per-frame multi-ms cost the moment a BGRA source lands.
Confidence: HIGH — Verification: read (path coldness verified by rg; no BGRA-emitting decoder found).

F-002 [S4 — pool keeps oversized buffers; no shrink, stale geometry lingers]
Where: staging.rs:62-84 (acquire), 92-96 (release)
Code:
```
        match best {
            Some(i) => slots.swap_remove(i),
            None => slots.pop().unwrap_or_default(),
        }
```
What is wrong: after a resolution change the pool retains up to 3 old-geometry buffers (released frames return the big Vecs); `acquire` will still reuse them (capacity covers), so peak memory is up to `3 × max(frame-bytes ever seen)` rather than `3 × current-frame-bytes`. Bounded and intentional ("keeps the big buffers big-frame-sized", L72-73), but the doc at L47-49 says "Memory is bounded at `RECYCLE_SLOTS · frame-bytes`" — with a stale big frame in the pool that is `RECYCLE_SLOTS · biggest-recent-frame-bytes`. A doc overstatement of a bounded, minor effect.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| RecyclePool::new | 56-60 | REAL | — | Mutex<Vec<Vec<u8>>> |
| RecyclePool::acquire | 66-84 | REAL | — | best-fit ≥ needed; poison-tolerant lock; O(3) scan |
| RecyclePool::release | 87-97 | REAL | — | bounded at 3, drop-on-full is correct |
| StagedFrame::drop | 120-128 | REAL | — | mem::take avoids double-free of pooled Vec |
| StagedFrame::detached | 134-141 | REAL (cfg test) | — | test builder |
| Debug/PartialEq/Eq | 144-163 | REAL | — | pixel equality, machinery excluded (honest) |
| StagedFrame::pixel | 170-185 | REAL | — | saturating, OOB → opaque black; degenerate-input test |
| StagingUploader::default/new | 219-243 | REAL | — | |
| StagingUploader::stats | 247-249 | REAL | — | Copy-out |
| StagingUploader::stage | 262-315 | REAL | S4 | typed gates; one copy; exact counters; F-001 swizzle loop |
| tests ×7 | 327-587 | REAL | — | all pass live; the phantom-citation history is disclosed in-comment (L490-495) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| rgba_upload_measures_exact_sizes_and_copies | YES | exact byte/copy counts, accumulation |
| bgra_upload_swizzles_in_the_same_single_pass | YES | byte-exact swizzle expectation |
| yuv_formats_are_typed_rejections... | YES | Nv12/I420/Yuyv422 → UploadFormatUnsupported |
| short_buffers_and_zero_extents_are_typed_errors | YES | exact error variants |
| pixel_access_never_panics_on_degenerate_input | YES | OOB + empty-buffer saturation |
| staged_buffers_return_to_the_pool_and_are_reused | PARTIAL | proves reuse-by-content, not allocation-freedom; that part is delegated to zero_alloc_present (which I ran: PASS) |
| hand_built_frames_and_clones_behave_like_plain_owners | YES | clone pool-handle sharing semantics |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the proof binary tests/zero_alloc_present.rs measures exactly zero bytes across 128 steady-state presents" | staging.rs:490-495 | TRUE (now) | ran the binary: 1 passed; the phantom-history is disclosed |
| "Memory is bounded at RECYCLE_SLOTS · frame-bytes" | staging.rs:47-49 | OVERSTATED | F-002: stale bigger buffers retained |
| "hardware path ... [VERIFY: GPU-side, where a device exists]" | staging.rs:9-11 | honest label | no GPU in sandbox; wgpu half unexercised here |
| "one measured staging copy" per frame | staging.rs:15-17 | TRUE | staging_copies counter + tests |

## 7. Dead / duplicated / unreachable code
`PixelFormat::Bgra8` arm is currently production-cold (no decoder emits it); kept as wire-format support. Not dead per contract.

## 8. Dependencies & flags
Crate renderer-wgpu (wgpu feature-gated per its Cargo.toml comment; this file is wgpu-free std code). Consumed by apps/cli + apps/desktop.

## 9. Open questions for the next agent
- backend.rs owner already flagged `StagedFrame::pixel` saturation semantics as a dependency (see audit/files/crates/renderer-wgpu/src/backend.rs.audit.md:81) — my run of backend tests wasn't in scope; the pixel() saturation rule itself is unit-tested here.
