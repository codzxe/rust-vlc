// Standalone: feed ffmpeg-LEGAL laced blocks (count byte = frames-1)
// to MkvDemuxer and count the frames it emits.
use demux::mkv::MkvDemuxer;
use plugin_api::{DemuxEvent, Demuxer};

fn el(out: &mut Vec<u8>, idv: &[u8], payload: &[u8]) {
    out.extend_from_slice(idv);
    let ln = payload.len();
    if ln < 0x7F {
        out.push(0x80 | ln as u8);
    } else if ln < 0x3FFF {
        out.push(0x40 | (ln >> 8) as u8);
        out.push((ln & 0xFF) as u8);
    } else {
        out.push(0x20 | (ln >> 16) as u8);
        out.push(((ln >> 8) & 0xFF) as u8);
        out.push((ln & 0xFF) as u8);
    }
    out.extend_from_slice(payload);
}

fn uint_payload(v: u64) -> Vec<u8> {
    let b = v.to_be_bytes();
    let mut i = 0;
    while i < 7 && b[i] == 0 {
        i += 1;
    }
    b[i..].to_vec()
}

fn mk_file(count_byte: u8, lacing_bits: u8, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut eh = Vec::new();
    el(&mut eh, &[0x42, 0x86], &[1]);
    el(&mut eh, &[0x42, 0xF7], &[1]);
    el(&mut eh, &[0x42, 0xF2], &[4]);
    el(&mut eh, &[0x42, 0xF3], &[8]);
    el(&mut eh, &[0x42, 0x82], b"matroska");
    el(&mut eh, &[0x42, 0x87], &[2]);
    el(&mut eh, &[0x42, 0x85], &[2]);
    el(&mut out, &[0x1A, 0x45, 0xDF, 0xA3], &eh);
    let mut seg = Vec::new();
    let mut info = Vec::new();
    el(&mut info, &[0x2A, 0xD7, 0xB1], &uint_payload(1_000_000));
    el(&mut seg, &[0x15, 0x49, 0xA9, 0x66], &info);
    let mut entry = Vec::new();
    el(&mut entry, &[0xD7], &uint_payload(1));
    el(&mut entry, &[0x83], &uint_payload(2));
    el(&mut entry, &[0x86], b"A_OPUS");
    el(&mut entry, &[0x9C], &uint_payload(1));
    let mut tracks = Vec::new();
    el(&mut tracks, &[0xAE], &entry);
    el(&mut seg, &[0x16, 0x54, 0xAE, 0x6B], &tracks);
    let mut cluster = Vec::new();
    el(&mut cluster, &[0xE7], &uint_payload(0));
    let mut block = vec![0x81, 0x00, 0x00, 0x80 | lacing_bits];
    block.push(count_byte);
    block.extend_from_slice(body);
    el(&mut cluster, &[0xA3], &block);
    el(&mut seg, &[0x1F, 0x43, 0xB6, 0x75], &cluster);
    el(&mut out, &[0x18, 0x53, 0x80, 0x67], &seg);
    out
}

fn run(label: &str, bytes: Vec<u8>) {
    let mut d = MkvDemuxer::new(demux::avi::test_support::mem_stream(bytes).unwrap()).unwrap();
    match d.start() {
        Ok(t) => {
            let mut packets = 0usize;
            let mut sizes: Vec<usize> = Vec::new();
            let mut damage = 0u64;
            loop {
                match d.next_event().unwrap() {
                    DemuxEvent::Packet(p) => {
                        packets += 1;
                        sizes.push(p.data.len());
                    }
                    DemuxEvent::EndOfStream => break,
                    _ => {}
                }
            }
            damage = d.damage_bytes();
            println!(
                "{label}: tracks={} packets={} sizes={:?} damage={}",
                t.len(),
                packets,
                sizes,
                damage
            );
        }
        Err(e) => println!("{label}: start FAILED [{}] {}", e.context.code, e),
    }
}

fn main() {
    // FFmpeg-legal fixed lace: count=2 means THREE frames of 2 bytes.
    let fixed_legal = mk_file(2, 0x04, &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    run("fixed count=2 (ffmpeg-legal: 3 frames)", fixed_legal);
    // FFmpeg-legal xiph lace: count=2 means THREE frames: 4, 5, remainder 6.
    let mut xiph = vec![4u8, 5];
    xiph.extend_from_slice(&[0xA1, 0xA2, 0xA3, 0xA4]);
    xiph.extend_from_slice(&[0xB1, 0xB2, 0xB3, 0xB4, 0xB5]);
    xiph.extend_from_slice(&[0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6]);
    let xiph_legal = mk_file(2, 0x02, &xiph);
    run("xiph count=2 (ffmpeg-legal: 3 frames)", xiph_legal);
    // The repo's own convention (count=3): what the in-repo fixture uses.
    let repo_style = mk_file(3, 0x04, &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    run("fixed count=3 (repo convention)", repo_style);
}
