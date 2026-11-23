# AUDIT: tools/xtask/tests/config_validator.rs
| Field | Value |
|---|---|
| Auditor | W01-A05 |
| Date | 2026-11-23 |
| Lines / functions | 458 / 29 (10 helpers + assert_fails + 17 tests + 1 Drop impl) |
| File verdict | REAL (executed: 17/17 pass) |
| Reachable from shipped entry point? | NO — test-only; reached by `cargo test -p xtask` (baseline: 4744 passed workspace-wide, includes these) and by my direct run |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 1 |
| Commands run | `./target/debug/deps/config_validator-3991d8058e10136f` -> 17 passed, 0 failed, exit 0 (0.02s); greps vs product.schema.json / scripts/full_gate.sh / tests/cli.rs; sed quote checks |

## 1. One-paragraph verdict
This is an honest, well-built integration-test suite: it spawns the REAL
xtask binary (`CARGO_BIN_EXE_xtask`, L29) against synthetic temp roots,
asserts the full exit-code contract (0 pass / 1 violation / 2 tool error)
plus message content, and every one of the 17 tests would fail under the
obvious mutations (always-pass, always-fail, message drift). I ran the
prebuilt suite: 17/17 green in 0.02s. The blind spot is the one the
orchestrator baseline exposed: the tool FAILS on the real repo (4 missing
brand assets — `assets/` does not exist at all) and no test in this file,
in tests/cli.rs, or in scripts/full_gate.sh ever runs the validator against
the repo's own product.toml, so the suite is structurally incapable of
noticing that the product ships in a state its own gate rejects. A second,
smaller honesty gap: the header claims the fixture schema "mirror[s]
product.schema.json's shapes" but the fixture's required-key sets are a
weakened subset of the real schema's.

## 2. What actually works (verified)
— ran `./target/debug/deps/config_validator-3991d8058e10136f` (built
18:25, newer than the 17:35 source) -> "test result: ok. 17 passed; 0
failed" exit 0. Real binary spawned per test (env! path), not a mock.
— exit-code contract verified by the suite itself: pass=0 (L217-223),
violation=1 (L200), xtask error=2 (L437-442), parse failure=1 (L451-457).
— fixture lifecycle is sound: temp dir per test keyed by pid+tag, Drop
cleanup (L90-94), stale-dir pre-clean (L53).

## 3. Findings
F-001 [S3]
Where: tools/xtask/tests/config_validator.rs:1-7 (whole file), evidence audit/runs/baseline/xtask-config-validator.txt:11-15
Code:
```
//! Fixture-based acceptance proof for `cargo xtask config-validator`
//! (issue #20, card 8.F1): a good product.toml passes against a schema
//! fixture mirroring product.schema.json's shapes, and each seeded failure
```
What is wrong: the suite proves the validator's logic on synthetic roots
but never validates the repository's own product.toml. Baseline:
`config-validator --locked-independent` on the real root -> "result:
FAILURE (4 violation(s))" — `assets/brand/icon.ico|icon.icns|icon-512.png|
logo.svg` all "does not exist under the root" (product.toml:97-101
references them; `ls assets/` -> No such file or directory). grep shows no
`config-validator` invocation in tests/cli.rs or scripts/full_gate.sh, so
nothing automated exercises the real root. Why it matters: the repo ships
referencing brand assets that do not exist (a real product defect for the
product.toml owner) and this green test suite certifies a tool whose
subject matter — this repo's config — fails it. What correct looks like:
one test running the validator on `--root <repo>` asserting PASS (or a
tracked-expected-failure until assets land).
Confidence: HIGH — Verification: ran (test binary + baseline cite) + read.

F-002 [S3]
Where: tools/xtask/tests/config_validator.rs:139-148 and 150-158 vs product.schema.json:17-18, 47, 110-111
Code:
```
      "required": ["name", "short_name", "description_key", "ids"],
```
What is wrong: the fixture schema's required sets are weakened subsets of
the real schema: fixture identity requires 4 keys, real requires 7
(adds publisher, display_version, version_code — product.schema.json:17-18);
fixture window requires 3, real requires 6 (adds height, min_width,
min_height — product.schema.json:110-111); fixture `ids` is a bare
`{"type": "object"}` (L147) while the real ids requires reverse_dns +
windows/macos/linux/web blocks (product.schema.json:47). The header's
"mirroring product.schema.json's shapes" overstates: msgid pattern and
width limits DO match (verified L194 vs product.schema.json:279, L155 vs
:114), but the required-key topology does not. Why it matters: a validator
regression that drops enforcement of a real-schema-only constraint (e.g.
missing `publisher`) keeps this suite green. What correct looks like: load
the real product.schema.json in the happy-path test (write only product.toml
fixtures) or keep a generated mirror with a drift check.
Confidence: HIGH — Verification: read + ran (grep diffs).

F-003 [S3]
Where: tools/xtask/tests/config_validator.rs:369-381 (pins implementation behavior in tools/xtask/src/cmd/config_validator.rs)
Code:
```
    // Catalog present but unparseable -> honest note, not a violation.
    must(fs::write(
        fx.root.join("locales/en-US.toml"),
        "%%% not toml [",
    ));
    let out = run_validator(&fx.root);
    assert_eq!(
        out.code,
        Some(0),
```
What is wrong: the suite documents — and pins as correct — that a CORRUPT
reference catalog (locales/en-US.toml unparseable) downgrades every msgid
existence check to a note and the product still PASSES validation (exit 0).
A shipped product with a broken catalog would render raw message keys at
runtime, yet the gate says PASS. The test is honest about the behavior; the
behavior is a swallowed error (checklist 6.2). Why it matters: the one
cross-file integrity check the validator owns is silently disabled by any
TOML typo in the catalog. What correct looks like: unreadable catalog =
violation (or at minimum a distinct exit code). File owner: the
config_validator.rs implementation agent.
Confidence: HIGH — Verification: ran (this very test passed, proving the
behavior) + read.

F-004 [S4]
Where: tools/xtask/tests/config_validator.rs:16-21
Code:
```
fn must<T, E: std::fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => unreachable!("fixture setup failed: {err}"),
    }
}
```
What is wrong: nothing functional — documented, test-only, lint-legal
(clippy `panic` deny does not fire on unreachable!). Noted only because the
comment claims "a broken fixture means a broken test, never a product
bug": true here because must() is only used for setup writes, never on
validator output. Kept as S4 for the pattern's exposure to future misuse
(someone calling must() on validator results would convert a product bug
into a test panic message that looks like fixture breakage).
Confidence: HIGH — Verification: read (all must() call sites are fs::*
setup — L53-87, L370, no validator output passes through must()).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| must | 16-21 | REAL | — | fixture-only expect shim; F-004 |
| run_validator | 28-41 | REAL | — | spawns real binary; stdout only, stderr ignored (ok — output verified on stdout) |
| Fixture::new | 48-58 | REAL | — | pid+tag temp root, pre-clean |
| write_product/write_good_product | 60-66 | REAL | — | GOOD_PRODUCT const |
| write_schema | 68-70 | REAL | — | writes the drifted fixture schema (F-002) |
| add_brand_assets | 72-76 | REAL | — | 4 one-byte files |
| add_catalog | 78-83 | REAL | — | 2 msgids matching GOOD_PRODUCT keys |
| add_theme | 85-87 | REAL | — | themes/dark.toml |
| Drop::drop | 90-94 | REAL | — | cleanup, `let _` swallow acceptable for temp dirs |
| assert_fails | 198-206 | REAL | — | exit==1 AND needle: the core honesty device |
| good_product_passes | 208-224 | REAL | — | catches always-fail mutation; asserts "result: PASS" |
| unknown_key_fails | 226-236 | REAL | — | needle "unknown key `short_nam`" |
| type_mismatch_fails | 238-248 | REAL | — | width = "wide" |
| missing_required_key_fails | 250-260 | REAL | — | only proves fixture-required keys (F-002) |
| reserved_name_fails | 262-272 | REAL | — | name = "VLC" -> banned |
| out_of_range_integer_fails | 274-284 | REAL | — | width=100 < minimum 480 |
| enum_violation_fails | 286-299 | REAL | — | crash_reports = "always" |
| non_boolean_feature_flag_fails | 301-311 | REAL | — | streaming-hls = "no" |
| dangling_brand_asset_fails | 313-322 | REAL | — | the failure mode the REAL repo is in (F-001) |
| unknown_locale_fails | 324-334 | REAL | — | locale = "xx-XX" |
| theme_outside_directory_listing_fails | 336-360 | REAL | — | two-phase: empty themes/ = note+exit 0, then wrong id = violation |
| missing_msgid_fails | 362-389 | REAL | — | pins the corrupt-catalog downgrade (F-003) |
| unknown_title_variable_fails | 391-404 | REAL | — | {app_name} not defined |
| wrong_schema_version_fails | 406-416 | REAL | — | schema_version 7 != 1 |
| wrong_app_id_fails | 418-431 | REAL | — | PascalCase rule needle |
| missing_files_are_xtask_errors | 433-443 | REAL | — | exit 2 contract |
| unparseable_product_toml_fails_with_exit_one | 445-458 | REAL | — | exit 1 + "parse:" |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 17 | YES | Each asserts a specific exit code AND a stdout needle; always-Pass breaks the 13 assert_fails tests; always-Fail breaks good_product_passes; message drift breaks needles. Verified by execution, not reasoning alone: ran suite -> 17/17. |
| caveat | — | The suite cannot catch F-001 (repo-root failure) or real-schema regressions (F-002) — coverage hole, not per-test dishonesty. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "each seeded failure mode ... fails with exit 1" | header L1-7 | REAL | 13 failure tests assert Some(1); executed |
| "schema fixture mirroring product.schema.json's shapes" | header L3 | PARTIALLY FALSE | required sets/ids shape drift (F-002); msgid pattern + width bounds do match |
| "testkit is not an xtask dev-dep — boundaries.toml dev_deps are Lead-owned" | L13-15 | REAL | boundaries.toml:14-16 confirms dev edges allowlisted separately; xtask Cargo.toml has no testkit |
| (implicit) repo's product.toml validates | — | FALSE | baseline xtask-config-validator.txt:15 "result: FAILURE (4 violation(s))" |

## 7. Dead / duplicated / unreachable code
None. Boilerplate repetition across tests (8-line fixture ritual per test)
is verbose but explicit; a helper would not change honesty.

## 8. Dependencies & flags
std-only test (fs, path, process) — consistent with xtask's zero-dep
posture (Cargo.lock L1406-1408). No feature gates. Uses CARGO_BIN_EXE_xtask
compile-time env — correct Cargo pattern.

## 9. Open questions for the next agent
- tools/xtask/src/cmd/config_validator.rs (implementation, 4 unreachable!/
  todo! hits by grep — not my file): confirm the corrupt-catalog downgrade
  (F-003) and whether the 4 unreachable! are in reachable paths.
- product.toml owner: 4 brand assets referenced at product.toml:97-101 do
  not exist (`assets/` absent) — the root cause of the baseline FAILURE.
- scripts/full_gate.sh owner: config-validator is not in the gate — the
  failure the baseline caught is invisible to full_gate (grep: 0 hits).
