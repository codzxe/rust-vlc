fn main() {
    // Overlapping cues: must survive as two independent cues.
    let doc = subtitles::srt::parse_str(
        "1\n00:00:01,000 --> 00:00:05,000\nfirst\n\n2\n00:00:02,000 --> 00:00:03,000\nsecond (inside first)\n",
    );
    println!(
        "== overlap: cues={} clean={} ids={:?} spans=({},{}) ({},{})",
        doc.cues.len(),
        doc.is_clean(),
        doc.cues.iter().map(|c| c.id.get()).collect::<Vec<_>>(),
        doc.cues[0].interval.start_us(),
        doc.cues[0].interval.end_us(),
        doc.cues.get(1).map(|c| c.interval.start_us()).unwrap_or(-1),
        doc.cues.get(1).map(|c| c.interval.end_us()).unwrap_or(-1),
    );
    // Duplicate ids / same times.
    let doc = subtitles::srt::parse_str(
        "1\n00:00:01,000 --> 00:00:02,000\na\n\n1\n00:00:01,000 --> 00:00:02,000\nb\n",
    );
    println!("== dup ids: cues={} clean={}", doc.cues.len(), doc.is_clean());
    // Numeric-only text line before a run-together cue.
    let doc = subtitles::srt::parse_str("1\n00:00:01,000 --> 00:00:02,000\n2\n00:00:03,000 --> 00:00:04,000\nsecond\n");
    println!(
        "== numeric-only text: cues={} damages={:?}",
        doc.cues.len(),
        doc.damages.iter().map(|d| d.kind).collect::<Vec<_>>()
    );
    // Minutes/seconds >59 roll up, hours >23 legal (already unit-tested; direct probe).
    let doc = subtitles::srt::parse_str("00:75:00,000 --> 00:76:00,000\nx\n");
    println!("== 75min: start={} clean={}", doc.cues[0].interval.start_us(), doc.is_clean());
}
