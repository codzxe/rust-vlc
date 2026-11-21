# AUDIT: crates/streaming/src/dash/resolve.rs#part2
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-21 |
| Chunk boundaries | **part2 = lines 524–1297** (file is 1297 lines). The half-file midpoint (648) falls inside `plan_representation` (524–712), so the split was moved outward to that function's start; part1 (A07) = lines 1–523 (module docs, `DashResolveError`, plan structs, `substitute`/format-tag math, `expand_timeline`, base folding, `effective_template`/`merge_template`). This audit covers 524–1297 only. |
| Lines / functions | 774 / 17 (3 fns + 2 test helpers + 12 test fns) |
| File verdict | UNREACHABLE (code is REAL and tested, but no shipped entry point reaches it — see reachability) |
| Reachable from shipped entry point? | **NO** — call chain: `plan_representation` ← `DashScheduler::new` (sched.rs:299) and `dash::fuzz::walk` (fuzz.rs:69). `DashScheduler` has ZERO callers outside sched.rs's own `#[cfg(test)]` (rg over apps/ + crates: no matches for `DashScheduler` outside sched.rs); `apps/cli` uses `streaming::{fetch, ranged, schemes, mockhttp}` only (rg "streaming::" apps/cli/src → demux_host.rs fetch/ranged/schemes/mockhttp, no `dash`). The `lumen` CLI links the code (streaming is a dep of apps/cli, no feature gate) but never calls it. |
| Findings | S0: 0 S1: 1 S2: 1 S3: 2 S4: 1 |
| Commands run | `rg -n "plan_representation"` → callers sched.rs:299, fuzz.rs:69, mod.rs:35 re-export, tests. `rg -n "DashScheduler"` apps/ crates/ → no non-test callers. `rg -n "streaming::" apps/cli/src` → fetch/ranged/schemes/mockhttp only. `rg -n "dash" docs/feature-matrix.toml` → row `network-dash`. `sed -n` on every quoted range. `rg "MAX_LINE_BYTES" crates/playlists/src/mpd.rs` → 64 KiB line cap enforced (mpd.rs:511-514, 2006-2015). Fixture files listed + read (`ffmpeg_av_template.mpd` is genuine ffmpeg-dash-muxer output shape). |

## 1. One-paragraph verdict
The resolution core in my chunk (`plan_representation`, `period_duration_ticks`, `fixed_segment_count` + the 27-test-strong test suite's 12 tests that live here) is genuinely implemented: typed refusals everywhere, checked arithmetic throughout, real ffmpeg-shaped fixtures with hand-verified expectations, and tests that would fail under concrete mutations. But the module's headline budget contract is breakable: `MAX_PLAN_URL_BYTES` is charged against the *pre-folding* substituted URL while the *stored* URL is the base-folded one, so a hostile 64 KiB `<BaseURL>` (legal under the parser's 64 KiB line cap) times a max-count timeline mints ~4 GiB of `String`s while charging ~4.6 MB — an OOM on a parse-clean 8 MiB input that the fuzz harness explicitly promises to survive. Plus a spec-wrong `Period@start` fallback dressed up as "spec §5.3.2.1". And none of it is reachable from the shipped binary — the DASH subsystem has no host.

## 2. What actually works (verified)
— `plan_representation` produces correct URLs for the committed ffmpeg template fixture: traced `ffmpeg_av_template.mpd` (timescale 10240, `S t="0" d="61440"`, `media="chunk-stream$RepresentationID$-$Number%05d$.m4s"`) through `substitute`+`effective_base`+`resolve_uri` by hand → `http://ex.test/vod/chunk-stream0-00001.m4s`, matching test `ffmpeg_template_fixture_resolves` (resolve.rs:996-1017). Verified by reading the fixture + code trace (no cargo runs allowed; baseline `cargo test --workspace` = 1681 passed/1 failed — the single failure is `apps/desktop/tests/zero_alloc_transport.rs`, not any streaming test, so these 12 passed; audit/runs/baseline/INDEX.md #3).
— Inheritance semantics: `template_inheritance_nearest_field_wins` (resolve.rs:1193-1296) constructs a Period-level template (duration/media/startNumber/init) + rep-level media override and asserts the merged plan — would fail if `effective_template`/`merge_template` (part1) regressed; verified the test's model construction matches the real `SegmentTemplate` shape (mpd.rs:299-316).

## 3. Findings

### F-001 [S1] — URL-byte budget charges the pre-folding URL, not the stored one: hostile BaseURL × max timeline = ~4 GiB allocation, OOM
Where: resolve.rs:609-617 (template branch; same hole at 647-649, SegmentList branch)
Code:
```rust
            bytes_budget = bytes_budget
                .checked_sub(substituted.len() + SEGMENT_STRUCT_COST)
                .ok_or(DashResolveError::PlanTooLarge { budget: "bytes" })?;
            segments.push(SegmentPlan {
                number,
                time: slot.time,
                duration: slot.duration,
                url: resolve_uri(&base, &substituted),
            });
```
What is wrong: the budget is debited `substituted.len()` — the *relative, pre-folding* media string — but the allocation actually kept is `resolve_uri(&base, &substituted)`, i.e. ~`base.len() + substituted.len()` per segment. `base` is the folded BaseURL chain (manifest URL + first `<BaseURL>` of MPD/Period/Set/Rep, lines 533-546); each level is a verbatim manifest string under the parser's 64 KiB line cap (mpd.rs:511-514). A parse-clean hostile MPD with one 64 KiB `<BaseURL>` and a single `<S d="1" r="65535">` (or `@duration="1"` over a 65536-tick period) mints 65536 segments × ~64 KiB ≈ **4 GiB** of stored URL `String`s while the budget is debited 65536 × (≈10 + 64) ≈ 4.8 MB — far under the 16 MiB ceiling, so no typed refusal fires. Four stacked 64 KiB BaseURL levels ≈ 16 GiB.
Why it matters: the module's own doc contract (lines 46-49, "a hostile 64 KiB template must not multiply into gigabytes of minted URLs") and the fuzz harness's contract ("never allocates without a ceiling", fuzz.rs:50; walk ceilings `WALK_TOTAL_SEGMENTS = MAX_PLAN_SEGMENTS`, fuzz.rs:47) are both violated — the `fuzz_demux_mpd` target OOMs on an ≤8 MiB input, which is exactly the input class it exists to survive. Any future DASH host inherits the OOM. Not reachable from the shipped binary today (see header), which is the only thing keeping this from being a shipped-DoS.
What correct looks like: debit `resolved.len() + SEGMENT_STRUCT_COST` after folding (charge the `url` actually pushed: `let url = resolve_uri(&base, &substituted); bytes_budget = bytes_budget.checked_sub(url.len() + SEGMENT_STRUCT_COST)…`), same fix in the SegmentList branch.
Confidence: HIGH — Verification: read (arithmetic traced: `expand_timeline` cap admits exactly 65536 slots from one `r=65535` entry; loop at 597-618 stores one folded String per slot). Not executed — no cargo allowed. REQUEST-ORCH-RUN: `cargo test -p streaming dash::resolve -- --nocapture` after adding a test with a 64 KiB `<BaseURL>` + `r="65535"` timeline asserting memory stays bounded (or that `PlanTooLarge{bytes}` fires).

### F-002 [S2] — `Period@start` fallback defaults to 0 for non-first periods, contradicting the spec it cites
Where: resolve.rs:714-722
Code:
```rust
/// The Period's end tick (relative to the period start) under a
/// timescale: `Period@duration` when present, else
/// `MPD@mediaPresentationDuration − Period@start` (a missing `@start`
/// is the MPD's beginning — spec §5.3.2.1), else `None`.
fn period_duration_ticks(mpd: &Mpd, period: &Period, timescale: u64) -> Option<u64> {
    let seconds = period.duration.or_else(|| {
        mpd.media_presentation_duration
            .map(|total| total - period.start.unwrap_or(0.0))
    })?;
```
What is wrong: ISO/IEC 23009-1 §5.3.2 defines a missing `Period@start` as the **end of the previous Period** (only the first Period defaults to the MPD start). The code substitutes 0.0 for every period. For a 2-period static MPD (P0: 40 s with `@start`, P1: no `@start`, no `@duration`, `mediaPresentationDuration="PT100S"`), P1's computed end is 100 s instead of 60 s.
Why it matters: `period_duration_ticks` feeds both the fixed-segment count (581) and the negative-`@r` timeline bound (573-574): the overestimate mints phantom segments (404s at fetch time) or tips the plan into `PlanTooLarge` refusals on manifests that are spec-conformant. The doc comment presents the wrong rule *as a citation of the spec*, so the defect is armored as conformance.
What correct looks like: when `@start` is absent, use the previous Period's `@start + @duration` (falling back to 0 only for `periods[0]`); if neither is derivable, return `None` (the "extent unknowable" refusal path already exists).
Confidence: MEDIUM — Verification: read + spec knowledge; the ISO text is not in the sandbox (UNVERIFIED against the literal standard; no network allowed). The internal inconsistency (comment claims period-relative end while defaulting to MPD-relative 0) is verifiable from the code alone.

### F-003 [S3] — zero-length period reports "extent unknown" instead of an empty plan
Where: resolve.rs:733-737 (with 581-582)
Code:
```rust
fn fixed_segment_count(mpd: &Mpd, period: &Period, timescale: u64, duration: u64) -> Option<u64> {
    let end = period_duration_ticks(mpd, period, timescale)?;
    let count = end / duration + u64::from(end % duration != 0);
    if count == 0 { None } else { Some(count) }
}
```
What is wrong: a period whose extent is *known* to be zero (`Period@duration="PT0S"` or a computed end of 0 ticks) yields `count == 0` → `None` → caller maps it to `TimelineExtentUnknown` (581-582). The extent is not unknown; it is zero, and the correct plan is an empty segment list.
Why it matters: a degenerate-but-legal period produces a misleading typed error (`streaming.dash.timeline-extent-unknown`) where the honest answer is 0 segments; error-string consumers and debug sessions are pointed at the wrong cause.
What correct looks like: return `Some(0)` for a zero end and let `plan_representation` emit an empty `segments` vec (the plan type already permits it), reserving `None` for genuinely missing facts.
Confidence: HIGH — Verification: read.

### F-004 [S3] — SegmentBase branch claims indexRange is "recorded" but discards it
Where: resolve.rs:672-675 (comment) and 699-711 (the plan it actually builds)
Code:
```rust
    // ---- SegmentBase / single-resource shape: one segment at the
    // representation's own resolved BaseURL (a SegmentBase's
    // indexRange splits that resource — byte-range interpretation
    // is the fetch layer's, recorded not guessed here).
```
(and)
```rust
    Ok(RepresentationPlan {
        representation_id: rep.id.clone(),
        bandwidth: rep.bandwidth,
        timescale,
        presentation_time_offset: pto,
        init_url: None,
        segments: vec![SegmentPlan {
```
What is wrong: the comment says the byte-range facts are "recorded" for the fetch layer, but `RepresentationPlan`/`SegmentPlan` (146-178) have no range fields; `SegmentBase@index_range`, `@indexRangeExact` and `<Initialization range>` (mpd.rs:369-380) are parsed by the model and then dropped on the floor here (`init_url: None` unconditionally). Nothing is recorded — the information never reaches the fetch layer the comment defers to.
Why it matters: a reader (or the matrix's evidence string "SegmentBase … typed plan budgets") is told the shape is carried through; in reality every SegmentBase plan is indistinguishable from "one whole-file segment, no init", so the fetch layer cannot implement the promised byte-range interpretation even if it wanted to.
What correct looks like: either extend `SegmentPlan`/`RepresentationPlan` with an optional range field populated from `segment_base.index_range`/`initialization_range`, or change the comment to "dropped, not recorded" and mark the SegmentBase lane refused-typed.
Confidence: HIGH — Verification: read (model fields at mpd.rs:369-380 confirmed present and unread in this branch).

### F-005 [S4] — SegmentList lookup ignores the Period level; the model can't even represent it
Where: resolve.rs:629-631
Code:
```rust
    // ---- SegmentList (the explicit URL list; nearest level wins
    // wholesale — no partial merge across list levels).
    if let Some(list) = rep.segment_list.as_ref().or(set.segment_list.as_ref()) {
```
What is wrong: ISO/IEC 23009-1 permits `<SegmentList>` at Period level too; this code considers only Representation and AdaptationSet. To be fair to this file, the parsed model has no `Period.segment_list` field at all (`pub struct Period`, mpd.rs:194-208 — the parser attaches SegmentList only to set/rep, mpd.rs:1523-1526), so resolve cannot consider what the model drops; the defect's root is in `playlists::mpd`.
Why it matters: a Period-level SegmentList manifest silently falls through to the SegmentBase/no-addressing path and refuses with `NoAddressing` for a spec-conformant document. Blast radius is small (Period-level SegmentList is rare in the wild), and the matrix notes multi-Period work as pending.
What correct looks like: model field + parser attach at Period level, then `.or(period.segment_list.as_ref())` here.
Confidence: HIGH — Verification: read of both files.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `plan_representation` | 524-712 | REAL (code-traced; exercised by tests) — UNREACHABLE from shipped entry | — | Typed refusals + checked arithmetic throughout (578-582, 586, 589-590, 598-600, 609-611, 643-657). Init-template substitution correctly omits Number/Time values (548-566) per §5.3.9. F-001 (budget hole) and F-004 live here. `Vec::with_capacity(slots.len().min(MAX_PLAN_SEGMENTS))` at 595 is redundant-but-harmless (slots already capped). |
| `period_duration_ticks` | 718-729 | REAL, spec-drift | S2 (F-002) | Negative/NaN/inf guarded by the range check (724); f64→u64 truncation acceptable for tick math. Wrong `@start` default (721). |
| `fixed_segment_count` | 733-737 | REAL, edge defect | S3 (F-003) | Ceil-division correct; `count == 0 → None` misreports a known-zero extent. |
| `tests::fixture` | 746-755 | REAL (test helper) | — | `unreachable!` on fs error — panic-in-test only, acceptable; fixtures verified to exist. |
| `tests::first_video_plan` | 757-774 | REAL (test helper) | — | Hard-coded `http://ex.test/...` reserved-host literals, correctly `lumen-allow-literal`-annotated. |
| `tests::substitutes_the_four_identifiers_plain` | 778-790 | REAL | — | Asserts exact output; would fail on any substitution regression. |
| `tests::format_tags_pad_and_convert` | 792-828 | REAL | — | Covers %05d/%03x/%o/space-pad/%3s; would fail on padding/format bugs. |
| `tests::dollar_escape_and_errors` | 830-885 | REAL | — | Covers `$$`, unknown id, missing value, unbalanced `$`, bad conversion, width>99 refusal. |
| `tests::timeline_expansion_t_d_r` | 889-935 | REAL | — | Exact 14-slot vector incl. inherited-t and negative-r; strong. |
| `tests::negative_r_bounded_by_next_t_or_period` | 937-970 | REAL | — | Covers the typed refusal + next-@t bound; matches expand_timeline part1 code. |
| `tests::timeline_overflow_and_caps_refuse_typed` | 972-992 | REAL | — | u64::MAX t and r=100-over-cap. |
| `tests::ffmpeg_template_fixture_resolves` | 996-1017 | REAL | — | Real fixture (read it: ffmpeg dash-muxer shape), hand-checkable expectations. |
| `tests::ffmpeg_template_fixture_audio_two_segments` | 1019-1048 | REAL | — | Two-S-entry timeline + timescale 44100. |
| `tests::ffmpeg_list_fixture_resolves_explicit_urls` | 1050-1083 | REAL | — | SegmentList lane; prefix assertions only (weaker, but combined with init/number checks acceptable). |
| `tests::hand_fixture_multi_level_baseurl_and_pto` | 1085-1116 | SHALLOW-ish assertion | — | Only asserts `http(s)://` prefix on minted URLs — does not pin the folded path values; a base-folding regression that produces the wrong absolute path still passes. Noted, not counted as a finding (other tests pin folding). |
| `tests::no_addressing_refuses_typed` | 1118-1191 | REAL | — | Full hand-built model; asserts the exact typed refusal. |
| `tests::template_inheritance_nearest_field_wins` | 1193-1296 | REAL | — | Nearest-field inheritance pinned end-to-end (URLs + numbers + init). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 12 above except `hand_fixture_multi_level_baseurl_and_pto` | YES | Exact-value assertions on outputs of the functions in this chunk (mutation: delete the `pad` zero-pad branch → `format_tags_pad_and_convert` fails; change `unwrap_or(0.0)` → `unwrap_or(1.0)` in `period_duration_ticks` → `template_inheritance_nearest_field_wins` (Period 4 s) still passes — that test uses `Period@duration`, so F-002 is untested; no test covers multi-period fallback). |
| `hand_fixture_multi_level_baseurl_and_pto` | NO (not for folding correctness) | Asserts only scheme prefixes; a wrong-but-absolute URL passes. |
| F-001 absence | — | No test anywhere feeds a long `<BaseURL>` with a large timeline — the budget hole is invisible to the suite (the fuzz mutation sweep mutates fixtures whose BaseURLs are tiny). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "a hostile 64 KiB template must not multiply into gigabytes of minted URLs" | resolve.rs:46-49 (module doc) | FALSE for the BaseURL vector (the 64 KiB *template* is covered; a 64 KiB *BaseURL* is not) | F-001; code at 609-617 charges `substituted.len()`, stores the folded URL. |
| "no I/O, no clock — pure function of the parsed model + the manifest URL" | resolve.rs:8-12 | REAL | Whole chunk is pure; no clock/fs/net calls (read). |
| "SegmentTemplate $Identifier$ substitution incl. printf format tags, SegmentTimeline t+n×d expansion incl. negative @r, SegmentList/SegmentBase, BaseURL folding, typed plan budgets" | docs/feature-matrix.toml `network-dash` evidence (line 1453) | REAL (code exists, tests pass per baseline #3) — but the whole feature is UNREACHABLE from the shipped binary (DashScheduler has no host; apps/cli never imports `streaming::dash`) | Call-chain grep in header; baseline INDEX #3. |
| "streaming 191 lib tests (2 ignored) incl. 27 dash" | matrix network-dash evidence | REAL | Counted: resolve 12 + sched 7 + abr 4 + fuzz 4 = 27 `#[test]` in crates/streaming/src/dash/. |
| matrix status `network-dash = partial` on win/linux/macos | feature-matrix.toml:1449 | MISLEADING | "Partial" for *playback* suggests some DASH playback is possible; no shipped path opens an MPD at all. The notes column does disclose pending items but not the zero-hosting fact. |

## 7. Dead / duplicated / unreachable code
- The entire `dash` module (incl. this chunk) is compiled into the shipped `lumen` binary (apps/cli → streaming dep, no feature gate) but never called: dead weight in the shipped artifact, live code for tests/fuzz.
- Part1 `Display` bug (cross-chunk, for A07): resolve.rs:105 and 109 contain `"$${identifier}$"` — Rust renders `$$` as two literal dollars, so the error string is `...identifier $$Unknown$` (doubled `$`). Verified with `cat -A` (raw bytes: `$${identifier}$`).

## 8. Dependencies & flags
- No feature gates on `dash` (crates/streaming/Cargo.toml has no `[features]`); always compiled.
- Uses `playlists::{resolve_uri, Mpd, Period, AdaptationSet, Representation, SegmentTemplate, SegmentTimelineEntry, parse_mpd_bytes}` — model gaps noted in F-005 belong to playlists/src/mpd.rs (A03/A05's chunk).
- Test-only: `std::fs` for fixtures; no dev-dep abuse.

## 9. Open questions for the next agent
- For A07 (resolve.rs part1): the `$$` double-dollar in `Display` (105, 109); also please confirm the 64 KiB "line cap" claim in the `UnknownIdentifier` doc (line 62) against mpd.rs's actual cap (it is `MAX_LINE_BYTES`, verified 511-514).
- For A02 (sched.rs): does your availability math consume `SegmentPlan.time` as period-relative ticks consistent with `period_duration_ticks`' period-relative end (573)? If sched treats @t as MPD-absolute, the negative-@r bound here is inconsistent for periods with `@start > 0`.
- For A10 (fuzz.rs): `WALK_TOTAL_SEGMENTS = MAX_PLAN_SEGMENTS` permits a single representation to mint the full 65536 segments — with F-001 that is the OOM vector; consider a bytes-based walk ceiling too.
- For the mpd.rs owners (A03/A05): Period-level `<SegmentList>` is dropped at parse (1523-1526 attach only set/rep) — root cause of F-005.
