# AUDIT: crates/demux/src/asf/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-21 |
| Lines / functions | 780 / 25 |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO (call chain proof below) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 3 S4: 2 |

## 1. One-paragraph verdict
The ASF/WMV/WMA demuxer core is genuinely implemented — header walk, fixed-packet Data
Object walk with fragment reassembly and hold-and-flush EOS drain, Simple-Index seek plus
a bounded linear fallback — and its ~36 unit tests all pass in the baseline run
(`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt`, `asf::tests::*` … ok,
suite totals 4534/7/49). But none of it is reachable from the shipped `lumen` binary: no
media-probe signature names the format, and the CLI container registry has no `asf` arm,
so every .asf/.wmv/.wma open dies at `probe.no-demuxer` — a fact the repo's own feature
matrix notes admit for linux while its windows/macos cells still say "implemented". On
top of that, the module docs falsely claim the media-probe signature landed, and the
struct docs still describe the pre-ASF-b state. Real code, dead in the shipped artifact.

## 2. What actually works (verified)
- `asf::tests::*` (28 unit tests) + `asf::fuzz::tests::hostile_sweeps_survive_without_panics`
  + `asf::guid/tests` + `asf::error/tests` all `ok` — verified by
  `rg "asf::" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → 40 asf test
  lines, zero failures (baseline suite totals 4534 passed / 7 failed / 49 ignored; the 7
  failures are desktop/first-vertical/localization, none demux).
- Packet-grammar bounds: `data::parse_packet` (read in full, crates/demux/src/asf/data.rs:129-296)
  validates every payload `data_range` against `bytes.len()` (line 269:
  `if end > bytes.len().saturating_sub(padding as usize)`), so the slice at mod.rs:482
  cannot panic. Verified by reading data.rs.
- `SimpleIndex::parse` (mod.rs:205-245): length floor 56, exact-close `(len-56) % 6 == 0`,
  count cross-check, allocation capped by the caller's `MAX_INDEX_BYTES` (1 MiB,
  checked at mod.rs:604 before any read). No unbounded allocation.

## 3. Findings

### F-001 [S2] Entire demuxer unreachable from the shipped entry point
Where: crates/demux/src/asf/mod.rs:142-154 (the `AsfDemuxer` struct no one constructs)
Code:
```rust
pub struct AsfDemuxer {
    reader: SourceReader,
    started: bool,
    header: Option<header::AsfHeader>,
    tracks: Option<Vec<Track>>,
```
What is wrong: No code outside `crates/demux/src/asf/*` references `AsfDemuxer`,
`asf::probe`, or `asf::EXTENSIONS` (rg across `crates/ apps/ tests/`). The shipped open
path is `lumen open` → probe (crates/media-probe) → `apps/cli/src/containers/mod.rs::open`,
whose match arms are exactly `adts aiff avi flac h264 hevc ivf matroska mp2 mp3 mp4 ogg
mpegps mpegts wav` — no `asf`. crates/media-probe/src contains zero asf/wmv/wma
references, so the probe can never even mint the id.
Why it matters: A user opening a .wmv/.wma in the shipped binary gets
`probe.no-demuxer` while docs/feature-matrix.toml:212 claims
`status = { windows = "implemented", linux = "partial", macos = "implemented" }`. The
repo's own matrix notes (row `container-asf-wmv`) record the refusal verbatim: "the open
path REFUSES the corpus files of probe format `asf` … (probe.no-demuxer)". ~2 kLOC of
tested demuxer compiled into the binary as dead weight.
What correct looks like: A media-probe signature emitting `asf` plus a
`containers/asf.rs` wiring unit and one registry arm — exactly the "Next step" the
matrix notes name; then the win/macos cells would have something to stand on.
Confidence: HIGH — Verification: rg "AsfDemuxer" crates/ apps/ tests/ → only
crates/demux/src/asf/*; rg -i "asf|wmv|wma" crates/media-probe/src/ → no matches;
sed -n '36,68p' apps/cli/src/containers/mod.rs (match arms read).

### F-002 [S2] Module docs claim a media-probe signature that does not exist
Where: crates/demux/src/asf/mod.rs:5-9
Code:
```rust
//! List, Extended Content Description, a bounded Header Extension
//! child walk — plus the [`probe`] gate and the media-probe
//! signature. **ASF-b landed the Data Object packet walk**:
```
What is wrong: "plus the probe gate and the media-probe signature" asserts the
media-probe integration landed. crates/media-probe/src has no asf signature (zero
matches for asf/wmv/wma, case-insensitive). The only "probe" that exists is the local
3-line `asf::probe` (mod.rs:96-98), which nothing outside tests/fuzz calls.
Why it matters: A reader of this module believes the format is wired into the open path.
It is not — the integration claim is false, and the false claim is the cover under which
matrix row `container-asf-wmv` carries two "implemented" statuses.
What correct looks like: Doc states the wiring state honestly: "probe gate local; open
path wiring pending (matrix note, issue #53)".
Confidence: HIGH — Verification: rg -i "asf|wmv|wma" crates/media-probe/src/ → nothing;
read mod.rs:96-98.

### F-003 [S3] `ticks_for` ignores its `track` parameter and clamps overflow to a bogus far-future stamp
Where: crates/demux/src/asf/mod.rs:570-585
Code:
```rust
    fn ticks_for(track: &Track, base: TimeBase, pts_ms: u32) -> i64 {
        let _ = track;
        let ms = i64::from(pts_ms);
        let (num, den) = base.as_parts();
        if den == 1000 && num == 1 {
```
and (mod.rs:580-583)
```rust
            let rate = i64::from(den) / i64::from(num.max(1));
            ms.checked_mul(rate)
                .and_then(|t| t.checked_div(1000))
                .unwrap_or(i64::MAX)
```
What is wrong: The `track` argument is dead (`let _ = track;`). Overflow (ms × rate
> i64::MAX — reachable with a crafted WAVEFORMATEX rate near u32::MAX and late stamps)
silently produces `i64::MAX` — a stamp 292 million years in the future presented as a
valid `Stamp::Present`, not a typed refusal.
Why it matters: Corrupt input yields a plausible-looking but garbage PTS instead of the
crate's own typed-corruption discipline; dead parameter misleads readers about the
function's inputs.
What correct looks like: Drop the parameter; on overflow return the typed
`bad_waveformatex`/corruption error (the function's callers already thread `Result`).
Confidence: HIGH — Verification: sed -n '567,585p' (quote verified); overflow reachable:
rate ≤ 4.29e9 (u32), ms ≤ 4.29e9 → product up to 1.8e19 > 9.2e18.

### F-004 [S3] Unvalidated u64 preroll clamps to u32::MAX and zeroes every PTS
Where: crates/demux/src/asf/mod.rs:508-509 (and identical pattern 551-552)
Code:
```rust
            let preroll = u32::try_from(preroll).unwrap_or(u32::MAX);
            let pts_ms = presentation_time_ms.saturating_sub(preroll);
```
What is wrong: `preroll_ms` is read raw from the File Properties Object
(header.rs:713 `preroll_ms: guid::u64le(payload, 56)`) with no bound check. A file
declaring preroll > 4 294 967 295 ms makes every packet's `pts_ms` collapse to 0 —
all packets stamped at t=0 with no error, instead of a typed bad-object refusal.
Why it matters: Malformed input silently produces a broken timeline (every frame "at
the start"), the exact class of silent corruption this crate's error taxonomy exists
to refuse.
What correct looks like: Validate `preroll_ms` in the header walk (preroll is
milliseconds; anything past a sane ceiling, e.g. u32::MAX, is corruption) and mint the
typed `bad-object-size`/`broadcast-unsupported` refusal there.
Confidence: HIGH — Verification: sed -n '506,510p' and '549,553p' (quotes verified);
rg -n "preroll" crates/demux/src/asf/header.rs → only the raw read at :713, no check.
Note: unreachable in shipped path today (F-001), so no user-visible blast radius yet.

### F-005 [S3] `seekability` promises Fast/SlowLinear that `seek` can refuse
Where: crates/demux/src/asf/mod.rs:770-778
Code:
```rust
        let indexed = self
            .header
            .as_ref()
            .is_some_and(|h| h.top_objects.iter().any(|o| o.guid == guid::SIMPLE_INDEX));
        if indexed {
            SeekSupport::Fast { indexed: true }
        } else {
            SeekSupport::SlowLinear
        }
```
What is wrong: `Fast { indexed: true }` is advertised on mere presence of a Simple
Index Object — but `seek` will fail with `demux.asf.index-too-large` (size > 1 MiB,
mod.rs:604) or `bad-object-size` (corrupt entries, mod.rs:206-214) without the
`seekability` answer ever reflecting that. Symmetrically, indexless files advertise
`SlowLinear` even when `packets_total > MAX_LINEAR_SEEK_PACKETS` (4096) — a state where
every seek deterministically fails `seek.too-slow` (mod.rs:737-739).
Why it matters: The host uses `SeekSupport` to decide whether to offer/plan seeking;
a promised-Fast file that hard-errors on seek is a UI-level contract break.
What correct looks like: `seekability` validates the index it vouches for (parse at
start, or answer `Fast` only when `load_simple_index` succeeded), and answers
`SlowLinear` only when the linear budget actually admits a scan.
Confidence: MEDIUM (no host consumer today — the demuxer is unwired, F-001; the
inconsistency is against the `SeekSupport` contract's meaning) — Verification:
sed -n '763,779p', '733,739p' (quotes verified); read load_simple_index 594-621.

### F-006 [S4] Stale struct doc still says `next_event` is not implemented
Where: crates/demux/src/asf/mod.rs:104-107
Code:
```rust
/// then drive it through the [`Demuxer`] trait: `start` runs the
/// GUID object walk and enumerates the audio/video track table (one
/// [`Track`] per Stream Properties object, codec descriptors per the
/// plan's §4.5 map); `next_event` refuses typed until ASF-b lands
/// the packet grammar (the module docs' honest not-yet).
```
What is wrong: ASF-b landed — the same file's module header (mod.rs:9-14) says
"ASF-b landed the Data Object packet walk" and `next_event` (mod.rs:651-697) implements
it. The struct doc contradicts both.
Why it matters: The primary API doc for the type describes a two-version-old state.
What correct looks like: Delete the "honest not-yet" sentence.
Confidence: HIGH — Verification: sed -n '100,108p' (verified) vs sed -n '9,14p' + code.

### F-007 [S4] Dead public API: `EXTENSIONS` const and `IndexEntry::packet_count` field
Where: crates/demux/src/asf/mod.rs:86 and 174-175
Code:
```rust
pub const EXTENSIONS: &[&str] = &["asf", "wmv", "wma"];
```
and
```rust
    #[allow(dead_code)]
    packet_count: u16,
```
What is wrong: `EXTENSIONS` has zero callers repo-wide (rg "asf::EXTENSIONS" → only
this definition); it is the constant a probe/registry integration would consume — the
integration that doesn't exist (F-001). `packet_count` is parsed, stored, and never
read (silenced with `#[allow(dead_code)]`).
Why it matters: `#[allow]` hiding a real lint (rule §3); dead surface area presenting
as an API.
What correct looks like: Wire EXTENSIONS into the probe hint layer (the fix for F-001
uses it) or delete it; same for the field.
Confidence: HIGH — Verification: rg "asf::EXTENSIONS|EXTENSIONS" within demux+cli → no
consumers; sed -n '83,86p','171,176p' verified.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `probe` | 96-98 | REAL-UNPROVEN (test-only) | S3 | 3 lines, window-anchored GUID check; callers: tests.rs:85-91, fuzz.rs:35 (result discarded) only. No shipped caller. |
| `SimpleIndex::parse` | 205-245 | REAL | — | Floor 56, exact-close, count cross-check, interval ≥1 enforced; alloc bounded by caller's 1 MiB check. No panic path found (all indexing guarded by len checks). |
| `SimpleIndex::bucket_of` | 250-256 | REAL | — | saturating_add, min-clamp to last bucket; `as usize` cast safe on 64-bit targets (workspace is 64-bit). |
| `AsfDemuxer::new` | 295-305 | REAL | — | Delegates rejection to SourceReader contract; no I/O. |
| `header` | 309-311 | REAL | — | Trivial accessor. |
| `file_properties` | 315-317 | REAL | — | Trivial accessor (copies FileProperties). |
| `streams` | 322-324 | REAL | — | Empty-slice default before start. |
| `stream_record_for_track` | 329-332 | REAL | — | `track.handle() as usize` — handle is the u32 minted at build_tracks:400; in-range by construction for own tracks. |
| `codec_list` | 336-338 | REAL | — | Trivial accessor. |
| `metadata` | 343-345 | REAL | — | Trivial accessor. |
| `data_object` | 350-352 | REAL | — | Trivial accessor. |
| `top_objects` | 356-358 | REAL | — | Trivial accessor; the seam F-005's answer should have used harder. |
| `damage_total` | 363-365 | REAL | — | Observability sum; zero callers outside tests (harvest seam, unwired). |
| `pcm_fact` | 371-379 | REAL | — | `starts_with("pcm_")` gate then (channels, rate); correct shape for the PcmDecoder seam; seam itself unwired (F-001). |
| `build_tracks` | 382-415 | REAL | — | rate=0 (audio record without WAVEFORMATEX) → TimeBase::new(1,0) Err → typed bad_waveformatex; video base 1/1000; span honestly UNKNOWN until stamps. |
| `build_walk` | 418-455 | REAL | — | min==max packet size enforced, packets_total = min(declared, extent-derived) — declared count never trusted past the object's own bytes. Correct bound discipline. |
| `walk_one_packet` | 459-525 | REAL | S3 | data_range slicing safe (validated in data.rs:269); undeclared-stream payloads skipped not fatal; F-004 preroll clamp lives here (:508). |
| `drain_held` | 532-565 | REAL | — | EOS flush of held fragments; same preroll clamp (F-004, :551). |
| `ticks_for` | 570-585 | REAL | S3 | F-003: dead `track` param, i64::MAX silent clamp. |
| `load_simple_index` | 594-621 | REAL | — | 1 MiB ceiling before read; None cached for indexless (no rescan); truncation typed. |
| `reanchor` | 626-633 | REAL | — | Clamps packet to packets_total, clears assembler+queue, re-arms eos. Seek commit is complete. |
| `Demuxer::start` | 637-649 | REAL | — | Idempotent via `started`; clones tracks on re-call (fine). |
| `Demuxer::next_event` | 651-697 | REAL | — | Queue-first, single-shot EOS drain, no infinite loop (each iteration either emits, advances next_packet, or terminates); no panic path. |
| `Demuxer::seek` | 699-761 | REAL | S3 | Indexed lane: bucket→entry→reanchor, landing = bucket start (honest). Linear lane: bounded 4096 packets, early break; F-005 inconsistency. Target conversion max(0)-clamped. |
| `Demuxer::seekability` | 763-779 | REAL | S3 | F-005: vouches for an index it never validated; SlowLinear even when budget always refuses. |

## 5. Test honesty
Tests live in `asf/tests.rs` (audited separately as my chunk #3). Baseline execution
verified: all `asf::tests::*` pass (`orchrun-test-workspace-nofailfast.log.txt`, lines
~1555-1620 of that log). Mutation reasoning for two representatives (full table in the
tests.rs audit): `walk_emits_whole_objects_with_preroll_subtracted_stamps` would fail if
preroll subtraction or reassembly broke (asserts concrete stamp values); `probe_anchors_on_the_header_guid_only`
would fail if the GUID compare or the ≥16 gate broke. No tautology observed in the names
that pass against this file's logic.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "plus the probe gate and the media-probe signature" | mod.rs:8 module docs | FALSE | No asf/wmv/wma signature in crates/media-probe/src (rg, zero matches); F-002. |
| "`next_event` refuses typed until ASF-b lands the packet grammar" | mod.rs:106-107 | FALSE | next_event implements the walk (mod.rs:651-697); module docs :9-14 say ASF-b landed; F-006. |
| ASF/WMV container "implemented" on windows/macos | docs/feature-matrix.toml:212 (`container-asf-wmv`) | FALSE (for shipped binary) | No registry arm (apps/cli/src/containers/mod.rs:36-68), no probe id; the row's own notes record probe.no-demuxer refusals; F-001. |
| "36-test asf unit suite … ffmpeg 7.1.5 sha256-pinned fixture differential" | docs/feature-matrix.toml:216 evidence | REAL (as test evidence) | 40 asf test lines ok in orchrun-test-workspace-nofailfast.log.txt; fixture differential is in tests.rs (chunk #3, audited there). |
| error-path list incl. "no site mints [`seek.unsupported`] anymore" | mod.rs:55-57 | UNVERIFIED | error.rs is another agent's file; I did not exhaustively re-verify the mint-site count. |

## 7. Dead / duplicated / unreachable code
- The entire module is dead in the shipped binary (F-001) though compiled (demux is a
  dependency of apps/cli).
- `EXTENSIONS` (F-007): zero callers repo-wide.
- `probe`, `damage_total`, `pcm_fact`, all facts accessors: test-only consumers.
- `ticks_for`'s `track` param (F-003).
- Preroll-subtraction block duplicated verbatim between `walk_one_packet` (506-509) and
  `drain_held` (549-552) — small, but a drift risk if one is fixed (F-004 hits both).

## 8. Dependencies & flags
- No feature gates: `pub mod asf;` unconditional (demux/src/lib.rs:45); compiled in
  every build including the shipped CLI. No unsafe, no external deps (house rule kept —
  verified by reading imports mod.rs:59-67: std + workspace crates only).
- Downstream seams referenced but unwired: media-probe signature, containers registry
  arm, PcmDecoder `pcm_fact` harvest (apps/cli/src/containers/* has no asf.rs).

## 9. Open questions for the next agent
- Who owns the "Next step: wire the missing probe signature / container-registry arm
  for `asf`" from matrix note (issue #53)? If a wiring unit exists on a branch, this
  verdict flips to REAL-UNPROVEN; nothing in this tree wires it.
- Does `SeekSupport::Fast` drive UI seek affordances in the engine (blast radius of
  F-005)? Trace `seekability()` consumers in playback-engine when the demuxer gets wired.
- error.rs: confirm no site mints `seek.unsupported` (mod.rs:55-57 claim) — I left it
  UNVERIFIED.
