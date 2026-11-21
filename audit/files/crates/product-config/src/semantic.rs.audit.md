# AUDIT: crates/product-config/src/semantic.rs
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-21 |
| Lines / functions | 565 / 22 (incl. 3 in-file tests; external suite tests/semantic.rs reviewed for honesty) |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point reaches any of it) |
| Reachable from shipped entry point? | NO — proof below |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | `rg -n "validate_semantics\|cross_field_checks\|SemanticInputs\|FeatureRegistry" --glob '!audit/**'` (callers: tests only + merge.rs:217); `rg -n "product-config" --glob '**/Cargo.toml'` (single dependent: apps/control-server); `rg -n "resolve_layers\|merge::layer\|load_partial_str"` (tests only); `sed -n` on all quoted ranges; read tools/xtask/src/cmd/config_validator.rs, src/load.rs, src/merge.rs, src/lines.rs, src/diagnostic.rs |

## 1. One-paragraph verdict
The semantic pass is genuine, carefully written, and mutation-sensitive-tested — and wired to nothing shipped. `validate_semantics` is called only by the crate's own test suite (`crates/product-config/tests/semantic.rs:22,289`); the one production caller of any symbol here is `merge.rs:217` (`cross_field_checks`), but `resolve_layers`/`layer` themselves have no caller outside tests. The workspace's only dependent on `product-config` is `apps/control-server`, which calls `load_file` (parse → schema → typed model) and never the semantic stage. The file header (lines 7–10) claims "the `cargo xtask config-validator` tool (card 1.E2 wiring) supplies the real listings" — that wiring does not exist: xtask is std-only by policy and `config_validator.rs` reimplements a strict subset of these checks with its own toml_lite, explicitly punting F1/F2 and magic bytes. Doc 07 §835 (C4) still advertises F1/F2 as config-validator coverage. Result: the rules engine that would catch "config declares features the engine doesn't have" (the exact Wave-8 `audio.set-eq`-class disease) is production-dead, and the committed `product.toml` is only ever checked with `SemanticInputs::none()` (tests/semantic.rs:289), which by construction skips every cross-file rule.

## 2. What actually works (verified)
— verified by `sed -n` trace of every rule function + baseline #3 (`audit/runs/baseline/test-workspace.log.txt`, 1681 passed): the in-file tests `pascal_case_derives_identifiers`, `path_problems_are_detected`, `magic_bytes_detect_formats` and the 16-test external suite `tests/semantic.rs` all pass and assert concrete diagnostic content (rule text, expected/found, line numbers via `marker_line`), so they fail under mutation.
— verified by baseline #10 (`audit/runs/baseline/xtask-config-validator.log.txt`, exit 1): the *tool-side* reimplementation flags 4 missing brand icons — corroborating that the tool, not this pass, is what runs.

## 3. Findings

### F-001 [S2]
Where: crates/product-config/src/semantic.rs:7-10, 141-158
Code:
```
//! registry) is an explicit [`SemanticInputs`] field: the crate is the
//! *mechanism*, the `cargo xtask config-validator` tool (card 1.E2 wiring)
//! supplies the real listings. `SemanticInputs::none()` runs only the pure
//! in-document checks.
```
```
pub fn validate_semantics(
    file: &str,
    config: &ProductConfig,
    inputs: &SemanticInputs,
    lines: &LineIndex,
) -> Vec<Diagnostic> {
```
What is wrong: The claimed production wiring is fictitious. `rg -n "validate_semantics"` → only `crates/product-config/tests/semantic.rs:22,289`. xtask cannot call it: `tools/xtask/Cargo.toml` is "Std-only by design: zero external dependencies", and `tools/xtask/src/cmd/config_validator.rs:19-21` states "Rule F1/F2 ... needs the registry surface; v0 checks that every `features.*` value is boolean" and "Magic-byte checks for brand assets are the crate's, not this tool's" — i.e. F1/F2 (config-vs-engine-registry intersection), brand magic bytes, and catalog joins are enforced against the committed `product.toml` **nowhere**: the crate's own committed-file test passes `SemanticInputs::none()` (tests/semantic.rs:289), which skips `feature_checks` (semantic.rs:405), `asset_checks` (semantic.rs:433-436), `injected_enums` listings, and `catalog_checks`.
Why it matters: F1/F2 is the declared firewall against "config declares a capability the engine does not have" — the exact class of defect Wave 8 found live (`audio.set-eq` awaiting a nonexistent backend). Doc 07 §835 (C4) claims `cargo xtask config-validator` covers "feature-exposure/registry intersection (F1/F2)"; it does not (baseline #10 output lists only 4 icon-existence violations, and the tool's own header admits the gap). The whole semantic stage is a paper gate for the shipped tree.
What correct looks like: config_validator.rs either shells through this crate (drop the std-only constraint for the validator, or move the pass into xtask) and supplies `SemanticInputs` with real `themes/locales/profiles` listings, `reference_catalog_keys` from `locales/en-US.toml`, and a `FeatureRegistry` derived from the engine's actual registry — so `root_product_toml_passes_semantic_none` becomes `...passes_semantic_full`.
Confidence: HIGH — Verification: read (caller greps) + baseline #10 cite.

### F-002 [S3]
Where: crates/product-config/src/semantic.rs:243-258
Code:
```
        if i > 0 {
            let prev = match entries.get(i - 1) {
                Some(p) => p,
                None => continue,
            };
            if entry.from != prev.to {
```
What is wrong: The chain rule enforced here is only "each entry continues the previous one" (i>0) plus per-entry `from < to`. Nothing checks that `entries[0].from == 1`, although the emitted hint asserts "migration entries must form one unbroken ascending chain from schema version 1 (doc 07 §2.11, §3.3)" (semantic.rs:254). A config with `entries = [{from: 3, to: 4}]`, `schema_version = 4` passes every migration check.
Why it matters: The diagnostic text promises a rule the code does not enforce; version-skipped configs are accepted by this pass. (Blast radius today is the test suite only — see F-001 — but the code is the declared reference implementation of doc 07 §2.11.)
What correct looks like: `if entries.first().is_some_and(|e| e.from != 1) { ctx.push("migrations.entries.0.from", ...) }`.
Confidence: HIGH — Verification: read; mutation reasoning (no first-entry check exists anywhere in 229-271).

### F-003 [S3]
Where: crates/product-config/src/semantic.rs:259-261
Code:
```
    let max_to = entries.last().map(|e| e.to);
    let expected_version = max_to.unwrap_or(1);
    if config.schema_version != expected_version {
```
What is wrong: `expected_version` is the **last** entry's `to`, not the **highest** — contradicting both the line-229 comment ("schema_version == max(to)") and doc 07 §428 ("`schema_version == max(migrations.entries.to)`"). Entries are read in document order (TOML array), so a malformed-but-individually-legal list like `[{from:1,to:5},{from:2,to:3}]` (gap diagnostic fires) computes expected=3 while the max target is 5, and the schema_version diagnostic demands the wrong value.
Why it matters: On precisely the configs that are already broken, the validator's `expected:` field misleads the fixer. Rule text and code disagree.
What correct looks like: `entries.iter().map(|e| e.to).max().unwrap_or(1)`, or reject non-contiguous chains before deriving the expected version.
Confidence: HIGH — Verification: read; doc 07 §428 quote checked (`rg -n "max\(migrations" docs/07-config-driven-system.md`).

### F-004 [S3]
Where: crates/product-config/src/semantic.rs:126-138
Code:
```
pub fn pascal_case(name: &str) -> String {
    let mut out = String::new();
    for word in name.split([' ', '-', '_']) {
```
What is wrong: Third copy of `pascal_case` in the tree (also `tools/xtask/src/cmd/config_validator.rs:607`, `tests/rebrand/rebrand_skeleton.rs:162`) with **divergent semantics**: this one splits only on space/hyphen/underscore and copies every other byte verbatim, so `identity.name = "Lumen 2.0"` derives `Lumen2.0` — a dotted string that is not a legal Windows AUMID / macOS bundle-id suffix. The cross-field rule at 177-207 then *enforces* that all four platform ids equal this possibly-illegal derived value; the derived value's own shape is never validated.
Why it matters: The rule can force configs to copy a garbage identifier, and the xtask twin gives opposite verdicts for the same file (W01 already recorded the divergence in `audit/files/product.schema.json.audit.md` F-003; this file is where the crate half lives).
What correct looks like: one `pascal_case` (and a `is_legal_id_suffix` check) shared by both validators, plus a semantic diagnostic when the derived id contains characters illegal on any target platform.
Confidence: HIGH — Verification: read + `rg -n "fn pascal_case"` (3 hits) + prior W01 finding cross-checked.

### F-005 [S4]
Where: crates/product-config/src/semantic.rs:447-460
Code:
```
    for (key_path, rel, kind) in assets {
        let path = Path::new(base_dir).join(rel);
        match std::fs::read(&path) {
```
What is wrong: Whole-file read to inspect at most 8 magic bytes (`magic_ok` consumes `bytes.iter().take(8)` for the error path, prefixes otherwise). A large `logo.svg` is fully buffered for a prefix check.
Why it matters: Wasted allocation in a validator; trivial today (icons are small), but the pattern scales badly if the pass is ever wired to bigger assets.
What correct looks like: `File::open` + `read_exact(&mut [0u8; 8])`, treating a short read as a failed magic check.
Confidence: HIGH — Verification: read.

### F-006 [S4]
Where: crates/product-config/src/semantic.rs:369-381
Code:
```
        if let Some(available) = listing
            && !available.contains(&value)
        {
            ctx.push(
                key_path,
                format!(
                    "must match a `{dir}{token}.toml` file (injected enum from the {dir} listing)"
                ),
                format!("one of: {}", available.join(", ")),
```
What is wrong: An explicitly-supplied **empty** listing (`Some(vec![])`) yields a violation for every default with the useless expected text "one of: ". The xtask twin deliberately treats an empty listing as an honest note ("empty listing = honest note, not a violation — those planes are parked, doc 07 §9 C4", config_validator.rs:12). The two implementations of the same §3.3 rule disagree on the empty case.
Why it matters: One more latent divergence between crate and gate that will bite the moment F-001's wiring is attempted with real (possibly empty) `profiles/` listings.
What correct looks like: empty listing → skip (or emit the parked-plane note) in both implementations, per doc 07 §9 C4.
Confidence: HIGH — Verification: read + config_validator.rs:12 quote.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SemanticInputs` (struct) | 20-36 | REAL | — | Plain input bundle; all `Option` fields gate a check group; `base_dir: None` skips fs entirely (documented, matches behavior) |
| `SemanticInputs::none()` | 38-43 | REAL | — | `default()`; used by tests; nothing ignores inputs silently except by documented design |
| `FeatureRegistry` (struct) | 47-50 | REAL (test-only callers) | — | `from_status` called only from tests/semantic.rs:201,218 — production-dead (F-001) |
| `FeatureStatus` | 53-56 | REAL | — | One bool; no ignored fields |
| `FeatureRegistry::from_status` | 60-71 | REAL | — | Collects into BTreeMap (dup ids silently last-wins — irrelevant, test-only input) |
| `FeatureRegistry::status` | 73-75 | REAL | — | Copy out of map; no panic paths |
| `FeatureRegistry::known_ids` | 77-79 | REAL | — | Allocates per F1 diagnostic; fine for a validator |
| `RESERVED_NAMES` | 85 | REAL | — | `["VLC", "VideoLAN", "vlc"]`; used at 164; case-sensitive — `Vlc`/`videoLAN` pass (schema `not.enum` shares the limitation; consistent pair) |
| `quoted` | 91-93 | REAL | — | `format!("{value:?}")` — matches structural pass convention |
| `Ctx` (struct) + `Ctx::push` | 95-121 | REAL | — | Always `Severity::Error`, `Stage::Semantic`; `line_of` fallback to deepest ancestor (lines.rs:64-76) can attribute a cross-field violation to the section header line — acceptable, documented in lines.rs |
| `pascal_case` | 126-138 | REAL | S3 (F-004) | Unicode-correct via `to_uppercase`; no illegal-char validation of result; triplicated/divergent across tree |
| `validate_semantics` | 141-158 | REAL but UNREACHABLE in production | S2 (F-001) | Test-only callers; orchestrates all five passes |
| `cross_field_checks` | 162-305 | REAL (production caller merge.rs:217, itself test-only-reached) | S3 (F-002/F-003 inside) | reserved name, derived ids, geometry, migrations, schema_version, telemetry→privacy, brand path shape — all traced correct except F-002/F-003 |
| `unsafe_path_problem` | 307-326 | REAL | — | Catches empty/absolute/backslash/drive-letter/`..`; `~` and `.` segments pass (harmless under `Path::join`); NUL byte → fs error later caught as missing file; no panics |
| `injected_enums` | 328-383 | REAL | S4 (F-006) | Empty-listing divergence; `None` listing defers silently (documented, tested at tests/semantic.rs:162-173) |
| `catalog_checks` | 385-402 | REAL | — | Exactly 2 keys (`description_key`, `title_idle_key`) — matches the model's only `*_key` fields (model.rs:50,103); `None` defers |
| `feature_checks` | 404-430 | REAL | S2 via F-001 | F1 unknown-id, F2 exposed-but-unimplemented — correct logic, no production input ever supplied |
| `asset_checks` | 432-472 | REAL | S4 (F-005) | Whole-file read; error message embeds absolute path (fine for a repo tool) |
| `AssetKind::name` / `magic` | 482-499 | REAL | — | Display strings only |
| `AssetKind::magic_ok` | 502-524 | REAL | — | ICO `00 00 01 00`, ICNS `icns`, PNG 8-byte sig, SVG BOM+whitespace-then-`<` — all correct; UTF-16 BOM correctly rejected (0xFF ≠ whitespace ≠ `<`) |
| `pascal_case_derives_identifiers` (test) | 531-536 | REAL | — | Concrete assertions on 3 inputs; mutation-sensitive |
| `path_problems_are_detected` (test) | 538-546 | REAL | — | 5 positive + 1 negative case; mutation-sensitive |
| `magic_bytes_detect_formats` (test) | 548-564 | REAL | — | Positive+negative per format; mutation-sensitive |

## 5. Test honesty (in-file + external suite for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| semantic::tests::pascal_case_derives_identifiers | YES | Asserts 3 exact outputs; deleting the capitalize loop fails it |
| semantic::tests::path_problems_are_detected | YES | Each `unsafe_path_problem` branch has a positive case; removing a branch fails the matching assert |
| semantic::tests::magic_bytes_detect_formats | YES | Wrong prefix constant or flipped `ok` fails an assert (e.g. `Ico.magic_ok(b"not-ico")` expects Err) |
| tests/semantic.rs (16 tests) | YES | Each asserts `rule`/`expected`/`found`/`line` of exactly one diagnostic (`one()` enforces count==1); fixtures are dedicated TOML files per rule |
| tests/semantic.rs::root_product_toml_passes_semantic_none | NO (for cross-file rules) | Runs `SemanticInputs::none()` — by construction cannot catch F1/F2/asset/catalog regressions on the committed file; proves only the in-document rules. This is the committed-file blind spot of F-001 |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the `cargo xtask config-validator` tool (card 1.E2 wiring) supplies the real listings" | semantic.rs:8-9 (module doc) | FALSE | xtask std-only, no product-config dep; config_validator.rs:19-21 punts F1/F2+magic; no caller of `validate_semantics` outside tests |
| "any shipped config failing its schema or semantic pass: … feature-exposure/registry intersection (F1/F2)" runs in config-validator | doc 07 §835 (C4) | FALSE for F1/F2 | Same proof + baseline #10 output (only 4 icon violations, tool header admits gap) |
| "schema_version == max(migrations.entries.to)" | doc 07 §428 | SHALLOW | Implemented as `entries.last().to` (F-003) — equal only for well-formed chains |
| "migration entries must form one unbroken ascending chain from schema version 1" | semantic.rs:254 hint text | SHALLOW | No first-entry check (F-002) |
| "One unit test per schema rule … F1 (unknown feature), F2 (exposed-but-unimplemented) … Golden diagnostics" | doc 07 §862 | REAL (crate side) | tests/semantic.rs implements exactly this and passes (baseline #3) — but proves the checker, never the real registry (tests/semantic.rs:201-222 fixtures) |

## 7. Dead / duplicated / unreachable code
- `validate_semantics`, `injected_enums`, `catalog_checks`, `feature_checks`, `asset_checks`, `FeatureRegistry*`: no production caller (F-001).
- `cross_field_checks`: reachable only via merge.rs:217, whose own entry points (`layer`, `resolve_layers`) are test-only (rg over non-audit tree: callers at crates/product-config/tests/merge.rs only).
- `pascal_case` triplication: semantic.rs:126 / config_validator.rs:607 / rebrand_skeleton.rs:162 (F-004).

## 8. Dependencies & flags
- File uses only `std` + crate-local modules (`diagnostic`, `lines`, `model`). No features gate it; the crate compiles into `apps/control-server` only (workspace `Cargo.toml` grep: single path dep at apps/control-server/Cargo.toml:43). `lumen` CLI / `lumen-tauri` do not link this crate at all — hence UNREACHABLE.

## 9. Open questions for the next agent
- merge.rs / lib.rs / runtime.rs owners: `resolve_layers`' six-layer model — same production-deadness applies to the whole merge module; confirm in your own audit.
- Whoever owns doc 07 §835/§862: reconcile the C4 gate description with config_validator.rs's honest-limits header, or land the 1.E2 wiring.
- `RESERVED_NAMES` case-sensitivity vs doc 07 §3.3(a) wording ("clean-room rule") — schema `not.enum` shares the same three exact strings; if "Vlc" must be reserved too, both need it (low priority).
