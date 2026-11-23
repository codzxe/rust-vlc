use std::time::Instant;
fn main() {
    // O(n^2) probe: one cue whose text is 200_000 bare '&'.
    let mut doc = String::from("WEBVTT\n\n00:00:01.000 --> 00:00:02.000\n");
    doc.push_str(&"&".repeat(200_000));
    doc.push_str("\n");
    let t = Instant::now();
    let parsed = subtitles::vtt::parse(&doc).unwrap();
    let d1 = t.elapsed();
    println!("amp-only 200KB: cues={} elapsed={:?}", parsed.cues.len(), d1);

    // Mixed pathological: & every 2 chars, no ';'.
    let mut doc2 = String::from("WEBVTT\n\n00:00:01.000 --> 00:00:02.000\n");
    let chunk = "&x";
    for _ in 0..100_000 { doc2.push_str(chunk); }
    doc2.push_str("\n");
    let t = Instant::now();
    let parsed2 = subtitles::vtt::parse(&doc2).unwrap();
    println!("&x pattern 200KB: elapsed={:?}", t.elapsed());
    let _ = parsed2.cues.len();

    // Cross-line tag strip behavior:
    let doc3 = b"WEBVTT\n\n00:00:01.000 --> 00:00:02.000\na < b\nc > d\n";
    let p3 = subtitles::vtt::parse(std::str::from_utf8(doc3).unwrap()).unwrap();
    for c in &p3.cues {
        println!("cross-line-tag cue text = {:?}", c.text);
    }
    let doc4 = b"WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nkeep <oops dropped\nsecond line\n";
    let p4 = subtitles::vtt::parse(std::str::from_utf8(doc4).unwrap()).unwrap();
    for c in &p4.cues {
        println!("unterminated-then-more-lines text = {:?}", c.text);
    }
}
