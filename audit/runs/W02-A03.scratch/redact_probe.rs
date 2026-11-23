// W02-A03 probe: does SourceUri::redacted() mask common credential query params?
fn main() {
    let cases = [
        "https://stream.example.org/v?access_token=ABC123",
        "https://stream.example.org/v?refresh_token=ABC123",
        "https://stream.example.org/v?client_secret=ABC123",
        "https://stream.example.org/v?auth_token=ABC123",
        "https://stream.example.org/v?token=ABC123",
        "https://stream.example.org/v?passwd=ABC123",
        "https://stream.example.org/v?apikey=ABC123",
        "https://stream.example.org/v?sessionid=ABC123",
        "https://stream.example.org/v#frag?token=LEAKME",
    ];
    for c in cases {
        match media_types::SourceUri::parse(c) {
            Ok(u) => println!("{c}\n  -> redacted: {}", u.redacted()),
            Err(e) => println!("{c}\n  -> parse error: {e}"),
        }
    }
    match media_types::SourceUri::from_file_path("//server/share/x.mkv") {
        Ok(u) => println!("from_file_path(//server/share/x.mkv) -> raw={}", u.raw()),
        Err(e) => println!("from_file_path(//server/share/x.mkv) -> err: {e}"),
    }
}
