//! The card 2.D1 acceptance proof: a headless render of the
//! deterministic synthetic frame produces the **expected PNG hash**
//! (doc 13 card 2.D1 AC; doc 03 §4.11 f "headless … renders frames to
//! an offscreen buffer" + "screenshot tests of the compositing
//! result").
//!
//! The chain under test is the crate's whole public path:
//! synthetic frame → [`Presenter::present`] → measured staging →
//! placement → software composite → sRGB surface → readback → PNG.
//!
//! # Why the committed hash is an acceptance proof and not a tautology
//!
//! Every layer the hash digests is independently pinned, so the
//! composite constant is *derived*, not observed-and-blessed:
//!
//! 1. the synthetic pattern is fully specified in code (module docs +
//!    unit-pinned pixel probes — no time, entropy or environment);
//! 2. the renderer is value-transparent at 1:1 placement (asserted:
//!    readback == the pure generator's bytes);
//! 3. the PNG is byte-stable by construction (stored deflate, no
//!    compressor heuristics) and is **structurally decoded and
//!    verified in-test** ([`common::decode_png`]: chunk CRCs, zlib
//!    header, stored-block bookkeeping, Adler-32, filter bytes) —
//!    the hash pins a verified image, not "whatever the encoder
//!    emitted";
//! 4. the SHA-256 is vector-pinned (FIPS 180-4) and was additionally
//!    cross-checked against the system `sha256sum` when the constant
//!    was recorded (worklog 2.D1), plus an out-of-tree Python zlib
//!    decode as a one-time oracle.
//!
//! Provenance of [`EXPECTED_PNG_SHA256`]: computed once on this
//! branch by the exact path this suite runs, re-derived by hand from
//! the decoded-image equality assertions above; any drift in
//! generator, placement, swizzle, encoder or hasher fails this suite.

mod common;

use common::{
    assert_sha256_vectors, decode_png, expected_pattern, present, sha256_hex, synthetic_request,
};
use media_types::GenerationId;
use renderer_wgpu::Renderer;
use renderer_wgpu::color::SurfaceFormat;
use renderer_wgpu::params::Extent2D;
use renderer_wgpu::screenshot;
use renderer_wgpu::surface::{PresentMode, SurfaceConfig};
use testkit::Must;

/// The AC's canonical extent (the crate-docs example): 64×48.
const AC_EXTENT: Extent2D = Extent2D {
    width: 64,
    height: 48,
};

/// The committed expected SHA-256 (hex) of the AC PNG byte stream.
///
/// Provenance (recorded on branch `wip/2d1-renderer-wgpu`, worklog
/// 2.D1): computed once by the exact path this suite runs —
/// `synthetic_frame(64×48)` → present → readback →
/// [`screenshot::encode_rgba`] — then **independently verified before
/// committing**: the system `sha256sum` agrees on the byte stream
/// (`1466e74a…eb30bf`), and an out-of-tree Python `zlib` oracle
/// decoded the same stream (chunk CRCs, IHDR 64×48 8-bit RGBA, sRGB
/// perceptual, IDAT inflation size, filter-0 defiltering) to pixels
/// byte-identical to the pure generator's output, with the pattern
/// spec's corner/grid/gradient probes recomputed independently.
/// Double-run byte-identity is proven by
/// [`double_run_renders_are_byte_identical`]. Changing the synthetic
/// pattern spec, the placement math, the swizzle, the stored-deflate
/// layout, the chunk order or the hasher breaks this constant by
/// design — that is the proof working.
const EXPECTED_PNG_SHA256: &str =
    "1466e74a10fefa702f517f2fe09db4da97a12a7c40f2334a95834ac992eb30bf";

/// The AC renderer: headless, BGRA8 sRGB surface, Fifo vsync (the
/// compositor-favorite configuration; doc 03 §4.11).
fn ac_renderer() -> Renderer {
    Renderer::new_headless(
        AC_EXTENT,
        SurfaceConfig {
            format: SurfaceFormat::Bgra8Srgb,
            present_mode: PresentMode::Fifo,
        },
    )
    .must()
}

/// Run the whole AC path once and return the PNG bytes.
fn render_png(renderer: &mut Renderer) -> Vec<u8> {
    present(
        renderer,
        &synthetic_request(AC_EXTENT, 0, GenerationId::FIRST),
    );
    let rgba = renderer.readback_rgba().must();
    screenshot::encode_rgba(AC_EXTENT, &rgba).must()
}

// ---------------------------------------------------------------------------
// THE AC
// ---------------------------------------------------------------------------

#[test]
fn ac_synthetic_frame_headless_render_produces_the_expected_png_hash() {
    assert_sha256_vectors("AC hash trust chain");

    let mut renderer = ac_renderer();
    assert_eq!(renderer.backend_name(), "software");
    let png = render_png(&mut renderer);

    // 1. Value truth: the readback is the pure generator's bytes (the
    //    renderer is transparent at 1:1 — the sRGB no-double-conversion
    //    invariant, now at the AC level).
    assert_eq!(renderer.readback_rgba().must(), expected_pattern(AC_EXTENT));

    // 2. Measured staging: the transfer-size instrumentation recorded
    //    exactly one copy of exactly the frame's bytes.
    let stats = renderer.staging_stats();
    assert_eq!(stats.frames_staged, 1);
    assert_eq!(stats.bytes_staged, 64 * 48 * 4);
    assert_eq!(stats.staging_copies, 1);

    // 3. The committed hash.
    let actual = sha256_hex(&png);
    assert_eq!(
        actual, EXPECTED_PNG_SHA256,
        "the AC PNG hash drifted (actual {actual}); see EXPECTED_PNG_SHA256's provenance note"
    );

    // 4. The PNG declares itself sRGB (the surface semantics).
    let decoded = decode_png(&png);
    assert_eq!(decoded.srgb_intent, Some(0), "sRGB chunk, perceptual");
}

// ---------------------------------------------------------------------------
// Double-run determinism (the card's second proof obligation)
// ---------------------------------------------------------------------------

#[test]
fn double_run_renders_are_byte_identical() {
    assert_sha256_vectors("double-run trust chain");

    // Two independently constructed renderers (fresh surfaces, fresh
    // staging buffers) produce byte-identical PNGs.
    let mut first = ac_renderer();
    let mut second = ac_renderer();
    let png_a = render_png(&mut first);
    let png_b = render_png(&mut second);
    assert_eq!(png_a, png_b, "two fresh renderers, byte-identical PNGs");
    assert_eq!(sha256_hex(&png_a), sha256_hex(&png_b));

    // And a re-present cycle on the same renderer is stable too (the
    // staging buffer reuse does not smear content).
    present(
        &mut first,
        &synthetic_request(AC_EXTENT, 33_333_334, GenerationId::FIRST),
    );
    let rgba = first.readback_rgba().must();
    let png_c = screenshot::encode_rgba(AC_EXTENT, &rgba).must();
    assert_eq!(png_c, png_a, "re-present cycle, byte-identical PNG");
    assert_eq!(sha256_hex(&png_c), EXPECTED_PNG_SHA256);
}

// ---------------------------------------------------------------------------
// The hash pins a verified image
// ---------------------------------------------------------------------------

#[test]
fn the_committed_hash_pins_a_verified_image() {
    let mut renderer = ac_renderer();
    let png = render_png(&mut renderer);
    let decoded = decode_png(&png);

    // Dimensions and pixel format.
    assert_eq!(decoded.width, 64);
    assert_eq!(decoded.height, 48);

    // Chunk census: IHDR, sRGB, IDAT, IEND — exactly, in order.
    let kinds: Vec<String> = decoded
        .chunks
        .iter()
        .map(|c| String::from_utf8_lossy(&c.kind).into_owned())
        .collect();
    assert_eq!(kinds, vec!["IHDR", "sRGB", "IDAT", "IEND"]);

    // The defiltered image is the synthetic pattern itself — the hash
    // digests a known-good image (every structural layer verified on
    // the way in: CRCs, zlib header, stored blocks, Adler-32).
    assert_eq!(decoded.rgba, expected_pattern(AC_EXTENT));
    assert_eq!(decoded.rgba.len(), 64 * 48 * 4);
}

// ---------------------------------------------------------------------------
// Sensitivity (non-vacuity)
// ---------------------------------------------------------------------------

#[test]
fn the_hash_is_sensitive_to_content_and_geometry() {
    let mut renderer = ac_renderer();
    let png = render_png(&mut renderer);
    let baseline = sha256_hex(&png);
    assert_eq!(baseline, EXPECTED_PNG_SHA256);

    // One changed pixel ⇒ a different PNG stream ⇒ a different hash.
    let mut rgba = renderer.readback_rgba().must();
    rgba[0] ^= 0x01;
    let mutated = screenshot::encode_rgba(AC_EXTENT, &rgba).must();
    assert_ne!(sha256_hex(&mutated), baseline, "content sensitivity");

    // A different extent ⇒ a different image ⇒ a different hash (the
    // letterboxed AC extent is not accidentally hashing as another).
    let other = Extent2D {
        width: 64,
        height: 47,
    };
    let mut renderer_b = Renderer::new_headless(
        other,
        SurfaceConfig {
            format: SurfaceFormat::Bgra8Srgb,
            present_mode: PresentMode::Fifo,
        },
    )
    .must();
    present(
        &mut renderer_b,
        &synthetic_request(other, 0, GenerationId::FIRST),
    );
    let png_b = screenshot::encode_rgba(other, &renderer_b.readback_rgba().must()).must();
    assert_ne!(sha256_hex(&png_b), baseline, "geometry sensitivity");
}

// ---------------------------------------------------------------------------
// sRGB present: both surface byte orders read back identically
// ---------------------------------------------------------------------------

#[test]
fn both_srgb_surface_orders_read_back_the_same_image() {
    // The BGRA surface swizzles on write and unswizzles on read; the
    // RGBA surface is a passthrough. Both must produce the same
    // readback (and therefore the same PNG hash) — the swizzle is an
    // involution, not a filter.
    let mut bgra = ac_renderer();
    let png_bgra = render_png(&mut bgra);

    let mut rgba_surface = Renderer::new_headless(
        AC_EXTENT,
        SurfaceConfig {
            format: SurfaceFormat::Rgba8Srgb,
            present_mode: PresentMode::Fifo,
        },
    )
    .must();
    let png_rgba = render_png(&mut rgba_surface);

    assert_eq!(png_bgra, png_rgba, "byte order is transport, not content");
    assert_eq!(sha256_hex(&png_rgba), EXPECTED_PNG_SHA256);
    assert_eq!(
        rgba_surface.staging_stats().bytes_staged,
        64 * 48 * 4,
        "same measured transfer through the other format"
    );
}
