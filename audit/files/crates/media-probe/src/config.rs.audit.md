# AUDIT: crates/media-probe/src/config.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-23 |
| Lines / functions | 153 / 10 (2 impl fns, 3 consts, 5 tests) |
| File verdict | REAL (validation machinery is test-only in production) |
| Reachable from shipped entry point? | YES — apps/cli/src/demux_host.rs:445 `ProbeConfig::default()` → `probe_stream` (CLI open path); apps/web/src/open.rs:231 + staged_source.rs:1198 (web). `min_score` consumed at signature/mod.rs:201 `confidence.meets(config.min_score)`; `max_lookahead` consumed at lib.rs:203 `fill_window`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `cargo test -p media-probe --lib config::` (exit 0, 5/5 ok); rg sweeps: ProbeConfig construction sites, `.meets(`, `lookahead|min_score` over *.rs/*.toml/*.ts/*.tsx, `probe_max_bytes` consumers (exit 0); sed quote checks (exit 0) |

## 1. One-paragraph verdict
The struct is small and its two fields are genuinely consumed on the
shipped probe path, with defaults matching the documented constants and
real, passing boundary tests. Two design-level defects: (1) the doc's
"[PROPOSAL, overridable from product.toml resource limits]" is unimplemented
— every production construction is `ProbeConfig::default()` (8 sites, all
default), no bridge reads any product.toml limit into this config, and the
one product.toml knob that does exist (`limits.probe_max_bytes`, default
16 MiB, schema range 1..=64 MiB) is parsed, validated and UI-surfaced but
never consumed by the probe — and its default exceeds this file's 1 MiB
hard ceiling, so wiring the proposal as-is would fail validation at
startup; (2) validation lives only in `new()` while the struct has public
fields, so `ProbeConfig { max_lookahead: <anything>, .. }` compiles and
`fill_window` enforces no ceiling of its own — the MP §12 memory bound is
advisory. No shipped caller bypasses it today.

## 2. What actually works (verified)
— `cargo test -p media-probe --lib config::` → 5/5 passed (defaults, boundaries 1/0 and ceiling/100, three typed-error shapes).
— min_score consumed: signature/mod.rs:201 `confidence.meets(config.min_score)`; meets is `>=` (report.rs:192-193). CONFIDENCE_MAX = 100 (report.rs:21).
— "single reader" claim: `fill_window` (lib.rs:216-236) is the only reader, honors the budget with a defensive clamp (`chunk.len().min(want)`), and contract.rs:325 `assert_probe_depth_bounded` is used by tests to pin total reads ≤ max_lookahead.

## 3. Findings
F-001 [S3]
Where: crates/media-probe/src/config.rs:4-9 (against product.toml:205, product-config/src/model.rs:178, ui schema.ts:2091)
Code: (verbatim)
```
//! Defaults are the doc's own values, both `[PROPOSAL, overridable from
//! product.toml resource limits]` in the spec: `max_lookahead = 64 KiB`
//! hard-capped at 1 MiB, `min_score = 60`. Construction validates —
```
What is wrong: the override path does not exist. Every production construction is `ProbeConfig::default()` (apps/cli/src/demux_host.rs:445, apps/web/src/open.rs:231, apps/web/src/io/staged_source.rs:1198, tools/mem-audit ×2, tools/sweep/src/worker.rs:296, tools/bench-vs-vlc/src/lib.rs:263); `ProbeConfig::new` with non-default values appears only in tests. Meanwhile product.toml:205 declares `probe_max_bytes = 16777216` ("schema 1 MiB..=64 MiB"), which product-config parses (model.rs:178, merge.rs:441) and the UI settings schema surfaces (ui/src/screens/settings/generated/schema.ts:2091 `defaultRef: "limits.probe_max_bytes"`) — but no code bridges it to ProbeConfig, and its 16 MiB default would be rejected by `new()` (ceiling 1 MiB, config.rs:56-57). Why it matters: a UI-visible knob wired to nothing, a proposal that cannot be wired without changing one of the two contradictory bounds, and three typed-error constructors that can never fire in the shipped product. What correct looks like: bridge limits.probe_max_bytes → ProbeConfig (and reconcile 16 MiB default vs 1 MiB ceiling) or drop the knob. Confidence: HIGH — Verification: ran (rg sweeps).

F-002 [S3]
Where: crates/media-probe/src/config.rs:35-41 (against lib.rs:216-236)
Code: (verbatim)
```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeConfig {
    /// The bounded header window, in bytes (1..=[`MAX_LOOKAHEAD_CEILING`]).
    pub max_lookahead: usize,
    /// The minimum score a hit needs to select a demuxer (0..=100).
    pub min_score: u8,
}
```
What is wrong: the invariants in the field docs (1..=1 MiB, 0..=100) are enforced only by `new()`; the fields are public and the struct is not `NonZeroUsize`-based or `#[non_exhaustive]`, so `ProbeConfig { max_lookahead: 100 << 20, min_score: 255 }` compiles, and `fill_window` would then allocate and read up to that bound — the 1 MiB MP §12 ceiling exists nowhere on the read path. Why it matters: the module doc's "never silent clamps" promise is advisory; one forgetful caller converts a documented hard cap into an unbounded read. No shipped caller bypasses `new()`/`default()` today (rg: zero literal constructions). What correct looks like: private fields + `new()`/`Default` only, or a ceiling re-check in `fill_window`. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DEFAULT_MAX_LOOKAHEAD | 18 | REAL | — | 64 KiB; matches doc; asserted by defaults test |
| MAX_LOOKAHEAD_CEILING | 23 | REAL | — | 1 MiB; enforced only in new() (F-002); contradicts product.toml probe_max_bytes default (F-001) |
| DEFAULT_MIN_SCORE | 27 | REAL | — | 60; consumed at signature/mod.rs:201 |
| ProbeConfig::new | 52-66 | REAL | — | 3 typed validations, all test-only in production (F-001); boundary tests pass |
| Default::default | 71-76 | REAL | — | Uses the DEFAULT_* consts; in range by construction |
| tests: defaults_are_the_doc_values | 100-104 | REAL | — | Fails if constants drift |
| tests: boundaries_construct | 107-117 | REAL | — | (1,0) and (ceiling,100) both accepted |
| tests: zero_lookahead_is_a_typed_error | 120-125 | REAL | — | Stage/code/kind asserted |
| tests: over_ceiling_lookahead_... | 128-144 | REAL | — | Full ResourceLimit destructuring incl. budget/attempted |
| tests: out_of_range_min_score... | 147-151 | REAL | — | Stage/code asserted |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| defaults_are_the_doc_values | YES | Exact 64*1024/60 equality |
| boundaries_construct | YES | Both extremes must construct and round-trip fields |
| zero_lookahead_is_a_typed_error | YES | Exact stage + code string + kind match |
| over_ceiling_lookahead_is_a_typed_probe_depth_limit | YES | Destructures ResourceLimit, asserts limit/budget/attempted |
| out_of_range_min_score_is_a_typed_error | YES | Exact stage + code |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Defaults ... `max_lookahead = 64 KiB` hard-capped at 1 MiB, `min_score = 60`" | config.rs:4-6 | REAL | consts 18/23/27 + defaults test executed |
| "[PROPOSAL, overridable from product.toml resource limits]" | config.rs:4-5 | PAPER-ONLY | No bridge exists; only default() constructions in production; the toml knob probe_max_bytes is unconsumed and contradicts the ceiling (F-001) |
| "Construction validates — ... typed errors at the boundary, never silent clamps" | config.rs:6-9 | REAL (for new()) | Tests executed; bypassable via pub fields (F-002) |
| "Probing reads **only** up to `max_lookahead` bytes ... the fill loop ... is the single reader" | config.rs:31-34 | REAL | lib.rs:203/216-236 read; contract.rs:325-345 pins total ≤ bound in tests |

## 7. Dead / duplicated / unreachable code
- No dead code in this file. Adjacent dead knob (owned by product.toml/product-config, noted for cross-reference): `limits.probe_max_bytes` parsed + UI-surfaced, never consumed (F-001).

## 8. Dependencies & flags
- media-types (EngineError), crate::error (3 constructors), crate::report::CONFIDENCE_MAX. No feature gates, no unsafe.

## 9. Open questions for the next agent
- Whoever owns product-config/UI: is `limits.probe_max_bytes` (schema 1 MiB..=64 MiB, default 16 MiB) intended to become max_lookahead? As written the two bounds are irreconcilable without an edit to one of them.
- If `#[non_exhaustive]`/private fields are ever added, re-check the tools/ constructors (they all use default(), so no breakage expected).
