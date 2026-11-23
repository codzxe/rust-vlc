# AUDIT: crates/demux/src/ps/error.rs
| Field | Value |
|---|---|
| Auditor | W05-A02 |
| Date | 2026-11-23 |
| Lines / functions | 150 / 9 (mint + 8 pub ctors) + 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: lumen-cli `containers/mod.rs:71` `"mpegps" => ps::open` -> `PsDemuxer` (ps/mod.rs) -> these ctors at ps/mod.rs:415, 942, 966, 1118, 1145 (usage counts: truncated 9, seek_out_of_range 6, bad_pes_header 7, not_started 4, bad_pack 3, not_ps/resync_failed/payload_too_large 1 each). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/demux-036dd705535b2fb3 'ps::error'` -> 2 passed, 0 failed; sed/rg verifications (exit 0) |

## 1. One-paragraph verdict
A real, complete typed-error constructor module: every constructor is called by
the PS demuxer, codes are unique and stable, the stage tag is Demux, and the two
in-file tests (executed) pin stage, code, display prefix/suffix, and the sniffed
Display cap. It is pure constructor code — zero arithmetic, zero indexing, zero
panic paths, no probe-twin pattern possible. Two hygiene defects: the module doc
promises "No payload bytes" while `not_ps` embeds sniffed stream bytes into the
error (bounded to 4 bytes by its only caller, 16 by Display), and `not_started`
is a byte-identical duplicate of `crate::error::not_started` carrying a
non-`ps.`-namespaced code inside the `ps` namespace.

## 2. What actually works (verified)
- Ran `./target/debug/deps/demux-036dd705535b2fb3 'ps::error'` -> "2 passed;
  0 failed" (`errors_carry_demux_stage_and_stable_codes`,
  `not_ps_sniff_is_capped_by_display`). Baseline concurs
  (audit/runs/baseline/cargo-test-workspace.txt:1702 demux unittests ok).
- All 8 constructors have live callers in ps/mod.rs (rg counts above) — no dead
  constructors.
- Codes are distinct dotted strings; kinds match their semantics (Unsupported
  for not_ps/not_started/seek_out_of_range, Corrupt{Truncated|InvalidStructure}
  for damage, ResourceLimit{Memory} for the ceiling).

## 3. Findings
F-001 [S4]
Where: crates/demux/src/ps/error.rs:3-6 (claim) vs :22-32 (code)
Code:
```
//! [`EngineError`](media_types::EngineError) values with stable dotted
//! codes. No payload bytes, no paths — offsets and counts only.
```
```
pub fn not_ps(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("mpeg-ps".to_string()),
                sniffed: sniffed.to_vec(),
```
What is wrong: The module doc says errors carry "no payload bytes ... offsets
and counts only"; `not_ps` stores `sniffed.to_vec()` — leading stream bytes — in
the error value. The constructor's own doc admits this ("sniffed carries the
leading bytes"), so the module doc is the false half. Blast radius is bounded:
the only caller passes at most 4 bytes (ps/mod.rs:938-942, `[0u8; 4]` head) and
Display caps the rendering (test: <=16 hex "cd" matches from 64 bytes). The
`to_vec` itself is unbounded in the constructor — any future caller with a big
slice would copy it whole. What correct looks like: cap the sniff in the
constructor (`sniffed.len().min(16)`) and fix the module doc.
Confidence: HIGH — Verification: read + rg (single caller).

F-002 [S4]
Where: crates/demux/src/ps/error.rs:102-105 vs crates/demux/src/error.rs:182-184
Code:
```
pub fn not_started() -> EngineError {
    mint(ErrorKind::Unsupported { format: None }, "demux.not-started")
}
```
What is wrong: Byte-identical duplicate of `crate::error::not_started` (same
kind, same code `demux.not-started`): one code, two minting sites, drift risk,
and the code breaks the module's own `demux.ps.` namespacing convention (every
other code here is `demux.ps.*`). ps/mod.rs calls the local copy 4 times; other
demuxers use the crate-root copy. What correct looks like: re-export or call
`crate::error::not_started`. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint (private) | 15-17 | REAL | — | |
| not_ps | 22-32 | REAL | S4 | F-001 sniffed bytes vs doc; caller-bounded to 4 |
| truncated | 37-44 | REAL | — | honest expected/found |
| bad_pack | 51-60 | REAL | — | offset detail |
| bad_pes_header | 66-75 | REAL | — | |
| resync_failed | 79-88 | REAL | — | |
| payload_too_large | 91-100 | REAL | — | budget/attempted |
| not_started | 103-105 | REAL | S4 | F-002 duplicate of crate::error::not_started |
| seek_out_of_range | 109-114 | REAL | — | |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_stable_codes | YES | pins stage, code, `[demux] ` prefix and ` (code)` suffix for all 8 ctors |
| not_ps_sniff_is_capped_by_display | YES | 64 sniffed bytes must render <=16 hex pairs; would catch a Display cap regression (note: tests the DISPLAY, not the stored vec — F-001's storage is unbounded) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No payload bytes, no paths — offsets and counts only" | error.rs:6 | FALSE for not_ps | `sniffed.to_vec()` :27; caller caps at 4 bytes |
| "`sniffed` carries the leading bytes ... (capped by the taxonomy's Display)" | error.rs:20-21 | REAL | test executed (Display cap); storage unbounded (F-001) |
| "Same shape as crate::error" | error.rs:3 | REAL | duplicate found is the shape's own downside (F-002) |
| every ctor doc describes its trigger | :19,34,46,62,77,90,102,107 | REAL-UNPROVEN | trigger conditions live in ps/mod.rs (other agent's file); codes/kinds/stage verified here |

## 7. Dead / duplicated / unreachable code
- `not_started`: duplicated with crates/demux/src/error.rs:182 (F-002).
- No dead constructors (all called, counts in header table).

## 8. Dependencies & flags
media_types error taxonomy + PipelineStage only; no feature gates; no unsafe;
compiled unconditionally as part of the `ps` module (registered "mpegps"
container lane). No new deps.

## 9. Open questions for the next agent
- ps/mod.rs owner: confirm each ctor's trigger doc (e.g. "unbounded stuffing
  run" for bad_pes_header) matches the actual raise sites — the raise conditions
  are in your file, the docs are in this one.
