# AUDIT: crates/demux/src/wav/fuzz.rs (full file, 251 lines)
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-11-21 |
| Lines / functions | 251 / 3 consts + 1 harness fn + Scrambler (2 methods) + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | Harness: NOT from `lumen` (fuzz-only by design) but WIRED to the libFuzzer target `fuzz/fuzz_targets/fuzz_demux_wav.rs:18-20` (registered in `fuzz/Cargo.toml:90-91`, `fuzz/README.md:57`; registry check PASS in baseline `full-gate.log.txt:46-48`); in-crate tests executed: 5/5 ok (`orchrun-test-workspace-nofailfast.log.txt:1922-1926`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "fuzz_demux_wav" fuzz/ scripts/` (target exists, wired); read of `fuzz/fuzz_targets/fuzz_demux_wav.rs` + `fuzz/README.md` (nightly campaign NOT run — Lead decision); `rg -n "minimal_pcm_wav" crates/demux/src/wav/test_support.rs` + byte-layout arithmetic (F-001); `sed -n` on all quoted ranges; baseline greps |

## 1. One-paragraph verdict
The WAV fuzz harness is a real, bounded, honestly-documented hostile-input
pass: construct → start → drain (200k-event budget) → five data-derived
seek probes each committing through `reanchor` and draining a bounded
window. The module doc states the exact contract (absence of
panic/hang/unbounded allocation, nothing else) and does not overclaim. The
in-crate deterministic sweeps (bit-scramble ladders at 0..64 flips over
three structural shapes, header-site truncations, extreme size patches) all
ran and passed in the baseline, and the libFuzzer target exists and is
registry-checked — but no actual fuzzing campaign has ever run (nightly
wiring is an unexecuted Lead decision). One test defect is real: the
"adversarial size fields" test's data-size patches target the WRONG byte
offset — they corrupt `nAvgBytesPerSec` inside the `fmt ` payload (a field
the demuxer never reads) while the actual `data` size field is never patched
with an extreme value, so the test's claimed coverage is not delivered.

## 2. What actually works (verified)
- 5/5 tests ok in baseline (:1922-1926): `adversarial_size_fields_survive`,
  `empty_and_tiny_inputs_survive`, `scrambled_extensible_headers_survive`,
  `scrambled_structured_files_survive`, `truncations_at_every_header_site_survive`.
- The harness is wired: `fuzz/fuzz_targets/fuzz_demux_wav.rs:18-20` calls
  `demux::wav::fuzz::fuzz_demux_wav(data)`; the registry check (35 targets,
  diff empty, "corpora present") PASSED in baseline (`full-gate.log.txt:46-48`).
- Bounds verified by read: event budget 200k (:66-68), post-seek drain 64
  (:106-111), seek probes 5 (:94), payload ceilings are the demuxer's own
  (MAX_PACKET_BYTES/MAX_FMT_BYTES/MAX_INFO_BYTES — none added here).
- The scrambler is deterministic (xorshift64*, fixed seeds) — every sweep is
  reproducible byte-for-byte.

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/wav/fuzz.rs:222-231
Code:
```rust
        for patch in [
            (4u32, u32::MAX),
            (4, 0),
            (16, u32::MAX), // fmt size (at 12 + 4)
            (16, 0),
            (16, 1),
            (28, u32::MAX), // data size (at 24 + 4)
            (28, 0),
            (28, 1),
        ] {
```
What is wrong: The patch site labeled "data size (at 24 + 4)" is not the
data size field. `minimal_pcm_wav` (test_support.rs:21-49) lays out:
`RIFF[0..4) size[4..8) WAVE[8..12) "fmt "[12..16) fmt-size[16..20)` then a
16-byte fmt payload `[20..36)` (tag[20..22) ch[22..24) rate[24..28)
**avg[28..32)** align[32..34) bits[34..36)) and only then
`"data"[36..40) data-size[40..44)`. Offset 28 is `nAvgBytesPerSec` — a
field the demuxer never reads (not in `WaveFormat`). The actual data size
field (offset 40) is never patched with an extreme value in this test.
Why it matters: The test's stated purpose — "Extreme declared sizes at the
interesting sites: … the data size" — is not delivered for the data size:
the three `(28, …)` patches are no-ops for coverage (corrupting an ignored
field). Extreme data-size declarations (`u32::MAX`, 0, 1) are exactly the
adversarial class the harness exists for (the mod.rs walk clamps them —
`data-extends-past-stream`, sentinel handling), and this test claims to
cover them while covering nothing. (Partial mitigation: the random
bit-scramble tests can hit offset 40 by chance, and the truncation tests
cut the data extent — but the deterministic extreme-value coverage the
test name promises does not exist.)
What correct looks like: `(40, u32::MAX)`, `(40, 0)`, `(40, 1)` — with the
comment fixed to "data size (at 36 + 4)".
Confidence: HIGH — Verification: read + byte-layout arithmetic against the
builder (test_support.rs:29-47), twice-checked.

### F-002 [S4]
Where: crates/demux/src/wav/fuzz.rs:14-18
Code:
```rust
//! The cargo-fuzz target file (`fuzz/fuzz_targets/fuzz_demux_wav.rs`)
//! is a Lead-owned dependency decision (`libfuzzer-sys`), exactly as
//! the AVI card recorded; until it lands the harness logic is proven
//! by the deterministic in-crate sweeps here and by the integration
//! corpus run over the mutated fixture family.
```
What is wrong: The "Wiring status (honest)" section is stale: the target
file HAS landed (`fuzz/fuzz_targets/fuzz_demux_wav.rs` exists, calls this
harness, is registered in `fuzz/Cargo.toml:90-91` and the README registry
row 57, and the registry check passed in the baseline gate). The irony is
the section's own name — the honesty note itself drifted out of date.
Why it matters: Auditors and the matrix read "wiring pending" when the
wiring is done and check-passed; the remaining honest gap (no campaign has
RUN — nightly wiring is still an unexecuted Lead decision per fuzz/README)
is now documented in the target file's own header instead.
What correct looks like: Update the module doc: target landed (5.Q1),
registry-checked; campaign execution still pending nightly wiring.
Confidence: HIGH — Verification: `ls fuzz/fuzz_targets/` + read of target
file + baseline `full-gate.log.txt:46-48`.

### F-003 [S4]
Where: crates/demux/src/wav/fuzz.rs:37-41 and 65-68
Code:
```rust
/// Maximum events drained per pass `[PROPOSAL: 200_000 — two orders
/// of magnitude above the largest sane WAV's packet count at the
/// 64 KiB packet budget; the walk's monotonicity makes hitting this a
/// bug, not media]`.
```
```rust
    loop {
        if drained >= FUZZ_EVENT_BUDGET {
            return;
        }
```
What is wrong: The doc declares that hitting the budget "is a bug, not
media" — but the harness silently `return`s on budget exhaustion. No
assert, no log, no distinguishable state: neither the deterministic sweeps
nor a future libFuzzer campaign can observe that the predicted bug
happened (libFuzzer only reports panics/crashes/timeouts, and the budget
prevents the timeout).
Why it matters: A non-monotone walk regression (the exact bug class the
budget guards) would pass every sweep green while silently truncating
every fuzz pass at 200k events.
What correct looks like: In `#[cfg(debug_assertions)]` builds (or behind a
fuzzing cfg), panic on budget exhaustion — turning the predicted bug into
a detectable one.
Confidence: HIGH — Verification: read (control flow); no run needed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FUZZ_EVENT_BUDGET` / `FUZZ_POST_SEEK_DRAIN` / `FUZZ_SEEK_PROBES` | 37-48 | REAL | S4 (F-003) | Budgets documented; exhaustion undetectable (F-003). |
| `fuzz_demux_wav` | 53-115 | REAL | — | Construct→start→drain→seek-probe loop; errors swallowed by contract (typed errors are the demuxer's job to pin); probes at 0/last/2last/3last/4last ticks (span=4×last, quarter 0..=4); `let _ = landing` (:105) is a dead store; `TimeBase::new(1, rate)` guards rate 0 twice (rate==0 already refused at parse). Sound and bounded. |
| `tests::Scrambler::{next_u64, flip_byte}` | 124-144 | REAL | — | Deterministic xorshift64*; modulo indexing over data.len — fine. |
| `empty_and_tiny_inputs_survive` | 146-152 | REAL | — | Empty, 2-byte, 12-byte-exact-head, 4 KiB 0xff. |
| `scrambled_structured_files_survive` | 154-168 | REAL | — | 64 severity levels × 1 flip each over a 1 KiB-payload WAV — real coverage of the walk under bit corruption. |
| `scrambled_extensible_headers_survive` | 170-215 | REAL | — | 40-byte extensible shape scrambled ×64 + 9 fmt-region truncations. Strong. |
| `adversarial_size_fields_survive` | 218-240 | REAL | S3 (F-001) | RIFF-size patches (incl. the 0xFFFFFFFF sentinel) and fmt-size patches are correctly sited; the data-size patches are mis-sited no-ops (F-001). |
| `truncations_at_every_header_site_survive` | 242-250 | REAL | — | 13 cut points across head/fmt/data. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 5 | ONLY on panic/hang — which IS the contract | These are survival sweeps, not conformance asserts (module doc is explicit). Mutation: an `unwrap` on short fmt → `scrambled_extensible…` panics; an unbounded walk → budget returns silently (F-003) — the one bug class these tests cannot catch. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "+FB fuzz harness" (container-wav evidence) | docs/feature-matrix.toml:229; docs/formats/container-wav.md:72-75 ("fuzz-harness survival") | REAL (existence + sweeps) / UNPROVEN (campaign) | Harness + target + registry-check real; 5 sweeps ran ok in baseline; no libFuzzer campaign has ever run (nightly wiring pending, fuzz/README LEAD-DECISION). |
| "until it lands the harness logic is proven by the deterministic in-crate sweeps" | fuzz.rs:14-18 | STALE | The target HAS landed (F-002). |
| "the walk's monotonicity makes hitting this a bug, not media" | fuzz.rs:37-41 | UNVERIFIABLE by this harness | Budget hit is silent (F-003). |

## 7. Dead / duplicated / unreachable code
- `let _ = landing;` (:105) — dead store silencing an unused variable.
- Scrambler duplicates the AVI fuzz harness's scrambler (documented "the AVI
  harness's shape") — third copy of xorshift64* in the crate family
  (fixtures.rs has another). Test-only; noted for the wave synthesizer.

## 8. Dependencies & flags
No deps beyond the demux crate itself + plugin_api/media_types; test module
uses only test_support. No unsafe, no feature gates. The libFuzzer target
lives outside the workspace (fuzz/Cargo.toml, stable-checkable per its
header).

## 9. Open questions for the next agent
- avi/fuzz.rs (A01): does its adversarial-patch test have the same
  mis-sited-offset class of defect as F-001? Check every `// x (at N + 4)`
  comment against the builder's real layout.
- Wave gate: REQUEST-ORCH-RUN not needed — the campaign gap is a documented
  Lead decision, not a run I need.
