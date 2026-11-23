# AUDIT: crates/demux/src/asf/data.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-23 |
| Lines / functions | 463 / 5 source fns (type_width, u16le, u32le, parse_packet, read_field) + ObjectAssembler impl (new, clear, feed, take_completed) |
| File verdict | UNREACHABLE (real, bounds-clean implementation; no shipped entry point wires the ASF demuxer) |
| Reachable from shipped entry point? | **NO** — verified by exhaustive search: no `asf` arm in apps/cli/src/containers/mod.rs, no containers/asf.rs, zero `asf` mentions in crates/media-probe/src (no probe signature), apps/desktop, apps/desktop-tauri, apps/web, apps/control-server, crates/playback-engine, crates/commands. The only callers of `data::` live in crates/demux/src/asf/mod.rs itself (mod.rs:469, 485, 748) — and nothing outside crates/demux/src/asf/ references `demux::asf` (rg, exit 0 matches). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/demux-5e0c7194d741938b --test-threads=2 'asf'` -> 36 passed in 1.54s (includes the live-ffprobe fixture cross-check); targeted run of the 4 data.rs tests -> 4 passed; rg reachability sweeps (above) |

## 1. One-paragraph verdict
The parser itself is the tightest code I have audited in this wave: every field read is bounds-checked against the packet slice, every allocation is ceilinged (32 MiB per reassembled object, checked on both the first fragment and every continuation), the compressed-replication form is typed-refused rather than guessed, and the hold-and-flush reassembly contract is coherent (I traced first/continuation/orphan/EOS paths). The asf test suite executes it for real — 36 passed, including a live ffprobe cross-check against the committed fixtures. But none of it ships: the ASF demuxer is wired into no entry point (CLI containers, media-probe, desktop, web, engine), and the compat-matrix row `container-asf-wmv` claims `implemented` on windows/linux/macos with **no evidence strings at all** while no shipped build can open a single .wma/.wmv/.asf file — a claim/reality contradiction that outclasses any in-file defect. One real in-file behavior issue: a fragment landing at a wrong offset on an open object is a sticky FATAL error for the whole demux, inconsistent with the module's own damage philosophy for the identical corruption class.

## 2. What actually works (verified)
- Full asf suite: `./target/debug/deps/demux-5e0c7194d741938b --test-threads=2 'asf'` -> `36 passed; 0 failed` in 1.54 s — the duration proves real work (fixture generation + live `ffprobe` subprocess in `fixture_family_cross_checks_against_live_ffprobe`, asf/tests.rs:1325-1346, which passed against this machine's ffprobe).
- The four data.rs-targeted tests pass: fragments_reassemble_into_whole_objects, malformed_packets_refuse_typed_never_guess, multi_payload_packets_parse_in_order, packet_fields_parse_and_close_exactly.
- Panic-path trace (read): every access is `bytes.get`/length-guarded (132-134, 138-140, 155-157, 162-168, 192-198, 201-203, 213-215, read_field:299-313, 236-238, 248-250); `pos + length` uses `checked_add` (266-268). The module's "the parse never indexes past its input" claim (line 48-49) holds.
- Allocation ceiling verified: first fragment `size` ≤ 32 MiB (416-418), `data.len() ≤ size` (419-421), continuation `new_len > size || > MAX_OBJECT_BYTES` refused (389-392). No unbounded allocation from header values.
- Doc's "no unwrap/expect/panic! in this non-test code" verified by rg (only `unwrap_or`, line 258).

## 3. Findings

F-001 [S2]
Where: tests/compat/compat-matrix.json (claims[... row container-asf-wmv]) vs the repo wiring; touching crates/demux/src/asf/data.rs (the "implemented" payload)
Code (data.rs — the unreachable seam under the claim):
```rust
pub fn parse_packet(bytes: &[u8]) -> Result<ParsedPacket, EngineError> {
    // Error-correction field: 1 flags byte + (flags & 0x0F) bytes.
    let mut pos = 0_usize;
```
plus compat-matrix claim (verified via python3 json): `{"row": "container-asf-wmv", "windows": "implemented", "linux": "implemented", "macos": "implemented", "web": "not_started"}` — no evidence field.
What is wrong: the matrix claims ASF/WMV/WMA containers are *implemented* on three platforms; the demuxer library code exists (this file, header.rs, mod.rs — genuinely working, 36 green tests), but no shipped entry point can reach it: no `asf` container arm in the CLI (apps/cli/src/containers/mod.rs — 12 containers, none asf), no ASF signature in media-probe (rg -i asf over crates/media-probe/src: zero hits — the probe layer cannot even identify the format), zero references in desktop/desktop-tauri/web/control-server/playback-engine/commands. A user opening a .wma file gets "not identified", not "implemented". The row carries no evidence strings, and the orchestrator's parity report already records 0/252 verified. This is a claim/reality contradiction at the platform-status level (the row's own definition of "implemented" is whatever the row means — by any user-visible meaning it is false). Confidence: HIGH — Verification: ran (exhaustive rg sweeps, each exit-checked) + read.
Confidence: HIGH — Verification: ran

F-002 [S3]
Where: crates/demux/src/asf/data.rs:383-392, propagated sticky by mod.rs:469/485 (`data::parse_packet(&bytes)?` / `.feed(...)... finished?`)
Code:
```rust
            Some(pending) if pending.number == media_object => {
                // Continuation fragment (must land exactly at the end).
                let start = pending.buffer.len() as u32;
                if offset != start {
                    return Err(error::packet_bad_length(offset as u64));
                }
```
What is wrong: a continuation fragment whose offset does not exactly equal the buffer length — i.e. ONE lost or reordered data packet anywhere in the stream — returns a typed Err that the caller propagates with `?`, sticky-failing the entire demux. The module's own damage doctrine (lines 356-362: "a gap the container declares by moving on — counted by the demuxer's damage observability, never invented"; orphans are "skipped, never fatal") is applied to new-object gaps and orphans, but the same corruption class inside an open object is fatal instead of damage-noted-and-abandoned. On a damaged-but-mostly-playable file the honest behavior per this codebase's own standards (the OGG damage policy, the ASF skip of undeclared-stream payloads at mod.rs:476-480) is to drop the object and continue. Blast radius today: none (unreachable, F-001); the day ASF ships it becomes a realistic damaged-file hard-fail. What correct looks like: abandon the open object (damage-note) and resync on the next offset==0 or matching-offset fragment. Confidence: HIGH — Verification: read (caller propagation traced at mod.rs:469/485; not executed against a crafted dropped-packet file — UNVERIFIED by run).
Confidence: HIGH — Verification: read

F-003 [S4]
Where: crates/demux/src/asf/data.rs:248-251 and 254-264
Code:
```rust
            if bytes.len() < pos + repl_len {
                return Err(error::packet_bad_length(pos as u64));
            }
            pos += repl_len;
```
What is wrong: on a 32-bit `usize` target (wasm32 is a shipped lane per the baseline rc-smoke), `pos + repl_len` with a crafted dword replicated-length (up to 0xFFFF_FFFF) wraps before the comparison in release builds — the guard can be skipped and `pos += repl_len` wraps backwards; the parse then continues from a wrong offset and either fails the close check or fabricates values (memory-safe: all later accesses are `get()`-based). On 64-bit no overflow is possible (pos ≤ ~64 KiB, repl_len ≤ 4 GiB). Same pattern avoided correctly at 266-268 with `checked_add`. What correct looks like: `repl_len.checked_add(pos)` like the payload length. Confidence: MEDIUM — Verification: read (arithmetic traced; wasm32 wrap not executed — UNVERIFIED by run).
Confidence: MEDIUM — Verification: read

F-004 [S4]
Where: crates/demux/src/asf/data.rs:254-257 vs 181-189
Code:
```rust
        let length = if plen_width == 0 {
            let rest = bytes.len().saturating_sub(pos + padding as usize);
            u32::try_from(rest).unwrap_or(u32::MAX)
```
What is wrong: the property-flags comment says code `00` = "absent (final payload only)" (182-184), but the loop never checks that only the LAST payload uses the absent form. The arithmetic closes the hole indirectly (a second payload with plen absent consumes the whole remainder and the next payload's header read then fails the end-bound check — traced), so behavior is safe-but-indirect: multi-payload packets with f2-plen=00 are refused through an accidental path rather than the direct rule. Note also the `0x3F` payload-count mask drops bit 6 (`0x40`) — the ASF spec's 1-byte vs 2-byte payload-count distinction is not implemented (ffmpeg writes the 1-byte form only; unverified shapes elsewhere are refused, this one is silently masked). Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| type_width | 99-106 | REAL | — | 1/2/4 byte widths, 0 = absent |
| u16le / u32le | 108-114 | REAL | — | panics only if caller skips guards — none do (traced) |
| parse_packet | 129-296 | REAL | S4 | EC skip, f1/f2 shapes typed-refused, padding both-set refused, repl_len 1 refused, close-exact rule enforced (285-287); F-003 wrap, F-004 mask/finality |
| read_field | 299-313 | REAL | — | bounds-checked, width 0 → Some(0) |
| ObjectAssembler::new / clear | 335-346 | REAL | — | one slot per header stream |
| ObjectAssembler::feed | 368-442 | REAL | S3 | hold-and-flush traced coherent; orphan skip ✓; F-002 sticky-fatal offset mismatch |
| ObjectAssembler::take_completed | 450-461 | REAL | — | EOS drain of complete-but-held |

## 5. Test honesty
(asf/tests.rs is another agent's file; the data.rs-relevant tests I ran:)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| packet_fields_parse_and_close_exactly | YES | field-by-field + exact close |
| multi_payload_packets_parse_in_order | YES | ordered payloads |
| malformed_packets_refuse_typed_never_guess | YES | typed refusals per shape |
| fragments_reassemble_into_whole_objects | YES | cross-packet reassembly output |
| walk_emits_whole_objects_with_preroll_subtracted_stamps | YES | end-to-end stamps |
| payloads_for_undeclared_streams_are_skipped_not_fatal | YES | the skip seam |
All ran green (36/36 asf suite). None of them covers F-002's wrong-offset-continuation shape (no dropped-packet fixture) — the gap is untested.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every layout below was byte-decoded against live muxer output and closes each packet exactly" | data.rs:3-7 | REAL | 36-test suite incl. committed fixtures + live ffprobe cross-check (ran green); the two committed fixtures exist in src/asf/fixtures (my files 10/14) |
| "no unwrap/expect/panic! in this non-test code" | data.rs:47-49 | REAL | rg verified (only unwrap_or at 258) |
| "every length is checked against the packet slice ... never indexes past its input" | data.rs:48-49 | REAL (64-bit) | traced all reads; F-003 notes the wasm32 wrap in the *check* arithmetic itself |
| "[MAX_OBJECT_BYTES] is container-trust input, never a promise to allocate" | data.rs:55-58 | REAL | ceiling enforced at 390-392, 416-421 |
| compat matrix `container-asf-wmv: implemented` (win/linux/macos) | tests/compat/compat-matrix.json | FALSE (no shipped wiring) | F-001 reachability sweeps |

## 7. Dead / duplicated / unreachable code
The entire asf module (this file included) is library-public but caller-less outside itself — see F-001. Within the file, nothing is dead.

## 8. Dependencies & flags
`media_types::EngineError` + `super::error` (typed asf codes, asserted by tests). Std-only, no unsafe. Compiled into the demux lib (release lane builds it; the linker dead-strips it from shipped binaries since nothing references it).

## 9. Open questions for the next agent
- Who owns the `container-asf-wmv` matrix row's "implemented" status? If "implemented" means "library code exists", the matrix needs an explicit wiring column — as written it is indistinguishable from a false user-facing claim (F-001).
- asf/mod.rs and asf/header.rs (other agents): confirm the stream-count ceiling that sizes `ObjectAssembler::new(stream_count)` — a header declaring thousands of streams mints that many slots (bounded but unvalidated here).
- If ASF ever ships: craft a dropped-packet fixture to pin F-002's fatal-vs-damage-noted behavior.
