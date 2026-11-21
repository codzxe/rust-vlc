# AUDIT: crates/localization/src/message.rs#part2 (lines 536–1071)
| Field | Value |
|---|---|
| Auditor | W11-A07 |
| Date | 2026-11-21 |
| Lines / functions | 536 / 19 (11 parser items + 8 tests) |
| File verdict | UNREACHABLE (code itself REAL — all 8 in-range tests pass in baseline) |
| Reachable from shipped entry point? | NO — chain: `Parser::parse_message` ← `Message::parse` (message.rs:276) ← `Catalog` load (catalog.rs:129) ← localization's own tests only. No crate/app depends on `localization` (verified: `grep -rn "localization" apps/*/Cargo.toml crates/*/Cargo.toml tests/*/Cargo.toml tools/*/Cargo.toml` → only self-mentions/comments, exit 1 for real deps; `grep -rn "use localization|localization::" apps/ crates/ tests/ tools/` → zero hits outside the crate). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 3 |
| Commands run | `rg`/grep: callers of `Message::parse` (catalog.rs:129 only), `localization` in all Cargo.tomls (no dependents), `l10n` in tools/xtask/src (none — doc 07 §6.4's `cargo xtask l10n-audit` does not exist), `audit_catalogs` callers outside crate (none), `select|plural|percent` in locales/en-US.toml (comments only — no catalog entry uses these forms). Baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2706-2714 (all 8 message::tests `ok`). sed -n verified every quote. |

Chunk boundary: part1 (A06) = lines 1–535 (data model + render); part2 (this
audit) = lines 536–1071, starting at the `ArmGrammar` doc comment — the whole
recursive-descent parser (`Parser` impl) plus the entire `#[cfg(test)] mod
tests`. Boundary chosen at the structural seam (close of `render_placeholder`
at 534/535 vs `/// Grammar of select/plural arm names` at 536).

## 1. One-paragraph verdict
The parser half of the message-format v1 engine is genuinely written, careful
(byte-exact error offsets traced and matching every test expectation), and
fully green in the baseline no-fail-fast run — but it is dead code by design
accident: no catalog entry in the repo uses `select`/`plural`/`percent` (they
appear only in comments of locales/en-US.toml), no crate depends on
`localization`, and the CI runner doc 07 §6.4 promises for it
(`cargo xtask l10n-audit`) does not exist. This is the wave's "declared but
never consumed" answer for the localization lane: a complete, tested
message-format engine wired to nothing. Beyond reachability, the parser has
three real defects: whitespace handling that makes newlines fatal in
structural positions with misleading error variants, unbounded recursion with
no depth cap, and redundant re-validation that silently drops data.

## 2. What actually works (verified)
— All 8 tests in this range pass: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2706-2714` — `message::tests::{number_formatting_helpers, parses_all_v1_forms, parses_plain_text_and_escapes, placeholder_kind_tags, rejects_malformed_messages, rendering_never_panics_on_bad_args, renders_select_and_plural, renders_simple_and_typed_placeholders}` all `ok` (45-test lib run, 44 passed / 1 failed — the failure is in fallback.rs, not this file).
— Error-offset arithmetic hand-traced against `rejects_malformed_messages` expectations (14 cases): e.g. `"{n, plural, one {x} two {y}}"` → `MissingOtherArm { offset: 4 }` and `"{n, select, a {x} a {y} other {z}}"` → `DuplicateArm { offset: 17 }` — both reproduced by manual pos accounting through `parse_arms`; all 14 match.
— `pos` invariant (`pos <= src.len()`) holds on every advance path (verified by reading each `pos +=` site: all guarded by a successful `peek`/`starts_with`); no slice panic path in `peek`/`starts_with`.

## 3. Findings

### F-001 [S2] Parser (and the whole `localization` crate) is unreachable: declared mechanism, zero consumers
Where: crates/localization/src/message.rs:536-862 (whole part2), cross: crates/localization/Cargo.toml, docs/07-config-driven-system.md §6.4
Code:
```rust
    fn parse_arms(
        &mut self,
        type_offset: usize,
        grammar: ArmGrammar,
    ) -> Result<PlaceholderKind, MessageError> {
```
What is wrong: The only caller chain is `Parser::parse_message` ← `Message::parse` (message.rs:276) ← `Catalog` load (catalog.rs:129) ← localization's own test suites (`tests/audit_proof_matrix.rs`, `tests/seed_integrity.rs`). No member of the workspace depends on the `localization` crate (every `Cargo.toml` grep hit is a comment or self-reference), and the one tool that reads `locales/en-US.toml` (xtask `config_validator.rs:483–497`) re-parses it with its own `toml_lite`, bypassing this engine. Doc 07 §6.4 claims the audit rules are "Run by `cargo xtask l10n-audit`" — that subcommand does not exist (`grep -rn "l10n" tools/xtask/src/` → exit 1, no matches); CI's own skip ledger admits it (ci.yml:468: "the l10n-audit xtask subcommand itself does not exist") and substitutes `cargo test -p localization` (ci.yml:438). No catalog entry uses select/plural/percent at all (`grep` in locales/en-US.toml hits comments only, lines 16/40–42/46).
Why it matters: This is the config-system wave's core hostile question answered concretely — an entire declared subsystem (message-format engine + catalog audit rules + fallback chain) is compiled by `cargo build --workspace` but linked into nothing a user can run. Doc 07 §6.4's CI-gate claim is unbacked; catalog integrity (rules 1–5) is enforced only by in-crate tests that the repo could delete without any shipped behavior changing.
What correct looks like: Either wire the crate in (xtask `l10n-audit` subcommand calling `audit_catalogs`, and the UI/settings layer consuming `Message::render`), or cut it and doc 07 §6.2–6.5 down to the UI-side `ui/src/i18n` reality that actually ships.
Confidence: HIGH — Verification: read (greps listed in header; baseline INDEX has no l10n gate stage).

### F-002 [S3] `skip_ws` accepts only space/tab: a newline in any structural position of select/plural is a hard parse error
Where: crates/localization/src/message.rs:573-577 (root cause), symptoms at 655–673, 746–760, 822–832
Code:
```rust
    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t')) {
            self.pos += 1;
        }
    }
```
What is wrong: ICU MessageFormat permits arbitrary whitespace (including newlines) between tokens. Here, `{n, select,\n  a {x} other {y}}` — a natural multi-line TOML basic string (`"""…"""`) or any `\n` escape — dies with `UnknownType { found: "" }` (arm-name position, parse_arms:782–787) or `InvalidName` (before the arm's `{`, lines 822–832). Same failure between placeholder name and type keyword (`ExpectedCommaOrClose` via parse_placeholder:656–673).
Why it matters: Catalogs are hand-edited translation files; long select/plural messages are exactly the ones translators wrap across lines. The failure mode is a hard catalog-load error whose message ("unknown placeholder type \"\"") points the author at the wrong problem. Blast radius today is zero only because nothing consumes the crate (F-001); the moment it is wired, this bites.
What correct looks like: `skip_ws` skipping `\n`/`\r` as well (they are not significant tokens in this grammar — `parse_message` already treats them as plain text inside bodies).
Confidence: HIGH — Verification: read (traced both failure paths line by line; not executed — no cargo per rules).

### F-003 [S3] Diagnostic variants lie about the failure: arm-name errors report `UnknownType`, expected-`{` errors report `InvalidName`
Where: crates/localization/src/message.rs:782-787 and 822-832
Code:
```rust
            let Some(arm_name) = self.parse_ident() else {
                return Err(MessageError::UnknownType {
                    offset: arm_offset,
                    found: String::new(),
                });
            };
```
and
```rust
                Some(c) => {
                    return Err(MessageError::InvalidName {
                        offset: self.pos,
                        found: c,
                    });
                }
```
What is wrong: `UnknownType`'s contract (part1, lines 173–176) is "The placeholder type keyword is not one of number/select/plural", rendered as `unknown placeholder type "" (v1: number/select/plural)`. It is emitted here when an *arm name* is missing/malformed. Likewise `InvalidName` ("invalid placeholder name character") is emitted when the char after a valid arm name is not `{`. Both render as messages about a different syntactic element than the one that failed.
Why it matters: `MessageError::offset` exists so "callers can map them to file positions" (part1:152–154) — catalog.rs surfaces these to authors via `catalog_message_error`. A translator told `unknown placeholder type ""` for a stray newline has no path to the actual defect. The typed-error design is undermined by variant reuse.
What correct looks like: dedicated `MissingArmName` / `ExpectedArmBody` variants (or at minimum `MissingName`/`ExpectedCommaOrClose`-style naming that matches what broke).
Confidence: HIGH — Verification: read.

### F-004 [S3] Unbounded recursion: no nesting-depth cap on select/plural
Where: crates/localization/src/message.rs:833-837
Code:
```rust
            // Sub-message: recursion; `#` only inside plural arms.
            let sub = self.parse_message(match grammar {
                ArmGrammar::Option => ParseContext::SelectArm,
                ArmGrammar::Category => ParseContext::PluralArm,
            })?;
```
What is wrong: `parse_message` → `parse_placeholder` → `parse_arms` → `parse_message` recurses once per nesting level with no depth limit. A catalog value (attacker-/author-controlled string, size bounded only by the TOML file) like `{a, select, other {` repeated N times drives N stack frames; a few tens of thousands of levels abort the process with stack exhaustion — not a `Result`, not a `MessageError`, uncatchable.
Why it matters: The module's stated invariant is "Rendering never panics" (part1:20–22) and parse-time is fully `Result`-typed; stack overflow breaks that guarantee at the library boundary. Today unreachable (F-001); the day this loads third-party locale packs (doc 07 §6.3: catalogs are per-language data files), it is a malformed-input crash (would be S1/S2 if wired).
What correct looks like: a `depth: u8` parameter through `parse_message`/`parse_arms` with a small cap (ICU implementations typically cap at 2–8; even 64 is generous) returning `MessageError::Unterminated`-style `NestingTooDeep`.
Confidence: MEDIUM — Verification: read (recursion path traced; overflow not executed — no cargo per rules).

### F-005 [S4] `PluralCategory` parsed twice; the second parse can silently drop an arm
Where: crates/localization/src/message.rs:843-850
Code:
```rust
            match grammar {
                ArmGrammar::Option => select_arms.push((arm_name, sub)),
                ArmGrammar::Category => {
                    // Checked above; the parse is total over valid names.
                    if let Some(category) = PluralCategory::parse(&arm_name) {
                        plural_arms.push((category, sub));
                    }
                }
            }
```
What is wrong: The category was already parsed and validated at lines 803–808; re-parsing and then *silently dropping the arm* on `None` is a second source of truth for the same fact. If `PluralCategory::parse`'s accepted set ever drifts (e.g. a new CLDR category added but the match here edited inconsistently), arms vanish from messages with no error.
Why it matters: Drift bait; the `if let Some` makes a future inconsistency a silent data loss instead of a compile error.
What correct looks like: bind `let category = PluralCategory::parse(&arm_name)` once (line 803), reuse it here.
Confidence: HIGH — Verification: read.

### F-006 [S4] O(n²) duplicate-arm detection, unbounded for select options
Where: crates/localization/src/message.rs:789-819
Code:
```rust
                    if select_arms
                        .iter()
                        .any(|(existing, _)| *existing == arm_name)
                    {
```
What is wrong: Linear scan per arm. Plural is bounded by 6 CLDR categories; select options are unbounded author data.
Why it matters: Negligible at realistic catalog sizes (dozens of arms); pure hygiene. Listed because the checklist demands it, not because it will ever fire.
What correct looks like: a `BTreeSet`/`HashSet` of seen names.
Confidence: HIGH — Verification: read.

### F-007 [S4] Test gaps: documented ICU arm limitations have zero coverage
Where: crates/localization/src/message.rs:865-1070 (tests module)
Code:
```rust
        // Doc 07 §6.2 example, adjusted: the crate's v1 subset requires the
        // mandatory `other` arm (`MissingOtherArm`) while the doc's example
        // omits it — reconciliation is flagged for the Lead
        // [VERIFY — doc 07 §6.2 vs. crate rule].
```
What is wrong: No test exercises (a) `{{`/`}}` inside an arm body (hard parse error per design — module doc part1:16–20 — but unverified by any test), (b) newline-in-arm failure (F-002), (c) `#` outside a plural arm rendering as literal text, (d) multi-byte text before an error site (byte-offset claim, part1:152–154, never tested with non-ASCII). The `[VERIFY]` comment itself documents a doc/code contradiction left unresolved since the crate landed (see §6).
Why it matters: The untested paths are precisely the documented limitations — the places a future refactor is most likely to break silently.
What correct looks like: One table-driven test row each: `"{a, select, other {x {{ y}}"` → error; `"#\n{n, plural, other {#}}"` top-level `#` literal; a case with `é` before a bad token asserting the byte offset.
Confidence: HIGH — Verification: read (grep of tests module for `{{`, newline, `é` cases — none present).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum ArmGrammar` | 538–543 | REAL | — | Trivial 2-variant marker enum; consumed by `parse_arms` only. |
| `enum ParseContext` | 550–557 | REAL | — | Top/SelectArm/PluralArm; drives escape + `#` semantics; documented accurately at 545–548. |
| `struct Parser<'a>` | 559–562 | REAL | — | src + byte pos; invariant pos ≤ len holds (see §2). |
| `Parser::peek` | 565–567 | REAL | — | No panic path (slicing at pos ≤ len). |
| `Parser::starts_with` | 569–571 | REAL | — | Delegates to str::starts_with; safe. |
| `Parser::skip_ws` | 573–577 | REAL (defect) | S3 | Space/tab only — F-002 root cause. |
| `Parser::parse_ident` | 581–595 | REAL | — | ASCII-ident grammar exactly as documented; `len_utf8` is always 1 here (ASCII-checked first); position preserved on None. |
| `Parser::parse_message` | 601–636 | REAL | — | Top-level `{{`/`}}` escape pushes one brace and skips 2 (correct); `}` break leaves pos for `Message::parse`'s UnmatchedClose; `#` special only in PluralArm. Recursive target of F-004. |
| `Parser::parse_placeholder` | 638–695 | REAL | — | Error mapping at name position (None/`}`/`,`/other) complete; dispatch to the three type parsers correct. |
| `Parser::parse_number_style` | 698–737 | REAL | — | Only `percent`; both missing-style and wrong-style paths return typed errors; offsets traced vs tests (`{n, number}` → 10, `{n, number, bytes}` → 12) ✓. |
| `Parser::parse_arms` | 739–862 | REAL (defects) | S3×2, S4×2 | Duplicate detection (789–819), has_other enforcement (853–857), sub-message close consumption (838–842) all correct; F-002/F-003/F-005/F-006 live here. Post-sub `Some(_) => pos += 1` is sound: parse_message can only stop before `}` or EOF (comment at 840 documents the invariant). |
| `tests::parses_plain_text_and_escapes` | 870–877 | REAL | — | Would fail if escapes broke (placeholder_set().len() would be >0 on `{{` treated as `{`). |
| `tests::parses_all_v1_forms` | 879–916 | REAL | — | Covers all four forms + nested placeholder in select arm; contains the [VERIFY] doc-contradiction comment (F-007). |
| `tests::rejects_malformed_messages` | 918–999 | REAL | — | 14 typed-error cases with exact offsets — the strongest test in the file; hand-traced all offsets (§2). |
| `tests::renders_simple_and_typed_placeholders` | 1001–1012 | REAL | — | Exercises part1 render via parse output; asserts text + zero diagnostics. |
| `tests::renders_select_and_plural` | 1014–1032 | REAL | — | `one`/`other` selection en-US; unmatched select value → `other` with no diagnostic (ICU semantics, asserted). |
| `tests::rendering_never_panics_on_bad_args` | 1034–1052 | REAL | — | Missing arg → literal + 2 diagnostics; type mismatch → literal + 1; would fail if degrade-on-error regressed. |
| `tests::number_formatting_helpers` | 1054–1063 | REAL | — | Part1 helpers (format_float/format_percent); runs in baseline ✓. |
| `tests::placeholder_kind_tags` | 1065–1069 | REAL | — | Tag strings asserted (part1); trivial but honest. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parses_plain_text_and_escapes | YES | Escape regression turns `{{` into a placeholder → len > 0 asserted. |
| parses_all_v1_forms | YES | placeholder_set (name→tag map) asserted for each form; a parser that dropped type info fails the map equality. |
| rejects_malformed_messages | YES | 14 exact (variant, offset, found) triples; removing dup-detection, has_other, or offset bookkeeping each break specific rows. |
| renders_simple_and_typed_placeholders | YES | Exact output string "Hello, Lumen! 45%" + empty diagnostics. |
| renders_select_and_plural | YES | Exact outputs for 1/5 counts and known/mkv select values. |
| rendering_never_panics_on_bad_args | YES | Asserts degraded text AND diagnostic counts (2 then 1) — a silent-Ok regression fails the count. |
| number_formatting_helpers / placeholder_kind_tags | YES (weakly) | Exact-string asserts on pure functions. |
All 8 verified passing: baseline orchrun-test-workspace-nofailfast.log.txt:2706-2714. Caveat: they prove the engine correct, not used (F-001).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Parameterized messages use an ICU-MessageFormat subset … v1 subset: {name}; {name, number, percent}; {name, select, …}; {name, plural, …}" | docs/07-config-driven-system.md §6.2 | REAL (code) / UNREACHABLE (shipped) | parse_placeholder:683–693 dispatch + parses_all_v1_forms ok in baseline; zero consumers (F-001). |
| Example `"player.error.unsupported" = "…{format, select, known {…} unknown {…}}"` (no `other` arm) | docs/07-config-driven-system.md §6.2 code block | FALSE (invalid under implemented grammar) | parse_arms:853–857 rejects with MissingOtherArm; message.rs:895–898 test comment admits the contradiction "[VERIFY — doc 07 §6.2 vs. crate rule]". en-US.toml:42 documents `other` as mandatory — doc 07 example is the outlier. |
| "Run by `cargo xtask l10n-audit` (backed by `crates/localization::audit_catalogs`)" | docs/07-config-driven-system.md §6.4 | PAPER-ONLY | `grep -rn "l10n" tools/xtask/src/` → no matches (exit 1); `audit_catalogs` has no caller outside crates/localization (grep exit 1); baseline INDEX stage list has no l10n gate. |
| "Literal braces are escaped {{ and }}" | locales/en-US.toml:46 header / module doc part1:16–20 | REAL (top level only) — limitation documented but untested | parse_message:605–609; inside arms `{{` is a hard InvalidName error (traced, F-007a); no test covers either. |
| Placeholder names `[A-Za-z_][A-Za-z0-9_]*` snake_case | doc 07 §6.2 / en-US.toml:44 | REAL | parse_ident:581–595 exactly this grammar. |

## 7. Dead / duplicated / unreachable code
- The entire part2 range is unreachable from shipped entry points (F-001) — reachable only from the crate's own lib tests and `tests/{audit_proof_matrix,seed_integrity}.rs`.
- `parse_arms` re-parsing `PluralCategory` (F-005) is intra-file duplication.
- The UI ships its own parallel i18n: `ui/src/i18n/messages.ts` (3017 lines, hand-written TS catalog + pseudo locale) — no code path connects it to this crate or to `locales/en-US.toml`; `ui/src/screens/settings/areas/language/locales.ts:16` hard-codes `AVAILABLE_LOCALES: ["en-US"]` while its comment implies derivation from the crate's directory scan.

## 8. Dependencies & flags
- Crate deps: `toml = "1.1"`, `thiserror = "2.0"` (crates/localization/Cargo.toml) — this file uses neither directly (pure std + `fmt`); imports `crate::locale::Locale`, `crate::plural::{PluralCategory, select_plural_category}`.
- No feature gates; `#[cfg(test)]` only for the tests module. No unsafe. No allow-attributes in range.
- `[lints] workspace = true` inherited; clippy clean per baseline run #5/#6.

## 9. Open questions for the next agent
- A06 (part1): does `render_placeholder`'s PluralCategoryMissing path ever fire given parse enforces `other`? (Runtime category-set vs declared arms — audit rule 5 territory.)
- A08 (audit.rs): rule 5 "plural conformance" — the `plural_arms_by_name` input (part1:299–303) is consumed where in audit.rs, and does the missing-`other` parse error make rule 5's "missing category" case unreachable at audit time?
- A10 (catalog.rs/lib.rs): confirm no `#[feature]`-gated or bin target exposes the crate outside tests (lib.rs:113 re-exports verified; no `src/bin`).
- Lead decision owed: doc 07 §6.2 example vs mandatory-`other` rule (the file's own [VERIFY] marker, unresolved).
