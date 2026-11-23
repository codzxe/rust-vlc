// W05-A06 probe: native simulation of mkv/mod.rs:732 on a 32-bit usize
// target (wasm32-unknown-unknown — the apps/web lane). Reproduces the
// exact expression `let cend = cstart + csize as usize;` with u32
// standing in for usize (wasm32's usize width) and release-mode
// wrapping arithmetic, then the payload slice.
fn main() {
    let payload: Vec<u8> = vec![0u8; 64]; // a BlockGroup payload
    let at: u32 = 0; // walk cursor
    let chdr_len: u32 = 2; // child header length (id 0xA1 + 1-byte size)
    let cstart: u32 = at + chdr_len;
    // A 5-byte size vint encodes up to 2^35-1; 0xFFFF_FFFF is encodable
    // (e.g. bytes 0x07 FF FF FF FF  -> value 0x7F_FFFF_FFFF? no: 0x07 is a
    // 6-byte head; the 5-byte head 0x0F gives 28 value bits; a 6-byte head
    // 0x03 gives 35 bits — both can carry 0xFFFF_FFFF).
    let csize: u64 = 0xFFFF_FFFF;
    // mod.rs:732 verbatim semantics on wasm32:
    let csize_as_usize: u32 = csize as u32; // truncating cast
    let cend: u32 = cstart.wrapping_add(csize_as_usize); // release add
    println!(
        "cstart={} csize={:#x} csize_as_32={:#x} cend={} payload.len()={}",
        cstart,
        csize,
        csize_as_usize,
        cend,
        payload.len()
    );
    // mod.rs:733-737: the bound check passes (cend <= payload.len()),
    // then the slice panics (start > end).
    if cend > payload.len() as u32 {
        println!("bound check rejects -> typed truncated (native 64-bit path)");
        return;
    }
    let child = &payload[cstart as usize..cend as usize];
    println!("slice ok, len {}", child.len());
}
