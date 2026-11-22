# AUDIT: crates/demux/src/mkv/ebml.rs

| Field | Value |
|---|---|
| Auditor | W05-A06 |
| Date | 2026-11-13 |
| Lines / functions | 670 / 17 (incl. 4 tests) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | YES — apps/cli/src/containers/mkv.rs → `demux::mkv::MkvDemuxer` (mod.rs:340 `open`, :906 `start`) → `ebml::parse_header_at` / `EbmlReader` on every element of every MKV/WebM open, seek, and packet read. Grep proof: `read_exact_at`/`read_prefix_at`/`parse_header_at` used from mod.rs, header.rs, cluster.rs, cues.rs, tags.rs, chapters.rs, editions.rs, attachments.rs (48+ call sites). |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:4 |
| Commands run | `Read` full file; `sed -n '9,18p;379,394p;405,413p;481,488p;447,455p' ebml.rs` (quote verification, exit 0); `rg` for `MAX_PARSE_DEPTH\|end_of_header\|READ_STEP_MAX\|read_prefix_into_at\|bad_vint\|mod fuzz` across crates/demux (exit 0); `ls fuzz/fuzz_targets` (exit 0); `wc -l` (exit 0). No cargo (adaptation). |

## 1. One-paragraph verdict

This is the EBML primitive layer — vint id/size parsing, the bounded positional reader, element-header parsing, and typed value decoders (uint/int/float/utf8) — and it is the honest core of the mkv zone: I traced every arithmetic path and found **no panic, no overflow, no unbounded allocation** reachable from in-tree callers. The hostile cases from the wave brief are all answered correctly: a 9-octet size pattern is rejected (`len > 8` → `Invalid`), an 8-byte size cannot exceed 2^56-1 (the first byte of an 8-octet vint must be `0x01`, contributing zero value bits), signed ints use correct two's-complement sign extension, floats accept only 0/4/8 bytes, UTF-8 is strict (never lossy), and unknown-size (all-ones) vints are flagged rather than trusted. The defects are documentation-reality gaps and hygiene, not code correctness: the header's depth-guard story references a `master::walk` that does not exist anywhere in the repo and claims "the parsers" carry `MAX_PARSE_DEPTH` when exactly one parser (chapters.rs) does; `end_of_header` is a dead, value-duplicated field; `read_exact_at` mixes saturating and non-saturating arithmetic in adjacent lines; and the reader hard-refuses unseekable/unknown-length sources, which is a typed refusal VLC does not have. Verdict REAL-UNPROVEN only because I could not execute the tests myself (no cargo); the in-file tests are byte-level and would catch regressions.

## 2. What actually works (verified)

- **Varint overflow question (wave brief): cannot panic or overflow.** `parse_vint_size` (210-244): the length loop rejects `len > 8` at 222-224; for `len == 8` the head byte must be `0x01` so `value_bits = first & (mask - 1) = 0x01 & 0x00 = 0`, leaving 7 data bytes → value ≤ 2^56-1, no `u64` overflow possible. A leading `0x00` byte is `Invalid` (214-216). The unknown-size marker requires the maximal encoding only (238: `value_bits == mask - 1 && data[1..len] all 0xFF`), so `[0x40,0xFF]` (value 255, known) is not mistaken for unknown. Verified by reading + in-file tests 598-653.
- `int_from_be` (505-518) sign extension verified: the accumulator starts at `0`/`-1` and shifts per byte, so `[0x80,0x00]` → -32768 (tested at 661-662: `[0xFF,0xFF]` → -1, `[0x00,0x01]` → 1). `> 8` bytes → `None` (506-508).
- `float_from_be` (523-534): exactly 0/4/8 bytes, else `None`; indexing guarded by the `match` on length. Tested 663-666.
- `utf8_from` (539-541): strict `from_utf8().ok()` — invalid UTF-8 becomes `None`, never a fabricated/lossy string. Tested 667-668.
- `declared_end()` (418-424) uses `checked_add` — a size of `u64::MAX` yields `None` (unknown-ish refusal by callers), never a wrapped position.
- `read_bounded` (313-324) chunks through `READ_STEP_MAX` (64 KiB) and treats `got == 0` as clean EOF — a source whose declared `size()` lies is handled by short-read classification, not a panic.
- The zero-alloc packet path is real: `read_prefix_into_at` (361-370) is called from mod.rs:812 (block payload) and exercised by `tests/zero_alloc_read_chain.rs` (grep, exit 0).
- Hostile-input sweeps exist in-tree: `src/mkv/fuzz.rs` (deterministic: empty, `0x1A45DFA3`, `0xFF`×4096, `0x00`×4096, 48 pseudo-random blobs ≤ 48 B) and the libFuzzer shim `fuzz/fuzz_targets/fuzz_demux_mkv.rs` exists (ls, exit 0). Whether the libFuzzer campaign ever ran: UNVERIFIED (fuzz.rs:14-16 admits it "needs the Lead's nightly wiring"; no run output available to me).

## 3. Findings

### E-001 [S3] Phantom `master::walk` reference and a false "the parsers" depth-guard claim
Where: crates/demux/src/mkv/ebml.rs:9-14
Code:
```rust
//! - **Unknown elements are skipped by their declared size** — never
//!   fatal, never descended into (no recursion; the parse depth is the
//!   fixed structural shape of the known masters, and the one
//!   data-driven descent the parsers make — [`master::walk`]-style
//!   child loops — carries an explicit depth counter refused past
//!   [`MAX_PARSE_DEPTH`]).
```
What is wrong: `master::walk` does not exist — repo-wide grep for `master` in `crates/demux/src` finds no module, struct, or function by that name (only this doc line). `MAX_PARSE_DEPTH` (line 46) is referenced by exactly one parser: chapters.rs:427 (`if depth + 1 > ebml::MAX_PARSE_DEPTH`). tags.rs uses its own `MAX_TAG_NESTING = 8` (tags.rs:111); header.rs:845-850 carries an explicit note silencing the unused import ("Silence the `MAX_PARSE_DEPTH` import when the fixed-shape parsers above carry the depth bound structurally").
Why it matters: the foundation file's safety story ("every data-driven descent carries an explicit depth counter refused past MAX_PARSE_DEPTH") is not what the code does — each walker rolls its own guard with different limits (8 for tags, 16 for chapters), and the doc points at a symbol that was never written. I verified no *unguarded* data-driven descent exists (tags/chapters/editions guarded; mod.rs/header.rs/cues/cluster/attachments walk fixed shapes), so this is documentation fraud, not a live bomb.
What correct looks like: one shared bounded-walk helper that actually enforces `MAX_PARSE_DEPTH`, or a doc that names the real guards.
Confidence: HIGH — Verification: read + grep (`MAX_PARSE_DEPTH` → ebml.rs:14/46, chapters.rs:427/734/750-752, header.rs:851 only).

### E-002 [S3] Unseekable / unknown-length sources are refused — live and pipe MKV cannot open
Where: crates/demux/src/mkv/ebml.rs:282-287
Code:
```rust
        let Some(length) = stream.size() else {
            return Err(unsupported("demux.source.unknown-length"));
        };
        if !stream.supports_seek() {
            return Err(unsupported("demux.source.unseekable"));
        }
```
What is wrong / why it matters: a Matroska stream over a pipe or a chunked HTTP response (no `Content-Length`, no seek) is typed-refused before one byte is parsed. VLC demuxes both sequentially (Cues only needed for seek). The doc (269-273) admits the scope choice ("cues-based seeking is positional"), i.e., the refusal exists because seek support was not separated from the open path — a typed refusal standing in for the unimplemented sequential mode. Reachable: any CLI open of a non-seekable source routes here.
What correct looks like: sequential open with a deferred seek refusal (`Unsupported` on `seek()` only).
Confidence: HIGH — Verification: read.
Note: this is a deliberate, documented refusal — flagged under §3.1 (typed refusal paths), not a stub.

### E-003 [S4] Saturating and non-saturating arithmetic mixed in adjacent lines of `read_exact_at`
Where: crates/demux/src/mkv/ebml.rs:385-391
Code:
```rust
        let bound = until.min(self.length);
        if at.saturating_add(want as u64) > bound {
            return Err(error::truncated(at + want as u64, bound));
        }
```
What is wrong: the guard uses `at.saturating_add`, but both error constructions use raw `at + want as u64` (lines 387 and 391). If `at + want` overflowed `u64`, the guard would have taken the `Err` branch and the error path would then panic (debug) or wrap (release). Same unchecked pattern at line 450: `let window = (pos + 12).min(bound);`.
Why it matters: `read_exact_at` and `parse_header_at` are `pub`; nothing stops a future caller from passing `at = u64::MAX - 5`. All current in-tree call sites derive `at` from real stream offsets (`data_pos` ≤ stream length + 12), so this is unreachable today — hygiene, not a live bug.
What correct looks like: `at.saturating_add(want as u64)` in the error paths too.
Confidence: HIGH — Verification: read (quotes verified `sed -n '379,394p;447,455p'`).

### E-004 [S4] `end_of_header` is a dead field, always identical to `data_pos`
Where: crates/demux/src/mkv/ebml.rs:410-412 (decl) and 485-486 (init)
Code:
```rust
    /// Absolute position of the payload (past the id+size vints).
    pub data_pos: u64,
    /// Absolute position just past the header vints.
    pub end_of_header: u64,
```
and
```rust
        data_pos: after_id + size.len as u64,
        end_of_header: after_id + size.len as u64,
```
What is wrong / why it matters: both fields are assigned the same expression; repo-wide grep finds no reader of `end_of_header` (only the declaration and the initializer). A public struct carries a duplicate-value dead field — every consumer that trusted the doc difference ("payload start" vs "just past the header vints") is reading a lie, because for an element header the payload *does* start just past the header vints; the field exists to express a distinction that does not exist.
What correct looks like: delete the field.
Confidence: HIGH — Verification: read + grep (exit 0).

### E-005 [S4] `READ_STEP_MAX` duplicated from the avi vertical
Where: crates/demux/src/mkv/ebml.rs:35-38 vs crates/demux/src/avi/riff.rs:177
Code:
```rust
/// Maximum bytes requested per `SourceStream::read` call (the avi
/// vertical's [`crate::avi::riff::READ_STEP_MAX`] value — one shared
/// per-source-call budget shape).
pub const READ_STEP_MAX: usize = 64 * 1024;
```
What is wrong / why it matters: two independent 64 KiB constants that must stay in sync for the "one shared budget shape" story to hold; a change to one silently diverges the other. The doc link at least makes the copy visible. Hygiene.
What correct looks like: one shared constant in the crate root.
Confidence: HIGH — Verification: read + grep.

### E-006 [S4] `parse_header_at` allocates a fresh `Vec` per element header on metadata walks
Where: crates/demux/src/mkv/ebml.rs:450-451
Code:
```rust
    let window = (pos + 12).min(bound);
    let bytes = reader.read_prefix_at(12, pos, window)?;
```
What is wrong / why it matters: `read_prefix_at` does `vec![0u8; want]` (line 344) — every element header in every master walk allocates and copies a ≤12-byte `Vec`. The packet path avoids this (`read_prefix_into_at` at mod.rs:812, `parse_header_mem` at mod.rs:905), so the cost lands on metadata walks (thousands of elements) and is amplified by walk-extension lies (see tags.rs F-002, where a lying child turns the rest of a file into per-element 12-byte reads). Not a hot-path defect; a per-call scratch buffer on `EbmlReader` would remove it.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_vint_id` | 179-207 | REAL | — | 1..=4 octets enforced (193-197); leading zero rejected; slicing guarded by the `data.len() < len` check; tested 548-595. |
| `parse_vint_size` | 210-244 | REAL | — | 1..=8 octets enforced (222-224); no u64 overflow possible (8-octet head contributes 0 value bits); unknown = maximal encoding only; tested 598-653. |
| `VintError` | 248-255 | REAL | — | Two-variant distinction; note mod.rs:905 `parse_header_mem` flattens `NeedMore`/`Invalid` into one `None` (mod.rs's problem — see open questions). |
| `EbmlReader::new` | 274-292 | REAL | S3 | Typed refusals: unknown-length (283), unseekable (286), empty (289) — E-002. |
| `EbmlReader::length` | 295-297 | REAL | — | Trivial. |
| `EbmlReader::seek_to` | 300-302 | REAL | — | Discards the returned position (`map(|_| ())`) — harmless. |
| `EbmlReader::prefix_len` (private) | 306-309 | REAL | — | Correct triple clamp (`until`, `length`, `at+max`), saturating; `as usize` safe because result ≤ `max`. |
| `EbmlReader::read_bounded` (private) | 313-324 | REAL | — | 64 KiB steps, clean EOF on `got == 0`; no busy-wait. |
| `EbmlReader::read_prefix_at` | 336-348 | REAL | S4 | Short read = data, never an error (documented); per-call `Vec` alloc (E-006). |
| `EbmlReader::read_prefix_into_at` | 361-370 | REAL | — | Zero-alloc twin; used by the packet path (mod.rs:812) and zero_alloc_read_chain.rs. |
| `EbmlReader::read_exact_at` | 379-394 | REAL | S4 | Correct truncation classification; non-saturating `at + want` in error paths (E-003). |
| `ElementHeader::declared_end` | 418-424 | REAL | — | `checked_add` — overflow becomes `None`, callers refuse. |
| `parse_header_at` | 439-488 | REAL | S4 | Parent-bound-cut header = clean `Ok(None)` (461-463, 474-476), stream-cut = typed truncation; `(pos + 12)` unchecked (E-003); 12-byte alloc per call (E-006). |
| `uint_from_be` | 492-501 | REAL | — | >8 bytes → `None`; empty → `Some(0)` (spec's zero-length default quirk); tested 657-660. |
| `int_from_be` | 505-518 | REAL | — | Correct two's complement (start 0/-1, shift per byte); tested 661-662. |
| `float_from_be` | 523-534 | REAL | — | 0/4/8 bytes only; NaN/Inf pass through (callers must check — mod.rs:456 does); tested 663-666. |
| `utf8_from` | 539-541 | REAL | — | Strict UTF-8; `None` on invalid — never lossy; tested 667-668. |
| `tests` (4) | 547-669 | REAL | — | Byte-level, would fail on regression (roundtrip + invalid-grammar + unknown-size + typed-value cases). |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| id_vints_roundtrip_all_four_lengths (548) | YES | Exact `VintId` equality on all four widths. |
| id_vints_reject_invalid_grammar (578) | YES | Zero head, 5-octet patterns, truncation each asserted. |
| size_vints_roundtrip_and_flag_unknown (598) | YES | Unknown flag asserted at 3 widths incl. the non-obvious `[0x7F,0xFF]`; 8-byte size value pinned. |
| typed_values_follow_the_grammar (656) | YES | uint/int/float/utf8 edge cases incl. `>8`-byte refusal. |
| Missing | — | No test for `parse_header_at` / `read_exact_at` bound semantics in this file (covered downstream in mkv integration tests — other agents' files; UNVERIFIED by me beyond grep). |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the one data-driven descent the parsers make — `master::walk`-style child loops — carries an explicit depth counter refused past `MAX_PARSE_DEPTH`" | ebml.rs:9-14 | FALSE | `master::walk` nonexistent (grep); `MAX_PARSE_DEPTH` used only by chapters.rs:427; tags.rs uses `MAX_TAG_NESTING=8`; header.rs:845-850 silences the unused import. E-001. |
| "a non-master with an unknown size is a typed `bad_vint`" | ebml.rs:15-17 | SHALLOW | The *refusal* exists but is minted as `bad_structure` at the call sites I checked (mod.rs:567, 632, 728 — `error::bad_structure`), not `bad_vint`; `bad_vint` is used only for vint-grammar failures (ebml.rs:467, 479; header.rs:328, 339). The taxonomy claim misnames the error. |
| "Declared sizes are hints: every size is checked against the enclosing bound before it is trusted" | ebml.rs:20-22 | FALSE (as an absolute) | `parse_header_at` returns headers whose `size` exceeds any bound without error (by design, readers clamp); the *walks* in tags.rs do not clamp child ends to parent ends (tags.rs F-002). The claim holds for `read_exact_at`/`read_prefix_at` reads only. |
| "Unknown sizes ... legal only for master elements" | ebml.rs:15-16 | SHALLOW | Enforced ad hoc per walker (mod.rs accepts unknown-size Segment/Cluster/Tags; tags.rs:373-375 refuses unknown-size Tag — also a master; inconsistent, see tags.rs F-005). |
| (fuzz.rs, adjacent) "Never panics, never hangs, never allocates without a ceiling — by construction" | src/mkv/fuzz.rs:35-37 | UNVERIFIED | No execution possible; deterministic sweeps cover ≤ 4 KiB inputs only; libFuzzer campaign wiring admitted unfinished (fuzz.rs:12-16). |

## 7. Dead / duplicated / unreachable code

- `end_of_header` (E-004): dead public field, duplicate value.
- `READ_STEP_MAX` (E-005): deliberate duplication of avi/riff.rs:177.
- No unreachable functions: all four pub readers and both vint parsers have in-tree callers (grep, exit 0). `pub mod fuzz` (mod.rs:46) compiles the fuzz harness into the production library unconditionally — reachable only via `fuzz/fuzz_targets/fuzz_demux_mkv.rs` (separate crate) and its own tests; from the shipped CLI it is UNREACHABLE surface.

## 8. Dependencies & flags

- `media_types` (EngineError), `plugin_api::SeekOrigin` + `SourceStream`, `crate::mkv::error`. No feature flags; unconditional `mod` tree (lib.rs:50). No `unsafe`. The reader depends on the source reporting a truthful `size()` — a lying source is handled by short-read classification (read_bounded EOF), verified by reading.

## 9. Open questions for the next agent

- mod.rs owner: `parse_header_mem` (mod.rs:901-906) flattens `VintError::NeedMore` and `Invalid` into `None` — a grammar-corrupt header mid-cluster silently ends the block walk with no damage counter. Does anything count that as damage?
- header.rs owner: the `MAX_PARSE_DEPTH` silence note (845-850) — confirm no fixed-shape walk there can actually loop unboundedly on crafted input (TrackEntry chains are sibling walks, so presumably bounded by segment size only — same file-size-proportional walk concern as tags.rs F-002).
- fuzz owner: has the nightly libFuzzer campaign ever executed? (`fuzz/README.md` claims "Lead-owned nightly wiring"; audit.yml:325 gates a fuzz tier behind a 02:00 schedule / dispatch — no run evidence in-repo.)
