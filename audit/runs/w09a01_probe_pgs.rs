// W09-A01 probe 4: execute pgs palette conversion (white/black under both
// matrices + the 709 red from the unit test) through the public API.
use subtitles::pgs::palette::{ColorMatrix, Palette};
use subtitles::pgs::segment::PaletteEntry;

fn main() {
    let mk = |y, cb, cr, a| PaletteEntry {
        id: 5,
        y,
        cb,
        cr,
        alpha: a,
    };
    for matrix in [ColorMatrix::Bt601, ColorMatrix::Bt709] {
        let p = Palette::from_entries(1, 0, std::iter::once(mk(235, 128, 128, 255)), matrix);
        let c = p.rgba[5];
        println!("{matrix:?} studio-white -> r={} g={} b={} a={}", c.r, c.g, c.b, c.a);
    }
    let p = Palette::from_entries(1, 0, std::iter::once(mk(81, 90, 240, 255)), ColorMatrix::Bt709);
    let c = p.rgba[5];
    println!("Bt709 red(81,90,240) -> r={} g={} b={} a={}", c.r, c.g, c.b, c.a);
    let p = Palette::from_entries(1, 0, std::iter::once(mk(16, 128, 128, 255)), ColorMatrix::Bt601);
    let c = p.rgba[5];
    println!("Bt601 studio-black -> r={} g={} b={} a={}", c.r, c.g, c.b, c.a);
    println!("height rule: 576={:?} 577={:?} 1080={:?}",
        ColorMatrix::for_video_height(576),
        ColorMatrix::for_video_height(577),
        ColorMatrix::for_video_height(1080));
}
