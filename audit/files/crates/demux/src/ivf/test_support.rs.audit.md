# AUDIT: crates/demux/src/ivf/test_support.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-13 |
| Lines / functions | 75 / 4 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO as live code — `pub mod test_support` (ivf/mod.rs:103) is compiled into the lib but no shipped caller exists; consumers are ivf/tests.rs (cfg(test), 23 tests pass), tests/ivf_fixtures.rs, and the fuzz shim fuzz/fuzz_targets/fuzz_demux_ivf.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `cargo test -p demux --lib ivf` → 23 passed (exit 0, consumers of this module); rg consumer census; od byte-verification of line 67 |

## 1. One-paragraph verdict
A correct, small test-builder module: the 32-byte DKIF header layout is field-exact
(magic/version/hdrlen=32/fourcc/w×h/rate/scale/frames/unused, all little-endian),
the 12-byte frame header (u32le size + u64le pts) is exact, and the payload ramp
(i % 251) is byte-labeled so truncations are detectable — good design. It executes:
the 23 in-crate ivf tests that build every input through these helpers pass. All
findings are hygiene: the demo_stream doc cites a doctest that does not exist, the
frame builder silently truncates declared sizes above u32::MAX, the module ships
compiled into the library because integration tests need it public, and mem_stream
returns Result for a documented never-failing operation.

## 2. What actually works (verified)
- `cargo test -p demux --lib ivf` → 23 passed — every test byte-shape in
  ivf/tests.rs is built by ivf_header/ivf_frame/mem_stream/demo_stream (import at
  tests.rs:12), so the builders' output is demuxer-accepted by construction
  (parse tests assert fields back).
- Layout arithmetic: 4+2+2+4+2+2+4+4+4+4 = 32 header bytes; frame = 4+8+payload;
  ramp period 251 (prime, non-zero, non-256 — zeros would hide truncation as the
  doc notes).
- Line 67's attribute is `#[must_use]` (od-verified: `# [ m u s t _ u s e ]`) — an
  earlier terminal-mangled display suggested otherwise; the source is well-formed.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/ivf/test_support.rs:64-66
Code:
```
/// An AV1-flavored demo stream: 30 fps, 64×48, three ramp frames at
/// pts 0/1/2 (the doctest's demo shape — the same field values
/// ffmpeg writes for a 30 fps delivery).
```
What is wrong: there is no doctest referencing demo_stream anywhere in the ivf
module (rg: the only "doctest" hit in crates/demux/src/ivf/ is this very comment) —
the doc cites a nonexistent artifact (stale reference, likely to a deleted or
never-landed doctest). The ffmpeg-field-values claim is UNVERIFIED (no ffmpeg in
sandbox). Confidence: HIGH on the dangling reference — Verification: read + rg.
F-002 [S4]
Where: crates/demux/src/ivf/test_support.rs:56
Code:
```
    out.extend_from_slice(&u32::try_from(payload_len).unwrap_or(u32::MAX).to_le_bytes());
```
What is wrong: for payload_len > u32::MAX the builder silently writes a declared
size (u32::MAX) smaller than the actual payload instead of failing loudly — a
mismatched-size frame appears without warning. Latent (tests would have to allocate
>4 GiB), but a builder whose output can silently contradict its own header is a
footgun in exactly the malformed-shape regime this module exists to serve.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/demux/src/ivf/test_support.rs:1-6 with ivf/mod.rs:103
Code:
```
//! Public like [`crate::avi::test_support`] on purpose: the
//! integration/fuzz suites need deterministic container shapes
//! (including deliberately malformed ones). Test-support surface, not
//! player surface — nothing in the demuxer uses it.
```
What is wrong: "nothing in the demuxer uses it" is true, but `pub mod test_support`
is un-gated, so this test-only code is compiled into every build of the demux lib
(release lane included; possibly linker-stripped later). The sharing design
(integration tests need pub items) forces this — a recorded tradeoff, not an
accident, but 4 modules now ship test-support surfaces this way (avi, ps, adts,
ivf). Confidence: HIGH — Verification: read + rg consumer census.
F-004 [S4]
Where: crates/demux/src/ivf/test_support.rs:15-20
Code:
```
/// # Errors
/// Propagates the (never-failing) URI parse.
pub fn mem_stream(
    bytes: impl Into<Arc<[u8]>,
) -> Result<Box<dyn SourceStream>, media_types::EngineError> {
```
What is wrong: a documented never-failing operation returning Result forces every
call site to `.must()`/handle a impossible error — vestigial error surface (same
shape as the avi twin). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mem_stream | 17-21 | REAL | S4 | never-failing Result (F-004); delegates to avi twin |
| ivf_header | 28-48 | REAL | — | field-exact 32-byte DKIF, little-endian |
| ivf_frame | 54-62 | REAL | S4 | silent size truncation >u32::MAX (F-002); ramp payload |
| demo_stream | 68-74 | REAL | S4 | dangling doctest reference (F-001) |

## 5. Test honesty
This module IS the test fixture factory; the honesty question is whether its output
is verified anywhere: yes — ivf/tests.rs parses the headers back and asserts fields
(e.g., IvfHeader::parse(&ivf_header(b"VP90",…)).must() at tests.rs:73), and the 23
tests pass by execution. The ramp labeling makes truncations observable rather than
silent (documented and real).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "nothing in the demuxer uses it" | test_support.rs:6 | REAL | rg census: only tests/fuzz consumers |
| "the doctest's demo shape" | test_support.rs:65 | FALSE | no such doctest exists (F-001) |
| "the same field values ffmpeg writes for a 30 fps delivery" | test_support.rs:65-66 | UNVERIFIED | ffmpeg absent from sandbox |
| "zero runs hide truncation bugs" (ramp rationale) | test_support.rs:51-52 | REAL | i % 251 labeling; consumers assert byte-exact payloads |

## 7. Dead / duplicated / unreachable code
The module as a whole is shipped-compiled but shipped-unused (F-003) — the only
"dead" aspect. No internal duplication.

## 8. Dependencies & flags
None beyond plugin_api/avi test seam. No feature gates.

## 9. Open questions for the next agent
- Whoever audits ivf/mod.rs: the header-length gate ("only 32 accepted") and the
  AV1/VP9/VP8 fourcc gate are asserted in tests.rs — confirm no other fourcc
  silently accepted (probe-side vs demux-side gate mismatch would matter).
