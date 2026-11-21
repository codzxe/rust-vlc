# AUDIT: crates/subtitles/src/vtt/settings.rs
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-21 |
| Lines / functions | 312 / 6 production fns (+1 struct) and 5 test fns |
| File verdict | UNREACHABLE (real, test-executed library code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `parse_settings`/`to_positioning` are `pub(super)`, called only from vtt/mod.rs (the cue parser); `subtitles::vtt::parse` is exercised only by crate tests (tests/vtt_acceptance.rs, tests/vtt_fuzz.rs) and fuzz/fuzz_targets/fuzz_sub_vtt.rs (lumen-fuzz workspace, not shipped). No app/engine crate depends on `subtitles` (repo-wide Cargo.toml grep: only fuzz/Cargo.toml:50). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | rg "vtt::settings" in baseline no-failfast log (5/5 tests ok, lines 5518-5522); Read of model/position.rs Percent::new (0..=100 bound confirmed at 41-46); sed verifications of all quotes; spec-conformance reasoning against the WebVTT cue-settings grammar (spec text not in sandbox — flagged UNVERIFIED where letter-of-spec matters). |

## 1. One-paragraph verdict
A small, disciplined settings parser: the documented subset table matches the code arm-for-arm (line/position/align with their suffixes, vertical+size as typed-unsupported, unknown names as typed notes), duplicates follow "first valid occurrence wins" exactly as the module doc states, and the 5 tests — all green in baseline (log:5518-5522) — assert concrete model mappings and damage kinds, including a 9-case bad-value sweep. Two spec-divergence defects survive: the WebVTT-valid keyword `auto` (legal value for both `line` and `position`, meaning "the default") is recorded as `BadSetting` — false damage on valid input — and the out-of-spec `align:middle` is silently accepted as `Center` with no damage note, contradicting this module's own "never silently applied" rule and absent from its doc table. Neither is reachable from the shipped binary; the whole VTT lane is library-plus-tests only.

## 2. What actually works (verified)
— Subset mapping: `subset_maps_to_model` pins `line:0%`→Percent, `line:-1,end`→LineNumber(-1)+Bottom, `position:75%,line-right`→Right — all match the doc table; passes in baseline (orchrun-test-workspace-nofailfast.log.txt:5521).
— Duplicate policy: `first_valid_occurrence_wins_and_order_overrides` pins first-valid-wins for `align` and encounter-order override (`position:10%,left align:right` → Right) — passes (log:5519). The code's `if !seen_x && parse_x(...) { seen_x = true }` marks `seen` ONLY on success — consistent with the doc's "first VALID occurrence" wording (invalid first + valid second → second applies).
— Bad-value typing: `bad_values_damage_not_panic` (9 inputs, each exactly one `BadSetting`) passes (log:5518) — `Percent::new` 0..=100 bound confirmed in model/position.rs:41-46, so `line:101%`/`position:150%` are genuinely rejected.
— `unsupported_and_unknown_are_typed_notes` (vertical/size → UnsupportedSetting, futurething → UnknownSetting) passes (log:5522); `positioning_merge_default_is_bottom_center` passes (log:5520).
— `parse_i32`/`parse_i64` length guards (11/20) are correct for the i32/i64 digit ranges; no panics anywhere in the file (no unwrap/index/division).

## 3. Findings

### F-001 [S3] Spec-valid `line:auto` / `position:auto` recorded as `BadSetting` — false damage on valid input
Where: crates/subtitles/src/vtt/settings.rs:107-117 and 148-151
Code:
```
    let placement = if let Some(digits) = main.strip_suffix('%') {
        match parse_i32(digits) {
            Some(v) => match Percent::new(v) {
```
```
    let Some(digits) = main.strip_suffix('%') else {
        out.push(DamageEvent::bad_setting("position", line_no));
        return false;
    };
```
What is wrong: in WebVTT, `line:auto` and `position:auto` are valid cue-setting values (`auto` is the spec's default, explicitly writable). Here `auto` has no `%` suffix and is not a number, so both land in the `BadSetting` damage path — a spec-conforming file is reported as damaged. The module's own discipline distinguishes "unsupported" (typed note) from "bad" (parse failure); `auto` is neither unsupported-in-principle nor bad — it is the default, and the honest classification would be an `UnsupportedSetting` note (or actual support: treat as absent).
Why it matters: the damage ledger is this crate's core honesty product; false positives on valid input train users to ignore it, and downstream "quality" metrics counting damage events will misreport valid files.
What correct looks like: match `"auto" => return true` (no placement, no damage) or `unsupported_setting` note, in both `parse_line` and `parse_position`.
Confidence: MEDIUM — Verification: read; WebVTT spec text is not in the sandbox (no network) — `auto`'s validity is elementary spec knowledge but the letter-of-spec check is UNVERIFIED in-environment.

### F-002 [S3] `align:middle` silently accepted as Center — out-of-spec value, no damage note, absent from the doc table
Where: crates/subtitles/src/vtt/settings.rs:183-186 (doc table line 16)
Code:
```
    match value {
        "start" | "left" => result.horizontal = Some(HorizontalAlign::Left),
        "center" | "middle" => result.horizontal = Some(HorizontalAlign::Center),
        "end" | "right" => result.horizontal = Some(HorizontalAlign::Right),
```
What is wrong: WebVTT's `align` grammar is `start|center|end|left|right` — `middle` is not a legal value (it is a legacy authoring error that strict players reject). The module doc's mapping table (line 16) lists only `align:start/center/end/left/right`. The code silently maps `middle`→Center with zero damage output, directly contradicting the module's header promise ("known-but-unsupported settings … recorded as typed damage notes, never silently applied" — here an UNKNOWN value is silently applied).
Why it matters: same-file contradiction of the file's own contract; a lenient alias should at least be documented and damage-noted (the crate does exactly that for `vertical`/`size`).
What correct looks like: either reject `middle` as `BadSetting` (spec-strict) or document it in the table and emit an `UnknownSetting`-class note (lenient-but-honest).
Confidence: HIGH for the doc/code contradiction (verified in-file); MEDIUM for the spec letter (no spec text in sandbox).

### F-003 [S4] Colon-less token produces an anonymous damage event
Where: crates/subtitles/src/vtt/settings.rs:58-60
Code:
```
        let Some((name, value)) = token.split_once(':') else {
            out.push(DamageEvent::bad_setting("", line_no));
            continue;
        };
```
What is wrong: a settings token without `:` (e.g. stray word after the timestamps) emits `BadSetting` with an EMPTY name — the damage event cannot say what was wrong, unlike every other call site which names the setting (`"line"`, `"position"`, `"align"`).
Why it matters: the damage ledger's diagnostic value; anonymous events are unfilterable noise.
What correct looks like: include the token (bounded) in the event, or a distinct `DamageKind::MissingColon`.
Confidence: HIGH — Verification: read.

### F-004 [S4] Unknown setting names are not deduplicated; known ones are
Where: crates/subtitles/src/vtt/settings.rs:90 vs 52-56
Code:
```
            other => out.push(DamageEvent::unknown_setting(other, line_no)),
```
What is wrong: `futurething:yes futurething:yes` emits two `UnknownSetting` notes (no `seen_` bookkeeping for unknown names), while known settings emit at most one note (first-occurrence rule). The dedup discipline is inconsistent within one loop.
Why it matters: damage-ledger noise inflation on repetitive malformed files; cosmetic but this crate sells ledger precision.
What correct looks like: a small seen-set for unknown names, or document why repeats are intentional.
Confidence: HIGH — Verification: read (no test covers duplicate unknown names — confirmed by the §5 coverage list).

### F-005 [S4] Coverage gaps: the "first valid" nuance, `auto`, and unbounded `LineNumber` are untested
Where: crates/subtitles/src/vtt/settings.rs:292-301 (the duplicate test)
Code:
```
        // Duplicate `align`: the first valid one (center) wins.
        let (o, d) = run("align:center align:left");
```
What is wrong: the test suite never exercises (a) an INVALID first occurrence followed by a VALID second (`line:bad line:5%` — the case where `seen_line` intentionally stays false; a refactor to "first occurrence wins regardless of validity" would pass every existing test), (b) `line:auto`/`position:auto` (F-001), (c) `align:middle` (F-002), (d) `line` values at the i64 extremes (the doc promises "negative counts from bottom" but no test pins what LineNumber(-9_223_372_036_854_775_808) does downstream).
Why it matters: the one behavior that distinguishes this parser from a naive one (validity-aware dedup) is exactly the untested behavior.
What correct looks like: add the four cases above with concrete expectations.
Confidence: HIGH — Verification: read of all 5 tests.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct SettingsOutcome` | 29-39 | REAL | — | plain Option-bag, merged by caller |
| `parse_settings` | 44-94 | REAL (flawed) | S3 | F-003 anonymous event, F-004 unknown dedup; duplicate policy matches doc ("first VALID occurrence") |
| `parse_line` | 97-135 | REAL (flawed) | S3 | F-001 (`auto` → BadSetting); suffix map start/center/end → Top/Middle/Bottom matches doc; returns false on bad suffix AFTER main parsed OK — damage once, no partial application (result.vertical set only on success path… note: `result.vertical` IS set before the later `result.line` assignment, but both only on the success return — verified no partial-write on the error paths) |
| `parse_position` | 138-173 | REAL (flawed) | S3 | F-001; requires `%`, Percent 0..=100 enforced (model/position.rs:41-46); five suffix values match the doc table |
| `parse_align` | 177-193 | REAL (flawed) | S3 | F-002 `middle` alias — undocumented, no note |
| `parse_i32` | 195-200 | REAL | — | length guard 11 = len("-2147483648") ✓ |
| `parse_i64` | 202-207 | REAL | — | length guard 20 ✓ |
| `to_positioning` | 210-221 | REAL | — | merges onto default; default's Bottom/Center pinned by test (baseline log:5520) |
| tests (5) | 236-311 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| subset_maps_to_model | YES | concrete model-type assertions incl. suffix maps |
| unsupported_and_unknown_are_typed_notes | YES | DamageKind variants pinned |
| bad_values_damage_not_panic | YES | 9 inputs × exactly-one-BadSetting |
| first_valid_occurrence_wins_and_order_overrides | PARTIALLY | pins valid-duplicate and override order; does NOT pin the invalid-then-valid case (F-005a) — a "first occurrence wins" refactor would pass |
| positioning_merge_default_is_bottom_center | YES | default merge pinned |
| Untested | — | `auto` values (F-001), `align:middle` (F-002), colon-less token name (F-003), duplicate unknown names (F-004), i64-extreme `line` numbers |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the supported subset maps onto the model's Positioning; known-but-unsupported settings (vertical, size) are recorded as typed damage notes, never silently applied" | settings.rs:1-5 | REAL with one hole | vertical/size → UnsupportedSetting verified + tested; but `align:middle` IS silently applied (F-002) — the "never silently applied" promise is broken by its own align alias |
| Mapping table rows (line/position/align + suffixes) | settings.rs:9-17 | REAL | every row matches the corresponding match arm (verified arm-by-arm) |
| "a duplicate setting name is ignored after its first valid occurrence" | settings.rs:19-20 | REAL (code matches doc) | seen_* flags set only on parse success (63-66 etc.); UNVERIFIED against the spec letter (no spec text in sandbox) — if the spec ignores duplicates regardless of validity, the code is MORE lenient than spec, which is safe-direction drift |
| "Unknown setting names … are recorded too and skipped, per the spec's forward-compatibility rule" | settings.rs:4-5 | REAL | unknown_setting note + skip (90) |
| feature-matrix `subtitle-webvtt` row claims (parsed/typed damage) | docs/feature-matrix.toml (row outside my range — 18-A06's) | UNREACHABLE | library-only, same crate-level verdict |

## 7. Dead / duplicated / unreachable code
- No dead code found; every fn is called (parse_settings/to_positioning from vtt/mod.rs; helpers from parse_*).
- File unreachable from shipped entry points (crate-level).

## 8. Dependencies & flags
- Depends on model/position types + super::DamageEvent only — clean layering (model ← format), matches the crate's dependency stance. No unsafe, no allocs beyond the caller's Vec, no feature flags.

## 9. Open questions for the next agent
- A07 (vtt/mod.rs): does the cue parser truncate/limit the settings string before calling parse_settings (a 10 MB single-token settings line would be split/parsed per token — O(n) but the DamageEvent count could be large; is the vtt damage ledger capped like pgs's MAX_DAMAGE_EVENTS=64?)
- A09 (vtt/damage.rs): confirm `BadSetting`'s shape carries the name string — F-003's empty-name event would be visible there.
- Whoever owns model/position.rs (A04): what does the renderer do with LineNumber beyond sane ranges (F-005d) — clamp or damage?
- W19 (docs): the settings subset table should document `auto` (F-001) and the `middle` alias (F-002) once the behavior is decided.
