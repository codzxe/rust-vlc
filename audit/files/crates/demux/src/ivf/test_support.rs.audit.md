# AUDIT: crates/demux/src/ivf/test_support.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-21 |
| Lines / functions | 74 / 5 items (module docs, `mem_stream`, `ivf_header`, `ivf_frame`, `demo_stream`) |
| File verdict | REAL |
| Reachable from shipped entry point? | Compiled into the shipped `demux` rlib (declared `pub mod test_support;` with **no cfg gate**, ivf/mod.rs:103) and linked into the `lumen` binary's dependency graph; **not on any player path** (no demuxer/engine call site). Users verified by rg: `ivf/tests.rs:12` (cfg-test unit suite), `tests/ivf_fixtures.rs:217,261` (integration differential — the reason the module must be pub and ungated), `ivf/fuzz.rs:50` (the production-compiled fuzz harness — load-bearing at runtime). All consumers executed in baseline: 22/22 ivf unit tests, ivf_fixtures 5/5, fuzz-lite sweeps (orchrun-test-workspace-nofailfast.log.txt:1691-1713, :2078-2089). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Full-file Read (74 lines) · `rg -n "pub fn mem_stream" crates/demux/src/avi/test_support.rs` + read of the body (delegation target exists, `BytesStream::new`, returns `Box<dyn SourceStream>` — the shared test seam as documented) · `rg -n "test_support::" crates/demux/src/ivf/*.rs crates/demux/tests/ivf_fixtures.rs` (the three consumer classes above) · `rg -n "pub mod test_support" ivf/mod.rs` (:103 — no cfg gate; production scope) · field-layout cross-check of `ivf_header` against `IvfHeader::parse`'s offsets (ivf/mod.rs:185-194 — byte-for-byte match) · baseline greps above. |

## 1. One-paragraph verdict
A 74-line fixture-builder module that does its one job correctly: `ivf_header` emits a byte-exact 32-byte DKIF header (every field at the offset `IvfHeader::parse` reads — cross-checked field by field), `ivf_frame` emits the 12-byte frame header plus a byte-labeled mod-251 ramp whose non-zero pattern is a deliberate truncation-bug detector, and `demo_stream` is the documented three-frame demo shape. The two defects are scope honesty, not bytes: the module is named and documented as test support yet ships ungated in the production library and is load-bearing for the production-compiled fuzz harness (so it can never be `#[cfg(test)]` as the name implies — the doc's "nothing in the demuxer uses it" is true of `IvfDemuxer` but the module is not test-scoped), and `ivf_frame` silently clamps an over-`u32` payload length to `u32::MAX`, emitting a frame whose declared size contradicts its payload — the one shape a deterministic builder must never produce by accident.

## 2. What actually works (verified)
— `ivf_header` (:28-48): field order/widths match `IvfHeader::parse` exactly (magic 0-3, version 4-5, header_len 6-7 = 32, fourcc 8-11, width 12-13, height 14-15, rate 16-19, scale 20-23, frames 24-27, unused 28-31 = zero). Total 32 bytes — asserted implicitly by every executed test that round-trips through `parse`.
— `ivf_frame` (:54-62): size prefix + 64-bit LE pts + ramp; the doc's rationale ("zero runs hide truncation bugs") is real — `clean_walk_passes_the_container_truth` asserts the exact ramp prefix `[0..8]` (ivf/tests.rs:159, 176), and the truncation sweeps (`fuzz_lite_truncations_never_panic_or_hang`) rely on distinguishable bytes.
— `demo_stream` (:68-74): internally consistent (header declares rate 30/scale 1/3 frames; three frames at pts 0/1/2) — consumed by the fuzz sweeps and the determinism cross-check (tests.rs:492-518).
— `mem_stream` (:17-21): a two-line delegation to the crate's shared `avi::test_support::mem_stream` (target verified) — the documented "shared test seam" pattern; its `# Errors` doc ("propagates the never-failing URI parse") matches the target's behavior.
— Execution: every consumer green in baseline (22/22 unit, 5/5 fixture differential, fuzz sweeps — orchrun :1691-1713, :2078-2089).

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/ivf/test_support.rs:1-6
Code:
```
//! In-crate byte builders for the IVF tests (card 5.I3b).
//!
//! Public like [`crate::avi::test_support`] on purpose: the
//! integration/fuzz suites need deterministic container shapes
//! (including deliberately malformed ones). Test-support surface, not
//! player surface — nothing in the demuxer uses it.
```
What is wrong: The module is named and scoped as test support but compiled into the production library unconditionally (`pub mod test_support;`, ivf/mod.rs:103, no cfg gate — verified) and is load-bearing for production code: `ivf/fuzz.rs:50` calls `test_support::mem_stream` at runtime, and fuzz.rs is itself ungated. The doc's last sentence is literally true only of the `IvfDemuxer` struct; read as a scope statement ("test-support surface") it misleads — this code ships in the rlib and links into every consumer of `demux`, and the module can never be `#[cfg(test)]`-gated precisely because the fuzz harness needs it.
Why it matters: Naming that misstates compile scope is the checklist's "naming that lies" class: a future contributor reasonably assuming `test_support` is test-only could add heavier fixture machinery here (larger demo streams, generator state) and silently ship it in the player binary; conversely someone "cleaning up test code" could try to cfg-gate it and break the fuzz build. The crate has four sibling modules with the same ambiguity (avi/wav/es/ivf) — this is the pattern's smallest instance, so the cheapest place to fix the naming.
What correct looks like: Rename to what it is (`builders`, `fixtures`, or `harness_support`) with a first-line doc stating "compiled into the lib: the integration suites and the fuzz harness consume it; no player path does" — or gate fuzz.rs's use behind the fuzz feature and cfg(test) the rest.
Confidence: HIGH — Verification: read; all call sites enumerated by rg; ivf/mod.rs:103 gate absence verified.

### F-002 [S4]
Where: crates/demux/src/ivf/test_support.rs:56
Code:
```
    out.extend_from_slice(&u32::try_from(payload_len).unwrap_or(u32::MAX).to_le_bytes());
```
What is wrong: A `payload_len` above `u32::MAX` is silently clamped to `u32::MAX` in the declared size while the actual payload written is `payload_len` bytes (the loop at :58-60 pushes all of them) — the builder emits a frame whose 4-byte size field lies about its own payload, with no error, no panic, no marker.
Why it matters: The module's contract is deterministic, honest container shapes (malformed shapes are built deliberately and visibly — e.g. tests hand-craft a 500-byte claim at tests.rs:285). A silent inconsistency generator is the opposite: a caller passing a 4 GiB+ length (a plausible future fuzz-scale or overflow-regression test) gets a shape whose malformation is an implementation artifact, not the caller's intent — poisoning whatever the test thought it was pinning. `payload_len` is `usize`, so the overflow input is representable on 64-bit hosts.
What correct looks like: `u32::try_from(payload_len).expect("ivf_frame payload_len exceeds u32")` — a builder bug should fail the test loudly, not mint a lying frame. (If an over-`u32` declared-size frame is ever wanted as a deliberate malformed shape, build it by hand as the 500-byte-claim test does.)
Confidence: HIGH — Verification: read; the payload loop's independence from the clamp traced.

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (scope claim) | 1-6 | REAL | S4 | F-001 — "test-support surface" vs production compile scope. |
| `mem_stream` | 17-21 | REAL | — | Delegation to the crate-shared seam (target verified); `# Errors` doc accurate. |
| `ivf_header` | 28-48 | REAL | — | Byte-exact vs `IvfHeader::parse` (field-by-field cross-check); `#[must_use]`; capacity preallocated. |
| `ivf_frame` | 54-62 | REAL | S4 | F-002 (silent u32::MAX clamp). Ramp design sound and load-bearing. |
| `demo_stream` | 68-74 | REAL | — | Internally consistent; consumed by fuzz sweeps (executed). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| Every ivf unit/fixture test that consumes these builders | YES (for builder drift) | The builders feed 22 unit tests + the 5-test ffmpeg differential; a field-order or ramp mutation fails `header_parse_roundtrips_the_field_set` / `clean_walk_passes_the_container_truth` immediately (both assert exact bytes). |
| (No tests exist *for* this module) | — | The module has no self-tests (avi/test_support has a small cfg-test module; this one none). Its correctness is pinned only indirectly through consumers — acceptable for 74 lines of builders, noted for completeness. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Test-support surface, not player surface — nothing in the demuxer uses it" | module docs :5-6 | REAL (player path) / **MISLEADING (compile scope)** | No player/engine call site (rg over apps/ + engine crates: zero hits); but the module is production-compiled and used by fuzz.rs:50 (F-001). |
| "`frames` is advisory and freely lied about" | :25-26 | REAL | Matches the demuxer's advisory-count contract (my ivf/mod.rs audit, tests.rs:216-227). |
| "The ramp is byte-labeled … zero runs hide truncation bugs" | :50-52 | REAL | Ramp asserted byte-exact by executed tests; the truncation sweeps rely on it. |

## 7. Dead / duplicated / unreachable code
- No dead items; all five exported symbols consumed (rg: tests.rs, ivf_fixtures.rs, fuzz.rs).
- The `mem_stream` two-line delegation duplicates nothing (it exists so ivf callers don't import from `avi::test_support` directly — a naming-boundary shim).

## 8. Dependencies & flags
- `plugin_api::SourceStream` (trait object), `std::sync::Arc`, and the crate-internal `avi::test_support` (the shared seam). No feature gates — the module compiles in every profile, which is F-001's point.

## 9. Open questions for the next agent
- The `avi/test_support.rs` and `wav/test_support.rs` assignees: same production-scope naming question (F-001's pattern); check whether their modules carry the same "nothing uses it" claims and whether any production code (fuzz harnesses) consumes them.
- If a sweep wave standardizes the four test_support modules: F-002's clamp and F-001's naming belong in the same cleanup.
