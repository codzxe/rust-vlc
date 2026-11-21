# AUDIT: crates/demux/src/asf/data.rs (462 lines, whole file)
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-11-21 |
| Lines / functions | 462 / 12 items (parse_packet, read_field, type_width, u16le/u32le, ParsedPayload, ParsedPacket, PendingObject, ObjectAssembler::{new,clear,feed,take_completed}) |
| File verdict | UNREACHABLE (real, tested code — no shipped entry point reaches it; see F-001) |
| Reachable from shipped entry point? | NO. Proof: `apps/cli/src/containers/mod.rs:49-74` has no `asf` arm (falls to `unwired_refusal`); `crates/media-probe/src/signature/` has 14 signature types, none ASF (rg -i "asf\|wma\|wmv" over crates/media-probe/src → 0 hits); repo-wide rg for `AsfDemuxer\|asf::` outside crates/demux → 0 hits. The repo's own compat-matrix triage confirms: docs/feature-matrix.toml:217 "the open path REFUSES the corpus files of probe format `asf` — unsupported … (probe.no-demuxer): probe hints: extension=wmv" |
| Findings | S0:0 S1:0 S2:0 S3:3 S4:2 |
| Commands run | read whole file; rg asf/wma/wmv over media-probe + CLI + repo-wide callers; sed containers/mod.rs:40-74; grep nofailfast log → 36 `test asf::*` all ok (1555-1590 region); read asf/tests.rs:609-640, 1128-1160 (fixture sha256 pins); grep feature-matrix.toml:207-217 and docs/formats/container-asf.md |

## 1. One-paragraph verdict
The parser and reassembler are careful, panic-free, fully bounds-checked code with a
36-test suite (all green in the nofailfast baseline, sha256-pinned ffmpeg fixtures, a
measured-truth differential) — and none of it can be reached from the shipped `lumen`
binary: no probe signature recognizes ASF, no container arm opens it, and the repo's own
compat-matrix note records the end-to-end refusal (`probe.no-demuxer`, extension=wmv). So
the file's verdict is UNREACHABLE, honestly recorded as `partial` in the matrix with the
wiring next-step named. Within the code itself: the grammar is corpus-derived by the
authors' own admission ("the spec-acquisition gate never opened", [VERIFY-P0]) — every
unobserved shape refuses typed, which is the honest posture, but it means non-ffmpeg ASF
producers' files are refused on layout guesses; a mid-object fragment offset mismatch is a
sticky hard error where every sibling demuxer's damage policy recovers; and two
doc-code contradictions (a "refused" arm that silently returns Ok(None), and a documented
error code the code does not emit).

## 2. What actually works (verified)
— 36 asf tests ok in nofailfast baseline (e.g. `asf::tests::fragments_reassemble_into_whole_objects`,
`malformed_packets_refuse_typed_never_guess`, `assembler_refusals_are_typed`,
`fixture_family_walks_match_the_measured_truth` — log lines 1561-1590 region, all `ok`).
— Panic-surface traced: every field read is guarded (`bytes.first()`, `bytes.get(pos)`,
`read_field` width checks, `checked_add` on `end`, `pos+8`/`pos+6`/`pos+2` pre-checks
before `u16le`/`u32le`); `Vec::with_capacity(payload_count as usize)` is bounded by the
6-bit count (≤63); the reassembly ceiling is enforced at both the first fragment
(size ≤ MAX_OBJECT_BYTES, data.rs:416-418) and continuation (390-391). No index, slice,
unwrap or arithmetic panic path found.
— The exact-close invariant (285-287: payloads + padding == slice length) plus the
per-payload bound `end > bytes.len() - padding` (269-271) make overruns typed, not
guessed — and `malformed_packets_refuse_typed_never_guess` (baseline ok) pins it.
— The hold-and-flush contract is real and unit-pinned: `fragments_reassemble_into_whole_objects`
(asf/tests.rs:609-639) drives 4+6-byte fragments to a completed 10-byte object with pts
999 and the abandon-on-new-number rule.

## 3. Findings

### F-001 [S3] The whole ASF lane — this file included — is unreachable from the shipped binary
Where: apps/cli/src/containers/mod.rs:49-74 (absence); evidence docs/feature-matrix.toml:217
Code (the dispatch this file never gets called from — containers/mod.rs:72-75):
```
        "mpegts" => ts::open(stream, opts),
        "wav" => wav::open(stream, opts),
        other => Err(unwired_refusal(other)),
    }
```
What is wrong: the CLI container registry has 14 arms (adts/aiff/avi/ivf/matroska/
flac/hevc/h264/mp2/mp3/mp4/ogg/mpegps/mpegts/wav); `asf` is not among them, media-probe
has no ASF signature (14 signature structs enumerated, none ASF), and a repo-wide search
finds zero users of `AsfDemuxer`/`asf::` outside the demux crate itself. A user opening a
.wmv/.wma/.asf file is refused at the probe stage (`probe.no-demuxer`).
Why it matters: ~2,000+ lines of tested demuxer (data.rs here is 462) ship in the binary
dead; the "Demux ASF container" capability exists only behind `cargo test -p demux`.
Mitigating, and why this is S3 not S2: the gap is honestly recorded — the feature-matrix
row `container-asf-wmv` was demoted `implemented→partial` by the repo's own triage with
the sniffed GUID bytes pasted and the next step named ("wire the missing probe signature /
container-registry arm"), and container-asf.md's status line says "implemented (demux)" —
a layer-scoped claim, not a user-facing one.
What correct looks like: the wiring unit the matrix note already names (the ivf/mpegps
open-path precedent), or the demux tree stays behind a feature flag until then.
Confidence: HIGH — Verification: three independent greps (probe signatures, container
arms, repo-wide callers) + the repo's own compat-matrix note.

### F-002 [S3] The packet grammar is corpus-derived; the spec was never acquired — the padding-bits reading is a guess that fits ffmpeg output
Where: crates/demux/src/asf/data.rs:3-7 (admission) and 152-170 (the reading)
Code (data.rs:3-7):
```
//! The packet/payload bit grammar this module parses is the
//! ffmpeg-7.1.5-verified subset (the spec-acquisition gate never
//! opened — see the worklog's [VERIFY-P0] verdict; every layout
//! below was byte-decoded against live muxer output and closes each
//! packet exactly):
```
Code (data.rs:152-153, 169):
```
    let padding = match f1 & 0x18 {
        0x00 => 0,
```
```
        _ => return Err(error::packet_bad_flags(pos as u64)),
```
What is wrong: the module reads the padding-length selector as two individual bits
(0x10 = 2-byte field, 0x08 = 1-byte field, both-set refused). The ASF specification
defines the data-packet flag bits as paired 2-bit length-type fields (packet length,
sequence, padding) — whether bit 4 alone is the correct 2-byte selector cannot be
verified in this sandbox (no spec text; UNVERIFIED). Everything closes exactly against
ffmpeg 7.1.5 muxer output, which proves self-consistency with one producer, not
conformance: files from other ASF writers (Windows Media Encoder, older VLC muxing, any
of the spec's legal shapes) hit `packet.bad-flags`/`packet.bad-length` typed refusals.
The same holds for the f1 bits-1-0 == 01-only rule (146-148) and the property-flags
subset (172-189).
Why it matters: "Demux ASF container incl. WMV payloads" is claimed implemented at the
demux layer with ffmpeg as the only oracle; the [VERIFY-P0] spec pin is still open
(feature-matrix.toml:217 notes "spec text unavailable in sandbox"). The blast radius is
bounded by the honest typed refusals and by F-001 (nothing shipped reaches this code
anyway).
What correct looks like: acquire the spec (the plan's own gate,
docs/plans/asf-selfbuild.md:337-342), pin the flag-bit tables against it, and record the
revision + URL in container-asf.md as the gate requires.
Confidence: HIGH on the facts (admission verbatim; reading traced); UNVERIFIED on the
spec's actual bit layout (blocked: no spec in sandbox).

### F-003 [S3] A mid-object offset mismatch is a sticky hard error — every sibling demuxer's damage policy recovers instead
Where: crates/demux/src/asf/data.rs:383-392
Code:
```
            Some(pending) if pending.number == media_object => {
                // Continuation fragment (must land exactly at the end).
                let start = pending.buffer.len() as u32;
                if offset != start {
                    return Err(error::packet_bad_length(offset as u64));
                }
```
What is wrong: if a single fragment of a multi-packet media object is lost or reordered,
the next continuation arrives with `offset != buffer.len()` and the parse returns a typed
error that (per the demuxer's sticky policy) terminates the stream. The sibling damage
policies all recover: ADTS bounded-resyncs past a lost syncword (mod.rs:50-57), AVI
resyncs the movi walk (avi/mod.rs:780-802), and even this module's own orphan-fragment
case (offset ≠ 0 with no open object, 407-413) skips and re-synchronizes. One lost
packet in a real-world capture kills playback here where ffmpeg (which does not
reassemble at all — it emits fragments) keeps going.
Why it matters: ASF's raison d'être is streaming/network capture — exactly the damaged
input class this policy refuses. The hold-and-flush contract doc sells damage-tolerance
("a gap the container declares by moving on", 356-359) but a *within-object* gap is fatal.
What correct looks like: treat the offset mismatch as the declared gap it is — drop the
incomplete object (damage-counted), skip until the next offset-0 fragment, mirroring the
orphan rule; keep the typed error only for the truly impossible (offset going backwards
inside one fragment sequence).
Confidence: MEDIUM-HIGH — code path traced; asf/mod.rs has no sticky mechanism (grep for
`fn fail`/`sticky` → 0 hits; the error propagates from `next_event` via `?` at 469/748),
and the failing packet's loop aborts before the walk advances — the stream stops
delivering either way; the exact retry-vs-terminal behavior on the next call is the
asf/mod.rs owner's to pin. Verification: read.

### F-004 [S4] Two doc-code contradictions: a "refused" arm that returns Ok(None), and a documented error code the code never emits for that case
Where: crates/demux/src/asf/data.rs:377-381 (silent skip) and 364-367 vs 390-391 (error code)
Code (377-381):
```
        let Some(slot) = self.slots.get_mut(slot) else {
            // Unknown stream: the demuxer skips the payload before
            // this point; a bad index here is a caller bug, refused.
            return Ok(None);
        };
```
Code (365-367 doc vs 389-391 code):
```
    /// Typed refusals: `payload.oversized` when the object exceeds
    /// [`MAX_OBJECT_BYTES`], `packet.bad-length` when a fragment
```
```
                if new_len > u64::from(pending.size) || new_len > MAX_OBJECT_BYTES {
                    return Err(error::payload_oversized(MAX_OBJECT_BYTES, new_len));
                }
```
What is wrong: (a) the else-arm's comment says "a caller bug, refused" but the code
returns `Ok(None)` — a silent drop, nothing refused; a real caller bug would be
invisible. (b) The `# Errors` doc promises `packet.bad-length` when "a fragment
overruns the declared object size", but the continuation path returns
`payload_oversized` for BOTH the size overrun and the ceiling; only the first-fragment
path (419-420) matches the doc. Also cosmetic: every `truncated` call mints
`expected = len+1, found = len` (e.g. 139, 156, 163) regardless of the real shortfall —
the message always claims "one more byte needed".
What correct looks like: `debug_assert!` + a typed caller-bug error in the else-arm (or
an honest comment); split the overrun from the ceiling per the doc; compute the real
expected size in the truncation constructor.
Confidence: HIGH — read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-49 | REAL (honest) | S3 | corpus-derived admission verbatim (F-002); house rules verified: std-only, no unsafe, no unwrap/expect/panic in non-test code (grep clean) |
| `MAX_OBJECT_BYTES` | 55-58 | REAL | — | 32 MiB ceiling, enforced at both feed paths |
| `ParsedPayload`/`ParsedPacket` | 62-96 | REAL | — | data_range borrows, length==end-start by construction |
| `type_width` | 99-106 | REAL | — | 1/2/4 widths; 0 only legal for offset |
| `u16le`/`u32le` | 108-114 | REAL | — | all call sites pre-checked (139,156,163,192-198,236-239) — no panic |
| `parse_packet` | 129-296 | REAL | S3 | exact-close invariant; every field guarded; padding/flag readings corpus-derived (F-002); plen_width==0 multi-payload degrades to typed bad-length (acceptable); `truncated(len+1, len)` synthetic (F-004c) |
| `read_field` | 299-313 | REAL | — | width-checked, returns None → typed by caller |
| `PendingObject` | 318-324 | REAL | — | |
| `ObjectAssembler::new`/`clear` | 335-346 | REAL | — | one slot per stream; clear for reanchor |
| `ObjectAssembler::feed` | 368-442 | REAL | S3/S4 | hold-and-flush traced correct (first-fills held, continuation closes, predecessor flush on next fragment, orphan skip); offset-mismatch sticky error (F-003); silent else-arm + doc'd-but-unemitted error code (F-004); ceiling enforced twice |
| `ObjectAssembler::take_completed` | 450-461 | REAL | — | EOS drain half of the contract; unit-pinned |

## 5. Test honesty (asf/tests.rs — 36 tests, all baseline-ok)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| fragments_reassemble_into_whole_objects | YES | exact buffer concat, pts, keyframe, abandon rule hardcoded |
| malformed_packets_refuse_typed_never_guess | YES | per-shape error codes pinned |
| assembler_refusals_are_typed | YES | typed refusal codes pinned |
| fixture_family_walks_match_the_measured_truth | YES for the ffmpeg lane | sha256-pinned fixtures + measured truth (header size/children/data-object facts); oracle is ffmpeg/ffprobe — same producer the grammar was derived from (F-002's ceiling) |
| broadcast_fixture_refuses… / header ceiling… / indexless seeks… | YES | typed codes and bounds pinned (baseline ok) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Status: implemented (demux)" | container-asf.md:3 | REAL (layer-scoped) | 36/36 tests ok in baseline; but the open path refuses ASF (F-001) — the matrix itself demoted the row to partial |
| "every layout below was byte-decoded against live muxer output" | data.rs:4-7 | REAL (self-consistent) | sha256-pinned fixture differential; one-producer oracle (F-002) |
| "a fragment whose head was never seen … is an orphan: skipped, never fatal" | data.rs:359-362 | REAL | traced 407-413 + unit test |
| "packet.bad-length when a fragment overruns the declared object size" | data.rs:366-367 | FALSE (for continuations) | code emits payload_oversized at 390-391 (F-004b) |
| "a bad index here is a caller bug, refused" | data.rs:379-380 | FALSE | returns Ok(None) (F-004a) |
| matrix row container-asf-wmv linux status | feature-matrix.toml:211-217 | partial (honest) | the row's own notes record probe.no-demuxer with sniffed bytes |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable from shipped builds (F-001) — dead weight in the
shipped `demux` rlib until the wiring unit lands. No internal dead code found.

## 8. Dependencies & flags
Std + media_types only (verified imports at 51-53); no feature gates; no unsafe; the
crate-level lints posture (no panic/unwrap in non-test code) holds in this file.

## 9. Open questions for the next agent
1. asf/mod.rs: does the demuxer make `packet_bad_length` from feed() sticky (F-003's
   blast radius depends on it)? Callers at mod.rs:469/748 propagate with `?` — check
   whether either wraps in `fail()`.
2. The property-flags payload-length "00 = absent (final payload only)" rule: the code
   applies absent-length to EVERY payload when plen_width==0 (256-258) — multi-payload
   packets with absent lengths degrade to typed refusal; confirm no fixture exercises
   this and the spec pin covers it.
3. If the ASF wiring unit (matrix note's next step) lands, re-run this audit's F-001
   reachability greps — the verdict flips to REAL-UNPROVEN (no end-to-end run) at that
   point.
