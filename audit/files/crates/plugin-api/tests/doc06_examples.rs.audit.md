# AUDIT: crates/plugin-api/tests/doc06_examples.rs
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-24 |
| Lines / functions | 382 / 11 (4 fixture consts, 1 helper, 6 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — test file; runs via `cargo test -p plugin-api --test doc06_examples` (executed by me, see below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | cargo test -p plugin-api --offline --test doc06_examples → 6 passed, exit 0; python3 mechanical verbatim diff (fixtures vs docs/06-plugin-architecture.md §2.3 blocks) → all 4 exact; rg doc/cross-checks (exit 0) |

## 1. One-paragraph verdict
Honest, mutation-sensitive acceptance tests for the plugin-api manifest parser over the four doc 06 §2.3 example manifests. The "VERBATIM" claim in the header is true — I mechanically diffed all four inline constants against the toml code blocks in docs/06-plugin-architecture.md:277-376 and all four match exactly. The suite proves double round-trip stability (parse→serialize→parse→serialize, bytes 2 == bytes 3), value identity, field-level parsing (kinds, priorities, permissions companion rule, allowlist contents, provenance), and five tampering refusals with typed error matches. All 6 tests pass under my own run. Two soft spots: fixture (a) compares sorted line sets (order-insensitive, so cross-line permutations pass), and fixture (d) lacks field-level asserts for api_version/priority/origin/publisher/source/license, so a parser that dropped `priority = 50` (whose extension default is also 50) would slip through (d).

## 2. What actually works (verified)
— `cargo test -p plugin-api --offline --test doc06_examples` (my run): 6 passed; 0 failed, exit 0. Baseline: audit/runs/baseline/cargo-test-workspace.txt:4856-4866 (same 6 ok, exit_code=0).
— python3 regex extraction: docs/06 toml blocks == the 4 consts byte-for-byte (stripped); `verbatim-match(exact): True` ×4.
— rg: `plugins/builtin/` does not exist (doc's own §3 LEAD-ATTENTION admits it) — the test correctly embeds the doc text inline instead of `include_str!`-ing nonexistent files.

## 3. Findings
F-001 [S4]
Where: crates/plugin-api/tests/doc06_examples.rs:165-178
Code:
```
    fn sorted_body(s: &str) -> Vec<String> {
        let mut lines: Vec<String> = s
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.trim_end().to_string())
            .collect();
        lines.sort();
        lines
    }
```
What is wrong: fixture (a)'s byte comparison is done on SORTED line sets — any reordering of lines between documents compares equal (e.g. two capability lines swapped), and a serializer that emitted `[capabilities]` fields in a different order than both the doc and serde's alphabetical claim would still pass. Within-line content IS exact. The rationale (serde alphabetical emission vs doc's `formats`-first order) is documented at 34-37 and 161-164, and fixture (b) uses a strict byte-exact comparison — so the weakness is confined to (a). What correct looks like: assert the exact serialized form with the known key order, as (b) does. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/plugin-api/tests/doc06_examples.rs:294-322
Code:
```
    for needle in [
        "id = \"ext.acme.lyrics-panel\"",
        "kind = \"ui-panel\"",
        "version = \"0.4.2\"",
        "surfaces = [\"side-panel\"]",
```
What is wrong: fixture (d)'s needle list omits `api_version`, `priority`, `origin`, `publisher`, `source`, `license`, and the field-level asserts (311-321) check only kind and the two permissions. A parser/serializer that silently dropped `priority = 50` would pass (d): `effective_priority()` for extensions defaults to 50 (fixture (c) line 253 labels 50 the "extension default posture", making the explicit `priority = 50` in both (c) and (d) indistinguishable from the default). Same blind spot partially in (c) — it asserts priority 50 but that equals the default. Confidence: HIGH — Verification: read + mutation reasoning (not executed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| const MATROSKA | 38-52 | REAL | — | byte-verbatim vs doc §2.3(a) (mechanically diffed) |
| const FLAC | 56-71 | REAL | — | byte-verbatim vs doc §2.3(b), incl. the line-wrapped sample_rates array |
| const MUSICBRAINZ | 76-105 | REAL | — | byte-verbatim vs doc §2.3(c), incl. `"""` multi-line settings and placeholder hash |
| const LYRICS | 108-129 | REAL | — | byte-verbatim vs doc §2.3(d), incl. inline comments |
| double_round_trip | 133-153 | REAL | — | value identity (146) + byte stability (151), both asserted; honest about what round-trip can and cannot prove (module doc 24-27) |
| fixture_a_builtin_matroska_demuxer | 155-189 | REAL | S4(F-001) | sorted-set compare + 5 field-level asserts |
| fixture_b_builtin_flac_decoder | 191-217 | REAL | — | strict byte-exact expected string; 13 sample rates, hw=false asserted |
| fixture_c_wasm_metadata_provider | 219-291 | REAL | — | 14 needles, negative [dependencies] check, provenance/allowlist/companion-rule field asserts |
| fixture_d_wasm_ui_panel | 293-322 | REAL | S4(F-002) | 7 needles + kind/permission asserts; missing priority/origin/publisher asserts |
| all_four_reparse_from_their_own_output | 324-348 | REAL | — | each fixture accepts its own serialization |
| doc_examples_reject_tampering | 350-382 | REAL | — | unknown field, bad semver, priority 5000, companion-rule violation, extension-without-provenance — all typed matches |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixture_a | YES (mostly) | field-level asserts catch dropped/renamed fields; sorted-set compare catches missing lines but not reordering (F-001) |
| fixture_b | YES | byte-exact expected output — strongest check in the file |
| fixture_c | YES | 14 needle strings + `!once.contains("[dependencies]")` + typed field asserts; would not catch a dropped priority equal to the default |
| fixture_d | PARTIALLY | F-002: priority/origin/api_version/publisher/source/license unguarded |
| all_four_reparse | YES for serialization that doesn't reparse | only proves reparse-ability, not completeness (correctly scoped in comment 326-328) |
| doc_examples_reject_tampering | YES | five concrete tampering shapes with exact `ManifestError` variant matches (Toml/Priority/Permissions/Provenance) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the four doc 06 §2.3 example manifests, VERBATIM" | doc06_examples.rs:1-2 | REAL | python3 diff: all 4 consts byte-equal to docs/06-plugin-architecture.md §2.3 toml blocks |
| "double round-trip stability … plus value identity; each fixture test below proves both" | doc06_examples.rs:24-27 | REAL | double_round_trip asserts both; my run passes |
| doc captions cite `plugins/builtin/lumen.demux.matroska.toml` files | docs/06-plugin-architecture.md:274,305 | PAPER-ONLY | `plugins/builtin/` does not exist (doc's own §3 LEAD-ATTENTION admits it); the test wisely uses inline constants, not include_str! |

## 7. Dead / duplicated / unreachable code
None. No `#[ignore]`, no tautologies, no `todo!`.

## 8. Dependencies & flags
plugin-api only (kind, manifest, ManifestError/Origin/PluginManifest). No features. No dev-deps.

## 9. Open questions for the next agent
- Does `PluginManifest::effective_priority()` really default extensions to 50 (affects F-002's exploitability)? Check crates/plugin-api/src/manifest.rs.
- When `plugins/builtin/` lands, these inline constants should switch to `include_str!` to keep doc/code/file in lockstep.
