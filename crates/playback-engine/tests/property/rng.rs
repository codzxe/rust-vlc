//! The shared property harness (card 3.D3): the seeded RNG, the fixed
//! seed bank, and the documented shrink procedure.
//!
//! # Why hand-rolled (the repo's own convention, `limits.rs` /
//! `backpressure_property.rs`)
//!
//! `proptest` is not a workspace dependency and dependency approval
//! is Lead-only (AGENTS.md §3.2, C5) — so every property loop here is
//! a deterministic seeded loop instead, per the card's fallback
//! clause. The trade is spelled out honestly:
//!
//! - *coverage*: a 64-seed bank over a few thousand adversarial ops
//!   per seed is a sampler, not an exhaustive prover — the exhaustive
//!   proofs (104-pair model check, bit-exactness proofs inside
//!   `media-types`) live in their own suites; these properties widen
//!   them with randomized adversarial shapes;
//! - *shrinking*: proptest's automatic shrinking is replaced by the
//!   documented bisection below — log₂ N reruns to a minimal failing
//!   op prefix, deterministic because every rerun rebuilds the case
//!   from the seed alone.

/// Deterministic xorshift* (the `limits.rs` discipline): one step is
/// three shifts and three xors; a failed seed reruns the exact op
/// sequence.
pub struct XorShift(u64);

impl XorShift {
    /// Seed the generator. Odd seeds only (a zero seed locks up).
    pub fn new(seed: u64) -> XorShift {
        XorShift(seed | 1)
    }

    /// The next raw word.
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform below `n` (n ≥ 1). Rejection-free (modulo bias is
    /// irrelevant for adversarial sampling and determinism is what
    /// matters).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// A uniformly random `i64` (full range, both signs).
    pub fn i64(&mut self) -> i64 {
        self.next() as i64
    }

    /// A uniformly random nonzero `u32`.
    pub fn u32(&mut self) -> u32 {
        loop {
            let word = (self.next() >> 32) as u32;
            if word != 0 {
                return word;
            }
        }
    }

    /// A fair coin.
    pub fn flip(&mut self) -> bool {
        self.next() & 1 == 1
    }

    /// One element of `items` (non-empty by caller contract).
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let index = self.below(items.len() as u64) as usize;
        &items[index]
    }
}

/// The fixed seed bank every gate runs: the first 64 odd integers
/// (1, 3, …, 127). Deliberately small, dense and reproducible — a
/// seed printed by a failure is its own rerun instruction
/// (`LUMEN_PROP_SEEDS=<seed>`).
pub const SEED_BANK: [u64; 64] = {
    let mut seeds = [0u64; 64];
    let mut index = 0;
    while index < 64 {
        seeds[index] = (index as u64) * 2 + 1;
        index += 1;
    }
    seeds
};

/// The bank, widened by `LUMEN_PROP_SEEDS` (comma-separated u64s; the
/// corpus suites' env-gated precedent for CI sweeps and local triage).
pub fn seeds() -> Vec<u64> {
    let mut seeds = SEED_BANK.to_vec();
    if let Ok(extra) = std::env::var("LUMEN_PROP_SEEDS") {
        for part in extra.split(',') {
            if let Ok(seed) = part.trim().parse::<u64>() {
                seeds.push(seed | 1);
            }
        }
    }
    seeds
}

/// Whether `seed` is the one the operator asked to shrink
/// (`LUMEN_PROP_SHRINK=<seed>`).
fn shrink_requested(seed: u64) -> bool {
    match std::env::var("LUMEN_PROP_SHRINK") {
        Ok(requested) => requested.trim().parse::<u64>() == Ok(seed),
        Err(_) => false,
    }
}

/// The documented shrink procedure: **bisection on the failing op
/// sequence**.
///
/// Given the full op list of a failing case and a predicate that
/// reruns any prefix (the case is rebuilt from the seed alone, so
/// every rerun is deterministic), return the minimal-length failing
/// prefix. log₂ N reruns; the result is minimal in prefix length
/// (not necessarily minimal in op parameters — value shrinking is
/// the follow-up triage step, as with any bisection-based shrinker).
pub fn shrink_prefix<T: Clone>(ops: &[T], still_fails: impl Fn(&[T]) -> bool) -> Vec<T> {
    if ops.is_empty() || !still_fails(ops) {
        // The caller only shrinks a failing case; an empty or
        // non-failing sequence has nothing to shrink.
        return Vec::new();
    }
    // An empty prefix cannot fail (a driver with no ops is a no-op),
    // so the search space is prefix lengths 1..=len.
    let mut lo = 1usize;
    let mut hi = ops.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if still_fails(&ops[..mid]) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    ops[..lo].to_vec()
}

/// Run one seeded case, shrinking it first if the operator asked for
/// this seed. `run` must be a pure function of `(seed, ops)` —
/// rebuilding the whole case per invocation — or shrinking lies.
pub fn run_case<T: Clone + std::fmt::Debug>(
    seed: u64,
    ops: &[T],
    run: impl Fn(u64, &[T]) -> Result<(), String>,
) -> Result<(), String> {
    match run(seed, ops) {
        Ok(()) => Ok(()),
        Err(failure) => {
            if shrink_requested(seed) {
                let minimal = shrink_prefix(ops, |prefix| run(seed, prefix).is_err());
                Err(format!(
                    "{failure}\n[shrunk] seed {seed}: minimal failing prefix \
                     {}/{} ops: {minimal:?}",
                    minimal.len(),
                    ops.len()
                ))
            } else {
                Err(format!(
                    "{failure}\n[rerun] LUMEN_PROP_SEEDS={seed} to reproduce; \
                     LUMEN_PROP_SHRINK={seed} to shrink"
                ))
            }
        }
    }
}

/// The documented shrink procedure, self-tested: the bisection finds
/// the minimal failing prefix of a synthetic predicate, and a
/// non-failing sequence shrinks to nothing. The family suites drive
/// it through [`run_case`] on every failure; this pins the machinery
/// itself (the same proof lives in packet-queue's property suite —
/// the harness is duplicated across the two test crates).
#[test]
fn shrink_prefix_bisects_to_the_minimal_failing_prefix() {
    let ops: Vec<u64> = (0..64).collect();
    // Fails iff the prefix contains the op valued 40 — the minimal
    // failing prefix is ops[0..=40] (41 ops).
    let fails = |prefix: &[u64]| prefix.contains(&40);
    let minimal = shrink_prefix(&ops, fails);
    assert_eq!(minimal.len(), 41, "the prefix must run through op 40");
    assert_eq!(minimal.last().copied(), Some(40));
    // An empty or non-failing sequence has nothing to shrink.
    assert!(shrink_prefix(&ops, |_| false).is_empty());
    assert!(shrink_prefix(&[], fails).is_empty());
}
