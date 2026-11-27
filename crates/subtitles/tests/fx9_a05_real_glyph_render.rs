//! F3.1, FX9-A05 — the real-glyph render threading: the
//! `*_with_pipeline` twins (`srt::render::{place_plain_text, text_box,
//! cue_layout}_with_pipeline` and `ass::render::{render_plain_text_cue,
//! render_styled_cue}_with_pipeline`) take a `&dyn font::GlyphPipeline`
//! — per-character advances (`GlyphPipeline::advance`, A02's default
//! method) for the line-width sums, and one pipeline mask per glyph
//! for the ink (straight-alpha coverage: the span colour's RGB with
//! the coverage-tinted alpha). The audit oracle is
//! `audit/files/crates/subtitles/src/{srt,ass}/render.rs.audit.md` +
//! REALITY-SCORE row 074 (textrender-subtitle-rasterizer UNREACHABLE:
//! "placeholder block glyphs only, and unwired").
//!
//! Test contracts (deliberate, per the assignment):
//!
//! * the real-metric claims run under a test-local variable-advance
//!   fake pipeline (so this file is compile-independent from A02's
//!   swash lane — it runs under ANY `GlyphPipeline`), the
//!   wrapper-equivalence claims under `MonospaceCellPipeline`;
//! * `CueInterval` imports from its REAL public path
//!   `subtitles::model::time` (it is private in `model::cue`);
//! * the plain twins render `cue.text` only, so every plain-path
//!   fixture carries genuinely renderable dialogue text (the crate's
//!   own `ac_basic.ass` payloads — "Hello Lumen" & co), never
//!   empty-text-with-runs (that is the typed `EmptyText` error);
//! * ASS colour literals are `&HAABBGGRR`: half-alpha WHITE is
//!   `0x80FF_FFFF` (`0x8000_FFFF` would be half-alpha YELLOW).
//!
//! Deterministic: no network, no display, no clock.

use subtitles::ass::layout::{AssAlignment, AssMargins, DEFAULT_PLAY_RES};
use subtitles::ass::render::{
    AssRenderCue, AssRenderError, AssRenderRun, AssRenderStyle, ass_colour_to_model,
    render_plain_text_cue, render_plain_text_cue_with_pipeline, render_styled_cue,
    render_styled_cue_with_pipeline,
};
use subtitles::compositing::target::Rgba;
use subtitles::font::{
    GlyphDecorations, GlyphMask, GlyphMetrics, GlyphPipeline, MetricsError, MonospaceCellPipeline,
};
use subtitles::model::cue::{Cue, CueId, CueText};
use subtitles::model::position::{
    Alignment, HorizontalAlign, LinePlacement, Percent, Positioning, VerticalAlign,
};
use subtitles::model::style::Color as ModelColor;
use subtitles::model::time::CueInterval;
use subtitles::srt::render::{
    SrtRenderError, TextMetrics, cue_layout, cue_layout_with_pipeline, nominal_glyph_metrics,
    place_plain_text, place_plain_text_with_pipeline, text_box, text_box_with_pipeline,
};
use testkit::Must;

const HD: (u32, u32) = (1920, 1080);

// `&HAABBGGRR` literals (ass::render::ass_colour_to_model's byte
// order): opaque white, opaque yellow, half-alpha WHITE — the last
// is 0x80FF_FFFF, NOT 0x8000_FFFF (that is half-alpha YELLOW).
const ASS_WHITE: u32 = 0x00FF_FFFF;
const ASS_YELLOW: u32 = 0x0000_FFFF;
const ASS_HALF_ALPHA_WHITE: u32 = 0x80FF_FFFF;

/// The test-local variable-advance fake lane: 'i' advances a quarter
/// cell, 'W' five thirds of one, a space just under half; everything
/// else the nominal cell. Masks are the glyph's own advance wide with
/// fractional (anti-aliased) coverage on the border samples — and in
/// `underlined` mode the PLAIN masks carry an underline strip below
/// the ascent, i.e. exactly what a decoration-aware pipeline renders
/// for underlined text. Decoration bits themselves are the in-tree
/// pipeline's contract (pinned by its own suite); this fake models
/// the lane-level shape so the seam tests stay compile-independent
/// from A02's swash.
struct FakeLane {
    underlined: bool,
}

impl GlyphPipeline for FakeLane {
    fn name(&self) -> &'static str {
        "fx9-a05-variable-advance-fake"
    }

    fn metrics(&self, size_px: u32) -> Result<GlyphMetrics, MetricsError> {
        // The placeholder's ratios — the variance under test is the
        // advance/mask shaping, not the span metrics.
        GlyphMetrics::from_font_size(size_px)
    }

    fn advance(&self, ch: char, metrics: &GlyphMetrics) -> u32 {
        match ch {
            'i' => metrics.advance_px / 4,
            'W' => metrics.advance_px * 5 / 3,
            ' ' if metrics.advance_px > 1 => metrics.advance_px / 2 - 1,
            _ => metrics.advance_px,
        }
    }

    fn rasterize(&self, ch: char, metrics: &GlyphMetrics, _deco: &GlyphDecorations) -> GlyphMask {
        let w = self.advance(ch, metrics).max(1);
        let ascent = metrics.ascent_px;
        let strip = ((ascent + 3) / 8).max(1);
        let extra = if self.underlined { strip } else { 0 };
        let height = ascent + extra;
        let mut alpha = vec![0u8; w as usize * height as usize];
        // A space carries no glyph ink (its underline strip below
        // still does — an underlined space is visible in real
        // typography too).
        if ch != ' ' {
            for y in 0..ascent {
                for x in 0..w {
                    let coverage = if x == 0 || y == 0 { 128 } else { 255 };
                    alpha[y as usize * w as usize + x as usize] = coverage;
                }
            }
        }
        if self.underlined {
            for y in ascent..height {
                for x in 0..w {
                    alpha[y as usize * w as usize + x as usize] = 255;
                }
            }
        }
        GlyphMask { width: w, height, alpha }
    }

    fn overhang(&self, _metrics: &GlyphMetrics, _deco: &GlyphDecorations) -> u32 {
        0
    }
}

/// An SRT cue with plain dialogue text (the plain twins render
/// `cue.text` — genuinely renderable, never empty).
fn srt_cue(text: &str, positioning: Positioning) -> Cue {
    Cue::new(
        CueId::new(1),
        CueInterval::new(0, 1_000_000).must(),
        CueText::Plain(text.into()),
        positioning,
    )
}

/// An ASS cue in the `ac_basic.ass` shape: bottom-center, 10/10/10
/// margins, opaque white, 20 script px.
fn ass_cue(text: &str) -> AssRenderCue {
    AssRenderCue {
        id: CueId::new(1),
        start_us: 1_000_000,
        end_us: 3_500_000,
        text: text.into(),
        runs: Vec::new(),
        pos: None,
        style: AssRenderStyle {
            alignment: AssAlignment::BOTTOM_CENTER,
            margins: AssMargins::ASS_DEFAULT,
            primary_colour: ASS_WHITE,
            font_size_px: 20,
        },
        z: 0,
    }
}

/// The alpha channel of an RGBA buffer.
fn alpha_values(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks(4).map(|px| px[3]).collect()
}

#[test]
fn fx9_a05_per_char_advance_variance() {
    let fake = FakeLane { underlined: false };
    let mono = MonospaceCellPipeline;

    // The nominal mapping the free functions thread: cell → advance,
    // line box → line height, the seam's pinned 80 % ascent.
    let tm = TextMetrics::new(40, 20);
    let gm = nominal_glyph_metrics(&tm);
    assert_eq!(
        gm,
        GlyphMetrics {
            advance_px: 20,
            line_height_px: 40,
            ascent_px: 32,
        }
    );

    // SRT: the box width is the SUM of per-char advances, not
    // char-count × cell. At the nominal 20-px cell the fake measures
    // W = 33, i = 5, space = 9, everything else 20: "Where is it?"
    // → 33+4×20+9+5+20+9+5+20+20 = 201 (the placeholder: 12 × 20 = 240).
    let cue = srt_cue("Where is it?", Positioning::default());
    let fake_box = text_box_with_pipeline(&cue, &gm, HD, &fake).must();
    let mono_box = text_box_with_pipeline(&cue, &gm, HD, &mono).must();
    assert_eq!(fake_box.width_px, 201, "the advance sum, not the cell count");
    assert_eq!(mono_box.width_px, 240, "one nominal cell per character");
    // The free function IS the placeholder lane.
    assert_eq!(text_box(&cue, &tm, HD).must(), mono_box);
    // Line height is the line box, unaffected by advance shaping.
    assert_eq!(fake_box.height_px, mono_box.height_px);
    // Per-line sums: "iW\ni" → line 0 = 5+33 = 38, line 1 = 5 → 38
    // (the placeholder: max(2, 1) × 20 = 40).
    let cue = srt_cue("iW\ni", Positioning::default());
    assert_eq!(
        text_box_with_pipeline(&cue, &gm, HD, &fake)
            .must()
            .width_px,
        38
    );
    assert_eq!(
        text_box_with_pipeline(&cue, &gm, HD, &mono)
            .must()
            .width_px,
        40
    );

    // ASS: the cue BITMAP width is the advance sum. At font 20
    // (cell 12) the fake measures W = 20, i = 3, space = 5:
    // "Where is it?" → 20+4×12+5+3+12+5+3+12+12 = 120 (placeholder:
    // 12 × 12 = 144).
    let m20 = GlyphMetrics::from_font_size(20).must();
    assert!(fake.advance('i', &m20) < fake.advance('W', &m20));
    let c = ass_cue("Where is it?");
    let via_fake = render_plain_text_cue_with_pipeline(&c, DEFAULT_PLAY_RES, &fake).must();
    let via_mono = render_plain_text_cue_with_pipeline(&c, DEFAULT_PLAY_RES, &mono).must();
    assert_eq!(via_fake.bitmap.width, 120);
    assert_eq!(via_mono.bitmap.width, 144);
    assert_eq!(render_plain_text_cue(&c, DEFAULT_PLAY_RES).must(), via_mono);

    // Styled runs: each run's advances come from its OWN metrics —
    // "Wi" at 20 px → 20+3 = 23, "W" at 10 px (cell 6) → 10; the
    // line is 33 px (the placeholder: 2×12 + 6 = 30).
    let mut c = ass_cue("Where is it?");
    c.runs = vec![
        AssRenderRun {
            text: "Wi".into(),
            color: ModelColor::WHITE,
            font_size_px: 20,
        },
        AssRenderRun {
            text: "W".into(),
            color: ModelColor::WHITE,
            font_size_px: 10,
        },
    ];
    let via_fake = render_styled_cue_with_pipeline(&c, DEFAULT_PLAY_RES, &fake).must();
    let via_mono = render_styled_cue_with_pipeline(&c, DEFAULT_PLAY_RES, &mono).must();
    assert_eq!(via_fake.bitmap.width, 33);
    assert_eq!(via_mono.bitmap.width, 30);
    assert_eq!(render_styled_cue(&c, DEFAULT_PLAY_RES).must(), via_mono);
}

#[test]
fn fx9_a05_non_solid_block_coverage() {
    let fake = FakeLane { underlined: false };
    let mono = MonospaceCellPipeline;

    // Real masks flow to the ink: the fake's fractional border
    // coverage (128) tints the ALPHA channel only (the mask-seam
    // contract — the RGB stays the span colour), so the rendered
    // bitmap carries 0, 128 AND 255 alphas.
    let c = ass_cue("Where is it?");
    let via_fake = render_plain_text_cue_with_pipeline(&c, DEFAULT_PLAY_RES, &fake).must();
    let bmp = &via_fake.bitmap;
    let alphas = alpha_values(&bmp.rgba);
    assert!(alphas.iter().any(|&a| a == 0), "blank space cells");
    assert!(
        alphas.iter().any(|&a| a == 128),
        "fractional (anti-aliased) edge coverage"
    );
    assert!(alphas.iter().any(|&a| a == 255), "solid core coverage");
    // The RGB never carries the coverage (straight alpha).
    assert_eq!(bmp.pixel(0, 0), Rgba::WHITE.with_alpha(128));
    assert_eq!(bmp.pixel(5, 5), Rgba::WHITE);

    // The placeholder lane is 0-or-255 only — the audit's
    // "solid block glyphs" contrast.
    let via_mono = render_plain_text_cue_with_pipeline(&c, DEFAULT_PLAY_RES, &mono).must();
    assert!(
        alpha_values(&via_mono.bitmap.rgba)
            .iter()
            .all(|&a| a == 0 || a == 255),
        "solid block coverage is 0-or-255 only"
    );

    // Half-alpha WHITE (`&H80FFFFFF` — the &HAABBGGRR byte order):
    // full coverage keeps the 127 alpha EXACTLY, fractional coverage
    // tints it — tint(127, 128) = (127×128+127)/255 = 64.
    let mut c = ass_cue("Where is it?");
    c.style.primary_colour = ASS_HALF_ALPHA_WHITE;
    let rendered = render_plain_text_cue_with_pipeline(&c, DEFAULT_PLAY_RES, &fake).must();
    let bmp = &rendered.bitmap;
    assert_eq!(bmp.pixel(5, 5), Rgba::WHITE.with_alpha(127));
    assert_eq!(bmp.pixel(0, 0), Rgba::WHITE.with_alpha(64));
    // The colour-literal sanity (a previous fixture bug had
    // half-alpha YELLOW here): 0x80FF_FFFF is half-alpha WHITE,
    // 0x8000_FFFF is half-alpha YELLOW.
    assert_eq!(
        ass_colour_to_model(ASS_HALF_ALPHA_WHITE),
        ModelColor::new(255, 255, 255, 127)
    );
    assert_eq!(
        ass_colour_to_model(0x8000_FFFF),
        ModelColor::new(255, 255, 0, 127)
    );
    assert_eq!(ass_colour_to_model(ASS_WHITE), ModelColor::WHITE);
    assert_eq!(
        ass_colour_to_model(ASS_YELLOW),
        ModelColor::new(255, 255, 0, 255)
    );
}

#[test]
fn fx9_a05_decoration_differences_preserved() {
    let plain = FakeLane { underlined: false };
    let underlined = FakeLane { underlined: true };

    // Pipeline level — the decoration-aware input exists (the
    // in-tree placeholder's pinned geometry): bold widens, underline
    // grows below the ascent, the overhang ledger reports both.
    let m = GlyphMetrics::from_font_size(20).must();
    let none = GlyphDecorations::NONE;
    let cell = MonospaceCellPipeline.rasterize('o', &m, &none);
    let bold = MonospaceCellPipeline.rasterize(
        'o',
        &m,
        &GlyphDecorations {
            bold: true,
            ..GlyphDecorations::NONE
        },
    );
    let underline = MonospaceCellPipeline.rasterize(
        'o',
        &m,
        &GlyphDecorations {
            underline: true,
            ..GlyphDecorations::NONE
        },
    );
    assert!(bold.width > cell.width, "faux bold widens the mask");
    assert!(
        underline.height > cell.height,
        "the underline strip grows below the ascent"
    );
    assert_eq!(MonospaceCellPipeline.overhang(&m, &none), 0);

    // Seam level — the render path does NOT flatten pipeline mask
    // geometry: a lane whose plain masks carry the underline strip
    // (what a decoration-aware pipeline produces for underlined text)
    // renders DIFFERENT ink through the same twin, while the base
    // glyph rows stay byte-identical (decorations add, never
    // replace).
    let c = ass_cue("Hello Lumen");
    let via_plain = render_plain_text_cue_with_pipeline(&c, DEFAULT_PLAY_RES, &plain).must();
    let via_underlined = render_plain_text_cue_with_pipeline(&c, DEFAULT_PLAY_RES, &underlined)
        .must();
    // Geometry unchanged (the strip is ink, not advance).
    assert_eq!(via_plain.bitmap.width, via_underlined.bitmap.width);
    assert_eq!(via_plain.bitmap.height, via_underlined.bitmap.height);
    assert_eq!(via_plain.bitmap.baseline, via_underlined.bitmap.baseline);
    // The full outputs differ.
    assert_ne!(via_plain, via_underlined);
    // The glyph rows (0..ascent) are byte-identical.
    let glyph_rows = m.ascent_px as usize * via_plain.bitmap.width as usize * 4;
    assert_eq!(
        via_plain.bitmap.rgba[..glyph_rows],
        via_underlined.bitmap.rgba[..glyph_rows]
    );
    // The strip rows (19..21 at the pinned max(1, (19+3)/8) = 2)
    // carry solid tinted ink in the underlined lane, blank in the
    // plain lane.
    assert_eq!(via_plain.bitmap.pixel(0, 19), Rgba::BLACK.with_alpha(0));
    assert_eq!(via_underlined.bitmap.pixel(0, 19), Rgba::WHITE);
    assert_eq!(via_underlined.bitmap.pixel(0, 20), Rgba::WHITE);
    // Below the strip: transparent in both.
    assert_eq!(
        via_underlined.bitmap.pixel(0, 21),
        Rgba::BLACK.with_alpha(0)
    );
}

#[test]
fn fx9_a05_placeholder_wrapper_contract_byte_identical() {
    let mono = MonospaceCellPipeline;
    let tm = TextMetrics::new(40, 20);
    let gm = nominal_glyph_metrics(&tm);

    // The ASS cue family: the ac_basic.ass payloads (real dialogue
    // text), colours incl. half-alpha WHITE (&HAABBGGRR), breaks,
    // spaces, unicode, pos override, non-default z and timing.
    let mut family: Vec<AssRenderCue> = vec![
        ass_cue("Hello Lumen"),
        ass_cue("Second line here\\Nwith two lines"),
        ass_cue("Late and long"),
        ass_cue("a b c"),
        ass_cue("é漢 W"),
        ass_cue("   "),
    ];
    let mut yellow = ass_cue("Second line here\\Nwith two lines");
    yellow.style.primary_colour = ASS_YELLOW;
    family.push(yellow);
    let mut half = ass_cue("Late and long");
    half.style.primary_colour = ASS_HALF_ALPHA_WHITE;
    half.style.alignment = AssAlignment::TOP_RIGHT;
    half.pos = Some((192, 144));
    half.z = 7;
    family.push(half);

    for c in &family {
        // BYTE-IDENTICAL: the free function == the twin driven with
        // the placeholder pipeline (the strict wrapper contract —
        // direct delegation, pinned so any future reimplementation
        // of the free path breaks loudly).
        let free = render_plain_text_cue(c, DEFAULT_PLAY_RES).must();
        let twin = render_plain_text_cue_with_pipeline(c, DEFAULT_PLAY_RES, &mono).must();
        assert_eq!(free.bitmap.rgba, twin.bitmap.rgba, "text {:?}", c.text);
        assert_eq!(free, twin, "text {:?}", c.text);
        // The styled lane with empty runs is the same placeholder
        // delegation.
        let s_free = render_styled_cue(c, DEFAULT_PLAY_RES).must();
        let s_twin = render_styled_cue_with_pipeline(c, DEFAULT_PLAY_RES, &mono).must();
        assert_eq!(s_free, s_twin, "text {:?}", c.text);
        assert_eq!(s_free, free, "empty-runs delegation");
    }

    // A known-good anchor through the twin: "Hello Lumen" at 20 px —
    // 11 cells × 12 = 132 wide, one 24-px line, baseline 19.
    let anchor =
        render_plain_text_cue_with_pipeline(&ass_cue("Hello Lumen"), DEFAULT_PLAY_RES, &mono)
            .must();
    assert_eq!(
        (
            anchor.bitmap.width,
            anchor.bitmap.height,
            anchor.bitmap.baseline
        ),
        (132, 24, 19)
    );

    // The styled family: mixed sizes/colours, breaks inside runs,
    // spaces-only runs (renders blank); cue.text stays genuinely
    // renderable (the plain fallback path renders it).
    let mut styled: Vec<AssRenderCue> = Vec::new();
    let mut c = ass_cue("Where is it?");
    c.runs = vec![
        AssRenderRun {
            text: "Where ".into(),
            color: ModelColor::new(255, 0, 0, 255),
            font_size_px: 20,
        },
        AssRenderRun {
            text: "is it?".into(),
            color: ModelColor::new(0, 0, 255, 255),
            font_size_px: 10,
        },
    ];
    styled.push(c);
    let mut c = ass_cue("Where is it?");
    c.runs = vec![AssRenderRun {
        text: "Where\\Nis it?".into(),
        color: ModelColor::WHITE,
        font_size_px: 20,
    }];
    styled.push(c);
    let mut c = ass_cue("Where is it?");
    c.runs = vec![AssRenderRun {
        text: "  ".into(),
        color: ModelColor::WHITE,
        font_size_px: 20,
    }];
    styled.push(c);
    for c in &styled {
        let free = render_styled_cue(c, DEFAULT_PLAY_RES).must();
        let twin = render_styled_cue_with_pipeline(c, DEFAULT_PLAY_RES, &mono).must();
        assert_eq!(free.bitmap.rgba, twin.bitmap.rgba, "runs of {:?}", c.text);
        assert_eq!(free, twin, "runs of {:?}", c.text);
    }

    // The SRT family: default, multi-line, clamped long line, percent
    // and line-number placements. Free == twin(nominal, placeholder)
    // for the box, the layout and the placed rect.
    let srt_family = vec![
        srt_cue("Hello Lumen", Positioning::default()),
        srt_cue("one\ntwo\nthree", Positioning::default()),
        srt_cue(&"x".repeat(500), Positioning::default()),
        srt_cue(
            "ab",
            Positioning::new(
                Alignment::new(VerticalAlign::Bottom, HorizontalAlign::Right),
                Some(LinePlacement::Percent(Percent::new(75).must())),
                Some(Percent::new(25).must()),
            ),
        ),
        srt_cue(
            "ab",
            Positioning::new(
                Alignment::new(VerticalAlign::Top, HorizontalAlign::Left),
                Some(LinePlacement::LineNumber(2)),
                None,
            ),
        ),
        srt_cue(
            "ab",
            Positioning::new(
                Alignment::new(VerticalAlign::Top, HorizontalAlign::Left),
                Some(LinePlacement::LineNumber(-1)),
                None,
            ),
        ),
    ];
    for cue in &srt_family {
        let free_box = text_box(cue, &tm, HD).must();
        let twin_box = text_box_with_pipeline(cue, &gm, HD, &mono).must();
        assert_eq!(free_box, twin_box);
        assert_eq!(
            cue_layout(cue, HD, &tm).must(),
            cue_layout_with_pipeline(cue, HD, &gm, &mono).must()
        );
        assert_eq!(
            place_plain_text(cue, &tm, HD).must(),
            place_plain_text_with_pipeline(cue, &gm, HD, &mono).must()
        );
    }
    // Known-good anchors through the twin (the pinned placeholder
    // math): "Hello Lumen" → 11 × 20 = 220 wide, 40 tall, bottom
    // center inside title-safe at y = 986; the 75 %/25 % cue →
    // (440, 770).
    let rect = place_plain_text_with_pipeline(&srt_family[0], &gm, HD, &mono).must();
    assert_eq!(
        (rect.x, rect.y, rect.width, rect.height),
        (850, 986, 220, 40)
    );
    let rect = place_plain_text_with_pipeline(&srt_family[3], &gm, HD, &mono).must();
    assert_eq!((rect.x, rect.y, rect.width, rect.height), (440, 770, 40, 40));
}

#[test]
fn fx9_a05_twin_error_paths_stay_typed() {
    let mono = MonospaceCellPipeline;
    let fake = FakeLane { underlined: false };
    let pr = DEFAULT_PLAY_RES;

    let reversed = {
        let mut c = ass_cue("Hello Lumen");
        c.start_us = 5_000_000;
        c.end_us = 4_999_999;
        c
    };
    let zero_res = ass_cue("Hello Lumen");
    let margin_over = {
        let mut c = ass_cue("Hello Lumen");
        c.style.alignment = AssAlignment::BOTTOM_LEFT;
        c.style.margins.left = 385;
        c
    };
    let zero_font = {
        let mut c = ass_cue("Hello Lumen");
        c.style.font_size_px = 0;
        c
    };
    let huge_font = {
        let mut c = ass_cue("Hello Lumen");
        c.style.font_size_px = 8193;
        c
    };
    let byte_guard = {
        let mut c = ass_cue("Hello Lumen");
        c.style.font_size_px = 8192;
        c.text = "x".repeat(300);
        c
    };

    // Every free-path typed failure is the twin's failure too, with
    // the same variant and payload — under the placeholder AND under
    // any other pipeline (the guards upstream of the pipeline).
    let cases: Vec<(AssRenderCue, (u32, u32), AssRenderError)> = vec![
        (
            reversed,
            pr,
            AssRenderError::EndBeforeStart {
                start_us: 5_000_000,
                end_us: 4_999_999,
            },
        ),
        (
            zero_res,
            (0, 288),
            AssRenderError::ZeroPlayRes { play_res: (0, 288) },
        ),
        (
            margin_over,
            pr,
            AssRenderError::MarginExceedsPlayRes {
                axis: "left",
                margin: 385,
                extent: 384,
            },
        ),
        (zero_font, pr, AssRenderError::ZeroFontSize),
        (
            huge_font,
            pr,
            AssRenderError::FontSizeTooLarge {
                size_px: 8193,
                max: 8192,
            },
        ),
        (
            byte_guard,
            pr,
            AssRenderError::TooLarge {
                width: 1_474_500,
                height: 9830,
            },
        ),
    ];
    for (cue, play_res, expected) in cases {
        let free = render_plain_text_cue(&cue, play_res);
        let twin = render_plain_text_cue_with_pipeline(&cue, play_res, &mono);
        assert_eq!(free, twin, "free == twin for {expected:?}");
        assert_eq!(twin, Err(expected));
        // The pipeline-independent guards stay typed under ANY
        // pipeline (the fake lane agrees with the placeholder).
        assert_eq!(
            render_plain_text_cue_with_pipeline(&cue, play_res, &fake),
            twin
        );
    }

    // EmptyText: no renderable line (empty, or only breaks) — the
    // plain twin renders cue.text only, which is why the plain-path
    // fixtures above carry real dialogue text.
    for text in ["", "\\N", "\\N\\N"] {
        let c = ass_cue(text);
        assert_eq!(
            render_plain_text_cue_with_pipeline(&c, pr, &mono),
            Err(AssRenderError::EmptyText),
            "text {text:?}"
        );
        assert_eq!(
            render_plain_text_cue(&c, pr),
            render_plain_text_cue_with_pipeline(&c, pr, &mono)
        );
    }

    // The W09-A03 audit's F-001 hole, closed: ~1.7 MB of crafted text
    // (436k breaks + one 873k-char line at the 8192-px guard) makes
    // width × height × 4 overflow u64 — the checked multiply is the
    // typed TooLarge with the REAL dimensions, never a debug panic
    // or a wrapped under-allocation.
    let crafted = format!("{}{}", "\\N".repeat(436_000), "x".repeat(873_000));
    let mut c = ass_cue("Late and long");
    c.style.font_size_px = 8192;
    c.text = crafted;
    assert_eq!(
        render_plain_text_cue_with_pipeline(&c, pr, &mono),
        Err(AssRenderError::TooLarge {
            width: 4_290_795_000,
            height: 4_285_889_830,
        })
    );
    assert_eq!(
        render_plain_text_cue(&c, pr),
        render_plain_text_cue_with_pipeline(&c, pr, &mono)
    );
    // A width past u32 itself is the u32::MAX-dimension variant.
    let mut c = ass_cue("Late and long");
    c.style.font_size_px = 8192;
    c.text = "x".repeat(874_000);
    assert_eq!(
        render_plain_text_cue_with_pipeline(&c, pr, &mono),
        Err(AssRenderError::TooLarge {
            width: u32::MAX,
            height: u32::MAX,
        })
    );

    // The styled twin's own guards: reversed interval with runs,
    // per-run zero size, per-run empty text.
    let mut c = ass_cue("Where is it?");
    c.start_us = 5_000_000;
    c.end_us = 4_999_999;
    c.runs = vec![AssRenderRun {
        text: "Where".into(),
        color: ModelColor::WHITE,
        font_size_px: 20,
    }];
    assert_eq!(
        render_styled_cue_with_pipeline(&c, pr, &mono),
        Err(AssRenderError::EndBeforeStart {
            start_us: 5_000_000,
            end_us: 4_999_999,
        })
    );
    let mut c = ass_cue("Where is it?");
    c.runs = vec![AssRenderRun {
        text: "Where".into(),
        color: ModelColor::WHITE,
        font_size_px: 0,
    }];
    assert_eq!(
        render_styled_cue_with_pipeline(&c, pr, &mono),
        Err(AssRenderError::ZeroFontSize)
    );
    let mut c = ass_cue("Where is it?");
    c.runs = vec![AssRenderRun {
        text: String::new(),
        color: ModelColor::WHITE,
        font_size_px: 20,
    }];
    assert_eq!(
        render_styled_cue_with_pipeline(&c, pr, &mono),
        Err(AssRenderError::EmptyText)
    );

    // The SRT twins keep the free path's typed failures: empty
    // extent, and the probe ceiling (3 × 3000 chars at a 65536²
    // extent — the D-2 bounded-memory refusal).
    let tm = TextMetrics::new(40, 20);
    let gm = nominal_glyph_metrics(&tm);
    let cue = srt_cue("Hello Lumen", Positioning::default());
    assert_eq!(
        place_plain_text_with_pipeline(&cue, &gm, (0, 1080), &mono),
        Err(SrtRenderError::ZeroExtent { extent: (0, 1080) })
    );
    assert_eq!(
        place_plain_text(&cue, &tm, (0, 1080)),
        place_plain_text_with_pipeline(&cue, &gm, (0, 1080), &mono)
    );
    let row = "x".repeat(3000);
    let big = srt_cue(&format!("{row}\n{row}\n{row}"), Positioning::default());
    assert!(matches!(
        place_plain_text_with_pipeline(&big, &gm, (65536, 65536), &mono),
        Err(SrtRenderError::ProbeTooLarge { .. })
    ));
    assert_eq!(
        place_plain_text(&big, &tm, (65536, 65536)),
        place_plain_text_with_pipeline(&big, &gm, (65536, 65536), &mono)
    );
}

#[test]
fn fx9_a05_styled_runs_render_through_the_pipeline() {
    let fake = FakeLane { underlined: false };
    let mono = MonospaceCellPipeline;

    // Per-run metrics from the pipeline, per-run colour tint,
    // per-run pen accumulation. "Where " at 20 px (W=20, the rest
    // 12, space 5 → 73) then "is it?" at 10 px (cell 6: i=1, s=6,
    // space=2, i=1, t=6, ?=6 → 22): 95 px wide, the max line box
    // 24, baseline the max ascent 19.
    let mut c = ass_cue("Where is it?");
    c.runs = vec![
        AssRenderRun {
            text: "Where ".into(),
            color: ModelColor::new(255, 0, 0, 255),
            font_size_px: 20,
        },
        AssRenderRun {
            text: "is it?".into(),
            color: ModelColor::new(0, 0, 255, 255),
            font_size_px: 10,
        },
    ];
    let r = render_styled_cue_with_pipeline(&c, DEFAULT_PLAY_RES, &fake).must();
    assert_eq!(
        (r.bitmap.width, r.bitmap.height, r.bitmap.baseline),
        (95, 24, 19)
    );
    // Run 1's 'W' (pen 0): border coverage 128 → red at half alpha;
    // the interior → solid red.
    assert_eq!(r.bitmap.pixel(0, 0), Rgba::RED.with_alpha(128));
    assert_eq!(r.bitmap.pixel(2, 2), Rgba::RED);
    // Run 2 starts at pen 73 ('i', 1 px wide, 10 px ascent): its
    // only column is border coverage → blue at half alpha; below
    // its ascent the line box is blank.
    assert_eq!(r.bitmap.pixel(73, 0), Rgba::BLUE.with_alpha(128));
    assert_eq!(r.bitmap.pixel(73, 10), Rgba::BLACK.with_alpha(0));
    assert_eq!(r.bitmap.pixel(73, 12), Rgba::BLACK.with_alpha(0));
    // The placeholder lane on the same cue: 6×12 + 6×6 = 108.
    let r_mono = render_styled_cue_with_pipeline(&c, DEFAULT_PLAY_RES, &mono).must();
    assert_eq!(r_mono.bitmap.width, 108);
    assert_eq!(render_styled_cue(&c, DEFAULT_PLAY_RES).must(), r_mono);

    // A `\N` inside a run splits lines with per-line advance sums:
    // "Where\Nis it?" at 20 px → line 0 "Where" = 68 (W=20 + 4×12),
    // line 1 "is it?" = 47 (i=3, s=12, space=5, i=3, t=12, ?=12) →
    // 68 wide, 2 × 24 tall, baseline 24 + 19 = 43.
    let mut c = ass_cue("Where is it?");
    c.runs = vec![AssRenderRun {
        text: "Where\\Nis it?".into(),
        color: ModelColor::WHITE,
        font_size_px: 20,
    }];
    let r = render_styled_cue_with_pipeline(&c, DEFAULT_PLAY_RES, &fake).must();
    assert_eq!(
        (r.bitmap.width, r.bitmap.height, r.bitmap.baseline),
        (68, 48, 43)
    );
    // Line 0's ink reaches its full 68-px width (the 'e' interior at
    // row 1); line 1 restarts its pen at 0 ('i' border) and ends at
    // 47 ('?' interior) — the earlier line's sum does not leak.
    assert_eq!(r.bitmap.pixel(0, 0), Rgba::WHITE.with_alpha(128));
    assert_eq!(r.bitmap.pixel(67, 1), Rgba::WHITE);
    assert_eq!(r.bitmap.pixel(0, 24), Rgba::WHITE.with_alpha(128));
    assert_eq!(r.bitmap.pixel(46, 25), Rgba::WHITE);
    assert_eq!(r.bitmap.pixel(47, 24), Rgba::BLACK.with_alpha(0));
    assert_eq!(r.bitmap.pixel(67, 24), Rgba::BLACK.with_alpha(0));
}
