// W10-A06 execution probe: does a 9-digit-year LOCAL overflow parse_iso8601_utc?
// Links the prebuilt playlists rlib; debug profile => overflow-checks ON => panic would prove the bug.
fn main() {
    let v = std::env::args().nth(1).unwrap_or_else(|| "999999999-01-01T00:00:00Z".to_string());
    let tag = format!("MPEGTS=0,LOCAL={v}");
    match playlists::hls::TimestampMap::parse_tag_value(&tag) {
        Some(m) => println!("PARSED local_ms={}", m.local_ms),
        None => println!("NONE (refused)"),
    }
}
