# AUDIT: crates/streaming/src/dash/resolve.rs
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-23 |
| Lines / functions | 1297 / 13 production fns + 2 test helpers + 12 tests |
| File verdict | REAL (library) — UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO (apps/cli depends on `streaming` but routes http(s) opens through `ranged::RangedResource` only; no app code calls `dash::*`) |
| Findings | S0: 0 S1: 3 S2: 1 S3: 3 S4: 2 |
| Commands run | `./target/debug/deps/streaming-c6cf1f9f1d2bf8f0 dash::resolve` -> 12 passed / 0 failed, exit 0; `rg` caller searches; `sed -n` quote verifications (all below) |

## 1. One-paragraph verdict
The resolver is genuine, carefully-engineered code: `$Identifier$` substitution with bounded printf tags, checked timeline arithmetic, typed refusals everywhere, and 12 real spec-derived tests that pass (executed from the prebuilt binary, which is newer than the source). But it has two arithmetic panic holes on attacker-controlled manifests (`@duration="0"`, `@d="0"` with negative `@r`) that falsify the fuzz harness's "panic-free by construction" contract, and a URL-bytes budget that is charged on the pre-fold relative reference instead of the resolved absolute URL, so a hostile BaseURL chain defeats the documented 16 MiB cap (up to ~GiB of minted URLs). CENC `ContentProtection` is parsed by the model and then silently dropped — the plan carries no encryption facts. Nothing in `apps/` calls this module, so no shipped entry point reaches it today.

## 2. What actually works (verified)
— `./target/debug/deps/streaming-c6cf1f9f1d2bf8f0 dash::resolve` -> `test result: ok. 12 passed; 0 failed; 0 ignored` (binary mtime 18:07 > source mtime 17:35, so current). Substitution vectors, timeline t/n×d expansion, inheritance merge, fixture resolution, typed refusals all pinned by real assertions on real fixtures.

## 3. Findings

F-001 [S1]
Where: crates/streaming/src/dash/resolve.rs:733-737
Code:
```
fn fixed_segment_count(mpd: &Mpd, period: &Period, timescale: u64, duration: u64) -> Option<u64> {
    let end = period_duration_ticks(mpd, period, timescale)?;
    let count = end / duration + u64::from(end % duration != 0);
    if count == 0 { None } else { Some(count) }
}
```
What is wrong: `end / duration` panics when `duration == 0`. The parser accepts `duration="0"` — `crates/playlists/src/mpd.rs:1819` is `template.duration = Some(fatal_u64("SegmentTemplate@duration", value)?)` and `fatal_u64` (mpd.rs:1666-1670) is a bare `parse::<u64>()` with no `> 0` check. Why it matters: a manifest `<SegmentTemplate duration="0" media="$Number$"/>` inside a period with a known end panics the resolver — untrusted-input panic in a crate whose fuzz contract (dash/fuzz.rs:50: "Never panics, never hangs, never allocates without a ceiling") promises panic-free. The correct behavior is a typed refusal (BadValue at parse, or MissingFact-style refusal here). Confidence: HIGH — Verification: RAN. Compiled probe (audit/runs/tmp/resolve_probe.rs) against the shipped rlibs with a hand-built model (`duration: Some(0)`, Period 10 s) and executed:
```
thread 'main' panicked at crates/streaming/src/dash/resolve.rs:735:17:
attempt to divide by zero
```
exit 101 — exactly the predicted line.

F-002 [S1]
Where: crates/streaming/src/dash/resolve.rs:405-421
Code:
```
                let next_t = entries[i + 1..].iter().find_map(|e| e.t);
                match next_t {
                    Some(t) if t > start => {
                        let span = t - start;
                        span / entry.d + u64::from(span % entry.d != 0)
                    }
```
What is wrong: `span / entry.d` (both occurrences, L410 and L416) divide by `entry.d` with no zero check. `timeline_entry_from_attrs` (mpd.rs:1889-1892) parses `@d` with `.parse::<u64>().ok()` — `d="0"` is accepted. A timeline entry with `@d="0"` and negative `@r` panics. Same contract falsification as F-001. What correct looks like: refuse typed (`BadValue` at parse: `@d` must be > 0). Confidence: HIGH — Verification: RAN. Compiled probe (audit/runs/tmp/resolve_probe2.rs): timeline `[S(t=0,d=0,r=-1), S(t=50,d=10)]` →
```
thread 'main' panicked at crates/streaming/src/dash/resolve.rs:410:25:
attempt to divide by zero
```
exit 101 — exactly the predicted line. (The positive-`@r` d=0 path does not divide and only mints zero-duration slots.)

F-003 [S1]
Where: crates/streaming/src/dash/resolve.rs:609-617
Code:
```
            bytes_budget = bytes_budget
                .checked_sub(substituted.len() + SEGMENT_STRUCT_COST)
                .ok_or(DashResolveError::PlanTooLarge { budget: "bytes" })?;
            segments.push(SegmentPlan {
                number,
                time: slot.time,
                duration: slot.duration,
                url: resolve_uri(&base, &substituted),
```
What is wrong: the `MAX_PLAN_URL_BYTES` budget (doc L46-49: "Budget on the accumulated URL bytes … of one representation's plan") is charged on `substituted.len()` — the relative reference — but the plan stores `resolve_uri(&base, &substituted)`, which prepends the folded BaseURL chain (up to 4 manifest-controlled `<BaseURL>` levels under the 64 KiB line cap). A manifest with a ~64 KiB BaseURL, `media="$Number$"` and 65 536 segments passes the 16 MiB budget (~4 MiB charged) while minting ~4 GiB of URL bytes — OOM on the 3 GB sandbox, defeating the documented bounded-memory contract and the fuzz harness's "never allocates without a ceiling" claim. Same pre-fold accounting in the SegmentList path (L647-649 charges `media.len()`, stores `resolve_uri(&base, media)` at L659). What correct looks like: charge the resolved URL length. Confidence: HIGH — Verification: read (arithmetic verified; no runtime OOM executed).

F-004 [S2]
Where: crates/streaming/src/dash/resolve.rs:718-729
Code:
```
    let seconds = period.duration.or_else(|| {
        mpd.media_presentation_duration
            .map(|total| total - period.start.unwrap_or(0.0))
    })?;
    let ticks_f = seconds * timescale as f64;
    if ticks_f.is_finite() && (0.0..=u64::MAX as f64).contains(&ticks_f) {
        Some(ticks_f as u64)
```
What is wrong: tick conversion through f64 truncates. Decimal-but-not-binary durations (e.g. `PT0.07S` @ timescale 44100: 0.07 stores as 0.069999…, product 3086.999…, `as u64` → 3086 instead of 3087) shift period ends and segment counts by one tick, and `$Time$`/`$Number$` then mint wrong URLs (404s) at exact boundaries. Also `u64::MAX as f64` rounds to 2^64, so the range guard admits values that saturate on cast. What correct looks like: exact integer arithmetic (parse xs:duration into integer ticks under the target timescale). Confidence: MEDIUM — Verification: read (f64 representation of 0.07 and truncating cast are language facts; no end-to-end 404 demonstrated).

F-005 [S3]
Where: crates/streaming/src/dash/resolve.rs:548-553 and 1086-1116
Code:
```
    let id_values = TemplateValues {
        representation_id: rep.id.as_deref(),
        bandwidth: rep.bandwidth,
        number: None,
        time: None,
    };
```
What is wrong: CENC is dropped. `Representation.content_protection` is parsed by the 6.C1a model ("recorded, never interpreted", mpd.rs:382) and `resolve.rs` never reads it — `rg content_protection resolve.rs` hits only test struct literals (L1134/1153/1240/1258). `RepresentationPlan` (L163-178) has no protection field, so a CENC manifest (the `hand_static_cenc.mpd` fixture) resolves to plaintext-assumed URLs with zero indication downstream that segments are ciphertext — the 6.C2 consumer cannot know it needs a DRM lane. The CENC fixture test (L1086-1116) asserts only `seg.url.starts_with("http…")` — it proves URL absoluteness, nothing about CENC. What correct looks like: propagate `content_protection` (or at least a "protected" flag) into the plan. Confidence: HIGH — Verification: read + rg.

F-006 [S3]
Where: crates/streaming/src/dash/resolve.rs:631
Code:
```
    if let Some(list) = rep.segment_list.as_ref().or(set.segment_list.as_ref()) {
```
What is wrong: SegmentList is consulted only at Representation and AdaptationSet levels; the `Period` model has no `segment_list` field (see test struct at L1157-1165), so a spec-valid Period-level SegmentList (ISO/IEC 23009-1 §5.3.9.5 allows MPD/Period/Set/Rep) leaves the representation with no addressing → typed `NoAddressing` refusal instead of a plan. Same class: MPD-level SegmentTemplate is not modeled. Honest-scope but wrong behavior on valid manifests. Confidence: HIGH — Verification: read (model structs at L1157-1179 have no such fields).

F-007 [S3]
Where: crates/streaming/src/dash/resolve.rs:735-736
Code:
```
    let count = end / duration + u64::from(end % duration != 0);
    if count == 0 { None } else { Some(count) }
```
What is wrong: a known-zero-extent period (duration 0 → count 0) is reported as `None`, which the caller (L581-582) converts to `TimelineExtentUnknown` — "extent unknown" for an extent that is exactly known (empty). Misleading error; should be an empty plan. Confidence: HIGH — Verification: read.

F-008 [S4]
Where: crates/streaming/src/dash/resolve.rs:406-413
Code:
```
                let next_t = entries[i + 1..].iter().find_map(|e| e.t);
                match next_t {
                    Some(t) if t > start => {
```
What is wrong: module doc (L23-25) says a negative `@r` "resolves against the next entry's `@t` when present" — the code requires `t > start`; a next `@t` that is ≤ `start` (malformed but present) silently falls through to the period-end branch instead of refusing, i.e. it guesses. Doc/behavior mismatch. Confidence: HIGH — Verification: read.

F-009 [S4]
Where: crates/streaming/src/dash/resolve.rs:359-372
Code:
```
fn pad(body: String, zero_pad: bool, width: usize) -> String {
    if body.len() >= width {
```
What is wrong: `body.len()` is bytes; printf width counts characters — a multi-byte `RepresentationID` pads to the wrong visual width. Cosmetic (IDs are ASCII in practice). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Display::fmt (DashResolveError) | 101-137 | REAL | — | stable dotted codes, no unbounded strings; $$ in format strings is literal (writes `$${identifier}$` — the `$$` is a Rust format escape producing `$`) |
| substitute | 193-273 | REAL | — | byte-safe slicing at ASCII `$`; `$$`, `%tag`, unbalanced `$`, unknown idents all handled; verified by tests |
| apply_int_format | 280-299 | REAL | — | d/u/q/x/X/o; refuses %s on ints |
| apply_string_format | 303-315 | REAL | — | only %s; refuses int conversions |
| parse_format_tag | 319-355 | REAL | — | width>99 refused (bounded output); no flag chars accepted (spec-faithful) |
| pad | 359-372 | REAL | S4 | byte-vs-char width (F-009) |
| expand_timeline | 389-452 | REAL | S1 | div-by-zero @d=0+negative @r (F-002); r+1 semantics, checked mul/add, cap-enforced |
| effective_base | 458-464 | REAL | — | resolve_uri("", absolute) returns verbatim (m3u.rs:684-686) — chain fold correct |
| effective_template | 471-490 | REAL | — | nearest-element base + outer fill; verified by inheritance test |
| merge_template | 494-516 | REAL | — | field-wise fill; timeline wins over duration at use site |
| plan_representation | 524-712 | REAL | S1/S2 | budget accounting bug (F-003); CENC drop (F-005); template/list/single-resource shapes all implemented |
| period_duration_ticks | 718-729 | REAL | S2 | f64 truncation drift (F-004) |
| fixed_segment_count | 733-737 | REAL | S1 | div-by-zero (F-001), zero-extent mislabel (F-007) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| substitutes_the_four_identifiers_plain | YES | exact expected string across all 4 idents |
| format_tags_pad_and_convert | YES | %05d/%03x/%o/%6d/%3s vectors with exact outputs |
| dollar_escape_and_errors | YES | pins UnknownIdentifier/MissingValue/BadTemplate variants incl. width>99 refusal |
| timeline_expansion_t_d_r | YES | 14-slot exact (time,duration) vector incl. inherited t and negative-r-to-period-end |
| negative_r_bounded_by_next_t_or_period | YES | pins TimelineExtentUnknown and next-@t bounding (span 35, d=10 → 4 repeats) |
| timeline_overflow_and_caps_refuse_typed | YES | u64::MAX t → TimelineOverflow; r=100 vs cap 10 → PlanTooLarge |
| ffmpeg_template_fixture_resolves | YES | full parse+plan over real fixture; exact init/media URLs, timescale, tick values |
| ffmpeg_template_fixture_audio_two_segments | YES | second adaptation set; exact t/d/number/url of segment 2 |
| ffmpeg_list_fixture_resolves_explicit_urls | YES | list path over real fixture (weaker: prefix asserts only) |
| hand_fixture_multi_level_baseurl_and_pto | PARTIAL | only asserts absolute-URL prefixes — proves BaseURL folding, proves nothing about CENC despite the fixture's name/purpose |
| no_addressing_refuses_typed | YES | exact NoAddressing on empty model |
| template_inheritance_nearest_field_wins | YES | exact merged URLs/numbers/init from hand-built model |
Coverage gap: no test with `@duration="0"`, `@d="0"`, or a giant-BaseURL budget attack — exactly the F-001/002/003 holes.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Budget on the accumulated URL bytes … of one representation's plan" | resolve.rs:46-49 | FALSE in effect | charged pre-fold (F-003); resolved URLs escape the budget |
| "Never panics, never hangs, never allocates without a ceiling" (fuzz harness over this resolver) | dash/fuzz.rs:50 | FALSE | F-001/F-002 div-by-zero panics; F-003 unbounded-by-contract alloc |
| "no I/O, no clock — pure function" | resolve.rs:10-11 | REAL | no I/O in body; verified by read |
| "A negative @r … resolves against the next entry's @t when present … never a guess" | resolve.rs:22-27 | FALSE (edge) | F-008: next @t ≤ start falls back to period end |
| "first <BaseURL> per level" folding | resolve.rs:17-19 | REAL | L533-545 `.first()` at each level |

## 7. Dead / duplicated / unreachable code
No dead code. The whole module is UNREACHABLE from shipped entry points: `apps/cli/Cargo.toml:62` depends on `streaming`, but the CLI's http(s) lane is `streaming::ranged::RangedResource` (apps/cli/src/demux_host.rs:461-492); `rg "dash::" apps/` → 0 hits. In-crate consumers: `dash::sched` (L299) and `dash::fuzz` (L69).

## 8. Dependencies & flags
Depends only on `playlists` (AdaptationSet/Mpd/Period/Representation/SegmentTemplate/SegmentTimelineEntry/resolve_uri). No feature gates on this path. No unsafe.

## 9. Open questions for the next agent
- Does `dash/sched.rs` re-check or propagate content protection (it receives RepresentationPlan, which lacks the field — so no)? Confirm the 6.C2 consumer's CENC posture.
- Does the mpd parser warn when a Period-level SegmentList is dropped (see F-006)? If silent, raise to parser-audit.
- Is a `$SubNumber$` identifier required by any targeted profile? (UNVERIFIED — I could not confirm ISO 23009-1's current identifier set from the repo alone.)
