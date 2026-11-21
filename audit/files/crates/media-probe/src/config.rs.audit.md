# AUDIT: crates/media-probe/src/config.rs (file: 152 lines)
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-21 |
| Lines / functions | 152 / 10 units (3 consts, `ProbeConfig` struct, `new`, `Default`, 2 test helpers, 5 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `ProbeConfig::default()` at `apps/cli/src/demux_host.rs:445` and `apps/web/src/open.rs` (the open path); executed in baseline: 5 `config::tests` green (orchrun-test-workspace-nofailfast.log.txt:3312-3316), and every MP4/AVI smoke probe ran under these defaults. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `Read` full file; `rg -n "ProbeConfig\s*\{" crates/ apps/` (no struct-literal construction in-tree — only `new()`/`default()`); `rg -n "probe_max_bytes" apps/` → zero; repo-wide `rg probe_max_bytes` (product.toml:205 + product-config parse/merge/model + settings schema + UI schema + W01's product.toml audit:283 "no consumer found … unenforced declarations"); `sed -n 236,241p docs/03-engine-plan.md` (spec wording verified — the pub fields are spec-mandated and the "[PROPOSAL, overridable …]" bracket is the spec's own); `sed -n 200,210p product.toml`; `sed -n` on every quoted range. |

## 1. One-paragraph verdict
Small, correct, well-tested config: the three doc constants are exactly the
spec's figures, `new()` refuses zero/over-ceiling lookahead and
out-of-range min_score with typed errors whose codes and kinds are pinned
by tests, boundary values (1, ceiling, 0, 100) construct, and the defaults
are asserted. Two drift findings, both rooted in the config story rather
than the arithmetic: the struct's `pub` fields (spec-mandated) make the
"hard cap" advisory-only — any struct literal bypasses every validation
and `fill_window` trusts the field blindly; and the "overridable from
product.toml resource limits" proposal is not implemented anywhere, while
the product.toml knob that DOES exist (`limits.probe_max_bytes`, schema
1..=64 MiB, surfaced in settings as `engine.probe-max-bytes`) has no
runtime consumer and is range-INCOMPATIBLE with this crate's 1 MiB
ceiling — wiring the override as documented would type-error on the
shipped 16 MiB default.

## 2. What actually works (verified)
— 5 tests green in baseline (:3312-3316): `boundaries_construct`,
`defaults_are_the_doc_values`, `out_of_range_min_score_is_a_typed_error`,
`over_ceiling_lookahead_is_a_typed_probe_depth_limit` (asserts the full
`ResourceLimit{limit: ProbeDepth, budget: 1 MiB, attempted: +1}` triple),
`zero_lookahead_is_a_typed_error`.
— Constants match doc 03 §4.2 verbatim (64 KiB / 1 MiB / 60 — cross-read
docs/03-engine-plan.md:239-240).
— Both shipped apps run `ProbeConfig::default()`; every smoke probe in the
baseline evidence ran under these bounds.

## 3. Findings

### F-001 [S3]
Where: crates/media-probe/src/config.rs:36-41 (with :52-58)
Code:
```rust
pub struct ProbeConfig {
    /// The bounded header window, in bytes (1..=[`MAX_LOOKAHEAD_CEILING`]).
    pub max_lookahead: usize,
    /// The minimum score a hit needs to select a demuxer (0..=100).
    pub min_score: u8,
}
```
What is wrong: Both fields are `pub`, so the validated constructor is
advisory: `ProbeConfig { max_lookahead: usize::MAX, min_score: 0 }` is a
legal expression that skips every check in `new()` (:53-61). The module
doc's "hard-capped at 1 MiB … typed errors at the boundary, never silent
clamps" is only true for callers who volunteer to use `new()`.
`fill_window` (lib.rs:216-236) trusts `config.max_lookahead` as its only
bound — a literal-constructed config removes the MP §12 memory bound
entirely (the window would grow to the source's full size).
Why it matters: The entire bounded-probe story rests on this one field; a
type that cannot enforce its own documented invariant invites the first
downstream "just construct it directly" commit to silently void the bound.
Mitigation found: zero in-tree struct-literal constructions (rg over
crates/ and apps/ — only `new()`/`default()` used; test helpers
probe_io.rs:90, fixtures_matrix.rs:144 both route through `new()`).
What correct looks like: private fields + `new()` + accessor methods (the
input crate's capacity pattern the doc itself cites), or at minimum a
`debug_assert` in `fill_window` re-checking the ceiling.
Confidence: HIGH — Verification: read + repo-wide construction grep.
Note: the `pub` shape is spec-mandated (doc 03:236 shows the exact struct),
so this is a faithfully-copied spec flaw — the finding stands against the
"hard-capped" language regardless.

### F-002 [S3]
Where: crates/media-probe/src/config.rs:4-6
Code:
```rust
//! Defaults are the doc's own values, both `[PROPOSAL, overridable from
//! product.toml resource limits]` in the spec: `max_lookahead = 64 KiB`
//! hard-capped at 1 MiB, `min_score = 60`. Construction validates —
```
What is wrong: The claimed override path does not exist. Repo-wide, the
only product.toml probe knob is `limits.probe_max_bytes` (product.toml:205,
value 16 MiB, schema 1 MiB..=64 MiB), parsed and merged by product-config
and surfaced to users as the settings key `engine.probe-max-bytes`
(schemas/settings.toml:1368, UI schema.ts:2091) — and it has **zero
runtime consumers** (`rg probe_max_bytes apps/` → no matches; W01's
product.toml audit already recorded "no consumer found … unenforced
declarations"). Nothing bridges product.toml to `ProbeConfig`; the apps
hard-run `default()`. Worse, the two systems contradict: this crate's hard
ceiling is 1 MiB, the settings knob's range is 1..=64 MiB — wiring the
override as documented would make the shipped default (16 MiB) a typed
`probe.lookahead-over-ceiling` error.
Why it matters: A user-facing settings control exists for a bound the
engine ignores, and the one proposal that would honor it is
range-incompatible with the enforcing validator — three artifacts
(product.toml, settings schema, ProbeConfig) telling three different
stories about the same number.
What correct looks like: either implement the bridge (and reconcile the
ranges — the ceiling must be ≥ the schema max for any override to be
legal), or delete the `engine.probe-max-bytes` settings row until the
override exists.
Confidence: HIGH — Verification: rg chains cited above; doc 03:239-241
confirms the bracket is the spec's proposal, so the file's quote is
accurate — the defect is the unimplemented (and unimplementable-as-ranged)
override plus the dead settings control it leaves behind.
Severity note: held at S3 for this file (the false-looking claim is a
faithful spec quote); the dead UI control itself is a settings-wave defect
to file against the settings wiring.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-9 | REAL | S3 | F-002 (override quoted, not implemented; range conflict) |
| `DEFAULT_MAX_LOOKAHEAD` | 16-18 | REAL | — | 64 KiB = doc value (verified vs doc 03:240) |
| `MAX_LOOKAHEAD_CEILING` | 20-23 | REAL | — | 1 MiB = doc value; enforced only via new() (F-001) |
| `DEFAULT_MIN_SCORE` | 25-27 | REAL | — | 60 = doc value |
| `ProbeConfig` struct | 35-41 | REAL | S3 | F-001 (pub fields bypass validation; spec-mandated shape) |
| `ProbeConfig::new` | 43-66 | REAL | — | three typed refusals, exact codes pinned by tests; no clamping (verified: every failure path returns, no min/max fixing) |
| `Default` | 69-77 | REAL | — | constants, in range by construction (comment accurate) |
| test helpers | 85-97 | REAL | — | ok/err panics loud |
| 5 tests | 99-151 | REAL | — | all green; boundary + typed-error shape asserted exactly (stage, code, kind, budget/attempted triple) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `defaults_are_the_doc_values` :100 | YES | pins 64 KiB/60 — constant drift fails it |
| `boundaries_construct` :107 | YES | 1/ceiling/0/100 must construct — an off-by-one in either bound check fails it |
| `zero_lookahead_is_a_typed_error` :120 | YES | code + stage + kind asserted |
| `over_ceiling_lookahead_is_a_typed_probe_depth_limit` :128 | YES | full ResourceLimit triple (limit kind, budget, attempted) asserted — a clamp would fail the Err match |
| `out_of_range_min_score_is_a_typed_error` :147 | YES | code asserted (kind not matched here — minor, covered by error.rs tests) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "max_lookahead = 64 KiB hard-capped at 1 MiB; min_score = 60" | config.rs:5-6 vs doc 03:239-240 | REAL | constants match; tests pin |
| "Construction validates … typed errors at the boundary, never silent clamps" | config.rs:7-9 | REAL via new() / advisory via literals | F-001 |
| "[PROPOSAL, overridable from product.toml resource limits]" | config.rs:4-5 | PAPER-ONLY (override) | F-002 — no bridge exists; settings knob dead |
| "Probing reads only up to max_lookahead bytes … no code path that probes more" | config.rs:31-34 | REAL (given a validated config) | fill_window trace (lib.rs audit); conditional on F-001's bypass not being used |

## 7. Dead / duplicated / unreachable code
- None in the file. (The dead `engine.probe-max-bytes` settings row is the
  F-002 artifact — lives in schemas/, not here.)

## 8. Dependencies & flags
- `media_types::EngineError`, `crate::error` constructors, `report::CONFIDENCE_MAX`
  — all workspace-internal; std-only; no feature gates; no unsafe.

## 9. Open questions for the next agent
- Settings/product-config wave: file the dead `engine.probe-max-bytes`
  control against its wiring (my F-002 documents the evidence; the row's
  own audit belongs to the settings owner).
- If the override is ever implemented, decide which range wins: the 1 MiB
  ceiling here or the 64 MiB schema max — they cannot both hold.
