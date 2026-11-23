# AUDIT: crates/media-types/src/ids.rs
| Field | Value |
|---|---|
| Auditor | W02-A03 |
| Date | 2026-11-23 |
| Lines / functions | 1065 / 29 prod fns+methods, 27 tests, 2 test helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen open <path>`: apps/cli/src/main.rs:18 `lumen_cli::run::run` -> apps/cli/src/parse.rs:286 `CliCommand::Open` -> apps/cli/src/dispatch.rs:81 `Command::SessionOpen` -> apps/cli/src/demux_host.rs:337 `SourceUri::from_file_path(text)` and :409 `SourceUri::parse(url)` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 4 S4: 3 |
| Commands run | `./target/debug/deps/media_types-1f4a7feaef2048e3 ids::tests` -> 27 passed, 0 failed (exit 0); rustc probe vs prebuilt rlib (below) -> exit 0; rg searches for callers |

## 1. One-paragraph verdict
The types are honest: every constructor validates (no `Ok(())`-regardless paths), redaction is real code with real passing tests, arithmetic is checked (`GenerationId::next`), and `Display`/`Debug` both route through `redacted()`. The one substantive dishonesty is the redaction allowlist itself: it is an exact-match list of 10 parameter names that misses the most common credential query parameter on the web (`access_token`, plus `refresh_token`, `client_secret`, `auth_token`, `passwd`, `sessionid`), so those values flow verbatim into every log/error surface through `Display` — contradicting lib.rs's "Display never embeds credentials" claim (executed proof below). Secondary lies: a doc comment cites a test that does not exist, `GenerationId::next()` has zero production callers (the engine bumps generations with its own `checked_add` in epoch.rs), and `TrackFlags::accessibility` is vocabulary no producer populates and no consumer reads.

## 2. What actually works (verified)
— verified by `./target/debug/deps/media_types-1f4a7feaef2048e3 ids::tests` -> `test result: ok. 27 passed; 0 failed` (exit 0). Also verified by baseline audit/runs/baseline/cargo-test-workspace.txt:3727-3753 (all 27 ids tests `ok`).
— verified by compiling a probe against the prebuilt rlib (`rustc --edition 2024 --extern media_types=target/debug/deps/libmedia_types-90550932653cd8ba.rlib`, source: audit/runs/W02-A03.scratch/redact_probe.rs, exit 0): `token`/`apikey`/`SIG` values are masked; userinfo collapses to `***@`; `from_file_path` POSIX vectors, drive forms, verbatim-prefix strip, UNC/relative typed refusals all behave as the tests claim; `//server/share/x.mkv` mints `file:////server/share/x.mkv` (F-006).

## 3. Findings
F-001 [S2]
Where: crates/media-types/src/ids.rs:64-75 and 335-341; claim contradicted at crates/media-types/src/lib.rs:32-34
Code:
```
const SECRET_QUERY_KEYS: [&str; 10] = [
    "token",
    "key",
    "apikey",
    "api_key",
    "auth",
    ...
```
```
        if SECRET_QUERY_KEYS.contains(&key.to_ascii_lowercase().as_str()) {
```
What is wrong: exact-match allowlist misses `access_token`, `refresh_token`, `client_secret`, `auth_token`, `passwd`, `sessionid` — the standard OAuth2 (RFC 6750) credential parameter among them. Executed proof (probe output, verbatim): `https://stream.example.org/v?access_token=ABC123` -> `redacted: https://stream.example.org/v?access_token=ABC123` (value NOT masked); same for the other five; `token`/`apikey` correctly mask. Since `Display`/`Debug` (L382-393) call `redacted()`, and lib.rs:32-34 claims errors' "Display never embeds credentials", any source URL carrying `?access_token=…` (ubiquitous for authenticated HLS/DASH) prints its credential verbatim into logs, errors and UI. Why it matters: MP §12 mandates "secret redaction in logs"; this is a realistic credential leak, not a theoretical one. What correct looks like: substring/suffix matching on the key list (`key.ends_with("token") || key.contains("secret") || …`) or a deny-by-default for `*_token`/`*secret*`/`*key*` shapes, plus adversarial fixtures covering the missed names. Confidence: HIGH — Verification: ran (probe executable, exit 0).

F-002 [S3]
Where: crates/media-types/src/ids.rs:240-242
Code:
```
    /// The redacted form for logs, errors and UI. See the type docs for
    /// the exact rules; the implementation is tested against adversarial
    /// fixtures (`redaction_is_adversarially_tested` and friends).
```
What is wrong: no test named `redaction_is_adversarially_tested` exists anywhere in the repo (rg over the whole tree: the only hit is this doc comment itself). The real tests are `redaction_hides_*`/`redaction_masks_*` (L911-987). A doc citing a nonexistent test is an evidence-string lie, and the "adversarially tested" claim is overstated given F-001's six untested credential names. Confidence: HIGH — Verification: read + rg (`redaction_is_adversarially_tested` -> 1 file, ids.rs only).

F-003 [S3]
Where: crates/media-types/src/ids.rs:457-476
Code:
```
    /// The next generation in the monotonic sequence (checked; a u64
    /// counter that has run its full course does not wrap — it errors).
    ...
    pub fn next(self) -> Result<GenerationId, EngineError> {
```
What is wrong: zero production callers. Every `GenerationId::next()` call site is `#[cfg(test)]` or fuzz code (crates/codecs/src/audio/vorbis/decoder.rs:1075 — inside `mod tests` at :1059; opus/decoder.rs:572; vorbis/fuzz.rs:97). The doc at L435-436 says "The engine owns bumping" — and the engine does it with its own `checked_add(1)` in crates/playback-engine/src/epoch.rs:179,235 plus `GenerationId::new` (epoch.rs:268), bypassing this method entirely. The type's headline API is dead in production; two checked-add implementations now exist and can drift. Confidence: HIGH — Verification: read + rg caller sweep.

F-004 [S3]
Where: crates/media-types/src/ids.rs:616-617
Code:
```
    /// The track carries visual descriptions/accessibility data.
    pub accessibility: bool,
```
What is wrong: no demuxer ever sets `accessibility: true` (mkv hardcodes `accessibility: false` at crates/demux/src/mkv/header.rs:700; mp4 uses `..TrackFlags::NONE`; every other demuxer uses `TrackFlags::NONE`), and no consumer reads it (the only read in the tree is the `NONE` test at ids.rs:1063). Dead vocabulary: the field advertises accessibility-data flow that does not exist. (`default_on`/`forced` are genuinely produced by mkv and read by playback-engine/src/tracks.rs:315, so the criticism is scoped to `accessibility`.) Confidence: HIGH — Verification: read + rg sweep incl. ui/src.

F-005 [S3]
Where: crates/media-types/src/ids.rs:845-849
Code:
```
    /// Test-only mirror of `input`'s resolver (`crates/input/src/uri.rs`):
    /// ...
    /// `C:/…`. Kept in lockstep by
    /// `from_file_path_round_trips_through_the_resolver_rules`.
```
What is wrong: the lockstep claim is circular. The named test (L826-843) checks the constructor against `mirror_resolve`/`mirror_decode` — a test-local copy of input's logic (ids.rs:850-906) — so it cannot detect drift between the mirror and the real resolver (crates/input/src/uri.rs:57-119); if input's decode/drive-rule changed, this test would still pass. Mitigation (verified): input's own test suite pins the real resolver against constructor-minted shapes (uri.rs tests L274-287), so the repo-level net exists — but the in-file claim that this test keeps the mirror in lockstep is false as stated. Confidence: HIGH — Verification: read (both files side by side).

F-006 [S4]
Where: crates/media-types/src/ids.rs:220-228
Code:
```
        let encoded = if is_windows_drive(path) {
            let normalized = path.replace('\\', "/");
            let (drive, rest) = normalized.split_at(2);
            format!("/{drive}{}", encode_file_path(rest))
        } else if path.starts_with('/') {
```
What is wrong: forward-slash UNC (`//server/share/x.mkv` — a spelling Windows accepts) is not refused at the constructor; it is minted as `file:////server/share/x.mkv` (probe output, executed) and only later typed-refused by the resolver (`input.uri.unc-path`, crates/input/src/uri.rs:99-101, test at :254). Asymmetric with the backslash spelling which refuses immediately (L208-210). Consequence: on POSIX, where `//x` is a legal (Linux: equivalent-to-`/x`) path, a hand-typed `//`-prefixed local path is refused as "network path" — platform-blind refusal. Blast radius small (canonicalize never produces `//`). Confidence: HIGH — Verification: ran (probe) + read.

F-007 [S4]
Where: crates/media-types/src/ids.rs:313-317
Code:
```
fn redact_query_values(text: &str) -> String {
    let Some(qpos) = text.find('?') else {
        return text.to_string();
    };
```
What is wrong: the first `?` anywhere — including inside a fragment — starts query parsing, so `https://h/p#frag?token=x` redacts fragment content as if it were a query (probe output: `-> https://stream.example.org/v#frag?token=***`). Over-redaction only; no leak path found. Confidence: HIGH — Verification: ran (probe).

F-008 [S4]
Where: crates/media-types/src/ids.rs:295
Code:
```
    let basename = path.rsplit('/').next().unwrap_or(path);
```
What is wrong: `rsplit('/').next()` is always `Some` (possibly empty), so `unwrap_or(path)` is dead code. Cosmetic. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Scheme::parse | 26-47 | REAL | — | grammar enforced, case-normalized; rejects `""`, `1http`, `héllo` (ran tests) |
| Scheme::as_str | 50-52 | REAL | — | accessor |
| Display for Scheme | 56-59 | REAL | — | writes normalized form |
| SourceUri::parse | 132-149 | REAL | — | scheme required; rejects ASCII control+space in rest; permissive by documented design |
| SourceUri::from_file_path | 191-233 | REAL | S4 (F-006) | verbatim-strip, UNC/relative refusals, drive/POSIX shapes all verified by tests + probe; forward-slash UNC gap |
| SourceUri::scheme | 236-238 | REAL | — | accessor |
| SourceUri::redacted | 243-264 | REAL | S2 (F-001), S3 (F-002) | real redaction; allowlist gap executed |
| SourceUri::raw | 268-270 | REAL | — | documented access-layer-only escape hatch |
| split_authority | 275-284 | REAL | — | correct first-delimiter authority split |
| redact_file_path | 289-309 | REAL | S4 (F-008) | basename-only; `/` and empty-path edges handled |
| redact_query_values | 313-345 | REAL | S2/S4 (F-001, F-007) | key=*** for listed keys only; fragment `?` confusion |
| is_windows_drive | 351-357 | REAL | — | exact 3-byte shape check; matches resolver rule |
| encode_file_path | 368-379 | REAL | — | frozen keep-set, uppercase %HH; byte-exact vectors pass |
| Display for SourceUri | 382-387 | REAL | — | redacted form only |
| Debug for SourceUri | 390-393 | REAL | — | redacted form only |
| StreamId::new/index | 405-412 | REAL | — | newtype pass-through |
| TrackId::new/handle | 425-432 | REAL | — | newtype pass-through |
| GenerationId::FIRST/new/counter | 445-455 | REAL | — | constants/accessors |
| GenerationId::next | 464-477 | REAL but production-dead | S3 (F-003) | checked_add + typed error; no prod callers |
| LanguageTag::parse | 509-552 | REAL | — | 2-8 alpha primary, subtag checks, canonical casing; `es-419` numeric untouched; grandfathered `i-*` rejected by documented v1 scope |
| LanguageTag::as_str / Display | 555-564 | REAL | — | normalized form |
| CodecDescriptor::new | 582-591 | REAL | — | rejects empty + control chars |
| CodecDescriptor::as_str / Display | 594-603 | REAL | — | accessor |
| TrackFlags::NONE | 625-631 | REAL | — | equals Default (tested) |
| Track (struct) | 634-653 | REAL | — | plain data; consumers in demux/playback-engine |
| tests: 27 fns + mirror_resolve/mirror_decode | 661-1065 | REAL | S3 (F-005) | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| scheme_parses_and_normalizes_case / scheme_rejects_bad_grammar | YES | exact-value and rejection assertions on grammar |
| source_uri_requires_a_scheme | YES | asserts kind/stage/code of error |
| source_uri_rejects_control_characters_and_spaces | YES | three bad vectors must error |
| source_uri_accepts_plugin_schemes | YES | unknown scheme must parse (open set pinned) |
| from_file_path_is_byte_exact_for_posix | YES | 4 exact expected-URI vectors |
| from_file_path_encodes_a_posix_backslash_as_data | YES | pins `%5C` + decode round-trip |
| from_file_path_strips_the_windows_verbatim_prefix | YES | exact `file:///C:/Users/a%20b/x.mp4` |
| from_file_path_normalizes_drive_forms | YES | 4 vectors incl. lowercase drive |
| from_file_path_rejects_unc_paths_typed / rejects_relative_paths_typed | YES | asserts error code/stage/kind per input |
| from_file_path_round_trips_through_the_resolver_rules | PARTIALLY | verifies constructor vs test-local mirror, not the real resolver (F-005); would still pass if input drifted |
| redaction_hides_userinfo_passwords / hides_user_only / masks_secret_query_values_only / keeps_benign_queries_and_fragments / hides_full_user_file_paths / file_scheme_still_masks_query_secrets / display_and_debug_are_redacted / handles_authorityless_and_port_forms | YES | exact-string assertions; each would fail if its rule broke — but the fixture set does not include the F-001 names, so the suite cannot catch the allowlist gap |
| raw_is_preserved_for_the_access_layer | YES | exact raw string |
| ids_are_distinct_newtypes | WEAK | pins accessors only; the "distinct types" claim is a compile-time property, as its own comment admits |
| generation_bumps_and_errors_at_exhaustion | YES | checks u64::MAX path errors |
| language_tag_normalizes_canonical_casing / rejects_malformed | YES | 3 normalization vectors + 6 rejections |
| codec_descriptor_rejects_empty_and_control_chars | YES | 2 rejections + 1 accept |
| track_flags_none_is_all_clear | WEAK | tautological NONE==Default plus field-false pins |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "validated, redactable SourceUri that anchors MP §12's no-secrets rule" | ids.rs:1-5 | SHALLOW | redaction real for 10 keys; `access_token` etc. leak (F-001 probe output) |
| "Display never embeds credentials" | lib.rs:32-34 | FALSE (for query creds) | probe: `?access_token=ABC123` survives redacted()/Display verbatim |
| "credential-shaped query values … masked" (rules list) | ids.rs:106-108 | SHALLOW | true only for the 10 exact names listed at L64-75 |
| "tested against adversarial fixtures (`redaction_is_adversarially_tested` …)" | ids.rs:101,242 | FALSE citation | test name nonexistent (rg: 1 hit = the comment) |
| "round-trip proof … kept in lockstep" | ids.rs:827-849 | SHALLOW | mirror-based; cannot detect resolver drift (F-005) |
| "The engine owns bumping" (GenerationId) | ids.rs:435-436 | misleading | engine bumps via epoch.rs:179,235, never calls GenerationId::next (F-003) |
| "wave 30's Windows open fix" (history claim) | ids.rs:152-153 | UNVERIFIED | requires git history; agents do not run git |
| Scheme grammar / byte-exact POSIX / drive / UNC / relative rules | ids.rs:10-15,162-180 | REAL | tests executed: 27 passed, exit 0 |

## 7. Dead / duplicated / unreachable code
- `GenerationId::next` — production-dead (F-003); duplicated checked-add logic in playback-engine/epoch.rs.
- `TrackFlags::accessibility` — no producer, no consumer (F-004).
- `unwrap_or(path)` at L295 — dead branch (F-008).
- No todo!/unimplemented!/unreachable! in production code of this file (unreachable! appears only in test bodies as assertion substitutes — acceptable pattern, verified by reading every occurrence).

## 8. Dependencies & flags
Crate is std-only, zero runtime deps (crates/media-types/Cargo.toml:23 "No dependencies. Deliberately boring"); dev-dep `testkit` only. No feature gates; nothing cfg'd out of the release lane. All public items compiled into the shipped build; reachability of the file proven via apps/cli chain (§ header).

## 9. Open questions for the next agent
- Does any shipped path log the raw (not redacted) SourceUri? `raw()` is documented access-layer-only — verify in crates/input and streaming that no log/error formats `raw()` (my grep of ids.rs consumers shows raw() used, worth a dedicated sweep).
- MP4 demuxer drops container default-track/forced-track flags (mp4/mod.rs:893-900 sets only still_image) while playback-engine/tracks.rs:315 selects on `default_on` — MP4 default-track selection silently degrades to first-of-kind. Belongs to the demux/mp4 auditor.
- Whether `access_token`-style sources are realistic in the shipped product (HLS with token auth) — streaming crate auditor should confirm any code path building such URIs.
