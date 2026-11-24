fn main() {
    let path = std::env::args().nth(1).expect("usage: ass_driver <file.ass>");
    let text = std::fs::read_to_string(&path).expect("read");
    let doc = subtitles::ass::parse(&text);
    println!("parsed ok: cues={}", doc.cues.len());
}
