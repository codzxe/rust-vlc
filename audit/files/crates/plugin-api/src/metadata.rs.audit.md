# AUDIT: crates/plugin-api/src/metadata.rs
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-24 |
| Lines / functions | 204 / 8 (1 enum, 1 struct, 1 trait, 1 fn, 4 test items) |
| File verdict | REAL (contract compiles, tested against a toy impl) but UNADOPTED — zero production implementors or consumers |
| Reachable from shipped entry point? | NO — `MetadataProvider`/`Metadata`/`lookup_not_served` referenced only inside plugin-api itself (rg over crates/ + apps/ + plugins/). plugin-host-wasm uses only `PluginKind::MetadataProvider` (the kind enum), not this trait. The example extension (plugins/extensions/metadata-provider) is raw wasm imports, not this trait. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | cargo test -p plugin-api --offline --lib metadata → 3 passed, exit 0; rg implementor/consumer searches (exit 0); sed quote verification (exit 0) |

## 1. One-paragraph verdict
Same shape as subtitle.rs one file over: a small, clean trait contract (pull-based extract from the engine-held probe window, on-demand gated lookups, absence-as-data vs damage-as-error) with an honest toy-impl test suite that runs green (my run + baseline). And the same defect: nothing implements or consumes it. The module doc claims "Builtin home: `media-probe` (L2)" — media-probe contains zero references to this trait and its own ProbeReport doc admits metadata extraction "is a later card; v0 reports the format, the confidence and the track hints only". The doc points at a builtin home that does not host it. Additionally the `Tag` "never empty" invariant is documented but not enforced anywhere (pub fields, no constructor/validation).

## 2. What actually works (verified)
— `cargo test -p plugin-api --offline --lib metadata` (my run): `extraction_reads_pairs`, `absence_is_data_and_damage_is_error`, `lookup_gates_on_served_kinds` — 3 passed, exit 0. Baseline: awk of cargo-test-workspace.txt plugin-api block shows the same 3 ok (exit_code=0).
— rg: the only `impl MetadataProvider` in the workspace is the test's PairProvider (metadata.rs:108); media-probe/src has no reference.

## 3. Findings
F-001 [S3]
Where: crates/plugin-api/src/metadata.rs:1-3
Code:
```
//! The metadata-provider contract (doc 06 §1.2 kind 14): tag / cover
//! art extraction at probe time, plus on-demand lookups. Builtin
//! home: `media-probe` (L2); allowed as a WASM extension in v1 (§1.4).
```
What is wrong: the claimed builtin home does not implement — or reference — this contract. crates/media-probe/src has zero `MetadataProvider`/`Metadata` mentions, and its own report.rs:265-268 says:
```
/// Metadata extraction (tags, cover art — the kind-14 plugin surface
/// whose builtin home is this crate) is a later card; v0 reports the
/// format, the confidence and the track hints only.
```
So media-probe v0 does no metadata extraction at all, and no WASM extension path routes through this trait either (plugin-host-wasm's metadata-provider example is raw wasm imports). The doc asserts a hosting relationship that exists nowhere. Why it matters: the kind-14 capability ("tag / cover art extraction at probe time, plus on-demand lookups") is contract-only; any doc/matrix row citing metadata providers as functional leans on this unadopted seam. What correct looks like: date-stamp the doc like media-probe honestly does ("later card"). Confidence: HIGH — Verification: ran (rg) + read.
F-002 [S4]
Where: crates/plugin-api/src/metadata.rs:40-46
Code:
```
pub struct Tag {
    /// Tag name, lowercase with `-` separators (the manifest
    /// vocabulary style; e.g. `title`, `album-artist`).
    pub name: String,
    /// The tag value (never empty — an empty value is no value).
    pub value: String,
}
```
What is wrong: both documented invariants (name lowercase-`-` style; value never empty) are unenforced — pub fields with no constructor, no validation method, no debug_assert. Any implementor can construct `Tag { name: "Title!", value: "" }` and the contract's vocabulary rule is broken silently. What correct looks like: a `Tag::new` that enforces both, or at minimum a documented validator. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/plugin-api/src/metadata.rs:70
Code:
```
    fn provides(&self) -> Vec<&'static str>;
```
What is wrong: the trait doc (67-69) requires `provides()` to be "a subset of the manifest's `provides` claim — doc 06 §7.2 capability honesty", but nothing enforces or checks the subset relation at any layer (the host derives grants from the manifest, not from this method; rg shows no caller comparing them). The honesty rule is documentation-only. Confidence: HIGH — Verification: read + rg (no consumer exists to enforce it).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| enum Metadata | 23-36 | REAL (unadopted) | — | Tag/CoverArt{mime,SharedBytes}/Lyrics; zero-copy claim rides SharedBytes |
| struct Tag | 39-46 | REAL (unadopted) | S4(F-002) | invariants documented, not enforced |
| trait MetadataProvider | 56-85 | REAL (unadopted) | S3(F-001) | extract/provides/lookup; Send bound matches worker-thread claim |
| lookup_not_served | 88-99 | REAL (unadopted) | — | typed Unsupported{FormatInfo{name:kind}} at Plugin stage; shape asserted by test |
| tests::PairProvider | 106-151 | REAL | — | toy impl proving contract shape incl. absence/damage split |
| tests::extraction_reads_pairs | 153-169 | REAL | — | 2 tags parsed, values asserted |
| tests::absence_is_data_and_damage_is_error | 171-183 | REAL | — | empty window ⇒ Ok(vec![]); `broken` ⇒ Err — the MP §8 distinction made executable |
| tests::lookup_gates_on_served_kinds | 185-203 | REAL | — | served kind ok; unserved ⇒ real lookup_not_served stage/kind asserted |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| extraction_reads_pairs | YES (for the toy) | asserts count + name + value |
| absence_is_data_and_damage_is_error | YES (for the toy) | both branches asserted |
| lookup_gates_on_served_kinds | YES | the only test touching production code (lookup_not_served): asserts stage==Plugin and kind==Unsupported |
Caveat: extract/lookup behavior tests exercise the test's own PairProvider, not any shipped provider — the production surface here is the trait + one error constructor.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Builtin home: `media-probe` (L2)" | metadata.rs:3 | FALSE (today) | media-probe: zero references to the trait; report.rs:265-268 admits extraction is "a later card" |
| "a plugin never opens sockets" (MP §12 posture) | metadata.rs:8-10 | REAL as contract rule; UNVERIFIED for real providers | no real implementor exists |
| "absence of metadata is Ok(vec![]), not an error" | metadata.rs:62-64 | REAL (vocabulary) | test asserts it |
| "must be a subset of the manifest's provides claim" | metadata.rs:67-69 | PAPER-ONLY | no enforcement anywhere (F-003) |

## 7. Dead / duplicated / unreachable code
The whole module is un-referenced production code (compiled into plugin-api, linked into the release lane via playback-engine/media-probe dep chain, never called). Planned seam, dead today. No duplication.

## 8. Dependencies & flags
media_types only (SharedBytes, EngineError, PipelineStage, ErrorKind/FormatInfo/ErrorContext, Corrupt/CorruptDetail in tests). No features.

## 9. Open questions for the next agent
- media-probe's "later card" for metadata extraction: scheduled in doc 13 or another passed window? (Same question as doc 06 §3 registry.)
- When a real provider lands, who enforces the provides⊆manifest-provides subset rule — the host (plugin-host-wasm) or the engine? Nothing today.
