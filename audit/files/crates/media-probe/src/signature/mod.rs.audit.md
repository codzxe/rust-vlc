# AUDIT: crates/media-probe/src/signature/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-02-14 |
| Lines / functions | 517 / 22 (trait + registry: 6 prod fns + better_tiebreak + 10 tests + 4 test helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: lumen-cli `apps/cli/src/demux_host.rs:444-448` -> `media_probe::probe_stream(&registry, ...)` -> `ProbeRegistry::probe` (this file :175-213) on every `lumen open` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `./target/debug/deps/media_probe-440f45029f776138 --test-threads=1 signature::` -> 50 passed (13 in this mod + 37 submodule), exit 0; sed quote verifications (byte-exact) |

## 1. One-paragraph verdict
The selection engine is real and live-verified: pure scoring over
`(prefix, meta)`, out-of-range scores genuinely discarded (verified
`Confidence::from_score` returns None above the cap — no clamping, matching
the doc), tie-break chain score→priority→shorter-prefix implemented exactly
as documented and pinned by dedicated Fixed-plugin tests, sub-threshold
refusals carry the best-known family name in a typed `probe.no-demuxer`
error with the sniffed bytes, and the built-in 14-signature registry is
panic-swept over 64×256 byte mutations plus all truncations. All defects are
hygiene: `bytes_examined` reports the window length instead of what the
winner consumed, `prefix_consumed` is trusted unpoliced (a lying plugin wins
tie-breaks), MAX_WALK_STEPS=256 can silently degrade hints on legitimately
fragmented files, and one test comment claims "two template windows" where
the code builds one.

## 2. What actually works (verified)
— verified by live run `./target/debug/deps/media_probe-... signature::` ->
"test result: ok. 50 passed", exit 0. Tie-break tests cover all three levels
(score over priority, priority over prefix, prefix final). The out-of-range
test covers both the liar-loses and liar-only-unnamed branches.
`Confidence::from_score` (report.rs:178-184) confirmed non-clamping by read.

## 3. Findings
F-001 [S4]
Where: crates/media-probe/src/signature/mod.rs:200-206
Code:
```
        match best {
            Some((hit, _, confidence)) if confidence.meets(config.min_score) => Ok(ProbeReport {
                format: hit.format,
                confidence,
                track_hints: hit.track_hints,
                bytes_examined: prefix.len(),
```
What is wrong: `bytes_examined` is the whole probe window handed in, not the
bytes the winning signature actually examined — `ProbeHit.prefix_consumed`
is computed by every signature, selected for in tie-breaks, then thrown
away. A 64 KiB window on a file whose winner anchored in 12 bytes reports
65536. Why it matters: the field is a metrics surface (`lumen probe`
output); it overstates work done and cannot detect a signature that anchors
but walks nothing. What correct looks like: `hit.prefix_consumed` (or
max across hits).
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/demux/../media-probe/src/signature/mod.rs:216-222
Code:
```
fn better_tiebreak(new: &ProbeHit, new_priority: u32, old: &ProbeHit, old_priority: u32) -> bool {
    new.score > old.score
        || (new.score == old.score
            && (new_priority > old_priority
                || (new_priority == old_priority && new.prefix_consumed < old.prefix_consumed)))
}
```
What is wrong: the registry polices `score` (range-checked via
`Confidence::from_score`) but trusts `prefix_consumed` unconditionally — a
plugin reporting `prefix_consumed: 0` wins every full tie dishonestly, and
nothing validates `prefix_consumed <= prefix.len()`. The module doc promises
the doc 06 §7 conformance kit will police plugins "once extension loading
exists"; today the wasm plugin dir (plugins/extensions/) exists but this
policing does not. Why it matters: the tie-break key is attacker-controlled
by any future third-party signature. What correct looks like:
`prefix_consumed.min(prefix.len())` at collection, like the score check.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/media-probe/src/signature/mod.rs:68-72
Code:
```
/// Iteration guard for one chunk/box walk (a corrupted size table
/// cannot spin the scanner; sizes that stall progress end the walk).
/// `[PROPOSAL]` — bounded work is the contract; the figure is a
/// safety net far above any real header's child count.
pub(crate) const MAX_WALK_STEPS: usize = 256;
```
What is wrong: "far above any real header's child count" is unproven for
fragmented MP4: a 64 KiB lookahead window can hold >256 top-level boxes
(moof+mdat pairs of a few hundred bytes each), at which point the walk stops
before reaching a trailing `moov`. Failure mode is degraded (hints/confidence
drop from 90 to the 70 anchor — still above min_score 60, selection
survives), so blast radius is hints, not correctness. What correct looks
like: raise the cap or make it window-relative (steps ≤ window/8), and pin
the fMP4 shape with a test.
Confidence: MEDIUM — Verification: read; fMP4 >256-box scenario not
constructed (UNVERIFIED end-to-end).

F-004 [S4]
Where: crates/media-probe/src/signature/mod.rs:472-474
Code:
```
        // Doc 03 §4.2(f): random prefixes must never panic any probe
        // plugin. Deterministic sweep: every byte value at every
        // position of two template windows, plus every truncation.
```
What is wrong: the comment claims "two template windows"; the code builds
ONE 64-byte template (line 478) carrying three magics. The sweep is still
real (64×256 mutations + 65 truncations, all 14 builtins), but the comment
misdescribes it — stale from an earlier revision. What correct looks like:
"one 64-byte template" or restore the second window.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ProbePlugin::extensions (default) | 55-57 | REAL | — | default empty; overridden by builtins |
| ProbePlugin::probe (trait) | 65-65 | REAL | — | pure contract; impls in avi/mp4/phase5/es |
| MAX_WALK_STEPS / score consts | 68-96 | REAL | S4 (F-003) | ladder values pinned by submodule tests |
| ProbeRegistry::empty | 114-118 | REAL | — | trivial |
| ProbeRegistry::with_builtins | 130-147 | REAL | — | 14 signatures; count pinned by test |
| ProbeRegistry::register | 150-152 | REAL | — | no dedup; fine for one-shot build |
| ProbeRegistry::len / is_empty | 155-162 | REAL | — | honest counters |
| ProbeRegistry::probe | 175-213 | REAL | S4 (F-001, F-002) | selection pass; range policing via Confidence; error path carries best hit |
| better_tiebreak | 217-222 | REAL | S4 (F-002) | exact doc-03 chain; full-tie keeps first (deterministic) |
| tests: 10 fns + Fixed/ok/registry/config helpers | 224-517 | REAL | S4 (F-004) | all pass live; honest machinery pins |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| higher_score_wins_over_priority | YES | score 90/prio 1 beats 70/prio 999 |
| priority_breaks_score_ties | YES | 20 beats 10 at equal score |
| shorter_prefix_breaks_priority_ties | YES | 10 beats 50 at equal score+prio |
| subthreshold_best_hit_names_the_family_in_the_error | YES | asserts code, stage, kind, name, sniffed bytes, error source chain |
| out_of_range_scores_are_discarded_not_clamped | YES | liar(101) neither wins nor is named |
| empty_registry_is_a_clean_unknown | YES | typed no-demuxer |
| builtin_registry_shape | YES | len == 14 (would catch a dropped signature) |
| builtin_signatures_never_panic_on_arbitrary_windows | YES (panic contract) | 16k+ probes; asserts nothing about results by design |
| track_hints_flow_from_hit_to_report | YES | hints + bytes_examined==1 pinned |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A hit whose score exceeds 100 ... is discarded, never clamped" | mod.rs:11-14 | REAL | report.rs:178-184 returns None; test at :398-447 |
| "The winner is the hit with the highest score; ties break by plugin priority ..., then by the shorter prefix consumed" | mod.rs:9-11 | REAL | better_tiebreak:217-222 + 3 tie tests, live |
| "a safety net far above any real header's child count" (256) | mod.rs:70-71 | REAL-UNPROVEN | fMP4 >256-box window not constructed (F-003) |
| "the doc 06 §7 conformance kit will flag such plugins loudly once extension loading exists" | mod.rs:13-14 | PAPER-ONLY | no plugin policing exists in media-probe today; prefix_consumed unpolicied (F-002) |

## 7. Dead / duplicated / unreachable code
- None found. `SCORE_FAMILY_ONLY` (:93) — used by submodule signatures
  (verified in the 37 passing submodule tests).

## 8. Dependencies & flags
media_types, plugin_api (manifest priority const), crate::config/report/error.
No unsafe, no feature gates, std-only.

## 9. Open questions for the next agent
- signature/avi.rs, mp4.rs, phase5.rs, es.rs (not assigned to me): verify
  each walk actually honors MAX_WALK_STEPS and clamps to the window on
  truncated prefixes — the no-panic sweep is smoke-level, and the
  hostile-focus truncation proofs live there and in tests/probe_io.rs.
- Is `prefix_consumed` ever surfaced anywhere (report/metrics)? rg suggests
  only tie-break — if truly unused elsewhere, F-001 is the only consumer gap.
