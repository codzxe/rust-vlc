# AUDIT: crates/plugin-api/src/kind.rs
| Field | Value |
|---|---|
| Auditor | W12-A07 |
| Date | 2026-11-21 |
| Lines / functions | 315 / 10 items (1 const, 1 enum ×16 variants, 4 methods, 1 Display impl, 1 enum, 4 tests) |
| File verdict | UNREACHABLE (real, exhaustively test-pinned taxonomy; consumed only by the manifest model and the unshipped wasm host) |
| Reachable from shipped entry point? | NO. `rg -ln "PluginKind\|InstanceScope" crates apps --type rust` → only plugin-api and plugin-host-wasm. In-tree consumers: manifest.rs:42-60 (serde bridge over `as_str`/`ALL`), manifest.rs `Capabilities::{first_listed_key,legal_keys}` (kind-keyed validation), install.rs:479 via api::kind_extension_legal (extension-legality refusal), testkit_wiring.rs:41. The shipped CLI uses plugin-api only for demux traits (rg `plugin_api::` apps/cli/src → Demuxer/SourceStream/DemuxEvent/SeekSupport only). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "InstanceScope\|instance_scope" crates --type rust` (0 consumers outside kind.rs — F-001); `rg -n "PluginKind::ALL" crates apps` (manifest.rs:51, testkit_wiring.rs:41); `rg -n "pub use\|pub mod" crates/plugin-api/src/lib.rs` (:92 re-exports only LUMEN_PREFIX + PluginKind — InstanceScope not even in the prelude); `ls crates/` (all crates named in the variant docs exist: input, streaming, demux, codecs, audio-engine, video-engine, audio-output, renderer-wgpu, subtitles, playlists, media-probe, device-discovery); `rg -n "pub trait Encoder" crates/codec-api/src` (encode.rs:34 — the Encoder claim is real); `rg -n "kind::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (:4443-4446 all 4 green); Read on manifest.rs:40-61 (serde bridge). |

## 1. One-paragraph verdict
A small, disciplined vocabulary file: sixteen `PluginKind` variants with kebab-case manifest spellings, an `ALL` const array that pins doc order, a v1 extension-allowlist that partitions all sixteen kinds both ways, and four tests that pin every table — all green in the no-fail-fast baseline (:4443-4446). The doc comments' crate references are real (every named crate exists; `codec_api::Encoder` exists at encode.rs:34). The one piece of furniture with nobody sitting on it: the entire §1.3 instance-scope taxonomy (`InstanceScope` + `instance_scope()`) has zero consumers — not the manifest model, not the wasm host, not even the crate's own prelude re-export (lib.rs:92 exports only `LUMEN_PREFIX, PluginKind`) — because the registry that would enforce construction/teardown ownership does not exist (doc 06 §3 Phase 2, same class as the `[dependencies]` non-enforcement in my manifest.rs#part2 audit). A vestigial `#[allow(clippy::match_same_arms)]` guards a match whose four arms are all distinct. Like the rest of the plugin-api manifest surface, none of it is reachable from a shipped binary.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4443-4446` → `kind::tests::{instance_scopes_match_doc_table, sixteen_kinds_in_doc_order, spellings_are_unique_kebab_case, v1_extension_policy_matches_doc_lists}` all ok.
— Verified by caller trace: `as_str` + `ALL` power the serde bridge (manifest.rs:42-60) so `kind = "decoder"` in TOML round-trips through this vocabulary; the extension policy is enforced at install (install.rs:479 → api::kind_extension_legal → `extension_allowed_in_v1`, traced in my api.rs audit) and pinned by `plugin_malicious_manifest` (plugin_install_lifecycle, green :4592).
— Verified the design guard: `PluginKind::ALL` is a fixed `[PluginKind; 16]`, and the three exhaustive matches (as_str/instance_scope/extension_allowed_in_v1) fail to compile if a 17th variant is added without updating all tables — the taxonomy cannot silently drift.

## 3. Findings

### F-001 [S3]
Where: crates/plugin-api/src/kind.rs:122-143, 173-188
Code:
```
    /// Who owns construction/teardown (doc 06 §1.3 instance-scope
    /// table).
    #[allow(clippy::match_same_arms)]
    pub const fn instance_scope(self) -> InstanceScope {
```
What is wrong: The entire instance-scope taxonomy is consumer-less. `rg "InstanceScope|instance_scope"` over all of crates+apps returns hits ONLY in kind.rs (definition + its own test). Nothing queries a kind's scope: no registry (doc 06 §3, Phase 2 — does not exist), no host lifecycle code, not the manifest model, and `InstanceScope` is not even re-exported by the crate prelude (lib.rs:92 `pub use kind::{LUMEN_PREFIX, PluginKind};` — the type is reachable only by full path). The method and its 16-entry table are documentation dressed as code: test-pinned (`instance_scopes_match_doc_table`, green) yet enforced nowhere.
Why it matters: Doc 06 §1.3 promises construction/teardown ownership semantics per kind; a reader of this file believes scope rules exist at runtime. They do not — the same PAPER-ONLY class as `[dependencies]` gating (manifest.rs#part2 F-002) and `plugins.failed` (ignored gap test, orchrun :3881). When a registry lands, an unenforced table is a silent drift source: nothing has ever executed these assignments.
What correct looks like: Either the Phase-2 registry consumes `instance_scope()` to own instance lifecycles, or the enum moves to doc 06 as prose until a consumer exists.
Confidence: HIGH — Verification: read + rg caller census (zero consumers).

### F-002 [S4]
Where: crates/plugin-api/src/kind.rs:124
Code:
```
    #[allow(clippy::match_same_arms)]
    pub const fn instance_scope(self) -> InstanceScope {
```
What is wrong: The allow is unnecessary: the match below (127-142) has four arms with four distinct bodies (`PerOpenSource`/`PerPlaybackSession`/`PerJob`/`AppSingleton`) — `clippy::match_same_arms` cannot fire on it as written. Baseline clippy is clean without needing it here.
Why it matters: A vestigial `#[allow]` silences exactly the lint that would flag a future edit that makes two arms accidentally identical (e.g., a kind moved into the wrong scope group by copy-paste). The checklist's "`#[allow]` hiding real lints" class — low blast radius, but the guard is pointed at nothing today and will mask the one mistake it exists for.
What correct looks like: Delete the attribute; clippy stays green (baseline run #5 proves the workspace passes without per-site allows of this kind).
Confidence: HIGH — Verification: read of 127-142 (four distinct bodies) + baseline clippy CLEAN (INDEX #5/#6).

## 4. Per-function table (315 lines, whole file)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `LUMEN_PREFIX` const | 6-8 | REAL | — | Consumed by manifest.rs validate (346) + prelude; lint-gated literal, legit protocol vocabulary. |
| `PluginKind` enum (16 variants) | 25-76 | REAL | — | `#[non_exhaustive]` + doc-order variants; per-variant docs name real crates (verified `ls crates/`: input, streaming, demux, codecs, audio-engine, video-engine, audio-output, renderer-wgpu, subtitles, playlists, media-probe, device-discovery all exist; `codec_api::Encoder` at encode.rs:34). |
| `PluginKind::ALL` | 79-97 | REAL | — | Fixed 16-array pins doc order; consumed by the serde bridge (manifest.rs:51) + testkit_wiring.rs:41. |
| `as_str` | 99-120 | REAL | — | 16 kebab spellings; uniqueness+charset test-pinned; feeds serde bridge, Display, error messages. |
| `instance_scope` | 122-143 | REAL (code) / PAPER-ONLY (use) | F-001 | Zero consumers tree-wide. |
| `extension_allowed_in_v1` | 145-164 | REAL | — | 7 allowed / 9 refused, partition test-pinned both directions; enforced at install (install.rs:479). Subtitle kinds' builtin-only classification is [PROPOSAL]-flagged honestly (149-152). |
| `Display` | 167-171 | REAL | — | = as_str; test-pinned (236). |
| `InstanceScope` enum | 173-188 | REAL (code) / consumer-less | F-001 | Not prelude-re-exported (lib.rs:92). |
| tests: `sixteen_kinds_in_doc_order` | 195-220 | REAL | — | See §5. |
| tests: `spellings_are_unique_kebab_case` | 222-238 | REAL | — | See §5. |
| tests: `v1_extension_policy_matches_doc_lists` | 240-284 | REAL | — | See §5. |
| tests: `instance_scopes_match_doc_table` | 286-314 | REAL | — | See §5 — pins a table nothing executes (F-001). |

## 5. Test honesty (all 4 green: orchrun run :4443-4446)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `sixteen_kinds_in_doc_order` | YES | `ALL.len()==16` + exact spelling vector (197-219); adding/removing/reordering a variant fails. |
| `spellings_are_unique_kebab_case` | YES | HashSet dup check, charset, edges, and `Display == as_str` (225-237). |
| `v1_extension_policy_matches_doc_lists` | YES | Both the allowed (250-261) and refused (270-283) vectors are exact — flipping any kind's policy fails one of them. |
| `instance_scopes_match_doc_table` | YES (for the mapping) | All 16 assignments asserted (288-313) — but the mapping feeds no production consumer (F-001); the test protects dead data. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The sixteen plugin kinds (doc 06 §1.2)" | kind.rs:1 | REAL | 16 variants, order+spellings test-pinned, green :4444. |
| "Doc 01 §2 defines fourteen; doc 06 splits Subtitle/Playlist into two each" `[PROPOSAL]` | kind.rs:12-18 | REAL-UNPROVEN | The split is implemented and flagged for Lead reconciliation (§10 Q1); cross-doc census belongs to the doc-01/doc-06 auditors. |
| "impls in `input`/`streaming`/`demux`/… L2-L4" (per-variant homes) | kind.rs:28-75 | REAL (crates exist) / PAPER-ONLY (as *plugin* impls) | Every named crate exists (ls); but nothing registers instances through a plugin registry — the "impls" are compiled-in domain traits (e.g. demux containers implement `plugin_api::Demuxer` directly, apps/cli/src/containers/*), and the registry is Phase 2. |
| "UiPanel … **extension only in v1** (§1.2 row 15)" | kind.rs:69-72 | REAL | `extension_allowed_in_v1` includes UiPanel; no builtin ui-panel manifest exists (plugins/ holds only extensions). |
| Subtitle kinds builtin-only `[PROPOSAL]` (doc leaves them unlisted) | kind.rs:149-152 | REAL | Refused list test-pinned (280-281); honest flag on the doc gap. |
| §1.3 instance-scope table | kind.rs:122, 173-188 | PAPER-ONLY | F-001 — no consumer enforces scopes. |

## 7. Dead / duplicated / unreachable code
- DEAD (consumer-less): `InstanceScope` + `instance_scope()` (F-001).
- Vestigial: `#[allow(clippy::match_same_arms)]` (F-002).
- Unused derives: `PartialOrd, Ord` on PluginKind — no sort/ordering use exists (rg: only the derive line). Harmless (copy of the access.rs newtype pattern); not counted separately.
- Layering note: PluginKind's serde bridge lives in manifest.rs:40-60 rather than beside the enum — deliberate ("keeps the enum free of the serde derive") but splits one vocabulary across two files; part1 (W12-A08) owns that site.
- UNREACHABLE from shipped entry points (header).

## 8. Dependencies & flags
- No imports beyond `std::fmt` — zero-dep vocabulary module; no feature gates; no unsafe. `[lints] workspace = true` inherited.
- The `lumen-allow-literal` marker on LUMEN_PREFIX is the sanctioned protocol-vocabulary escape (consistent with api.rs consts).

## 9. Open questions for the next agent
- Phase-2 registry owners (doc 06 §3): three consumer-less enforcement seams now catalogued from this domain — `[dependencies]` gating (manifest.rs#part2 F-002), `plugins.failed` events (orchrun :3881 ignored gap), and instance scopes (F-001 here). Is there a single card tracking all three, or are they drifting separately?
- Doc 06 auditor: §1.3's table is code-pinned but runtime-dead — the doc should say the scopes are aspirational until the registry lands.
