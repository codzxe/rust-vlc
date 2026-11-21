# AUDIT: crates/streaming/src/dash/resolve.rs#part1 (lines 1–648)
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-21 |
| Lines / functions | 648 (of 1297) / 12 fns + 5 types + 3 consts |
| File verdict | UNREACHABLE (real, tested code — no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — see chain below |
| Findings | S0: 0 S1: 2 S2: 1 S3: 1 S4: 3 |
| Commands run | `rg -n "plan_representation|RepresentationPlan"` → only callers are `dash/sched.rs`, `dash/fuzz.rs`, in-file tests; `rg "streaming::(dash|hls)"` apps/ → no matches; `rg streaming apps/cli/Cargo.toml` → dep exists but apps/cli/src uses only `streaming::{fetch,ranged,schemes,mockhttp}` (demux_host.rs); `sed -n` on all quoted ranges; `rg fatal_u64/timeline_entry_from_attrs` crates/playlists/src/mpd.rs (parser accepts 0); baseline INDEX #3 (1681 passed / 1 failed — failure was apps/desktop zero_alloc, not streaming). |

## 1. One-paragraph verdict
The DASH URL-resolution core (template `$Identifier$` substitution, printf tags, SegmentTimeline `t+n×d` expansion, BaseURL folding, plan budgets) is genuinely implemented, carefully overflow-checked in most places, and unit-tested — but it is **dead weight in the shipped product**: the `lumen` CLI links the `streaming` crate yet touches only `fetch`/`ranged`/`schemes`/`mockhttp`; nothing outside `crates/streaming` (or its own tests and the cargo-fuzz target) calls `plan_representation`. Within the dead lane there are two arithmetic **panic paths** (`@d="0"` with a negative `@r`, and `@duration="0"` with a known period end) that contradict the fuzz harness's "Never panics" contract, and the 16 MiB URL budget fails to charge the folded `BaseURL` prefix, so a hostile manifest can mint gigabytes of URLs — the exact scenario the budget's own doc comment claims to prevent.

## 2. What actually works (verified)
— `substitute`/`expand_timeline`/`plan_representation` logic verified by code trace; their unit tests (in part2's range, lines 778–1048) assert exact output strings and would fail under concrete mutations (e.g. breaking the `$$` escape breaks `dollar_escape_and_errors`'s `Ok("a$b.m4s")` assertion at line 833). Baseline `cargo test --workspace` (INDEX #3) passed all streaming tests (1681/1681 non-desktop), so these executed green on 2026-11-21.
— Call-chain refutation (reachability): `rg -n "plan_representation"` → `dash/sched.rs:…`, `dash/fuzz.rs:41`, in-file tests only; `rg "streaming::dash"` across apps/ → only `fuzz/fuzz_targets/fuzz_demux_mpd.rs`. `dash::DashScheduler` (re-exported by dash/mod.rs:36-37) has no consumer outside the crate.

## 3. Findings

### F-001 [S1] Division-by-zero panic: negative `@r` with `@d="0"`
Where: crates/streaming/src/dash/resolve.rs:405-417 (divisions at 410 and 416)
Code:
```
                match next_t {
                    Some(t) if t > start => {
                        let span = t - start;
                        span / entry.d + u64::from(span % entry.d != 0)
                    }
                    // … else the period end (ticks), …
                    _ => match period_end {
                        Some(end) if end > start => {
                            let span = end - start;
                            span / entry.d + u64::from(span % entry.d != 0)
```
What is wrong: `entry.d` is used as a divisor with no zero check. The MPD parser accepts `d="0"`: `crates/playlists/src/mpd.rs:1896-1898` — `attr(attrs, "d").and_then(|v| v.trim().parse::<u64>().ok()).ok_or(MpdError::BadValue { attr: "S@d" })` — only a *missing or unparseable* `@d` is refused; `0` is a legal parse result. A manifest with `<S t="0" d="0" r="-1"/><S t="10" d="5"/>` (or any period end > start) parses clean, then `expand_timeline` executes `span / 0` → panic.
Why it matters: The fuzz harness contract says "Never panics, never hangs, never allocates without a ceiling" (`crates/streaming/src/dash/fuzz.rs:50-51`) and the module doc promises typed refusals — "never a guess" (resolve.rs:26-27). A one-line MPD crashes the process running `fuzz_demux_mpd`. In the (currently dead) engine lane this is a remote crash on untrusted manifest input.
What correct looks like: reject `@d <= 0` at parse time (BadValue) or return `DashResolveError::BadValue`-equivalent from `expand_timeline` before dividing (e.g. `if entry.d == 0 { return Err(TimelineOverflow-or-new-variant) }`).
Confidence: HIGH — Verification: read (parser acceptance traced at mpd.rs:1896-1898; division traced above); execution UNVERIFIED (no cargo in sandbox per §8).

### F-002 [S1] Division-by-zero panic: `@duration="0"` on the fixed-duration path (panic site in part2, call site here)
Where: crates/streaming/src/dash/resolve.rs:578-582 (call) → :733-737 (div, part2/A06's range)
Code:
```
            let duration = template.duration.ok_or(DashResolveError::MissingFact {
                fact: "SegmentTemplate@duration",
            })?;
            let count = fixed_segment_count(mpd, period, timescale, duration)
                .ok_or(DashResolveError::TimelineExtentUnknown)?;
```
and the callee (line 735, cross-boundary): `let count = end / duration + u64::from(end % duration != 0);`
What is wrong: `SegmentTemplate@duration="0"` parses to `Some(0)` (`crates/playlists/src/mpd.rs:1819` via `fatal_u64`, mpd.rs:1664-1669 — no zero rejection), and when the period end is known (`Period@duration` or `MPD@mediaPresentationDuration`), `fixed_segment_count` divides by it.
Why it matters: same "Never panics" fuzz-contract violation as F-001; a minimal static MPD (`<Period duration="PT10S">… duration="0" …`) panics the resolver. Flagging cross-boundary so A06/W10-A06 covers the `fixed_segment_count` body explicitly.
What correct looks like: typed refusal (`MissingFact`/`BadTemplate`-class) for `duration == 0` before the division, or parser-side rejection.
Confidence: HIGH — Verification: read (both sites + parser traced); execution UNVERIFIED (no cargo).

### F-003 [S2] URL byte budget never charges the folded BaseURL prefix — bounded-memory contract is violable
Where: crates/streaming/src/dash/resolve.rs:596-618 (template path; same pattern at 641-660, SegmentList path)
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
            });
```
What is wrong: the budget is charged with `substituted.len()` but the stored `url` is `resolve_uri(&base, &substituted)`, whose length is ≈ `base.len() + substituted.len()` for relative references (`resolve_uri` merges onto the base's directory — crates/playlists/src/m3u.rs:729-738). `base` is the folded chain of `manifest_url` + up to four first-per-level `<BaseURL>`s (resolve.rs:533-546), each bounded only by the 64 KiB line cap.
Why it matters: the constant's own doc says "a hostile 64 KiB template must not multiply into gigabytes of minted URLs" (resolve.rs:46-49). It does not hold for a hostile *BaseURL*: 65,536 timeline segments (== `MAX_PLAN_SEGMENTS`, each passing because substituted is short) × a ~256 KiB folded base ≈ 16 GiB of `String` data — an unbounded-allocation/OOM violation of the fuzz harness contract (fuzz.rs:12-16) reachable purely from manifest bytes.
What correct looks like: charge `resolved.len()` (compute `resolve_uri` first, budget its output), or additionally charge `base.len()` per segment.
Confidence: HIGH — Verification: read (arithmetic traced; `resolve_uri` merge semantics read at m3u.rs:720-738); execution UNVERIFIED.

### F-004 [S3] SegmentList inheritance silently ignores Period-level SegmentList
Where: crates/streaming/src/dash/resolve.rs:629-631
Code:
```
    // ---- SegmentList (the explicit URL list; nearest level wins
    // wholesale — no partial merge across list levels).
    if let Some(list) = rep.segment_list.as_ref().or(set.segment_list.as_ref()) {
```
What is wrong: only Representation and AdaptationSet are consulted; a spec-conformant MPD carrying `<SegmentList>` at Period level (legal per ISO/IEC 23009-1 §5.3.x, and handled for `SegmentTemplate` two branches above via `period.segment_template`) yields `NoAddressing`. The comment documents "no partial merge across list levels" but not the outright drop of the Period level — the two shapes disagree.
Why it matters: a legal manifest is refused with a wrong diagnostic ("representation has no template, list, base or BaseURL" when it does have one, at Period level). Blast radius limited: lane unreachable from shipped binary.
What correct looks like: `.or(period.segment_list.as_ref())` (whole-element nearest-wins is fine), or a documented typed refusal naming the Period level.
Confidence: HIGH — Verification: read.

### F-005 [S4] `Display` renders `$$id$` instead of `$id$`
Where: crates/streaming/src/dash/resolve.rs:103-109
Code:
```
            DashResolveError::UnknownIdentifier { identifier } => write!(
                f,
                "streaming.dash.unknown-identifier: template references unknown identifier $${identifier}$"
            ),
```
What is wrong: Rust format strings only escape `{`/`}`; `$` is literal, so the output contains a doubled dollar (`…identifier $$Foo$`). The intent was clearly `$Foo$` (DASH template syntax).
Why it matters: user-facing diagnostic text is wrong on two of eight variants (same `$$` at line 109).
What correct looks like: `"...unknown identifier ${identifier}$"`.
Confidence: HIGH — Verification: read (Rust format-string semantics).

### F-006 [S4] `pad` pads by bytes, not chars
Where: crates/streaming/src/dash/resolve.rs:359-372
Code:
```
fn pad(body: String, zero_pad: bool, width: usize) -> String {
    if body.len() >= width {
        return body;
    }
```
What is wrong: `body.len()` is byte length; `%<width>s` on a multi-byte `$RepresentationID$` (e.g. 2 CJK chars, 6 bytes) pads to byte width, producing fewer visible columns than the tag promises (printf semantics count characters).
Why it matters: cosmetic correctness of minted URLs/diagnostics for non-ASCII representation ids; DASH ids are usually ASCII, so impact is marginal.
What correct looks like: `body.chars().count() >= width` with char-based repeat, or documented byte-width semantics.
Confidence: HIGH — Verification: read.

### F-007 [S4] `NoAddressing` reused for "present but incomplete" shapes — misleading diagnostics
Where: crates/streaming/src/dash/resolve.rs:567-570 and :646
Code:
```
        let media = template
            .media
            .as_deref()
            .ok_or(DashResolveError::NoAddressing)?;
```
What is wrong: a representation that *has* an effective `SegmentTemplate` but lacks `@media`, or a `SegmentURL` lacking `media`, reports "representation has no template, list, base or BaseURL" — factually false for that input.
Why it matters: error-driven debugging and any UI surfacing of `streaming.dash.no-addressing` misleads the operator.
What correct looks like: a dedicated variant (e.g. `MissingFact { fact: "SegmentTemplate@media" }`).
Confidence: HIGH — Verification: read.

## 4. Per-function table (part1 range; `plan_representation` audited in full because it starts here — its tail 649–712 is formally A06's)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Display for DashResolveError` | 100–138 | REAL | S4 | `$$` literal bug (F-005); otherwise complete, bounded |
| `impl Error for DashResolveError` | 140 | REAL | — | empty impl, correct |
| `substitute` | 193–273 | REAL | — | byte-index slicing safe (`$`/ident boundaries ASCII); `$$`, `%tag`, unbalanced-`$` all typed; no panic path found |
| `apply_int_format` | 280–299 | REAL | — | x/X/o/d/u/q; `s` refused; delegates bounds to parse_format_tag |
| `apply_string_format` | 303–315 | REAL | — | only plain/`%s`; int conv refused |
| `parse_format_tag` | 319–355 | REAL | — | width parse failure → typed error (no panic); width ≤ 99 enforced |
| `pad` | 359–372 | REAL | S4 | byte-vs-char (F-006) |
| `expand_timeline` | 389–452 | REAL | S1 | div-by-zero `@d=0`+negative `@r` (F-001); all other arithmetic checked_mul/checked_add; cap enforced before push |
| `effective_base` | 458–464 | REAL | — | `resolve_uri("", manifest_url)` returns manifest verbatim (absolute ref) — correct |
| `effective_template` | 471–490 | REAL | — | nearest-owns + outer-fills matches doc §inheritance |
| `merge_template` | 494–516 | REAL | — | all 7 fields merged; timeline included |
| `plan_representation` | 524–712 | REAL | S1,S2,S3,S4 | F-002 (581), F-003 (596-618), F-004 (631), F-007 (570); SegmentList branch budget same defect (647-649); single-resource branch (672–712) is part2's — noted, no finding of mine there |
| types `SegmentPlan`/`RepresentationPlan`/`TemplateValues`/`TimelineSlot`/`DashResolveError` | 144–187, 58–98 | REAL | — | plain data; `PartialEq` on error type enables exact-match tests |
| consts `MAX_PLAN_SEGMENTS`/`MAX_PLAN_URL_BYTES`/`SEGMENT_STRUCT_COST` | 44–53 | REAL | S2 | budget semantics miss base (F-003) |

## 5. Test honesty
No tests in part1's range (all tests live at 741–1297, A06's part2). The part2 tests covering part1 functions (`substitutes_the_four_identifiers_plain` 779, `format_tags_pad_and_convert` 793, `dollar_escape_and_errors` 831, `timeline_expansion_t_d_r` 890, `negative_r_bounded_by_next_t_or_period` 938, `timeline_overflow_and_caps_refuse_typed` 973) assert exact strings/counts and would fail under the mutations I reasoned through (break `$$` escape → 833 fails; swap pad operands → 803 fails; drop `+1` on `r` → 916-934 count fails). None cover `@d="0"` or `@duration="0"` — the two panic inputs are untested (F-001/F-002).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "SegmentTimeline … negative `@r` … else refuses typed — never a guess" | resolve.rs:21-27 (module doc) | FALSE for `@d=0` | F-001: parse-clean input panics instead of refusing |
| "a hostile 64 KiB template must not multiply into gigabytes of minted URLs" | resolve.rs:46-49 | FALSE for hostile BaseURL | F-003: 65,536 × 256 KiB folded base ≈ 16 GiB passes the budget |
| "MPD parse, template/list resolution, MPD fuzz … landed" (status partial, win/linux/macos) | docs/feature-matrix.toml `network-dash` (row ~1444) | REAL-UNPROVEN + UNREACHABLE from shipped binary | code exists & tested (baseline #3), but no shipped entry point calls it (call chain §2) |
| "Never panics, never hangs, never allocates without a ceiling" | dash/fuzz.rs:50-51 | FALSE | F-001/F-002 panic; F-003 unbounded allocation |
| "Inheritance is attribute-wise … nearest defined value wins" | resolve.rs:13-16 | REAL for SegmentTemplate; FALSE-by-omission for SegmentList | F-004 (Period-level list dropped) |

## 7. Dead / duplicated / unreachable code
The whole module is unreachable from the shipped product: `lumen` CLI (apps/cli) links `streaming` but its only streaming usage is `fetch`/`ranged`/`schemes`/`mockhttp` (apps/cli/src/demux_host.rs:…, `rg streaming:: apps/cli/src` → 8 hits, none dash/hls). `lumen-tauri` (apps/desktop-tauri/Cargo.toml) has **no** streaming dependency. Consumers of `plan_representation`: `dash/sched.rs` (itself unconsumed outside the crate), `dash/fuzz.rs` (fuzz target), in-file tests. `apps/desktop` (lumen-desktop, prototype skeleton, not a shipped entry point) also depends on streaming — no dash usage found there either (`rg "streaming::dash" apps/` → only fuzz/).

## 8. Dependencies & flags
`playlists` (workspace) — used for `Mpd` model + `resolve_uri`; `ureq =3.5.0` is the crate's HTTP dep (not touched by this file). No feature gates on the dash module (`crates/streaming/src/lib.rs:32` unconditional `pub mod dash`). Baseline build #1 compiled it clean; clippy #5 clean.

## 9. Open questions for the next agent
- A06 (part2): confirm F-002's div at :735 and audit `period_duration_ticks` f64→u64 cast (723-725) for precision loss on huge `mediaPresentationDuration` values (I did not audit it — your range).
- Whoever owns `sched.rs`/`abr.rs`: is the "session" (6.C2) ever constructed outside tests? If not, the entire `network-dash` matrix row's user-visible status is not_started-in-product regardless of code existence.
- Orchestrator: REQUEST-ORCH-RUN (panic repro, non-blocking): a test/inline check running `plan_representation` on `<MPD type="static" mediaPresentationDuration="PT10S"><Period><AdaptationSet><Representation id="v" bandwidth="1"><SegmentTemplate timescale="1" media="s$Number$.m4s"><SegmentTimeline><S t="0" d="0" r="-1"/><S t="10" d="5"/></SegmentTimeline></SegmentTemplate></Representation></AdaptationSet></Period></MPD>` — expected: panic `attempt to divide by zero` at resolve.rs:410.
