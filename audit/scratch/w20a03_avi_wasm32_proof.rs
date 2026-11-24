// W20-A03 re-verification proof 1: AVI probe walk_hdrl 32-bit panic.
// Verbatim loop logic from crates/media-probe/src/signature/avi.rs:99-130
// (walk_hdrl), emulated with usize=u32 (wasm32 pointer width) plus the
// identical native 64-bit version. Hostile input: RIFF/AVI /LIST hdrl /
// "JUNK" chunk declaring size 0xFFFFFFFF (bytes 24..32 of the prefix).
fn u32_le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

const MAX_WALK_STEPS: usize = 256;

fn hostile_prefix() -> Vec<u8> {
    let mut p = vec![0u8; 4096];
    p[0..4].copy_from_slice(b"RIFF");
    p[8..12].copy_from_slice(b"AVI ");
    p[12..16].copy_from_slice(b"LIST");
    p[20..24].copy_from_slice(b"hdrl");
    p[24..28].copy_from_slice(b"JUNK");
    p[28..32].copy_from_slice(&u32::MAX.to_le_bytes());
    p
}

// Native (usize = 64-bit): the verbatim walk_hdrl body.
fn walk_hdrl_native(prefix: &[u8], start: usize, end: usize) -> usize {
    let mut pos = start;
    let mut steps = 0;
    while pos + 8 <= end && steps < MAX_WALK_STEPS {
        steps += 1;
        let fourcc = &prefix[pos..pos + 4];
        let size = u32_le(prefix, pos + 4) as usize;
        let payload_start = pos + 8;
        let payload_end = payload_start.saturating_add(size).min(end);
        let _ = (fourcc, payload_start, payload_end);
        pos = pos
            .saturating_add(size)
            .saturating_add(8)
            .saturating_add(size & 1);
    }
    pos.min(end)
}

// wasm32 (usize = u32, release semantics: arith wraps, slices always check):
// same body with u32 arithmetic. `pos + 8` in the while guard wraps to 7,
// the guard passes, then `&prefix[pos..pos+4]` = &prefix[0xFFFF_FFFF..3].
fn walk_hdrl_wasm32(prefix: &[u8], start: usize, end: usize) -> u32 {
    let mut pos: u32 = start as u32;
    let mut steps = 0usize;
    while pos.wrapping_add(8) <= end as u32 && steps < MAX_WALK_STEPS {
        steps += 1;
        let fourcc = &prefix[pos as usize..(pos.wrapping_add(4)) as usize];
        let size = u32_le(prefix, pos as usize + 4);
        let payload_start = pos.wrapping_add(8);
        let payload_end = payload_start.saturating_add(size).min(end as u32);
        let _ = (fourcc, payload_start, payload_end);
        pos = pos
            .saturating_add(size)
            .saturating_add(8)
            .saturating_add(size & 1);
    }
    pos
}

fn main() {
    let prefix = hostile_prefix();
    let native_pos = walk_hdrl_native(&prefix, 24, prefix.len());
    println!(
        "native(64-bit usize): loop terminated, returned pos={} — NO panic",
        native_pos
    );
    std::panic::set_hook(Box::new(|info| {
        eprintln!("PANIC: {}", info);
    }));
    let result = std::panic::catch_unwind(|| walk_hdrl_wasm32(&prefix, 24, prefix.len()));
    println!("wasm32(u32 usize): {:?}", result.map_err(|_| "panicked"));
}
