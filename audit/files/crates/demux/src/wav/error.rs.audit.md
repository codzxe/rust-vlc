# AUDIT: crates/demux/src/wav/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-02-14 |
| Lines / functions | 197 / 10 constructors + 1 mint helper + 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — every constructor except `payload_too_large` is minted from `wav/mod.rs` (35 minting sites, rg-verified), which is reachable via containers/mod.rs:73 `"wav" => wav::open`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `target/debug/deps/demux-036dd705535b2fb3 wav::` → 43 passed (includes both error.rs tests); rg for `payload_too_large` minting sites (only definition + its own test); rg for constructor call sites in mod.rs (35) |

## 1. One-paragraph verdict
A thin, honest typed-error factory: ten constructors, each minting `EngineError` at stage
`Demux` with a stable `demux.wav.*` code, no secrets (byte counts/offsets only), no taxonomy
redefinition. Nine of ten constructors are live (35 minting sites in wav/mod.rs); the tenth
(`payload_too_large`) is dead-by-design and says so in its own doc. Both unit tests execute and
pin the stage, the codes, the Display shape, and the sniff-cap behavior. Only hygiene-level
defects.

## 2. What actually works (verified)
- Executed within the 43-pass `wav::` run: `errors_carry_demux_stage_and_wav_codes` (10 exact
  codes + `"[demux] … (code)"` Display shape) and `not_wav_sniff_capped_at_sixteen_bytes_by_display`
  (64 sniffed bytes → ≤16 "ab" pairs in Display).
- Constructor-to-caller wiring verified by rg: 35 `error::…` minting sites in wav/mod.rs cover
  all constructors except `payload_too_large`.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/wav/error.rs:153
Code:
```rust
pub fn payload_too_large(budget: u64, attempted: u64) -> EngineError {
```
What is wrong: dead code — zero minting sites outside its own test (rg over crates/, apps/,
tools/). The doc (lines 147-152) honestly declares it "vocabulary without a minting site
today", so this is documented dead surface, not a lie; it still ships an unreachable error
path in the public API. Confidence: HIGH — Verification: rg (only definition + test).
F-002 [S4]
Where: crates/demux/src/wav/error.rs:92
Code:
```rust
pub fn unsupported_format(_offset: u64) -> EngineError {
```
What is wrong: the `_offset` parameter is accepted and ignored — the error carries no offset
(`sniffed: Vec::new()`), while its siblings `bad_structure`/`bad_format_params` embed the
offset in `CorruptDetail::InvalidStructure`. Inconsistent diagnostics for the same family of
parse-site refusals (a user facing `unsupported-format` gets no position). What correct looks
like: embed the offset or drop the parameter. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint | 23-25 | REAL | — | stage Demux + ErrorContext code |
| not_wav | 30-40 | REAL | — | sniffed bytes ride FormatInfo (Display-capped, tested) |
| truncated | 47-54 | REAL | — | expected/found pair |
| bad_structure | 59-68 | REAL | — | offset embedded |
| bad_format_params | 75-84 | REAL | — | offset embedded |
| unsupported_format | 92-102 | REAL | S4 | `_offset` ignored (F-002) |
| no_fmt / no_data | 107-126 | REAL | — | — |
| not_started | 131-136 | REAL | — | — |
| seek_out_of_range | 140-145 | REAL | — | — |
| payload_too_large | 153-162 | REAL | S4 | dead-by-design (F-001) |
| tests | 168-196 | REAL | — | executed green |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_wav_codes | YES | 10 constructors × exact stage/code/Display shape |
| not_wav_sniff_capped_at_sixteen_bytes_by_display | YES | 64-byte sniff → ≤16 pairs counted in the rendered text |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Constructors take byte counts and offsets only — never paths, URIs or payload content" | error.rs:12-15 | REAL | read: every signature is u64/usize/&[u8]-sniff |
| "Sniffed magic bytes are capped by the taxonomy's own Display" | error.rs:15-16 | REAL | test executed (§2) |
| payload_too_large "Vocabulary without a minting site today" | error.rs:147-152 | REAL | rg confirms zero minting sites |

## 7. Dead / duplicated / unreachable code
- `payload_too_large` (F-001). Nothing else.

## 8. Dependencies & flags
- `media_types` only (error taxonomy). No features, no unsafe, no panics.

## 9. Open questions for the next agent
- The crate-root `error.rs` (AVI's file) carries the same-shaped `payload_too_large`
  (crates/demux/src/error.rs:141) — check whether IT has a live minting site or is the same
  dead-by-design pattern (out of my file scope).
