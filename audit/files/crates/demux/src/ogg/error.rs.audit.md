# AUDIT: crates/demux/src/ogg/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-02-14 |
| Lines / functions | 203 / 14 (12 constructors + mint + 2 tests) |
| File verdict | REAL (2 of 12 constructors UNREACHABLE: `not_ogg`, `header_overrun`) |
| Reachable from shipped entry point? | YES for 10/12: lumen-cli `apps/cli/src/containers/ogg.rs:81` -> `OggDemuxer` -> mod.rs call sites (:161, :717, :906, :1178, :1190/1229/1246, :1319). `not_ogg` and `header_overrun` have ZERO production callers (rg over crates/ + apps/). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `./target/debug/deps/demux-5e0c7194d741938b --test-threads=1 'ogg::error'` -> 2 passed, exit 0; rg caller sweeps (`not_ogg`, `header_overrun`: only error.rs's own tests); sed quote verifications |

## 1. One-paragraph verdict
Uniform, table-tested typed-error mint: all twelve constructors share one
`mint` helper, stage `Demux` and dotted codes are pinned by a table-driven
test over every constructor plus a Display-capped sniff test, and both tests
pass live. Ten of twelve constructors are wired into the demuxer's real
paths. The two dead ones are not cosmetic: `not_ogg` — the constructor that
carries sniffed bytes precisely so a non-Ogg file can be diagnosed — is
never called; a garbage file surfaces instead as `no_streams()`
(mod.rs:1178) with no sniffed payload, so the diagnostic the API was
designed for never reaches users. `header_overrun` is equally vestigial: the
real header ceiling (mod.rs:1274 `stash_header`) damage-notes and skips
rather than erroring, so the constructor's documented semantics describe a
path that does not exist. Smaller defects: `bad_version` drops its
`_version` argument, and `recovery_budget_exhausted` labels a byte-scan
budget as `LimitKind::Memory`.

## 2. What actually works (verified)
— verified by live run `ogg::error` -> "test result: ok. 2 passed", exit 0;
caller wiring for 10 constructors confirmed by rg (mod.rs:161 bad_descriptor,
:717 recovery_budget_exhausted, :906 packet_overrun, :1178 no_streams,
:1190/1229/1246 seek_out_of_range, :1319 not_started; page.rs uses
bad_capture/bad_version/bad_structure/truncated_page). The header-stash
guard the `header_overrun` doc alludes to exists for real at
mod.rs:1270-1280 (bounded at MAX_HEADER_BYTES = 256 KiB, mod.rs:95) — no
unbounded-allocation hole, just a dead error constructor.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/ogg/error.rs:23-34
Code:
```
/// The bytes are not an Ogg container (bad or absent capture).
pub fn not_ogg(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("ogg".to_string()),
                sniffed: sniffed.to_vec(),
```
What is wrong: dead in production — zero callers outside error.rs's own
tests (rg over crates/ + apps/). A non-Ogg file never produces
"demux.ogg.not-ogg": the walk's read_page hits bad_capture, sync-recovers to
end-of-stream, and `open` reports `no_streams()` (mod.rs:1178) — an error
with NO sniffed bytes and a misleading "structure" framing for what is
really "this is not an Ogg file". The one constructor built to carry the
first-bytes evidence (sniffed payload, capped at 16 by Display) is unwired.
Compounding: the test `not_started_and_not_ogg_are_typed`
(ogg/tests.rs:480-490) never constructs not_ogg — its name promises the
not-ogg path, its body only checks not-started. Also, unlike the
media-probe twin, `sniffed.to_vec()` is unclamped at construction (the full
window is copied; only Display caps at 16). What correct looks like: open()
checks the first page's capture once and emits not_ogg with the sniffed
prefix; or delete the constructor and rename the test.
Confidence: HIGH — Verification: read + rg + live tests.

F-002 [S4]
Where: crates/demux/src/ogg/error.rs:100-111
Code:
```
/// A header packet accumulated past the per-stream ceiling —
/// refusing to trust the declaration into an allocation (MP §12).
pub fn header_overrun(budget: u64, attempted: u64) -> EngineError {
```
What is wrong: zero production callers. The actual enforcement
(mod.rs:1270-1280 `stash_header`) does NOT error — it damage-notes
("ogg: header budget exceeded") and skips the header, letting the stream
demux its data. So the constructor's doc ("refusing to trust the
declaration") describes semantics the shipped path never uses. No safety
hole (the stash is genuinely bounded at 256 KiB), but the typed-refusal
story is false for this path. What correct looks like: delete, or make
stash_header's skip path observable the way the doc claims.
Confidence: HIGH — Verification: read + rg.

F-003 [S4]
Where: crates/demux/src/ogg/error.rs:51-52 and 156-164
Code:
```
pub fn bad_version(offset: u64, _version: u8) -> EngineError {
```
(and `recovery_budget_exhausted` using `LimitKind::Memory` for a scan budget)
What is wrong: (a) `_version` is accepted and silently dropped — the
offending version byte never reaches the error payload or Display, losing
the one diagnostic byte the caller had; (b) `recovery_budget_exhausted`
classifies a byte-SCAN budget as `LimitKind::Memory` — the Display then says
"resource limit: memory (budget N, attempted M)" when nothing was allocated;
the natural kind is a scan/read budget. Also
`"demux.ogg.seek.out-of-range"` (:150) breaks the sibling dash convention
("demux.ogg.not-started" style) with a dot inside the last segment.
Confidence: HIGH — Verification: read + media-types kind_summary rendering
(checked in the media-probe error audit).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint | 19-21 | REAL | — | single seam, stage Demux |
| not_ogg | 24-34 | UNREACHABLE | S3 (F-001) | no callers; sniff copy unclamped |
| bad_capture | 38-47 | REAL | — | page.rs:102,144 |
| bad_version | 51-60 | REAL | S4 (F-003a) | `_version` ignored |
| bad_structure | 64-73 | REAL | — | page.rs:148 |
| bad_descriptor | 79-86 | REAL | — | mod.rs:161; doc honestly marks near-unreachable |
| truncated_page | 91-98 | REAL | — | page.rs:152,158 |
| header_overrun | 102-111 | UNREACHABLE | S4 (F-002) | real guard damage-notes instead |
| packet_overrun | 115-124 | REAL | — | mod.rs:906 |
| no_streams | 128-135 | REAL | — | mod.rs:1178 (also the de-facto not-ogg error, F-001) |
| not_started | 138-143 | REAL | — | mod.rs:1319 |
| seek_out_of_range | 147-152 | REAL | S4 (F-003c) | dot-in-segment code |
| recovery_budget_exhausted | 156-165 | REAL | S4 (F-003b) | scan budget labeled Memory |
| tests (2) | 171-201 | REAL | — | table over all 12; Display-capped sniff |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_ogg_codes | YES | every constructor's stage + code + Display prefix/suffix pinned |
| not_ogg_sniff_capped_by_display | YES | 64×0xCD → ≤16 "cd" hex pairs in Display |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "refusing to trust the declaration into an allocation (MP §12)" (header_overrun) | error.rs:100-101 | FALSE for this constructor | real guard at mod.rs:1274 damage-notes; constructor never called |
| "Constructors take byte counts and offsets only — never paths, URIs or payload content" | error.rs:9-12 | REAL | verified across all 12 signatures; not_ogg's sniffed is content but explicitly an L0 content hint (the doc's own parenthetical) |

## 7. Dead / duplicated / unreachable code
- `not_ogg` (F-001), `header_overrun` (F-002) — production-dead.
- bad_capture/bad_version/bad_structure mint byte-identical ErrorKinds
  (Corrupt::InvalidStructure{Some(offset)}) distinguished only by code —
  acceptable design (codes are the discriminator), noted.

## 8. Dependencies & flags
media-types taxonomy only. No unsafe, no features.

## 9. Open questions for the next agent
- ogg/mod.rs (not assigned to me): confirm whether `open()` should emit
  not_ogg when byte 0 lacks the capture — today a renamed .ogg JPEG probes
  as "ogg" (extension route) then fails demux with no-streams and no sniffed
  bytes; the CLI surfaces a worse message than the media-probe stage would.
- The `#[must_use]` attributes in mod.rs rendered as `#ust_use]` in my sed
  view — display artifact of this session's terminal, not the file; verify
  with od if it matters.
