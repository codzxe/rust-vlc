# AUDIT: crates/subtitles/src/font/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A03 |
| Date | 2026-11-21 |
| Lines / functions | 693 / 36 (+8 test fns) |
| File verdict | UNREACHABLE (real, unit-tested code; zero production callers for the entire provider-resolution half; crate itself unreachable from shipped entry points) |
| Reachable from shipped entry point? | NO — `subtitles` crate is a dependency of `fuzz/Cargo.toml` only (rg over all Cargo.tomls); `lumen` CLI / lumen-tauri link no `subtitles` code. In-crate: NO — rg `FontStack\|FontProvider\|FontRequest` over `crates/subtitles/src` hits only this file, `lib.rs` doc comments, and this file's tests. `ass/render.rs` imports only `crate::font::{MetricsError, GlyphMetrics, MAX_FONT_SIZE_PX}` (pipeline half). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 3 |
| Commands run | `rg -n "FontStack\|FontProvider\|FontRequest" crates/subtitles/src` (only self+tests+lib.rs docs) · `rg -n "FontStack\|MonospaceCellPipeline\|GlyphPipeline" crates/subtitles/tests` (no hits) · `rg -n "family\|font_name\|\\\\fn" crates/subtitles/src/ass` (\fn lands in style.font_name, render ignores) · `rg "subtitles" **/Cargo.toml` (only fuzz/) · `rg -n "font::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (8/8 ok, lines 5411-5418) · `sed -n` on every quoted range |

## 1. One-paragraph verdict
The magic-sniffing and provider-resolution code is genuinely implemented, panic-free, and honestly unit-tested (8 tests, all executed and green in the orchestrator no-fail-fast run), but it is a seam with no other half: nothing in the entire repository — not the ASS/VTT/CC render paths, not any demuxer, not the shipped binaries — ever constructs a `FontStack` or calls `lookup`. The module doc's central wiring claim ("[`FontStack::empty`] is the stack the render paths use, and every `\fn`-driven lookup misses honestly") is contradicted by the code: render paths perform no font lookup at all, `\fn` is stored into `style.font_name` (ass/tags.rs:255-257) and silently ignored by `MonospaceCellPipeline`/`GlyphMetrics`. On top of that: two doc-vs-code contradictions (consultation order, trail completeness), a per-lookup full-font-file clone design, and a stale bundled-font citation. Correct as typed data plumbing; dead as a subsystem.

## 2. What actually works (verified)
— `font::tests` 8/8 passed in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5411-5418` (container_magic_sniffing_table, family_matching_folds_ascii_case, provider_prefers_exact_style_then_family, empty_stack_misses_with_empty_chain, resolution_walks_providers_in_added_order, synthesis_flags_report_the_style_gap, with_bundled_places_the_default_last, provider_len_and_emptiness). Note: the primary fail-fast baseline (`test-workspace.log.txt`) never ran them — it died at `apps/desktop/tests/zero_alloc_transport.rs:98` before reaching `subtitles`; only the orchestrator's `--no-fail-fast` rerun executed this crate's tests.

## 3. Findings

### F-001 [S3]
Where: crates/subtitles/src/font/mod.rs:123-490
Code:
```rust
pub struct FontStack {
    providers: Vec<MemoryFontProvider>,
}
```
What is wrong: The entire source-resolution half of the module (`FontRequest`, `FontFace`, `FontProvider`, `MemoryFontProvider`, `GlyphSynthesis`, `ProviderStep`, `FontResolution`, `FontStack` — ~370 lines) has zero production callers anywhere in the repo. rg over `crates/subtitles/src` finds `FontStack`/`FontProvider`/`FontRequest` only in this file and `lib.rs` doc comments; rg over `crates/subtitles/tests` finds none; the only crate depending on `subtitles` is `fuzz` (which fuzzes srt/ass/vtt only).
Why it matters: ~370 lines of public API surface kept alive solely by its own unit tests — the "seam" has no producer (no demuxer attachment adapter) and no consumer (render paths bypass it). Dead code presented in `lib.rs:79-87` as the deliverable of issue #17 G-5.
What correct looks like: Either wire `FontStack::resolve` into the render path (per-run font resolution, even if every lookup misses today) or cut the provider half until the swash/skrifa admission lands.
Confidence: HIGH — Verification: read + rg callers (commands in header).

### F-002 [S3]
Where: crates/subtitles/src/font/mod.rs:53-55
Code:
```rust
//! audit doc; until it lands [`FontStack::empty`] is the stack the
//! render paths use, and every `\fn`-driven lookup misses honestly.
```
What is wrong: False claim. No render path constructs any `FontStack` — `ass/render.rs:72` re-exports only `crate::font::{GlyphMetrics, MAX_FONT_SIZE_PX}` and computes metrics via `GlyphMetrics::from_font_size(cue.style.font_size_px)` (ass/render.rs:366); the `\fn` tag is parsed into `state.style.font_name` (ass/tags.rs:255-257) and then never consulted. There is no lookup, honest or otherwise.
Why it matters: The module's honesty narrative ("misses honestly", "full consultation trail per lookup" — lib.rs:81-82) is documentation fiction: the mechanism that would record the miss does not exist in any call path. A reader auditing G-5 closure would believe lookups happen and miss.
What correct looks like: Doc states plainly: "no render path consults a FontStack today; \fn is recorded on the style and ignored by the placeholder pipeline."
Confidence: HIGH — Verification: read + rg (`FontStack` callers; `font_name` consumers in ass/render.rs — none).

### F-003 [S3]
Where: crates/subtitles/src/font/mod.rs:319-343
Code:
```rust
            if f.bold == request.bold && f.italic == request.italic {
                return Some(f.clone());
            }
            if family_fallback.is_none() {
                family_fallback = Some(f.clone());
            }
```
What is wrong: Every hit clones the full `FontFace`, whose `data: Vec<u8>` is the entire font file (typically 100 KB–2 MB). The fallback path clones too (line 333) even when a later exact-style match wins and the fallback is discarded, and the catch-all path clones again (line 338).
Why it matters: The intended VSFilter-style use is per-cue/per-run resolution (module docs §"Resolution policy"): wired as designed, every subtitle cue would memcpy a whole font file, and a style-miss probe would memcpy two. The trait returns owned `FontFace` (`fn lookup(&self, request: &FontRequest) -> Option<FontFace>`, line 257), baking the copy into the contract.
What correct looks like: Return a borrowed handle (`Option<&FontFace>`) or an `Arc<FontFace>`; keep bytes shared.
Confidence: HIGH — Verification: read (no run — code unreachable, so no perf measurement possible).

### F-004 [S4]
Where: crates/subtitles/src/font/mod.rs:36-38 vs 431-432
Code:
```rust
//! 1. Providers are consulted in ADDED order — the demuxer's attached
```
```rust
    pub fn add_provider(mut self, provider: MemoryFontProvider) -> Self {
        self.providers.insert(0, provider);
```
What is wrong: The module doc says consultation follows ADD order; `add_provider` inserts at index 0, so successive `add_provider` calls are consulted in REVERSE add order (LIFO). The method's own doc (line 428: "consulted BEFORE the providers already added") states the opposite of the module doc. The two-providers case (e.g. attachments from two sources) silently contradicts "consulted in ADDED order".
Why it matters: Ordering is the entire point of a fallback chain; the docs disagree with each other and one disagrees with the code.
What correct looks like: One true sentence: "providers are consulted most-recently-added first; `with_bundled` always last."
Confidence: HIGH — Verification: read + traced `insert(0, _)` + `for provider in &self.providers` (resolve, line 458).

### F-005 [S4]
Where: crates/subtitles/src/font/mod.rs:452-454 vs 476-480
Code:
```rust
    /// The trail is complete: every provider appears in `chain` (a
    /// `Missed` step for each miss), the winner is the FIRST hit, and
```
```rust
                    return FontResolution {
                        face: Some(face),
                        synthesis,
                        chain,
                    };
```
What is wrong: `resolve` returns at the first hit; providers after the winner are neither consulted nor recorded, so "every provider appears in `chain`" is false whenever a resolution succeeds before the last provider (the normal case). The module's own test `resolution_walks_providers_in_added_order` shows a 1-entry chain for a 2-provider stack (lines 607-613).
Why it matters: The trail is advertised as the honest-degradation ledger; it under-reports by design while claiming completeness.
What correct looks like: "chain records every provider consulted up to and including the winner; later providers are not consulted."
Confidence: HIGH — Verification: read + test output cited in §2.

### F-006 [S4]
Where: crates/subtitles/src/font/mod.rs:49-51
Code:
```rust
//! The only font file in-repo is `ui/src/assets/fonts/
//! nunito-latin-var.woff2` (OFL 1.1) — a WOFF2 container, which needs
//! Brotli decompression before any parser can read it (a second new
```
What is wrong: The cited file does not exist. The only font in-repo is `ui/src/assets/fonts/InterVariable.woff2` (verified by ls; docs/waves/w26/findings/w26.g03.md:140 confirms the UI font stack is Inter, "no Nunito").
Why it matters: A concrete repo citation that is false — the exact class of stale evidence this audit exists to catch. The substantive conclusion (WOFF2 ⇒ Brotli ⇒ out of scope) survives, but the reference is wrong.
What correct looks like: Cite `InterVariable.woff2`.
Confidence: HIGH — Verification: ls of ui/src/assets/fonts + repo-wide rg for *.ttf/*.otf/*.woff2 (only InterVariable.woff2).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module re-exports (`pub use pipeline::{…}`) | 62-67 | REAL | — | Pipeline half used by ass/render.rs; provider half unused |
| `FontFormat` (enum) | 74-82 | REAL | — | Tags only; no collection handling anywhere (accepts `ttcf` it cannot split) |
| `FontFaceError` (enum) | 85-101 | REAL | — | 3 typed refusals; matches doc claims |
| `Display::fmt for FontFaceError` | 103-119 | REAL | — | Correct strings |
| `impl Error for FontFaceError` | 121 | REAL | — | Trivial |
| `FontRequest` (struct) | 125-133 | REAL | S3 | No production constructor callers (F-001) |
| `FontRequest::new` | 137-144 | REAL | — | Allocates family string per call |
| `FontRequest::family_eq` | 148-150 | REAL | — | ASCII fold, byte-exact for non-ASCII, honest doc |
| `FontFace` (struct) | 154-166 | REAL | — | Owns full font bytes |
| `FontFace::from_bytes` | 174-191 | REAL | — | Empty→EmptyData before sniff; no size cap on `data` (unbounded intake — noted, demuxer's duty) |
| `FontFace::family/bold/italic/data/format` | 194-221 | REAL | — | Accessors, const where possible |
| `sniff_format` | 227-243 | REAL | — | <4 bytes: zero-padded magic + typed refusal; no panic path; never sees empty data (guarded) |
| `FontProvider` (trait: `name`, `lookup`) | 253-258 | REAL | S3 | Owned-`FontFace` return bakes in the copy (F-003) |
| `MemoryFontProvider` (struct) | 268-273 | REAL | S4 | Derives `Default` ⇒ name `""`, violates "stable provider name" (line 254) |
| `MemoryFontProvider::new` | 277-284 | REAL | — | const |
| `MemoryFontProvider::catch_all` | 288-292 | REAL | — | Builder |
| `MemoryFontProvider::with_face` | 295-299 | REAL | — | Builder, no dedup |
| `MemoryFontProvider::len / is_empty` | 302-311 | REAL | — | Clippy-canonical pair |
| `MemoryFontProvider::lookup` (FontProvider impl) | 319-343 | REAL | S3 | Exact-style-anywhere beats earlier family hit — correct; but full-file clones (F-003) |
| `GlyphSynthesis` (enum) | 347-353 | REAL | S3 | Dead (F-001) |
| `ProviderStep` (enum) | 356-371 | REAL | S3 | Dead (F-001) |
| `FontResolution` (struct) | 375-384 | REAL | S3 | Dead (F-001) |
| `FontResolution::family_matched` | 390-400 | REAL | — | `any` over ≤1 Supplied — correct |
| `FontStack` (struct) | 407-410 | REAL | S3 | Dead (F-001) |
| `FontStack::empty` | 415-420 | REAL | — | const, honest doc |
| `FontStack::with_provider` | 423-426 | REAL | — | Delegates |
| `FontStack::add_provider` | 430-434 | REAL | S4 | `insert(0)` LIFO vs module doc FIFO claim (F-004) |
| `FontStack::with_bundled` | 441-448 | REAL | — | Always last (push); PROPOSAL-honest doc; no caller can pass a real face today |
| `FontStack::resolve` | 455-489 | REAL | S4 | Early return vs "complete trail" doc (F-005); synthesis logic correct |
| test `ttf_bytes` / `face` helpers | 502-510 | REAL | — | Fixtures, not production |
| test `container_magic_sniffing_table` | 512-551 | REAL | — | Fails under sniff mutation (e.g. dropping the WOFF arm) |
| test `family_matching_folds_ascii_case` | 553-571 | REAL | — | Fails if fold removed |
| test `provider_prefers_exact_style_then_family` | 573-585 | REAL | — | Fails if first-family-hit returned unconditionally |
| test `empty_stack_misses_with_empty_chain` | 588-594 | REAL | — | Fails if resolve fabricated a face |
| test `resolution_walks_providers_in_added_order` | 596-644 | REAL | — | Asserts chain contents; fails under order mutation |
| test `synthesis_flags_report_the_style_gap` | 646-662 | REAL | — | Fails if flags not pushed |
| test `with_bundled_places_the_default_last` | 664-680 | REAL | — | Fails if bundled not last / family_matched misreported |
| test `provider_len_and_emptiness` | 682-691 | REAL | — | Fails if empty provider answered a lookup |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 8 tests | YES | Each asserts concrete values (formats, chain vectors, synthesis vectors, family names); no tautologies, no mocks of the subject. Mutation checks reasoned per row in §4. Caveat: they are the ONLY thing keeping this code alive (F-001). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "until it lands FontStack::empty is the stack the render paths use, and every \fn-driven lookup misses honestly" | font/mod.rs:54-55 | FALSE | No FontStack construction outside tests (rg); \fn → style.font_name ignored by render (ass/render.rs:366) |
| "Providers are consulted in ADDED order" | font/mod.rs:36 | FALSE (≥2 add_provider calls) | `insert(0, …)` line 432; method doc line 428 says the opposite |
| "The trail is complete: every provider appears in chain" | font/mod.rs:452-453 | FALSE (winner case) | Early return lines 476-480; own test shows 1-entry chain for 2 providers |
| "No bundled face yet (recorded, not hidden)" | font/mod.rs:47-55 | REAL | No ttf/otf in repo; only ui/…/InterVariable.woff2 (but the cited filename nunito-latin-var.woff2 is wrong — F-006) |
| "nothing in this record is admitted… in-tree deliverable is the SEAM" | docs/dependencies/records/font-stack-0.2.10.md:7-14 | REAL | No swash/skrifa in any Cargo.toml (rg); seam-only code |
| "every text cue currently renders as placeholder blocks" | matrix row textrender-subtitle-rasterizer notes | REAL | ass/render.rs placeholder policy; MonospaceCellPipeline name "monospace-cell-placeholder" |
| font stack seam: "the bundled default and the future MKV-attachment adapter implement [FontProvider]" | lib.rs:80-82 | FALSE (today) | Neither exists: no bundled face (F-006), no attachment adapter (rg); only MemoryFontProvider implements it |

## 7. Dead / duplicated / unreachable code
- Whole provider-resolution half (F-001): unreachable in-crate AND unshipped (fuzz-only dependency).
- `GlyphSynthesis`, `ProviderStep`, `FontResolution::family_matched` currently serve only tests.
- `FontFormat::Collection` accepted by sniff but nothing splits collections — a face-set tag with no consumer.

## 8. Dependencies & flags
- No feature flags. Deps: media-types (unused by THIS file — no media-types type appears here; it is a crate-level dep). Dev: testkit (Must), media-clock. No unsafe, no panics (verified: no unwrap/expect/index outside tests; indexing in `sniff_format` is length-guarded).

## 9. Open questions for the next agent
- font/pipeline.rs (other agent): does `GlyphPipeline` ever get called through the trait object, or only via `GlyphMetrics::from_font_size` direct calls (which would make the trait itself dead too)?
- If/when swash admission lands, who calls `FontStack::resolve` per run — and will the per-lookup clone (F-003) be fixed first?
