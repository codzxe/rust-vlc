# AUDIT: tools/xtask/src/cmd/rebrand.rs
| Field | Value |
|---|---|
| Auditor | W01-A08 |
| Date | 2026-11-23 |
| Lines / functions | 1254 / 20 (prod) + 18 (test fns/helpers, 15 #[test]) |
| File verdict | REAL-UNPROVEN (orchestration `rebrand_inner` never executed by me; unit layers verified via orchestrator baseline) |
| Reachable from shipped entry point? | NO — dev tool only: `cargo xtask rebrand` -> `cmd::SUBCOMMANDS` entry `run: rebrand::run` (tools/xtask/src/cmd/mod.rs:120). Not linked into lumen-cli/desktop; no scripts/*.sh or .github workflow invokes it (rg over scripts/ = 0 hits). Only other caller: integration test tests/rebrand/rebrand_skeleton.rs (spawns the binary). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 4 |
| Commands run | rg rebrand scripts/ -> 0 matches; rg 'rebrand::run' -> cmd/mod.rs:120 only; sed verifications (all quotes); baseline citation audit/runs/baseline/cargo-test-workspace.txt:6688-6702 (15/15 unit tests ok) and :6786-6792 (skeleton: 4 ok, 3 #[ignore]d boot-tier) |

## 1. One-paragraph verdict
This is real, working code, not a stub: a std-only TOML-subset reader, a byte-preserving product.toml value rewriter, a longest-first token rewriter, an allowlist parser, and an orchestrator that copies 5 identity surfaces into a fresh dest dir. The unit tests are mutation-sensitive (count==19 / count==2 assertions, 8 malformed-input cases, absence assertions on "Lumen"/"dev.lumen"). The defects are honesty defects, not fakeness: the SAMPLE_PRODUCT fixture comment claims a sync test that does not exist (and the fixture IS out of sync with product.toml), the allowlist machinery is behaviorally inert (parsed, counted, printed — never consulted when deciding what to rewrite), and the rebranded tauri.conf.json keeps the source version string while product.toml gets the target's, producing internally inconsistent installer metadata. `rebrand_inner` itself was never executed by me (running it would write into target/, outside audit/); its behavior is evidenced only by the orchestrator baseline skeleton tests.

## 2. What actually works (verified)
— verified by citation of audit/runs/baseline/cargo-test-workspace.txt lines 6688-6702: all 15 `cmd::rebrand::tests::*` pass (reader, rewriter, allowlist, args, slug).
— verified by citation of baseline lines 6788-6792: `rebrand_skeleton::{nova_fixture_rebrand_passes_skeleton_assertions, rebrand_malformed_config_is_an_error, rebrand_missing_config_file_is_an_error, rebrand_missing_identity_fields_is_an_error, reverse_control_real_product_toml_is_a_noop}` all pass — the no-op test proves rebrand on the real product.toml produces byte-identical surface copies.
— verified by `ls`: all 5 SURFACES files plus tools/xtask/data/rebrand-allowlist.toml and tests/rebrand/{nova.toml,assets/*} exist in-tree, so the hard failure mode "cannot read surface" cannot fire today.
— verified by `od -c` on tests/rebrand/nova.toml tail: `[migrations]\n# entries = []\n` intact (a suspicious `igrations]` in terminal cat was a display artifact, not file corruption).

## 3. Findings
F-001 [S3]
Where: tools/xtask/src/cmd/rebrand.rs:852-853, 860-861 vs product.toml:27,29
Code:
```
    // The committed product definition, trimmed to the identity-bearing
    // tables (kept in sync by test below against the real file).
```
```
display_version = "0.1.0-dev"
version_code = 1
```
What is wrong: There is no "test below" (or anywhere) comparing SAMPLE_PRODUCT to the real product.toml — rg for SAMPLE_PRODUCT shows use only inside this test module, no sync test exists. And the fixture is demonstrably out of sync: real product.toml has `display_version = "0.1.3"` / `version_code = 4`. Why it matters: the comment asserts a maintained invariant that silently rotted; a reader trusts a sync guarantee that never existed. What correct looks like: either a test that extracts the identity tables from the real product.toml and diffs them against SAMPLE_PRODUCT, or deletion of the claim. Confidence: HIGH — Verification: read (rg + sed on both files).

F-002 [S3]
Where: tools/xtask/src/cmd/rebrand.rs:766,774-778,784-786 + tools/xtask/data/rebrand-allowlist.toml:15-16
Code:
```
    let exempt_surfaces = SURFACES.iter().filter(|s| is_exempt(&allowlist, s)).count();
```
```
        "allowlist: {} exempt path(s), {} in the v0 surface set",
```
```
# directory prefix. The tool scans each entry for old identity tokens and
# reports the counts as `exempt:` lines.
```
What is wrong: The allowlist has ZERO behavioral effect on the rebrand. The surface loop (`for surface in SURFACES`) processes all 5 surfaces unconditionally; no `is_exempt` check gates any rewrite. The tool prints no `exempt:` lines anywhere (only `rebrand:`/`dest:`/`allowlist:`/`touched:`/`copied-asset:`/`missing-asset:`/`result:`), so the data file's documented contract ("scans each entry for old identity tokens and reports the counts as `exempt:` lines") is not implemented. With the committed allowlist (14 entries, none matching a surface), the printed count "0 in the v0 surface set" is decorative output implying an exemption mechanism that does not exist in this code path. Why it matters: a maintainer extending SURFACES believes listing a path in the allowlist exempts it — it does not. What correct looks like: either implement the scan+report or fix the allowlist header to say the file is currently consumed by check-identity only. Confidence: HIGH — Verification: read (full-file read + rg for print statements). Note: whether check_identity.rs honors this file is checked in that file's audit.

F-003 [S3]
Where: tools/xtask/src/cmd/rebrand.rs:15 (module doc surface table) + apps/desktop-tauri/tauri.conf.json:4
Code:
```
//! | `apps/desktop-tauri/tauri.conf.json` | `productName`, window `title`, `identifier` (reverse-DNS + `.desktop`) | token rewrite (card 4.H2 — the installer-identity config-file surface; ... |
```
```
  "productName": "Lumen",
  "version": "0.1.3",
```
What is wrong: The surface table lists only 3 identity keys for tauri.conf.json, but the file also carries `"version": "0.1.3"`, which equals product.toml's `display_version` today. `display_version` is one of the 19 rewritten identity fields, yet the token rewriter has no token for it (nova target's "0.2.0-nova" never appears in the rebranded tauri.conf.json). A rebranded dest copy therefore contains productName "Nova Player" with version "0.1.3" — internally inconsistent installer metadata. No test asserts the tauri version (rg over tests/rebrand/ shows version assertions only for product.toml's `identity.display_version`). Why it matters: the tool's entire purpose is a consistent rebranded surface set; the installer version is identity-bearing and left stale. What correct looks like: rewrite tauri.conf.json `"version"` from the target display_version, or document the exclusion explicitly. Confidence: HIGH — Verification: read (both files + fixture + tests).

F-004 [S4]
Where: tools/xtask/src/cmd/rebrand.rs:480-484,497-502
Code:
```
    for raw in src.split('\n') {
        let line = raw.trim();
```
```
            let indent_len = raw.len() - line.len();
            let mut new_raw = String::with_capacity(raw.len() + new_value.len());
            new_raw.push_str(&raw[..indent_len]);
            new_raw.push_str(&line[..span.start]);
```
What is wrong: On a CRLF product.toml, `raw.trim()` strips the trailing `\r`; untouched lines are pushed as `raw.to_string()` (keeping `\r`) but rewritten lines are rebuilt from the trimmed `line`, losing the `\r` — mixed line endings in one output file. Latent only (repo files are LF). What correct looks like: detect the line terminator and re-append it. Confidence: HIGH (code path) — LOW (real-world trigger) — Verification: read.

F-005 [S4]
Where: tools/xtask/src/cmd/rebrand.rs:754-758,784-786
Code:
```
    if dest.exists() {
        return Err(format!(
            "dest {} already exists — refusing to overwrite (delete it first)",
```
What is wrong: dest-existence is checked once up front, but surface writes happen incrementally; if surface #3's read fails after #1-2 were written, a partial dest is left behind that then blocks every re-run ("already exists — refusing"). No cleanup on error. Why it matters: minor operational annoyance; the error message even instructs manual deletion. What correct looks like: write to a temp dir and rename, or remove the partial dest on error. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: tools/xtask/src/cmd/rebrand.rs:254-264,277-287
Code:
```
                if let Some(message) = reject_or_skip(line) {
                    return Err(ParseErr {
```
```
    let Some((_, rest)) = line.split_once('=') else {
        // not an assignment at all — reject
        return Some("expected `key = value`".to_string());
```
What is wrong: Two subset-reader asymmetries: (a) a multi-line TOML array's continuation lines (e.g. `"en",`) hit "expected `key = value`" and reject a valid file — real product.toml and nova.toml only use single-line arrays, so latent; (b) `foo == "x"` (invalid TOML) is silently skipped because the first `=` splits and the remainder does not start with `"` — invalid input accepted instead of rejected. What correct looks like: treat a non-`"` rest after `=` as an error unless it matches a scalar grammar. Confidence: HIGH — Verification: read (logic traced; not executed).

F-007 [S4]
Where: tools/xtask/src/cmd/rebrand.rs:546-556
Code:
```
        (source.name.clone(), target.name.clone()),
        (source.short_name.clone(), target.short_name.clone()),
```
What is wrong: With the committed identity (name == short_name == "Lumen"), the two pairs collide on `from`; the post-sort `dedup_by(|a, b| a.0 == b.0)` keeps whichever lands first (the name pair) and silently discards the short_name mapping — so every "Lumen" token (including `lumen-cli`-style strings via the lowercase pair) becomes "Nova Player"/"nova player", never "Nova". Deterministic and arguably intended, but the precedence rule is undocumented and the dedup only works because same-`from` pairs share a sort length and original adjacency. What correct looks like: an explicit documented precedence (name wins) instead of an ordering accident. Confidence: MEDIUM — Verification: read (dedup semantics traced by hand; not executed).

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse_args | 69-114 | REAL | — | dup/missing-value/unknown-flag handling; tested (baseline 6690-6693) |
| run | 117-128 | REAL | — | thin dispatch Help/Rebrand -> ExitCode |
| Identity::fields | 160-188 | REAL | — | 19 (key,value) pairs, order stable |
| parse_toml_subset | 211-268 | REAL | S4 | F-006 asymmetries; O(n²) dup-check fine at n~60 |
| reject_or_skip | 273-308 | REAL | S4 | F-006(b) silently skips `==`-shaped lines |
| bad_escape | 313-324 | REAL | — | rejects all escapes but \" and \\ (documented subset) |
| table_header_name | 327-335 | REAL | — | handles [t] and [[t]] |
| string_assignment | 340-371 | REAL | — | span excludes quotes; preserves trailing comments |
| is_bare_key_char | 373-375 | REAL | — | ascii alnum + _ - |
| extract_identity | 380-425 | REAL | — | reports all 19 missing together; tested (6694) |
| blank_identity | 427-450 | REAL | — | 19 empty strings |
| rewrite_product_toml | 467-514 | REAL | S4 | byte-preserving rebuild; F-004 CRLF; elides equal values |
| token_table | 523-558 | REAL | S4 | longest-first; F-007 undocumented precedence via dedup |
| rewrite_tokens | 564-582 | REAL | — | single left-to-right pass, first-match-wins; O(n·17) fine |
| parse_allowlist | 599-647 | REAL | S3 | parses fine but result is inert (F-002); stray pre-table lines silently ignored |
| is_exempt | 650-658 | REAL | S3 | correct matching (dir prefix tested) but never consulted for gating |
| find_workspace_root | 677-691 | REAL | — | ancestor scan for [workspace]; duplicated from gate.rs (comment admits) |
| slug | 694-704 | REAL | — | tested incl. "A  B--C" -> "a-b-c" |
| rebrand | 708-716 | REAL | — | exit 0/2 wrapper |
| rebrand_inner | 718-846 | REAL-UNPROVEN | S3/S4 | never run by me; F-002/F-003/F-005 live here; baseline skeleton tests exercise it |

## 5. Test honesty (15 #[test] in this file + helpers)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| reader_parses_nested_tables_and_skips_non_string_values | YES | asserts 3 exact lookups + version_code absent |
| reader_rejects_malformed_lines_with_line_numbers | YES | 8 distinct malformed inputs x message needles |
| missing_identity_fields_are_all_reported_together | YES | asserts exact missing count 18 |
| product_toml_rewrite_replaces_identity_values_in_table_context | YES | asserts count==19, presence of 6 targets, absence of "Lumen"/"dev.lumen"/"#4f8cff" |
| product_toml_rewrite_elides_unchanged_values | YES | count==0 + byte-equality |
| rebuild_preserves_spacing_and_comments_exactly | YES | byte-exact expected strings incl. comment survival |
| readme_title_rule_rewrites_name_tokens | YES | count==2 + absence of "Lumen" |
| token_table_orders_longest_first_and_drops_identity_equals | YES | sortedness + empty-on-identity-equal |
| allowlist_parses_entries_and_matches_files_and_directories | YES | 5 positive/negative match assertions |
| allowlist_integrity_requires_path_reason_and_owner | YES | 3 broken blocks -> error |
| args_require_config / args_reject_unknown_flags / args_reject_missing_and_duplicate_values / args_accept_help_config_dest | YES | exact message needles; pattern-matched arms |
| slug_is_deterministic | YES | 3 exact expected values |
Gaps: no test for CRLF input, no test that the allowlist gates surface rewrites (it doesn't — see F-002), no test that rebrand_inner handles a missing surface file. The 3 #[ignore]d boot-tier placeholders live in tests/rebrand/rebrand_skeleton.rs, not here (baseline 6786-6793).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The working tree is NEVER modified in place: all output goes to --dest" | rebrand.rs:6-8 | REAL-UNPROVEN | every write goes to dest_path (L792-796, 821-825); not executed by me |
| "an existing dest is an error" | rebrand.rs:8 | REAL | L754-758 |
| "Every touched file and every token replacement is reported on stdout (touched:/copied-asset:/missing-asset:)" | rebrand.rs:21-24 | REAL | L797-805, 826, 832 |
| "Missing assets are reported, not fatal" | rebrand.rs:37-38 | REAL | L829-834 |
| "kept in sync by test below against the real file" (SAMPLE_PRODUCT) | rebrand.rs:852-853 | FALSE | F-001: no such test; 0.1.0-dev/1 vs real 0.1.3/4 |
| "The tool scans each entry for old identity tokens and reports the counts as `exempt:` lines" | rebrand-allowlist.toml:15-16 (the file this tool loads) | FALSE | F-002: no exempt: output, no scan |
| tauri.conf.json identity = productName/title/identifier only | rebrand.rs:15 | SHALLOW | F-003: file also carries version="0.1.3" left stale |
| "locales/en-US.toml ... none — no identity literal outside the allowlisted header" | rebrand.rs:19 | REAL | rg over locales/en-US.toml: 1 hit, the header comment line 2 |

## 7. Dead / duplicated / unreachable code
- `Exempt.reason`/`Exempt.owner` are `#[allow(dead_code)]` fields (L592-595) — parsed and integrity-checked, never read for behavior. The entire allowlist is decorative in this command (F-002).
- `find_workspace_root` is a copy of gate.rs's private helper — admitted in its own comment (L675-676); duplication by freeze-policy, not accident.
- Not dead: everything else is reachable from `rebrand::run`.

## 8. Dependencies & flags
Std-only (fs, path, process, env, collections via full paths) — matches the xtask C5 std-only policy stated in the module doc. No feature flags gate this file; `cargo xtask rebrand` dispatch is unconditional in cmd/mod.rs.

## 9. Open questions for the next agent
- Does check_identity.rs consume tools/xtask/data/rebrand-allowlist.toml as its header promises ("the future check-identity subcommand consumes this same file")? (audited next in this agent's file 5)
- Who, if anyone, ships a rebranded tree? If nothing consumes dest/, rebrand is a fixture-generation tool only, which bounds F-003's real blast radius.
