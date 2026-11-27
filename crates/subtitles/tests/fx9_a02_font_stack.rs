//! F3.1, FX9-A02 — the admitted real font stack (swash 0.2.10 over the
//! bundled DejaVu Sans face) behind the `font::GlyphPipeline` seam. The
//! audit oracle is `audit/files/crates/subtitles/src/font/pipeline.rs.audit.md`:
//! F-001 ("the entire 'font pipeline' loads no fonts ... 'A', 'é' and
//! '漢' rasterize to the same block"), F-002 (the seam had zero
//! consumers) and F-003 (degenerate caller metrics + decorations could
//! panic) — plus REALITY-SCORE row 074 (textrender-subtitle-rasterizer
//! UNREACHABLE: "placeholder block glyphs only").
//!
//! Every assertion runs the REAL pipeline over the REAL committed face
//! (deterministic, no network, no display):
//!
//! * the committed asset loads and parses under swash (byte size pinned
//!   to the admission amendment's provenance record);
//! * the session seams (`session_stack`/`session_pipeline`) hand out
//!   the real stack — name, resolution trail, synthesis ledger;
//! * metrics are FACE-derived (they disagree with the placeholder's
//!   pinned 3/5-6/5-em ratios at two probe sizes, and scale coherently);
//! * `advance` is real per-character shaping ('i' and a space are
//!   narrower than the nominal digit-zero cell, 'W' wider than 'i') —
//!   the monospace default's one-cell answer is the contrast;
//! * masks differ per glyph (audit F-001), carry anti-aliased
//!   coverage (255 cores, 1..=254 edges, 0 counters — a solid block is
//!   0-or-255 only), and stay baseline-aligned ('g' descends below the
//!   baseline row, 'n' sits on it);
//! * decorations KEEP the real geometry (underline/strikeout strips at
//!   the pinned ratio, real ink untouched above the baseline, faux
//!   bold/italic only widening, the overhang ledger's exact ratios);
//! * rasterize stays total over junk chars AND hand-built degenerate
//!   metrics with every decoration (the F-003 hole, closed by
//!   clamping — the placeholder panicked here).

use subtitles::font::{
    bundled_face, session_pipeline, session_stack, FontFormat, FontRequest, GlyphDecorations,
    GlyphMask, GlyphMetrics, GlyphPipeline, GlyphSynthesis, MAX_FONT_SIZE_PX, MetricsError,
    MonospaceCellPipeline, ProviderStep, SwashPipeline,
};
use testkit::Must;

/// The session's real pipeline (the committed face through the FX9-A03
/// seam — one construction per test; parsing 759,720 bytes is cheap
/// next to what these assertions prove).
fn real_pipeline() -> Box<dyn GlyphPipeline> {
    session_pipeline()
}

/// Total coverage of a mask (the ink mass — dilation comparisons).
fn coverage_sum(mask: &GlyphMask) -> u64 {
    mask.alpha.iter().map(|&a| u64::from(a)).sum()
}

/// The horizontal ink extent (first/last nonzero-coverage column) of a
/// mask — the anti-solid-block measurement.
fn ink_span(mask: &GlyphMask) -> Option<(u32, u32)> {
    let mut span: Option<(u32, u32)> = None;
    for y in 0..mask.height {
        for x in 0..mask.width {
            if mask.alpha_at(x, y) > 0 {
                span = Some(match span {
                    Some((min, max)) => (min.min(x), max.max(x)),
                    None => (x, x),
                });
            }
        }
    }
    span
}

#[test]
fn fx9_a02_bundled_face_loads() {
    let face = bundled_face().must();
    // lumen-allow-literal: the face's own family name — round-trip data, not display identity
    assert_eq!(face.family(), "DejaVu Sans");
    assert_eq!(face.format(), FontFormat::TrueType);
    assert!(!face.bold());
    assert!(!face.italic());
    // The committed asset, pinned to the admission amendment's
    // provenance (docs/dependencies/records/font-stack-0.2.10.md R11):
    // a changed asset re-triggers the record (doc 11 §3.7).
    assert_eq!(face.data().len(), 759_720);
    // The committed bytes parse under the real stack (construction is
    // the only fallible step — the pipeline is then total).
    let bytes: &'static [u8] = Box::leak(face.data().to_vec().into_boxed_slice());
    assert!(
        SwashPipeline::new(bytes, 0).is_some(),
        "the committed DejaVu Sans must parse under swash 0.2.10"
    );
    // Garbage bytes are the typed refusal, not a panic.
    assert!(SwashPipeline::new(b"definitely not a font", 0).is_none());
}

#[test]
fn fx9_a02_session_pipeline_is_swash() {
    let pipeline = real_pipeline();
    // The session seam hands out the REAL stack; the placeholder's
    // ledger name is the negative control.
    // lumen-allow-literal: the pipelines' ledger names — round-trip data, not display identity
    assert_eq!(pipeline.name(), "swash-dejavu-sans");
    assert_eq!(MonospaceCellPipeline.name(), "monospace-cell-placeholder");
    // And it is live: real metrics at a real size.
    let m = pipeline.metrics(64).must();
    assert!(m.ascent_px >= 1);
    assert!(m.line_height_px >= m.ascent_px);
    assert!(m.advance_px >= 1);
}

#[test]
fn fx9_a02_session_stack_serves_bundled_default() {
    let stack = session_stack();
    // The bundled face answers its own family name, with the honest
    // trail and no synthesis.
    let hit = stack.resolve(&FontRequest::new("DejaVu Sans", false, false));
    assert!(hit.family_matched());
    assert!(hit.synthesis.is_empty());
    // lumen-allow-literal: the provider's name — round-trip data, not display identity
    assert_eq!(
        hit.chain,
        vec![ProviderStep::Supplied {
            provider: "bundled",
            family_matched: true
        }]
    );
    let face = hit.face.must();
    // lumen-allow-literal: the face's own family name — round-trip data, not display identity
    assert_eq!(face.family(), "DejaVu Sans");
    // Any other family falls back to the bundled default — face
    // supplied, family NOT matched, style gaps reported as synthesis.
    let fallback = stack.resolve(&FontRequest::new("NoSuchFamily", true, true));
    assert!(!fallback.family_matched());
    assert_eq!(
        fallback.synthesis,
        vec![GlyphSynthesis::FauxBold, GlyphSynthesis::FauxItalic]
    );
    // lumen-allow-literal: the provider's name — round-trip data, not display identity
    assert_eq!(
        fallback.chain,
        vec![ProviderStep::Supplied {
            provider: "bundled",
            family_matched: false
        }]
    );
    let face = fallback.face.must();
    // lumen-allow-literal: the face's own family name — round-trip data, not display identity
    assert_eq!(face.family(), "DejaVu Sans");
}

#[test]
fn fx9_a02_metrics_are_face_derived() {
    let pipeline = real_pipeline();
    let mono = MonospaceCellPipeline;
    // Every size in the seam's range answers with well-formed metrics.
    for size in [8u32, 16, 24, 32, 48, 64, 96, 128, 256] {
        let m = pipeline.metrics(size).must();
        assert!(m.ascent_px >= 1, "ascent at size {size}");
        assert!(m.line_height_px >= m.ascent_px, "line box at size {size}");
        assert!(m.advance_px >= 1, "advance at size {size}");
    }
    // FACE-derived, not the placeholder's pinned 3/5-6/5-em ratios: the
    // DejaVu metrics disagree with the monospace-widest-char policy at
    // both probe sizes (the digit-zero cell is ~0.64 em vs 0.6 em, the
    // ascent ~0.93 em vs the 0.96-em box).
    for size in [48u32, 64] {
        let real = pipeline.metrics(size).must();
        let placeholder = mono.metrics(size).must();
        assert_ne!(
            real, placeholder,
            "swash metrics at size {size} must be face-derived, not the pinned ratios"
        );
        assert_ne!(
            real.advance_px, placeholder.advance_px,
            "the face's digit-zero advance is not the 3/5-em placeholder cell"
        );
    }
    // Face metrics scale coherently: double the size, double the box
    // (each size rounds independently; tolerance 2 px).
    let m64 = pipeline.metrics(64).must();
    let m128 = pipeline.metrics(128).must();
    assert!(
        (i64::from(m128.ascent_px) - 2 * i64::from(m64.ascent_px)).abs() <= 2,
        "ascent must scale with the size: {m64:?} vs {m128:?}"
    );
    assert!(
        (i64::from(m128.line_height_px) - 2 * i64::from(m64.line_height_px)).abs() <= 2,
        "line box must scale with the size: {m64:?} vs {m128:?}"
    );
    assert!(
        (i64::from(m128.advance_px) - 2 * i64::from(m64.advance_px)).abs() <= 2,
        "cell advance must scale with the size: {m64:?} vs {m128:?}"
    );
    // The seam's typed guards are shared with the placeholder (the
    // contract both implementations must keep): zero and beyond-MAX
    // are refused, MAX itself answers.
    assert_eq!(pipeline.metrics(0), Err(MetricsError::ZeroFontSize));
    assert_eq!(
        pipeline.metrics(MAX_FONT_SIZE_PX + 1),
        Err(MetricsError::FontSizeTooLarge {
            size_px: MAX_FONT_SIZE_PX + 1,
            max: MAX_FONT_SIZE_PX
        })
    );
    let at_max = pipeline.metrics(MAX_FONT_SIZE_PX).must();
    assert!(at_max.ascent_px >= 1);
}

#[test]
fn fx9_a02_advance_is_real_per_char() {
    let pipeline = real_pipeline();
    let m = pipeline.metrics(64).must();
    // Real shaped advances: 'i' is narrower than its own nominal cell
    // (the digit-zero advance), narrower than 'W', and a space is
    // narrower than the cell — none of which a monospace cell policy
    // can express.
    let adv_i = pipeline.advance('i', &m);
    let adv_w = pipeline.advance('W', &m);
    let adv_space = pipeline.advance(' ', &m);
    assert!(
        adv_i < m.advance_px,
        "an 'i' ({adv_i} px) must advance less than the nominal cell ({} px)",
        m.advance_px
    );
    assert!(
        adv_i < adv_w,
        "an 'i' ({adv_i} px) must be narrower than a 'W' ({adv_w} px)"
    );
    assert!(
        adv_space < m.advance_px,
        "a space ({adv_space} px) is narrower than the nominal cell ({} px)",
        m.advance_px
    );
    // The default trait method (the placeholder's policy — what A05
    // sums when only the nominal cell is known) answers the nominal
    // cell for EVERY character: the contrast that proves the override.
    let mono = MonospaceCellPipeline;
    let mono_m = mono.metrics(64).must();
    assert_eq!(mono.advance('i', &mono_m), mono_m.advance_px);
    assert_eq!(mono.advance('W', &mono_m), mono_m.advance_px);
}

#[test]
fn fx9_a02_masks_differ_per_char() {
    let pipeline = real_pipeline();
    let m = pipeline.metrics(64).must();
    let none = GlyphDecorations::NONE;
    let mask_i = pipeline.rasterize('i', &m, &none);
    let mask_o = pipeline.rasterize('o', &m, &none);
    let mask_w = pipeline.rasterize('W', &m, &none);
    // Shape invariants hold on every mask.
    for mask in [&mask_i, &mask_o, &mask_w] {
        assert_eq!(mask.alpha.len(), (mask.width * mask.height) as usize);
        assert!(!mask.is_blank());
    }
    // Per-glyph masks DIFFER — the audit's F-001 oracle (pre-fix every
    // non-space codepoint rasterized to one identical solid block).
    assert_ne!(mask_i.alpha, mask_w.alpha);
    assert_ne!(mask_i.alpha, mask_o.alpha);
    assert_ne!(mask_o.alpha, mask_w.alpha);
    assert!(
        mask_i.width < mask_w.width,
        "the 'i' mask ({} px) is narrower than the 'W' mask ({} px)",
        mask_i.width,
        mask_w.width
    );
    // The 'i' stem occupies a much narrower column band than the 'W'
    // body — real ink extents, not one block width.
    let (min_i, max_i) = ink_span(&mask_i).must();
    let (min_w, max_w) = ink_span(&mask_w).must();
    assert!(
        max_i - min_i < max_w - min_w,
        "the 'i' ink band ({min_i}..={max_i}) is narrower than the 'W' band ({min_w}..={max_w})"
    );
    // Anti-aliased coverage is REAL: fully-covered cores (255),
    // fractional edges (1..=254), and genuinely empty samples (0) —
    // a solid block is 0-or-255 only, and nothing inks the very top
    // of the ascent box (cap height < ascent).
    assert!(
        mask_w.alpha.iter().any(|&a| a == 255),
        "the 'W' core must be fully covered"
    );
    assert!(
        mask_w.alpha.iter().any(|&a| a > 0 && a < 255),
        "the 'W' edges must carry fractional (anti-aliased) coverage"
    );
    assert_eq!(
        mask_w.alpha_at(0, 0),
        0,
        "cap height never reaches the top of the ascent box"
    );
    // The placeholder's negative control: its 'i' and 'W' masks are
    // byte-identical 0-or-255 solid blocks (the audit's exact finding).
    let mono = MonospaceCellPipeline;
    let mono_m = mono.metrics(64).must();
    let mono_i = mono.rasterize('i', &mono_m, &none);
    let mono_w = mono.rasterize('W', &mono_m, &none);
    assert_eq!(mono_i.alpha, mono_w.alpha);
    assert!(
        mono_i.alpha.iter().all(|&a| a == 0 || a == 255),
        "placeholder coverage is 0-or-255 only"
    );
}

#[test]
fn fx9_a02_descender_reaches_below_baseline() {
    let pipeline = real_pipeline();
    let m = pipeline.metrics(64).must();
    let none = GlyphDecorations::NONE;
    let mask_g = pipeline.rasterize('g', &m, &none);
    let mask_n = pipeline.rasterize('n', &m, &none);
    // A baseline-aligned real face: 'n' sits ON the baseline (the mask
    // ends within rounding of the ascent row), 'g' descends BELOW it —
    // the mask grows past the ascent (DejaVu's descender is ~0.23 em)
    // with real ink under the baseline row.
    assert!(
        mask_g.height >= m.ascent_px + 6,
        "the 'g' mask ({} px) must reach below the {}-px ascent",
        mask_g.height,
        m.ascent_px
    );
    assert!(
        mask_n.height <= m.ascent_px + 2,
        "the 'n' mask ({} px) must sit on the {}-px baseline",
        mask_n.height,
        m.ascent_px
    );
    assert!(mask_g.height > mask_n.height);
    let below = &mask_g.alpha[m.ascent_px as usize * mask_g.width as usize..];
    assert!(
        below.iter().any(|&a| a > 0),
        "the 'g' descender must put real ink below the baseline"
    );
    // And the ascent area itself is real ink, not a block: the 'n'
    // body carries anti-aliased coverage.
    assert!(mask_n.alpha.iter().any(|&a| a > 0 && a < 255));
}

#[test]
fn fx9_a02_decorations_keep_real_geometry() {
    let pipeline = real_pipeline();
    let m = pipeline.metrics(64).must();
    let none = GlyphDecorations::NONE;
    let plain = pipeline.rasterize('o', &m, &none);
    // The seam's pinned strip ratio, duplicated from the placeholder's
    // documented geometry: max(1, (ascent+3)/8).
    let strip = ((m.ascent_px + 3) / 8).max(1);
    // Underline: the strip lives BELOW the baseline — solid across the
    // full mask width, and the real glyph ink above the baseline is
    // UNTOUCHED (decorations add, they never replace).
    let underline = GlyphDecorations {
        underline: true,
        ..GlyphDecorations::NONE
    };
    let u = pipeline.rasterize('o', &m, &underline);
    assert!(
        u.height >= m.ascent_px + strip,
        "the underlined mask ({} px) must grow past the {}-px ascent by the {}-px strip",
        u.height,
        m.ascent_px,
        strip
    );
    for y in m.ascent_px..m.ascent_px + strip {
        for x in 0..u.width {
            assert_eq!(
                u.alpha_at(x, y),
                255,
                "underline row {y} must be solid across the mask"
            );
        }
    }
    assert_eq!(u.width, plain.width, "the underline grows height only");
    for y in 0..m.ascent_px {
        for x in 0..plain.width {
            assert_eq!(
                u.alpha_at(x, y),
                plain.alpha_at(x, y),
                "row {y} above the baseline must keep the real 'o' ink"
            );
        }
    }
    // Strikeout: the strip is centred on ascent/2, INSIDE the ascent —
    // the mask box does not grow.
    let strikeout = GlyphDecorations {
        strikeout: true,
        ..GlyphDecorations::NONE
    };
    let s = pipeline.rasterize('o', &m, &strikeout);
    let top = (m.ascent_px / 2).saturating_sub(strip / 2);
    for y in top..top + strip {
        for x in 0..s.width {
            assert_eq!(s.alpha_at(x, y), 255, "strikeout row {y} must be solid");
        }
    }
    assert_eq!(s.height, plain.height, "the strike lives inside the ascent");
    // Faux bold: widens only, and only ADDS coverage.
    let bold = GlyphDecorations {
        bold: true,
        ..GlyphDecorations::NONE
    };
    let b = pipeline.rasterize('o', &m, &bold);
    assert!(b.width > plain.width, "faux bold must widen the mask");
    assert_eq!(b.height, plain.height, "faux bold widens only");
    assert!(coverage_sum(&b) >= coverage_sum(&plain));
    // Faux italic: shears wider, horizontal only.
    let italic = GlyphDecorations {
        italic: true,
        ..GlyphDecorations::NONE
    };
    let it = pipeline.rasterize('o', &m, &italic);
    assert!(it.width > plain.width, "faux italic must shear the mask wider");
    assert_eq!(it.height, plain.height, "the shear is horizontal only");
    // The overhang ledger: the placeholder's pinned ratios over the
    // nominal cell (decoration extras only — real per-glyph overhang
    // is not reportable through that signature, module docs).
    assert_eq!(pipeline.overhang(&m, &none), 0);
    assert_eq!(pipeline.overhang(&m, &bold), (m.advance_px / 6).max(1));
    assert_eq!(pipeline.overhang(&m, &italic), m.advance_px / 4);
    let both = GlyphDecorations {
        bold: true,
        italic: true,
        ..GlyphDecorations::NONE
    };
    assert_eq!(
        pipeline.overhang(&m, &both),
        (m.advance_px / 6).max(1) + m.advance_px / 4
    );
}

#[test]
fn fx9_a02_rasterize_is_total_over_degenerate_inputs() {
    let pipeline = real_pipeline();
    let m = pipeline.metrics(32).must();
    // The audit's F-003 hole is CLOSED under the real stack: hand-built
    // degenerate metrics (the pub-field contract) with every decoration
    // clamp instead of panicking — the placeholder panicked here.
    let degenerate = [
        GlyphMetrics {
            advance_px: 0,
            line_height_px: 0,
            ascent_px: 0,
        },
        GlyphMetrics {
            advance_px: 0,
            line_height_px: 0,
            ascent_px: 1,
        },
        GlyphMetrics {
            advance_px: 7,
            line_height_px: 1,
            ascent_px: 3,
        },
        GlyphMetrics {
            advance_px: 1024,
            line_height_px: 4096,
            ascent_px: 2048,
        },
    ];
    let all_deco = GlyphDecorations {
        bold: true,
        italic: true,
        underline: true,
        strikeout: true,
    };
    for metrics in degenerate {
        for ch in ['A', ' ', '\u{0}', '漢', '\u{10FFFF}'] {
            for deco in [all_deco, GlyphDecorations::NONE] {
                let mask = pipeline.rasterize(ch, &metrics, &deco);
                assert!(mask.width >= 1 && mask.height >= 1, "nonempty box for {ch:?}");
                assert!(mask.width <= 2 * MAX_FONT_SIZE_PX);
                assert!(mask.height <= 2 * MAX_FONT_SIZE_PX);
                assert_eq!(
                    mask.alpha.len(),
                    (mask.width * mask.height) as usize,
                    "shape invariant for {ch:?}"
                );
            }
        }
    }
    // Junk codepoints under sane metrics: total and well-formed (the
    // placeholder suite's junk list, mirrored), and the space carries
    // no ink of its own.
    for ch in ['\u{0}', '\u{7f}', 'é', '漢', '\u{10FFFF}', '\t', '\u{a0}', ' '] {
        let mask = pipeline.rasterize(ch, &m, &GlyphDecorations::NONE);
        assert!(mask.width >= 1 && mask.height >= 1, "nonempty box for {ch:?}");
        assert_eq!(
            mask.alpha.len(),
            (mask.width * mask.height) as usize,
            "shape invariant for {ch:?}"
        );
    }
    let space = pipeline.rasterize(' ', &m, &GlyphDecorations::NONE);
    assert!(space.is_blank(), "a space carries no glyph ink");
}
