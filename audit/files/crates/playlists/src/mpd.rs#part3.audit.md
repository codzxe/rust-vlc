# AUDIT: crates/playlists/src/mpd.rs#part3 (lines 1143-1715)
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-21 |
| Chunk boundary | part3 = **lines 1143-1715**: the tail of `Driver::on_start` (the `SegmentTemplate` arm's struct literal opens at 1138 — its match arms from `SegmentList` on are mine), all Driver methods through `finish`, `attach_segment_list`, and the attr-plumbing helpers through the end of `warn_bool` (1715). `apply_period_attrs` (1717+) is part4 (W10-A01). part2 (W10-A03) ends at 1142 mid-literal; I read 973-1142 for context (Frame enum, Driver struct, push/feed/on_start head) but did not audit it. |
| Lines / functions | 573 / 18 (see §4) |
| File verdict | UNREACHABLE (real, robust tree-builder — but the MPD parser has no production caller; only `#[cfg(test)]` use-sites and the cargo-fuzz target invoke it) |
| Reachable from shipped entry point? | **NO.** `rg -ln "parse_mpd" crates/ apps/` → `playlists/src/mpd.rs` (definition), `playlists/src/lib.rs:87` (re-export), and `streaming/src/dash/{abr,sched,resolve}.rs` (**all `#[cfg(test)]` imports**, verified: abr.rs:330, sched.rs:477, resolve.rs:744 are inside `mod tests`) plus `streaming/src/dash/fuzz.rs` (cargo-fuzz harness, not shipped). The playlists crate is *linked* into the CLI (via streaming) but no runtime path parses an MPD. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "^(pub )?(fn\|struct\|enum\|impl\|mod\|const)" crates/playlists/src/mpd.rs` (structure map); Read 973-1722 (range + context); `sed -n '1479,1488p;1648,1650p;1066,1077p;1616,1624p'` (quote verification); `rg -ln "parse_mpd" crates/ apps/` + `rg -n "parse_mpd" crates/streaming/src/ crates/playlists/src/lib.rs` (reachability); `rg -n "mpd::tests::" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (16/16 tests ok at :4278-4295). |

## 1. One-paragraph verdict
This range is the heart of the hand-rolled MPD tree builder, and it is careful work: depth-guarded pushes, name-matched end tags with dirty-world salvage (missing intermediate closes attach what was built instead of erroring), first-wins duplicate-segment policy with typed warnings, and a consistent fatal-vs-warn split (timeline numerics fatal, representation facts warn). I traced the salvage loop (`on_end` 1589-1593) by hand on a 4-deep stack — pop/attach order is correct. The defects are policy inconsistencies in `attach` (silent drop of misplaced AdaptationSet/Representation where every sibling arm warns — F-001) and two comment/behavior drifts (F-002, F-003). The killer fact for this wave: none of it runs in the shipped binary — the parser is exercised only by tests (16/16 green in baseline) and the fuzz target; a user opening a `.mpd` never reaches this code.

## 2. What actually works (verified)
— `mpd::tests::*` 16/16 `ok` — `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4278-4295`, including `dirty_world_degrades_to_warnings`, `truncated_refuses`, `depth_exceeded_refuses`, `bounded_input_battery`, `fuzz_lite_mutations_never_panic` (the module's own test names cover exactly this range's behaviors).
— Salvage semantics hand-traced: stack `[Root, Period, AdaptationSet, Representation]`, end tag `Period` → match_idx=1, pops Representation (attaches to AdaptationSet), AdaptationSet (attaches to Period), Period (pushes to `mpd.periods`) — each attach sees the correct parent because pops are innermost-first.
— Depth guard: every push in my range routes through `Driver::push` (1023-1031) which enforces `MAX_MPD_DEPTH` (64) — including the `S`, `SegmentURL`, `Initialization`, and descriptor arms' non-self-closing tails.

## 3. Findings

### F-001 [S3]
Where: crates/playlists/src/mpd.rs:1479-1488 (and 1478)
Code:
```
            Frame::AdaptationSet(set) => {
                if let Some((_, _, Frame::Period(period))) = self.stack.last_mut() {
                    period.adaptation_sets.push(set);
                }
            }
            Frame::Representation(rep) => {
                if let Some((_, _, Frame::AdaptationSet(set))) = self.stack.last_mut() {
                    set.representations.push(rep);
                }
            }
```
What is wrong: These two `attach` arms silently discard the frame when the parent does not match: a `Representation` closing under a `Period` (or Root), or an `AdaptationSet` closing under Root, vanishes with **no warning** — while the sibling arms in the very same match (`SegmentTemplate` 1514-1519, `SegmentList` 1528-1533, `SegmentBase` 1544-1549, `SegmentTimeline` 1560-1565) all push `MpdWarning::UnsupportedHere`. The reach is real: `on_start` pushes `Frame::AdaptationSet`/`Frame::Representation` unconditionally (1092-1136), so `<MPD><Representation id="x" bandwidth="1"/></MPD>` parses "successfully" and yields an MPD with zero representations, zero warnings. Similarly `Frame::Period(period) => self.mpd.periods.push(period)` (1478) flattens a Period nested inside an AdaptationSet/Period into a top-level period with no diagnostic.
Why it matters: The module's stated posture is warn-tolerance for dirty-world manifests (every other misplacement warns). Here malformed structure is swallowed without a trace: a manifest whose AdaptationSets sit one level wrong produces a silently empty ladder, and the downstream consumer (`DashScheduler::new`) reports `BadLadder` — "empty representation ladder" — with the true cause (misplacement) invisible. Diagnosis cost for exactly the malformed inputs this parser claims to tolerate.
What correct looks like: Add `_ => self.warnings.push(MpdWarning::UnsupportedHere { line, name: "AdaptationSet"/"Representation"/"Period" })` arms mirroring the SegmentTemplate/SegmentList pattern.
Confidence: HIGH — Verification: read (all `on_start` push sites and all `attach` arms cross-checked; no other warning emission on these paths).

### F-002 [S4]
Where: crates/playlists/src/mpd.rs:1648-1650
Code:
```
// Segment-template attachment inlines the first-wins rule (see the
// `attach` arms); `attach_segment_list` below is the list-shape
// helper of the same rule.
```
What is wrong: The comment says `attach_segment_list` is "below" — the function is **above** it (1635-1646); the comment sits after the function it describes, pointing the reader at nothing.
Why it matters: A maintainer following the pointer scans downward and finds `attr`/`fatal_u64` instead — small, but this file's comments are otherwise load-bearing documentation of parse posture.
What correct looks like: Move the comment above `attach_segment_list` (or drop "below").
Confidence: HIGH — Verification: read (function at 1635-1646, comment at 1648-1650).

### F-003 [S4]
Where: crates/playlists/src/mpd.rs:1066-1077 vs 1616-1623
Code:
```
        if self.stack.is_empty() {
            // After the root closed: structural garbage, skipped.
            self.warnings.push(MpdWarning::StrayElement {
```
```
    fn finish(&mut self) -> Result<(), MpdError> {
        if !self.root_seen {
            return Err(MpdError::NotMpd);
        }
        if let Some((_, line, _)) = self.stack.last() {
            return Err(MpdError::Truncated { line: *line });
        }
```
What is wrong: The on_start comment promises post-root stray elements are "skipped", but a non-self-closing stray is *pushed* as `Frame::Unknown` (1075) and if never closed, `finish` turns it into a fatal `Truncated` error — `</MPD><foo>` (no close) refuses the whole document instead of skipping the garbage.
Why it matters: Comment/behavior drift on an edge a dirty-world manifest can hit (trailing junk after the root element); the code is arguably right (truncation posture) but the comment claims the opposite behavior.
What correct looks like: Comment says: "skipped if balanced/self-closing; an unclosed stray is treated as truncation at EOF (finish)."
Confidence: HIGH — Verification: read (push at 1075 → finish's stack check at 1622-1623; no pop path for Unknown other than a matching end tag or salvage into a deeper match).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Driver::on_start` (tail arms; head is part2) | 1046-1320 (mine: 1143-1320) | REAL | — | Arms: SegmentList/SegmentBase/SegmentTimeline construction + `push_or_close`; `S` (timeline-only, fatal entry parse via `timeline_entry_from_attrs?`); `Initialization` (SegmentList↔sourceURL, SegmentBase↔range, first-wins); `SegmentURL` (list-only); `ContentProtection`; descriptor group; `BaseURL` (self-closing ⇒ empty-string base, kept verbatim — documented); fallback → UnknownElement warn + skip. All pushes depth-guarded. Range attr dropped silently when parent is SegmentList (both sourceURL+range marked "known" at 1209) — tolerance, noted not filed. |
| `Driver::skip_start` | 1322-1336 | REAL | — | `let _ = attrs;` explicit ignore (attrs already noted by caller); self-closing → Ok, else push Unknown. |
| `Driver::push_or_close` | 1338-1352 | REAL | — | Attach-on-self-close against current stack top — correct parent for e.g. `<SegmentTemplate/>` inside AdaptationSet (verified against attach arms). |
| `Driver::apply_mpd_attrs` | 1354-1406 | REAL | — | `@type` strict (static/dynamic, fatal otherwise); 5 duration attrs fatal via `fatal_duration`; AST/publishTime stored raw (validated downstream by dash sched's ISO parser — cross-ref my sched.rs F-002: bad AST becomes `BadAvailabilityTime` at scheduling, consistent layering); unknown attrs warn. |
| `Driver::note_unknown_attrs` | 1408-1419 | REAL | — | Linear scan; known-list contract documented. |
| `Driver::attach_protection` | 1421-1444 | REAL | — | AdaptationSet/Representation only; Period-level ContentProtection (legal DASH) → UnsupportedHere warn — model gap is recorded in matrix notes ("CENC decrypt pending"), acceptable. |
| `Driver::attach_descriptor` | 1446-1457 | REAL | — | Same parents; `_` warns with the element name. |
| `Driver::attach_base_url` | 1459-1473 | REAL | — | Trims; Root/Period/AdaptationSet/Representation; others warn. Double-trim with on-close path is harmless. |
| `Driver::attach` | 1475-1570 | REAL | S3 (F-001) | Period flattens unconditionally; AdaptationSet/Representation silent-drop (F-001); SegmentTemplate/SegmentList/SegmentBase/SegmentTimeline first-wins + DuplicateSegmentDescription; BaseUrl delegates; Root/Unknown no-op. |
| `Driver::on_end` | 1572-1595 | REAL | — | Innermost name match; stray end → MismatchedEndTag warn; salvage loop hand-verified (§2). O(n) reverse scan per end tag — fine at MAX_MPD_DEPTH=64. |
| `Driver::on_text` | 1597-1614 | REAL | — | Whitespace-only early-out; BaseURL concatenates chunks; Unknown drops; else StrayText warn. |
| `Driver::finish` | 1616-1631 | REAL | S4 (F-003) | NotMpd / Truncated / NoPeriods-warn — honest EOF posture. |
| `attach_segment_list` | 1634-1646 | REAL | — | First-wins helper, warning on duplicate. |
| `attr` | 1654-1661 | REAL | — | First-match-wins documented (dup attrs illegal XML anyway). |
| `fatal_u64` | 1663-1669 | REAL | — | trim+parse→BadValue. |
| `fatal_duration` | 1671-1674 | REAL | — | Delegates to `parse_xs_duration` (part4); BadValue on None. |
| `warn_u64` | 1676-1694 | REAL | — | Junk → None + BadAttrValue warn. |
| `warn_bool` | 1696-1715 | REAL | — | "true"/"1"/"false"/"0" — exactly xs:boolean's lexical space; junk → None + warn. |

## 5. Test honesty
No tests live in my range (test module starts 2086 — parts 4/5). Coverage of this range from the module's suite (all green, orchrun log :4278-4295): `dirty_world_degrades_to_warnings` (salvage/misplacement posture — would fail if salvage attached to wrong parents), `truncated_refuses` (finish), `depth_exceeded_refuses` (push guard), `bounded_input_battery` + `fuzz_lite_mutations_never_panic` (panic-freedom of the tree builder). Note for part4/part5 auditors: none of the 16 test names obviously pins the F-001 silent-drop shape (`<Representation>` directly under `<Period>`) — likely an untested branch.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "MPD parse … landed" (cards 6.C1a) | docs/feature-matrix.toml:1454 (network-dash evidence) | REAL at library level, UNREACHABLE in product | Code exists; 16/16 tests green (orchrun :4278-4295); zero production callers (rg, header). |
| "warn-tolerance posture / dirty world degrades to warnings" | module docs + test name | SHALLOW exception | F-001: two attach arms drop misplaced frames with no warning — tolerance is inconsistent, not total. |
| Matrix: "6.C1c fuzz_demux_mpd cargo-fuzz target + in-crate deterministic mutation sweep" | docs/feature-matrix.toml:1453 | REAL | `fuzz.rs:39,59` calls `parse_mpd_bytes`; `fuzz_lite_mutations_never_panic` green (orchrun :4295). |

## 7. Dead / duplicated / unreachable code
- The parser is production-unreachable (see header) — compiled into the CLI binary via the streaming→playlists dependency chain, but no runtime path calls it.
- No dead code within my range; every helper has callers in this file.

## 8. Dependencies & flags
- std-only within this range (`std::ops::Range` used by scanners is part2). No feature gates, no unsafe, no unwrap/expect (workspace lints). `MpdWarning`/`MpdError` types from parts 1/2 of this file.

## 9. Open questions for the next agent
- part4 (W10-A01): does `apply_*_attrs` keep the fatal/warn split consistent with the helpers audited here (`fatal_u64` for timeline-level, `warn_u64` for representation-fact)? And does `parse_xs_duration` (1916) reject `P0Y0M0DT0H0M0S`-style junk or accept it?
- part5 (W10-A09): does any test pin the F-001 shape (Representation directly under Period → zero warnings)? If yes, it contradicts my reading and I want to know.
- Whoever audits `streaming/src/dash/resolve.rs`: the silent empty-ladder from F-001 surfaces to you as `BadLadder`/`NoRepresentation` — keep the traceback note.
