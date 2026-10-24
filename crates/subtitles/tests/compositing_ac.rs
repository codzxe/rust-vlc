//! Card 5.L1 acceptance: synthetic cue RENDER tests — position
//! (10 %/90 % pixel-exact on 1920×1080), overlap z-order stacking,
//! alpha blending, and scaling CIF↔4K — all through the real
//! [`SubtitleCompositor::compose`] path onto an [`Rgba8Surface`],
//! with hand-computed expected pixels (integer math only, no
//! approximation anywhere in the chain).

use media_types::time::{Stamp, TimeSpan, Timestamp};
use subtitles::compositing::{
    Anchor, CompositedCue, CueBitmap, CueLayout, HorizontalAlign, Rational, Rgba, Rgba8Surface,
    SubtitleCompositor, VerticalAlign,
};
use testkit::Must;

fn ns(ticks: i64) -> Timestamp {
    Timestamp::from_parts(ticks, 1, 1_000_000_000).must()
}

fn span(t0: i64, t1: i64) -> TimeSpan {
    TimeSpan {
        start: Stamp::Present(ns(t0)),
        end: Stamp::Present(ns(t1)),
    }
}

fn layout(u: (u32, u32), v: (u32, u32)) -> CueLayout {
    let mut layout = CueLayout::BOTTOM_CENTER;
    layout.anchor = Anchor::new(
        Rational::new(u.0, u.1).must(),
        Rational::new(v.0, v.1).must(),
    )
    .must();
    layout.alignment.horizontal = HorizontalAlign::Center;
    layout.alignment.vertical = VerticalAlign::Center;
    layout
}

#[test]
fn position_10_90_is_pixel_exact_on_1080p() {
    // 1/10 of 1920 = 192; 9/10 of 1080 = 972 — exact rationals of the
    // extent. A 128x32 center-aligned cue lands at top-left
    // (192 - 64, 972 - 16) = (128, 956), inside the 5% title-safe box
    // (x >= 96, bottom 988 <= 1026): NO clamping, pure anchor math.
    let mut target = Rgba8Surface::new((1920, 1080), Rgba::WHITE).must();
    let cue = CompositedCue::new(
        span(0, 1_000_000_000),
        CueBitmap::solid(128, 32, Rgba::BLACK).must(),
        layout((1, 10), (9, 10)),
        0,
    )
    .must();
    let mut compositor = SubtitleCompositor::new();
    let drawn = compositor
        .compose(&[cue], ns(500_000_000), &mut target)
        .must();
    assert_eq!(drawn, 1);

    // Cue rect corners are exactly the anchor math: (128,956)..
    // (255,987). Corners inside are cue-black, one pixel outside is
    // the untouched white background.
    assert_eq!(target.pixel(128, 956).must(), Rgba::BLACK);
    assert_eq!(target.pixel(255, 987).must(), Rgba::BLACK);
    assert_eq!(target.pixel(191, 971).must(), Rgba::BLACK);
    assert_eq!(target.pixel(127, 955).must(), Rgba::WHITE);
    assert_eq!(target.pixel(256, 988).must(), Rgba::WHITE);
    assert_eq!(target.pixel(255, 955).must(), Rgba::WHITE);
    assert_eq!(target.pixel(128, 988).must(), Rgba::WHITE);
}

#[test]
fn position_top_right_quadrant_is_pixel_exact() {
    // 9/10 of 1920 = 1728, 1/10 of 1080 = 108: top-left
    // (1728 - 64, 108 - 16) = (1664, 92), inside the safe box.
    let mut target = Rgba8Surface::new((1920, 1080), Rgba::WHITE).must();
    let cue = CompositedCue::new(
        span(0, 1_000_000_000),
        CueBitmap::solid(128, 32, Rgba::BLACK).must(),
        layout((9, 10), (1, 10)),
        0,
    )
    .must();
    let mut compositor = SubtitleCompositor::new();
    compositor
        .compose(&[cue], ns(500_000_000), &mut target)
        .must();
    assert_eq!(target.pixel(1664, 92).must(), Rgba::BLACK);
    assert_eq!(target.pixel(1791, 123).must(), Rgba::BLACK);
    assert_eq!(target.pixel(1663, 91).must(), Rgba::WHITE);
    assert_eq!(target.pixel(1792, 124).must(), Rgba::WHITE);
}

#[test]
fn semi_transparent_blends_round_to_nearest() {
    // White at a=128 over a black video plane:
    // (128*255 + 127*0 + 127)/255 = 32767/255 = 128 per channel.
    let mut target = Rgba8Surface::new((64, 64), Rgba::BLACK).must();
    let cue = CompositedCue::new(
        span(0, 1_000_000_000),
        CueBitmap::solid(16, 16, Rgba::WHITE.with_alpha(128)).must(),
        layout((1, 2), (1, 2)),
        0,
    )
    .must();
    let mut compositor = SubtitleCompositor::new();
    compositor.compose(&[cue], ns(0), &mut target).must();
    let px = target.pixel(32, 32).must();
    assert_eq!(
        px,
        Rgba {
            r: 128,
            g: 128,
            b: 128,
            a: 255
        }
    );
    // Outside the 16x16 cue (top-left at (32-8, 32-8) = (24,24)):
    // untouched black video.
    assert_eq!(target.pixel(23, 23).must(), Rgba::BLACK);
}

#[test]
fn overlap_stacks_by_z_order_not_insertion_luck() {
    // Two 64x64 cues at the frame center: red opaque (z=0) and blue
    // at a=128 (z=1). Blue must blend OVER red in the intersection —
    // and swapping the z keys must swap the visible result.
    let red = CueBitmap::solid(64, 64, Rgba::RED).must();
    let blue = CueBitmap::solid(64, 64, Rgba::BLUE.with_alpha(128)).must();
    let center = layout((1, 2), (1, 2));
    let mut target = Rgba8Surface::new((1920, 1080), Rgba::WHITE).must();
    let mut compositor = SubtitleCompositor::new();

    let bottom = CompositedCue::new(span(0, 1_000_000_000), red.clone(), center, 0).must();
    let top = CompositedCue::new(span(0, 1_000_000_000), blue.clone(), center, 1).must();
    let drawn = compositor
        .compose(&[bottom, top], ns(0), &mut target)
        .must();
    assert_eq!(drawn, 2);
    // (a*s + (255-a)*d + 127)/255 with a=128, s=blue, d=red:
    // r = (0 + 127*255 + 127)/255 = 127; g = 0; b = (128*255 + 127)/255 = 128.
    assert_eq!(
        target.pixel(960, 540).must(),
        Rgba {
            r: 127,
            g: 0,
            b: 128,
            a: 255
        }
    );

    // Swapped z: the OPAQUE cue is first in the slice but has the
    // HIGHER z — it must still draw on top and fully cover the
    // translucent blue beneath (if z were ignored, slice order would
    // put blue on top and tint the result to [127, 0, 128]).
    let mut target = Rgba8Surface::new((1920, 1080), Rgba::WHITE).must();
    let bottom = CompositedCue::new(span(0, 1_000_000_000), blue, center, 0).must();
    let top = CompositedCue::new(span(0, 1_000_000_000), red, center, 1).must();
    compositor
        .compose(&[top, bottom], ns(0), &mut target)
        .must();
    assert_eq!(target.pixel(960, 540).must(), Rgba::RED);

    // The mirror: the TRANSLUCENT cue is first in the slice with the
    // HIGHER z — it tints the opaque red beneath ([127, 0, 128]);
    // slice order alone would have covered it with red.
    let red = CueBitmap::solid(64, 64, Rgba::RED).must();
    let blue = CueBitmap::solid(64, 64, Rgba::BLUE.with_alpha(128)).must();
    let mut target = Rgba8Surface::new((1920, 1080), Rgba::WHITE).must();
    let bottom = CompositedCue::new(span(0, 1_000_000_000), red, center, 0).must();
    let top = CompositedCue::new(span(0, 1_000_000_000), blue, center, 1).must();
    compositor
        .compose(&[top, bottom], ns(0), &mut target)
        .must();
    assert_eq!(
        target.pixel(960, 540).must(),
        Rgba {
            r: 127,
            g: 0,
            b: 128,
            a: 255
        }
    );

    // Tie on z: the LATER slice entry draws on top (track order).
    let red = CueBitmap::solid(64, 64, Rgba::RED).must();
    let blue = CueBitmap::solid(64, 64, Rgba::BLUE.with_alpha(128)).must();
    let mut target = Rgba8Surface::new((1920, 1080), Rgba::WHITE).must();
    let first = CompositedCue::new(span(0, 1_000_000_000), red, center, 0).must();
    let second = CompositedCue::new(span(0, 1_000_000_000), blue, center, 0).must();
    compositor
        .compose(&[first, second], ns(0), &mut target)
        .must();
    assert_eq!(
        target.pixel(960, 540).must(),
        Rgba {
            r: 127,
            g: 0,
            b: 128,
            a: 255
        }
    );
}

#[test]
fn partial_overlap_has_exact_seam() {
    // Red 64x64 at the center (rect x 928..992); translucent blue
    // 64x64 shifted right by 32 px (anchor u = 1/2 + 32/1920 = 31/60,
    // rect x 960..1024). Left of the seam: pure red; right of it:
    // blue over red; past 1024: untouched video.
    let mut target = Rgba8Surface::new((1920, 1080), Rgba::BLACK).must();
    let mut compositor = SubtitleCompositor::new();
    let red = CompositedCue::new(
        span(0, 1_000_000_000),
        CueBitmap::solid(64, 64, Rgba::RED).must(),
        layout((1, 2), (1, 2)),
        0,
    )
    .must();
    let blue = CompositedCue::new(
        span(0, 1_000_000_000),
        CueBitmap::solid(64, 64, Rgba::BLUE.with_alpha(128)).must(),
        layout((31, 60), (1, 2)),
        1,
    )
    .must();
    compositor.compose(&[red, blue], ns(0), &mut target).must();
    // 31/60 of 1920 = 992.5 -> 992 (round to nearest); top-left
    // (992 - 32, 540 - 32) = (960, 508). Red spans x [928, 992),
    // blue spans x [960, 1024): the overlap is [960, 992).
    assert_eq!(target.pixel(959, 540).must(), Rgba::RED);
    assert_eq!(
        target.pixel(960, 540).must(),
        Rgba {
            r: 127,
            g: 0,
            b: 128,
            a: 255
        }
    );
    // The LAST overlap pixel, then blue alone over the background,
    // then untouched video one past blue's right edge.
    assert_eq!(
        target.pixel(991, 540).must(),
        Rgba {
            r: 127,
            g: 0,
            b: 128,
            a: 255
        }
    );
    assert_eq!(
        target.pixel(1023, 540).must(),
        Rgba {
            r: 0,
            g: 0,
            b: 128,
            a: 255
        }
    );
    assert_eq!(target.pixel(1024, 540).must(), Rgba::BLACK);
}

#[test]
fn scaling_cif_authored_cue_grows_to_4k() {
    // A 128x32 cue authored for CIF (352x288) presented on 4K
    // (3840x2160): uniform scale = min(3840/352, 2160/288) = 15/2 —
    // 128x32 -> exactly 960x240 (128*15/2 = 960, 32*15/2 = 240).
    // Anchor stays RELATIVE: bottom-center (1/2, 9/10) of 4K pins the
    // rect at (1920 - 480, 1944 - 240) = (1440, 1704).
    let mut rgba = Vec::with_capacity(128 * 32 * 4);
    for sy in 0..32u32 {
        for sx in 0..128u32 {
            rgba.extend_from_slice(&[sx as u8, sy as u8, (255 - sx) as u8, 255]);
        }
    }
    let bitmap = CueBitmap::from_rgba(128, 32, 32, rgba).must();
    let mut cue_layout = layout((1, 2), (9, 10));
    cue_layout.alignment.vertical = VerticalAlign::Bottom;
    cue_layout.reference = Some((352, 288));
    let cue = CompositedCue::new(span(0, 1_000_000_000), bitmap, cue_layout, 0).must();

    // The placement itself: exact rect.
    let rect = subtitles::compositing::place_cue(&cue.bitmap, &cue.layout, (3840, 2160)).must();
    assert_eq!(
        (rect.x, rect.y, rect.width, rect.height),
        (1440, 1704, 960, 240)
    );

    // The render: nearest-neighbor pixel-center mapping. dst(1440,1704)
    // <- src(0,0); dst(2399,1704) <- src(127,0) = (1919*128)/1920 = 127;
    // dst(1440,1943) <- src(0,31) = (479*32)/480 = 31.
    let mut target = Rgba8Surface::new((3840, 2160), Rgba::BLACK).must();
    let mut compositor = SubtitleCompositor::new();
    let drawn = compositor.compose(&[cue], ns(0), &mut target).must();
    assert_eq!(drawn, 1);
    assert_eq!(
        target.pixel(1440, 1704).must(),
        Rgba {
            r: 0,
            g: 0,
            b: 255,
            a: 255
        }
    );
    assert_eq!(
        target.pixel(2399, 1704).must(),
        Rgba {
            r: 127,
            g: 0,
            b: 128,
            a: 255
        }
    );
    assert_eq!(
        target.pixel(1440, 1943).must(),
        Rgba {
            r: 0,
            g: 31,
            b: 255,
            a: 255
        }
    );
    // One pixel outside the scaled rect: untouched.
    assert_eq!(target.pixel(1439, 1704).must(), Rgba::BLACK);
    assert_eq!(target.pixel(1440, 1944).must(), Rgba::BLACK);
}

#[test]
fn scaling_4k_authored_cue_shrinks_to_cif() {
    // The reverse: 240x120 authored at 4K, presented at CIF — scale
    // 352/3840 = 11/120; 240x120 -> (240*11+60)/120 = 22, (1320+60)/120
    // = 11. Anchor (1/2, 9/10) of CIF = (176, 259): rect (165, 250, 22, 11).
    let mut rgba = Vec::with_capacity(240 * 120 * 4);
    for sy in 0..120u32 {
        for sx in 0..240u32 {
            rgba.extend_from_slice(&[sx as u8, sy as u8, (255 - sx) as u8, 255]);
        }
    }
    let bitmap = CueBitmap::from_rgba(240, 120, 120, rgba).must();
    let mut cue_layout = layout((1, 2), (9, 10));
    cue_layout.alignment.vertical = VerticalAlign::Bottom;
    cue_layout.reference = Some((3840, 2160));
    let cue = CompositedCue::new(span(0, 1_000_000_000), bitmap, cue_layout, 0).must();

    let rect = subtitles::compositing::place_cue(&cue.bitmap, &cue.layout, (352, 288)).must();
    assert_eq!(
        (rect.x, rect.y, rect.width, rect.height),
        (165, 248, 22, 11)
    );

    let mut target = Rgba8Surface::new((352, 288), Rgba::BLACK).must();
    let mut compositor = SubtitleCompositor::new();
    compositor.compose(&[cue], ns(0), &mut target).must();
    // dst(165,248) <- src(5,5) = (1*240)/44 = 5.45 -> 5, (1*120)/22 = 5.
    assert_eq!(
        target.pixel(165, 248).must(),
        Rgba {
            r: 5,
            g: 5,
            b: 250,
            a: 255
        }
    );
    // dst(186,260) <- src(234,114) = (43*240)/44 = 234.5 -> 234,
    // (21*120)/22 = 114.5 -> 114.
    assert_eq!(
        target.pixel(186, 258).must(),
        Rgba {
            r: 234,
            g: 114,
            b: 21,
            a: 255
        }
    );
}

#[test]
fn inactive_cues_draw_nothing() {
    // Not-yet-started and already-ended cues leave the plane alone;
    // the compositor reports how many cues it drew.
    let mut target = Rgba8Surface::new((64, 64), Rgba::WHITE).must();
    let mut compositor = SubtitleCompositor::new();
    let early = CompositedCue::new(
        span(1_000_000_000, 2_000_000_000),
        CueBitmap::solid(16, 16, Rgba::BLACK).must(),
        layout((1, 2), (1, 2)),
        0,
    )
    .must();
    let late = CompositedCue::new(
        span(0, 1_000_000_000),
        CueBitmap::solid(16, 16, Rgba::BLACK).must(),
        layout((1, 2), (1, 2)),
        0,
    )
    .must();
    let cues = [early, late];
    // now = 3s: both outside their intervals.
    let drawn = compositor
        .compose(&cues, ns(3_000_000_000), &mut target)
        .must();
    assert_eq!(drawn, 0);
    assert_eq!(target.pixel(32, 32).must(), Rgba::WHITE);
    // now = 1.5s: exactly the early one.
    let drawn = compositor
        .compose(&cues, ns(1_500_000_000), &mut target)
        .must();
    assert_eq!(drawn, 1);
    assert_eq!(target.pixel(32, 32).must(), Rgba::BLACK);
}
