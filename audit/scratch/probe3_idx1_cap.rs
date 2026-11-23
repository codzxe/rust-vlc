// Standalone: does a physically-real idx1 payload over MAX_IDX1_ENTRIES kill the open?
use demux::avi::AviDemuxer;
use plugin_api::Demuxer;

fn chunk(out: &mut Vec<u8>, fourcc: &[u8; 4], payload: &[u8]) -> usize {
    let offset = out.len();
    out.extend_from_slice(fourcc);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    if payload.len() & 1 == 1 {
        out.push(0);
    }
    offset
}
fn end_sized(out: &mut Vec<u8>, size_pos: usize) {
    let size = out.len() - size_pos - 4;
    let bytes = (size as u32).to_le_bytes();
    out[size_pos..size_pos + 4].copy_from_slice(&bytes);
}
fn strh(scale: u32, rate: u32, length: u32, ssize: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(56);
    v.extend_from_slice(b"vids");
    v.extend_from_slice(b"MJPG");
    v.extend_from_slice(&[0u8; 12]);
    v.extend_from_slice(&scale.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&length.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&ssize.to_le_bytes());
    v.extend_from_slice(&[0u8; 8]);
    v
}
fn strf_video() -> Vec<u8> {
    let mut v = Vec::with_capacity(40);
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&320u32.to_le_bytes());
    v.extend_from_slice(&240i32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&24u16.to_le_bytes());
    v.extend_from_slice(b"MJPG");
    v.extend_from_slice(&[0u8; 20]);
    v
}
fn build_avi(idx1_entries: usize) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"RIFF");
    let rp = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"AVI ");
    out.extend_from_slice(b"LIST");
    let hp = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"hdrl");
    let mut avih = Vec::new();
    for value in [33_333u32, 0, 0, 0x110, 60, 0, 1, 0, 320, 240] {
        avih.extend_from_slice(&value.to_le_bytes());
    }
    avih.extend_from_slice(&[0u8; 16]);
    chunk(&mut out, b"avih", &avih);
    out.extend_from_slice(b"LIST");
    let sp = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"strl");
    chunk(&mut out, b"strh", &strh(1, 30, 3, 0));
    chunk(&mut out, b"strf", &strf_video());
    end_sized(&mut out, sp);
    end_sized(&mut out, hp);
    // movi with 3 chunks
    out.extend_from_slice(b"LIST");
    let mp = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(b"movi");
    let movi_form_pos = out.len() as u64 - 4; // position of 'movi' fourcc
    let mut positions = Vec::new();
    for i in 0..3u8 {
        positions.push(chunk(&mut out, b"00dc", &[i, 1, 2, 3, 4, 5, 6, 7]) as u64);
    }
    end_sized(&mut out, mp);
    // idx1: `entries` valid entries, all pointing at the first chunk
    let mut idx: Vec<u8> = Vec::with_capacity(idx1_entries * 16);
    for _ in 0..idx1_entries {
        idx.extend_from_slice(b"00dc");
        idx.extend_from_slice(&0x10u32.to_le_bytes());
        let rel = ((positions[0] - movi_form_pos) as u32).to_le_bytes();
        idx.extend_from_slice(&rel);
        idx.extend_from_slice(&8u32.to_le_bytes());
    }
    chunk(&mut out, b"idx1", &idx);
    end_sized(&mut out, rp);
    out
}
fn main() {
    for (name, n) in [
        ("control (524288 = cap)", 524_288usize),
        ("over-cap (524289 = cap+1)", 524_289usize),
    ] {
        let bytes = build_avi(n);
        println!(
            "{}: file {} bytes, idx1 payload {} bytes",
            name,
            bytes.len(),
            n * 16
        );
        let stream = demux::avi::test_support::mem_stream(bytes).unwrap();
        let mut d = match AviDemuxer::new(stream) {
            Ok(d) => d,
            Err(e) => {
                println!("  construct err: {}", e.context.code);
                continue;
            }
        };
        match d.start() {
            Ok(tracks) => println!("  start OK: {} tracks", tracks.len()),
            Err(e) => println!("  start FAILED: [{}] {} (stage {:?})", e.context.code, e, e.stage),
        }
    }
}
