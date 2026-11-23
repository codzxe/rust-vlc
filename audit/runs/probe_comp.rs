use subtitles::compositing::{CueBitmap, CompositedCue, CueLayout, Rgba, Rgba8Surface, SubtitleCompositor};
use media_types::time::{Stamp, TimeSpan, Timestamp, TimeBase};

fn ts(ns: i64) -> Timestamp {
    let tb = TimeBase::new(1, 1_000_000_000).unwrap();
    Timestamp::new(ns, tb)
}

fn main() {
    let rgba = vec![255,0,0,128, 0,0,255,255];
    let bmp = CueBitmap::from_rgba(2, 1, 0, rgba).unwrap();
    let layout = CueLayout::BOTTOM_CENTER;
    let span = TimeSpan { start: Stamp::Present(ts(0)), end: Stamp::Present(ts(1_000_000_000)) };
    let cue = CompositedCue::new(span, bmp, layout, 0).unwrap();
    let mut surf = Rgba8Surface::new((8, 4), Rgba::WHITE).unwrap();
    let mut comp = SubtitleCompositor::new();
    let n = comp.compose(std::slice::from_ref(&cue), ts(500_000_000), &mut surf).unwrap();
    let mut found = Vec::new();
    for y in 0..4 { for x in 0..8 {
        let p = surf.pixel(x, y).unwrap();
        if p != Rgba::WHITE { found.push((x, y, p)); }
    }}
    println!("drawn={} non-white pixels:", n);
    for (x, y, p) in &found { println!("  ({x},{y}) r={} g={} b={} a={}", p.r, p.g, p.b, p.a); }
    let n2 = comp.compose(std::slice::from_ref(&cue), ts(2_000_000_000), &mut surf).unwrap();
    println!("at 2s drawn={} (expect 0)", n2);
    // z-order: blue z=0, red z=1 same placement -> red on top
    let red = CueBitmap::from_rgba(1, 1, 0, vec![255,0,0,255]).unwrap();
    let blue = CueBitmap::from_rgba(1, 1, 0, vec![0,0,255,255]).unwrap();
    let c0 = CompositedCue::new(TimeSpan { start: Stamp::Present(ts(0)), end: Stamp::Present(ts(1_000_000_000)) }, blue, CueLayout::BOTTOM_CENTER, 0).unwrap();
    let c1 = CompositedCue::new(TimeSpan { start: Stamp::Present(ts(0)), end: Stamp::Present(ts(1_000_000_000)) }, red, CueLayout::BOTTOM_CENTER, 1).unwrap();
    let mut surf2 = Rgba8Surface::new((8, 4), Rgba::WHITE).unwrap();
    comp.compose(&[c0, c1], ts(0), &mut surf2).unwrap();
    let mut nz = Vec::new();
    for y in 0..4 { for x in 0..8 { let p = surf2.pixel(x, y).unwrap(); if p != Rgba::WHITE { nz.push((x,y,(p.r,p.g,p.b))); } } }
    println!("z-order non-white: {:?} (expect red on top: (255,0,0))", nz);
    // overlap blend: red a=128 over blue a=255 at same spot
    let rb = CueBitmap::from_rgba(1, 1, 0, vec![255,0,0,128]).unwrap();
    let bb = CueBitmap::from_rgba(1, 1, 0, vec![0,0,255,255]).unwrap();
    let d0 = CompositedCue::new(TimeSpan { start: Stamp::Present(ts(0)), end: Stamp::Present(ts(1_000_000_000)) }, bb, CueLayout::BOTTOM_CENTER, 0).unwrap();
    let d1 = CompositedCue::new(TimeSpan { start: Stamp::Present(ts(0)), end: Stamp::Present(ts(1_000_000_000)) }, rb, CueLayout::BOTTOM_CENTER, 1).unwrap();
    let mut surf3 = Rgba8Surface::new((8, 4), Rgba::WHITE).unwrap();
    comp.compose(&[d0, d1], ts(0), &mut surf3).unwrap();
    let mut ov = Vec::new();
    for y in 0..4 { for x in 0..8 { let p = surf3.pixel(x, y).unwrap(); if p != Rgba::WHITE { ov.push((x,y,(p.r,p.g,p.b,p.a))); } } }
    println!("overlap blend: {:?} (expect r=(128*255+127*0+127)/255=128, g=0, b=(128*0+127*255+127)/255=127)", ov);
}
