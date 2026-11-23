// W09-A07 probe: exercise subtitles::srt::parse on the committed fixtures.
// Compiled against the prebuilt debug rlib (no cargo build).
use std::path::Path;

fn probe(path: &str) {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            println!("== {path}: READ-ERROR {e}");
            return;
        }
    };
    let doc = subtitles::srt::parse(&bytes);
    println!(
        "== {path}: {} bytes, {} cues, {} damage (dropped {}), encoding {:?}",
        bytes.len(),
        doc.cues.len(),
        doc.damages.len(),
        doc.dropped_damage_events,
        doc.encoding
    );
    for c in doc.cues.iter().take(4) {
        println!(
        "   cue id={} [{},{}] text={:?}",
        c.id.get(),
        c.interval.start_us(),
        c.interval.end_us(),
        c.text
    );
    }
    for d in doc.damages.iter().take(8) {
        println!("   damage {:?} @line {}", d.kind, d.line_no);
    }
    let _ = Path::new(path);
}
fn main() {
    let base = "/home/z/my-project/audit-workspace/crates/subtitles/tests/fixtures";
    for f in [
        "/canonical/out1.srt",
        "/fuzz/xff3_120.srt",
        "/fuzz/flip2_024.srt",
        "/fuzz/flip2_112.srt",
        "/fuzz/trunc2_064.srt",
    ] {
        probe(&format!("{base}{f}"));
    }
    // Hand-hostile inputs (W09-A07 own cases):
    let doc = subtitles::srt::parse_str("00:00:01.5 --> 00:00:02,25\nx\n");
    println!(
        "== probe `,5` semantics: start={} end={} (doc says ,5=500ms; test says 5ms)",
        doc.cues[0].interval.start_us(),
        doc.cues[0].interval.end_us()
    );
    let doc = subtitles::srt::parse_str("00:00:01,000 junk --> 00:00:02,000\nx\n");
    println!(
        "== probe start-side trailing junk: cues={} damages={:?}",
        doc.cues.len(),
        doc.damages.iter().map(|d| d.kind).collect::<Vec<_>>()
    );
    let doc = subtitles::srt::parse_str("1\n00:00:01,000 --> 00:00:02,000\na\n2\n00:00:05,000 --> 00:00:03,000\nreversed-after-resync\n");
    println!(
        "== probe reversed-at-resync: cues={} damages={:?}",
        doc.cues.len(),
        doc.damages.iter().map(|d| d.kind).collect::<Vec<_>>()
    );
    let doc = subtitles::srt::parse_str("\u{FEFF}1\n00:00:01,000 --> 00:00:02,000\nx\n");
    println!(
        "== probe stray FEFF via parse_str: cues={} damages={:?}",
        doc.cues.len(),
        doc.damages.iter().map(|d| d.kind).collect::<Vec<_>>()
    );
    let mut b = b"\xEF\xBB\xBF".to_vec();
    b.extend_from_slice(b"\xEF\xBB\xBF1\n00:00:01,000 --> 00:00:02,000\nx\n");
    let doc = subtitles::srt::parse(&b);
    println!(
        "== probe double UTF-8 BOM: cues={} damages={:?}",
        doc.cues.len(),
        doc.damages.iter().map(|d| d.kind).collect::<Vec<_>>()
    );
}
