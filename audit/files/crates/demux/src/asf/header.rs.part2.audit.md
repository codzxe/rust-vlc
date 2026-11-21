# AUDIT: crates/demux/src/asf/header.rs#part2 (lines 543-1085)
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-21 |
| Lines / functions | 543 audited / 9 (children-walk body of `parse_at` + 8 private fns) |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point reaches it — see below) |
| Reachable from shipped entry point? | NO. `AsfHeader::parse` is called only at `crates/demux/src/asf/mod.rs:641` (`AsfDemuxer::start`); `AsfDemuxer::new` has no caller outside `crates/demux/src/asf/tests.rs` (rg over crates/ + apps/). The CLI open path has no ASF arm: `apps/cli/src/containers/mod.rs:50-75` (15 ids, no `asf`); `crates/media-probe` has no ASF signature (rg `asf\|wmv\|wma` in media-probe/src: zero hits). The matrix itself records it: docs/feature-matrix.toml:217 "the open path REFUSES the corpus files of probe format `asf` — unsupported … (probe.no-demuxer)". |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 4 |
| Commands run | `rg -n "AsfDemuxer::new\|AsfHeader::parse"` (callers: mod.rs:641 + tests only); `rg -i "asf\|wmv\|wma" crates/media-probe/src` (empty); `rg -n "reserved_matches"` (3 sites in header.rs + 2 test asserts — no production consumer); `rg -n -A8 "pub fn u16le"` guid.rs (OOB → returns 0, no panic); `rg -n -B4 -A14 "pub struct DamageLog"` avi/header.rs; `rg -n "u64::MAX\|checked_add\|wrapping"` asf/ (only header.rs:1054 hardens); `rg -n -i "asf" docs/feature-matrix.toml`; baseline greps; `sed -n` verification of every quote. |

Chunk boundary: nominal part2 = 543-1085 (part1 = 1-542, A04's — structs, constants,
`utf16_lossy`, `codec_descriptor`, `parse`, `parse_at` head). My range: the
`parse_at` children-walk body (543-701) through end of file. All line numbers
below are in asf/header.rs unless stated.

## 1. One-paragraph verdict
Real, non-stub parsing code with genuine bounds discipline (every table constant
proposed-and-documented, every read prefix-capped, 36 asf tests green in the
baseline — `orchrun-test-workspace-nofailfast.log.txt:1555-1589` including the
ffmpeg fixture differential at 1567 and the live-ffprobe cross-check at 1953) —
but none of it is reachable from the shipped binary, and the children walk has
one unhardened arithmetic hole the sibling walk three screens below closed: a
child object whose QWORD size is near `u64::MAX` wraps `off + size`, passes the
overrun check, and produces a reversed slice range — a panic on crafted input
(S2). Around it: a damage ledger whose documented "sum of events" invariant is
broken by direct total mutation with mixed byte/count units (S3), a
documented-but-nonexistent damage note for Data-Object reserved bytes (S3), and
a Data-Object header read that ignores the object's declared size (S3), plus
four hygiene defects. The module docs are unusually honest about what was
byte-verified vs recalled — that honesty is undermined by the two doc/code
drifts below.

## 2. What actually works (verified)
— 36/36 asf tests pass in baseline (log 1555-1589 + 1953), including
   `fixture_family_walks_match_the_measured_truth ... ok` and
   `fixture_family_cross_checks_against_live_ffprobe ... ok`.
— Bounds constants all present and used: MAX_HEADER_BYTES 4 MiB (74, checked
   513), MAX_HEADER_CHILDREN (79, checked 549), MAX_TOP_OBJECTS (83, checked
   1024), MAX_EXTENSION_CHILDREN (86, checked 994), MAX_CODEC_LIST_ENTRIES
   (90, 864/917), MAX_METADATA_ENTRIES (101, 937), field caps (94, 98).
— `u16le/u32le/u64le` are panic-free on short buffers (guid.rs:184-189: OOB
   returns 0 — verified read), so the codec-list cross-check at 666 cannot
   panic on short `info` blobs.
— `walk_top_level` uses `checked_add` for `pos + size` (1054) — overflow-safe.

## 3. Findings
### F-001 [S2]
Where: crates/demux/src/asf/header.rs:555-562
Code:
```
            let size = guid::u64le(&bytes, off + 16);
            if size < 24 {
                return Err(error::bad_object_size(header_offset + off as u64));
            }
            let end = off as u64 + size;
            if end > header_size {
                return Err(error::bad_object_size(header_offset + off as u64));
            }
            let payload = &bytes[off + 24..end as usize];
```
What is wrong: `end = off as u64 + size` is unchecked arithmetic on a
file-controlled QWORD. For `size` in the ~53 values just below `u64::MAX`
(e.g. `0xFFFF_FFFF_FFFF_FFFF` at off=30), the add wraps (release profile:
silently; debug: 'attempt to add with overflow' panic), `end` becomes
`off - 1`, the `end > header_size` overrun check passes (off−1 < header_size),
and `&bytes[off + 24..end as usize]` is a slice with start > end — panic
("slice index starts at 54 but ends at 29" shape). The author hardened the
identical pattern in the same file's top-level walk — line 1054 uses
`pos.checked_add(size).is_none_or(...)` — but not here.
Why it matters: a crafted ASF/WMV file crashes the demuxer thread instead of
refusing typed; violates the crate's own house rule (lib.rs:38-39: no panics in
non-test code). Not reachable through the shipped CLI today (no probe
signature, no registry arm), which is why this is S2 and not S1 — it becomes a
user-facing crash the moment the asf arm lands. No test covers it: the overrun
suite patches the HEADER size to 9_999 (`asf/tests.rs:177-190`), never a child
size near u64::MAX, and the fuzz sweep's random QWORD space makes hitting a
~53-value window vanishingly unlikely (baseline `hostile_sweeps_survive_without_panics ... ok`
proves nothing about this window).
What correct looks like: `let end = off.checked_add(size)...` / use
`u64::checked_add` and refuse `bad_object_size` on overflow — exactly the
1054 pattern; plus one regression test with size `u64::MAX`.
Confidence: HIGH — Verification: read + arithmetic reasoning (no cargo runs allowed; REQUEST-ORCH-RUN below).

### F-002 [S3]
Where: crates/demux/src/asf/header.rs:609-623 (with 896-898, 967, 1004)
Code:
```
                CODEC_LIST => {
                    let (entries, noted) = Self::parse_codec_list(payload)?;
                    damage.total = damage.total.saturating_add(noted);
```
What is wrong: Three call sites (611, 616, 622) mutate `damage.total` directly,
bypassing `DamageLog::note` — breaking the ledger's documented invariant
"Total skipped bytes (the sum of `events`)" (`crates/demux/src/avi/header.rs:390-391`).
Worse, the units are mixed: `parse_codec_list` adds whole `payload.len()` byte
counts (896-898, 867), `parse_extended_content_description` adds **1 per
non-string metadata pair** (967: `noted += 1`), and `walk_header_extension`
adds **1 per non-ESP child** (1004) — counts poured into a bytes total.
Why it matters: `AsfDemuxer::damage_total()` (mod.rs:363-364, doc "Total
bytes/objects … skipped") surfaces this number; any consumer reading it as
bytes (the DamageLog contract) gets nonsense after any ECD/header-extension
damage. The event list also loses the reason strings for these skips — the
"never silent" damage policy (avi/header.rs:382-383: "repairs are counted and
surfaced, never silent") is only half-kept.
What correct looks like: return damage notes as `(bytes, reason)` pairs and
push them through `damage.note`, keeping counts and bytes in separate fields.
Confidence: HIGH — Verification: read (all five sites + DamageLog contract traced).

### F-003 [S3]
Where: crates/demux/src/asf/header.rs:1051 (doc claim at 313-315, part1's range)
Code:
```
                data = Some(DataObjectLocation {
                    offset: pos,
                    size,
                    packet_count: guid::u64le(&body, 16),
                    reserved_matches: body[24..26] == [0x01, 0x01],
                });
```
What is wrong: The `reserved_matches` field's doc says "damage-noted when not;
never fatal" (lines 313-315) — but no code anywhere notes damage for a reserved
bytes mismatch. rg over the whole repo: `reserved_matches` appears only at its
definition (316), this producer (1051), the no-data default (1078), and two
test asserts (`asf/tests.rs:123,1163`). It is production-write-only: no
consumer reads it, no damage note exists for the false case.
Why it matters: a documented behavior that does not exist — the exact
doc-vs-code drift this audit exists to find. A file with non-`01 01` reserved
bytes passes with zero observable note, contradicting the field's contract.
What correct looks like: either `damage.note(0, "data-reserved-bytes")` when
false, or delete the doc claim and the field.
Confidence: HIGH — Verification: read + repo-wide rg (5 hits total, enumerated above).

### F-004 [S3]
Where: crates/demux/src/asf/header.rs:1040-1052
Code:
```
            if guid_at == DATA && data.is_none() {
                // Data Object payload head: FileID(16) + total packet
                // count QWORD(8) + 2 reserved bytes (observed 01 01).
                let body = reader.read_prefix_at(26, pos + 24)?;
```
What is wrong: The 26-byte Data-Object header read happens whenever the GUID
is DATA, with only the generic `size >= 24` floor from 1032 — no check that
the declared size covers the 50 bytes (24 header + 26 body) being read. A DATA
object declaring size 24..49 mid-file makes `read_prefix_at(26, pos + 24)` read
into the FOLLOWING object's bytes; `packet_count` is then parsed from foreign
bytes and stored as fact, silently (no damage note, no typed refusal). Only at
EOF does the short read produce the typed truncation (1044-1045).
Why it matters: a malformed (or adversarially minimal) Data Object yields a
bogus packet count in `DataObjectLocation` — the fact ASF-b will key its packet
walk off. Silent wrong-data on malformed input is the failure class this
demuxer's own damage policy forbids.
What correct looks like: `if size < 50 { damage.note(size, "data-object-too-small"); }` (or a typed refusal) before the 26-byte read.
Confidence: HIGH — Verification: read (no intervening size check between 1032 and 1043).

### F-005 [S4]
Where: crates/demux/src/asf/header.rs:649-653
Code:
```
        let Some(properties) = properties else {
            // An ASF without File Properties cannot even be refused
            // honestly as broadcast — structurally incomplete.
            return Err(error::bad_object_size(header_offset));
        };
```
What is wrong: A MISSING File Properties Object is reported as
`demux.asf.bad-object-size` — but no object has a bad size; the object is
absent. The taxonomy (documented at 466-470) has no missing-FPO code, so the
closest semantic was misused.
Why it matters: error-code consumers (and users) are told the structure is
bad-sized at `header_offset` when the real condition is "mandatory object
absent" — misleading diagnostics on damaged files.
What correct looks like: a dedicated `demux.asf.no-file-properties` constructor.
Confidence: HIGH — Verification: read; error.rs code list at 466-470 confirms no such code.
### F-006 [S4]
Where: crates/demux/src/asf/header.rs:937 (contrast 917-919)
Code:
```
        for _ in 0..count.min(MAX_METADATA_ENTRIES) {
```
What is wrong: An ECD declaring more than 1024 entries is silently truncated
at the cap — no damage note. The codec list handles the identical condition
with `noted += (count - MAX_CODEC_LIST_ENTRIES) as u64` (917-919). Inconsistent
policy for the same shape.
What correct looks like: note the excess entries like the codec list does.
Confidence: HIGH — Verification: read, both loops traced.
### F-007 [S4]
Where: crates/demux/src/asf/header.rs:900 (also 940, 950, 989)
Code:
```
            if off + 2 > payload.len() {
                return Err(error::bad_object_size(0));
            }
```
What is wrong: `bad_object_size(0)` hard-codes offset 0; the constructor puts
it in `CorruptDetail::InvalidStructure { offset: Some(0) }` (asf/error.rs:65-73)
— the typed error then claims the corruption is at byte 0 of the file.
Why it matters: false position info in diagnostics; the sibling errors in this
file pass real offsets (556, 560, 569…).
What correct looks like: thread the payload's absolute offset into these four
call sites (the parse fns already receive `payload` but not its offset — add it).
Confidence: HIGH — Verification: read; constructor signature traced.
### F-008 [S4]
Where: crates/demux/src/asf/header.rs:748
Code:
```
        let _ = ecd_len; // Error-correction payloads are ASF-b's parse.
```
What is wrong: The ECD length is parsed (733) and discarded. Documented as
ASF-b's concern, but the object's own consistency (54 + tsd_len + ecd_len ≤
payload) is never checked — a TSD+ECD pair overrunning its object is silently
tolerated with no damage note, unlike every other overrun in this file.
What correct looks like: `if 54 + tsd_len + ecd_len > payload.len() as u64 { damage.note(...) }` — keep skipping the bytes, but record it.
Confidence: HIGH — Verification: read; no other use of ecd_len in the fn.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_at` (children-walk body) | 543-701 | REAL | S2 | F-001 wrap panic; duplicate-FPO/SP first-wins rules damage-noted (566-567, 598-601); command/unknown streams skipped with reasons (591-597); codec-list cross-check (664-680) degenerately "agrees" on tag 0 vs empty info (u16le OOB → 0) — advisory only; no-FPO mislabel (F-005); broadcast/empty-streams typed refusals (654-659). |
| `parse_file_properties` | 705-719 | REAL | — | Pure field extraction at pinned offsets, payload ≥ 80 guaranteed by caller (568). No panic paths. |
| `parse_stream_properties` | 725-775 | REAL | S4 | TSD bounded (MAX_TSD_BYTES, 737-739); truncation typed; `let _ = ecd_len` (F-008); tsd_bytes cloned into SharedBytes (773) — one bounded alloc per stream. |
| `parse_waveformatex` | 779-806 | REAL | — | 18-byte floor + cbSize-tail closure checked (802-804); channels/rate zero refused; bits/align unvalidated (downstream refuses — acceptable). |
| `parse_video_info` | 816-849 | REAL | — | data_size ≥ 40 floor; bi_size ∈ [40, data_size] guard for unverified shapes (833-838); `extradata_len = data_size - 40` is correct for the measured family (biSize = 40+extradata per ffmpeg) but unexercised for data_size > bi_size — module doc flags it honestly. |
| `parse_codec_list` | 854-921 | REAL | S4 | Bounded (entries cap 256, field cap 4 KiB); over-long display strings clamp and abort remaining entries with `noted += payload.len()` (896-898) — whole-payload byte count, sloppy but bounded; short-payload (<20) silent empty return (858-860) — no note; mid-entry info-len truncation is a hard `bad_object_size(0)` error (899-901) — inconsistent with the soft string handling and offset-less (F-007). |
| `parse_extended_content_description` | 927-974 | REAL | S4 | Non-string types counted not guessed (966-968) — but count-unit into byte-total (F-002); silent cap truncation (F-006); three `bad_object_size(0)` sites (F-007); name/value clamps use saturating_sub — no panic. |
| `walk_header_extension` | 982-1010 | REAL | S4 | data_size end-check typed; children counted, ESP vs non-ESP distinguished, non-ESP noted as counts (F-002); the `off as u64 + size` wrap (996) walks backwards but is bounded by MAX_EXTENSION_CHILDREN (1024) — no panic, minor. |
| `walk_top_level` | 1014-1085 | REAL | S3 | checked_add-hardened (1054) — the pattern F-001 misses; DATA min-size hole (F-004); overrun DATA damage-noted and honest location kept (1055-1061); no-DATA zeroed location + note (1068-1082) documented. |

## 5. Test honesty (tests live in asf/tests.rs — A02/A07's chunks; assessed only as evidence for this code)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| fixture_family_walks_match_the_measured_truth (baseline 1567) | YES | Byte-exact walks against sha-pinned ffmpeg fixtures (per matrix evidence string). |
| fixture_family_cross_checks_against_live_ffprobe (baseline 1953) | YES | Live ffprobe codec + count cross-check (needs corpus; passed in baseline run). |
| header_size_below_floor_and_overrun_refuse_bad_object_size (baseline 1570) | PARTIAL | Covers header-size floor/overrun (9_999) — does NOT cover child-size wrap (F-001 gap). |
| hostile_sweeps_survive_without_panics (baseline 1562) | NO (for F-001) | Random mutation almost surely never lands in the ~53-value QWORD window that triggers the wrap panic. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| `container-asf-wmv` status windows=implemented, macos=implemented | docs/feature-matrix.toml:215 | FALSE | The open path is platform-INDEPENDENT code with no asf arm (containers/mod.rs:50-75) and no probe signature (rg empty); the same probe.no-demuxer refusal the matrix records for linux applies identically on windows/macos. Only linux=partial is honest. |
| "36-test asf unit suite in the 454-test demux crate" | feature-matrix.toml:216 evidence | REAL (stale count) | 36 asf tests confirmed (baseline 1555-1589); demux crate now runs 462 tests (baseline 1489-1490) — evidence string not refreshed. |
| DataObjectLocation.reserved_matches "damage-noted when not" | header.rs:313-315 | FALSE | F-003: no such note exists; field is production-write-only. |
| Module docs' byte-verified layouts (FPO 80 B, SP offsets, WFX, short video form, codec list, ECD) | header.rs:9-56 | REAL | `fixture_family_walks_match_the_measured_truth ... ok` (baseline 1567) — walks close the fixtures exactly. |
| "[VERIFY-P0] spec pin recorded … clean-room" | feature-matrix.toml:217 notes | REAL-UNPROVEN | Honest labeling; no spec exists in-repo to re-verify against; ffmpeg-output verification is the only oracle and it passes (1567, 1953). |

## 7. Dead / duplicated / unreachable code
- The entire file is unreachable from shipped entry points (see header table) —
  real code, dead lane pending the probe-signature/registry wiring the matrix
  notes promise.
- `reserved_matches`: production-write-only (F-003).
- `TopObject`/`skipped_objects`/`metadata`/`codec_list` are exposed on
  `AsfDemuxer` getters (mod.rs:337-357) with no shipped consumer — same
  unreachable-lane caveat, not dead within the crate's own API.

## 8. Dependencies & flags
No feature gates. Reuses `crate::avi::header::DamageLog` and
`crate::avi::riff::SourceReader` (cross-module reuse, documented). No unsafe,
std-only. All tests ran in baseline default selection (release profile).

## 9. Open questions for the next agent
- F-001 needs a regression test once a fix lands; REQUEST-ORCH-RUN (see log):
  a targeted test constructing a child QWORD size of `u64::MAX` at header
  offset 30 asserting a typed refusal (would panic today, release or debug).
- asf/tests.rs#part2 (my next file) — check whether the fixture-differential
  truths pin `damage_total()` values anywhere (F-002's unit mixing would then
  be observable, not just latent).
- Whoever wires the asf probe arm: F-001 escalates from S2 to user-facing
  crash on day one of that wiring.
