//! 6.VTA-ruopus differential harness: OUR ogg demuxer -> ruopus decode -> raw f32le.
//! Usage: vta-ruopus <file.ogg> [--out out.pcm] [--trim-per-ch N] [--hostile]
use plugin_api::Demuxer;
use ruopus::{MultistreamDecoder, OpusDecoder};

enum Dec { Mono(OpusDecoder), Multi(MultistreamDecoder) }
impl Dec {
    fn decode(&mut self, data: &[u8]) -> Result<Vec<f32>, String> {
        match self {
            Dec::Mono(d) => d.decode_packet(data).map_err(|e| format!("{e:?}")),
            Dec::Multi(d) => d.decode_packet(data).map_err(|e| format!("{e:?}")),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args[1].clone();
    let mut out_path: Option<String> = None;
    let mut trim_per_ch: Option<usize> = None;
    let mut hostile = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => { out_path = Some(args[i + 1].clone()); i += 2; }
            "--trim-per-ch" => { trim_per_ch = Some(args[i + 1].parse().unwrap()); i += 2; }
            "--hostile" => { hostile = true; i += 1; }
            _ => { eprintln!("unknown arg {}", args[i]); std::process::exit(2); }
        }
    }
    let data = std::fs::read(&path).expect("read input");
    let stream = demux::avi::test_support::mem_stream(data).expect("mem stream");
    let mut ogg = demux::ogg::OggDemuxer::new(stream).expect("ogg open");
    // Pump the WHOLE file first: the demuxer harvests header packets
    // (OpusHead/OpusTags) into per-serial storage during pumping, and only
    // then are they queryable via header_packets(). Data packets are
    // buffered per track until the opus serial is known.
    let started = ogg.start().expect("ogg start");
    eprintln!("DEBUG started tracks={}", started.len());
    let mut all: Vec<_> = Vec::new();
    loop {
        match ogg.next_event() {
            Ok(plugin_api::DemuxEvent::Packet(p)) => all.push((p.track, p.data.to_vec())),
            Ok(plugin_api::DemuxEvent::EndOfStream) => break,
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    eprintln!("DEBUG serials={:?} tracks={:?}", ogg.serial_numbers(), ogg.tracks().len());
    for &s in &ogg.serial_numbers() { eprintln!("DEBUG serial={:#x} headers={}", s, ogg.header_packets(s).len()); }
    let mut opus_serial: Option<u32> = None;
    for &s in &ogg.serial_numbers() {
        if let Some(h) = ogg.header_packets(s).first() {
            if h.len() >= 19 && &h[..8] == b"OpusHead" { opus_serial = Some(s); break; }
        }
    }
    let serial = match opus_serial {
        Some(s) => s,
        None => { eprintln!("NO OpusHead found"); std::process::exit(3); }
    };
    let head = ogg.header_packets(serial)[0].clone();
    let channels = head[9] as usize;
    let preskip = u16::from_le_bytes([head[10], head[11]]) as usize;
    let gain_q7_8 = i16::from_le_bytes([head[14], head[15]]);
    let mapping_family = head[18];
    let track_id = ogg.serial_link(serial)
        .and_then(|idx| ogg.tracks().get(idx).map(|t| t.id))
        .expect("opus track");
    eprintln!("OpusHead: channels={} preskip={} gain={}q7.8 mapping_family={}", channels, preskip, gain_q7_8, mapping_family);

    let dec = if channels <= 2 && mapping_family == 0 {
        Dec::Mono(OpusDecoder::new(channels))
    } else {
        // RFC 7845 multistream head: [18]=family [19]=streams [20]=coupled [21..]=mapping
        let streams = head[19] as usize;
        let coupled = head[20] as usize;
        let mapping = head[21..].to_vec();
        Dec::Multi(MultistreamDecoder::with_rate(48_000, streams, coupled, &mapping))
    };
    let mut dec = dec;
    let mut pcm: Vec<f32> = Vec::new();
    let mut packets = 0u64;
    let mut decode_errors = 0u64;
    for (track, bytes) in &all {
        if *track != track_id { continue; }
        packets += 1;
        match dec.decode(bytes) {
            Ok(v) => pcm.extend_from_slice(&v),
            Err(e) => { decode_errors += 1; eprintln!("packet {}: decode error: {}", packets, e); }
        }
    }
    // Pre-skip: discard preskip*channels leading decoded samples (RFC 7845 5.1).
    let skip = preskip * channels;
    let after_pre = if pcm.len() > skip { &pcm[skip..] } else { &pcm[..] };
    // End-trim to the reference sample count when provided.
    let final_len = trim_per_ch.map(|n| (n * channels).min(after_pre.len())).unwrap_or(after_pre.len());
    let out = &after_pre[..final_len];
    eprintln!(
        "packets={} decode_errors={} decoded_total_per_ch={} after_preskip_per_ch={} final_per_ch={}",
        packets, decode_errors, pcm.len() / channels, after_pre.len() / channels, final_len / channels
    );
    if let Some(p) = out_path {
        let bytes: Vec<u8> = out.iter().flat_map(|f| f.to_le_bytes()).collect();
        std::fs::write(p, bytes).expect("write pcm");
    } else {
        use std::io::Write;
        let mut o = std::io::stdout().lock();
        for f in out { o.write_all(&f.to_le_bytes()).unwrap(); }
    }
    if hostile { hostile_suite(&mut dec, channels); }
}

/// Hostile-input contract check: truncated / garbage / zero-length / huge
/// packets must not panic (decode returns Result; we still wrap in
/// catch_unwind to catch any assert-path).
fn hostile_suite(dec: &mut Dec, channels: usize) {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("truncated", vec![0xFB, 0xFF, 0xE0]),
        ("garbage", vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]),
        ("zero_length", vec![]),
        ("one_byte_toc", vec![0x38]),
        ("huge", vec![0xFF; 4096]),
        ("huge_zeros", vec![0x00; 4096]),
    ];
    let mut failures = 0;
    for (name, data) in cases {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| dec.decode(&data)));
        match r {
            Ok(Ok(v)) => println!("hostile {:<14}: OK ({} samples, concealment/DTX path)", name, v.len() / channels.max(1)),
            Ok(Err(e)) => println!("hostile {:<14}: Err({})", name, &e[..e.len().min(100)]),
            Err(_) => { failures += 1; println!("hostile {:<14}: PANIC", name); }
        }
    }
    if failures > 0 { std::process::exit(4); }
}
