# AUDIT: crates/media-probe/src/lib.rs (file: 445 lines)
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-21 |
| Lines / functions | 445 / 14 units (crate doc + doctest, mod decls/re-exports, `probe_stream`, `fill_window`, 2 test helpers, 9 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `media-probe` is a dependency of `apps/cli` (Cargo.toml:56) and `apps/web` (:63); `probe_stream` is the open-path step called at `apps/cli/src/demux_host.rs:447`. Executed in baseline: doctest green (orchrun-test-workspace-nofailfast.log.txt:6437 `crates/media-probe/src/lib.rs - (line 85) ... ok`), all 9 `tests::*` green (:3311-3320 region, listed §5), and the MP4/AVI smoke lanes ran through this crate (INDEX.md). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `Read` full file (twice, both halves); `sed -n 3309,3400p` + `grep` on the baseline media-probe section (80 lib tests green incl. all 9 `tests::*`); `sed -n 6434,6440p` baseline (doctest ok); `rg -n "contract" apps/cli/Cargo.toml apps/web/Cargo.toml crates/media-probe/Cargo.toml` (feature declared, NOT enabled by either app — plain path deps; house-rule claim holds); `rg -n "media-probe" apps/*/Cargo.toml` (reachability); `rg -n "with_builtins" apps/` (call sites from signature/mod.rs audit); `sed -n` on every quoted range. |

## 1. One-paragraph verdict
The crate root is a thin, honest orchestration: `probe_stream` requires
seekability, fills a bounded window from the current position (no hidden
seek-to-start — pinned by test), scores it purely, and restores the
position; `fill_window` asks for exactly the remaining budget each read,
stops at EOS, and defensively clamps contract-violating over-serves. The
test suite is the strong kind: an instrumented real-trait source
(`CountingSource`) proves depth bounds, position restoration, non-consumption
by replay, exact rung boundaries (90/80/70/typed-unknown at 36/35/12/11
bytes), short-read robustness, unseekable refusal-before-any-read, and
error propagation — all green in the baseline, plus a doctest that runs
the same shape through an inline `SourceStream` impl. Two findings: the
crate doc repeats the registry's false "every landed container" claim
(the ASF gap — functional half filed against signature/mod.rs), and the
doctest-vs-instrumented-source parity pin, while real, documents a past
audit-driven rewrite whose "audit F-2" reference is unverifiable from the
tree (provenance nit only).

## 2. What actually works (verified)
— Doctest green: baseline :6437 (pure probe + stream probe + position
restore + non-consumption replay, all asserted).
— All 9 `tests::*` green (baseline media-probe lib section):
`depth_bound_holds_for_every_window_size`, `doctest_shape_probes_like_the_instrumented_source`,
`garbage_windows_are_low_confidence_rejections`,
`non_seekable_streams_are_a_typed_refusal`, `probe_stream_scores_and_restores`,
`probing_from_a_nonzero_position_scores_that_window`,
`read_failures_propagate_from_the_stream`, `short_reads_are_robust`,
`window_bound_is_exact`.
— House rules verified for this file's production code (lines 159-236):
zero `unwrap`/`expect`/`panic!`/`todo!`/`unsafe` (only `unwrap_or` at
:220, which is a total function); the `contract` module is
`#[cfg(any(test, feature = "contract"))]` (:160-161) and neither shipped
app enables the feature (Cargo.toml greps) — the "release binaries carry
none of its panicky conformance vocabulary" claim holds structurally.
— Bound tracing: `fill_window` allocates `with_capacity(hint ≤ max)` and
grows the window by at most `want = max - len` per iteration; loop exits
at `len == max` or EOS — no unbounded path, no spin (empty chunk breaks).
— The over-serve clamp (:232-233) is memory-safe AND position-safe: the
probe restores the pre-read position afterwards (:207-209), so discarded
over-serve bytes cannot corrupt the demuxer's subsequent view.

## 3. Findings

### F-001 [S3]
Where: crates/media-probe/src/lib.rs:21-26
Code:
```rust
//! - [`signature`] — the [`ProbePlugin`](signature::ProbePlugin) seam
//!   (doc 03 §4.2's trait) and the
//!   [`ProbeRegistry`](signature::ProbeRegistry) that scores
//!   signatures and applies the selection rules; the builtin AVI and
//!   MP4 signatures with their documented scoring ladders, plus the
//!   Phase-5 R3 consolidation rows (card 5.R3-REG): WAV, AIFF, Ogg,
//!   Matroska/WebM, ADTS/ADIF, MPEG-TS, MPEG-PS and IVF — every
//!   landed container, magic-anchored, mirror-pinned to its
//!   demuxer's own probe gate by the round-trip suite.
```
What is wrong: "every landed container" is false — the ASF demuxer landed
(`crates/demux/src/asf/**`, 36 tests green in baseline) but no ASF
signature exists (rg -i "asf|wmv|wma" over this crate: zero matches), so
ASF is neither magic-anchored nor mirror-pinned; the shipped open path
refuses every .wmv/.wma/.asf with `probe.no-demuxer`.
Why it matters: the crate-level doc is the first thing a maintainer reads;
it asserts a completeness the registry does not have, which is exactly how
the ASF wiring gap stayed unfixed (the repo's own compat-matrix triage in
docs/feature-matrix.toml:217 dates from 2026-11-10 and the doc still
overclaims today).
What correct looks like: scope the claim to the 14 actually-registered
formats, or land the AsfSignature and make it true.
Confidence: HIGH — Verification: full evidence chain in my
signature/mod.rs audit F-001 (S2 there — the functional half); this file's
finding is the duplicated false claim only, hence S3.

### F-002 [S4]
Where: crates/media-probe/src/lib.rs:243-246
Code:
```rust
    /// The doctest's inline in-memory source, against the crate's own
    /// instrumented source: both must probe identically (audit F-2's
    /// doctest rewrite keeps the example honest — this pin keeps the
    /// crate honest about it).
```
What is wrong: The comment cites "audit F-2's doctest rewrite" — an
external audit artifact not present in the tree. The rewrite's existence
and content are unverifiable from the repository (no audit/F-2 file; the
wave-0 audit tree lives outside the repo proper).
Why it matters: Provenance-by-reference to a nonexistent artifact is the
same class as citing a doc that does not say what is claimed — a reader
cannot check what the rewrite changed or why.
What correct looks like: describe the invariant in-place ("the doctest
example and the instrumented contract source must produce the same
report") without the phantom citation, or commit the referenced audit
note.
Confidence: HIGH — Verification: read + rg for "audit F-2"/"F-2" in the
crate (only this comment); UNVERIFIED what the original audit said —
that is the point.
Blast radius: one comment.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| crate doc | 1-79 | REAL | S3 | F-001 (false completeness claim); all other design claims verified (pure scoring :40-43 ✓, non-consumption :44-52 ✓ test-pinned, one-shot :53-56 ✓ no re-probe loop in code, extension-as-hint :57-59 ✓ — extension only in error payload, house rules :71-79 ✓ for this file) |
| doctest | 85-157 | REAL | — | real `SourceStream` impl (Window) with correct seek clamping; asserts probe + 90 rung + restore + replay; ran green :6437 |
| mod decls / re-exports | 159-171 | REAL | — | `contract` properly cfg-gated; re-export surface matches the doc's module map |
| `probe_stream` | 193-211 | REAL | — | seekability gate before ANY read (test: total_served 0 on refusal); position-honest (no seek-to-0; pinned by nonzero-position test); restore only-if-moved; errors propagate with correct stages (Probe for policy, Open for source) |
| `fill_window` | 216-236 | REAL | — | exact-budget asks; EOS one-shot break; over-serve clamp (documented :230-232); capacity hint ≤ max; no spin; ceiling enforcement delegated to ProbeConfig validation (config.rs — my file 7; constructor-validated there per config tests :4-8 green) |
| test helpers `ok`/`avi_header` | 268-280 | REAL | — | trivial, honest |
| `doctest_shape_probes_like_the_instrumented_source` | 247-266 | REAL | S4 | F-002 (phantom "audit F-2" citation); the parity pin itself is real and green |
| `probe_stream_scores_and_restores` | 282-297 | REAL | — | 90 rung + bytes_examined + restore + depth bound + replay-proof |
| `window_bound_is_exact` | 299-326 | REAL | — | the ladder-boundary test: 90 at 36B, 80 at 35B (one byte short of avih), 70 at 12B (magic+form), typed unknown at 11B — exact |
| `depth_bound_holds_for_every_window_size` | 328-361 | REAL | — | 8 bounds × (bound + restore) asserts; EOS case asserts exactly 2 requests ("fill + EOS probe only") — no retry spin |
| `short_reads_are_robust` | 363-375 | REAL | — | 7-byte chunk limit; every request ≤ 4096 asserted via instrumentation |
| `non_seekable_streams_are_a_typed_refusal` | 377-390 | REAL | — | refusal BEFORE any read (total_served == 0) — the strong form of the claim |
| `read_failures_propagate_from_the_stream` | 392-403 | REAL | — | stage `Open` (access taxonomy), scripted code passthrough |
| `garbage_windows_are_low_confidence_rejections` | 405-424 | REAL | — | 5 near-misses (off-by-one EBML, RIFX, broken ftyp) all typed-refused |
| `probing_from_a_nonzero_position_scores_that_window` | 426-444 | REAL | — | probes the window AT 40, restores 40 — kills any hidden seek-to-start policy |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 9 | YES | mutations reasoned: (a) remove the position restore → `assert_position_restored` + replay asserts fail; (b) read past the bound → `assert_probe_depth_bounded` and the `requests() ≤ bound` assert fail; (c) retry-on-EOS → the 2-request assert fails; (d) seek-to-start policy → nonzero-position test fails; (e) drop the over-serve clamp → a contract-violating source could grow the window (not directly tested — instrumentation gap noted, but the clamp is defensive-only); (f) score-rung drift → window_bound_is_exact's 90/80/70 ladder asserts fail. Subject exercised for real (probe_stream over CountingSource — a real-trait instrumented double, not a mock of the subject). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Never probes unbounded data … no code path that reads past it" | lib.rs:6-8 | REAL | fill_window :224-234 traced; depth tests green |
| "The probe never consumes stream bytes … position is restored" | lib.rs:44-47 | REAL | :207-209 + restore/replay tests green |
| "One-shot probing in v1 … no re-probe loop" | lib.rs:53-56 | REAL | no loop in probe_stream; single fill_window call |
| "The extension is a hint, not truth … surfaces only in the no-hit error payload" | lib.rs:57-59 | REAL | extension read only via SourceMeta; routing never consults it (demux_host.rs:443-451 trace) |
| "Std-only, zero external dependencies" | lib.rs:73 | REAL | Cargo.toml [dependencies] empty except workspace crates (media-types/plugin-api are workspace-internal) |
| "no unwrap/expect/panic!/todo! in non-test code" | lib.rs:74-75 | REAL | verified for :159-236 by read; crate-wide clippy `-D warnings` clean (baseline #5/#6) |
| "every landed container" probed | lib.rs:24-26 | **FALSE** | F-001 (ASF) |

## 7. Dead / duplicated / unreachable code
- None in this file. (The unreachable-by-shipping ASF demuxer consequence
  is documented under signature/mod.rs F-001.)

## 8. Dependencies & flags
- `media_types`, `plugin_api` (workspace); zero external crates — matches
  the C5 house rule.
- `contract` feature: declared `[]` (Cargo.toml:36), enabled only under
  test — verified neither app passes it.

## 9. Open questions for the next agent
- A09 (contract.rs): the depth/restore assertion helpers this file's
  tests lean on — verify they fail loudly on violation (your test
  `depth_assertions_fail_loudly_on_violations` suggests they do — green
  :3318).
- A04 (report.rs): `SourceMeta::from_stream` — confirm it never reads
  stream bytes (metadata getters only); the non-consumption guarantee
  leans on it.
