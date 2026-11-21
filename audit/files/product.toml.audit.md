# AUDIT: product.toml
| Field | Value |
|---|---|
| Auditor | W01-A08 |
| Date | 2026-11-21 |
| Lines / functions | 244 / 11 sections (+ ~40 config keys) |
| File verdict | FALSE (self-claims contradict shipped code; see F-001..F-003) |
| Reachable from shipped entry point? | PARTIAL — build-time only: `apps/desktop/src/window_config.rs:357` `include_str!("../../../product.toml")` (compile-time). NOT read at runtime by the shipped `lumen` CLI (no `product-config` dep in apps/cli/Cargo.toml). Consumed by xtask tooling (rebrand, lint-literals, check-identity, web-budget, config-validator) and packaging scripts. |
| Findings | S0: 3 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `rg -n "product\\.toml" -l` (consumers); `rg -n "config-validator" scripts/ .github/` → NOT WIRED anywhere; `ls assets/brand/ themes/ profiles/` → missing; `python3` JSON read of product.schema.json `features` → `additionalProperties: {type: boolean}`; sed -n verifications of every quote below; baseline INDEX.md runs #10, #16. |

## 1. One-paragraph verdict
product.toml bills itself as the "Single source of truth", validated so that "a violation
is a build-breaking error", where "you cannot even name an unimplemented feature" (rule
F1). All three claims are false. The validator that reads this file is wired into NO
workflow or gate script while the file currently FAILS its own validation (4 missing
brand assets — baseline run #10, exit 1). Rule F1 is unimplemented in every validation
surface that runs (schema accepts any boolean key; xtask config-validator v0 checks
booleans only; the `validate_semantics` F1 code path is test-only). The feature table
asserts `audio-output = true # default audio device output` while the shipped CLI's
audio path is a null sink that discards samples. The runtime values it claims to own
(queue caps, network cache, engine feature list) are duplicated as hard-coded mirror
constants in the CLI because nothing loads this file at runtime.

## 2. What actually works (verified)
- `web.size_budgets` IS read from this file by `cargo xtask web-budget --check`
  (`tools/xtask/src/cmd/web_budget.rs:299` "Budgets (product.toml [web.size_budgets]
  via toml_lite)") — baseline INDEX.md #16: PASS (`gate-web_budget___check.log`).
- xtask config-validator DOES run and correctly fails on this file's dangling brand
  assets — baseline INDEX.md #10: exit 1, 4 violations (icons). The tool is real; its
  enforcement is not (F-001).
- Both msgid keys exist in the reference catalog: `locales/en-US.toml:70`
  `"product.description"`, `:75` `"window.title.idle"` (rg) — consistent with
  baseline #10 reporting only the 4 icon violations.
- `streaming-http = true` (line 157) corresponds to a real, reachable code lane:
  `apps/cli/src/demux_host.rs:427` `"http" | "https" => open_http_source(url, fetch)?`
  in the CLI open path. REAL-UNPROVEN (code traced; no baseline run exercises an
  http open).
- Structural schema validation is real: product.schema.json required keys match this
  file's sections (verified by JSON read); baseline #10 parse succeeded (violations
  were cross-file, not structural).

## 3. Findings

### F-001 [S0] "Build-breaking error" claim is false: validator unwired, file ships failing
Where: product.toml:4-7 (cross-evidence: baseline #10; `.github/workflows/*`, `scripts/full_gate.sh`)
Code:
```
# Single source of truth for product decisions (MP §5). Validated by
# product.schema.json via `cargo xtask config-validator`; a violation is a
# build-breaking error with an actionable diagnostic (MP §5), never a silent
# fallback. Do not hand-edit files generated from this one (§8, §9).
```
What is wrong: "a violation is a build-breaking error" is false. `rg -n
"config-validator" scripts/ .github/` finds ZERO invocations: ci.yml runs xtask
check-architecture, fixtures, schema-gen, codegen, lint-literals, check-identity,
rebrand, sbom, web-budget — not config-validator. full_gate.sh (xtask block, lines
29-71) does not run it either. Meanwhile this very file FAILS the validator today —
baseline INDEX.md #10: `cargo xtask config-validator` exit 1, "4 violations: brand
icon assets (`icon.ico`, `icon.icns`, `icon-512.png`, `logo.svg`) do not exist" — and
no build, gate, or CI job breaks. `scripts/release/gen-icon.py:5` itself admits "those
brand assets do not exist yet in this repository".
Why it matters: the SSOT ships permanently red with zero enforcement; the promised
safety property ("never a silent fallback") is itself a silent fallback — violations
exist right now and nothing notices.
What correct looks like: a ci.yml step `cargo run -p xtask --locked -- config-validator`
plus the four `assets/brand/` files (or placeholder assets) so the claim is true.
Confidence: HIGH — Verification: read + baseline #10 cited; greps listed in header.

### F-002 [S0] Rule F1 claim is false: you CAN name (and enable) unimplemented features
Where: product.toml:135-140 (cross-evidence: product.schema.json `features`;
tools/xtask/src/cmd/config_validator.rs:19-20; crates/product-config/src/semantic.rs:404-405)
Code:
```
# Semantics (doc 05 §6.4): effective availability =
#     engine build features  ∩  this table  ∩  host capability (e.g. web).
# A feature id that is not in the engine's static feature registry is a
# VALIDATION ERROR here (rule F1) — you cannot even name an unimplemented
# feature, let alone enable it. UIs grey out non-exposed commands from data
```
What is wrong: every validation surface that actually runs accepts arbitrary feature
ids. (a) product.schema.json: `"features": {"type": "object", "additionalProperties":
{"type": "boolean"}}` — any key, boolean value. (b) xtask config_validator.rs:19-20:
```
//! * Rule F1/F2 (feature ids vs the engine feature registry) needs the
//!   registry surface; v0 checks that every `features.*` value is boolean.
```
(c) The only real F1 implementation, `crates/product-config/src/semantic.rs:404-405`:
```
fn feature_checks(ctx: &mut Ctx<'_>, config: &ProductConfig, inputs: &SemanticInputs) {
    if let Some(registry) = &inputs.feature_registry {
```
is dead in production: `validate_semantics` is called ONLY from
`crates/product-config/tests/*` (rg), never with the real engine registry from any
binary, gate, or CI step. The "engine's static feature registry" itself is a
hard-coded string list in `apps/cli/src/host.rs:1638-1640`, never fed to any
validator. The schema's own `$comment` ("unknown feature ids are rejected in the
semantic pass") repeats the false claim.
Why it matters: the file's security-shaped guarantee — "Config can never enable a
missing codec, protocol or hardware capability" (lines 132-133) — does not exist.
A `features.h264 = true` row would pass every running check.
What correct looks like: supply the engine registry (the host.rs feature list) to
`validate_semantics` inside the config-validator xtask and in CI.
Confidence: HIGH — Verification: read (schema JSON, both .rs files, rg for callers).

### F-003 [S0] `audio-output = true # default audio device output` — shipped binary has no audio device output
Where: product.toml:150 (cross-evidence: apps/cli/src/host.rs:79,1639;
apps/cli/src/pipeline_host.rs:52; apps/cli/Cargo.toml:13; baseline engine-info)
Code:
```
core-playback = true            # session open/play/pause/seek/stop of local files
local-file = true               # file:// source access
audio-output = true             # default audio device output
video-render = true             # video presentation (video-in-webview prototype, MP §7)
```
What is wrong: the shipped `lumen` CLI's entire audio path runs over the null device:
`apps/cli/src/host.rs:79` `use audio_output::null::NullDevice;` and
`apps/cli/src/pipeline_host.rs:52` `use audio_output::null::{NullDevice, NullSink};`
— the CLI imports nothing from the real `cpal` backend that exists in
crates/audio-output/src/cpal.rs. `apps/cli/Cargo.toml:13` admits "the audio path over
the null device — no audio hardware in this sandbox, ADR-005 era". The engine's own
report hard-codes `"audio.null-device"` (host.rs:1639; baseline `lumen engine-info` →
`features: first-vertical:avi-mjpeg-pcm, audio.null-device,
video.software-compositor`). The one end-to-end lane (AVI MJPEG+PCM, baseline
rc-smoke) decodes PCM into a discard sink — no audible output exists.
Why it matters: this row violates the file's own quoted rule (lines 132-133: "Feature
exposure lists only implementations that exist. Config can never enable a missing
codec, protocol or hardware capability"). A user reading the SSOT believes audio
device output ships; it does not.
What correct looks like: `audio-output = false # null-device only until the cpal
backend is wired (ADR-005 follow-up)`.
Confidence: HIGH — Verification: read (imports, Cargo.toml comment) + baseline
engine-info output.

### F-004 [S2] "Single source of truth"/"reads from here at runtime" — false: shipped CLI duplicates the values as mirror constants
Where: product.toml:13-15 and 201-204 (cross-evidence: apps/cli/src/demux_host.rs:69-73;
crates/streaming/src/ranged.rs:32-35; apps/cli/src/host.rs:1562-1569)
Code:
```
# Rebrandable without touching code: `cargo xtask rebrand` consumes these
# values (§8). Every surface that shows a name reads from here at runtime;
# packaging metadata is regenerated from here at build time.
```
and
```
queue_packets_audio = 1024      # bounded audio packet queue (doc 03 §4). schema 64..=8192
queue_packets_video = 512       # bounded video packet queue. schema 64..=4096
```
What is wrong: the shipped CLI never loads product.toml (no `product-config` dep in
apps/cli/Cargo.toml). The queue caps are duplicated as hard-coded mirror constants,
`apps/cli/src/demux_host.rs:71-73`:
```
pub const PRODUCT_QUEUE_PACKETS_AUDIO: u64 = 1024;
/// The video half of the same mirror.
pub const PRODUCT_QUEUE_PACKETS_VIDEO: u64 = 512;
```
(the comment above them: "when the product-config load lands it replaces this pair
1:1 ... `[PROPOSAL]`"). Same pattern for the network cache:
`crates/streaming/src/ranged.rs:32-34` "The default window mirrors `product.toml`'s
`limits.cache_network_mib = 64` ... Live product-config plumbing lands with the http
open-path card". Settings defaults that reference product.toml are reported as
placeholder strings, `apps/cli/src/host.rs:1569`:
`DefaultValue::Ref(path) => SettingValue::String(format!("-> product.toml {path}"))` —
the CLI literally answers with "-> product.toml limits.x" instead of a value.
Why it matters: editing this "single source of truth" changes nothing at runtime in
the shipped binary; the file and the code can (and will) drift apart silently — no
test or gate compares the mirrors to the file.
What correct looks like: load product.toml through `crates/product-config` (which
exists, with `load.rs`/`runtime.rs`) in the CLI host, or add a gate asserting the
mirror constants equal the file's values.
Confidence: HIGH — Verification: read all cited lines; rg for product-config dependents.

### F-005 [S3] `theme`/`profile` point at files that do not exist; "must match" is unenforced
Where: product.toml:106-113 (cross-evidence: `ls themes/` → only `generated/`,
`tokens.json`; `profiles/` absent; config_validator.rs:469-472)
Code:
```
# Theme id: must match themes/<id>.toml. The design package is dark-first
# (design/README.md), so the product default is dark; light ships alongside.
theme = "dark"
```
What is wrong: `themes/` contains no `*.toml` at all (`themes/dark.toml` does not
exist), and the `profiles/` directory does not exist, yet `profile = "default"`
claims "must match profiles/<id>.toml". The validator deliberately downgrades both to
a note, `tools/xtask/src/cmd/config_validator.rs:469-472`:
```
                None => notes.push(format!("{key}: `{dir}/` does not exist yet — enum injection skipped (parked plane, doc 07 §9 C4)")),
                Some(ids) if ids.is_empty() => notes.push(format!(
                    "{key}: `{dir}/` has no *.toml candidates — enum injection skipped until the plane lands (doc 07 §9 C4)"
                )),
```
Why it matters: the "must match" contract is a no-op today; a reader cannot tell
`theme = "dark"` resolves to anything. (The downgrade is at least documented in the
tool output as a note.)
What correct looks like: either ship `themes/dark.toml` + `profiles/default.toml`, or
state "parked plane" in product.toml itself the way the tool does.
Confidence: HIGH — Verification: read + ls.

### F-006 [S3] `brand.accent_color` is dead for rendering and contradicts the shipped accent
Where: product.toml:91-95 (cross-evidence: themes/tokens.json:22; rg "4f8cff" in
ui/src and themes/ → no matches)
Code:
```
[brand]
# Accent color. MUST equal the theme token `color.accent.primary` once the
# token extraction lands (doc 08); until then this literal is provisional.
# [VERIFY] against design package token extraction.
accent_color = "#4f8cff"
```
What is wrong: nothing renders this value — `#4f8cff` appears in no UI/theme source
(only in xtask lint fixtures). The shipped default accent is orange:
`themes/tokens.json:22` `"default-accent": "orange"`. The key's only live consumer is
lint tooling (`lint_literals.rs:179`, `check_identity.rs:24`, `rebrand.rs:411`) as an
anti-hardcoding reference.
Why it matters: the SSOT's declared brand color (blue) is not the color the product
shows (orange); a rebrand that trusts this key changes nothing visible. The comment
is honest about being provisional, which keeps this at S3.
What correct looks like: either wire accent_color into token generation or mark the
key `# [PARKED] not yet consumed by rendering`.
Confidence: HIGH — Verification: read + rg.

### F-007 [S4] Placeholder URLs and publisher entity shipped in the SSOT
Where: product.toml:24, 228-231
Code:
```
help = "https://lumen-player.example.org/docs"
support = "https://lumen-player.example.org/support"
privacy = "https://lumen-player.example.org/privacy"
update_feed = "https://updates.lumen-player.example.org/feed.json"
```
What is wrong: all four product URLs are RFC-2606 example.org placeholders (honestly
tagged `[PROPOSAL]` at line 227); `publisher = "Lumen Contributors"` is a placeholder
entity (line 24). Any shipped About/Help surface will point users at dead domains.
Why it matters: user-facing dead links if a release is cut from this SSOT.
What correct looks like: block release packaging while urls.* match `example.org`
(a validator rule), or land real domains.
Confidence: HIGH — Verification: read.

## 4. Per-function table (config sections/keys)
| Key/Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| schema_version | 10 | REAL | — | =1; validator cross-check vs migrations max(to) implemented (config_validator.rs:524-565) |
| identity.name/short_name/display_version | 17-32 | REAL | — | display_version "0.1.3" matches shipped `lumen 0.1.3` self-report (baseline smoke header); consumed by rebrand/lint tooling |
| identity.publisher | 24 | PAPER-ONLY | S4 | placeholder entity (F-007) |
| identity.description_key | 32 | REAL | — | msgid exists in locales/en-US.toml:70; checked by validator |
| identity.ids.* | 34-54 | REAL | — | app ids all "dev.lumen.Lumen" == reverse_dns+PascalCase(name); validator enforces (passed, baseline #10) |
| window.* | 63-84 | REAL | — | consumed build-time by apps/desktop window_config.rs (include_str!, tests at :356,:444); title vars validated |
| brand.icon_* / logo_svg | 96-101 | FALSE | S0 | paths do not exist (F-001); "validated formats" not checked by any running tool |
| brand.accent_color | 95 | SHALLOW | S3 | dead for rendering; contradicts tokens.json orange (F-006) |
| defaults.theme | 108 | SHALLOW | S3 | themes/dark.toml missing (F-005) |
| defaults.locale | 111 | REAL | — | locales/en-US.toml exists; validator joins it |
| defaults.profile | 113 | SHALLOW | S3 | profiles/ dir missing (F-005) |
| telemetry.* | 124-129 | REAL-UNPROVEN | — | defaults off (honest); no telemetry code found to consume them — dead-but-safe keys |
| features.* (true rows) | 148-157 | MIXED | S0 | audio-output row false (F-003); core-playback/local-file/track-selection/config-surface plausible vs baseline AVI lane; streaming-http REAL-UNPROVEN (code lane exists) |
| features.* (false rows) | 155-168 | REAL | — | explicitly false with phase annotations — honest intent documentation |
| playback.* | 175-193 | REAL-UNPROVEN | — | hw_decode_policy "software-only" consistent with h264 sw decode failure (baseline smoke); not traced to engine code in this audit |
| limits.* | 200-208 | SHALLOW | S2 | values duplicated as mirror constants, not loaded at runtime (F-004) |
| web.size_budgets | 212-221 | REAL | — | read by web-budget --check, baseline #16 PASS |
| urls.* | 225-231 | PAPER-ONLY | S4 | example.org placeholders (F-007) |
| migrations | 233-244 | REAL | — | empty, born at v1; validator logic verified present |

## 5. Test honesty
Tests for this file live in crates/product-config/tests and tools/xtask/tests:
- `crates/product-config/tests/semantic.rs:201-222` constructs `FeatureRegistry::from_status` with 2-3 fixture ids — it tests the checker, never the real engine registry; it proves F-002's point: the real registry is never supplied.
- `apps/desktop/src/window_config.rs:356` `the_shipped_product_toml_parses` — would fail if product.toml [window] broke parsing: REAL guard for [window] only.
- `tools/xtask/tests/config_validator.rs` — exercises the validator on fixture TOMLs; would fail if cross-file checks regressed. Real, but irrelevant while the command runs nowhere in CI (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a violation is a build-breaking error" | product.toml:5-6 | FALSE | no workflow/gate invokes config-validator (rg); baseline #10 exit 1 with no consequence |
| "you cannot even name an unimplemented feature" (rule F1) | product.toml:137-139 | FALSE | schema additionalProperties boolean; config_validator.rs:19-20; semantic.rs F1 test-only |
| "Single source of truth"/"reads from here at runtime" | product.toml:4,14 | FALSE | mirror constants demux_host.rs:71-73, ranged.rs:32-34; host.rs:1569 placeholder default |
| "Feature exposure lists only implementations that exist" | product.toml:132-133 (quoting MP §5) | FALSE | audio-output=true vs NullDevice imports |
| "Paths are repo-relative; existence is checked by config-validator" | product.toml:87 | REAL | config_validator.rs:440-453; baseline #10 proves it runs — and finds them missing |
| web budgets "reads THESE values" | product.toml:214-216 | REAL | web_budget.rs:299-303; baseline #16 PASS |
| telemetry "opt-in only, default OFF" | product.toml:119-125 | REAL | enabled=false, crash_reports="off" in file |

## 7. Dead / duplicated / unreachable code (in/for this file)
- Dead-in-production F1 seam: `crates/product-config/src/semantic.rs:404` (`validate_semantics` has no production caller).
- Mirror constants duplicating limits: `apps/cli/src/demux_host.rs:71-73` (queues), `crates/streaming/src/ranged.rs:32-34` (network window).
- `brand.accent_color`: no rendering consumer.
- `version_code`, `max_playlist_items`, `max_open_sessions`, `probe_max_bytes`, `max_wasm_extension_memory_mib`: no consumer found in apps/cli (rg for literals/keys) — unenforced declarations.

## 8. Dependencies & flags
- Readers: tools/xtask (rebrand, lint-literals, check-identity, config-validator, web-budget, schema-gen), crates/product-config (library; dependents: apps/control-server, crates/plugin-api, crates/settings-schema — none is the shipped CLI), apps/desktop (include_str!), scripts/release/*, scripts/packaging/*.
- No feature flags gate this file's content.

## 9. Open questions for the next agent
- Does `lumen-tauri` read product.toml at runtime (baseline #2: tauri build UNVERIFIED — sandbox lacks gtk)? If yes, F-004's runtime claim may hold for the GUI only.
- Are `probe_max_bytes`/`max_open_sessions`/`max_playlist_items` enforced anywhere (crates/input, playback-engine)? Not found in apps/cli; a dedicated grep in demux/input crates would settle whether they are dead keys.
- Who generates `docs/web/size-record.toml` measured sizes (trust chain for web-budget PASS)?
