# AUDIT: crates/demux/src/mkv/chapters.rs#part1 (lines 1-437)
| Field | Value |
|---|---|
| Auditor | W05-A05 |
| Date | 2026-11-21 |
| Lines / functions | 437 (chunk 1-437 of 876; part2=438-876 is W05-A04's and holds `parse_track_numbers`/`parse_display`/`read_uint`/`read_flag`/`chapter_table_too_large`/`flag_beyond_duration` fns + `mod tests`) / 31 items |
| File verdict | REAL for the parse (executed on every MKV open, unit + generated-fixture ffprobe differential baseline-ok); UNREACHABLE for the exposure surface (ChapterTarget has no shipped consumer — chapters never reach the user from ANY container) |
| Reachable from shipped entry point? | YES for the parse: `lumen open x.mkv|.webm` → `apps/cli/src/demux_host.rs:458 containers::open` → `apps/cli/src/containers/mod.rs:57 "matroska" => mkv::open` → `MkvDemuxer::start` walk (mod.rs:377-425 `chapters::ID_CHAPTERS => self.collect_chapters(&child)`) → `parse_chapters`. NO for `ChapterTarget`/`chapter_targets()`: callers are only `crates/demux/tests/mkv_extras_wiring.rs:413,590,642,939` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | full Read of chapters.rs 1-876 (context; audited 1-437); `rg -n "parse_chapters\|ChapterTarget\|default_edition\|atoms_flattened\|flag_beyond_duration" crates/ apps/` → callers mapped (mod.rs walk + tests only); Read mod.rs 270-320 (chapter_targets ns→ticks), 377-425 (walk), 455-495 (collect/flag wiring); `rg -n "chapter" apps/cli/src/host.rs apps/cli/src/dispatch.rs apps/cli/src/containers/mkv.rs` → host.rs:1311 `chapters: Vec::<Chapter>::new()`, dispatch.rs:206 refusal, mkv.rs:112-115 documented gap; baseline greps: 11/11 `mkv::chapters::tests::` ok (nofailfast:1723-1733), mkv_attachments_chapters 2 ok (:2089-2094), mkv_extras_wiring 9 ok incl. 4 chapter_targets tests (:2125-2133); `ls /tmp/my-project/corpus-v4/media` → absent; `which ffmpeg ffprobe` → /usr/bin (generated-fixture lane feasible); spec cross-check of all 19 element IDs vs the Matroska elements table; `sed -n` on every quoted range |

## 1. One-paragraph verdict
The parse is real, bounded, and executed: all 19 Matroska element IDs match the spec, every count (editions 64, atoms 65,536, displays 16, tracks 64) and the 16-deep nesting guard are enforced at the parse site with typed refusals or damage rows counted in bytes, the ns-not-scale-ticks chapter-time rule is the spec's own quirk correctly implemented, and the unit suite (11 tests) plus a live-ffprobe differential on an ffmpeg-generated fixture both passed in the baseline. What the module does not tell you in its own doc: none of this reaches the user. The shipped host always reports `chapters: []` (host.rs:1311), chapter navigation is a typed refusal in the CLI dispatch, the container module records the gap ("MediaFacts carries no chapter vocabulary … v0 out of scope"), and the module's advertised UI surface (`ChapterTarget`, issue #34) has zero production callers — so every MKV open pays the parse and retains the list for nothing user-visible, and a second, engine-side chapter model (playback-engine/src/chapters.rs, W03-A08's) sits equally unfed. Additionally the doc's "Verified against ffprobe on the corpus (0007 …)" cites corpus evidence that loud-skips in every committed run — the executed verification is the generated-fixture lane.

## 2. What actually works (verified)
- Element IDs (55-92): all 19 cross-checked against the Matroska spec elements table — Chapters 0x1043A770, EditionEntry 0x45B9, EditionUID 0x45BC, EditionFlagHidden 0x45BD, EditionFlagDefault 0x45DB, EditionFlagOrdered 0x45DD, ChapterAtom 0xB6, ChapterUID 0x73C4, ChapterTimeStart 0x91, ChapterTimeEnd 0x92, ChapterFlagHidden 0x98, ChapterFlagEnabled 0x4598, ChapterTrack 0x8F, ChapterTrackNumber 0x89, ChapterDisplay 0x80, ChapString 0x85, ChapLanguage 0x437C, ChapLanguageIETF 0x437D, ChapCountry 0x437E — all exact.
- Ceilings enforced at the parse site: editions (292-293, over-ceiling skipped with a byte-counted damage row), atoms (344-349 and 430-435: shared budget across editions and nesting, 65,537th is the typed `demux.mkv.chapter-table-too-large` ResourceLimit), displays (414-417), depth (427-428: `depth + 1 > ebml::MAX_PARSE_DEPTH` (=16, ebml.rs:46) → `bad_structure`, checked BEFORE recursing — the nesting bomb cannot overflow the stack). All pinned by baseline-ok tests: `atom_table_ceiling_is_the_typed_resource_limit`, `edition_ceiling_skips_with_damage`, `nested_atoms_parse_to_the_depth_guard` (16 ok / 17 refused), `unknown_size_inside_chapters_is_bad_structure`, `truncated_atom_field_is_the_typed_truncation` (nofailfast:1723-1733).
- The ns rule (doc 8-16, `ChapterAtom.start_ns` ns): implemented as raw u64 ns with no scale math anywhere in the parse; the ns→ticks conversion happens only at the (test-only) `chapter_targets` seam via integer division (mod.rs:297 `atom.start_ns / scale`). Verified live by the executed generated-fixture differential (`generated_fixture_attachments_chapters_differential ... ok`, nofailfast:2092, 0.18 s with live ffprobe per mkv_attachments_chapters.rs:1-22) which compares per-chapter ns start/end + titles against `ffprobe -show_chapters`.
- Containment/lie checks: every level (parse_chapters 283-289, parse_edition 321-325, parse_atom 385-389) refuses unknown-size inner elements typed and rejects children overrunning their parent (`child_end > end` → `error::truncated`); the loop advance `pos = child_end` is strictly increasing (child_end = data_pos + size ≥ data_pos > pos), so no livelock.
- Damage convention: rows are `(bytes, reason)` — `child_end - pos` per skipped unit — and `collect_chapters` folds them into the demuxer's byte-denominated `damage_total` (mod.rs:491). Consistent with the TS lane (contrast the MP4 entry-count notes, my tests.rs#part2 F-003).
- Degenerate atom (no mandatory ChapterTimeStart): `Ok(None)` → damage row, siblings survive (449-456, part2 boundary; pinned by `atom_without_time_start_is_damage_skipped`).

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/mkv/chapters.rs:37-42 (doc) + 246-256 (ChapterTarget); shipped-layer evidence: apps/cli/src/host.rs:1311, apps/cli/src/dispatch.rs:206, apps/cli/src/containers/mkv.rs:112-115, crates/demux/src/mkv/mod.rs:276+590-642-939-caller-map
Code:
```rust
//! # Exposure seam (the card's "chapter list for the UI layer")
//!
//! [`MkvDemuxer::chapters`] is the typed getter over the list the
//! demuxer **retains at `start`** (issue #34 wired the `open()` seam:
//! the on-demand positional re-scan of the 5.D2a card is gone — the
//! list is parsed once during the walk and held on the struct).
```
```rust
                chapters: Vec::<Chapter>::new(),
```
What is wrong: the module doc presents the "chapter list for the UI layer" as the card's purpose and names `MkvDemuxer::chapters` the exposure seam — but no shipped code beyond the getter reads any of it. The CLI host builds `MediaInfo` with a hard-coded empty chapter vector (host.rs:1311); chapter navigation is the typed refusal "chapters and their navigation are the chapter card" (dispatch.rs:206); the container module records the gap ("Chapters/Editions/ordered-chapters: demuxer-side metadata only — `MediaFacts` carries no chapter vocabulary … ordered-chapter navigation is v0 out of scope", mkv.rs:112-115). The only consumers of `ChapterTarget`/`chapter_targets()` are four calls in `crates/demux/tests/mkv_extras_wiring.rs` (413, 590, 642, 939); the command layer the mod.rs doc names is admitted not to exist ("the command layer does not exist yet, so nothing downstream reads it today", mod.rs:273-274). Net: every MKV open pays the chapters parse and retains the list on the struct, and the user can never see a chapter from any container — the MP4 host shows the same empty-vector gap (mp4-smoke-open-session-420.txt: `chapters: []` on a file whose unit tests prove chapter parsing works).
Why it matters: a reader of chapters.rs believes the UI chapter list is wired; it is parsed-and-dropped at the last shipped hop. A second, parallel chapter model exists in the engine (`crates/playback-engine/src/chapters.rs`, its own baseline-ok tests at nofailfast:3749-3752) and is equally unfed — two chapter implementations, zero user-visible chapters. Effort duplication with a documented-but-dead payoff.
What correct looks like: either the host maps `ChapterList`→`Chapter` into `MediaInfo` (one call site at host.rs:1311) or the module doc's first paragraph states plainly that no shipped surface consumes the list yet and names the owning card for the wiring.
Confidence: HIGH — Verification: read + repo-wide caller greps; shipped-layer sites quoted; MP4 smoke evidence cited.

### F-002 [S3]
Where: crates/demux/src/mkv/chapters.rs:12-14 (doc claim); evidence gates: crates/demux/tests/mkv_attachments_chapters.rs:36-43 + 246-249, apps/cli/src/containers/mkv.rs:636-637; executed line: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2089-2094
Code:
```rust
//!   scale ticks, chapter timestamps are ns). Verified against
//!   ffprobe on the corpus (0007: raw `0x0F4240` = 1e6 ns with
//!   TimestampScale 1e6) and on the ffmpeg-generated fixture. The
```
```rust
fn corpus_attachments_chapters_differential_vs_ffprobe() {
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP: corpus-v4 absent (LUMEN_CORPUS unset, /tmp/my-project/corpus-v4 missing)");
        return;
    };
```
What is wrong: the doc's primary verification citation is the corpus lane 0007 — and every corpus-gated differential in this area takes the loud-skip branch in the committed runs: `/tmp/my-project/corpus-v4/media` does not exist (verified in this sandbox; the same absence W04-A10 established for the Ogg lane), and the skip is an `eprintln!` + `return` that still reports `ok` (nofailfast:2091 `corpus_attachments_chapters_differential_vs_ffprobe ... ok`; the sibling `mkv_corpus` binary finished in 0.00 s — no corpus walk happened). The 0007 lane in the CLI truth table (apps/cli/src/containers/mkv.rs:815-824) asserts stream counts only and is itself corpus-gated (:636-637).
Why it matters: the ns-unit claim is TRUE (the spec is unambiguous and the executed generated-fixture differential verified it live against ffprobe), but the specific corpus citation "0007: raw 0x0F4240 …" is unverifiable from any committed run — a future reader chasing the evidence finds a skip. This is the same skip-reports-ok defect class as the Ogg (W04-A10 F-002) and MP4 (my tests.rs#part2 F-001) corpus gates.
What correct looks like: cite the generated-fixture differential (executed, deterministic, ffmpeg-built at test time) as the primary evidence, keep the corpus citation as secondary, and make skips `ignored`-visible so `ok` cannot masquerade as a pass.
Confidence: HIGH — Verification: doc + gates read; dir absence verified; baseline lines cited; generated lane's 0.18 s runtime + ffmpeg presence support its real execution.

### F-003 [S4]
Where: crates/demux/src/mkv/chapters.rs:398-402 (no ordering validation), consumer: mod.rs:296-305 (span build)
Code:
```rust
            ID_CHAPTER_TIME_START => {
                start = read_uint(reader, &child, len, child_end)?;
            }
            ID_CHAPTER_TIME_END => {
                parsed.end_ns = read_uint(reader, &child, len, child_end)?;
            }
```
What is wrong: `ChapterTimeEnd < ChapterTimeStart` (an inverted span) parses without complaint; `flag_atom` (part2, 582-595) flags it normally, and the (test-only) `chapter_targets` conversion would mint a `TimeSpan` whose `end` precedes its `start` — a negative-width chapter. Nothing in the damage ledger records the inversion (the doc's damage list covers over-ceiling, degenerate and nesting only).
Why it matters: a malformed file yields nonsense seek targets instead of a damage note — the module's own honesty policy ("kept and flagged, never dropped") is not applied to the one geometrically impossible shape. Blast radius today is zero because no shipped consumer reads the spans (F-001).
What correct looks like: on `end_ns < start`, either clear `end_ns` to `None` with a damage row (`"atom-end-before-start"`) or set a `past_duration`-style tolerant flag.
Confidence: HIGH on the absence of a check (all reads traced); MEDIUM on impact (unconsumed surface).
Verification: read.

### F-004 [S4]
Where: crates/demux/src/mkv/chapters.rs:329-342 and 393-409
Code:
```rust
            ID_EDITION_UID => {
                if let Some(value) = read_uint(reader, &child, len, child_end)? {
                    parsed.uid = value;
                }
            }
```
What is wrong: repeated scalar fields silently overwrite — a second `EditionUID` (or `ChapterUID`, flags, `ChapterTimeStart`/`End`) replaces the first with no damage note. The spec declares these single-occurrence; a damaged or adversarial file carrying two `ChapterTimeStart` elements yields the LAST one as the atom's start with zero observability.
Why it matters: the module's damage ledger records every other tolerance decision (over-ceiling, degenerate atoms, unknown elements are skipped silently but that is documented at 300 and 442-445); duplicate-field overwrite is the one silent mutation of parsed state, and it is undocumented. Cosmetic today (mkvtoolnix never emits dupes; fuzz corpus would).
What correct looks like: first-wins (`if parsed.uid == 0`) or a damage row on the second occurrence, plus one doc line naming the policy.
Confidence: HIGH — Verification: read; all scalar arms traced.

## 4. Per-function table (part1 items; `parse_track_numbers`/`parse_display`/`read_uint`/`read_flag`/`chapter_table_too_large`/`flag_beyond_duration`/`flag_atom` + `mod tests` live in part2 = W05-A04)
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-42 | REAL (claims) | S3/S3 | ns rule + ceilings + guards all true; exposure-seam framing overstates (F-001); corpus citation unexecuted (F-002) |
| 19 element-ID consts | 55-92 | REAL | — | all match the Matroska spec table (cross-checked) |
| `MAX_EDITIONS`/`MAX_CHAPTER_ATOMS`/`MAX_CHAPTER_DISPLAYS`/`MAX_CHAPTER_TRACKS` | 97/104/108/112 | REAL | — | 64/65,536/16/64; all enforced at parse sites; PROPOSAL annotations carry measured rationale |
| `ChapterDisplay` (struct) | 117-127 | REAL | — | all-Option tolerant text fields |
| `ChapterAtom` (struct) | 131-154 | REAL | S4 | `past_duration` tolerant note; no end≥start invariant (F-003) |
| `ChapterAtom::title` | 159-162 | REAL | — | first titled display; test-pinned |
| `ChapterAtom::flatten_into` | 164-170 | REAL | — | pre-order DFS; recursion bounded by parse-time depth ≤16 for file-derived trees |
| `EditionEntry` (struct) | 174-189 | REAL | — | plain data |
| `ChapterList` (struct) | 193-202 | REAL | — | damage rows (bytes, reason) — consistent byte convention |
| `ChapterList::atoms_flattened` | 208-217 | REAL | — | ffprobe's chapter-list order (differential's shape) |
| `ChapterList::default_edition` | 224-231 | REAL | — | default-flag → first non-hidden → first; pinned by test |
| `ChapterList::flag_beyond_duration` | 237-240 | REAL | — | delegates to part2 fn; called by the walk (mod.rs:461) — reachable |
| `ChapterTarget` (struct) | 246-256 | UNREACHABLE | S3 | test-only consumers (F-001); doc "scale ticks" matches the mod.rs conversion |
| `ChapterDamage` (type) | 259 | REAL | — | alias |
| `parse_chapters` | 270-304 | REAL | — | unknown-size clamp to bound; inner unknown-size/overrun typed; edition ceiling + skip-damage; loop strictly advances |
| `parse_edition` | 308-363 | REAL | S4 | shared atom budget; degenerate atoms damage-noted; duplicate scalars last-wins (F-004) |
| `parse_atom` (part1 portion 367-437) | 367-437 | REAL | S4 | enabled=true default ✓ spec; depth guard BEFORE recursion; display/atom ceilings; mandatory-TimeStart check completes in part2 (449-456) |

## 5. Test honesty (part1 has no in-chunk tests — `mod tests` at 597-875 is part2/W05-A04; cited as evidence)
| Test (part2) | Would it fail if the code were broken? | Reason |
|---|---|---|
| `nested_atoms_parse_to_the_depth_guard` (733-757) | YES | builds exactly MAX_PARSE_DEPTH and +1 chains; 16 parse / 17 refuse pinned by count and code |
| `atom_table_ceiling_is_the_typed_resource_limit` (760-780) | YES | 65,537 atoms → exact code + ResourceLimit kind |
| `editions_atoms_and_displays_parse_the_full_record` (657-702) | YES | every field asserted (uid/flags/times/track_numbers/all 4 display strings/title/flatten) |
| `times_past_the_duration_are_the_tolerant_note` (826-839) | YES | flagged count + per-atom flag + unflagged sibling |
| `extreme_times_land_in_the_tolerant_bucket` (842-853) | YES | u64::MAX start: kept, flagged, no overflow (raw ns, no scale math) |
| `generated_fixture_attachments_chapters_differential` (integration, mkv_attachments_chapters.rs:381+) | YES (when ffmpeg present) | live ffprobe `-show_chapters` compared per chapter (ns start/end + title); executed in baseline (0.18 s) |
| `corpus_attachments_chapters_differential_vs_ffprobe` (245+) | NO — skip-reports-ok | corpus absent → eprintln+return → `ok` (F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Chapter times are nanoseconds … do not scale with TimestampScale" | chapters.rs:8-16 | REAL | spec rule; parse stores raw ns; live-verified by the executed generated-fixture ffprobe differential (nofailfast:2092) |
| "Verified against ffprobe on the corpus (0007: raw 0x0F4240 …)" | chapters.rs:12-14 | REAL-UNPROVEN (citation) | corpus lanes loud-skip in all committed runs (dir absent); the executed verification is the generated fixture — F-002 |
| "Bounded memory (MP §12) … ceilings … typed refusal, never an unbounded allocation" | chapters.rs:17-24 | REAL | ceiling sites 292/344/414/479(part2)/430; tests nofailfast:1723-1733 |
| "Recursive nesting is guarded … never a stack overflow" | chapters.rs:25-29 | REAL | depth+1 > 16 guard before recursion (427-428); test 733-757 |
| "Chapter time beyond the segment duration is a tolerant note … never dropped" | chapters.rs:30-33 | REAL | flag_beyond_duration wired at start (mod.rs:461); tests 826-853 |
| "the card's 'chapter list for the UI layer'" (exposure seam) | chapters.rs:37-42 | SHALLOW | getter exists and retains, but no shipped surface reads it (F-001): host.rs:1311 hard-codes `chapters: []`, dispatch.rs:206 refuses navigation |

## 7. Dead / duplicated / unreachable code
- `ChapterTarget` + `MkvDemuxer::chapter_targets()` (mod.rs:276-310): test-only callers — the shipped binary never builds a target. The mod.rs doc admits the command layer does not exist (273-274).
- Parallel model duplication: `crates/playback-engine/src/chapters.rs` (engine-side `Chapter` model with its own resolution logic and baseline-ok tests, nofailfast:3749-3752) is never fed by any container — both chapter stacks are dead-ended (F-001).
- Nothing dead within part1 itself; every parse path is exercised by the walk on chapter-bearing files.

## 8. Dependencies & flags
- No feature gates; `pub mod chapters` unconditionally compiled into demux (used by the mkv walk on every open). Imports: media_types error/time, `crate::mkv::{ebml, error, header::MAX_TEXT_FIELD}` — all in-crate. No unsafe, no external crates.
- Baseline execution: 11/11 `mkv::chapters::tests::` ok (nofailfast:1723-1733); integration suites mkv_attachments_chapters 2/2 ok (:2089-2094), mkv_extras_wiring 9/9 ok (:2125-2133, four chapter_targets tests), mkv_editions_tags 14/14 ok — of which the corpus-gated members skipped-but-ok (F-002).

## 9. Open questions for the next agent
- For W05-A04 (chapters.rs#part2): `parse_display` reads `read_prefix_at` then checks `len <= MAX_TEXT_FIELD` — an over-ceiling text field is silently dropped (not damage-noted, unlike over-ceiling displays); `parse_track_numbers` silently ignores a 65th+ track number and non-`ChapterTrackNumber` children — consistent tolerance, but worth rows in part2's table.
- For the host owner (Wave 3/apps-cli domain): host.rs:1311 is the single call site where `ChapterList`→`Chapter` mapping would light up the entire surface for both MKV and MP4 — the engine-side model (playback-engine/src/chapters.rs) is already built and tested.
- REQUEST-ORCH-RUN (optional, would close F-002's corpus half): `LUMEN_CORPUS=<restored-dir> cargo test --release -p demux --test mkv_attachments_chapters --test mkv_editions_tags corpus` to execute the corpus differentials once with the corpus present.
