// W09-A01 probe: does a crafted PlayResX + \pos overflow `num + den/2`
// in ass::parser::apply_position_override (debug rlib: overflow checks on)?
fn main() {
    let src = concat!(
        "[Script Info]\n",
        "PlayResX: 9223372036854775\n",
        "PlayResY: 288\n",
        "\n",
        "[Events]\n",
        "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        "Dialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,{\\pos(92233720368547,10)}boom\n"
    );
    let doc = subtitles::ass::parse(src);
    println!(
        "SURVIVED cues={} unmappable={} applied={}",
        doc.cues.len(),
        doc.stats.position_overrides_unmappable,
        doc.stats.position_overrides_applied
    );
}
