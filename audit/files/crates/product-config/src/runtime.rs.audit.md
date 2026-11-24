# AUDIT: crates/product-config/src/runtime.rs
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 213 / 9 (InstallerIdentity, AboutFacts structs; resolve_window_title, log_line_prefix, log_startup_line, cli_version_line, cli_usage_header, installer_identity, about_facts + 2 tests) |
| File verdict | REAL (library-only; hosts unwired) |
| Reachable from shipped entry point? | NO. Every caller is a test: crates/product-config/src/runtime.rs's own tests + tests/rebrand/rebrand_acceptance.rs:106-261 (workspace test) + one #[ignore]d rebrand_skeleton.rs:637. No app calls any of the 7 functions: the shipped window title is the hard-coded literal `"title": "Lumen"` in apps/desktop-tauri/tauri.conf.json:14; window geometry is applied by desktop-tauri's own `parse_window_table(include_str!("../../../product.toml"))` (apps/desktop-tauri/src/app.rs:243) — a separate parser in the `desktop` crate, not product-config::runtime. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg all 7 function names across repo (callers = tests only); read apps/desktop/src/main.rs:20-56 (skeleton, prints spec only); read apps/desktop-tauri/tauri.conf.json:12-20 (static title/height); baseline cargo-test-workspace.txt:5028 (41 unit tests pass, exit 0). |

## 1. One-paragraph verdict
Correct, tiny, well-tested pure functions — and completely unwired. All seven resolvers do exactly what their docs claim (I verified the substitution logic and the exact-value tests, which load the REAL product.toml so config drift fails loudly), and the header honestly registers the missing hosts ("Hosts that do not exist yet (the CLI banner printer, the runtime logger) are recorded as honest gaps"). But doc 07 §8.1's runtime-plane row — "every surface reads `product-config` at run time: window title, About, CLI banner, log prefix, control-server engineInfo, PWA manifest" — is contradicted by the shipped tree: the tauri window title is a literal in tauri.conf.json, the CLI banner is a literal in apps/cli, and nothing links runtime.rs. The rebrand acceptance test proves the FUNCTIONS resolve a Nova fixture; it cannot prove any shipped surface renders them.

## 2. What actually works (verified)
— verified by baseline (41 unit tests, exit 0) incl. `the_shipped_product_toml_resolves_lumen_identity`: loads the real product.toml via include_str (drift = loud failure) and asserts "Big Buck Bunny — Lumen", idle "Lumen", prefix "lumen", "Lumen 0.1.3" (matches product.toml:27), "dev.lumen.desktop", "assets/brand/logo.svg".
— substitution safety verified by read: `title_template.replace("{product_name}", …).replace("{media_title}", …)` — {product_name} is substituted BEFORE the media title is inserted, so a media title containing "{product_name}" or "{media_title}" is never re-expanded (no recursive injection). Test 2 pins this (template with no slot → media ignored).

## 3. Findings
F-001 [S3]
Where: crates/product-config/src/runtime.rs:1-24 (header) vs doc 07 §8.1 (docs/07-config-driven-system.md:792) and the shipped tree
Code:
```
//! Doc 07 §8.1 splits rebrand into a **build-time** plane (`cargo xtask
//! rebrand` rewrites packaging) and a **runtime** plane: "every surface
//! reads `product-config` at run time". Phase 1 shipped the model + the
//! build-time tool; the functions here are the *resolution* half of the
//! runtime plane — the single seam every identity-bearing runtime surface
```
What is wrong: The seam has no seams attached. rg over the whole repo: the 7 functions are called only from tests (rebrand_acceptance.rs + unit tests). The shipped surfaces named by doc 07 §8.1 do NOT read them: window title = hard-coded "Lumen" (apps/desktop-tauri/tauri.conf.json:14); CLI banner = literal in apps/cli (per the header's own gap note); About = build-info seam (card 4.E4) per the header. The header discloses the gap honestly — the DOC claim is what is false-in-practice today. Same class as merge.rs F-002: real, tested code with zero production blast radius. Confidence: HIGH — Verification: ran (exhaustive rg).
F-002 [S4]
Where: apps/desktop-tauri/tauri.conf.json:12-19 (cross-file drift touching this crate's contract)
Code:
```
        "label": "main",
        "title": "Lumen",
        "width": 1280,
        "height": 800,
        "minWidth": 940,
        "minHeight": 600,
```
What is wrong: The first-paint window spec disagrees with the config this crate owns: product.toml `[window]` says width=1280/height=720/min 940x600; tauri.conf.json says 1280x800 (min 940x600 — the w28 fix DID land on the mins). The 800px height is masked at runtime only if `apply_window_spec` (app.rs:243) overrides the size after first paint — i.e. the product-config values are second-class to the static file. Filed here because runtime.rs is the module whose contract ("surfaces read product-config at run time") this drift violates. Confidence: HIGH — Verification: read both files; desktop-tauri does not build in this sandbox (baseline: gdk-sys), so the override behavior is UNVERIFIED-executed.
F-003 [S4]
Where: crates/product-config/src/runtime.rs:70-77
Code:
```
    match media_title {
        Some(media) => window
            .title_template
            .replace("{product_name}", &identity.name)
            .replace("{media_title}", media),
        None => identity.name.clone(),
    }
```
What is wrong: The idle branch returns the raw `identity.name`, not the localized `title_idle_key` wording — the doc comment (56-63) explains the localization happens in the host, and doc 07 §8.3 step 4 accepts "Nova Player" as the idle title, so this is a documented v1 simplification, not a bug. Recorded so the next agent does not mistake it for a completed localization path. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| InstallerIdentity (struct) | 25-42 | REAL | — | plain data |
| AboutFacts (struct) | 44-54 | REAL | — | plain data |
| resolve_window_title | 64-77 | REAL | S4 | correct + injection-safe; localization deferred (F-003) |
| log_line_prefix | 81-84 | REAL | — | lowercase short_name |
| log_startup_line | 88-94 | REAL | — | name + version + reverse_dns |
| cli_version_line | 98-101 | REAL | — | "{name} {display_version}" |
| cli_usage_header | 104-107 | REAL | — | fixed English wording ("The {} command-line player") — not localized, matches doc 07 §8.2 wording row |
| installer_identity | 112-121 | REAL | — | {reverse_dns}.desktop shape matches committed tauri.conf.json identifier |
| about_facts | 125-133 | REAL | — | 1:1 field map |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_shipped_product_toml_resolves_lumen_identity | YES | loads the REAL product.toml (include_str — product drift breaks the test loudly) and asserts 9 exact surface values incl. version 0.1.3 |
| title_resolution_substitutes_only_the_legal_variables | YES | template without a slot → media ignored; exact string |
The rebrand acceptance suite (tests/rebrand/rebrand_acceptance.rs) adds 5 surface proofs against the Nova fixture with PASS/BLOCKED verdicts per row — honest reporting (installer build BLOCKED(infra), CLI host wiring BLOCKED are recorded, not claimed PASS).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every surface reads product-config at run time" | doc 07 §8.1:792 | FALSE in shipped tree | zero non-test callers; tauri title literal (F-001) |
| "Hosts that do not exist yet ... recorded as honest gaps" | runtime.rs:20-23 | REAL | header + acceptance report mark CLI/logger/installer as gaps |
| window title template {product_name}/{media_title} | doc 07 §2.2, §8.3 | REAL | unit test asserts "Big Buck Bunny — Lumen" from the real template |
| installer desktop id `{reverse_dns}.desktop` | runtime.rs:110-111 | REAL | test asserts "dev.lumen.desktop"; tauri.conf.json identifier matches shape |

## 7. Dead / duplicated / unreachable code
- All 7 functions: unreachable from shipped entry points (F-001) — kept alive by tests only.
- Duplication: `desktop::window_config::parse_window_table` (apps) vs product-config's schema-validated loader — two independent product.toml [window] parsers exist; the apps one is a compile-time include_str embed (a rebranded product.toml requires recompiling the shell).

## 8. Dependencies & flags
- std-only + crate::model types. No features, no unsafe. Crate-level: thiserror not used here.

## 9. Open questions for the next agent
- Does `apply_window_spec` (apps/desktop-tauri/src/app.rs:243) actually override the 1280x800 first-paint size with product.toml's 720? Verify on a GTK-capable host (UNVERIFIED here — sandbox lacks GTK per baseline).
- apps/cli's banner: which wave owns wiring cli_version_line into it? Check the CLI auditor's findings for a literal "Lumen" banner.
