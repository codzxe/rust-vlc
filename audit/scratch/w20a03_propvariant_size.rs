// W20-A03 re-verification proof 2: PropVariant size on x86_64.
// Verbatim type definitions from crates/platform-windows/src/windows/com.rs
// (:237-256). The real Win32/x64 PROPVARIANT is 24 bytes (8-byte vt/reserved
// header + 16-byte payload union, whose members include CAUB {ULONG cElements;
// UBYTE* pElements} and DECIMAL — both 16 bytes on x64).
use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub union PropVariant {
    pub header: PropVariantHeader,
    pub raw: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PropVariantHeader {
    pub vt: u16,
    pub reserved: [u16; 3],
    pub pointer: *mut u16,
}

fn main() {
    println!("size_of::<PropVariant>() = {}", size_of::<PropVariant>());
    println!("align_of::<PropVariant>() = {}", align_of::<PropVariant>());
    // The real ABI member shapes on x64 (counted-array member, 16 bytes):
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Caub {
        c_elements: u32,
        p_elements: *mut c_void,
    }
    println!("size of a CAUB-style member on x64 = {}", size_of::<Caub>());
    // header(8: vt+3 reserved) + union(16) = the documented 24-byte
    // PROPVARIANT; the crate's 16-byte pin leaves 8 bytes of every
    // GetValue/PropVariantClear write outside the Rust object.
    assert_eq!(size_of::<PropVariant>(), 16, "crate pins 16 bytes");
    println!("crate size 16 vs x64 ABI 24 => 8-byte under-write domain");
}
