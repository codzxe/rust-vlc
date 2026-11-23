# AUDIT: crates/subtitles/src/font/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-23 |
| Lines / functions | 692 / 27 (19 prod incl. 6 trivial getters, 8 tests) |
| File verdict | UNREACHABLE (real, tested code; zero shipped callers, zero in-crate callers) |
| Reachable from shipped entry point? | NO — no dependency edge from lumen-cli/desktop to the `subtitles` crate at all (see F-001) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | rg for FontStack/FontFace users (exit 0, self+docs only); rg 'subtitles' in apps/*/Cargo.toml + crates/*/Cargo.toml (only fuzz/Cargo.toml:50); sed quote verification (exit 0) |

## 1. One-paragraph verdict
The font "stack seam" is honestly labeled (module docs admit no real font stack exists,
placeholder glyphs only) and the code that IS here is genuine: magic sniffing with typed
refusals, an ASCII-fold provider with exact-style preference, and a resolution chain with
a full consultation trail — all covered by meaningful tests that pass (baseline). But the
entire file is dead code twice over: (a) the `subtitles` crate is not a dependency of
lumen-cli, desktop, desktop-tauri, web, or ANY engine crate — only of the fuzz harness;
(b) even inside its own crate nothing consults `FontStack` — `ass/render.rs` imports only
`MetricsError`, `GlyphMetrics`, `MAX_FONT_SIZE_PX` from `crate::font`, and the module doc's
claim that render paths use `FontStack::empty()` is contradicted by rg. A seam with no
consumer on either side is scaffolding, not a shipped feature.

## 2. What actually works (verified)
— font::tests (8) pass: audit/runs/baseline/cargo-test-workspace.txt lines 5881-5888
(container_magic_sniffing_table, empty_stack_misses_with_empty_chain, family_matching_folds_ascii_case,
provider_len_and_emptiness, provider_prefers_exact_style_then_family, resolution_walks_providers_in_added_order,
synthesis_flags_report_the_style_gap, with_bundled_places_the_default_last — all `ok`).
Not re-run (cargo-build-budget); orchestrator baseline cited.

## 3. Findings
F-001 [S2]
Where: apps/cli/Cargo.toml:47-65, apps/desktop/Cargo.toml:33-54, fuzz/Cargo.toml:50
Code: (apps/cli deps list, verbatim excerpt)
```
[dependencies]
audio-engine = { path = "../../crates/audio-engine" }
...
video-engine = { path = "../../crates/video-engine" }
```
What is wrong: `rg -n 'subtitles' --glob 'Cargo.toml'` over the workspace finds exactly one
dependent: `fuzz/Cargo.toml:50: subtitles = { path = "../crates/subtitles" }`. No app, no
engine crate links the subtitles crate. Every line of this file is unreachable in the
release lane (`cargo build --release -p lumen-cli -p desktop`). Why it matters: the crate
describes itself as "the Lumen subtitle compositing contract" — in the shipped binary that
contract binds nothing. What correct looks like: playback-engine/video-engine consuming the
crate, or the crate marked as a not-yet-wired library. Confidence: HIGH — Verification: ran rg.
F-002 [S3]
Where: crates/subtitles/src/font/mod.rs:54-55, crates/subtitles/src/ass/render.rs:63-72
Code: (verbatim, mod.rs)
```
audit doc; until it lands [`FontStack::empty`] is the stack the
render paths use, and every `\fn`-driven lookup misses honestly.
```
What is wrong: The doc asserts render paths USE `FontStack::empty()`. They do not.
`rg FontStack|FontFace|FontRequest|FontResolution` over crates/subtitles/src outside
font/mod.rs returns zero code hits (lib.rs:80 is a doc comment; ass/render.rs imports only
`MetricsError`, `GlyphMetrics`, `MAX_FONT_SIZE_PX`). The stack half of this module has no
consumer anywhere — the "every \fn lookup misses" outcome holds only because nothing
consults a stack at all. Why it matters: doc describes wiring that does not exist. What
correct looks like: render path calling `FontStack::resolve`, or the doc stating no caller
exists yet. Confidence: HIGH — Verification: ran rg.
F-003 [S3]
Where: crates/subtitles/src/font/mod.rs:319-343
Code: (verbatim excerpt)
```
        let mut family_fallback = None;
        for f in &self.faces {
            if !request.family_eq(&f.family) {
                continue;
            }
            if f.bold == request.bold && f.italic == request.italic {
                return Some(f.clone());
            }
```
What is wrong: every successful lookup clones the full `FontFace` including `data:
Vec<u8>` — the raw font file bytes are copied per lookup. A bundled face plus MKV
attachments is easily megabytes; per-cue resolution in a render loop would copy them for
every subtitle event. Nothing today makes this hot (no callers), so this is latent, not
live. What correct looks like: `Arc<FontFace>`/`Arc<[u8]>` or returning a borrow.
Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/subtitles/src/font/mod.rs:36-38 vs 431-434
Code: (verbatim)
```
//! 1. Providers are consulted in ADDED order — the demuxer's attached
//!    fonts are added first, so ATTACHED beats the bundled default
```
vs
```
    pub fn add_provider(mut self, provider: MemoryFontProvider) -> Self {
        self.providers.insert(0, provider);
```
What is wrong: `add_provider` inserts at index 0, so consultation order is REVERSE of
added order (last added is consulted first). The policy doc sentence "consulted in ADDED
order" is false as written; `add_provider`'s own doc ("consulted BEFORE the providers
already added") is the accurate one, and the two contradict each other. Net behavior
(attached beats bundled) is right; test name `resolution_walks_providers_in_added_order`
(line 597) reinforces the wrong model — in that test `attached` is added LAST.
Confidence: HIGH — Verification: read + sed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Display for FontFaceError | 103-119 | REAL | — | correct messages, magic hex |
| FontRequest::new | 138-144 | REAL | — | plain ctor |
| FontRequest::family_eq | 148-150 | REAL | — | eq_ignore_ascii_case, honest non-ASCII note |
| FontFace::from_bytes | 174-191 | REAL | — | empty check then sniff; no panic |
| FontFace getters (family/bold/italic/data/format) | 193-221 | REAL | — | const accessors |
| sniff_format | 227-243 | REAL | — | correct magics (00 01 00 00/OTTO/ttcf/wOFF/wOF2); <4B zero-padded refusal |
| FontProvider trait | 253-258 | REAL | — | 2-fn contract |
| MemoryFontProvider::new | 278-284 | REAL | — | const ctor |
| MemoryFontProvider::catch_all | 289-292 | REAL | — | builder flag |
| MemoryFontProvider::with_face | 296-299 | REAL | — | push |
| MemoryFontProvider::len/is_empty | 303-311 | REAL | — | clippy len/is_empty pair |
| MemoryFontProvider::lookup | 319-343 | REAL | F-003 | exact-style-over-earlier-family logic correct; per-lookup clone |
| FontResolution::family_matched | 390-400 | REAL | — | scans chain; only one Supplied exists |
| FontStack::empty | 416-420 | REAL | — | |
| FontStack::with_provider | 424-426 | REAL | — | |
| FontStack::add_provider | 431-434 | REAL | F-004 | insert(0) = reverse-added consultation |
| FontStack::with_bundled | 442-448 | REAL | — | pushed last; PROPOSAL-marked, no face exists in-repo |
| FontStack::resolve | 456-489 | REAL | — | first hit wins, full trail, synthesis flags |
| tests (8) | 512-691 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| container_magic_sniffing_table | YES | asserts exact FontFormat/Err variants incl. 3-byte and WOFF cases |
| family_matching_folds_ascii_case | YES | 4 fold cases + 2 negative cases (non-ASCII byte-exact) |
| provider_prefers_exact_style_then_family | YES | asserts bold flag of returned face |
| empty_stack_misses_with_empty_chain | YES | asserts None + empty chain |
| resolution_walks_providers_in_added_order | YES | asserts full ProviderStep vectors (order-sensitive) — name misleading though (F-004) |
| synthesis_flags_report_the_style_gap | YES | exact synthesis vectors for 3 scenarios |
| with_bundled_places_the_default_last | YES | asserts family_matched()==false + trail |
| provider_len_and_emptiness | YES | basic but real assertions |
Tests are honest for what the module does; the gap is that the module does nothing shipped.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "until it lands FontStack::empty is the stack the render paths use" | mod.rs:54-55 | FALSE | rg: no FontStack use outside font/mod.rs (F-002) |
| "Providers are consulted in ADDED order" | mod.rs:36 | FALSE as written | insert(0) at mod.rs:432 (F-004) |
| audit record at docs/dependencies/records/font-stack-0.2.10.md, [PROPOSAL] | mod.rs:14-15 | REAL-UNPROVEN | file exists (ls); PROPOSAL status not independently verified |
| "No panics. Typed refusals only" | mod.rs:57-60 | REAL | read: no unwrap/panic paths; empty + short-data handled (lines 180-191, 229-235) |
| "the only font file in-repo is ui/src/assets/fonts/nunito-latin-var.woff2" | mod.rs:49-50 | UNVERIFIED | did not enumerate all font files in repo |

## 7. Dead / duplicated / unreachable code
The FontStack/FontProvider/FontFace/FontRequest/ProviderStep/GlyphSynthesis surface
(lines 69-490) has zero callers outside its own #[cfg(test)] module — dead even in-crate.
The file verdict UNREACHABLE follows from F-001.

## 8. Dependencies & flags
Crate deps: media-types only (path); dev: testkit, media-clock (crates/subtitles/Cargo.toml).
No feature gates on this module. No unsafe. WOFF2 decompression deliberately refused
(typed CompressedContainer) — honest.

## 9. Open questions for the next agent
- Who is supposed to wire `subtitles` into playback-engine, and does any doc/matrix row
  claim rendered subtitles in the shipped player? (Doc auditors: check feature-matrix
  subtitle rows against F-001.)
- font/pipeline.rs (sibling, not assigned to me) is the only font module actually consumed
  (by ass/render.rs) — audit its placeholder-glyph quality claims separately.
