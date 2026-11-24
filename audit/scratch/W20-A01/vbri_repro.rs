// Verbatim-shape reproduction of crates/codecs/src/audio/mp3/xing.rs:339-367
// driven by a 36-byte MPEG-2 L3 8kbps/16kHz mono frame carrying "VBRI"
// at the ancillary offset 13 (side info LSF mono = 9) — rest.len() = 19.
fn parse_vbri_rest(frame: &[u8]) -> usize {
    let pos = 13usize;                                  // find_magic_vbri hit
    let rest = frame.get(pos + 4..).unwrap_or(&[]);     // xing.rs:339
    if rest.len() < 14 {                                // xing.rs:340 guard
        return 0;                                       // (typed error path)
    }
    let _frames = u32::from_be_bytes([rest[10], rest[11], rest[12], rest[13]]); // :355
    let toc_entries = u16::from_be_bytes([rest[14], rest[15]]);          // :364
    let _toc_scale = u16::from_be_bytes([rest[16], rest[17]]);           // :365
    let _toc_entry_size = u16::from_be_bytes([rest[18], rest[19]]);      // :366
    let _toc_frames_per_entry = u16::from_be_bytes([rest[20], rest[21]]);// :367
    usize::from(toc_entries)
}

fn main() {
    let mut frame = vec![0xFFu8, 0xF2, 0x18, 0xC0];
    frame.extend_from_slice(&[0u8; 9]);        // side info (LSF mono)
    frame.extend_from_slice(b"VBRI");           // magic at 13
    frame.extend_from_slice(&[0x41u8; 19]);     // rest = 19 nonzero bytes
    assert_eq!(frame.len(), 36);
    let n = parse_vbri_rest(&frame);
    println!("toc_entries={n} (no panic)");
}
