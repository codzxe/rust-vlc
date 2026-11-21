# AUDIT: crates/commands/src/error.rs
| Field | Value |
|---|---|
| Auditor | W02-A06 |
| Date | 2026-11-21 |
| Lines / functions | 424 / 19 items (2 enums + 2 as_str + 2 Display + cap const + struct + 3 methods + Display + Error impl + 3 redaction fns + test trait + 8 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen <any failing command>` → apps/cli/src/dispatch.rs:62 `lumen_dispatch` → `route` → e.g. `unserved()` at 164-174 constructs `CommandError::new(Stage::Control, ErrorCode::UnsupportedForHost, …).with_detail(…)`; also every `Err` result the CLI prints) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 1 |
| Commands run | `sed -n` on all quoted ranges; `rg -n "redact\("` over apps+crates → sole caller error.rs itself; `rg -n "fn redacted"` → media-types/src/ids.rs:243 (the parallel URI redactor); `rg -n "CommandError \{"` → no production struct-literal bypass; `rg -n "SourceSpec::Url"` -A12 apps/cli/src/demux_host.rs:399-440 (blocked-scheme message embeds raw `{url}`); baseline log rg `error::tests::` at 1354-1361 → 8/8 ok (commands crate block, test-workspace.log.txt:1303) |

## 1. One-paragraph verdict
This is the §2.13 typed error set plus the V8 redaction machinery, and the core of it is real: two wire-stable enums with serde/`as_str` duals pinned by an exhaustive test that passed in the baseline run, a `CommandError` whose every in-tree construction routes through `redact()`, and eight honest tests (round-trip, redaction positives, cap, no-op passthrough) that all executed green. The defects are in the redaction net's blind spots and its cost model: `collapse_paths`' URL-protection rule also shields `file://` URLs, so a local path inside a URL passes through untouched (the V8 "no full user paths" rule is only upheld because callers pre-redact with a *different*, URI-aware implementation in media-types — two redaction truths that can drift); Windows paths are not covered at all; `mask_userinfo` is O(n²) on inputs without `"://"` and `MESSAGE_CAP` truncates the output only after the full unbounded scan; and the `VariantCount` cfg(test) consts are a hand-maintained number compared against itself — a fake guard that cannot detect a new unpinned variant.

## 2. What actually works (verified)
- All 8 tests passed in baseline run #3 (commands-crate block): `audit/runs/baseline/test-workspace.log.txt:1354-1361` — `error::tests::{command_error_roundtrips_on_the_wire, details_are_redacted_too, error_construction_collapses_absolute_paths, error_construction_redacts_userinfo, long_messages_are_capped_without_panicking, plain_messages_pass_through_verbatim, serde_wire_names_match_as_str, stage_and_code_wire_names_are_stable}` all `ok`.
- Wire-name duality verified: `serde_wire_names_match_as_str` (309-352) enumerates all 13 Stage + all 18 ErrorCode variants (counts cross-checked by hand against the enums at 15-29 and 63-103) and asserts serde output equals `as_str` — would fail on any spelling divergence in the current set.
- Redaction-at-construction verified: `rg "CommandError \{"` finds no production struct-literal bypass; every constructor in apps/ and crates/ goes through `new`/`with_detail`, both of which call `redact` (167, 175).
- `truncate` (261-272): byte cap with char-boundary backoff + `…` marker; the CJK case (413-416) proves no panic on multi-byte boundaries.

## 3. Findings

### F-001 [S3] `redact()` cannot redact paths inside URLs — `file://` local paths pass through untouched; Windows paths not covered at all
Where: crates/commands/src/error.rs:229-234 (rule), 237-252 (blind spot)
Code:
```rust
/// `/home/alice/dir/file.mkv` → `…/file.mkv` — absolute POSIX paths
/// collapse to the final component (MP §12: no full user paths). A '/'
/// only starts a path candidate at a token boundary (string start or
/// after whitespace) so URL path separators (`scheme://host/x`) are
/// never mistaken for filesystem paths.
```
```rust
    'outer: while !rest.is_empty() {
        let boundary = out.is_empty() || out.ends_with(char::is_whitespace);
        if rest.starts_with('/') && boundary {
```
What is wrong: the boundary rule protects every slash that follows a scheme — including all three slashes of `file:///home/alice/x.mkv`. Traced by hand through both passes: `mask_userinfo` finds `://` but the authority segment (`/home/…` — the path is the authority position for file URLs) contains no `@`, so nothing is masked; `collapse_paths` then sees each `/` with `out` ending in `file:`, `file:/`, `file://` — never whitespace — so no path is collapsed. The full user path inside a `file://` URL survives `redact()` verbatim. The same trace shows Windows paths (`C:\Users\alice\x.mkv`) are untouched (no leading `/`; the doc says "POSIX paths" while the product ships Windows MSI packaging). The V8 rule currently holds only because callers pre-redact: the file lane names sources via `SourceUri::redacted()` (crates/media-types/src/ids.rs:243 — a URI-aware implementation with its own `redact_file_path`), i.e., there are TWO redaction implementations, and the message-level net in this file is the weaker one. Any future message that embeds a raw `file://` URL (the blocked-scheme arm at apps/cli/src/demux_host.rs:432-440 already embeds raw `{url}` for non-file schemes) leaks the machine path. Also: `mask_userinfo`'s `find("://")` is case-sensitive, so `HTTP://user:pass@host` is not masked.
Why it matters: MP §12's "no full user paths" is a privacy rule for the wire (control server, web host). The type-level guarantee this module advertises ("no `Display` path can leak credentials or full user paths", 195-196) is false for URL-embedded and Windows paths; it is currently saved by caller discipline, not by the type.
What correct looks like: either route message redaction through the URI-aware logic (detect `scheme://` prefixes and delegate), or add a file-URL rule to `collapse_paths` (a `/` after `file://` authority *is* a local path) and a Windows-drive rule; pin it with a test asserting `redact("see file:///home/alice/x.mkv")` contains no `/home/`.
Confidence: HIGH — Verification: read + hand trace of both passes; no runtime execution (cargo forbidden). No in-tree leak path proven active (blocked-scheme arm never receives `file:`; file lane pre-redacts) — hence S3 latent, not S2.

### F-002 [S3] `mask_userinfo` is O(n²) and `MESSAGE_CAP` truncates only after the full unbounded scan
Where: crates/commands/src/error.rs:208-226 (scan), 197-201 (cap-last order), 137-139 (claim)
Code:
```rust
        let cut = rest.char_indices().nth(1).map_or(rest.len(), |(i, _)| i);
        out.push_str(&rest[..cut]);
        rest = &rest[cut..];
```
What is wrong: when a message contains no `"://"`, every iteration of the `'outer` loop runs `rest.find("://")` — a linear scan of everything remaining — and then advances exactly one character (the `cut` above). For a message of length n with no `"://"`, total work is n + (n−1) + … = O(n²). `redact` runs `mask_userinfo` and `collapse_paths` on the FULL input and only then truncates to `MESSAGE_CAP` (197-201), so the 1 KiB cap bounds the stored message, not the scan cost. A multi-megabyte error message (e.g., a file-controlled metadata blob embedded in a `format!`) turns construction into a pathological CPU burn; the comment's "stops any accidental diagnostic firehose" (137-138) is true only for memory, not time.
Why it matters: error construction sits on the CLI's failure paths and the control server's hot refusal path; a hostile or merely pathological input string converts a cheap failure into a multi-second stall.
What correct looks like: bound the input first (e.g., cap at 64 KiB — far above any sane message — then redact, then truncate to 1024), and advance the no-match case past the scanned `"://"` position or by the full `find` miss length instead of one char.
Confidence: HIGH on the complexity (loop structure verified by read); MEDIUM on real-world trigger (no proven path passes unbounded file-controlled strings into messages today — in-tree `format!` messages are short).
Confidence entry: HIGH/MEDIUM — Verification: read; no execution (cargo forbidden).

### F-003 [S3] `VariantCount` is a fake guard; the wire-name tests cannot fail when a new variant ships unpinned
Where: crates/commands/src/error.rs:274-286, 300-305
Code:
```rust
// Variant-count helpers used by the wire-name stability test.
#[cfg(test)]
trait VariantCount {
    const VARIANT_COUNT: usize;
}
#[cfg(test)]
impl VariantCount for Stage {
    const VARIANT_COUNT: usize = 13;
}
#[cfg(test)]
impl VariantCount for ErrorCode {
    const VARIANT_COUNT: usize = 18;
}
```
```rust
        assert_eq!(Stage::VARIANT_COUNT, 13);
        assert_eq!(ErrorCode::VARIANT_COUNT, 18);
```
What is wrong: `VARIANT_COUNT` is a hand-maintained constant compared against a literal of the same value in the test — it is derived from nothing. Add a 14th `Stage` variant and the assert still passes (const still says 13, test still says 13); nothing in the type system or the test computes the real variant count. Likewise `serde_wire_names_match_as_str` (309-352) is an open enumeration: a new variant not added to the list is simply never pinned, and no failure occurs. The comment at 274 promises these helpers serve the "wire-name stability test" — the stability contract for additions is mechanically unenforced.
Why it matters: the whole point of the 1.D4 wire-name pinning is that a new or renamed variant cannot silently diverge between serde's kebab-case derive and the hand-written `as_str`. Today a new `ErrorCode` variant with a typo in `as_str` (e.g. `"resourcelimit"`) ships green: the schema (schemars/serde) says `resource-limit`, `Display`/`summary()` say `resourcelimit`, and every telemetry key built from `summary()` drifts.
What correct looks like: a macro that generates both the enum and an ALL array (like `CommandId::ALL` in catalog/ids.rs, which the spec.rs tests DO zip against), or `strum`-style derive; the test then iterates the ALL array and cannot miss additions.
Confidence: HIGH — Verification: read; the catalogs' own ALL-array pattern (spec.rs:909-916) proves the repo knows the correct shape.

### F-004 [S4] `with_detail` values are not capped; `Deserialize` bypasses redaction
Where: crates/commands/src/error.rs:172-177, 152 (derive)
Code:
```rust
    /// Attach a machine-readable hint (V2's `allowed_range`, §2.13's
    /// `format` etc.); key and value are redacted.
    pub fn with_detail(mut self, key: &str, value: impl Into<String>) -> Self {
        self.details.insert(redact(key), redact(&value.into()));
        self
    }
```
What is wrong: two rule gaps. (1) `MESSAGE_CAP` applies to `message` only — a detail value of any length (and any number of details) is stored verbatim after redaction, so the "diagnostic firehose" cap does not cover the map that rides the same wire object. (2) `CommandError` derives `Deserialize` (152), and deserialization constructs the struct without `redact` — the doc claim "display-safe **by construction**" (146) holds for `new`/`with_detail` but not for the serde path. In-tree, only tests/conformance deserialize `CommandError` (rg found no production `from_str::<CommandError>`), so this is a doc-precision gap, not an active hole.
Why it matters: the asymmetry invites a future caller to stuff a long blob into a detail believing the type enforces budgets; the serde bypass voids the "by construction" wording the module's security story leans on.
What correct looks like: cap detail values (e.g., 256 bytes) in `with_detail`, cap the detail count, and either implement `Deserialize` through a validating constructor or document that deserialized instances are trusted-wire only.
Confidence: HIGH — Verification: read; rg for external `redact(` callers and deserialization sites.

## 4. Per-function table
| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Stage` enum (13 variants) | 15-29 | REAL | — | Serde kebab-case derive; distinct from media-types' PipelineStage (claim verified: pipeline_host.rs:149 uses PipelineStage for engine errors). |
| `Stage::as_str` | 31-49 | REAL | — | 13 hand-written kebab names; pinned vs serde by test 309-328 for all current variants. |
| `Stage` Display | 52-56 | REAL | — | Delegates to as_str. |
| `ErrorCode` enum (18 variants) | 61-103 | REAL | — | 7 media classes + 11 command-layer classes; doc references (V1/V2/V5/V6/V8) consistent with detail usage found (allowed_range mentioned at 172-173). |
| `ErrorCode::as_str` | 105-129 | REAL | — | 18 hand-written names; pinned by test 329-351. |
| `ErrorCode` Display | 131-135 | REAL | — | Delegates to as_str. |
| `MESSAGE_CAP` | 137-139 | REAL | S3-adjacent (F-002) | Caps stored message only; scan-before-cap ordering is the defect. |
| `CommandError` struct | 152-159 | REAL | S4 (F-004) | camelCase serde rename is a no-op for these single-word fields (round-trip test pins exact JSON). |
| `CommandError::new` | 161-170 | REAL | — | Redacts at construction ✓ (claim verified; no struct-literal bypass in production code). |
| `CommandError::with_detail` | 172-177 | REAL | S4 (F-004) | Redacts key+value ✓ (test 399-405); no length/count cap. |
| `CommandError::summary` | 179-183 | REAL | — | `[stage.code]`; used by tests and telemetry keys. |
| `CommandError` Display | 186-190 | REAL | — | `summary: message` (test 384). |
| `impl std::error::Error` | 192 | REAL | — | No `source()` by design (redaction forbids chains). |
| `redact` | 197-201 | REAL | S3 (F-001, F-002) | Order: mask → collapse → truncate. `pub` with no external callers (rg) — the parallel implementation is `SourceUri::redacted` (media-types/ids.rs:243). |
| `mask_userinfo` | 205-227 | REAL | S3 (F-002), F-001 note | Correct for lowercase-scheme URLs with userinfo (tests 374-385, 399-405 baseline-passed); case-sensitive `"://"`; O(n²) no-match scan. |
| `collapse_paths` | 234-258 | REAL | S3 (F-001) | Correct for bare POSIX paths at token boundaries (test 388-396); blind to file:// URL paths and Windows paths; trailing-`/` directory path collapses to bare `/` (over-redaction, harmless). |
| `truncate` | 261-272 | REAL | — | Byte cap + boundary backoff + `…`; output ≤ cap+3 bytes (test allows 1028 ✓). |
| `VariantCount` trait/impls | 275-286 | STUB-as-guard | S3 (F-003) | cfg(test)-only consts compared to themselves; guards nothing. |
| tests ×8 | 292-424 | REAL | S3 (F-003) | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| stage_and_code_wire_names_are_stable (292-305) | YES for the 4 spot-checked spellings; NO for additions | The VARIANT_COUNT asserts compare a hand-kept const to a hand-kept literal — adding a variant without touching either passes (F-003). |
| serde_wire_names_match_as_str (309-352) | YES for all 31 current variants | Exhaustive today (13+18 verified against the enums); open list — a new variant goes unpinned silently (F-003). |
| command_error_roundtrips_on_the_wire (354-371) | YES | Exact JSON string assert + structural round-trip; catches any serde rename or field drift. |
| error_construction_redacts_userinfo (373-385) | YES | Positive (`***@example.org`) AND negatives (`alice`/`secret` absent). |
| error_construction_collapses_absolute_paths (387-396) | YES | Positive (`…/movie.mkv`) and negative (`/home/` absent). Does NOT cover file:// or Windows paths — F-001 has no negative test. |
| details_are_redacted_too (398-405) | YES | Both a URL detail and a path detail asserted redacted. |
| long_messages_are_capped_without_panicking (407-417) | YES | 5000-byte and CJK-boundary inputs; cap + `…` asserted. |
| plain_messages_pass_through_verbatim (419-423) | YES | Redaction no-op pinned (guards against over-redaction). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "message is display-safe by construction … routes every message through redact" | error.rs:146-147 | REAL (in-tree) / S4 caveat (F-004) | All production constructors go through `new` (rg: no struct-literal bypass); serde Deserialize path bypasses. |
| "masks userinfo in URLs (`user:pass@` → `***@`)" | error.rs:147-148 | REAL | Test 374-385, baseline-passed; case-sensitivity gap noted (F-001). |
| "collapses absolute paths to their final component (`/home/alice/x.mkv` → `…/x.mkv`)" | error.rs:148-150 | REAL (bare POSIX) / FALSE-by-omission (URL-embedded, Windows) | Test 388-396 passes; hand trace shows `file://` paths and `C:\…` paths survive (F-001). |
| "details values pass through the same redactor" | error.rs:151 | REAL | with_detail:175; test 398-405. |
| "1 KiB … stops any accidental diagnostic firehose" | error.rs:137-138 | SHALLOW | Caps stored message only; scan unbounded (F-002), details uncapped (F-004). |
| Wire names of Stage/ErrorCode are stable kebab strings | error.rs:8-12, 58-60 | REAL (current set) | Test 309-352 all-variant pin, baseline-passed; not mechanically enforced for additions (F-003). |

## 7. Dead / duplicated / unreachable code
- `pub fn redact` (197): zero external callers (rg over apps+crates) — exported for a future that hasn't arrived; the live redaction-for-sources path is `SourceUri::redacted` (media-types/src/ids.rs:243), a second, URI-aware implementation. Two redaction truths, one of them (this one) weaker — the duplication is the root of F-001.
- `VariantCount` (275-286): cfg(test)-only, guards nothing (F-003) — dead weight posing as a pin.
- Nothing unreachable otherwise; every other item has live callers.

## 8. Dependencies & flags
- schemars + serde only (lines 3-6); no feature gates; no unsafe; no panics on any input (all slicing preceded by length checks; `unwrap_or` fallbacks at 243, 214).
- BTreeMap for details: deterministic serialization order — verified by the exact-JSON test (363-366). Correct choice for a wire type.

## 9. Open questions for the next agent
- media-types wave (ids.rs): does `SourceUri::redacted()`'s test suite cover the same adversarial set this module's docs claim ("adversarially tested", ids.rs:242-243)? If the two redactors ever disagree, the wire shows different spellings for the same source in message vs detail.
- CLI wave: demux_host.rs:432-440 embeds the raw `{url}` in blocked-scheme messages — confirm no reachable scheme embeds a local path (file is matched earlier; a malformed `file:` URL failing `SourceUri::parse` at 409-414 depends on the parse error's Display — if it echoes the raw input, F-001 becomes an active leak).
- Wave owning wire compat: if F-003's ALL-array pattern is adopted, the schema/checksum regeneration (codegen.rs) will need a coordinated bump.
