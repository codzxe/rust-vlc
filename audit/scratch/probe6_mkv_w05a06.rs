// W05-A06 probe: (1) adjudicate W04-A06's S1 lacing inversion on the real
// MkvDemuxer; (2) window-slide livelock for a block spanning >1 window;
// (3) block-before-Timecode damage accounting; (4) BlockGroup keyframe flag.
use demux::mkv::MkvDemuxer;
use plugin_api::{DemuxEvent, Demuxer};

fn el(out: &mut Vec<u8>, idv: &[u8], payload: &[u8]) {
    out.extend_from_slice(idv);
    let ln = payload.len();
    if ln < 0x80 {
        out.push(0x80 | ln as u8);
    } else if ln < 0x4000 {
        out.push(0x40 | (ln >> 8) as u8);
        out.push((ln & 0xFF) as u8);
    } else if ln < 0x20_0000 {
        out.push(0x20 | (ln >> 16) as u8);
        out.push(((ln >> 8) & 0xFF) as u8);
        out.push((ln & 0xFF) as u8);
    } else {
        // 4-byte vint (value bits 28 — enough for the 32 MiB block gate).
        out.push(0x10 | (ln >> 24) as u8);
        out.push(((ln >> 16) & 0xFF) as u8);
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

/// Build a one-track mkv whose single cluster carries `blocks` (already
/// encoded as raw child bytes: header+payload pairs appended verbatim).
fn mk_file_with_blocks(blocks: &[u8]) -> Vec<u8> {
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
    // Cluster: Timecode first, then caller blocks.
    let mut cluster = Vec::new();
    el(&mut cluster, &[0xE7], &uint_payload(0));
    cluster.extend_from_slice(blocks);
    el(&mut seg, &[0x1F, 0x43, 0xB6, 0x75], &cluster);
    el(&mut out, &[0x18, 0x53, 0x80, 0x67], &seg);
    out
}

fn simple_block(count_byte: u8, lacing_bits: u8, body: &[u8]) -> Vec<u8> {
    let mut block = vec![0x81, 0x00, 0x00, 0x80 | lacing_bits];
    block.push(count_byte);
    block.extend_from_slice(body);
    let mut out = Vec::new();
    el(&mut out, &[0xA3], &block);
    out
}

fn run(label: &str, bytes: Vec<u8>) {
    let mut d = MkvDemuxer::new(demux::avi::test_support::mem_stream(bytes).unwrap()).unwrap();
    match d.start() {
        Ok(t) => {
            let mut packets = 0usize;
            let mut sizes: Vec<usize> = Vec::new();
            let mut keyflags: Vec<bool> = Vec::new();
            let mut recovered = 0u64;
            loop {
                match d.next_event().unwrap() {
                    DemuxEvent::Packet(p) => {
                        packets += 1;
                        sizes.push(p.data.len());
                        keyflags.push(p.keyframe);
                    }
                    DemuxEvent::Recovered { .. } => recovered += 1,
                    DemuxEvent::EndOfStream => break,
                    _ => {}
                }
            }
            println!(
                "{label}: tracks={} packets={} sizes={:?} key={:?} recovered={} damage={}",
                t.len(),
                packets,
                sizes,
                keyflags,
                recovered,
                d.damage_bytes()
            );
        }
        Err(e) => println!("{label}: start FAILED [{}] {}", e.context.code, e),
    }
}

fn main() {
    // ---- (1) Lacing adjudication (W04-A06 S1) ----------------------
    // Fixed, ffmpeg-legal: count byte 2 = THREE frames of 2 bytes.
    run(
        "fixed count=2 (ffmpeg-legal: 3 frames)",
        mk_file_with_blocks(&simple_block(2, 0x04, &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66])),
    );
    // Fixed, repo convention: count byte 3.
    run(
        "fixed count=3 (repo convention)",
        mk_file_with_blocks(&simple_block(3, 0x04, &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66])),
    );
    // Xiph, ffmpeg-legal: count 2 = 3 frames (4, 5, remainder 6).
    let mut xiph = vec![4u8, 5];
    xiph.extend_from_slice(&[0xA1; 4]);
    xiph.extend_from_slice(&[0xB1; 5]);
    xiph.extend_from_slice(&[0xC1; 6]);
    run("xiph count=2 (ffmpeg-legal: 3 frames)", mk_file_with_blocks(&simple_block(2, 0x02, &xiph)));
    // EBML, ffmpeg-legal: count 2 = 3 frames; first size vint 0x82 (value 2
    // -> size 3), equal-delta 0 -> 0xBF (bias 63). Frames 3,3, remainder.
    let mut ebml = vec![0x82u8, 0xBF];
    ebml.extend_from_slice(&[0xAA; 3]);
    ebml.extend_from_slice(&[0xBB; 3]);
    ebml.extend_from_slice(&[0xCC; 4]);
    run("ebml count=2 (ffmpeg-legal: 3 frames)", mk_file_with_blocks(&simple_block(2, 0x06, &ebml)));
    // EBML, repo convention: count 3.
    run("ebml count=3 (repo convention)", mk_file_with_blocks(&simple_block(3, 0x06, &ebml)));

    // ---- (2) BlockGroup keyframe flag ------------------------------
    // BlockGroup 0xA0 { Block 0xA1 { track 1, rel 0, flags 0x80(no lacing),
    // payload } } — no ReferenceBlock => keyframe per the module's claim.
    let mut blk = vec![0x81, 0x00, 0x00, 0x00];
    blk.extend_from_slice(b"payload");
    let mut group = Vec::new();
    el(&mut group, &[0xA1], &blk);
    let mut gb = Vec::new();
    el(&mut gb, &[0xA0], &group);
    run("blockgroup no-refblock (keyframe expected)", mk_file_with_blocks(&gb));
    // SimpleBlock twin with the keyframe bit set, for contrast.
    let mut sb = vec![0x81, 0x00, 0x00, 0x80];
    sb.extend_from_slice(b"payload");
    let mut sbel = Vec::new();
    el(&mut sbel, &[0xA3], &sb);
    run("simpleblock keyframe bit set", mk_file_with_blocks(&sbel));

    // ---- (3) Block before Timecode: damage accounting ---------------
    // Cluster children: [SimpleBlock(keyframe,2B)] [Timecode] [SimpleBlock].
    let mut b1 = vec![0x81, 0x00, 0x00, 0x80, 0x11, 0x22];
    let mut out1 = Vec::new();
    el(&mut out1, &[0xA3], &b1);
    let mut b2 = vec![0x81, 0x00, 0x01, 0x80, 0x33, 0x44];
    let mut out2 = Vec::new();
    el(&mut out2, &[0xA3], &b2);
    // Timecode element bytes, then both blocks around it.
    let mut tc = Vec::new();
    el(&mut tc, &[0xE7], &uint_payload(0));
    let mut blocks = out1.clone();
    blocks.extend_from_slice(&tc);
    blocks.extend_from_slice(&out2);
    // Cluster payload = block, timecode, block — build by hand: mk_file
    // prepends its own Timecode, so instead build a cluster with children
    // [SimpleBlock][Timecode][SimpleBlock] directly.
    let mut cluster = Vec::new();
    cluster.extend_from_slice(&blocks);
    let mut file = mk_file_with_blocks(&cluster);
    // mk_file already put a Timecode first; the extra mid-cluster Timecode
    // is skipped as an unknown element. The first block still precedes the
    // FIRST Timecode only if we reorder; simplest: rebuild with blocks
    // where the leading element is the SimpleBlock and mk_file's Timecode
    // follows it. Do that by constructing the file manually:
    let _ = &mut file;
    let mut manual = Vec::new();
    let mut eh = Vec::new();
    el(&mut eh, &[0x42, 0x86], &[1]);
    el(&mut eh, &[0x42, 0xF7], &[1]);
    el(&mut eh, &[0x42, 0xF2], &[4]);
    el(&mut eh, &[0x42, 0xF3], &[8]);
    el(&mut eh, &[0x42, 0x82], b"matroska");
    el(&mut eh, &[0x42, 0x87], &[2]);
    el(&mut eh, &[0x42, 0x85], &[2]);
    el(&mut manual, &[0x1A, 0x45, 0xDF, 0xA3], &eh);
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
    // Cluster children in VIOLATING order: SimpleBlock, Timecode, SimpleBlock.
    let mut cluster2 = Vec::new();
    cluster2.extend_from_slice(&out1);
    cluster2.extend_from_slice(&tc);
    cluster2.extend_from_slice(&out2);
    el(&mut seg, &[0x1F, 0x43, 0xB6, 0x75], &cluster2);
    el(&mut manual, &[0x18, 0x53, 0x80, 0x67], &seg);
    run("block-before-timecode (damage counted?)", manual);

    // ---- (4) Window livelock: one block spanning >1 window ----------
    // A 5 MiB SimpleBlock inside a cluster that declares room for it:
    // gate accepts (<= 32 MiB) but the window slide re-reads the same
    // 4 MiB window forever. Control: 3 MiB block parses fine.
    if std::env::args().nth(1).as_deref() == Some("livelock") {
        let mut body = vec![0x81, 0x00, 0x00, 0x00];
        body.extend(std::iter::repeat(0x5Au8).take(5 * 1024 * 1024));
        let mut blk_el = Vec::new();
        el(&mut blk_el, &[0xA3], &body);
        run("5MiB block (expect HANG)", mk_file_with_blocks(&blk_el));
    }
    if std::env::args().nth(1).as_deref() == Some("control") {
        let mut body = vec![0x81, 0x00, 0x00, 0x00];
        body.extend(std::iter::repeat(0x5Au8).take(3 * 1024 * 1024));
        let mut blk_el = Vec::new();
        el(&mut blk_el, &[0xA3], &body);
        run("3MiB block (control)", mk_file_with_blocks(&blk_el));
    }
}
