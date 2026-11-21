# AUDIT: crates/demux/src/ps/error.rs
| Field | Value |
|---|---|
| Auditor | W05-A01 |
| Date | 2026-11-21 |
| Lines / functions | 150 / 9 (+2 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — every constructor has a live call site in `crates/demux/src/ps/mod.rs` (verified by rg: `not_ps` :942, `truncated` :437/485/501/520/526/539/566/581/722, `bad_pack` :432/452/463, `bad_pes_header` :272/280/282/529/535/572/602, `resync_failed` :831, `payload_too_large` :893, `not_started` :415/966/1118/1145, `seek_out_of_range` :1121/1126/1130/1148), and the demuxer is reachable via `apps/cli/src/containers/mod.rs:71` `"mpegps" => ps::open` → `ps.rs:144-148`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read of error.rs + call-site sweep `rg -n "error::(not_ps\|truncated\|bad_pack\|bad_pes_header\|resync_failed\|payload_too_large\|not_started\|seek_out_of_range)" crates/demux/src/ps/mod.rs`; `rg -n '"demux.not-started"' crates/demux/src/` (duplication pattern); `rg "ps.seek.out-of-range\|ps.resync-failed\|ps.payload-too-large"` repo-wide (trigger coverage); baseline `sed -n '1874,1875p' audit/runs/baseline/test-workspace.log.txt`. |

## 1. One-paragraph verdict
Clean, honest typed-error factory: 8 constructors, each one live call-site-verified, stable dotted codes, stage `Demux`, taxonomy kinds never redefined, and a unit test that pins every code and the Display format (executed in baseline). No fabrication surface — a file this small either mints what it claims or doesn't, and it does. Two hygiene defects only: the `seek_out_of_range` doc promises a refusal for targets "outside the SCR-observed extent" while the demuxer actually *clamps* past-extent (and below-extent) targets; and the `not_started`/`mint` pattern is copy-pasted across eight module error files with nothing but per-file tests keeping the codes identical.

## 2. What actually works (verified)
— All 8 constructors minted with stable codes and called from live paths (call-site list above; rg-verified).
— Unit tests executed and passed in baseline: `audit/runs/baseline/test-workspace.log.txt:1874-1875` (`ps::error::tests::errors_carry_demux_stage_and_stable_codes … ok`, `not_ps_sniff_is_capped_by_display … ok`).
— `not_ps`'s sniffed payload is bounded at the only call site: `mod.rs:939-942` reads a `[u8; 4]` head buffer and passes `&head[..got]` — the `sniffed.to_vec()` copy is ≤4 bytes, so the Display cap the doc mentions is a display concern, not an allocation risk (verified by read).
— `payload_too_large` wired to the real ceiling: `mod.rs:890-893` `if want > MAX_PACKET_BYTES { … payload_too_large(MAX_PACKET_BYTES, want) }` with `MAX_PACKET_BYTES = 64 * 1024` (mod.rs:102).

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/ps/error.rs:107-108 vs crates/demux/src/ps/mod.rs:1128-1132
Code:
```rust
/// A seek target outside the SCR-observed extent, or without a
/// usable timestamp.
pub fn seek_out_of_range() -> EngineError {
```
```rust
        let last = self.last_scr.unwrap_or(0);
        if ticks < 0 {
            return Err(error::seek_out_of_range());
        }
        let clamped = u64::try_from(ticks).unwrap_or(0).min(last);
```
What is wrong: the doc's first clause is false. A target *past* the SCR-observed extent is not refused — it is clamped (`min(last)`); a target *below* the first observed SCR is neither refused nor clamped (it lands at itself; only `min` upper-bounds). The error actually fires only for a non-`Present` target stamp (:1121), a base-conversion failure (:1126), or negative ticks (:1130).
Why it matters: doc/code drift on the one error a caller would catch to distinguish "bad target" from "clamped landing" — an integrator coding against the doc will expect `Err` for past-end seeks and instead get a clamped `Ok`. Limited blast radius (wording), hence S4.
What correct looks like: "A seek target without a usable timestamp (non-present stamp, unconvertible base, or negative ticks) — past-extent targets are clamped to the last observed SCR, not refused."
Confidence: HIGH — Verification: read of both sites.

### F-002 [S4]
Where: crates/demux/src/ps/error.rs:103-105 (pattern repeated in es/error.rs:33, ts/mod.rs:169, adts/error.rs:195, mp4/error.rs:142, ivf/error.rs:140, mkv/error.rs:173-174, crate error.rs:183)
Code:
```rust
pub fn not_started() -> EngineError {
    mint(ErrorKind::Unsupported { format: None }, "demux.not-started")
}
```
What is wrong: the identical constructor (and the `mint` helper) is hand-copied into eight module error files, all claiming the same stable code `demux.not-started`. The file-disjoint-seam rationale is documented elsewhere (tags.rs:132-136: zones mint locally "with the same taxonomy"), but nothing structural keeps the copies identical — eight tests pin eight copies of one string.
Why it matters: copy-paste duplication with drift risk; a rename in one module's file diverges the code namespace silently (each per-file test still passes). Hygiene, not behavior.
What correct looks like: one shared constructor in `crate::error` (which already exists at error.rs:183!) re-exported or called by the modules, or a single test asserting all module `not_started()` codes agree.
Confidence: HIGH — Verification: rg sweep of definitions and codes (all currently identical).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mint` | 15-17 | REAL | — | trivial, correct |
| `not_ps` | 22-32 | REAL | — | `Unsupported` + `FormatInfo{ name: "mpeg-ps" }` (CLI format id is `mpegps` — display-only mismatch, note); sniffed bounded by call site (≤4 B) |
| `truncated` | 37-44 | REAL | — | `Corrupt::Truncated{expected,found}`; 9 call sites |
| `bad_pack` | 51-60 | REAL | — | doc lists caller-side conditions (stuffing overrun, SCR width checks) — proof lives in mod.rs (other agent); 3 call sites |
| `bad_pes_header` | 66-75 | REAL | — | doc lists caller-side conditions ('10' prefix, header overrun, stuffing run, stamp vocabulary); 7 call sites |
| `resync_failed` | 79-88 | REAL | — | 1 call site (mod.rs:831); never triggered by any test (see §5) |
| `payload_too_large` | 91-100 | REAL | — | wired to `MAX_PACKET_BYTES` (mod.rs:890-893); never triggered by any test (§5) |
| `not_started` | 103-105 | REAL | F-002 | duplicate ×8 across modules |
| `seek_out_of_range` | 109-114 | REAL | F-001 | doc misdescribes the clamp |
| tests: `errors_carry_demux_stage_and_stable_codes` | 120-142 | REAL | — | pins stage+code+Display for all 8; bites on code/format change |
| tests: `not_ps_sniff_is_capped_by_display` | 144-149 | REAL | — | caps Display hex at ≤16 pairs; bites on taxonomy Display change |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `errors_carry_demux_stage_and_stable_codes` | YES | exact code + `[demux] ` prefix + ` ({code})` suffix per constructor |
| `not_ps_sniff_is_capped_by_display` | YES | counts `cd` occurrences in Display |
Gap (cross-ref: ps/tests.rs F-001, S2): the constructor tests are string-level — no test input anywhere mints `demux.ps.resync-failed`, `demux.ps.payload-too-large`, or `demux.ps.seek.out-of-range` through the demuxer (repo-wide rg; only this file's unit test constructs them directly). The codes are proven stable; the *trigger conditions* in mod.rs are unverified by tests.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "A seek target outside the SCR-observed extent, or without a usable timestamp" | error.rs:107-108 | FALSE (first clause) | mod.rs:1128-1132 clamps past-extent, passes below-extent (F-001) |
| "the honest flavor is truncation, not 'not a PS' — the AVI/WAV D-9 precedent" | error.rs:34-36 | REAL | `wrong_head` vs `truncated_pack` unit tests assert the split (ps/tests.rs:347-360, baseline :1899-1900) |
| "every extraction is width-checked before it is trusted" (SCR arithmetic) | error.rs:48-50 | CALLER-CLAIM (mod.rs, other agent) | out of this file's scope; flag to mod.rs owner |
| matrix row `container-mpeg-ps-vob` evidence strings | docs/feature-matrix.toml (PS row, another agent's range) | not audited here | — |

## 7. Dead / duplicated / unreachable code
- No dead constructors (all 8 called). Duplication: `not_started`/`mint` pattern ×8 (F-002).
- No `unsafe`, no unwrap/expect/panic/todo (house rule kept).

## 8. Dependencies & flags
`media_types::{EngineError, ErrorKind, PipelineStage}` + `media_types::error::{CorruptDetail, ErrorContext, FormatInfo, LimitKind}` — taxonomy consumed, never redefined (per crate rule). Std-only. No feature gates; compiled in every build (module unconditional). Tests use only `super::*`.

## 9. Open questions for the next agent
- ps/mod.rs owner: (a) do the `bad_pack`/`bad_pes_header` doc bullet conditions each have a real check at the call sites (SCR width checks, stuffing-run bound)? (b) should a below-extent seek target (below first SCR) land at itself or at the extent's floor? — the clamp only upper-bounds (F-001 context).
- Trigger tests for the three untested codes should be added wherever ps tests live (ps/tests.rs F-001 remediation list).
