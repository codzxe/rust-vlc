# AUDIT: crates/media-types/src/ids.rs (part2, lines 533–1065 of 1065)
> Chunk scope: part2 = lines 533–1065 — the tail of `LanguageTag::parse`
> (the function straddles the 532/533 boundary; audited whole here because
> this chunk is processed first), `LanguageTag::as_str`/`Display`,
> `CodecDescriptor`, `TrackFlags`, `Track`, and the entire `#[cfg(test)] mod
> tests` (655–1065, 27 tests + 2 test-only helpers — including the tests that
> exercise part1's `Scheme`/`SourceUri`/`from_file_path`, audited here).
> Part1 (lines 1–532) is a separate audit file by the same agent.
> Boundaries derived from assignments.tsv LOC column (532+533=1065=file
> length; same arithmetic verified against time.rs 619+619+620=1858).

| Field | Value |
|---|---|
| Auditor | W02-A10 |
| Date | 2026-11-21 |
| Lines / functions | 533 (of 1065) / 8 production units + 29 test units |
| File verdict | REAL (production code executed via shipped open lane; in-file test suite REAL-UNPROVEN — no baseline run reached it, see §5) |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/host.rs:904` `open_source` → `apps/cli/src/demux_host.rs:337` `SourceUri::from_file_path` / `:409` `SourceUri::parse` → demux header parse → `LanguageTag::parse` (`crates/demux/src/mp4/mod.rs:883`), `CodecDescriptor::new` (`mp4/mod.rs:879`, `wav/mod.rs:655`, `mkv/header.rs:228`, `avi/header.rs:289`, `ts/mod.rs:368`, `ogg/mod.rs:161`, `adts/mod.rs:250`, `aiff/mod.rs:463`, `es/mp3.rs:255`, `es/annexb.rs:172`) → `Track` → events. Executed: baseline `mp4-smoke-open-session.txt` shows `tracks 2 ready` + `MediaInfoReady { ... tracks: [Video h264, Audio aac] ... }` — Track construction through this vocabulary ran.) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `sed -n '1,135p;135,400p;400,535p;535,660p;660,860p;860,1065p' ids.rs` (full read); `rg -n "^(pub )?(fn|struct|enum|impl...)"` (outline: 8 impls, 27 tests, 2 helpers); `rg SourceUri::parse|from_file_path` (callers: demux_host.rs:337/409, apps/web/src/open.rs:168, fetch_port.rs:301, source.rs:95/122); `rg LanguageTag::parse|CodecDescriptor::new|TrackFlags` (demux call-site map above); `sed -n '49,125p' crates/input/src/uri.rs` (production resolver read for mirror-fidelity check); `rg -n "unc_path\(\)|nul_in_path\(\)" crates/input/src/uri.rs` (:89-90, :99-100); `rg "media_types|media-types" audit/runs/baseline/test-workspace*.log.txt` (NO unittest section for media-types in either log — fail-fast aborted before it); `rg -n "Running unittests" both logs` (12/14 sections, none media-types) |

## 1. One-paragraph verdict
The production half of this chunk (LanguageTag normalization, CodecDescriptor, TrackFlags, Track) is small, correct vocabulary code that the shipped MP4/AVI open lane demonstrably executes (baseline open-session transcripts show fully-populated tracks); `LanguageTag::parse`'s canonical-casing algorithm was hand-traced through `EN`/`zh-hans-cn`/`PT-br`/`es-419` and is right, including the parts[0] upper-then-lower corner. The weakness is the test module: (a) the round-trip "proof" runs against a hand-copied `mirror_resolve`/`mirror_decode` that omits four of the production resolver's refusal rules, so the one property it headlines ("every constructed URI decodes back to the path the caller handed in") is demonstrated only against a permissive clone — and is in fact false for `//`-leading and NUL-containing paths (see part1 F-002); (b) the "adversarial" redaction fixtures are adversarial only against the ten denylist keys already implemented, so the OAuth-style gap (`access_token` & co., part1 F-001 S2) is invisible to this suite; (c) none of these 27 tests has baseline execution evidence — both baseline `cargo test --workspace` runs failed fast at `apps/desktop`/`first-vertical` and never reached the media-types package, a coverage hole in the Wave-0 evidence that the INDEX's "1681 passed" headline does not surface.

## 2. What actually works (verified)
— `LanguageTag::parse` casing rules — verified by hand-trace: 2-letter alphabetic subtag → UPPER (`pt-br`→`pt-BR`), 4-letter → Title (`zh-hans-cn`→`zh-Hans-CN`), non-alphabetic (e.g. `419`) left verbatim, primary re-lowercased last (line 551) repairing the 2-letter rule's uppercase of `EN`→`en`; rejects empty/lead/trail/double hyphen, primary not 2–8 ASCII letters, non-ASCII subtags (alphabetic/alphanumeric checks are ASCII predicates). Would the tests catch a break? Yes — `language_tag_normalizes_canonical_casing` (1019) pins all three casing shapes with exact `as_str()` equality.
— `CodecDescriptor::new` rejects empty and ASCII-control descriptors (582-592); `codec_descriptor_rejects_empty_and_control_chars` (1050) pins both plus the accept case — mutation: dropping the control check fails the `mp4a\t40.2` assert.
— `GenerationId::next` checked-add + typed `ResourceLimit` error pinned by `generation_bumps_and_errors_at_exhaustion` (1009) — mutation: `wrapping_add` fails the `u64::MAX` error assert.
— Test suite quality (reasoning, not execution — see §5): the from_file_path tests pin exact byte vectors (`file:///a%20b/c%25d%3Fe%23f`, `%C3%BC` for UTF-8, `%5C` for POSIX backslash-as-data, verbatim-prefix strip, drive forms, typed UNC/relative refusals with exact error codes); redaction tests assert both presence of masking (`token=***`) and absence of leaks (`!r.contains("alice")`). Real assertions, not tautologies.
— Reachability executed: baseline `mp4-smoke-open-session.txt` (INDEX §"interactive open") — `event: tracks 2 ready`, `MediaInfoReady { ... tracks: [Video h264, Audio aac] ... }` — Track/CodecDescriptor/MediaKind construction through this chunk ran in the shipped binary; `rc-smoke-engine-only.log.txt` 6/6 AVI lane likewise.

## 3. Findings

### F-001 [S3]
Where: crates/media-types/src/ids.rs:845-878 (mirror doc + `mirror_resolve`), 881-909 (`mirror_decode`); production counterpart crates/input/src/uri.rs:85-100
Code:
```rust
    /// Test-only mirror of `input`'s resolver (`crates/input/src/uri.rs`):
    /// empty/`localhost` authority folds away, the path percent-decodes,
    /// then the RFC 8089 §E.3.1 drive rule strips `/C:/…` back to
    /// `C:/…`. Kept in lockstep by
    /// `from_file_path_round_trips_through_the_resolver_rules`.
```
and the production checks the mirror does not have (uri.rs:89-90, 99-100):
```rust
    if decoded.contains(&0) {
        return Err(error::nul_in_path());
```
```rust
    if decoded.starts_with("//") {
        return Err(error::unc_path());
```
What is wrong: The mirror implements only the resolver's happy path (authority fold + percent-decode + drive rule). It omits four production refusal rules: NUL-byte refusal (uri.rs:89-90), decoded-`//` UNC refusal (uri.rs:99-100), `?`/`#` suffix refusal (uri.rs:85-87), and the remote-authority refusal is an `assert!` (ids.rs:860-863) where production returns `Err` (uri.rs:72-73). The "kept in lockstep by <this very test>" claim is circular: the test compares the constructor against the mirror, so nothing detects drift between the mirror and `crates/input`. Concretely: `SourceUri::from_file_path("//tmp/x")` succeeds (part1 lines 223-224 accept any `/`-leading path) and mints `file:////tmp/x`; the production resolver refuses it (`input.uri.unc-path`), but `mirror_resolve` returns `//tmp/x` happily — so `from_file_path_round_trips_through_the_resolver_rules` (826) passes on an input the shipped resolver rejects. Same for `/tmp/a\0b` (mints `%00`, resolver refuses `nul_in_path`, mirror decodes it back).
Why it matters: This test is the file's only round-trip guarantee and its doc is the recorded evidence that constructor and resolver agree. It proves the property only for a permissive copy of one side; the two input classes where the real pair diverges sail through. A future change to input's resolver rules fails no test here.
What correct looks like: Either complete the mirror (add the four refusals and property-assert that `from_file_path` outputs never trip them), or add an integration test inside `crates/input` running the real `parse_file_path` over a corpus of `from_file_path` outputs (the layering rule forbids a dev-dependency from media-types to input, not a test in input's own suite).
Confidence: HIGH — Verification: read (both sides traced line-by-line; `sed -n '49,125p' crates/input/src/uri.rs`).

### F-002 [S4]
Where: crates/media-types/src/ids.rs:908 (section header), 928-930 (the one secret-key test)
Code:
```rust
    // ---- SourceUri: redaction (MP §12 — adversarial) ----------------------------
```
```rust
    fn redaction_masks_secret_query_values_only() {
        let uri = SourceUri::parse(
            "https://cdn.example.org/hls/index.m3u8?token=ABCDE123&segment=42&SIG=deadbeef", // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
```
What is wrong: The suite labels itself "adversarial" but every secret-key fixture uses a key already in `SECRET_QUERY_KEYS` (`token`, `SIG`). No vector exists for credential-shaped keys outside the denylist (`access_token`, `client_secret`, `authorization`) — precisely the blind spot of part1 F-001 (S2) — and no userinfo-password-containing-`@` vector (`http://alice:p@ss@example.org/` — which the implementation survives via `rsplit_once('@')`, but nothing pins it).
Why it matters: A redaction suite is only as good as its negative space; testing only the implemented denylist makes the denylist look complete by construction. The streaming crate delegates its whole "credentials never surface" contract to this function (`crates/streaming/src/schemes.rs:819-821` cites `SourceUri::redacted` as the redaction authority).
What correct looks like: Add fixtures for common OAuth/OIDC parameter names and assert they are masked (they would currently FAIL, exposing the gap), plus a password-with-`@` userinfo vector.
Confidence: HIGH — Verification: read (test bodies 911-997 inspected; no out-of-denylist key vector exists).

### F-003 [S4]
Where: crates/media-types/src/ids.rs:531 and 551
Code:
```rust
        let mut parts: Vec<String> = tag.split('-').map(str::to_string).collect();
```
```rust
        parts[0] = parts[0].to_ascii_lowercase();
```
What is wrong: The tag is split twice (validation pass at 521-522, normalization pass at 531 — a second `split('-')` plus a `Vec<String>` of per-subtag heap allocations), and the primary subtag is first uppercased by the 2-letter region rule inside the loop (a 2-letter primary like `EN` gets the `PT`-style upper) then repaired by a special-case re-lowercase at 551. Output is correct (verified by trace: `EN`→`en`, `PT-br`→`pt-BR`); the path is convoluted.
Why it matters: Language tags are cold, tiny inputs, so cost is irrelevant — but the double-fix structure invites regression: anyone extending the casing rules must remember parts[0] takes a second, out-of-loop correction, and the in-file tests cover that corner with exactly one 2-letter primary (`EN`).
What correct looks like: Normalize the primary before the loop (or skip index 0 in the loop) and split once.
Confidence: HIGH — Verification: read (full function 509-553 traced).

## 4. Per-function table (production units in 533–653 + straddling fn)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `LanguageTag::parse` (straddles boundary; whole fn audited here) | 509-553 | REAL | S4 (F-003) | Validates: empty/hyphen shapes, primary 2–8 ASCII letters (byte-len safe: non-ASCII fails the alphabetic predicate), subtags non-empty alphanumeric (single-char subtags like `en-1` accepted — documented v1 scope, doc honest); normalization correct by trace; no cap on subtag count (bounded by input length). Typed error `Unsupported`@`Demux`, code `track.language.invalid`. |
| `LanguageTag::as_str` | 555-558 | REAL | — | accessor, trivial and correct |
| `impl Display for LanguageTag` | 560-563 | REAL | — | writes normalized form |
| `CodecDescriptor::new` | 582-592 | REAL | — | rejects empty + ASCII control; no length bound (bounded by registry-minted source); typed error |
| `CodecDescriptor::as_str` | 594-597 | REAL | — | accessor |
| `impl Display for CodecDescriptor` | 599-603 | REAL | — | verbatim descriptor (doc: "never a secret" — call-site names are codec ids like `mp4a.40.2`, `mjpeg`, verified) |
| `TrackFlags` struct + `NONE` const | 611-633 | REAL | — | 4 documented flags; `NONE == default` pinned by test 1060; constructed by demuxers (mp4/mod.rs:894-899, mkv/header.rs:697) — reachable |
| `Track` struct (9 pub fields) | 635-653 | REAL | — | pure data; every field populated by demux headers (mp4/mod.rs:879-901 verified); `timing: TrackTiming` (time.rs, other agents' chunks) |

## 5. Test honesty (mod tests, 655–1065 — 27 tests + 2 helpers)
**No baseline execution evidence.** Both baseline workspace test runs abort before the media-types package: run #3 (`test-workspace.log.txt`) failed fast at `apps/desktop` `zero_alloc_transport` (log ends line 2412 `TEST_EXIT=101`; unittest sections = audio_engine…demux, desktop only); run #4 (`test-workspace-allfeatures.log.txt`) progressed only to `first-vertical` then aborted (14 unittest sections, none media-types). `rg "media_types" both logs` → only the `Compiling media-types` line. The INDEX "1681 passed / 1 failed" headline is a partial-workspace count. Every verdict below is mutation-reasoning, not execution.
REQUEST-ORCH-RUN: `cargo test -p media-types --lib ids::` (executes all 27 tests in this file; converts this section from reasoned to executed).
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| scheme_parses_and_normalizes_case (662) | YES | exact `as_str`/`to_string` equality on "HTtPs"→"https" |
| scheme_rejects_bad_grammar (669) | YES | 6 bad vectors incl. non-ASCII "héllo", digit-lead "1http" |
| source_uri_requires_a_scheme (678) | YES | asserts kind/stage/code triple, not just is_err |
| source_uri_rejects_control_characters_and_spaces (690) | YES | tab/space/newline vectors — fails if the control filter were dropped |
| source_uri_accepts_plugin_schemes (698) | YES | open-set proof: unknown scheme parses |
| from_file_path_is_byte_exact_for_posix (709) | YES | exact-vector pins: `%20 %25 %3F %23 %C3%BC` — keep-set mutation fails |
| from_file_path_encodes_a_posix_backslash_as_data (729) | YES | pins `%5C` + mirror-decode round-trip |
| from_file_path_strips_the_windows_verbatim_prefix (752) | YES | `\\?\C:\…` → `file:///C:/…` exact |
| from_file_path_normalizes_drive_forms (765) | YES | both slash spellings, lowercase drive, bare root `C:\` |
| from_file_path_rejects_unc_paths_typed (781) | YES | both UNC spellings; asserts code+stage+kind+`to_string()` |
| from_file_path_rejects_relative_paths_typed (806) | YES | incl. bare `C:` (no separator) and `\no-slash` |
| from_file_path_round_trips_through_the_resolver_rules (826) | PARTLY — see F-001 | proves round-trip against the incomplete mirror only; blind to `//`/NUL divergences |
| mirror_resolve / mirror_decode (850/885, helpers) | — | test-only; drifted copy of input's resolver (F-001) |
| redaction_hides_userinfo_passwords (911) | YES | exact output + two leak-negation asserts |
| redaction_hides_user_only_userinfo (921) | YES | `user@` also collapses to `***@` |
| redaction_masks_secret_query_values_only (928) | YES but narrow | only denylisted keys (F-002); case-insensitive `SIG` pinned |
| redaction_keeps_benign_queries_and_fragments (945) | YES | full-string equality (no over-masking) |
| redaction_hides_full_user_file_paths (955) | YES | `!contains("home")` — directory tree must not leak |
| redaction_file_scheme_still_masks_query_secrets (965) | YES | file+query combo exact equality |
| redaction_display_and_debug_are_redacted (972) | YES | both formatting paths negation-checked |
| redaction_handles_authorityless_and_port_forms (981) | YES | urn + host:port pass-through |
| raw_is_preserved_for_the_access_layer (990) | YES | raw() exact equality (redaction must not mutate raw) |
| ids_are_distinct_newtypes (999) | WEAK | comment admits distinctness is compile-time; pins accessors only — fails if `index()`/`handle()` altered, proves nothing about type distinctness |
| generation_bumps_and_errors_at_exhaustion (1009) | YES | `u64::MAX` exhaustion error pinned |
| language_tag_normalizes_canonical_casing (1019) | YES | 3 exact casing shapes |
| language_tag_rejects_malformed (1041) | YES | 6 vectors incl. 20-char primary |
| codec_descriptor_rejects_empty_and_control_chars (1050) | YES | empty + `\t` rejected, valid accepted |
| track_flags_none_is_all_clear (1060) | WEAK | `NONE == default` + all-false pins — shallow but real (a Default flip fails it) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "mismatched shapes are rejected early with an actionable error instead of silently matching nothing" (LanguageTag) | ids.rs:498-499 (doc 01 §3.1 vocabulary) | SHALLOW at shipped-path level | The function returns a typed error (509-517); but all three shipped consumers discard it — `mp4/mod.rs:883` `.and_then(\|tag\| ...parse(tag).ok())`, `mkv/header.rs:675` `.ok()`, `ts/mod.rs:1183` `if let Ok(lang)` — malformed tags silently become `None` in every shipped lane. Filed as part1 F-003 (doc block lives in part1's range). |
| "Non-empty by construction" (CodecDescriptor) | ids.rs:578 | REAL | `new` rejects empty (582-592); all minting routes through `new` (call-site map in header) |
| "The set is intentionally tiny" (TrackFlags) | ids.rs:613-615 | REAL | 4 flags; demuxers use NONE or the documented subset |
| "Redaction rules (all unit-tested against adversarial fixtures)" | ids.rs:101 | SHALLOW | fixtures adversarial only against the implemented denylist (F-002); suite never executed in baseline (§5) |

## 7. Dead / duplicated / unreachable code
`mirror_resolve`/`mirror_decode` (850-909) are deliberate test-only duplication of `crates/input/src/uri.rs` logic — the duplication itself is the defect vector (F-001): nothing keeps it in lockstep with the production copy it claims to mirror. No dead production code in this range.

## 8. Dependencies & flags
None: media-types is std-only (zero `[dependencies]`, verified by W01-A09's manifest audit). No feature gates any code in this range; only cfg is the standard `#[cfg(test)]`. No unsafe.

## 9. Open questions for the next agent
1. Demux agent: the `.ok()` swallows at mp4/mod.rs:883, mkv/header.rs:675, ts/mod.rs:1183 discard `LanguageTag` typed errors — is silent-None intended UX (baseline INDEX also notes a garbage `language: Some("voe")` default in demux), or a swallowed-error S2 on the demux side?
2. Input-crate agent: confirm `parse_file_path`'s refusal set (NUL, `//`, `?`/`#`, remote authority) vs `from_file_path`'s acceptance set is a known asymmetry (part1 F-002) — uri.rs's "neither layer can mint what the other rejects late" is violated by `//`-paths and `%00`.
3. Orchestrator: media-types unit tests have NO baseline execution evidence (§5); same for every package after `first-vertical` in run order (input, media-*, playback-engine, platform-*, playlists, plugin-*, product-config, renderer-wgpu, settings-schema, streaming, subtitles, theme-tokens, transcode, video-engine, apps/cli, apps/web). The "1681 passed" headline overstates workspace coverage. REQUEST-ORCH-RUN filed above.
