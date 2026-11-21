# AUDIT: crates/plugin-api/src/manifest.rs#part1 (lines 1-646)
| Field | Value |
|---|---|
| Auditor | W12-A08 |
| Date | 2026-11-21 |
| Chunk boundary | part1 = lines 1-646 per wave-00/assignments.tsv (LOC 646). Contents: module docs, `PluginKind` serde bridge, id/limit constants, `PluginId` newtype + serde, `PluginManifest` struct + `from_toml`/`to_toml`/`validate`/`effective_priority`, `SemverField`, `Origin`, `Capabilities` struct + `first_listed_key`/`legal_keys`/`present_string_lists` + `validate` (586-715, **spans the boundary** — the "list rules" loop body 644-715 is part2/A07's line range; I audited the whole function because a cut-in-half check cannot be assessed otherwise, and I flag the overlap here so A07 does not re-litigate 1-643). part2 (A07) = 647-1293: `Permissions`, `ConfigSchema`, `Dependencies`, `Provenance`, `Pubkey`, `ManifestError`, tests (~1045-1293). |
| Lines / functions | 646 / 23 (functions/impls) + 6 consts + 5 struct/enum defs |
| File verdict | UNREACHABLE (real, tested validation model — but no shipped entry point parses a manifest anywhere; see reachability) |
| Reachable from shipped entry point? | NO — call chain ends before this file. `lumen` CLI depends on plugin-api (apps/cli/Cargo.toml:59) but imports ONLY host traits: `rg -n "plugin_api" apps/cli/src` → `Demuxer`, `SourceStream`, `DemuxEvent`, `SeekSupport`, `Access` in containers/*.rs, demux_host.rs, bench.rs — zero hits for `PluginManifest`/`from_toml`. `rg plugin-host-wasm apps/*/Cargo.toml` → no matches (neither apps/cli nor apps/desktop-tauri depend on the host). Every `PluginManifest::from_toml` caller: plugin-host-wasm (install.rs:568, imports.rs:312, bin/lumenext.rs:142,186 — dev tooling) + plugin-api's own tests. desktop-tauri: `rg plugin apps/desktop-tauri/Cargo.toml` → no matches. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | `rg -n "plugin_api\|PluginManifest" apps/cli/src` (traits only — no manifest use in shipped CLI) · `rg "plugin-host-wasm" apps/*/Cargo.toml` (none) · `rg -n "PluginManifest::from_toml" crates --glob '!crates/plugin-api/**'` (plugin-host-wasm only) · `rg -n "plugins/builtin"` repo (does not exist; doc 06:157,274,293,559 + A10's lib.rs audit already flag this) · `rg -n "sample_rates\|misplaced\|not legal for kind" crates/plugin-api` (no negative test for misplaced numeric keys) · Read docs/06-plugin-architecture.md:196-235 (§2.2 capability table — `legal_keys` matches doc rows exactly) · `sed -n '595,621p;424,429p;76,80p;159,166p'` (quote verification) · baseline INDEX #3 (`cargo test --workspace` 1681 passed — manifest tests green), #5 (clippy CLEAN — the no-op loop at 597-607 passes clippy) |

## 1. One-paragraph verdict
The manifest model is genuinely implemented: grammar-enforcing `PluginId`, serde round-tripping, cross-field validation, a per-kind capability key-set check that faithfully mirrors doc 06 §2.2's table, with in-crate negative tests that do bite (verified by reading `capabilities_primary_key_enforced_per_kind`, manifest.rs:1164-1190, green in baseline #3). But the central advertised invariant — "misplaced keys [rejected] by `validate`" (manifest.rs:428) — is FALSE for `sample_rates` and `bit_depths`: the membership loop that should check them is a literal no-op `for … {}` and the follow-up loop checks only 4 of the 6 scalar keys. And none of this machinery is reachable from any shipped binary: the `lumen` CLI uses plugin-api purely as a trait vocabulary; the only manifest consumer (plugin-host-wasm) is a dependency of nothing that ships. This is contract code with no contractor attached.

## 2. What actually works (verified)
— `PluginId::parse` grammar: reasoned through all edge cases (empty, 1/4 segments, `a..b` empty middle, trailing dot, uppercase, >64 bytes, leading `-`) — each hits a typed `ManifestError::Id`; matches the documented `^[a-z0-9][a-z0-9-]*(\.[a-z0-9][a-z0-9-]*){1,2}$` grammar. Verified by reading 102-137; exercised green in baseline test-workspace.log.txt (#3, plugin-api suite).
— `PluginManifest::validate` cross-field rules (331-401): origin↔prefix, extension⇒provenance, builtin⇒no permissions/ no provenance, priority ≤ 1000, allowlist⇒http-fetch — each has a negative test in `cross_field_rules`/`priority_defaults_and_range`/`unknown_fields_rejected` (1193-1292, green, baseline #3). Would fail if broken (asserts exact `ManifestError` variants).
— `legal_keys`/`first_listed_key` (505-549) match doc 06 §2.2 table rows (docs/06-plugin-architecture.md:206-220) line for line, all 16 kinds.

## 3. Findings

### F-001 [S2] Capability key-set membership is never enforced for `sample_rates`/`bit_depths` — misplaced-key rejection is broken for 2 of 21 keys
Where: crates/plugin-api/src/manifest.rs:597-621 (with 553-578, 426-428)
Code:
```rust
        // (membership for the scalar keys is checked below)
        for (_name, _value) in [
            ("max_channels", self.max_channels),
            (
                "sample_rates",
                self.sample_rates.as_deref().map(|v| v.len() as u32),
            ),
            (
                "bit_depths",
                self.bit_depths.as_deref().map(|v| v.len() as u32),
            ),
        ] {}
        for name in ["max_channels", "latency_class", "hardware", "supports_seek"] {
```
What is wrong: The first loop has an empty body — it checks nothing. The second loop checks membership for only 4 scalar keys (`max_channels`, `latency_class`, `hardware`, `supports_seek`); `sample_rates` and `bit_depths` appear in neither it nor `present_string_lists` (553-578, which lists the 16 `Vec<String>` slots and omits both numeric lists). Therefore a manifest of kind `subtitle-parser` (or any non-decoder/encoder kind) carrying `[capabilities] sample_rates = [44100]` or `bit_depths = [16]` passes `Capabilities::validate` — the key is not legal for that kind per doc 06 §2.2 (table row `subtitle-parser`: `formats`, `features` only; docs/06-plugin-architecture.md:215) and per this file's own contract.
Why it matters: The struct doc promises "unknown keys are rejected by `deny_unknown_fields` and misplaced keys by [`validate`]" (manifest.rs:426-428). `deny_unknown_fields` cannot fire — the keys ARE struct fields. The §2.2 capability table is the manifest-fraud surface (a manifest lying about capabilities; doc 06:202-204 calls a false claim "a manifest lie that blocks shipping"), and the validation that is supposed to police it is silently half-missing. The in-crate tests never exercise a misplaced numeric key (`rg "not legal for kind"` → tests only use `formats`-on-`Decoder` at 1171), so the gap is invisible to CI.
What correct looks like: Add `"sample_rates"`, `"bit_depths"` to the 608 loop's name list with `is_some()` presence probes (exactly the `max_channels` pattern), delete the dead loop, and add a negative test asserting `ManifestError::Capabilities` for a `subtitle-parser` manifest with `sample_rates`.
Confidence: HIGH — Verification: read (control-flow traced: `present_string_lists` 553-578 excludes numeric lists; both loops read in full; legal_keys 529-536 shows only Decoder/Encoder admit these keys). Not executed (no cargo per rules) but the path is deterministic; tests green in baseline #3 do not cover it.

### F-002 [S3] Dead no-op loop left in the validator (leftover scaffolding)
Where: crates/plugin-api/src/manifest.rs:597-607
Code:
```rust
        for (_name, _value) in [
            ("max_channels", self.max_channels),
            (
                "sample_rates",
                self.sample_rates.as_deref().map(|v| v.len() as u32),
            ),
            (
                "bit_depths",
                self.bit_depths.as_deref().map(|v| v.len() as u32),
            ),
        ] {}
```
What is wrong: A loop whose body is `{}` and whose bindings are all underscore-prefixed — it constructs a 3-tuple array and discards it. This is scaffolding for the membership check that was never written (or was gutted), retained in the most security-relevant validator in the crate.
Why it matters: Dead code in a validator is worse than dead code elsewhere: a reader auditing "is sample_rates checked?" sees it listed here and stops looking. It also survived `cargo clippy --workspace --all-targets -- -D warnings` CLEAN (baseline #5) — evidence the lint net does not catch this class.
What correct looks like: Delete the loop; the membership check belongs in the 608 loop (see F-001).
Confidence: HIGH — Verification: read; clippy baseline #5 cited.

### F-003 [S4] `publisher_slug` ignores its match scrutinee and recomputes `segments()`
Where: crates/plugin-api/src/manifest.rs:159-166
Code:
```rust
    pub fn publisher_slug(&self) -> Option<&str> {
        match self.segments().get(1) {
            // Only meaningful for `ext.` ids; `lumen.` segment 2 is a
            // kind-area, and unprefixed ids have no publisher.
            _ if self.prefix() == Some(EXT_PREFIX) => self.segments().get(1).copied(),
            _ => None,
        }
    }
```
What is wrong: The scrutinee `self.segments().get(1)` is computed and thrown away — both arms start with `_`. `segments()` is then rebuilt inside the arm. Behavior is correct (an `ext.` id always has ≥2 non-empty segments, so `get(1)` is `Some`), but the construct lies about being a match.
Why it matters: Misleading structure in the id-namespacing API; a future editor could "simplify" to return the scrutinee and change behavior for unprefixed ids.
What correct looks like: `if self.prefix() == Some(EXT_PREFIX) { self.segments().get(1).copied() } else { None }`.
Confidence: HIGH — Verification: read.

### F-004 [S4] Doc typo: "builtins win contentional ties"
Where: crates/plugin-api/src/manifest.rs:78-79
Code:
```rust
/// Default priority for extensions — builtins win contentional ties
/// (doc 06 §2.2 [PROPOSAL]).
```
What is wrong: "contentional" is not a word (intended: "conventional" or "contested").
Why it matters: Public API doc noise in the crate whose whole job is a precise contract.
What correct looks like: Fix the word.
Confidence: HIGH — Verification: read.

### F-005 [S4] `CAPABILITY_ENTRY_LIMIT` doc says "summed across all lists" — numeric lists are excluded from the sum
Where: crates/plugin-api/src/manifest.rs:68-70 (behavior: 643-646 accumulator + 553-578)
Code:
```rust
/// Maximum capability entries per manifest, summed across all lists
/// (doc 06 §2.2 — oversized claims are rejected as a DoS guard).
pub const CAPABILITY_ENTRY_LIMIT: usize = 64;
```
What is wrong: The `total` accumulator (643) counts only what `present_string_lists` yields (the 16 string lists). `sample_rates`/`bit_depths` get a separate per-list ≤64 check (689-713, part2's lines) but are never added to `total`, so a manifest can legally carry 64 string entries + 64 sample rates + 64 bit depths = 192 entries.
Why it matters: Doc/code drift on the documented DoS guard; small but real budget inflation.
What correct looks like: Either add numeric-list lengths to `total` or scope the doc to "string-list entries".
Confidence: HIGH — Verification: read (accumulator init at 643 in my range; increment at 680 and numeric-list block 688-713 read for the trace, part2's lines).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `impl Serialize for PluginKind` | 42-46 | REAL | — | Trivial, `as_str` delegate. |
| `impl Deserialize for PluginKind` | 48-60 | REAL | — | Linear scan of `ALL`; error text hard-codes "sixteen" (57) — stale if a 17th kind lands. No panic paths. |
| `PluginId::parse` | 102-137 | REAL | — | Grammar fully enforced (traced: empty/1-seg/4-seg/empty-seg/upper/64B+ all typed errors). No panics; `s.len()` bytes = documented bytes. |
| `PluginId::as_str` | 140-142 | REAL | — | Trivial. |
| `PluginId::prefix` | 146-154 | REAL | — | Correct; `starts_with` cannot cross-match `lumen.`/`ext.`. |
| `PluginId::publisher_slug` | 159-166 | REAL | S4 | F-003: scrutinee discarded, double `segments()`. |
| `PluginId::segments` (private) | 169-171 | REAL | — | Trivial. |
| `Display`/`Serialize`/`Deserialize for PluginId` | 174-191 | REAL | — | Deserialize revalidates via `parse` — good (no `From<String>` bypass found). |
| `SemverField` + serde impls + `Display` | 249-272 | REAL | — | Delegates validation to `Semver::parse` (semver.rs is A10's file). |
| `Origin::as_str` / `Display` | 284-298 | REAL | — | Kebab-case serde ("builtin"/"extension") consistent with `as_str`. |
| `PluginManifest::from_toml` | 306-310 | REAL | — | parse → validate; first-error-out. |
| `PluginManifest::to_toml` | 319-324 | REAL | — | Round-trip tests live in part2/tests. |
| `PluginManifest::validate` | 331-401 | REAL | — | All 7 cross-field rules present and negatively tested (1193-1332 range read). Does NOT pin `api_version == ENGINE_PLUGIN_API_MAJOR` — by design (window negotiation in plugin-host-wasm api.rs:97); the lib.rs "CI checks builtin manifests" claim is FALSE (A10 finding, cross-ref). |
| `PluginManifest::effective_priority` | 407-415 | REAL | — | const fn; declared priority pre-validated ≤1000; no clamping needed. |
| `Capabilities::first_listed_key` | 505-521 | REAL | — | 16 kinds, exhaustive match; matches doc table first keys. |
| `Capabilities::legal_keys` | 524-549 | REAL | — | Verified against doc 06 §2.2 table (206-220): all 13 rows identical. |
| `Capabilities::present_string_lists` | 553-578 | REAL | S2 | Omits numeric lists → root of F-001. |
| `Capabilities::validate` | 586-715 (spans boundary) | REAL | S2/S3 | F-001/F-002 (597-621). Primary-key emptiness match's `_ => true` arm (634) fails closed (rejects) — safe, but would misreport any future primary key. List-rule body 644-715 is part2's range; empty-entry/dup/32B/charset/64-entry checks traced for F-005 consistency. |
| Consts `EXT_PREFIX`…`DEFAULT_PRIORITY_EXTENSION` | 62-80 | REAL | S4 | F-004 typo at 78. Values match doc 06 §2.2 [PROPOSAL] defaults (100/50). |
| Struct `PluginManifest` | 214-245 | REAL | — | `deny_unknown_fields` + `skip_serializing_if` — faithful to §2.2 shape. |
| Struct `Capabilities` | 429-499 | REAL | S2 | F-001 (fields exist for every §2.2 key). |
| Enum `Origin` / struct `SemverField` | 249-282 | REAL | — | See rows above. |

## 5. Test honesty (tests for this chunk live in part2's range, 1045-1293)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `capabilities_primary_key_enforced_per_kind` (1164-1190) | YES | Eight mutation-shaped assertions (misplaced `formats`, missing primary, dup, uppercase, 33B, 65 entries, 64 ok) each pin a distinct `validate` branch. Mutation check: delete the 589 membership check → 1171 fails. But it never mutates `sample_rates`/`bit_depths` placement → F-001 invisible. |
| `cross_field_rules` (1193-1234) | YES | Asserts exact `ManifestError` variants for 4 cross-field rules. |
| `priority_defaults_and_range` (1237-1270) | YES | Pins 100/50 defaults + `Priority(1001)` refusal. |
| `unknown_fields_rejected` (1273-1292) | YES | 3 negative TOMLs incl. `[capabilities.sneaky]` sub-table and typo'd permission. |
| Baseline: all green (INDEX #3, 1681 passed). | — | plugin-api suite included. |
| GAP | — | No test constructs a manifest with a misplaced `sample_rates`/`bit_depths` — the one branch that is broken is the one branch untested (F-001). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "unknown fields **rejected**" | manifest.rs:2-3 (module doc) | REAL | `deny_unknown_fields` on all structs; negative test 1273-1292. |
| "Round-trip is value-exact by construction" | manifest.rs:17-20 | REAL-UNPROVEN (tests exist in part2/doc06_examples.rs, green baseline #3; not re-run) | doc06_examples.rs:134-207 asserts byte-exact round-trip. |
| "misplaced keys by [`validate`]" | manifest.rs:426-428 | FALSE (for `sample_rates`/`bit_depths`) | F-001. |
| §2.2 capability key table per kind | docs/06-plugin-architecture.md:206-220 | REAL (in `legal_keys`) / FALSE (in enforcement) | legal_keys matches; enforcement misses 2 keys (F-001). |
| "total capabilities per manifest ≤ 64 entries" | docs/06:222-224 + manifest.rs:68-70 | SHALLOW | F-005: numeric lists escape the sum. |
| "A disk manifest claiming builtin is rejected as provenance forgery" | docs/06:197-200 | REAL | validate 345-365 origin↔prefix; test 1227-1233. (Whether "disk" loading exists at all: plugin-host-wasm install.rs — not reachable from shipped bins.) |

## 7. Dead / duplicated / unreachable code
- Dead no-op loop 597-607 (F-002).
- The whole manifest model is UNREACHABLE from shipped entry points (see header): the `lumen` CLI's 20+ `plugin_api::` imports are trait imports only; no app binary depends on plugin-host-wasm. The model's only consumers are plugin-host-wasm (itself unreferenced by apps) and tests. This matches A10's lib.rs finding (plugin surface has no runtime consumer) and the wave-12 hostile question "is there a real plugin loaded anywhere?" — for the manifest layer, the answer is NO.

## 8. Dependencies & flags
- deps: media-types, codec-api (path), serde+toml+thiserror — matches crate doc's allowlist claim (Cargo.toml comments; allowlist file not re-verified by me).
- No feature flags gate any part1 code (unconditional compilation, workspace build #1 includes it).
- No unsafe, no panics (`unwrap`/`expect` absent from 1-646 — verified by reading; hygiene counts INDEX #86-89).

## 9. Open questions for the next agent
- A07 (manifest.rs part2): the `Capabilities::validate` list-rules body (644-715) is in your range — I traced it for F-001/F-005 consistency; do not re-derive the membership gap. Check `Permissions`/`Provenance`/`Pubkey` validation (esp. `network_host_allowlist` entry grammar — is it hostname-validated anywhere? `validate` 393-399 only checks the implies-http-fetch rule).
- A09/A02 (plugin-host-wasm): `install.rs:568` is the only disk-manifest entry — confirm the tamper/hash refusal actually reaches `PluginManifest::from_toml` with hostile TOML (untrusted-input reachability of F-001: a forged manifest with `sample_rates` on a `ui-panel` kind would install cleanly).
- A10 (lib.rs): already recorded the plugins/builtin CI-check FALSE claim — my file corroborates (no builtin manifests exist anywhere; `rg "plugins/builtin"` hits docs only).
