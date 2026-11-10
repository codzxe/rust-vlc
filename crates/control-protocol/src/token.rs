//! Session-token utilities — card 7.J1 (doc 05 §5.4).
//!
//! The server generates a 256-bit token per start; the client presents
//! it in `hello`; comparison is constant-time. Token material NEVER
//! enters error messages or logs (V8 redaction — the only place it is
//! ever printed is the controlling terminal the operator owns, by the
//! host app's explicit `--print-token`-style decision).

use std::time::{SystemTime, UNIX_EPOCH};

/// Compare two byte strings in constant time w.r.t. their CONTENT.
///
/// Length is treated as public (the token's base64 length is visible
/// on the wire regardless); when lengths match, every byte is compared
/// and accumulated before the decision — no early exit on the first
/// difference.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// A generated token (base64 of 32 random bytes — 256 bits).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token(pub String);

/// Generate a 256-bit token as base64 text.
///
/// Entropy source, in order:
/// 1. `/dev/urandom` (the OS CSPRNG) — the primary, Unix-only path.
/// 2. A fallback mix of wall-clock nanos, the process id and a
///    `RandomState`-derived hash — **weak** `[VERIFY]`: resistant to
///    trivial guessing but not to a determined local attacker; callers
///    MUST log a warning when this path is used. The Windows entropy
///    path joins with the platform adapter crates (doc 01 §2.7, Phase 7
///    item 1) — recorded gap, not a silent assumption.
///
/// The returned `EntropySource` says which path won (log it, never the
/// token).
pub fn generate_token() -> (Token, EntropySource) {
    match read_urandom(32) {
        Ok(bytes) => (
            Token(crate::base64::encode(&bytes)),
            EntropySource::OsRandom,
        ),
        Err(_) => (Token(weak_fallback_token()), EntropySource::WeakFallback),
    }
}

/// Where a token's entropy came from (logged; never the token itself).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntropySource {
    /// `/dev/urandom` — the OS CSPRNG.
    OsRandom,
    /// The time/pid/hash mix — weak, log a warning.
    WeakFallback,
}

/// Read exactly `n` bytes from `/dev/urandom`.
fn read_urandom(n: usize) -> Result<Vec<u8>, std::io::Error> {
    use std::io::Read;
    let mut file = std::fs::File::open("/dev/urandom")?;
    let mut buf = vec![0u8; n];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

/// The weak fallback: fold wall-clock nanos, pid and an address-hashed
/// RandomState into 32 bytes via Fx-style mixing (std only).
fn weak_fallback_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or_default();
    let pid = u64::from(std::process::id());
    let seed = nanos ^ (pid << 32) ^ RandomSeed::next();
    let mut bytes = Vec::with_capacity(32);
    let mut state = seed | 1;
    for _ in 0..4 {
        // splitmix64 finalizer — a well-mixed std-only generator.
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        bytes.extend_from_slice(&z.to_le_bytes());
    }
    crate::base64::encode(&bytes)
}

/// A per-process `RandomState`-derived seed (address randomness without
/// unsafe code: `RandomState::new()` is itself seeded randomly by std).
struct RandomSeed;

impl RandomSeed {
    fn next() -> u64 {
        use std::hash::{BuildHasher, Hasher};
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u64(0x4c55_4d45_4e00_0001); // "LUMEN" tag
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_and_unequal_comparisons() {
        assert!(constant_time_eq(b"", b""));
        assert!(constant_time_eq(b"token", b"token"));
        assert!(!constant_time_eq(b"token", b"tokeN"));
        assert!(!constant_time_eq(b"token", b"toke"));
        assert!(!constant_time_eq(b"token", b"tokenx"));
        // One-bit differences at every position are caught.
        for i in 0..8 {
            let mut a = [0u8; 8];
            let mut b = [0u8; 8];
            a[i] = 1 << (i % 8);
            b[i] = 0;
            assert!(!constant_time_eq(&a, &b), "position {i}");
            assert!(constant_time_eq(&a, &a));
        }
    }

    #[test]
    fn generated_tokens_are_256_bit_and_distinct() {
        let (token, source) = generate_token();
        assert_eq!(token.0.len(), 44, "base64 of 32 bytes, padded");
        assert_eq!(token.0.chars().last(), Some('='));
        let (other, _) = generate_token();
        assert_ne!(token, other, "two draws must differ");
        // Decode round-trips to 32 bytes (structure check; the value is
        // random).
        match crate::base64::decode(&token.0) {
            Ok(bytes) => assert_eq!(bytes.len(), 32),
            Err(err) => unreachable!("generated token not valid base64: {err}"),
        }
        let _ = source;
    }

    #[test]
    fn weak_fallback_is_well_formed() {
        let token = weak_fallback_token();
        assert_eq!(token.len(), 44);
        assert_ne!(token, weak_fallback_token());
    }
}
