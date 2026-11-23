# AUDIT: crates/codecs/src/video/h264/hw/frame_pool.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2025-06-27 |
| Lines / functions | 367 / 10 fns + 5 tests |
| File verdict | REAL — correct, tested, executed; but the hw lane it serves is unreachable from shipped hosts |
| Reachable from shipped entry point? | NO — the pool's only consumers are the vaapi (linux) / videotoolbox (macos) sessions inside `H264HwDecoder`; the CLI wires `"h264" \| "avc1" => H264SwDecoder::new` unconditionally (apps/cli/src/pipeline_host.rs:346) and `H264Decoder::from_setting` / the `playback.hw-decode-policy` setting have ZERO non-test callers (rg proof below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 frame_pool` -> 5/5 ok; rg wiring analysis; sed verification |

## 1. One-paragraph verdict
The pool itself is exactly what its docs claim: a hard-capped (≤64), typed,
panic-free slot lifecycle with generation-tagged handles that make stale
handles no-ops even after drain-and-reuse — and I executed all 5 of its tests,
including the 400-cycle steady-state recycle proof. The code has one
theoretical aliasing window (u64 generation wraps after 2³² drains, which is
not a real risk) and one design honesty gap: `release()` accepts slots in
`Decoding` state, which is documented but untested. The significant finding is
reachability: the whole hardware lane this pool serves is dead in shipped
builds — the CLI always constructs the software H264 decoder, the
`playback.hw-decode-policy` product setting is declared in product-config but
consumed by nothing, and the pipeline_host comment admits the hw lane "stays
the opt-in Phase-7 path". Real code, honest comment, no shipped caller.

## 2. What actually works (verified)
— `./target/debug/deps/codecs-bae0f1756cc0a809 frame_pool` -> "5 passed; 0
failed": lifecycle/clamp/drain-invalidation/garbage-handles/steady-state all
ok (my run).
— Baseline: cargo-test-workspace.txt carries the same tests green.
— Reachability proof (rg, no -r mangling): `from_setting` callers outside the
h264 module = tests/h264_hw_path.rs only; `hw-decode-policy` in apps/cli +
playback-engine = the one comment line pipeline_host.rs:342.

## 3. Findings
F-001 [S3]
Where: apps/cli/src/pipeline_host.rs:336-346 (context for this file's consumers)
Code:
            // (`H264Decoder::from_setting`, `playback.hw-decode-policy`)
            // stays the opt-in Phase-7 path: the shipped
            // `software-only` default resolves here, and the decoder's
            // own `configure` gate stays the honest authority.
            "h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),
What is wrong: the `playback.hw-decode-policy` setting exists in the
product-config model (crates/product-config/src/model.rs) and `H264Decoder::
from_setting` exists and is tested, but no shipped host reads either — the
setting is wired to nothing. Every h264/avc1 track takes the software decoder
regardless of policy. Why it matters: this pool (and the vaapi/videotoolbox
sessions it serves) compiles into the release lane and is exercised only by
tests; a user setting `hw-decode-policy = "prefer-hardware"` changes nothing.
The comment documents the gap honestly, so this is not fraud — it is a
declared-but-unwired setting plus dead-in-shipped-build code.
Confidence: HIGH — Verification: rg caller analysis (commands in §2).
F-002 [S4]
Where: crates/codecs/src/video/h264/hw/frame_pool.rs:236-238, 243-248
Code:
    fn handle_of(&self, index: usize) -> u64 {
        (self.generation << 32) | u32::try_from(index).unwrap_or(u32::MAX) as u64
    }
...
    fn split_handle(&self, handle: u64) -> Option<usize> {
        if handle >> 32 != self.generation {
            return None;
        }
What is wrong: generation is a u64 bumped by wrapping_add and shifted left 32;
after 2³² drains the shift discards high bits and the generation half can
repeat, so a handle minted 2³² drains earlier could alias a live one.
Unreachable in practice (one drain per stream renegotiation; 4 billion
renegotiations needed). What correct looks like: generation as u32 with the
wrap explicitly documented, or a monotonically checked counter.
Confidence: HIGH (arithmetic) / LOW (impact) — Verification: read.
F-003 [S4]
Where: crates/codecs/src/video/h264/hw/frame_pool.rs:183-198
Code:
    /// Return an emitted (or decoding — flush/teardown paths) slot
    /// to the free set and drop its resource. `true` when the slot
    /// was live. This is the present-side release hook.
    pub fn release(&mut self, handle: u64) -> bool {
What is wrong: the documented release-of-a-Decoding-slot path (flush/teardown)
has no test — the tests only release Emitted slots. If a future refactor
tightens the state match, the teardown path breaks silently.
Confidence: HIGH — Verification: read (all 5 tests inspected; ran).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HwSurfacePool::new | 103-110 | REAL | — | clamps to MAX_SURFACES |
| capacity / in_use / emitted | 113-134 | REAL | — | O(n), n≤64 |
| acquire | 142-162 | REAL | — | free-first, then grow to capacity, then typed PoolExhausted |
| emit | 170-181 | REAL | — | Decoding→Emitted only; double-emit no-op |
| release | 186-198 | REAL | S4 | Decoding release documented, untested (F-003) |
| holds_resource | 205-212 | REAL | — | T: PartialEq busy predicate |
| resource | 216-221 | REAL | — | borrowed; None after release |
| drain | 227-230 | REAL | — | generation bump; tested |
| handle_of / split_handle | 236-249 | REAL | S4 | 2³²-drain alias window (F-002) |
| tests (5) | 260-366 | REAL | — | executed, 5/5 (§2) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| acquire_emit_release_lifecycle_is_bounded_and_typed | Yes | exhaustion values, double emit/release, recycle identity c==a, mapping swap |
| capacity_is_clamped_to_the_safety_cap | Yes | 74→64 clamp, zero-cap typed failure |
| drain_invalidates_every_prior_handle | Yes | stale no-ops + generation advance assert |
| garbage_handles_are_noops_never_panics | Yes | u64::MAX etc. |
| steady_state_acquire_release_recycles_without_growth | Yes | 100 rounds × 4 slots, cap enforced each round |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bounded by construction ... exhaustion is a *typed* condition ... never an unbounded allocation" | L10-15 | REAL | code + executed tests |
| "That side of the contract is the 7.B/R5 renderer follow-up [PROPOSAL]; until it lands, a lane that exhausts its pool reports the typed error" | L26-31 | REAL | honest record of the missing present-side release; also means nothing shipped can release slots back |
| "ids never alias" (drain) | L224-225, 234-235 | REAL for practical horizons / theoretical wrap (F-002) | read |

## 7. Dead / duplicated / unreachable code
The pool is live code inside a dead-in-shipped-builds lane (F-001): consumers
are the OS-gated vaapi/videotoolbox sessions, reachable only through
`H264Decoder::from_setting`, which only tests call. Module is unconditional in
the codecs crate (h264 mod has no cfg gate), so it compiles into the release
lane as unreachable code.

## 8. Dependencies & flags
std-only (fmt). No unsafe (rg: zero hits outside doc text — none here at all).
No feature gates on frame_pool itself; consumers are `cfg(target_os)`.

## 9. Open questions for the next agent
- h264/hw/mod.rs + vaapi/videotoolbox sessions (not in my list): do the
  sessions' tests run on this Linux sandbox (baseline shows hevc_hw_path /
  h264_hw_path suites green — the vaapi lane must be compiling and being
  tested with fake bridges; real-driver execution is UNVERIFIED here)?
- The `playback.hw-decode-policy` unwired-setting finding belongs in whoever
  audits product-config/settings-schema wiring: is it surfaced in the UI?
