# AUDIT: crates/playlists/src/mpd.rs#part2 (lines 571-1140)
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2026-11-21 |
| Lines / functions | 570 (571-1140) / 24 (MpdWarning tail, `parse_mpd`, `parse_mpd_bytes`, `Token`, `Scanner` + 12 methods, `is_namespace_decl`, `Frame`, `Driver` + 4 methods incl. `on_start` head). Part boundaries: part1 (W10-A05) = 1-570 (model structs, `MpdError`); **part2 (MINE) = 571-1140** (warning-enum tail, parse entry points, the whole XML scanner, tree-build driver head); part3 (W10-A02) = 1141-1710 (rest of `on_start` — the `SegmentTemplate` arm's struct-init tail at 1137-1140 is the cut point — `on_end`, `finish`, attr plumbing); part4 (W10-A01) = 1711-2280; part5 (W10-A04) = 2281-2855 (tests). `on_start` straddles 1140: I audited 1046-1140 (root/stray handling + Period/AdaptationSet/Representation arms) and leave the segment-arm continuation to part3 — noted in §9. |
| File verdict | UNREACHABLE (real, well-engineered, fuzz-tested parser — but no production code path ever calls the parse entry points) |
| Reachable from shipped entry point? | NO for this part's entry points. `rg -n "parse_mpd" crates apps` → callers are: mpd.rs's own `#[cfg(test)]` tests, `streaming/src/dash/{sched,abr,resolve}.rs` **all inside `#[cfg(test)] mod tests`** (verified: sched.rs:474-477, abr.rs:327-330, resolve.rs:741-744), and `streaming/src/dash/fuzz.rs:39` (fuzz target, not shipped). No app source references it. The `playlists` crate IS compiled into the shipped `lumen` binary only transitively (apps/cli → streaming → playlists), but the DASH lane (parse + `streaming::dash` session) has no production driver: `rg -n "dash" apps/cli/src` → none. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "parse_mpd\|playlists::" crates apps` → production-caller census (above); `rg -n "cfg\(test\)\|mod tests" crates/streaming/src/dash/*.rs` → all parse_mpd_bytes uses test-gated; `sed -n '1572,1655p'` → on_end/on_text/finish context for straddle; `sed -n` on every quoted range; `rg -n "mpd::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → 16/16 ok, suite 46 passed 0 failed (line ~4296) |

## 1. One-paragraph verdict
Part2 is the mechanical heart of the MPD parser — the entry-point size/line guards, a hand-rolled std-only XML pull scanner (comments, CDATA, PIs, naive DOCTYPE skip, dual quotes, entity-decode-once, namespace-blind local names), and the tree-build driver's head — and it is genuinely careful work: every consume path provably makes progress (no infinite loops), every slice boundary is ASCII-anchored (no panic on multi-byte UTF-8, and the input is `&str` so it is valid by construction), the depth guard caps the element stack at 64 for known AND unknown frames (the entity-bomb bound is structural: DOCTYPE entities are never registered), and truncation is a typed refusal that outranks salvage. The 16 in-crate tests plus the mutation sweep ran green in the no-fail-fast baseline run. The two real problems: the whole lane is a shipped-binary orphan — not one production call site invokes `parse_mpd`, so the feature matrix's `network-dash: partial` is code-landing truth, not user capability — and the scanner's damage-resync path can quietly convert a self-closing tag into an unclosed one, turning recoverable dirty input into a fatal Truncated refusal, plus a mislabeled warning and an error-precedence nit.

## 2. What actually works (verified)
— All 16 `mpd::tests` executed green: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4278-4295` (e.g. `truncated_refuses … ok`, `depth_exceeded_refuses … ok`, `bounded_input_battery … ok`, `fuzz_lite_mutations_never_panic … ok`), playlists suite `46 passed; 0 failed`.
— Truncation posture verified by reading + test: cut at 5 positions of the real ffmpeg fixture all yield `MpdError::Truncated` (test at 2514-2529, asserts code `playlists.mpd.truncated`).
— Entity-bomb bound verified structurally: DOCTYPE skipped at 826-831 via naive `>`-skip, custom entities never registered (grep: no entity table exists in the scanner); `bounded_input_battery` (2547-2560) parses a billion-laughs shape with only `BadEntity` warnings.
— Scanner progress/termination traced by hand through every loop (next_token, scan_start_tag attr loop, resync_attr, name loops): each iteration consumes ≥1 byte or breaks — no spin path found (see F-003 for the price paid for that guarantee).

## 3. Findings

### F-001 [S3] Parse entry points are production orphans — the "DASH playback: partial" matrix row is code-landing, not shipped capability
Where: crates/playlists/src/mpd.rs:652-661 (and 685-688)
Code:
```rust
pub fn parse_mpd(text: &str) -> Result<MpdParsed, MpdError> {
    if text.len() > MAX_MPD_BYTES {
        return Err(MpdError::TooLarge {
```
What is wrong: `parse_mpd`/`parse_mpd_bytes` have zero non-test, non-fuzz callers. The only in-repo consumers are `#[cfg(test)] mod tests` blocks in `streaming/src/dash/{sched,abr,resolve}.rs` and the `dash/fuzz.rs` target. Nothing in `apps/cli` or `apps/desktop-tauri` routes an `.mpd` URL or body to it; `streaming::dash` (the would-be consumer) is itself referenced by no app.
Why it matters: docs/feature-matrix.toml `network-dash` (line ~1444) claims status `partial` on windows/linux/macos with evidence "MPD parse, template/list resolution, … landed" — every word about code is true, but a user of the shipped `lumen` binary cannot get any MPD opened: the lane dead-ends before the first fetch. The module doc's own promise (line 36-38) that "the DASH session cards (C1b/C2) consume this module" is future tense — the consuming session exists only as unreachable library code.
What correct looks like: either wire a production path (URL open → sniff MPD → `parse_mpd_bytes` → dash session) or downgrade the matrix row's user-facing status and add an explicit "not reachable from shipped hosts" note.
Confidence: HIGH — Verification: ran rg caller census (header); read all caller contexts.

### F-002 [S3] Attribute-damage resync eats the self-closing `/`, converting recoverable dirty input into a fatal Truncated refusal
Where: crates/playlists/src/mpd.rs:954-964
Code:
```rust
    /// Attribute-damage resync: skip to the next whitespace or `>`
    /// (a `/` is consumed — only `/` immediately before `>` is a
    /// terminator; stopping at a lone `/` would spin).
    fn resync_attr(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() || c == '>' {
                break;
            }
            self.advance_char();
        }
    }
```
What is wrong: for `<SegmentURL media=chunk1/>` (unquoted value — damage), the attr loop hits the no-quote branch (921-925), resync consumes `chunk1/` including the `/`, and the tag then terminates at `>` with `self_closing = false`. The element is pushed open; if no matching end tag follows, `finish()` (part3, 1618-1626) returns `MpdError::Truncated` — the entire manifest is refused over one damaged attribute, in direct contradiction of the module's dirty-world posture (module docs 70-74: unknown/dirty is warning, not failure).
Why it matters: a broken server emitting a single unquoted attribute in a self-closing tag kills the whole DASH open with a misleading "truncated" diagnosis; the documented tolerance contract is violated at exactly the input class it exists for.
What correct looks like: `resync_attr` should stop at `/` when the next char is `>` (peek two), or the scan loop should re-check `rest_starts_with("/>")` after resync before treating `>` as a plain terminator; the spin concern only applies to a lone `/` NOT followed by `>`.
Confidence: MEDIUM — Verification: read + traced the exact byte walk (`<foo bar=junk/>` → resync eats `junk/` → `>` terminates non-self-closing); no in-tree test covers unquoted-attr + self-closing combination (test list checked: dirty_world test at 2599 covers unknown elements/attrs, not this).

### F-003 [S4] `MismatchedEndTag` warning message lies for the trailing-junk case
Where: crates/playlists/src/mpd.rs:864-870
Code:
```rust
            match self.src[self.pos..].find('>') {
                Some(i) => {
                    self.take(self.pos + i + 1);
                    self.warnings
                        .push(MpdWarning::MismatchedEndTag { line: start_line });
                }
```
What is wrong: this site pushes `MismatchedEndTag` when an end tag has junk after its name (`</Period x>`), but the variant's message (599-602) says "end tag with no matching open element (ignored)" — a claim that is false here: the name usually matches fine and `on_end` (part3, 1572-1595) will close it normally.
Why it matters: diagnostics that explain the wrong condition send a debugger hunting for a structural mismatch that does not exist; the same variant is correctly used by `on_end` for genuine strays, so users cannot distinguish the two cases.
What correct looks like: a distinct `BadEndTag`/`TrailingJunkInEndTag` variant, or an honest shared message ("end tag syntax damage / unmatched end tag").
Confidence: HIGH — Verification: read both sites (864-870, 1586-1589) and the variant doc (599-604).

### F-004 [S4] A start tag truncated mid-attribute can report `BadValue` instead of `Truncated`
Where: crates/playlists/src/mpd.rs:943-951
Code:
```rust
        if !terminated {
            self.truncated_at_eof();
        }
        Token::Start {
            line: start_line,
            name: local_name(&raw_name),
            attrs,
            self_closing,
        }
```
What is wrong: a truncated start tag is still emitted as a `Token::Start` with its partial attributes; `parse_mpd` feeds it to the driver (664-665) BEFORE checking `scanner.truncated` (667-669). If the partial value is junk (a cut inside `type="stat…`), the driver's fatal `BadValue` fires first, so the manifest is diagnosed with a value error when the real defect is truncation. A cut one byte earlier or later yields `Truncated` — the reported cause depends on where the byte stream ends.
Why it matters: misdiagnosed root cause for exactly the flaky-network class of failure this parser is built for; not a correctness bug (both are typed refusals).
What correct looks like: in `parse_mpd`, check `scanner.truncated` before feeding the final token, or have `scan_start_tag` suppress the token when it marks truncation.
Confidence: HIGH — Verification: read both code paths; traced `<MPD type="stat` (cut) → `BadValue` vs `<MPD type=` (cut) → `Truncated`.

## 4. Per-function table (part2 only)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MpdWarning` enum tail (8 variants) | 571-642 | REAL | S4 | Variants carry structural names only (MP §12 honored — verified each variant's fields). |
| `parse_mpd` | 652-679 | REAL (unreachable) | S3 | Size guard (8 MiB), line-cap guard, feed loop, truncation outranks finish, warnings merge documented order. F-001/F-004. |
| `parse_mpd_bytes` | 685-688 | REAL (unreachable) | S3 | UTF-8 gate typed `InvalidUtf8`; delegates. |
| `enum Token` | 694-709 | REAL | — | Start/End/Text; local names; CDATA raw (documented). |
| `struct Scanner` | 716-722 | REAL | — | pos/line/truncated/warnings; no allocation beyond warnings. |
| `Scanner::new` | 725-733 | REAL | — | line 1 start. |
| `Scanner::take` | 737-743 | REAL | — | Char-boundary contract honored by all callers (audited: every end is ASCII-anchored); newline count per consumed slice. |
| `Scanner::peek_char` | 745-747 | REAL | — | O(1) subslice. |
| `Scanner::advance_char` | 749-752 | REAL | — | len_utf8-correct; no-op at EOF (pos stays, callers guard). |
| `Scanner::skip_ws` | 754-758 | REAL | — | char::is_whitespace. |
| `Scanner::rest_starts_with` | 760-762 | REAL | — | Prefix checks only. |
| `Scanner::truncated_at_eof` | 766-771 | REAL | — | First-wins line record; consumes rest. |
| `Scanner::skip_after` | 773-780 | REAL | — | Comment/PI/DOCTYPE skip; miss → truncation. |
| `Scanner::next_token` | 782-850 | REAL | — | The lexer dispatch; every branch consumes or terminates (progress proof); BadTag literal-`<` fallback; CDATA `]]>` first-occurrence semantics correct for `a]]]>`. |
| `Scanner::scan_end_tag` | 852-878 | REAL | S4 | Junk-after-name → F-003 mislabel; missing `>` at EOF → truncation. |
| `Scanner::scan_start_tag` | 880-952 | REAL | S3 | Attr loop: progress in all paths; unquoted-value damage → resync → F-002; unterminated value at EOF degrades to truncation (correct); duplicate attrs both recorded, first-wins downstream (XML-spec consistent). |
| `Scanner::resync_attr` | 957-964 | REAL | S3 | F-002. |
| `is_namespace_decl` | 969-971 | REAL | — | xmlns/xmlns:* dropped silently — documented. |
| `enum Frame` | 977-988 | REAL | — | Builder-per-frame stack. |
| `struct Driver` | 990-996 | REAL | — | stack bounded by push guard. |
| `Driver::new` | 999-1020 | REAL | — | All Mpd fields initialized explicitly (compile-enforced). |
| `Driver::push` | 1023-1031 | REAL | — | Depth guard fires for Unknown frames too — bomb bound; DepthExceeded{depth: len+1} honest. |
| `Driver::feed` | 1033-1044 | REAL | — | Pure dispatch. |
| `Driver::on_start` head | 1046-1140 | REAL | — | Root gate `NotMpd`; self-closing root ok (NoPeriods later); post-root strays → warning + Unknown swallow; Period/AdaptationSet/Representation arms construct fully-initialized builders (compile-enforced). SegmentTemplate arm begins 1137 — tail belongs to part3. |

## 5. Test honesty (tests live in part5, lines 2088-2855; they execute THIS part's code)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| truncated_refuses (2514) | YES | 5 cut points on the real fixture, all must be `Truncated` with exact code; cuts cross text/comment/tag boundaries. |
| depth_exceeded_refuses (2531) | YES | 70-deep nesting; asserts depth > MAX_MPD_DEPTH — kills the push guard removal. |
| bounded_input_battery (2547) | YES | Oversize/line-cap/UTF-8 each pinned to exact variant; bomb must complete with BadEntity. |
| dirty_world_degrades_to_warnings (2599) | YES (partial) | Covers unknown elements/attrs, missing intermediate end tag, junk facts, duplicate segment descriptions — does NOT cover F-002's unquoted-attr+self-closing shape. |
| fuzz_lite_mutations_never_panic (2803) | YES (weak) | Panic-hunt only; would catch slice-boundary regressions, not semantic drift. |
| namespace_tolerant_local_names (2718) / base_url_and_entity_decoding (2736) / error_code_table (2765) | YES | Pin prefix-stripping, entity decode-once, and every error code string. |
All ran green: orchrun-test-workspace-nofailfast.log.txt:4278-4295.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "DASH playback … status partial (win/linux/macos); MPD parse … landed" | docs/feature-matrix.toml `network-dash` (~1444) | UNREACHABLE (code real, shipped binary cannot reach it) | F-001 caller census; `rg -n "dash" apps/cli/src` → none. |
| "EOF with any element still open, or EOF inside a tag/comment/CDATA/PI, is a typed refusal" | mpd.rs module docs 89-93 | REAL | Traced scanner paths + truncated_refuses test green. |
| "Entity-expansion bombs are structurally impossible" | module docs 63-68 | REAL | DOCTYPE naive-skip (826-831) + no entity table anywhere in scanner (rg); bounded_input_battery green. |
| "Nesting deeper than MAX_MPD_DEPTH (64) → DepthExceeded, element stack capped" | module docs 61-62 | REAL | push guard 1023-1031 applies to all frames; depth test green. |
| "unknown elements and attributes are warnings, never failures" | module docs 72-74 | FALSE-in-one-case | F-002: unquoted attr in a self-closing tag escalates to fatal Truncated. |
| "a mismatched end tag salvages what closed cleanly" | module docs 74-75 | REAL | on_end salvage loop (part3 1591-1597) + dirty_world test. |

## 7. Dead / duplicated / unreachable code
- The entire part is unreachable from shipped binaries (F-001) — but internally every item is used by `parse_mpd`/tests.
- Scanner is a deliberate sibling copy of the xspf scanner (module docs 49-53 record the duplication decision and the follow-up) — accepted duplication, flagged as recorded debt.

## 8. Dependencies & flags
std + thiserror only (crate-level, per playlists/Cargo.toml audited in wave-1: no external deps); no feature gates on this module; `MAX_LINE_BYTES` borrowed from `crate::m3u` (in-crate). No cfg-gated production code.

## 9. Open questions for the next agent
- part3 (W10-A02): `on_start` 1141+ continues the SegmentTemplate arm whose struct-init begins at 1137-1140 (my cut); also verify `apply_mpd_attrs` treats `@type` junk as fatal (my F-004 trace assumed it — confirmed fatal per module docs 78-83, but the code is yours).
- part3: `finish()`'s `Truncated { line: frame line }` reports the OPEN element's start line, not EOF — intentional? (diagnostic nuance, not a defect).
- Matrix-rows auditor for wave 10/11: `network-dash` row's "partial" needs a user-reachability verdict, not a code-landing verdict (F-001).
