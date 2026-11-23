// W09-A01 probe 3: negative \pos coordinate -> clamped to 0% instead of
// unmappable? (parser.rs doc says "never clamped", "round-half-up")
fn main() {
    let src = concat!(
        "[Script Info]\n",
        "PlayResX: 384\n",
        "PlayResY: 288\n",
        "\n",
        "[Events]\n",
        "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        "Dialogue: 0,0:00:01.00,0:00:02.00,Default,,0,0,0,,{\\pos(-5,10)}neg\n",
        "Dialogue: 0,0:00:03.00,0:00:04.00,Default,,0,0,0,,{\\pos(-6,10)}neg2\n"
    );
    let doc = subtitles::ass::parse(src);
    for cue in &doc.cues {
        println!(
            "pos={:?} line={:?}",
            cue.positioning.position.map(|p| p.get()),
            cue.positioning.line
        );
    }
    println!(
        "applied={} unmappable={}",
        doc.stats.position_overrides_applied, doc.stats.position_overrides_unmappable
    );
}
