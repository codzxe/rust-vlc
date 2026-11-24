# AUDIT: crates/product-config/tests/rebrand_host.rs (+ hosted tests/rebrand/rebrand_acceptance.rs)
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 14 (host) + 315 hosted / host: 0 fn; hosted: 7 test fns + 1 helper |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test plane). Reachable from CI: `cargo test -p product-config --test rebrand_host` — executed, 7 passed. Host compiles repo-root tests/rebrand/rebrand_acceptance.rs via `#[path]` (od-verified bytes: `#[path = "../../../tests/rebrand/rebrand_acceptance.rs"]`), which is otherwise outside any package. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p product-config --test rebrand_host` -> 7 passed, 0 failed; `od -c` byte-verification of the `#[path]` line and of runtime.rs attributes (a display artifact initially made `#[must_use` render as `#ust_use` — od proves the source is intact; NOT a finding); rg cross-check that the externally-cited docs evidence (tools/xtask tests/cli.rs -> rebrand_skeleton.rs) exists and asserts what is claimed |

## 1. One-paragraph verdict
This is the honest kind of acceptance suite: the Nova fixture is loaded through the real pipeline (`include_str!`-ed committed product.schema.json + `load_str`), every runtime surface is asserted with exact expected strings (title, installer ids, About facts, CLI banner, log prefix/startup line), source-identity leakage is explicitly tested against ("Lumen"/"lumen"/"dev.lumen" must not appear), and the acceptance report pins its own blocker register (`blocked.len() == 2`, `passed == 7`) so a BLOCKED line can neither silently persist nor silently appear. The two BLOCKED entries are accurate against reality (cli host-wiring really is hardcoded in help.rs — my File-2 execution confirms the CLI banner does not consume product-config; installer build really is infra-blocked per baseline gdk-sys failure). The only defect is cosmetic: the report's doc claims "Every PASS line is backed by a real assertion above" but the `docs` surface's check boolean is a hardcoded `true` whose real evidence lives in another suite — disclosed inline, but the self-description overstates, and nothing in THIS suite fails if that external test is deleted.

## 2. What actually works (verified)
— `cargo test -p product-config --test rebrand_host` -> 7 passed, 0 failed (fixture-load, window-title, installer-identity, about, cli-identity, logs, acceptance-report).
— Cross-check of the external docs evidence: tools/xtask/tests/cli.rs:104 hosts `tests/rebrand/rebrand_skeleton.rs` via the same `#[path]` convention, and rebrand_skeleton.rs:490-493 asserts the rewritten `schemas/generated/settings-help.txt` carries Nova and not "lumen" — the evidence string in the report is real, not invented.
— Blocker accuracy: "apps/cli help.rs v0 hardcodes its banner" — verified independently: my settings-help.txt audit executed `./target/debug/lumen --help` and the banner is generated from hardcoded literals in help.rs:101-127 with `[PROPOSAL]` markers; product-config resolvers are not on the CLI's dependency list.
— runtime.rs resolvers exercised by these tests are real one-line implementations (read: `log_line_prefix` = `identity.short_name.to_lowercase()`, `cli_version_line` = `format!("{} {}", name, display_version)`, `resolve_window_title` = template `.replace()` with idle fallback to `identity.name`).

## 3. Findings
F-001 [S4]
Where: tests/rebrand/rebrand_acceptance.rs:218-222 (claim) vs 265-269 (the docs check)
Code: (verbatim, 218-222)
```
/// The per-surface acceptance report (doc 07 §8.3; printed by the gate
/// log at docs/gates/logs/rebrand-acceptance-4h2.log). Every PASS line
/// is backed by a real assertion above (and re-asserted here); every
/// BLOCKED line names the exact infra/wiring reason — no fabricated
/// greens, per the honest-blocker precedent.
```
Code: (verbatim, 265-269)
```
        (
            "docs",
            true, // evidence lives in the xtask-hosted test below
            "build-time plane: tools/xtask tests/cli.rs asserts dest README/settings-docs/settings-help + tauri.conf.json carry Nova, zero Lumen".to_string(),
        ),
```
What is wrong: the `docs` PASS line's check boolean is a constant `true`; the backing assertion lives in a different test binary (verified to exist — rebrand_skeleton.rs:490-493 — so this is disclosure, not fabrication). But the doc claim "Every PASS line is backed by a real assertion above (and re-asserted here)" is false for this one surface, and if the xtask-side test were deleted or weakened, this report would still print `docs PASS`. What correct looks like: either move a cheap dest-assertion into this suite or reword the doc ("every PASS line except the build-time docs plane, asserted in tools/xtask"). Confidence: HIGH — Verification: read + rg of the external evidence.
F-002 [S4]
Where: tests/rebrand/rebrand_acceptance.rs:104-123 (assertions) + crates/product-config/src/runtime.rs:57-77 (the resolver they pin)
Code: (verbatim, rebrand_acceptance.rs:111-115)
```
    let idle = resolve_window_title(&config.window, &config.identity, None);
    assert_eq!(
        idle, NOVA_NAME,
        "surface window-title: idle fallback is the product name"
    );
```
What is wrong: the runtime's idle path returns `identity.name` and never reads `window.title_idle_key` (runtime.rs references `title_idle_key` only in doc comments and a fixture literal; rg: 3 hits, all non-execution). The test bakes that in (`idle == NOVA_NAME`), so a product whose `title_idle_key` resolves to different copy than its name would silently get the name as the idle title — the localized idle-title plane does not exist at runtime (consistent with the workspace-wide finding that no localization consumer is reachable; see my message.rs audit). Disclosed in runtime.rs's own doc ("whose pre-localization rendering is the product name itself"), so hygiene, not fraud. What correct looks like: assert idle title through a resolvable key seam once localization is wired, or document the gap in the acceptance report. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (hosted test file >100 LOC)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| (host) rebrand_host.rs module | 13-14 | REAL | — | od-verified `#[path]` wiring; target path resolves to repo-root tests/rebrand/ |
| nova() | 62-69 | REAL | — | real pipeline: committed schema parse + load_str; `unreachable!` only on fixture-invalid (assert semantics) |
| surface_fixture_load_nova_through_the_real_pipeline | 76-98 | REAL | — | 6 exact field assertions incl. accent |
| surface_window_title_resolves_nova | 103-123 | REAL | S4 | F-002: pins the non-localized idle path; leak check for Lumen tokens is real |
| surface_installer_identity_derives_from_nova_config | 131-155 | REAL | — | 5 exact derived-id assertions (desktop_bundle = reverse_dns + ".desktop") |
| surface_about_facts_render_nova | 161-175 | REAL | — | 4 exact assertions incl. logo path |
| surface_cli_identity_resolves_nova | 180-193 | REAL | — | exact banner + usage header |
| surface_log_identity_resolves_nova | 198-211 | REAL | — | exact prefix + startup line |
| rebrand_acceptance_report | 223-315 | REAL | S4 | F-001 docs check constant; blocker pin (2) + count pin (7) are honest and brittle-by-design |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 6 surface tests | YES | exact string equality against resolver outputs over a really-loaded config |
| rebrand_acceptance_report | YES (6 of 7 checks) | re-asserts each surface's condition; pins blocked==2 and passed==7 — tamper-evident |
| docs check within report | NO (in this suite) | constant `true` (F-001); enforced only in tools/xtask's suite |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every PASS line is backed by a real assertion above (and re-asserted here)" (L220-221) | rebrand_acceptance.rs | FALSE for `docs` | F-001: constant true; external evidence verified real (rebrand_skeleton.rs:490-493) |
| "The Nova fixture is loaded through the *real* runtime config path" (L18-19) | rebrand_acceptance.rs | REAL | nova() uses include_str! product.schema.json + load_str; test run passes |
| "BLOCKED surfaces ... asserted to match the known blockers exactly" (L34-37) | rebrand_acceptance.rs | REAL | assert_eq blocked.len()==2 (L306-310), executed |
| Blocker "apps/cli help.rs v0 hardcodes its banner" (L282) | rebrand_acceptance.rs | REAL | independently verified: lumen --help banner from help.rs literals; CLI does not depend on product-config |
| Blocker installer "no bundler/toolchain in this sandbox" (L277) | rebrand_acceptance.rs | REAL | baseline cargo-build-desktop-tauri.txt: gdk-sys failure (environment) |

## 7. Dead / duplicated / unreachable code
- No dead code. The `#[path]` double-hosting pattern (acceptance via product-config, skeleton via tools/xtask) is deliberate (C5 std-only xtask) and both hosts verified present.
- Display-artifact note (not a finding): grepping/sedding this crate's `#[must_use` lines through this harness strips `[m` (SGR-reset misparse) — od -c proves source bytes intact. Recorded so later agents don't file phantom corruption.

## 8. Dependencies & flags
Test plane of product-config (dev-dependencies none beyond the crate itself; uses include_str! for schema + fixture). No features, no unsafe in the hosted file. `unreachable!` used twice as fixture-load asserts (consistent with workspace test idiom).

## 9. Open questions for the next agent
- runtime.rs auditor: F-002's title_idle_key gap originates there; the acceptance suite only pins current behavior.
- docs/gates/logs/rebrand-acceptance-4h2.log — the report's printed destination: does the committed log match the current report output? Not checked (out of my file list; UNVERIFIED).
