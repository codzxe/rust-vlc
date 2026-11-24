// Standalone reproduction of crates/demux/src/asf/header.rs:553-562's
// child-walk arithmetic, driven with the crafted child size u64::MAX.
fn main() {
    let bytes: Vec<u8> = vec![0u8; 62];            // header buffer (header_size = 62)
    let header_size: u64 = std::hint::black_box(62);
    let off: usize = 30;                           // first child offset (header.rs:547)
    let size: u64 = std::hint::black_box(u64::MAX); // child size QWORD read at off+16 (header.rs:554)
    if size < 24 { return; }                        // header.rs:555 guard passes
    let end = off as u64 + size;                    // header.rs:558 (unchecked add)
    if end > header_size { return; }                 // header.rs:559
    let payload = &bytes[off + 24..end as usize];   // header.rs:562
    println!("payload len {}", payload.len());
}
