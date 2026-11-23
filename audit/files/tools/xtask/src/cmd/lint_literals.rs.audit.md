# AUDIT: tools/xtask/src/cmd/lint_literals.rs
| Field | Value |
|---|---|
| Auditor | W01-A04 |
| Date | 2025-11-23 |
| Lines / functions | 1561 / 25 production fns + 9 unit tests + 1 test trait |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (dev tool, not in release binary) — YES from gate lane: scripts/full_gate.sh:50 `cargo xtask lint-literals` -> main.rs:124-125 dispatch -> cmd/mod.rs:158-162 -> `lint_literals::run`; shared by check_identity.rs (`use super::lint_literals::{self, Identity, Scope}`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 |
| Commands run | prebuilt `target/debug/xtask lint-literals` on real tree (exit 1, reproduced baseline exactly); 5 synthetic-fixture runs under audit/runs/W01-A04.scratch/ (exit 0/1/2 all exercised); rg/sed greps as cited |

## 1. One-paragraph verdict
This is a real, working, hand-rolled lexer+classifier gate — not a stub. I executed the prebuilt binary (newer than source, so current) against the real workspace and reproduced the baseline failure byte-for-byte: `FAIL — 1 violation(s): apps/cli/src/host.rs:1799: identity — identity(identity.name)`, exit 1, 512 allowlisted sites, 565 files / 19258 literals scanned. The baseline violation is a TRUE POSITIVE by the gate's own rules: host.rs:1799 is the string literal `"lumen-backpressure-{}-{}s.wav"` (a test-scratch temp filename inside `#[cfg(test)] mod session_backpressure_tests` at host.rs:1760-1761); `contains_word` matches `lumen` case-insensitively as a whole word (`-` is a word boundary), `identity.name = "Lumen"` from product.toml is the first brand entry, and identity findings have NO test-context exemption (only loopback URLs do, L41-46) — the module docs (L48-52) explicitly state cfg(test) modules are scanned and legitimate literals must carry inline markers. host.rs:1799 carries no marker, while the identical pattern at apps/cli/src/bench.rs:557 does (`// lumen-allow-literal: cfg(test) scratch filename — test-only, never a shipped surface`). The gate is honest; the repo ships red on a one-line missed annotation. Two genuine parser holes exist (below), both demonstrated with fixtures; neither hides an in-tree violation today.

## 2. What actually works (verified)
- `./target/debug/xtask lint-literals` on real tree -> exact baseline reproduction incl. exit 1 (full output in tool-results/bash_1795458399252_de1d57494e04.txt).
- Synthetic fixtures under audit/runs/W01-A04.scratch/ws{,2}: exit 0 clean tree ("PASS — 0 violations, 0 allowlisted"); exit 1 violations; exit 2 missing product.toml field.
- Verified behaviors: inline marker on/above line suppresses and prints reason; marker text INSIDE a string literal suppresses nothing; loopback URL in a `#[cfg(test)] mod helper2;` file is allowed with fixed reason; `b"..."` and `c"..."` literals ARE scanned (url + identity findings fired); `#[doc = "Lumen ..."]` attribute string produces NO finding (blind spot, F-002); `#[cfg(test)] pub mod helper;` file NOT recognized as test context (blind spot, F-001).
- All 9 unit tests pass: audit/runs/baseline/cargo-test-workspace.txt:6676-6685 (`cmd::lint_literals::tests::* ... ok`).

## 3. Findings
F-001 [S3]
Where: tools/xtask/src/cmd/lint_literals.rs:1031-1038
Code:
```
                } else if c == ';' && test_attr_pending {
                    // A pending test attribute resolved without a
                    // body: a file-level `mod name;` pulls a whole
                    // file into test context.
                    let tokens: Vec<&str> = since_attr.split_whitespace().collect();
                    if tokens.first() == Some(&"mod") && tokens.len() >= 2 {
                        resolve_mod_file(file, tokens[1], &mut mod_files);
```
What is wrong: `tokens.first() == Some(&"mod")` rejects `#[cfg(test)] pub mod X;` / `pub(crate) mod X;` — the visibility prefix makes the first token `pub`, so the mod file is never marked test context. Why it matters: the spec (L41-46) allows loopback URLs "inside `#[cfg(test)]` regions"; a `pub` file-level test mod IS such a region. Demonstrated: fixture `#[cfg(test)] pub mod helper;` with `helper.rs` containing `"http://127.0.0.1:9/blindspot"` fired as a violation, while the identical plain `mod helper2;` was allowed. In-tree instance: apps/web/src/io/mod.rs:66-67 `#[cfg(test)]` / `pub mod mock_fetch;` — mock_fetch.rs is scanned as production context. No in-tree false positive today only because mock_fetch.rs uses non-loopback `media.example` URLs (which need markers regardless; it has 10+). check_panics.rs:633-635 has the identical defect; its test `pub_cfg_test_mod_declaration_is_recognized` (check_panics.rs:1050-1055) only covers the inline-braces form `pub mod tests { }`, not the file-level `pub mod X;` form, so the shared blind spot is untested in both tools. What correct looks like: skip leading `pub`/`pub(...)`/`unsafe` tokens before checking for `mod`.
Confidence: HIGH — Verification: ran (fixture reproduction + in-tree grep, exactly one `pub mod` cfg(test) decl in tree).

F-002 [S3]
Where: tools/xtask/src/cmd/lint_literals.rs:1195-1213 (and the consuming branch at 992-1007)
Code:
```
        if c == '"' {
            text.push(c);
            j += 1;
            while j < chars.len() {
                let s = chars[j];
                text.push(s);
                span += 1;
```
What is wrong: `capture_attribute` swallows string literals inside `#[...]` attributes whole; the `#` branch (L1003-1006) then skips the entire span, so a literal inside ANY attribute is never extracted and never classified. Demonstrated: fixture `#[doc = "Lumen brand string inside an attribute"]` on apps/y/src/main.rs:1 produced NO finding while the direct literal on line 3 of the same file fired `identity(identity.name)`. Why it matters: the C1/C2 gate is defeated by moving a brand string into an attribute (`#[command(name = "Lumen")]`, `#[serde(rename = "…")]`, tauri config attrs). The module's own "Honest limitations (v0)" list (L67-81) documents macro bodies, char literals, `~`, and `::1` — but NOT attribute strings; the omission is undocumented, not declared. Mitigation: rg over crates/*/src and apps/*/src finds ZERO attribute-embedded brand strings in-tree today (command: `rg -n -U '#\[[^\]\n]*"[^"\n]*[Ll]umen' ...` -> no matches), and the repo uses no clap/derive-attribute arg parsing, so the hole is latent. What correct looks like: extract attribute-inner string literals as literal sites (they are compiled in), or document the exemption in the limitations list.
Confidence: HIGH — Verification: ran (fixture) + read + rg (in-tree count 0).

F-003 [S4]
Where: tools/xtask/src/cmd/lint_literals.rs:1164-1181
Code:
```
    if *next == '\\' {
        let mut j = at + 2;
        while j < chars.len() && j <= at + 12 {
            if chars[j] == '\'' {
                return Some(j - at + 1);
```
What is wrong: for the char literal `'\''` the scan matches the ESCAPED quote as the terminator and returns 3 (actual length 4), violating the doc contract "The length (in chars, including both quotes)". Harmless today: the sole caller (L978) uses the value only as a predicate (`is_some()`), and `Mode::Char` re-lexes the escape correctly. Mutation hazard for any future caller that consumes the length.
Confidence: HIGH — Verification: read (single caller confirmed by rg).

F-004 [S4]
Where: tools/xtask/src/cmd/lint_literals.rs:204-220
Code:
```
        let name = get("identity.name");
        let short_name = get("identity.short_name");
        let publisher = get("identity.publisher");
        let reverse_dns = get("identity.ids.reverse_dns");
```
What is wrong: `Identity::load` hard-requires all 10 fields — including the six C2-only technical ids and `brand.accent_color` — even when `run` executes the C1 scan, which uses only the brand trio (classify L298 gates technical/accent on `scope == Scope::C2`). Verified: deleting `identity.ids.web.app_id` from a fixture product.toml makes `lint-literals` exit 2 with `xtask error: internal error: product.toml is missing the identity field(s) identity.ids.web.app_id`. The comment "Every field this gate needs" is false for C1. Over-strict, not fraudulent (exit 2 = "tool broken", never a false PASS).
Confidence: HIGH — Verification: ran (fixture).

F-005 [S4]
Where: tools/xtask/src/cmd/lint_literals.rs:609-622
Code:
```
            if base == "crates"
                && CONFIG_LAYER_CRATES
                    .iter()
                    .any(|name| rel_member == format!("crates/{name}"))
```
What is wrong: exemption and report paths compare forward-slash strings, but `rel()` returns `Path::display()` output, which is backslash-separated on Windows — on a Windows host the four config-layer crates would be scanned and their brand literals would mass-fire. Dev-tool portability only; the gate lane runs on Linux (baseline).
Confidence: MEDIUM — Verification: read (Windows behavior not executable in this sandbox; UNVERIFIED by run).

F-006 [S4]
Where: apps/cli/src/host.rs:1798-1802 (external — context for this gate's shipped state)
Code:
```
        let path = std::env::temp_dir().join(format!(
            "lumen-backpressure-{}-{}s.wav",
            std::process::id(),
            seconds
        ));
```
What is wrong: the repo ships with this gate permanently RED (full_gate sections 7 and 8) over one unannotated test-scratch literal. The repo's own convention exists (bench.rs:557 carries the exact marker this site needs). A main gate that is always red is alarm fatigue: contributors cannot tell a new violation from the known one. Not a defect of this file — pointer for the apps/cli/host.rs auditor.
Confidence: HIGH — Verification: ran (baseline reproduction) + sed (host.rs:1795-1802, bench.rs:553-558).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run | 115-161 | REAL | — | executed; exit 0/1/2 all verified; arg loop + `--root` seam |
| Identity::load | 188-257 | REAL | S4 | F-004 over-strict field set; executed (ok + missing-field paths) |
| classify | 289-329 | REAL | — | executed; kind order + C1/C2 gating verified by unit test + fixture |
| brand_match | 333-340 | REAL | — | first-match label; executed via identity findings |
| technical_match | 344-362 | REAL | — | C2-only; exact substring + case-insensitive accent |
| contains_word | 366-385 | REAL | — | case-insensitive whole word; unit-tested; boundary = ascii_alnum |
| url_match | 394-418 | REAL | — | earliest scheme, host to first delimiter, userinfo dropped; unit-tested |
| abs_path_match | 424-439 | REAL | — | documented marker subset (`/etc/`, `/var/`, other drives not scanned — declared in docs/tests) |
| hex_color_match | 448-478 | REAL | — | 3/6 hex digits, both boundaries; unit-tested incl. `{:#010x}` |
| Outcome::print | 526-565 | REAL | — | exact spec line format verified against baseline output |
| rel | 569-574 | REAL | S4 | F-005 separator fragility |
| scan_workspace | 577-698 | REAL | S3 | F-001 consequence lives here (is_test_context L662-667); shrink-guard errors on missing crates//apps/ (L587-606) — good |
| count_rs_files | 701-708 | REAL | — | swallows IO error as 0 count (cosmetic counter only) |
| inline_marker_reason | 715-735 | REAL | — | executed: same-line/above markers, in-string marker rejected, empty reason rejected |
| collect_rs_files | 739-754 | REAL | — | sorted, recursive; extension check only (`.rs` exact) |
| find_root | 759-782 | REAL | — | executed (walk-up from subdir); `[workspace]` at line start only |
| scan_source | 828-1061 | REAL | S3 | the lexer; F-001/F-002 live in its attribute/mod branches; escapes, raw strings, nesting all handled and tested |
| decode_escape | 1064-1112 | REAL | — | n/t/r/0/\\/"/'/xNN/u{...} with saturating mul; unknown escape kept |
| escape_len | 1116-1131 | REAL | — | must stay in sync with decode_escape (it does for all tested shapes) |
| count_newlines | 1135-1140 | REAL | — | keeps line counter honest across skipped spans |
| looks_like_raw_string | 1145-1160 | REAL | — | b/c/_ prev-char allowance verified by `c"lumen cstr"` fixture firing |
| char_literal_len | 1164-1181 | REAL | S4 | F-003 wrong length for `'\''`; value discarded by only caller |
| capture_attribute | 1187-1231 | REAL | S3 | F-002 string swallowing |
| is_test_attribute | 1237-1292 | REAL | — | test/cfg(test)/any/all, not(not(test)); unit-tested |
| resolve_mod_file | 1296-1320 | REAL | — | 2018 layout both candidates; executed (helper2 resolved, helper not — F-001) |
| tests::Must trait | 1331-1353 | REAL | — | test-local unwrap shim |
| 9 unit tests | 1355-1560 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| word_matching_is_case_insensitive_and_bounded | YES | asserts positive + negative boundaries, case, empty needle |
| url_matching_extracts_hosts | YES | host extraction, loopback flag, userinfo, non-scheme rejection |
| absolute_path_shapes | YES | all markers + non-marker `/etc/passwd` |
| hex_color_shapes | YES | 3/6-digit, boundaries, 7-digit-run and format-string rejections |
| markers_need_a_reason | YES | same-line/above/empty-reason/no-marker matrix |
| scanner_extracts_literals_and_skips_comments | YES | exact literal-vector equality incl. decoded `\u{4c}` (anti-smuggling) and comment immunity |
| scanner_tracks_test_regions_and_mod_files | YES | exact region lines + in/out membership |
| file_level_test_mod_resolves | YES | writes real temp files, asserts resolved path, cleans up |
| test_attribute_shapes | YES | cfg(any/all/not) discrimination |
| classify_c1_vs_c2_scopes | YES | C1 ignores GUID, C2 catches it; accent dual-classification |
All pass (baseline cargo-test-workspace.txt:6676-6685). Gap: no test covers `#[cfg(test)] pub mod X;` (F-001) or attribute-inner literals (F-002) — the suite shares the implementation's blind spots. No test asserts exit codes (verified manually by me instead).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Exit code: 0 no violations \| 1 violations found \| 2 xtask error" (L85) | module doc | REAL | all three executed (fixtures + real tree) |
| "`b"…"` byte strings and raw strings are scanned" (L76-77) | module doc | REAL | `b"https://media.example/byte"` fired url finding (fixture ws2) |
| "a marker without a reason suppresses nothing" (L39-40) | module doc | REAL | unit test + `reason.starts_with('/')`/empty checks (L728-731) |
| "loopback hosts are allowed only inside #[cfg(test)] regions" (L41-46) | module doc | REAL with hole | works for plain `mod X;` (fixture), broken for `pub mod X;` (F-001) |
| "Honest limitations (v0)" list (L67-81) | module doc | INCOMPLETE | omits the attribute-string exemption (F-002) |
| USAGE text "Exit code: 0/1/2" and `--root` seam (L99-112) | USAGE const | REAL | executed |
| full_gate.sh:49-53 "lint-literals" section | scripts | REAL | baseline full_gate.txt:59-63 FAIL matches my reproduction |

## 7. Dead / duplicated / unreachable code
- None dead. `technical_match` + `accent_color` are unused by C1 but consumed by check_identity's C2 scope (check_identity.rs:80) — cross-module live code, not dead.
- Duplication: the cfg(test)-region/mod-decl lexer block (L985-1049) is a near-verbatim copy of check_panics.rs:585-647 — copy-paste divergence risk; both carry the same F-001 defect, evidence the copies were never reconciled.

## 8. Dependencies & flags
Std-only (no deps; xtask Cargo.toml `[package] name = "xtask"`, `edition = "2024"`, `publish = false`, `[lints] workspace = true`). Relies on `toml_lite` (in-crate minimal TOML parser) for product.toml. No feature gates. Not part of any shipped binary.

## 9. Open questions for the next agent
- apps/cli/host.rs auditor: is host.rs:1799 the only unannotated `lumen-*` scratch literal? (bench.rs:557 and 512 other sites carry markers; a one-line marker or a config-driven prefix turns full_gate §7/§8 green.)
- check_identity.rs auditor: C2 inherits F-001/F-002 verbatim via the shared scan_workspace; the same fixture results apply.
- Should `#[cfg(test)] struct S { … }` / `const X = { … }` bodies opening "test regions" (pending attr resolved by ANY `{`, L1015-1021) be tightened? Only widens the loopback-URL exemption — cosmetic today.
