// W09-A01 probe 2: (a) do the dvb_vobsub_fuzz_survival.rs clean seeds actually
// compose regions (or is the region-stride invariant vacuous)? (b) does an
// input with >MAX_DAMAGE_EVENTS damage contradict the test's
// `damage.len() <= MAX_DAMAGE_EVENTS` invariant?
use subtitles::dvb::{DvbDecoder, MAX_DAMAGE_EVENTS};

fn seg(kind: u8, page: u16, data: &[u8]) -> Vec<u8> {
    let mut v = vec![0x0F, kind];
    v.extend_from_slice(&page.to_be_bytes());
    v.extend_from_slice(&(data.len() as u16).to_be_bytes());
    v.extend_from_slice(data);
    v
}

fn page_seg(timeout: u8, version: u8, state: u8, entries: &[(u8, u16, u16)]) -> Vec<u8> {
    let mut d = vec![timeout, (version << 4) | (state << 2)];
    for &(id, x, y) in entries {
        d.push(id);
        d.push(0);
        d.extend_from_slice(&x.to_be_bytes());
        d.extend_from_slice(&y.to_be_bytes());
    }
    seg(0x10, 1, &d)
}

fn region_seg(
    id: u8, w: u16, h: u16, depth_code: u8, clut: u8, fill: bool, fill_color: u8,
    objects: &[(u16, u16, u16)],
) -> Vec<u8> {
    let mut d = vec![
        id, (2 << 4) | (u8::from(fill) << 3), (w >> 8) as u8, w as u8,
        (h >> 8) as u8, h as u8, 0b0010_0000 | (depth_code << 2), clut, 0, fill_color,
    ];
    for &(oid, x, y) in objects {
        d.extend_from_slice(&oid.to_be_bytes());
        d.push((x >> 8) as u8);
        d.push(x as u8);
        d.push((y >> 8) as u8);
        d.push(y as u8);
    }
    seg(0x11, 1, &d)
}

fn clut_seg(id: u8, version: u8, entries: &[(u8, u8, u8, u8, u8)]) -> Vec<u8> {
    let mut d = vec![id, version << 4];
    for &(eid, y, cb, cr, alpha) in entries {
        d.extend_from_slice(&[eid, 0b1110_0001, y, cr, cb, alpha]);
    }
    seg(0x12, 1, &d)
}

fn object_seg(id: u16, top: &[u8], bottom: &[u8]) -> Vec<u8> {
    let mut d = id.to_be_bytes().to_vec();
    d.push(0);
    d.extend_from_slice(&(top.len() as u16).to_be_bytes());
    d.extend_from_slice(&(bottom.len() as u16).to_be_bytes());
    d.extend_from_slice(top);
    d.extend_from_slice(bottom);
    seg(0x13, 1, &d)
}

fn dvb_seed_2bit() -> Vec<u8> {
    let mut top = Vec::new();
    for v in 0u8..6 {
        top.push(0x10);
        top.push(0b0010_0000 | ((v % 5) << 2) | ((v % 3) + 1));
        top.push(0x00);
        top.push(0xF0);
    }
    let bottom = top.clone();
    let mut p = Vec::new();
    p.extend_from_slice(&page_seg(6, 1, 1, &[(3, 40, 60)]));
    p.extend_from_slice(&region_seg(3, 24, 12, 1, 7, true, 1, &[(9, 2, 1)]));
    p.extend_from_slice(&clut_seg(7, 0, &[(1, 235, 128, 128, 0), (2, 82, 90, 240, 40), (3, 41, 240, 110, 255)]));
    p.extend_from_slice(&object_seg(9, &top, &bottom));
    p.extend_from_slice(&seg(0x80, 1, &[]));
    p
}

fn dvb_seed_8bit() -> Vec<u8> {
    let mut top = vec![0x22];
    top.extend_from_slice(&[0x11u8; 16]);
    top.push(0x12);
    top.extend_from_slice(&[0x05, 0x00, 0x83, 0x07, 0x00, 0x00]);
    top.push(0xF0);
    let mut p = Vec::new();
    p.extend_from_slice(&page_seg(9, 1, 1, &[(1, 100, 200)]));
    p.extend_from_slice(&region_seg(1, 8, 2, 3, 5, false, 0, &[(2, 0, 0)]));
    p.extend_from_slice(&clut_seg(5, 0, &[(7, 200, 128, 128, 0)]));
    p.extend_from_slice(&object_seg(2, &top, &[]));
    p.extend_from_slice(&seg(0x14, 1, &[0x15, 0x02, 0xCF, 0x02, 0x3F]));
    p.extend_from_slice(&seg(0x80, 1, &[]));
    p
}

fn dvb_seed_stream() -> Vec<u8> {
    let mut s = dvb_seed_2bit();
    s.extend_from_slice(&page_seg(6, 2, 0, &[(3, 40, 60)]));
    s.extend_from_slice(&object_seg(9, &[0x10, 0b01_01_00_00, 0xF0], &[]));
    s.extend_from_slice(&seg(0x80, 1, &[]));
    s
}

fn report(name: &str, payload: &[u8]) {
    let mut dec = DvbDecoder::new();
    let u = dec.decode_pes(payload, Some(90_000));
    let strides_ok = u
        .regions
        .iter()
        .all(|r| r.rgba.len() == r.width as usize * r.height as usize * 4);
    println!(
        "{name}: regions={} damage={} first_damage={:?} stride_ok={}",
        u.regions.len(),
        u.damage.len(),
        u.damage.first().map(|d| format!("{d:?}")),
        strides_ok
    );
}

fn main() {
    report("seed0-2bit", &dvb_seed_2bit());
    report("seed1-8bit", &dvb_seed_8bit());
    report("seed2-stream", &dvb_seed_stream());

    // >MAX unknown segments -> damage overflow path.
    let mut flood = Vec::new();
    for _ in 0..(MAX_DAMAGE_EVENTS + 6) {
        flood.extend_from_slice(&seg(0x77, 1, &[])); // unknown type byte
    }
    let mut dec = DvbDecoder::new();
    let u = dec.decode_pes(&flood, Some(0));
    let overflow_present = u
        .damage
        .iter()
        .any(|d| matches!(d, subtitles::dvb::DvbDamage::DamageOverflow { .. }));
    println!(
        "flood: damage.len()={} MAX={} overflow_marker_present={} (test asserts len<=MAX)",
        u.damage.len(),
        MAX_DAMAGE_EVENTS,
        overflow_present
    );
}
