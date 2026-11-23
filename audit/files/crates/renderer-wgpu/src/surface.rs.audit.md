# AUDIT: crates/renderer-wgpu/src/surface.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 594 / 10 prod + 7 tests |
| File verdict | REAL (headless surface executed in baseline; typed errors exact-pinned) |
| Reachable from shipped entry point? | YES — apps/cli/Cargo.toml:60 + apps/desktop/Cargo.toml:42 depend on renderer-wgpu; pipeline_host.rs imports `renderer_wgpu::renderer::Renderer` + surface types; renderer.rs:88 constructs `HeadlessSurface::new`; backend.rs:104/230 SoftwareCompositor drives `fill_rect`/`write_rect` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg reachability + Rect-caller traces; sed quote verification; baseline cite |

## 1. One-paragraph verdict
A clean, honest CPU surface implementation. `HeadlessSurface` is a real in-memory framebuffer: construction validates extent and present-mode (headless = Fifo only), `fill_rect`/`write_rect` bounds-check before writing, short upload buffers are a typed `UploadSizeMismatch` (never a partial write), `readback_rgba` un-swizzles the native format, and resize reallocates cleared (non-fatal). The unit tests assert exact error variants and byte-level BGRA native order (baseline :5245-5251, 7/7 ok). The only defects are two hardening/consistency nits in the bounds helper, neither reachable from current callers without an upstream placement-math bug.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:5245-5251: all 7 surface tests ok, incl. `fill_write_readback_round_trip_through_native_order` (asserts native buffer bytes `&[0, 0, 255, 255]` for red — real BGRA check) and `write_rect_rejects_short_buffers_and_bad_rects` (exact `expected: 16, found: 7`).
- Reachability chain verified by rg (see table).
- `renderer-wgpu/tests/zero_alloc_present.rs` and `backend_fallback.rs` exist (other agents' files) exercising this surface.

## 3. Findings
F-001 [S4]
Where: crates/renderer-wgpu/src/surface.rs:242-246
Code:
```
        if rect.x + rect.width > self.extent.width || rect.y + rect.height > self.extent.height {
            return Err(RendererError::ZeroExtent {
                extent: rect.extent(),
                from_frame: false,
            });
        }
```
What is wrong: unchecked u32 addition. A `Rect { x: u32::MAX, width: 5, .. }` panics on overflow in debug builds and wraps in release — the wrapped sum (4) passes the bounds check, and the subsequent `base = ... + rect.x as usize * 4` indexes far past `buffer.len()`, panicking in the slice index. So the "typed error, never a panic" contract of the trait docs (lines 149-153) does not hold for wrapping rects. Current callers are safe (backend.rs:104 uses `Rect::covering` with x=0; the frame pass uses placement-computed rects derived from validated extents), so this is a hardening gap, not a live bug. What correct looks like: `u32::checked_add` comparisons. Confidence: HIGH — Verification: read + caller trace; the wrapping path itself UNVERIFIED (not executed — would require a hostile Rect no current caller mints).

F-002 [S4]
Where: crates/renderer-wgpu/src/surface.rs:248-249
Code:
```
        let stride = self.extent.width as usize * self.config.format.bytes_per_pixel() as usize;
        Ok((stride, rect.width as usize * 4))
```
What is wrong: the stride honors `format.bytes_per_pixel()` but the row-bytes hardcodes `4`. Both current `SurfaceFormat` variants are 4 bpp, so behavior is correct today, but a future non-4-bpp format makes `write_rect`'s `needed` computation disagree with the actual write loop (which also assumes 4-byte chunks via `as_chunks::<4>`). Consistency debt. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PresentMode::wgpu_present_mode / is_vsync | 60-73 | REAL | — | name mapping unit-pinned |
| SurfaceConfig::validate | 101-110 | REAL | — | typed PresentModeUnsupported with supported set |
| HeadlessSurface::new | 205-220 | REAL | — | validates mode + extent; `extent.rgba8_bytes()?` typed |
| HeadlessSurface::config / presented_count | 224-232 | REAL | — | accessors |
| HeadlessSurface::checked_span | 235-250 | REAL | S4 | F-001/F-002 |
| RenderSurface::resize (impl) | 266-277 | REAL | — | reallocates cleared |
| fill_rect | 279-290 | REAL | — | per-pixel pack into native format |
| write_rect | 292-312 | REAL | — | short-buffer typed refusal; per-row chunk writes |
| present | 314-319 | REAL | — | saturating counter; trivially synchronous |
| readback_rgba | 321-332 | REAL | — | unpacks native → RGBA, chunks_exact(stride) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| present_mode_wgpu_names_and_vsync_classification | YES | exact strings + booleans |
| config_validation_refuses_unsupported_modes_loudly | YES | exact error enum equality |
| headless_construction_validates_extent_and_mode | YES | exact ZeroExtent + PresentModeUnsupported |
| fill_write_readback_round_trip_through_native_order | YES | pixel-level assertions + raw native buffer byte check |
| write_rect_rejects_short_buffers_and_bad_rects | YES | exact UploadSizeMismatch numbers; OOB/zero rects typed |
| resize_is_non_fatal_and_reallocates | YES | extent + buffer len + cleared bytes |
| rgba_surface_format_skips_the_swizzle | YES | identity bytes both native and readback |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "headless ... runs everywhere a CPU does ... no GPU, no window" | surface.rs:6-9 | REAL | pure Vec<u8> implementation; baseline tests run without GPU |
| "never a silent downgrade" (present-mode validation) | surface.rs:33-35 | REAL | validate() + test |
| "UploadSizeMismatch ... never a partial write" | surface.rs:159-161 | REAL | length check precedes any write (line 295) |
| "a rectangle outside the surface extent" → typed error | surface.rs:151-153 | REAL for sane rects; FALSE for wrapping rects (F-001: release-mode wrap passes check, panics later) | read |

## 7. Dead / duplicated / unreachable code
None. All functions are trait-required or accessors used by tests/renderer.

## 8. Dependencies & flags
Intra-crate only (color/error/params). No features, no unsafe. The crate ships in both lumen-cli and desktop release lanes (Cargo.toml refs above).

## 9. Open questions for the next agent
- backend.rs placement math (other agent): confirm dst.x + dst.width can never wrap given frame-extent validation upstream (F-001's exploitability depends on it).
- The wgpu windowed path: negotiated in backend.rs; this audit only covers the headless implementation.
