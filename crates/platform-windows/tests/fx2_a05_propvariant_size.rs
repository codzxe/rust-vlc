//! FX2-A05 regression — the `PROPVARIANT` FFI size (W08-A10 F-001,
//! re-verified executed by W20-A03's repro: this repo's layout
//! arithmetic produced `size_of::<PropVariant>() = 16`). The real x64
//! ABI is 24 bytes — an 8-byte vt/reserved header plus a 16-byte
//! payload union (whose counted-array members, e.g. `CAUB`/`BLOB`,
//! are 16 bytes) — so the 16-byte pin left 8 bytes of every
//! `IPropertyStore::GetValue`/`PropVariantClear` write outside the
//! Rust stack object (the stack under-write class the audit flagged).
//!
//! `PropVariant` lives in the crate's `#[cfg(target_os = "windows")]`
//! module tree, so this suite carries the same cfg (plus the x64
//! pointer-width gate the size claim is scoped to): on non-Windows
//! hosts it builds as an empty test target — zero tests, compiled
//! green, the wrong-OS seam behavior stays covered by
//! `seam_behaviour.rs` — and the assertions execute on the Windows
//! x64 lanes only.

#![cfg(all(target_os = "windows", target_pointer_width = "64"))]

use platform_windows::windows::com::{PropVariant, PropVariantHeader};

/// The x64 ABI contract, exactly as the audit executed it: 24 bytes,
/// 8-byte alignment (W20-A03's repro printed `align_of = 8`).
#[test]
fn fx2_a05_propvariant_matches_the_x64_abi_size_and_alignment() {
    assert_eq!(
        std::mem::size_of::<PropVariant>(),
        24,
        "the x64 PROPVARIANT ABI is 24 bytes (8-byte header + 16-byte payload union)"
    );
    assert_eq!(
        std::mem::align_of::<PropVariant>(),
        8,
        "the x64 PROPVARIANT ABI is 8-byte aligned (the payload members are pointer-wide)"
    );
}

/// The field order W08-A10 recorded for the leading bytes: `vt` at 0,
/// the three reserved words filling 2..8, the payload slot at 8
/// (where the `VT_LPWSTR` string pointer lives) — 16 header bytes.
#[test]
fn fx2_a05_propvariant_header_keeps_the_recorded_field_order() {
    assert_eq!(std::mem::offset_of!(PropVariantHeader, vt), 0);
    assert_eq!(std::mem::offset_of!(PropVariantHeader, reserved), 2);
    assert_eq!(std::mem::offset_of!(PropVariantHeader, pointer), 8);
    assert_eq!(std::mem::size_of::<PropVariantHeader>(), 16);
}
