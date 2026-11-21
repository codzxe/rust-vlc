# AUDIT: crates/media-probe/src/signature/mod.rs (file: 517 lines)
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-21 |
| Lines / functions | 517 / 24 units (trait `ProbePlugin` with 2 methods, 6 consts, 2 structs, `ProbeRegistry` 6 methods, `better_tiebreak`, 4 test helpers, 9 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `ProbeRegistry::with_builtins()` constructed at `apps/cli/src/demux_host.rs:444` (the `lumen` open path: probe → `containers::open` dispatch) and `apps/web/src/open.rs:230`; executed in baseline: all 9 `signature::tests` green (orchrun-test-workspace-nofailfast.log.txt:3373-3391) + 3 `registry_roundtrip` tests green (:3425-3428) + the AVI/MP4 smoke lanes ran through this registry (INDEX.md MP4 matrix). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `Read` full file; `rg -in "asf|wmv|wma" crates/media-probe/src/` → **zero matches**; `rg -n "with_builtins" apps/` (3 call sites); `sed -n 425,505p apps/cli/src/demux_host.rs` (open path: probe result IS the container id, no extension fallback); `rg -n "asf" apps/cli/src/containers/mod.rs` → none (mod list: adts aiff avi es ivf mkv mp4 ogg ps ts wav — no asf); `rg -n "AsfDemuxer" apps/ crates/` → definition/docs/tests only, zero external callers; `rg -c "test asf::" baseline` → 36 green; `rg -n "asf" docs/feature-matrix.toml` (row 207-217: own triage admits `probe.no-demuxer` on .wmv, status demoted implemented→partial, "Next step: wire the missing probe signature"); `sed -n 240,300p crates/media-probe/tests/registry_roundtrip.rs` (14-row extension table, no asf); `sed -n 3422,3428p baseline` (roundtrip green); `rg -n "fn extensions"` (implemented by avi/es/mp4/phase5, consumed by roundtrip test :260); `sed -n` on every quoted range. |

## 1. One-paragraph verdict
The registry core is real and tight: scoring is a pure function over the
window, out-of-range scores are discarded (not clamped, pinned by test),
the three-level tie-break is implemented exactly as documented and pinned
by three dedicated tests, the sub-threshold error carries the best-known
family name with sniffed bytes, and a deterministic never-panic sweep runs
all 14 builtins over byte-mutated and truncated windows — all green in the
baseline. The file's one serious problem is what it does NOT register: an
ASF signature does not exist anywhere in media-probe, so the shipped open
path (which dispatches purely on the probe result) typed-refuses every
.wmv/.wma/.asf file while a complete, 36-test-green ASF demuxer sits
unreachable in the demux crate — and this file's own doc comment claims
the builtin set covers "every container whose demuxer card landed in waves
4–7", which is false. Two hygiene findings: the never-panic test's comment
promises "two template windows" but builds one, and the report's
`bytes_examined` always equals the full window regardless of the winner.

## 2. What actually works (verified)
— All 9 tests green in baseline: `higher_score_wins_over_priority`,
`priority_breaks_score_ties`, `shorter_prefix_breaks_priority_ties`,
`subthreshold_best_hit_names_the_family_in_the_error`,
`out_of_range_scores_are_discarded_not_clamped`,
`empty_registry_is_a_clean_unknown`, `builtin_registry_shape` (len==14
matches the 14 register calls I counted),
`builtin_signatures_never_panic_on_arbitrary_windows`,
`track_hints_flow_from_hit_to_report` (log :3373-3391).
— Round-trip suite green: `extension_lists_cover_their_fixtures` (14 rows),
`near_misses_and_garbage_stay_unprobed`,
`every_landed_container_probes_to_its_format_id` (:3425-3428).
— Engine-level execution through this registry: MP4 smoke matrix
(INDEX.md "Corrected MP4 smoke matrix" — mp4/aac/hevc files probed and
dispatched) and AVI first-vertical lifecycle (INDEX.md:71-78).
— Tie-break correctness re-derived: `better_tiebreak` (strictly-greater at
each level) + first-registered-wins on full ties = deterministic total
order; the three tests mutate exactly one level each.

## 3. Findings

### F-001 [S2]
Where: crates/media-probe/src/signature/mod.rs:120-147 (doc claim at :126-128, register list at :130-146)
Code:
```rust
    /// consolidation rows (card 5.R3-REG): WAV, AIFF, Ogg,
    /// Matroska/WebM, ADTS/ADIF, MPEG-TS, MPEG-PS and IVF, every
    /// container whose demuxer card landed in waves 4–7, plus the
    /// raw elementary-stream family (card 6.OPW-es): MPEG audio in
```
What is wrong: The builtin set claims to include "every container whose
demuxer card landed in waves 4–7" — but ASF is missing. Evidence chain:
(1) an ASF demuxer exists and landed (`crates/demux/src/asf/**` — mod/header/
data/guid/tests/fuzz/fixtures; matrix evidence string names cards
"6.ASF-a/b/c"); (2) 36 `asf::*` tests green in baseline (:1555+);
(3) `rg -in "asf|wmv|wma" crates/media-probe/src/` → zero matches — no
`AsfSignature` exists; (4) the shipped open path is
probe-only: `apps/cli/src/demux_host.rs:444-451` uses
`report.format.as_str()` as the container id for `containers::open`, and
`apps/cli/src/containers/mod.rs` wires adts/aiff/avi/es/ivf/mkv/mp4/ogg/ps/
ts/wav — no asf arm, "anything not wired falls to `unwired_refusal`";
(5) `rg AsfDemuxer apps/ crates/` → zero callers outside its own module.
Net: **every .wmv/.wma/.asf file is refused with `probe.no-demuxer` in
both shipped frontends, and the entire ASF demuxer is dead code in the
shipped binary.** The repo knows: docs/feature-matrix.toml:217 records the
compat-matrix triage — "the open path REFUSES the corpus files of probe
format `asf` … (probe.no-demuxer) … Next step: wire the missing probe
signature / container-registry arm for `asf`" — status demoted to partial,
but the fix never landed and this file's doc comment still asserts
completeness.
Why it matters: A fully implemented, unit-tested container family is
unreachable by users; the code comment tells the next maintainer the
registry is complete when it is not (the exact class of drift the audit
exists to catch). WMA audio decode is separately blocked codec-side (row
`codec-audio-wma`), but the container lane (demux → facts) could run and
currently cannot even be entered.
What correct looks like: an `AsfSignature` anchoring on the ASF header
GUID (30 26 b2 75 …), registered in `with_builtins`, an `asf` arm in
`containers/mod.rs`, and the doc claim made true (or the claim scoped to
the actually-registered list).
Confidence: HIGH — Verification: all five evidence points above are rg/sed
reads plus baseline citations; nothing inferred.
Note: severity S2 not S0 because the feature-matrix row itself honestly
documents the refusal and partial status (no shipped doc claims ASF
playback works); the false completeness claim lives in this file's doc
comment.

### F-002 [S4]
Where: crates/media-probe/src/signature/mod.rs:472-478
Code:
```rust
        // Doc 03 §4.2(f): random prefixes must never panic any probe
        // plugin. Deterministic sweep: every byte value at every
        // position of two template windows, plus every truncation.
        let r = ProbeRegistry::with_builtins();
        let meta = SourceMeta::new(None, None);
        let cfg = config(60);
        let mut template = vec![0u8; 64];
```
What is wrong: The comment promises "every byte value at every position of
**two template windows**" — the body builds ONE 64-byte template (RIFF…AVI
…moov markers) and sweeps 64×256 single-byte mutations plus 65 truncations
of it. No second template exists.
Why it matters: The stated coverage is a claim about what is tested; a
reader trusting it believes MKV/OGG/ADTS/TS anchor regions got the
single-byte sweep too (they did not — e.g. an `EBML` magic or ADTS
sync-word template is never mutated).
What correct looks like: either add the second (non-RIFF) template, or fix
the comment to say one template.
Confidence: HIGH — Verification: read of the full test body :471-492.
(The sweep that DOES run is green in baseline :3391.)

### F-003 [S4]
Where: crates/media-probe/src/signature/mod.rs:201-206
Code:
```rust
            Some((hit, _, confidence)) if confidence.meets(config.min_score) => Ok(ProbeReport {
                format: hit.format,
                confidence,
                track_hints: hit.track_hints,
                bytes_examined: prefix.len(),
            }),
```
What is wrong: `bytes_examined` is hardcoded to the full window length for
every winner, regardless of how deep the winning signature actually
walked (`hit.prefix_consumed` is available and used only for tie-breaking).
A 4-byte-anchor winner on a 4096-byte window reports 4096 bytes examined.
Why it matters: The report's field name promises per-probe depth
accounting; the CLI/media-info surface can display it. It is not a lie
about the registry (all plugins did scan the window) but it erases the
per-format depth signal the `ProbeHit.prefix_consumed` field exists to
carry.
What correct looks like: `bytes_examined: hit.prefix_consumed` (or a
window/`prefix_consumed` pair if both facts are wanted).
Confidence: MEDIUM — Verification: read of probe + ProbeHit construction
in the Fixed test plugin (:252-257, which sets prefix_consumed but the
report never surfaces it; the `track_hints_flow_from_hit_to_report` test
asserts `bytes_examined == 1` only because the window itself is 1 byte —
the aliasing is invisible there).
Note: the semantics may be deliberate ("the engine examined this many
bytes"); if so the field name should say window_bytes.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-20 | REAL | — | selection rules + threshold described accurately vs code |
| `ProbePlugin::extensions` | 55-57 | REAL | — | default `&[]`; implemented by all builtin submodules; consumed by roundtrip test :260 — the "mirror-pinned" doc claim is REAL (test green :3425) |
| `ProbePlugin::probe` | 65-66 | REAL | — | pure `(prefix, meta)` contract; submodules are other agents' files (avi=A01, es/mp4=A02/A05, phase5=A04) |
| `MAX_WALK_STEPS` | 72 | REAL | — | 256-step iteration guard, PROPOSAL-annotated; consumed by submodule walks |
| score consts (6) | 91-96 | REAL | — | ladder 50..95; 100 reserved; sub-60 FAMILY_ONLY feeds the error path by design |
| `Entry` | 99-102 | REAL | — | plugin + priority |
| `ProbeRegistry::empty`/`Default` | 107-118 | REAL | — | duplicate construction paths (derive + fn) — trivial |
| `ProbeRegistry::with_builtins` | 130-147 | REAL | S2 | F-001: 14 builtins, no ASF; doc claims wave-4–7 completeness (false) |
| `ProbeRegistry::register` | 150-152 | REAL | — | no dup-format guard — two plugins for the same format would both run, tie-break decides; acceptable for a registry seam |
| `ProbeRegistry::len`/`is_empty` | 155-162 | REAL | — | accessors; `is_empty`/`len` without `Clippy::len_without_is_empty` issue — fine |
| `ProbeRegistry::probe` | 175-213 | REAL | S4 | pure scoring pass; out-of-range discard via `Confidence::from_score` (None); F-003 (bytes_examined); no early-exit on high score (14 plugins always run — fine at this scale) |
| `better_tiebreak` | 217-222 | REAL | — | strict-total-order; deterministic; three tests pin each level |
| test helpers `ok`/`Fixed`/`registry`/`config` | 231-271 | REAL | — | Fixed plugin is a legit test double of the SEAM (registry logic under test, not the plugin) — not a mock of the subject |
| 9 tests | 273-516 | REAL | S4 | all green (baseline :3373-3391); F-002 comment lie in the never-panic sweep; each tie-break test mutates exactly one level — honest |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `higher_score_wins_over_priority` :274 | YES | score 70/prio 999 vs 90/prio 1 → "bbb"; a priority-first compare fails |
| `priority_breaks_score_ties` :300 | YES | equal scores, 10 vs 20 → "bbb" |
| `shorter_prefix_breaks_priority_ties` :330 | YES | equal score+priority, 50 vs 10 consumed → "bbb" |
| `subthreshold_best_hit_names_the_family_in_the_error` :360 | YES | asserts code `probe.no-demuxer`, stage, kind, name, sniffed bytes, AND a non-empty `source()` chain — a silent-refusal mutation fails it |
| `out_of_range_scores_are_discarded_not_clamped` :398 | YES | 101-score liar must neither win nor be named — a clamp-to-100 mutation fails BOTH asserts |
| `empty_registry_is_a_clean_unknown` :450 | YES | typed refusal, code asserted |
| `builtin_registry_shape` :461 | YES (shape) | pins count 14 — adding/removing a builtin fails it; does NOT probe any real bytes |
| `builtin_signatures_never_panic…` :471 | YES (survival) | a panicking builtin fails the test; byte-coverage limited (F-002) |
| `track_hints_flow_from_hit_to_report` :495 | YES | hint vector + codec + bytes_examined asserted end-to-end |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Scoring is a pure function of (prefix, meta)" | mod.rs:7-8 | REAL | probe takes `&self`, no state mutated; plugins handed only window+meta |
| "A hit whose score exceeds 100 … is discarded, never clamped" | mod.rs:11-14 | REAL | `Confidence::from_score` gate :186-188 + test :398 green |
| "ties break by plugin priority … then by the shorter prefix consumed" | mod.rs:9-11 | REAL | better_tiebreak :217-222 + 3 tests green |
| "A best hit scoring below min_score is a typed Unsupported carrying … the best hit's format name, the sniffed bytes, the source hints" | mod.rs:16-20 | REAL | test :360 green (name/sniffed/chain all asserted) |
| "every container whose demuxer card landed in waves 4–7" is in with_builtins | mod.rs:126-128 | **FALSE** | F-001: ASF demuxer landed (36 tests green) but no signature/wiring exists; repo's own matrix note admits the refusal |
| "extensions … Mirror-pinned to each demuxer's own EXTENSIONS table by the round-trip suite" | mod.rs:51-54 | REAL | registry_roundtrip :242-267 green (:3425) |

## 7. Dead / duplicated / unreachable code
- The entire `crates/demux/src/asf/**` tree is unreachable from shipped
  entry points (F-001's consequence — the finding belongs to this file's
  registry because this file is the gate that excludes it).
- No dead code within this file itself; `SCORE_COMPAT_BRAND`/`SCORE_FAMILY_ONLY`
  are consumed by the mp4/es submodules (other agents' files — flagged for
  their verification).

## 8. Dependencies & flags
- `plugin_api::manifest::DEFAULT_PRIORITY_BUILTIN` (the manifest priority
  figure); `crate::{config::ProbeConfig, error, report}` — workspace
  internal. No feature gates; no unsafe; no panics in production paths
  (unreachable! only in tests).
- Submodule surface: avi/es/mp4/phase5 are private mods re-exported —
  their audits are A01/A02/A05/A04's.

## 9. Open questions for the next agent
- A09 (contract.rs): does `SourceMeta::from_stream` surface the extension
  hint used in the no-demuxer error? (The matrix note shows
  "extension=wmv" riding the error — confirm where that string is built.)
- Whoever owns docs/feature-matrix.toml row 207: the row's evidence string
  still cites the asf suite as if exercisable; the notes field honestly
  records the open-path refusal — reconcile evidence vs notes (row is
  W18/W19 matrix agents' scope).
- A04 (phase5.rs): confirm every phase5 signature's extensions() matches
  the roundtrip table (the test only checks fixture extensions are
  CONTAINED, not equality — extra extensions would pass unnoticed).
