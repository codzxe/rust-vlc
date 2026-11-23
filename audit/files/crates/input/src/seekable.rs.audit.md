# AUDIT: crates/input/src/seekable.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-23 |
| Lines / functions | 126 / 4 (2 traits, 1 shared fn, 1 test) |
| File verdict | REAL (small, correct, reachable trait surface; duplicated elsewhere by necessity) |
| Reachable from shipped entry point? | YES — `Seekable`/`RawRead` are the generics `BufferedReader` wraps (crates/input/src/buffered.rs:38,76) → `FileSource`/`FileStream` (file.rs) → `FileAccess` → apps/cli/src/demux_host.rs:392-397 (the `SourceSpec::File` arm of `open_source_with`) → shipped `lumen open <path>`. `resolve_origin` is called on the shipped seek path at crates/input/src/file.rs:231. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |

## 1. One-paragraph verdict
A 100-line trait-mint that does exactly what its docs say: `Seekable` (typed seeks, `Option<u64>` length), `RawRead` (chunked reads, `Ok(0)`-only-at-end), and `resolve_origin` — an i128-checked `SeekOrigin` resolver whose math is boundary-tested (negative → `input.seek.before-start`, past-u64::MAX → `input.seek.overflow`, the exact `u64::MAX` boundary resolves). The one test passes (I ran it). The defects are structural, not logical: the shared helper is `pub(crate)`, which FORCES two byte-level duplicate implementations elsewhere in the tree (streaming/src/ranged.rs:255 and streaming/src/hls/media.rs:366 reimplement it because they cannot see this one), and the `SeekOrigin` vocabulary has no `End` variant, so end-relative seeks are every caller's manual `length()` arithmetic.

## 2. What actually works (verified)
— `cargo test -p input --lib seekable` → `1 passed; 0 failed`. The boundary test covers Start, Current±, negative refusal, overflow refusal, and the exact u64::MAX resolve — it would fail if the i128 guard regressed.
— Reachability executed indirectly: the shipped file lane (demux_host file arm) is the same lane the baseline MP4 smoke walked (audit/runs/baseline, AGENT-SPEC §13), and every CLI seek on a local file flows through file.rs:231 → this function.

## 3. Findings
F-001 [S4]
Where: crates/input/src/seekable.rs:86 (vs crates/streaming/src/ranged.rs:255 and crates/streaming/src/hls/media.rs:366)
Code:
```
pub(crate) fn resolve_origin(from: u64, origin: SeekOrigin) -> Result<u64, EngineError> {
```
What is wrong: the "shared" helper is crate-private, so the streaming crate carries its own copy (`ranged.rs:255 fn resolve_origin` — same name, same job, used at ranged.rs:933 on the SHIPPED http lane) and hls/media.rs:366 a third method-shaped variant. Three implementations of seek-origin math that must stay in agreement; a fix to one (e.g. a policy change on the u64::MAX boundary) silently misses the others. The doc comment (:76-80) even narrates moving it here "so the module can be feature-gated" — the sharing stopped at the crate wall. What correct looks like: the helper belongs next to `SeekOrigin` in plugin-api (where `SourcePosition` already lives), exported once.
Confidence: HIGH — Verification: read all three sites (rg `resolve_origin`).
F-002 [S4]
Where: crates/input/src/seekable.rs:86-100 (vocabulary: crates/plugin-api/src/access.rs:89-94)
Code:
```
    match origin {
        SeekOrigin::Start(to) => Ok(to),
        SeekOrigin::Current(delta) => {
```
What is wrong: no `SeekOrigin::End` variant exists in the vocabulary (access.rs:89-94 defines only Start/Current) — the module doc's "shared seek semantics" cannot express POSIX SEEK_END, so every end-relative seek in the tree is hand-rolled `length()? + offset` arithmetic at call sites (and unknown-length sources make that impossible). Not this file's bug — the limitation is minted one layer down — but this file is where the semantics are declared.
Confidence: HIGH — Verification: read access.rs enum.
F-003 [S4]
Where: crates/input/src/seekable.rs:88
Code:
```
        SeekOrigin::Start(to) => Ok(to),
```
What is wrong: `Start` is returned unchecked — `Start(u64::MAX)` resolves to `Ok(u64::MAX)` and only the length-aware caller (file.rs:87-92 `to > len` → `input.seek.out-of-range`) catches it. Consistent with the doc ("sources whose length is unknown accept any target"), but it means the typed `input.seek.overflow` refusal exists only for the `Current` path; a `Start(u64::MAX)` on an unknown-length source reaches the underlying store as-is. Blast radius: the underlying I/O error is still typed. Note-level.
Confidence: HIGH — Verification: read + traced file.rs caller.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| trait Seekable (position/length/seek) | 37-54 | REAL | — | Option-length per MP §8; out-of-range policy lives in impls |
| trait RawRead (read) | 65-72 | REAL | — | Ok(0)-at-end contract; position-unchanged-on-Err |
| resolve_origin | 86-100 | REAL | S4 | F-001 crate-private → 2 duplicates; F-003 Start unchecked (by design) |
| resolve_origin_math_is_checked (test) | 111-125 | REAL | — | boundary-exact |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| resolve_origin_math_is_checked | YES | 6 assertions incl. both refusal codes and the exact u64::MAX boundary |
Single-test file — the trait contracts themselves are enforced by the contract module (feature-gated) and buffered.rs's own suite, both outside this file.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "used by the file access stack and the synthetic conformance source alike" | seekable.rs:76-78 | REAL | file.rs:231 + contract.rs:216 (rg) |
| "a value and its absence are different types" | seekable.rs:5-7 | REAL | `length() -> Option<u64>` |
| "audit F-2 moved it here … so the module can be feature-gated" | seekable.rs:78-80 | REAL | contract.rs now imports it (:64) |

## 7. Dead / duplicated / unreachable code
Duplicated: ranged.rs:255 and hls/media.rs:366 re-implement this helper (F-001). Nothing dead in this file.

## 8. Dependencies & flags
`media_types::EngineError`, `plugin_api::SeekOrigin`. No features. Clean minimal surface.

## 9. Open questions for the next agent
- ranged.rs (streaming): is its `resolve_origin` byte-identical in policy to this one (u64::MAX boundary, before-start code)? Divergence there is a live hazard for the http lane.
- plugin-api: any card planning `SeekOrigin::End`?
