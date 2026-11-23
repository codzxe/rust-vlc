// W09-A01 probe 5: run the SRT parser over my five assigned fuzz fixtures and
// report cue counts + damage kinds (what each mutation actually produces).
use subtitles::srt;

fn main() {
    let dir = "/home/z/my-project/audit-workspace/crates/subtitles/tests/fixtures/fuzz/";
    for name in ["xff3_000.srt", "bom_be_on_utf8.srt", "flip2_056.srt", "flip2_096.srt", "trunc2_032.srt"] {
        let bytes = std::fs::read(format!("{dir}{name}")).unwrap();
        let doc = srt::parse(&bytes);
        let kinds: Vec<String> = doc
            .damages
            .iter()
              .map(|d| format!("{:?}", d.kind))
            .collect();
        let texts: Vec<&str> = doc.cues.iter().map(|c| match &c.text {
            subtitles::model::cue::CueText::Plain(t) => t.as_str(),
            subtitles::model::cue::CueText::Styled(_) => "<styled>",
        }).collect();
        println!(
            "{name}: cues={} damages={} kinds={:?} texts={:?}",
            doc.cues.len(),
            doc.damages.len(),
            kinds,
            texts
        );
    }
}
