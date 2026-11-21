# AUDIT: crates/media-types/src/ids.rs (part1, lines 1–532 of 1065)
> Chunk scope: part1 = lines 1–532 — module doc, `Scheme`, the redaction
> constants, `SourceUri` (parse / from_file_path / scheme / redacted / raw)
> and its private helpers (`split_authority`, `redact_file_path`,
> `redact_query_values`, `is_windows_drive`, `encode_file_path`), Display/Debug
> impls, `StreamId`, `TrackId`, `GenerationId`, `MediaKind`, and the
> `LanguageTag` struct+doc (its `parse`, which straddles the 532/533 boundary,
> is audited whole in the part2 file). The whole `mod tests` (655–1065,
> including the tests of these functions) is audited in the part2 file.
> Boundaries from assignments.tsv LOC column (532+533=1065).

| Field | Value |
|---|---|
| Auditor | W02-A10 |
| Date | 2026-11-21 |
| Lines / functions | 532 (of 1065) / 24 production units |
| File verdict | REAL (executed end-to-end in the shipped open lane; one S2 redaction gap) |
| Reachable from shipped entry point? | YES, EXECUTED (call chain: `lumen` CLI → `apps/cli/src/host.rs:904` `open_source` → `apps/cli/src/demux_host.rs:297` `std::fs::canonicalize` → `:337` `SourceUri::from_file_path(text)`; URL lane `:409` `SourceUri::parse(url)`; apps/web `open.rs:168`, `fetch_port.rs:301`, `source.rs:95/122`; `GenerationId::new` at `apps/cli/src/host.rs:1216`. Baseline `mp4-smoke-open-session.txt` `ok: accepted session=1` + `tracks 2 ready` proves the lane ran — the session could not open without this constructor) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `sed -n '1,135p;135,400p;400,535p' ids.rs` (full read of range); `sed -n '64,77p;210,216p;217,230p;368,379p;441,478p;496,500p;55,59p'` (verbatim quote verification); `rg -n "SourceUri::parse|SourceUri::from_file_path" crates apps` (caller map: demux_host.rs:337/409, web open.rs/fetch_port.rs/source.rs, 20+ test files); `rg -n "LanguageTag::parse|CodecDescriptor::new|TrackFlags|GenerationId" crates/demux apps` (demux + CLI call sites); `rg -n "access_token|client_secret|authorization" crates` (no second redaction layer — streaming relies on SourceUri::redacted, schemes.rs:819-821); `rg 'format!\("file://' crates apps` (hand-roll-retirement claim: only ids.rs:232 mints in production; demux_host.rs:940 is a test input); `sed -n '49,125p' crates/input/src/uri.rs` + `rg -n "unc_path|nul_in_path" input/uri.rs` (resolver refusal set for F-002); `python3 -c "import os; print(os.path.realpath('//tmp/x'))"` → `/tmp/x` (Linux realpath collapses `//` — std::fs::canonicalize uses realpath(3)); `sed -n '294,296p' crates/media-types/src/error.rs` (Display format `[stage] kind (code)` matches test assertion) |

## 1. One-paragraph verdict
This is the best-audited kind of L0 code: small, pure, typed-refusal constructors with exact-vector tests — and it demonstrably runs in the shipped `lumen open` lane. But its one security-facing contract has a hole with a documented promise behind it: the query-secret redaction is a ten-key denylist that misses the OAuth/OIDC parameter family (`access_token`, `client_secret`, `authorization`, `refresh_token`), so those credential values ride verbatim through `redacted()`/`Display`/`Debug` into logs and errors, contradicting both this crate's own lib.rs claim ("Display never embeds credentials", lib.rs:32-34) and MP §12 ("secret redaction in logs"), and inheriting downstream into the streaming crate, which names `SourceUri::redacted` as its credential-hiding authority (schemes.rs:819-821). Second, the constructor/resolver pair is asymmetric: `from_file_path` happily mints `file:////tmp/x` from a `//`-leading POSIX path (and `%00` from a NUL byte) that the production resolver (`crates/input/src/uri.rs:99-100, 89-90`) refuses as UNC/NUL — a wrong-typed-late-refusal that the shipped CLI happens to dodge only because it canonicalizes first (Linux realpath collapses `//`; verified). Third, the LanguageTag doc's "actionable error instead of silently matching nothing" is defeated by every shipped consumer, all of which `.ok()`-discard the error. The remaining code (Scheme grammar, split/encode helpers, newtype ids, checked GenerationId) traced clean.

## 2. What actually works (verified)
— `Scheme::parse` grammar — verified by trace: empty→refuse, first char must be ASCII alpha, rest ASCII alnum/`+`/`-`/`.`, case-normalized to lowercase; non-ASCII (e.g. `héllo`) fails the ASCII predicates. `scheme_rejects_bad_grammar` (test, part2 file) pins 6 vectors.
— `from_file_path` byte-exactness — verified against its own test vectors by trace: POSIX keep-set is exactly RFC 3986 unreserved + `/`; space→`%20`, `%`→`%25`, `?`→`%3F`, `#`→`%23`, UTF-8→per-byte `%C3%BC`, POSIX `\`→`%5C` (data, not separator); verbatim `\\?\` prefix stripped, `\\?\UNC\` and plain `\\` refused typed `uri.file.unc-path`; drive forms slash-normalized with the RFC 8089 `/C:` empty-authority prefix; relative input refused (`uri.file.relative-path`); the final `parse` is never unwrapped (defensive `map_err`). Executed in baseline: `mp4-smoke-open-session.txt` accepted-session + tracks-ready cannot happen without it.
— Redaction userinfo case — verified by trace incl. the adversarial shape `http://alice:p@ss@example.org/` (`rsplit_once('@')` at 251 splits on the LAST `@`, so a password containing `@` does not leak the host split; userinfo collapses to `***@`).
— Display/Debug are redacted forms (381-394) — `raw()` is the only raw accessor and its doc restricts it to the access layer; `raw_is_preserved_for_the_access_layer` pins that redaction never mutates raw.
— Hand-roll retirement claim — `rg 'format!\("file://'` over crates+apps: the only production minter is ids.rs:232 itself; demux_host.rs:940 is a test-INPUT vector (verified context at 929-943). Claim "every former hand-rolled site delegates here": REAL.
— Error Display contract — error.rs:294-295 renders `[stage] kind (code)`; the test's expected `"[open] unsupported (uri.file.unc-path)"` matches the implementation.

## 3. Findings

### F-001 [S2]
Where: crates/media-types/src/ids.rs:64-77 (list), 335-340 (the membership test); contradicted claim at crates/media-types/src/lib.rs:32-34
Code:
```rust
const SECRET_QUERY_KEYS: [&str; 10] = [
    "token",
    "key",
    "apikey",
    "api_key",
    "auth",
    "signature",
    "sig",
    "password",
    "secret",
    "session",
];
```
and the membership test it feeds:
```rust
        if SECRET_QUERY_KEYS.contains(&key.to_ascii_lowercase().as_str()) {
            out.push_str(key);
            out.push('=');
            out.push_str(REDACTED);
```
and the crate contract it breaks (lib.rs:32-34):
```rust
//! - Errors name the failing stage, carry structured context, and their
//!   `Display` never embeds credentials or full user paths — URIs appear
//!   only through [`SourceUri::redacted`] (MP §12).
```
What is wrong: The query-secret redaction is a closed ten-key denylist. The OAuth/OIDC parameter family — `access_token`, `refresh_token`, `client_secret`, `authorization`/`auth_token`, and common variants (`passwd`, `pass`, `assertion`) — is absent. A realistic credentialed media URL (`https://cdn.example.org/hls/index.m3u8?access_token=eyJhbGciOi...`) passes `redact_query_values` untouched: the key is not in the list, so the pair is copied verbatim (line 339-340), and the bearer token flows into every log, error string and UI surface that formats the `SourceUri` via `Display`/`Debug`/`redacted()`. `rg access_token|client_secret crates` shows no second redaction layer exists — `crates/streaming/src/schemes.rs:819-821` explicitly delegates: "the redacted URI form is media-types' (SourceUri::redacted)".
Why it matters: MP §12 (docs/MASTER-PROMPT.md:254) mandates "secret redaction in logs"; lib.rs:32-34 promises Display never embeds credentials; the streaming crate's own tests assert "Credential-shaped queries never surface" while only ever exercising `token=` (a listed key). An OAuth-protected HLS/DAI URL — the canonical VLC-use-case for query credentials — leaks its bearer token to logs. This is exactly the class of defect a denylist cannot see and its own "adversarial" test suite (part2 F-002) does not test.
What correct looks like: Widen the list (cheap, still a denylist) AND/OR switch the default: mask any query value whose key matches `*token*|*secret*|*key*|*auth*|*password*|*signature*|*session*|credential*` substrings, or mask ALL query values for non-interactive sinks and keep the allowlist for debug UI. Add out-of-denylist fixtures to the suite (they fail today).
Confidence: HIGH — Verification: read (list + membership path + downstream delegation traced; no other redactor exists in-tree).

### F-002 [S3]
Where: crates/media-types/src/ids.rs:219-227 (acceptance branch); production refusal at crates/input/src/uri.rs:99-100 (and 89-90)
Code:
```rust
        let encoded = if is_windows_drive(path) {
            let normalized = path.replace('\\', "/");
            let (drive, rest) = normalized.split_at(2);
            format!("/{drive}{}", encode_file_path(rest))
        } else if path.starts_with('/') {
            encode_file_path(path)
        } else {
            return Err(refusal("uri.file.relative-path"));
        };
```
and the production resolver's counter-rule (uri.rs:99-100):
```rust
    if decoded.starts_with("//") {
        return Err(error::unc_path());
```
What is wrong: The constructor accepts ANY `/`-leading string as a POSIX absolute and encodes it byte-exact — including `//tmp/x` (a legal POSIX path with an implementation-defined leading `//`) and `/tmp/a\0b` (NUL encodes to `%00`). Both mint URIs (`file:////tmp/x`, `file:///tmp/a%00b`) that the production resolver refuses late: decoded-`//` → `input.uri.unc-path` (uri.rs:99-100), decoded NUL → `nul_in_path` (uri.rs:89-90). uri.rs's own comment claims the two layers mirror each other "so neither layer can mint what the other rejects late" (uri.rs:95-97) — false for these two input classes. The shipped CLI escapes only by accident of ordering: `demux_host.rs:297` canonicalizes before calling the constructor, and Linux `realpath` collapses `//` (verified: `os.path.realpath('//tmp/x')` → `/tmp/x`); any non-canonicalizing caller (the constructor's doc invites direct use: "callers canonicalize first and pass `canonical.to_str()`" — a convention, not an enforcement) gets a URI whose open fails with a UNC/NUL error naming the wrong defect.
Why it matters: The constructor/resolver pair is the file-path↔URI round-trip contract; a one-way acceptance set wider than the resolver's is a divergence factory, and the file's own round-trip test cannot see it because its mirror omits exactly these two refusals (part2 F-001). On POSIX-strict or macOS hosts where `//` may be preserved by canonicalize (UNVERIFIED — no macOS in sandbox), even the CLI's canonicalize-first path could mint the refusable URI.
What correct looks like: `from_file_path` refuses `//`-leading and NUL-containing inputs itself (typed `uri.file.unc-path` / a new `uri.file.nul-byte`), making its acceptance set a subset of the resolver's; or `resolve_file` collapses a leading `//` after decoding instead of refusing. Both sides then match the "neither mints what the other rejects" comment.
Confidence: HIGH — Verification: read (both functions traced; Linux realpath behavior verified via python; macOS canonicalize `//`-preservation UNVERIFIED — no macOS sandbox).

### F-003 [S3]
Where: crates/media-types/src/ids.rs:498-499 (claim); refuting call sites crates/demux/src/mp4/mod.rs:883, crates/demux/src/mkv/header.rs:675, crates/demux/src/ts/mod.rs:1183
Code:
```rust
/// `-`); full BCP 47 is a data problem owned by the localization crate,
/// not the vocabulary crate — mismatched shapes are rejected early with
/// an actionable error instead of silently matching nothing.
```
and all three shipped consumers (mp4/mod.rs:883):
```rust
            .and_then(|tag| media_types::ids::LanguageTag::parse(tag).ok());
```
What is wrong: The doc justifies the validator by claiming malformed tags produce an "actionable error" rather than silence. Every shipped consumer discards the error: mp4 `.ok()`, mkv `.ok()` (header.rs:675), ts `if let Ok(lang)` (ts/mod.rs:1183). In the shipped MP4 lane (the litmus path), a container tag that fails validation silently becomes `language: None` — exactly the "silently matching nothing" outcome the sentence rules out. The error is reachable only to tests.
Why it matters: The sentence is the recorded rationale for validating in L0 at all; it is false end-to-end. Either the demuxers should surface the refusal (a `Track` with no language where the container declared one is a metadata bug worth an error/warn), or this doc should say "rejected early; hosts may downgrade to None".
What correct looks like: Fix the doc to match reality, or make demux carry the typed error (demux-audit side). At minimum drop "instead of silently matching nothing" — nothing in the shipped tree acts on the error.
Confidence: HIGH — Verification: read (all three call sites sed-verified; LanguageTag::parse itself does return the typed error — the function is honest, the claim about its effect is not).

### F-004 [S4]
Where: crates/media-types/src/ids.rs:464-477
Code:
```rust
    pub fn next(self) -> Result<GenerationId, EngineError> {
        self.0.checked_add(1).map(GenerationId).ok_or_else(|| {
            EngineError::new(
                PipelineStage::Control,
                ErrorKind::ResourceLimit {
                    limit: crate::error::LimitKind::SessionGenerations,
                    budget: u64::MAX,
                    attempted: u64::MAX,
                },
```
What is wrong: The exhaustion error reports `attempted: u64::MAX`, but the attempted value is `u64::MAX + 1` (unrepresentable); `u64::MAX` is the value already held. The limit metadata is self-inconsistent: budget == attempted == u64::MAX reads as "asked for exactly the budget", not "overflowed the counter".
Why it matters: `LimitKind`'s `budget`/`attempted` fields are structured data a UI or telemetry consumer renders ("attempted X of budget Y"); here the pair says something untrue about the failure. Cosmetic because the branch is unreachable in practice (2^64 generations), but it is a typed-error contract in the most-reviewed crate.
What correct looks like: `attempted: u64::MAX` with a comment that +1 overflowed, or an `ErrorKind` variant / context note that says "counter exhausted" without a fake attempted value (the `generation.exhausted` code already carries the truth).
Confidence: HIGH — Verification: read (checked_add semantics; LimitKind fields at error.rs:171-185).

### F-005 [S4]
Where: crates/media-types/src/ids.rs:368-379
Code:
```rust
fn encode_file_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/');
        if keep {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
```
What is wrong: Each escaped byte allocates a temporary `String` via `format!("%{byte:02X}")` — one heap format call per non-unreserved byte.
Why it matters: Nothing measurable: paths are cold, short inputs on the open path (once per source, not per frame). Pure hygiene — the fix is three `out.push` calls or a static hex table; noted because this crate advertises "deliberately boring" and a per-byte format! is the least boring way to write a percent-encoder.
What correct looks like: `out.push('%'); out.push(hex_hi(byte)); out.push(hex_lo(byte));` with a 16-byte hex-digit table, no allocation beyond the output buffer.
Confidence: HIGH — Verification: read (allocation-free alternative trivially available; call frequency once per open, verified via caller map).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Scheme::parse` | 26-48 | REAL | — | RFC 3986 scheme grammar; ASCII-only predicates; lowercases; open set (plugin schemes parse). No length bound (input-bounded). Typed `Unsupported`@`Open` `uri.scheme.invalid` |
| `Scheme::as_str` | 50-52 | REAL | — | accessor |
| `impl Display for Scheme` | 55-59 | REAL | — | normalized form |
| `SECRET_QUERY_KEYS` (const) | 64-77 | REAL (incomplete) | S2 (F-001) | 10-key denylist; OAuth family missing |
| `REDACTED` (const) | 78-80 | REAL | — | `"***"` |
| `VERBATIM_PREFIX`/`VERBATIM_UNC_PREFIX`/`UNC_PREFIX` (consts) | 84-94 | REAL | — | correct Rust escaping (`\\?\`, `\\?\UNC\`, `\\`); UNC check ordered before the verbatim strip so `\\?\UNC\` can't fall into the drive path |
| `SourceUri::parse` | 132-149 | REAL | — | split_once(':') → Scheme::parse → reject ASCII control + space in rest; permissive about path content by documented design (scheme-specific validation is the access layer's job, MP §6); raw preserved verbatim. Edge traced: `file:` (empty rest) parses — harmless, resolver would refuse later |
| `SourceUri::from_file_path` | 191-235 | REAL | S3 (F-002) | Pure (no I/O); verbatim/UNC/drive/POSIX rules traced correct incl. byte-exact vectors; acceptance set ⊃ resolver's for `//` and `%00` inputs |
| `SourceUri::scheme` | 236-239 | REAL | — | accessor |
| `SourceUri::redacted` | 243-266 | REAL | S2 (F-001, query arm) | userinfo collapse correct (rsplit_once handles `@`-in-password); file-scheme basename-only correct; query arm only as good as the denylist; non-file schemes keep full paths (documented rule 2/4 choice — open question §9) |
| `SourceUri::raw` | 268-271 | REAL | — | doc restricts to access layer (convention only — no type enforcement; see §9) |
| `split_authority` | 275-287 | REAL | — | `unwrap_or("")` unreachable post-parse (fields private, parse guarantees ':') |
| `redact_file_path` | 289-311 | REAL | — | basename-only + `…/` prefix; `path=="/"` special case; trailing-slash paths yield `…/` with empty name (cosmetic, no leak); query/fragment handed to query redactor |
| `redact_query_values` | 313-349 | REAL | S2 (F-001) | first-`?`/first-`#` split; keys matched case-insensitively; `?token` (no `=`) becomes `token=***` (adds `=`, over-masking, harmless); a `?` inside a fragment is mis-parsed as query start (masks MORE, no leak); frag content passes verbatim by documented rule 4 |
| `is_windows_drive` | 351-365 | REAL | — | `^[A-Za-z]:[/\]`, len≥3; byte indexing safe (ASCII checks); `C:` (no sep) correctly NOT drive → relative refusal |
| `encode_file_path` | 368-379 | REAL | S4 (F-005) | keep-set = unreserved + `/` exactly; UTF-8 per-byte `%HH` uppercase correct |
| `impl Display for SourceUri` | 381-386 | REAL | — | redacted form (MP §12) |
| `impl Debug for SourceUri` | 389-394 | REAL | — | `SourceUri({redacted})` — no raw leak |
| `StreamId::new`/`index` | 405-417 | REAL | — | plain newtype; no invariant to check |
| `TrackId::new`/`handle` | 425-433 | REAL | — | plain newtype |
| `GenerationId::{FIRST,new,counter}` | 445-457 | REAL | — | const FIRST=0; no invariant |
| `GenerationId::next` | 464-477 | REAL | S4 (F-004) | checked_add, no wrap; typed ResourceLimit; metadata wrong-by-one |
| `MediaKind` (enum, 4 variants) | 481-497 | REAL | — | Audio/Video/Subtitle/Data; used by every demuxer (call-site map) |
| `LanguageTag` struct + doc | 500-507 | REAL | S3 (F-003, doc claim) | `parse` (509-553) straddles the chunk boundary — audited whole in the part2 file (F-003 there covers internals; the doc claim is filed here where the text lives) |

## 5. Test honesty
No tests in this range — the whole `mod tests` (655–1065) lies in part2 and is audited there (including the tests that exercise these functions: scheme 662-676, source-uri parse 678-706, from_file_path 709-850, redaction 911-997, ids 999-1017). Summary verdict from part2: assertions are real (exact vectors + leak negations), but (a) the round-trip proof runs against an incomplete mirror (part2 F-001) and (b) no baseline run executed them (fail-fast coverage hole, part2 §5).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Display … never embeds credentials or full user paths — URIs appear only through SourceUri::redacted (MP §12)" | lib.rs:32-34 | SHALLOW | mechanically true that all formatting routes through redacted(), but redacted() itself passes unlisted credential query values verbatim (F-001) — the promise is broken by the redactor, not bypassed |
| "every former hand-rolled `encode + "file://{…}"` site delegates here" | ids.rs:152-153 | REAL | `rg 'format!\("file://'` → only ids.rs:232 mints in production; demux_host.rs:940 is a test input vector (context verified) |
| "Redaction rules … all unit-tested against adversarial fixtures" | ids.rs:101 | SHALLOW | fixtures adversarial only against the implemented denylist (part2 F-002); suite never executed in baseline |
| "mismatched shapes are rejected early with an actionable error instead of silently matching nothing" | ids.rs:498-499 | FALSE at shipped-path level | all three consumers `.ok()`-discard (F-003) |
| "Pure (the L0 charter, doc 01 §2.2): no I/O, no canonicalization" | ids.rs:155-157 | REAL | from_file_path body 191-235 has no I/O/clock/fs calls (traced) |
| "The constructor never guesses a base: relative input is refused, not absolutized" | ids.rs:160-161 | REAL | refusal branch 226-227 + test 806 |
| "the set is deliberately open: protocols are plugins (MP §6)" | ids.rs:13-14 | REAL | Scheme::parse accepts any grammar-valid scheme; `source_uri_accepts_plugin_schemes` (698); MP §6 is the plugin doc — consistent |
| "a `\\` inside a POSIX filename is *data* — encoded `%5C`" | ids.rs:178-179 | REAL | encode branch traced; test 729 pins `%5C` |

## 7. Dead / duplicated / unreachable code
None dead in this range. Near-duplication worth noting: `Scheme::parse`'s grammar is tiny and single-sited (no other crate re-implements scheme validation — `rg` over the caller map shows all scheme handling goes through `Scheme`/`SourceUri`; this answers W01-A09's open question 3: input does NOT duplicate the parser, it only decodes — the split encode(here)/decode(input) is asymmetric by layering, not duplicated).

## 8. Dependencies & flags
Std-only crate (zero deps — W01-A09 manifest audit). No features, no cfg, no unsafe in this range. `use crate::error::{…}` and `use crate::time::TrackTiming` — both in-crate (L0 charter intact).

## 9. Open questions for the next agent
1. F-001 fix owner: media-types or streaming? The denylist lives here, but the threat model (which query keys real streaming sources carry) lives in streaming — the fix should be co-designed.
2. `SourceUri::raw()` is access-layer-only by doc convention, not by type. Is there any lint (lint-literals/architecture gate) that catches a UI crate calling `raw()`? If not, the MP §12 guarantee rests on review discipline — worth a wave-3 check of `rg "\.raw\(\)"` in ui/apps.
3. Non-file schemes keep full paths in `redacted()` (rule 2 is file-only). If a plugin access scheme ever carries local paths (`dvd://`, `smb://`), user paths leak into logs by design. No such scheme is shipped today (engine-info: file/http lanes only) — latent, not current.
4. Demux agent: decide whether the `.ok()` swallows (F-003) are intended; the baseline's `language: Some("voe")` garbage default suggests demux-side language handling needs the same hostility this file got.
