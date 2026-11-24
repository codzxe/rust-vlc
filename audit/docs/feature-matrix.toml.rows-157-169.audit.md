# CLAIM AUDIT: docs/feature-matrix.toml rows 157-169 (config-feature tail + plugin-feature + platform-feature head)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 13 rows (rows 157-169: 8 config-feature, 4 plugin-feature, 1 platform-feature) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 0 / 5 (rows 158, 160, 161, 162, 169) / 0 / 0 / 0 / 1 (row 163 evidence citation FALSE — path deleted) / 0 — 7 not_started rows ACCURATE |
| Internal contradictions found | 2 (row 160 status vs description; row 163 evidence vs filesystem) |

## 1. Blunt verdict
7 of 13 rows claim `not_started` on all four platforms with `evidence = ""` and the claim is
accurate (censuses below; catalog ids exist with zero handlers for plugin rows 165-166; the
orphan `crates/plugin-host-wasm` behind row 167 is wired to nothing). The 6 substantive rows
have verifiable evidence strings: rows 158/160/161/162/169's evidence paths ALL exist and their
test suites pass in the orchestrator baseline. Two defects: (1) row 163 (`config-advanced-
folding`) claims `implemented` citing `ui/src/screens/settings/areas/advanced.tsx` — a file
DELETED in w26.k13 (the b02 fold; slug now redirects to `performance`) — the shipped
xtask feature-audit itself flags this exact row: "violation: 163: evidence cites missing path
`ui/src/screens/settings/areas/advanced.tsx` (R4)" (audit/runs/baseline/xtask-feature-audit.txt:11).
The folding capability is still real (32 `advanced = true` flags + `partitionAdvanced` +
`AdvancedDisclosure`), so the status is defensible but the citation is stale garbage. (2) Row 160
(`config-hotkey-editor`) claims `implemented` for "rebinding, conflict detection, Global column,
search" — Global column and search DO NOT EXIST in the editor (`rg "search|Global"
HotkeysEditor.tsx` -> 0 hits; `hotkey-global` -> 0 code hits repo-wide); the notes half-disclose
("Global column + search ride hotkey-global") but the status + description promise 4 things and
2 exist. Prior audit W15-A05 verdict for the editor itself is SHALLOW (mock store).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | status all "not_started", evidence "" — extensions area | :2194-2204 | ACCURATE (feature absent) | No plugin commands: `system.list-plugins` etc. are catalog ids only (crates/commands/src/catalog/ids.rs:166-168), zero handlers in apps/ (rg `ListPlugins` apps/ -> 0). UI area exists as honest-empty seam (ui/src/screens/settings/areas/extensions.tsx:19-26: "NO extensions command exists in the command catalog and product.toml [features] wasm-extensions = false ... so the list seam below is EMPTY"). |
| 2 | "implemented" w/l/m + evidence: rebrand.rs, product.toml/schema, config_validator | :2214-2215 | REAL-UNPROVEN (baseline tests) | All paths exist (rebrand.rs 1254 LOC, config_validator.rs 1007, product.toml 244, product.schema.json 300). Baseline: audit/runs/baseline/cargo-test-workspace.txt:5139-5145 (7 `rebrand_acceptance::surface_*_nova` tests ok) + :6688-6702 (15 unit tests ok). Prior audit audit/files/tools/xtask/src/cmd/rebrand.rs.audit.md:5 "REAL-UNPROVEN". |
| 3 | not_started, evidence "" — settings profiles | :2218-2228 | ACCURATE (no code) | `rg -il "profile" ui/src/screens/settings/` -> 0 profile-store/manager hits (only appearance/skin unrelated). No profiles module anywhere in ui/src or crates. |
| 4 | "implemented" w/l/m: hotkey editor "rebinding, conflict detection, Global column, search" | :2238-2239 | REAL-UNPROVEN but INFLATED | HotkeysEditor.tsx (267 LOC) + conflicts.ts + overrides.ts + generated/bindings.ts + ShortcutsScreen.tsx all exist; tests pass (baseline ui-test.txt:118 HotkeysEditor 8 tests, :52 ShortcutsScreen 14, :173 conflicts 7). BUT "Global column, search" absent: rg over HotkeysEditor.tsx -> 0; `hotkey-global` repo-wide (non-docs) -> 0. W15-A05:7 verdict "SHALLOW (polished editor wired to a MOCK store; no runtime consumer)". |
| 5 | "partial" w/l/m: 141-binding keymap + user layer + 9 macOS [VERIFY] overrides; persistence localStorage MOCK | :2250-2251 | ACCURATE (verified exact) | `awk` count: exactly 141 binding entries before `[platform_overrides.macos]`, 9 entries inside it (keymaps/default.toml:267-281). persistence.ts:4-8 verbatim: "MOCK persistence seam (card 4.E1)... the REAL write path is the app layer's — `config.set-setting`". ui-inventory.md:428 corroborates "141 bindings + 4 jump-size scalars + 9 macOS overrides". |
| 6 | "implemented" w/l/m: en-US 100% key coverage, pseudo-locale proof | :2262-2263 | REAL-UNPROVEN (baseline) | messages.ts (3017 LOC, 1448-key exact pin) + messages.test.ts + pseudo.ts + pseudo.render.test.tsx exist; baseline ui-test.txt:155 (messages 6 tests), :163 (pseudo.render 7 tests), :146 (i18n-no-literals lint rule 2 tests). Test pins `expect(keys.length).toBe(1448)` (messages.test.ts:29). |
| 7 | "implemented" w/l/m: advanced folding; evidence cites advanced.tsx | :2274-2275 | Evidence citation FALSE; capability real | `ui/src/screens/settings/areas/advanced.tsx` DOES NOT EXIST (ls of areas/ -> advanced.test.tsx only). xtask-feature-audit.txt:11 "violation: 163: evidence cites missing path `ui/src/screens/settings/areas/advanced.tsx` (R4)". Folding moved to per-area disclosure: schemas/settings.toml has 32 `advanced = true` flags; shared.tsx:97 `partitionAdvanced`; SettingsSections.tsx:126 `AdvancedDisclosure`. advanced.test.tsx:1-4 states the AREA "is DELETED — the b02 registry retired it"; its 2 tests cover only the slug redirect (baseline ui-test.txt:172). |
| 8 | not_started, evidence "" — defaults sourcing | :2278-2288 | ACCURATE (no code) | No config-walk default-seeding tooling: rg 'config-walk|vlcrc' crates/ tools/ -> 0 (only docs/inventory references). |
| 9 | not_started, evidence "" — builtin plugin registry | :2295-2306 | ACCURATE (ids only) | ids.rs:166-168 define the three command names; rg over apps/cli/src/dispatch.rs + apps/control-server/src/dispatch.rs for ListPlugins -> 0 arms. No plugin registry runtime exists. |
| 10 | not_started, evidence "" — codec plugin API | :2308-2318 | ACCURATE shipped | codec-api crate exists (contracts, per prior audits audit/files/crates/codec-api/) but plugin enumeration/enable/disable surface = absent; status honest for the feature as described. |
| 11 | not_started, evidence "" — WASM sandboxed extensions | :2320-2331 | ACCURATE shipped; orphan crate exists | crates/plugin-host-wasm/ exists (17 src files incl. supervisor.rs, fsjail.rs, limits.rs) but is a dependency of NOTHING (rg over all Cargo.tomls: only its own; product.toml:166 `wasm-extensions = false`). Notes do not mention this orphan. |
| 12 | not_started, evidence "" — addons finder/storage | :2333-2344 | ACCURATE (no code) | rg 'addons finder|addon-store|addons-storage' crates/ apps/ -> 0. |
| 13 | "partial" w/l/m: aout cpal seam, AC suites, ADR-005 flip point | :2360-2362 | ACCURATE (disclosed seam) | All evidence paths exist: cpal.rs:43 `pub const AUDIO_ADR005_ADMITTED: bool = false;`; BackendNotAdmitted (cpal.rs:58); lib.rs:138/143 NullBackend/UnsupportedBackend exports; tests exist and pass in baseline (cargo-test-workspace.txt:189-216 callback_budget/clock_truth/pcm_pipeline/sine_sink_hash all ok; :241 "PACKAGE audio-output exit_code=0"). Notes admit "no real-device evidence exists (typed marker refuses on every platform)". |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Row 160 status "implemented" for a dialog including "Global column, search" (2232-2234, 2238) | Notes (2240): "Global column + search ride hotkey-global" — i.e. depend on nonexistent work | The notes are true: HotkeysEditor.tsx has no search input and no Global column (rg -> 0); `hotkey-global` appears in zero code files. Status should be partial-vs-description. |
| Row 163 evidence: "ui/src/screens/settings/areas/advanced.tsx + advanced.test.tsx" (2275) | Filesystem: advanced.tsx deleted (w26.k13); advanced.test.tsx:1-4 says AREA "is DELETED" | The matrix cites a deleted file; shipped xtask feature-audit flags it (baseline:11). Feature survives as per-area disclosure — status OK, citation false. |

## 4. Fabrication / inflation indicators
- Row 163: evidence string cites a file that does not exist — the only row in this chunk the
  shipped toolchain itself flags (one of the 2 known R4 violations in xtask-feature-audit.txt).
  Not invented capability (32 advanced flags + disclosure are real), but the citation is unmaintained.
- Row 160: "implemented" for a 4-part description of which 2 parts (Global column, search)
  are absent — inflation partially disclosed in the same row's notes.
- Rows 158/162/169: no inflation found — evidence paths verified on disk, tests pass in
  baseline, and each row's notes disclose its open seams (runtime assertions parked;
  1448-key catalog; cpal refuses everywhere).

## 5. Matrix rows (row id, platform statuses claimed, evidence strings, code actually found, executed?, verdict)

| Row | id | Statuses claimed (w/l/m/web) | evidence (verbatim key parts) | Code actually found | Executed? | Verdict |
|---|---|---|---|---|---|---|
| 157 | config-area-extensions | ns/ns/ns/ns (2202) | "" (2203) | UI area screen exists honest-empty (extensions.tsx, disabled toolbar); no plugin commands (ids only) | Nothing to execute; parity config-feature web 0/20 | Claim ACCURATE; UI surface staged |
| 158 | config-rebrand | impl/impl/impl/ns (2214) | "tools/xtask/src/cmd/rebrand.rs (xtask rebrand command — product.toml-driven identity assertions); product schema product.toml + product.schema.json validated by ... config_validator.rs" (2215) | All 5 paths exist; rebrand.rs REAL-UNPROVEN per W01-A08 audit; runtime acceptance assertions parked #[ignore] (disclosed) | Baseline cargo-test :5139-5145 (7 acceptance tests ok), :6688+ (15 unit ok) | Claim REAL-UNPROVEN; notes disclose pending 4.H2 pass |
| 159 | config-profiles | ns/ns/ns/ns (2226) | "" (2227) | None | Nothing | Claim ACCURATE; feature absent |
| 160 | config-hotkey-editor | impl/impl/impl/ns (2238) | "ui/src/screens/settings/areas/hotkeys/HotkeysEditor.tsx (+ HotkeysEditor.test.tsx, conflicts.ts conflict detection, overrides.ts user layer, generated/bindings.ts from keymaps/default.toml); browser surface ... ShortcutsScreen.tsx" (2239) | All paths exist; editor = rebinding + conflict detection only; Global column + search absent; persistence = localStorage mock (W15-A05: editor SHALLOW) | Baseline ui-test :118 (8 tests), :52 (14), :173 (7), :172 (2) | Evidence VERIFIED; status INFLATED vs description (2 of 4 capabilities missing) |
| 161 | config-keymap-layers | partial/partial/partial/ns (2250) | "keymaps/default.toml (141-binding seed, per-entry provenance, [platform_overrides.macos] block); user layer ... overrides.ts ...; persistence seam ui/src/screens/settings/persistence.ts (localStorage MOCK — real config.set-setting bridge open)" (2251) | 141 bindings counted exact; 9 macOS overrides counted exact; persistence.ts verbatim admits MOCK (:4-8) | awk counts + sed reads (this audit) | Claim ACCURATE, honestly partial |
| 162 | config-localization | impl/impl/impl/ns (2262) | "ui/src/i18n/messages.ts (+ messages.test.ts, en-US key coverage) + pseudo.ts pseudo-locale proof (+ pseudo.render.test.tsx); consumers throughout ui/src (useMsg)" (2263) | messages.ts 1448 keys pinned; pseudo.ts real transform; lint rule i18n-no-literals exists | Baseline ui-test :155/:163/:146 all pass | Claim REAL-UNPROVEN (baseline-executed) |
| 163 | config-advanced-folding | impl/impl/impl/ns (2274) | "schemas/settings.toml (advanced = true flags ...) + AdvancedArea registered as the 'advanced' area in ui/src/screens/settings/areas/index.tsx (ui/src/screens/settings/areas/advanced.tsx + advanced.test.tsx)" (2275) | advanced.tsx MISSING (flagged by xtask feature-audit R4); AdvancedDisclosure fold real (SettingsSections.tsx:126, shared.tsx:97); 32 advanced=true in schema; advanced.test.tsx = redirect test only | Baseline ui-test :172 (2 tests); awk counted 32 flags | Status defensible; evidence FALSE citation (deleted path) |
| 164 | config-defaults-source | ns/ns/ns/ns (2286) | "" (2287) | None | Nothing | Claim ACCURATE |
| 165 | plugin-builtin-registry | ns/ns/ns/ns (2304) | "" (2305) | Catalog ids only (ids.rs:166-168); zero dispatchers; extensions.tsx empty seam | Nothing | Claim ACCURATE |
| 166 | plugin-codec-api | ns/ns/ns/ns (2316) | "" (2317) | codec-api contracts exist but no plugin registry/enumeration | Nothing | Claim ACCURATE (as scoped) |
| 167 | plugin-wasm-extensions | ns/ns/ns/ns (2329) | "" (2330) | Orphan crate plugin-host-wasm (17 files) wired to nothing; product.toml:166 wasm-extensions = false | Nothing | Claim ACCURATE shipped; orphan crate undisclosed |
| 168 | plugin-addon-store | ns/ns/ns/ns (2342) | "" (2343) | None | Nothing | Claim ACCURATE |
| 169 | platform-aout-cpal | partial/partial/partial/ns (2360) | "crates/audio-output/** (ADR-005 seam: AudioBackend trait + NullBackend + typed UnsupportedBackend stubs, AC suites sine_sink_hash/callback_budget/clock_truth); crates/audio-output/src/cpal.rs (AUDIO_ADR005_ADMITTED flip point ...)" (2361) | All paths verified; AUDIO_ADR005_ADMITTED=false (cpal.rs:43); AC suite files exist; W08-A02: crate REAL, NullBackend consumed by apps/cli | Baseline cargo-test :189-216 ok, package exit 0 (:241) | Claim ACCURATE partial-seam, fully disclosed |

ns = not_started, impl = implemented. Line numbers are feature-matrix.toml raw lines.

## 6. Row-count / boundary proof
`rg -c '^\[\[feature\]\]' docs/feature-matrix.toml` -> 252 rows; 157th match at line 2194
(`config-area-extensions`), 170th at 2364 (`platform-aout-wasapi`) — chunk = lines 2194-2363,
13 rows exactly. Parity-report category totals consistent: config-feature 20 rows (11/20
implemented per platform — rows 158/160/162/163 plus 7 from earlier chunks), plugin-feature
4 rows 0/4, platform-feature 18 rows 0/18 (partial not counted) —
audit/runs/baseline/xtask-parity-report.txt:13,23,19.
