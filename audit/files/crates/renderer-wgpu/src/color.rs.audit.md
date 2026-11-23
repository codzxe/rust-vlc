# AUDIT: crates/renderer-wgpu/src/color.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 176 / 7 (SurfaceFormat::{wgpu_texture_format,bytes_per_pixel,pack,unpack}, Rgba::{from_components,source_over}) + 3 tests |
| File verdict | REAL (with one latent math defect in an unused-yet export) |
| Reachable from shipped entry point? | YES — renderer-wgpu is a dependency of both apps (apps/cli/Cargo.toml:60, apps/desktop/Cargo.toml:42); SurfaceFormat used by renderer.rs (L284, L424) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg callers (source_over: none outside color.rs; SurfaceFormat: renderer.rs); math re-derivation of blend cases; baseline tests cited |

## 1. One-paragraph verdict
The renderer's pixel vocabulary: two sRGB 8-bit surface formats with correct BGRA/RGBA swizzles (round-trip + real-swap pinned by tests), a stringly-typed wgpu enum-name mapping that is a documented pre-dependency shape pin, and a straight-alpha source-over blend whose per-channel math is correct but whose RESULT ALPHA IS HARDCODED 255 — correct only when the `below` layer is opaque. The doc sells it as "unit-tested here so the Phase-5 overlay card consumes a proven function"; it is proven only for opaque below, and it currently has zero non-test callers, so the defect is latent.

## 2. What actually works (verified)
— verified by math re-derivation: blend (200,·,·,128) over black → (25600+127)/255 = 100 ✓; (0,0,0,128) over white → (32385+127)/255 = 127 ✓ — matches the test's pinned values; the rounding is the standard (t+127)/255.
— verified swizzle correctness by read: BGRA pack [b,g,r,a] with the non-identity swap pinned at L150-153.
— baseline: renderer-wgpu crate suites pass (see cargo-test-workspace.txt renderer-wgpu sections; color tests run in the lib binary).

## 3. Findings
F-001 [S3]
Where: crates/renderer-wgpu/src/color.rs:96-119
Code: (verbatim, L113-118)
```
        Rgba([
            blend(r, below.0[0]),
            blend(g, below.0[1]),
            blend(b, below.0[2]),
            255,
        ])
```
What is wrong: the result alpha is a literal 255 — true source-over over a TRANSLUCENT `below` yields alpha = a_top + a_below·(1−a_top) < 255; this function silently makes every composite opaque. The doc comment claims the general primitive ("Source-over alpha compositing of `self` over `below` in 8-bit straight alpha") and that the Phase-5 overlay card "consumes a proven function", but the tests only pin opaque-below cases and the function accepts any `below`. Why it matters: the first translucent-over-translucent use (e.g. subtitles over a transparent letterbox region, or nested overlays) composites wrongly. Currently latent: rg shows NO callers outside color.rs and its tests. What correct looks like: compute the composite alpha and blend with un-premultiplied math (or document + assert `below.0[3] == 255`). Confidence: HIGH — Verification: read + math; no caller to exercise.
F-002 [S4]
Where: crates/renderer-wgpu/src/color.rs:42-50
Code: (verbatim)
```
    pub fn wgpu_texture_format(self) -> &'static str {
        match self {
            SurfaceFormat::Bgra8Srgb => "Bgra8UnormSrgb",
            SurfaceFormat::Rgba8Srgb => "Rgba8UnormSrgb",
        }
    }
```
What is wrong: the hardware mapping is a stringly-typed name (the documented Lead-gated pre-dependency pin); a typo would only surface at the future import site, and the unit test only pins the string against itself. Honest about it (L21-25), but it is a shape pin, not a verified mapping. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SurfaceFormat::wgpu_texture_format | 44-50 | REAL (shape pin) | S4 | F-002 |
| SurfaceFormat::bytes_per_pixel | 53-56 | REAL | - | constant 4, pinned |
| SurfaceFormat::pack / unpack | 59-74 | REAL | - | correct swizzles; round-trip + non-identity pinned |
| Rgba consts + from_components | 84-94 | REAL | - | |
| Rgba::source_over | 100-119 | REAL (latent defect) | S3 | alpha hardcoded 255 (F-001); per-channel math correct |
| tests (3) | 126-175 | REAL | - | behavioral; alpha edges pinned only for opaque below |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| wgpu_format_names_are_the_real_enum_variants | NO (tautology against its own literals) | pins the string to the same string — drift protection only |
| swizzles_round_trip_per_pixel | YES | a wrong swizzle fails round-trip or the swap pin |
| source_over_covers_the_alpha_edges_and_a_middle_case | YES for the opaque-below contract; NO for the general contract | no translucent-below case (F-001 blind spot) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "proven byte-exactly by the headless round-trip AC (frame in == readback out)" | L13-14 | REAL-UNPROVEN here | the AC lives in renderer tests (png_hash/presenter_seam); not re-run in this audit — baseline suites green |
| "unit-tested here so the Phase-5 overlay card consumes a proven function" | L98-99 | OVERSTATED | proven only for opaque below (F-001) |
| "pinned by unit test before the dependency exists" | L23-25 | REAL (as a string pin) | F-002 notes its limits |

## 7. Dead / duplicated / unreachable code
`source_over` is currently unused outside tests (the Phase-5 consumer does not exist yet) — exported ahead of need.

## 8. Dependencies & flags
No wgpu dependency yet (Lead-gated import) — the module is std-only. Crate is wired into both shipped apps.

## 9. Open questions for the next agent
- When the overlay card lands, audit its use of source_over against F-001 first.
- The headless round-trip AC (byte-exact passthrough) is the crate's core color claim — verify in presenter_seam/png_hash suites if assigned.
