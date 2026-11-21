# AUDIT: crates/codecs/src/video/h264/hw/feed.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-21 |
| Lines / functions | 284 / 15 items (3 consts, 5 prod fns, helper, 6 tests + test helper) |
| File verdict | UNREACHABLE (pure, host-tested, correct — but its only production consumer is the macOS-cfg'd VideoToolbox lane, which no shipped binary reaches) |
| Reachable from shipped entry point? | NO — production callers are exclusively `videotoolbox/session.rs` (:160 `stamp_nanos`, :163/:180 `NANO_TIMESCALE`, :181 `nanos_stamp`, :689 `AVCC_LENGTH_SIZE`, :793 `avcc_sample_into`), a `#[cfg(target_os = "macos")]` module (hw/mod.rs:96-97) that itself sits behind `H264Decoder::from_setting` with no shipped caller (hw/mod.rs audit F-001). `annexb_stream` and `avcc_sample` have NO production callers at all. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | rg "annexb_stream\|avcc_sample\|stamp_nanos\|nanos_stamp\|NANO_TIMESCALE\|AVCC_LENGTH_SIZE" repo-wide (consumers listed above; the two `annexb_stream_feeds_aus...` hits are test-fn NAMES in h264/hevc hw_path suites, not calls); rg "hw::feed::tests::" baseline log → 6/6 ok (:557-562); byte-level re-derivation of the AVCC/Annex-B forms |

## 1. One-paragraph verdict
The best-engineered dead code in the hw tree: five total, unsafe-free, allocation-explicit functions (AVCC sample serialization with u32-refusal, Annex-B re-serialization, and the nanosecond↔Stamp round trip VideoToolbox's CMTime needs), each pinned by byte-exact host tests that all ran green in baseline (:557-562). The math is right — I re-derived the wire forms and the overflow refusals — and the test suite is honest (parse-back walks, capacity-settle assertions, negative-timestamp round trips). But every production consumer is the macOS-only VideoToolbox session inside the hw tree no shipped binary calls, so the whole module executes only under `cargo test`; `annexb_stream` is doubly dead (its documented consumer, the Media Foundation lane, does not exist — hw/mod.rs:51-56 "NOT IMPLEMENTED"), as is the one-shot `avcc_sample` (superseded by `avcc_sample_into` at the sole call site).

## 2. What actually works (verified)
— `rg -n "hw::feed::tests::" audit/runs/baseline/test-workspace.log.txt` → all 6 tests `ok` (:557-562): wire forms, u32-overflow refusal, buffer reuse, empty inputs, stamp round trip.
— Auditor re-derivation: AVCC form `[be32 len][nal]…` matches ITU-T H.14496-15's length-prefixed sample; `avcc_length_prefix` refuses `len > u32::MAX` via `u32::try_from` (no truncating cast anywhere); `annexb_stream`'s checked-capacity chain (:96-101) degrades to `with_capacity(0)` on overflow and still builds correctly — no panic path exists in the module; `NANO_TIMESCALE = 10⁹` fits i32 (CMTime's field) with 2.1× headroom.
— The parse-back proof in `avcc_sample_prefixes_every_nal_with_be32_lengths` (:167-177) walks the produced sample with the be32 lengths and lands exactly at `sample.len()` — the strongest kind of wire-form evidence short of a golden file.

## 3. Findings

### F-001 [S2] The module's entire production surface is unreachable — its only consumer is the macOS lane of the dead hw tree
Where: crates/codecs/src/video/h264/hw/feed.rs:1-138; consumer map videotoolbox/session.rs:160-181, 689, 793
Code:
```
//! All of it is total, allocation-explicit, and host-testable —
//! the lanes' driver-mediated halves are compile-verified only
//! (`[VERIFY]`, no macOS/Windows hardware in this sandbox); these
//! pure halves are the parts this host can and does pin.
```
What is wrong: On Linux (the only platform any shipped binary is built for in baseline), `pub mod feed` is compiled unconditionally but nothing calls it: its production callers are all inside `#[cfg(target_os = "macos")] pub mod videotoolbox` (hw/mod.rs:96-97), and even on macOS that lane is reachable only through `H264Decoder::from_setting` → `installed_bridges`, which no shipped host constructs (full flag/constructor trace in my hw/mod.rs audit F-001: apps/cli/src/pipeline_host.rs:346 hard-codes `H264SwDecoder`, product.toml:168 `hw-decode = false`).
Why it matters: Same class as the hw/mod.rs F-001 finding — correctly-built, well-tested machinery compiled into every shipped binary and executed only by the test suite. The "host-tested" label in the module docs is true but invites the reader to conflate test-execution with production reachability.
What correct looks like: Gate the whole hw tree (feed included) behind a default-off `codec-h264-hw` feature like `codec-av1`/`codec-vp9` (crates/codecs/Cargo.toml:60-81), or wire a shipped consumer.
Confidence: HIGH — Verification: read + exhaustive rg caller trace (both decoders, both lanes, all apps).
### F-002 [S3] `annexb_stream` and `avcc_sample` are doubly dead — their documented consumers do not exist
Where: crates/codecs/src/video/h264/hw/feed.rs:17-19, 49-63, 83-108
Code:
```
//! - Media Foundation: the **Annex-B byte stream** — every NAL
//!   prefixed with a 4-byte start code ([`annexb_stream`]); the
//!   form the H.264 MFT's `MFVideoFormat_H264` input type takes.
```
What is wrong: `annexb_stream` has zero production callers (rg: only its own tests; the `annexb_stream_feeds_aus_and_emits_opaque_frames` hits are test-function names). Its stated consumer — the Windows Media Foundation lane — does not exist and is documented as not existing (hw/mod.rs:51-56: "Media Foundation (Windows, 7.B2): NOT IMPLEMENTED — no MF lane exists in this crate"). The doc here uses the present tense ("the form the H.264 MFT's MFVideoFormat_H264 input type takes") as if the lane were live. `avcc_sample` (one-shot) is likewise test-only — the sole production site uses `avcc_sample_into` (session.rs:793).
Why it matters: Two of five public functions exist only for tests or for a lane the tree explicitly disclaims; a maintainer reading feed.rs would reasonably infer an MF consumer. Dead code in a shipped crate.
What correct looks like: Delete `annexb_stream` until the MF lane lands (git history keeps it), and either delete `avcc_sample` or mark both `#[cfg(test)]`-adjacent with an honest "no consumer yet" doc line.
Confidence: HIGH — Verification: read + rg across all crates/apps (non-md).
### F-003 [S4] `avcc_sample_into`'s `None` leaves a partial sample in `out` — contract undocumented
Where: crates/codecs/src/video/h264/hw/feed.rs:65-81
Code:
```
pub fn avcc_sample_into<'a, I>(units: I, out: &mut Vec<u8>) -> Option<()>
where
    I: IntoIterator<Item = &'a [u8]>,
{
    out.clear();
    for unit in units {
        let prefix = avcc_length_prefix(unit.len())?;
```
What is wrong: On a u32-overflow refusal the function returns `None` with `out` cleared and partially written (every unit before the oversized one is already in the buffer). The doc says "clears `out`, writes the sample into it, `None` on the same `u32`-space refusal" — a reader could assume `None` ⇒ `out` untouched or empty. The sole caller (session.rs:793-797) discards the buffer on `None` (scratch reuse), so no bug manifests.
Why it matters: Contract ambiguity in a reusable-buffer API is how future callers mint use-after-discard bugs; one doc sentence fixes it.
What correct looks like: Document "on `None`, `out` holds a prefix of the would-be sample — callers must treat it as poison" or leave `out` untouched until success is proven (pre-compute total length first).
Confidence: HIGH — Verification: read; partial-write behavior follows directly from the `?` at :76.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ANNEXB_START_CODE` const | 28-30 | REAL | — | 4-byte form, correct per Annex B. |
| `AVCC_LENGTH_SIZE` const | 32-35 | REAL | — | 4; consumed by session.rs:689 as the nalUnitHeaderLength. |
| `NANO_TIMESCALE` const | 37-41 | REAL | — | 10⁹; fits i32; the CMTime base. |
| `avcc_length_prefix` | 43-47 | REAL | — | `u32::try_from` refusal — no truncating cast; test-pinned at the u32::MAX boundary (:224-228). |
| `avcc_sample` | 49-63 | REAL | S3 | One-shot allocating form; production-dead (F-002). |
| `avcc_sample_into` | 65-81 | REAL | S4 | Hot-path form; correct bytes; partial-write-on-None (F-003); capacity-settle behavior test-pinned (:191-206). |
| `annexb_stream` | 83-108 | REAL | S3 | Correct bytes + checked capacity; production-dead for a nonexistent MF lane (F-002). |
| `stamp_nanos` | 110-128 | REAL | — | Sign-preserving; TimeBase constructed per call (trivial; called once per frame in the unreachable lane); `Stamp::Missing` → None. |
| `nanos_stamp` | 133-138 | REAL | — | Inverse; `Err → Stamp::Missing` (never a guess); round-trip test incl. negative (:272-275). |
| test `avcc_sample_prefixes_every_nal_with_be32_lengths` | 157-178 | REAL | — | Byte-exact + parse-back walk; ran ok (:560). |
| test `avcc_sample_into_reuses_the_buffer_without_regrowing` | 180-213 | REAL | — | Capacity-settle assertion over 64 rounds; ran ok (:559). |
| test `avcc_length_prefix_refuses_the_u32_overflow_never_truncates` | 215-238 | REAL | — | Boundary + both serializer refusal paths; ran ok (:558). |
| test `annexb_stream_prefixes_every_nal_with_start_codes` | 240-261 | REAL | — | Includes the zero-run-in-NAL non-mangling case; ran ok (:557). |
| test `empty_inputs_produce_empty_outputs` | 263-268 | REAL | — | Ran ok (:561). |
| test `stamp_nanos_round_trips_through_the_nanosecond_base` | 270-283 | REAL | — | Both directions × 5 values incl. negative; ran ok (:562). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| avcc_sample_prefixes_every_nal_with_be32_lengths | YES | Byte-equality against an independently built expectation + a parse-back cursor walk; endianness or off-by-one breaks it instantly. |
| avcc_sample_into_reuses_the_buffer_without_regrowing | YES | Equality vs the one-shot form AND a capacity-never-grows assertion across 64 rounds. |
| avcc_length_prefix_refuses_the_u32_overflow_never_truncates | YES | Exact boundary values (u32::MAX ok, +1 → None). |
| annexb_stream_prefixes_every_nal_with_start_codes | YES | Concatenation equality + the EPB-preservation case. |
| empty_inputs_produce_empty_outputs | YES | Trivially. |
| stamp_nanos_round_trips... | YES | Exact Some/eq assertions both directions. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "these pure halves are the parts this host can and does pin" | feed.rs:21-24 | REAL | 6/6 tests green in baseline (:557-562). |
| "Media Foundation: the Annex-B byte stream … the form the H.264 MFT's MFVideoFormat_H264 input type takes" | feed.rs:17-19 | PAPER-ONLY | No MF lane exists (hw/mod.rs:51-56); annexb_stream has no production caller (F-002). |
| "The buffer keeps its capacity across pictures — zero steady-state allocation" | feed.rs:67-68 | REAL | Capacity-settle test (:191-206) pins it. |

## 7. Dead / duplicated / unreachable code
- Whole module production-unreachable (F-001).
- `annexb_stream` (83-108) and `avcc_sample` (49-63): no production callers (F-002).
- No duplication: this is the single serializer module; the sw decoder's framing lives in parser::nal (different layer, legitimately separate).

## 8. Dependencies & flags
`media_types::Stamp` + `media_types::time` only; no unsafe, no features, compiled unconditionally (hw tree is ungated — crates/codecs/Cargo.toml:25-27 documents the choice). Wave-6 hostile question: none of this affects shipped H.264 reachability — the shipped sw path uses parser::nal's framing (sw.rs:92), not this module.

## 9. Open questions for the next agent
- hevc/hw.rs references feed? (rg showed hevc's own hw test file only.) If a future HEVC hw lane wants the same serializers, F-002's deletion decision belongs to that card.
- videotoolbox/ffi.rs (A05): the CMTime struct's `timescale: i32` — confirm NANO_TIMESCALE's i32 typing matches the ffi decl (it must be signed 32; a u32 mismatch would silently reinterpret).
