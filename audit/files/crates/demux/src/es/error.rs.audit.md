# AUDIT: crates/demux/src/es/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-11-21 |
| Lines / functions | 78 / 6 (1 private helper + 5 pub constructors) |
| File verdict | REAL (3/5 constructors executed+asserted in baseline; `au_too_large`, `bad_header` traced but never executed in any committed run → per-function REAL-UNPROVEN) |
| Reachable from shipped entry point? | YES (call chain: `apps/cli/src/containers/mod.rs:62-66` match arms `"flac"/"hevc"/"h264"/"mp2"/"mp3"` → `apps/cli/src/containers/es.rs` `open_annexb`/`open_mpeg_audio` → `demux::es::{annexb,mp3}` → `super::error::*` at 14 call sites, rg-verified) |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:2 |
| Commands run | `rg -n "no_sync|not_started|truncated|au_too_large|bad_header" crates/demux/src/es/` (all 5 constructors live); `rg -n '"demux\.es' crates/demux/src/es/*.rs` (8 family codes, mint sites mapped); `rg -n "au-too-large|bad-header|recovery-budget|seek\.unsupported" --type rust crates/ apps/` (zero pins for 2 of 5 codes); `rg -n "fn mint\(" crates/` (30 repo copies); `sed -n` on every quote below; baseline cites from `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:1680-1690, 2019-2026` |

## 1. One-paragraph verdict
Real, live, mostly-pinned error vocabulary: every constructor is called from real demux code
(mp3.rs 10 sites, annexb.rs 5 sites), the module is reachable from the shipped CLI through the
raw-ES container arms, and 11 `es::tests` + 4 differential `es_corpus` tests are green in the
nofailfast baseline, three of them asserting this file's exact code strings. The defects are
quality-level: `bad_header`'s doc contract is violated by half its callers (who pass a
fabricated `offset: Some(0)` for a failure mode that cannot fire), 2 of 5 codes are pinned
nowhere in the repo while `es/mod.rs:31` claims "all unit-pinned", the header's "never …
payload content" rule is contradicted by `no_sync` itself, and the `demux.es.*` family is
minted from 4 scattered sites via duplicated helpers. No S0–S2: no panic path, no swallowed
error, no wrong-kind mint found; kinds map cleanly onto the media-types taxonomy.

## 2. What actually works (verified)
— `no_sync`/`truncated`/`not_started` executed and code-asserted: baseline
`orchrun-test-workspace-nofailfast.log.txt:1680-1690` — 11 `es::tests … ok`, including
`h264_no_sync_is_typed_unsupported_after_scan` (asserts `demux.es.no-sync`, tests.rs:148,154),
`mp3_truncated_final_frame_is_typed_sticky` (`demux.es.truncated`, tests.rs:241,253),
`next_event_before_start_is_typed` (`demux.es.not-started`, tests.rs:183).
— Happy lanes differential: same log `:2019-2026` — `tests/es_corpus.rs` 4/4 ok vs
ffprobe packet geometry/counts (h264 26 AUs, hevc 56, aac 79, mp3 lanes).
— `au_too_large` budget value traced real: `annexb.rs:74` `MAX_AU_BYTES = 16 MiB`, ceiling
enforced at `annexb.rs:450-452` (`if size > MAX_AU_BYTES` → this constructor).

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/es/error.rs:67-78 (constructor), misuse at crates/demux/src/es/mp3.rs:255 and crates/demux/src/es/annexb.rs:172
Code:
```rust
/// A structure that parses as a claim but declares impossible shape
/// (e.g. an mp3 header whose size arithmetic collapsed).
pub fn bad_header(offset: u64) -> EngineError {
```
```rust
    // Static ASCII literal: the grammar cannot refuse it; the typed
    // error is the panic-free residue (8.A1 sweep).
    CodecDescriptor::new(name).map_err(|_| super::error::bad_header(0))
```
(mp3.rs:254-255; identical at annexb.rs:171-172)
What is wrong: the constructor documents a header-shape failure carrying the offending
`offset`, but 2 of its 4 call sites use it for a *codec-name registry* failure
(`CodecDescriptor::new` on the static literals "mp3"/"mp2"/"h264"/"hevc") and pass a
fabricated `offset: 0` — there is no header at offset 0 in that failure mode; there is no
failure mode at all, as the call-site comments admit the input cannot be refused.
Why it matters: if the residue ever fired, the typed error would assert
`Corrupt::InvalidStructure { offset: Some(0) }` — a false location claim in a
machine-readable compatibility surface; and dead residue wired to the wrong vocabulary hides
the fact that no honest error exists for that branch.
What correct looks like: either a dedicated unreachable-in-practice constructor (e.g.
`internal_invariant(code)`) or a panic-free `expect`-style typed internal error that does
not claim a byte offset; `bad_header` reserved for real parse-site failures (mp3.rs:366 is
the only honest call).
Confidence: HIGH — Verification: read + rg (both sites), call-path traced; cannot fire
(static ASCII names).

### F-002 [S4]
Where: crates/demux/src/es/error.rs:2-5 vs 20-30
Code:
```rust
//! media-types' (never redefined here); codes are `demux.es`-scoped,
//! stable, and carry byte counts/offsets only — never paths or
//! payload content.
```
```rust
pub fn no_sync(format: &'static str, sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some(format.to_string()),
                sniffed: sniffed.to_vec(),
```
What is wrong: the header rule "never … payload content" is contradicted by the file's own
first constructor, which clones sniffed stream bytes into the error. The annexb lane passes
the whole 64 KiB open-scan window (`annexb.rs:412` `&win`, `MAX_OPEN_SCAN_BYTES = 64*1024`
at annexb.rs:84) into `FormatInfo.sniffed` — an unbounded-in-type `Vec<u8>`
(media-types/src/error.rs:80) whose Display caps at 16 bytes (media-types error.rs:311-322)
— while the mp3 lane passes exactly 16 (`mp3.rs:412` `read_prefix_at(16, 0)`). The sentence
is copied from `adts/error.rs` ("Constructors take byte counts and offsets only — never
paths, URIs or payload content") whose `not_adts(sniffed)` has the same contradiction.
Why it matters: the "no secrets" contract is the file's stated design rule; a reader auditing
for leak-free errors is misled, and the two lanes embed 16 vs 65,536 bytes for the same code.
What correct looks like: header states the real rule (leading-bytes hint, capped at N in
Display, window bytes retained in the value), and both lanes pass the same capped prefix.
Confidence: HIGH — Verification: read all three files + media-types caps; sizes rg-verified.

### F-003 [S3]
Where: crates/demux/src/es/error.rs:56-65, 69-78 (unpinned codes); claim at crates/demux/src/es/mod.rs:31
Code:
```rust
//! ## Error paths (typed, all unit-pinned)
```
```rust
        "demux.es.au-too-large",
```
```rust
        "demux.es.bad-header",
```
What is wrong: `demux.es.au-too-large` and `demux.es.bad-header` are asserted nowhere in the
repo — `rg "au-too-large|bad-header|recovery-budget|seek\.unsupported" crates/ apps/` finds
zero references outside the mint sites — yet mod.rs:31 claims every es error path is
"unit-pinned". Every sibling this file claims to mirror ships an in-file constructor→code
table test (adts/error.rs:219, ivf/error.rs:165, asf/error.rs:342, ogg/error.rs:185);
es/error.rs ships none.
Why it matters: `ErrorContext` codes are a declared compatibility surface
(media-types/src/error.rs:218-221: "part of the compatibility surface once a release
ships"); mutation reasoning — rename either string, or change `au_too_large`'s kind from
`ResourceLimit` to `Corrupt`, and no test in the suite fails. The fuzz harness explicitly
swallows errors without checking codes (fuzz.rs:19-20 "all swallowed here"). `au_too_large`
is additionally never executed: no fixture anywhere has a >16 MiB access unit, so the
constructor has zero executions in all committed runs.
What correct looks like: the sibling table test (constructor → code + kind assertions) in
this file, and mod.rs:31 downgraded to "3/5 pinned" until then; an es_corpus case with a
>16 MiB AU to execute the ceiling.
Confidence: HIGH — Verification: rg repo-wide (no pins), read sibling error.rs test modules,
baseline log read (es tests assert only 3 codes).

### F-004 [S4]
Where: crates/demux/src/es/error.rs:12-14 vs crates/demux/src/es/mp3.rs:279-285 and annexb.rs:534-537
Code:
```rust
fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}
```
```rust
fn mint(kind: media_types::ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Demux,
```
```rust
        Err(EngineError::new(
            media_types::PipelineStage::Demux,
            media_types::ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("demux.es.seek.unsupported"),
```
What is wrong: two byte-identical free `mint` helpers inside one module tree (error.rs:12-14,
mp3.rs:279-285), a third sticky-replay `mint(&self)` (annexb.rs:329-331), and annexb.rs:534-537
bypasses all of them with a direct `EngineError::new`. The `demux.es.*` family's 8 codes are
minted from 4 sites across 3 files; this module — "THE typed error constructors for the raw
elementary-stream demuxers" — owns only 5 of 8. Repo-wide this is the 30th copy of
`fn mint(` (rg over crates/).
Why it matters: the family's single point of truth is the stated design (scope discipline);
splitting it guarantees drift — a future kind/code convention change must be replicated in
four places, and nothing enforces it.
What correct looks like: one `demux::es`-scoped mint (this file) with the three stray codes
(`demux.es.mp3.recovery-budget`, `demux.es.mp3.seek.unsupported`,
`demux.es.seek.unsupported`) routed through constructors here; the R3 consolidation the
adts header itself promises.
Confidence: HIGH — Verification: rg `fn mint\(` (30 hits) + read of all 4 es mint sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mint` (private) | 12-14 | REAL | — | Correct 3-arg mint; 30th repo copy (F-004). No panic path. |
| `no_sync` | 20-30 | REAL | S4(F-002) | Live at mp3.rs:413, annexb.rs:412; code asserted in baseline (tests ok). Embeds payload bytes (16 mp3 / 64 KiB annexb) contradicting the header rule. |
| `not_started` | 33-38 | REAL | — | Live at 9 sites (mp3 6, annexb 3); code asserted (tests.rs:183, baseline ok). `Unsupported{format:None}` for a caller-contract violation is the taxonomy's least-bad fit (no State kind exists — media-types error.rs:190-214); consistent with adts/ivf/wav siblings. |
| `truncated` | 42-52 | REAL | — | Live at mp3.rs:373, 541; code asserted ×2 + kind asserted (tests.rs:241-247); sticky replay verified in test. `expected/found` mapping correct at both call sites. |
| `au_too_large` | 56-65 | REAL-UNPROVEN | S3(F-003) | Live at annexb.rs:451; budget = real `MAX_AU_BYTES` (annexb.rs:74). Zero executions (no >16 MiB-AU fixture) and zero code pins in the entire repo. |
| `bad_header` | 69-78 | REAL-UNPROVEN | S3(F-001) | Honest call at mp3.rs:366 (ID3 `size > MAX_ID3_BYTES`); dishonest residue calls at mp3.rs:255/annexb.rs:172 with fabricated offset 0. Code string pinned nowhere. |

## 5. Test honesty (tests in es/tests.rs + tests/es_corpus.rs execute this file's output)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| es::tests::h264_no_sync_is_typed_unsupported_after_scan | YES | Asserts kind `Unsupported` AND code `demux.es.no-sync` twice (initial + sticky replay) — tests.rs:144-155. Mutation: swap kind to Corrupt or change code → both asserts fail. |
| es::tests::mp3_no_sync_is_typed_unsupported_after_scan | YES | Same code+kind asserts on the mp3 lane (tests.rs:258-268). |
| es::tests::mp3_truncated_final_frame_is_typed_sticky | YES | Asserts code, kind shape `Corrupt::Truncated`, 1-packet-prefix behavior, and sticky replay (tests.rs:234-254). |
| es::tests::next_event_before_start_is_typed | PARTIAL | Asserts only the code string (tests.rs:183), not the kind — a kind mutation (e.g. `Corrupt`) would pass. |
| es::tests::empty_source_refuses_at_construction | NO (not this file) | Asserts kind only; the refusal is minted in SourceReader/constructor code, not via these constructors. |
| es_corpus::* (4 tests) | NO (not this file) | Happy-lane differential vs ffprobe; `err.is_none()` asserted — exercises no constructor. |
| (absent) au_too_large / bad_header pin | — | No test would fail on any mutation of these two constructors (F-003). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "codes … carry byte counts/offsets only — never paths or payload content" | es/error.rs:3-5 | FALSE | `no_sync` embeds `sniffed.to_vec()` (error.rs:25); annexb passes 64 KiB window (annexb.rs:412, :84) — F-002. |
| "Mirrors the `adts::error` pattern" | es/error.rs:2 | SHALLOW | Mint pattern mirrored; adts's in-file code-pinning test module (adts/error.rs:215+) NOT mirrored — F-003. |
| "Error paths (typed, all unit-pinned)" | es/mod.rs:31 | FALSE | 3/5 codes pinned (tests.rs:148/154/183/241/253/264); au-too-large/bad-header pinned nowhere (rg repo-wide). |
| "the ceiling is refused, never grown (MP §12)" | es/error.rs:54-55 | REAL | annexb.rs:450-452 hard-refuses `size > MAX_AU_BYTES`; no growth path (read). Executed: UNVERIFIED (no big-AU fixture). |
| "contract-order violations are `demux.es.not-started`" | es/mod.rs:38-39 | REAL | tests.rs:183 asserts the code; baseline :1690 `next_event_before_start_is_typed … ok`. |

## 7. Dead / duplicated / unreachable code
- Dead-in-practice: the `bad_header(0)` residue arms (mp3.rs:255, annexb.rs:172) —
  unreachable (static ASCII codec names); F-001.
- Never-executed: `au_too_large` (no fixture with >16 MiB AU in the tree).
- Duplicated: free `mint` ×2 within es (error.rs:12, mp3.rs:279), 30 repo-wide; F-004.
- Not dead: `no_sync`, `not_started`, `truncated` — all live, executed, asserted.

## 8. Dependencies & flags
- Depends only on `media_types` (error taxonomy + `EngineError`); std-only otherwise —
  matches the es module's "zero external dependencies (C5)" house rule (es/mod.rs:23-24).
- No feature flags gate this file (no `#[cfg]` in file; es module unconditionally compiled —
  `crates/demux/src/es/mod.rs:41-47` plain `pub mod`).
- No `unsafe`, no `unwrap`/`expect`/`panic!` in the file (house rule held; read).

## 9. Open questions for the next agent
- Who consumes `FormatInfo.sniffed` beyond Display? If any UI/log path prints the raw
  struct (Debug), the 64 KiB annexb window lands in logs — trace `FormatInfo` consumers.
- `demux.es.seek.unsupported` (annexb.rs:536) vs `demux.es.mp3.seek.unsupported`
  (mp3.rs:687): two spellings for the same refusal class on sibling lanes — drift or
  deliberate? (Worth one look by the mp3/annexb file auditor.)
- F-003's fix belongs with whoever owns `es/mod.rs:31` — coordinate to avoid double-counting.
