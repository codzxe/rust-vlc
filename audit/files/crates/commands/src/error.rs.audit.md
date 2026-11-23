# AUDIT: crates/commands/src/error.rs
| Field | Value |
|---|---|
| Auditor | W02-A05 |
| Date | 2026-11-23 |
| Lines / functions | 424 / 21 (12 fns/methods/impls, 1 test-only trait, 8 tests) |
| File verdict | REAL (with a genuine redaction bypass in `mask_userinfo`) |
| Reachable from shipped entry point? | YES — `apps/cli` is package `lumen-cli` (apps/cli/Cargo.toml:33) and constructs `CommandError` in dispatch.rs/run.rs; same type is the wire vocabulary for desktop/web/control-server |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/commands-84df364c190318ab error` -> 9 passed, exit 0; rg/sed cross-refs |

## 1. One-paragraph verdict
The §2.13 common error set is real and exactly matches doc 05 §2.13's enums (13 Stage + 18 ErrorCode variants, verbatim; wire names pinned by an executed test, 9/9 pass). The redaction pipeline (`redact` = mask_userinfo → collapse_paths → truncate) is real code with real tests — but it is NOT the unconditional guarantee the module doc comment sells: `mask_userinfo` stops at the FIRST `@` in a URL authority, so any credential containing a literal `@` (accepted by curl/VLC/WHATWG parsers, which split at the LAST `@`) leaks the password tail into the "display-safe" message (S2); `collapse_paths` only collapses whitespace-delimited POSIX paths, so `file:///home/…` URLs, punctuation-adjacent paths (`(…)`, `=`, quotes) and Windows `C:\…` paths pass through verbatim while the comment claims "display-safe by construction" (S3). Additionally, doc 05 §6.2's additive-change contract ("new ErrorCode/Stage values — clients must degrade unknown codes to a generic failure") has NO mechanism here: serde rejects unknown enum variants, so a future v1.x error code makes the entire `CommandError`/`WireResponse` fail to deserialize on a v1 client instead of degrading (S3).

## 2. What actually works (verified)
— `./target/debug/deps/commands-84df364c190318ab error` -> "test result: ok. 9 passed" exit 0: wire-name stability (both enums, all variants), serde↔as_str equality, wire round-trip golden, single-`@` userinfo masking, whitespace-path collapse, details redaction, 1024-byte cap with CJK char-boundary safety, plain-text passthrough.
— Doc 05 §2.13 enum definitions match this file variant-for-variant (doc 05 L338-357 vs error.rs L15-29, L63-103).

## 3. Findings
F-001 [S2]
Where: crates/commands/src/error.rs:210-219
Code:
```
let limit = tail.find(['/', '?', '#', ' ']).unwrap_or(tail.len());
if let Some(at) = tail[..limit].find('@') {
    out.push_str(head);
    out.push_str("***@");
    rest = &tail[at + 1..];
```
What is wrong: masks only up to the FIRST `@` in the authority. curl/WHATWG/VLC parse userinfo as everything before the LAST `@` before the path, and tolerate an unencoded `@` inside credentials. Trace (code read, line-by-line): input `http://user:p@ss@host/` → output `http://***@ss@host/` (password tail `ss` leaks); input `http://user@example.com:secret@evil.com/x` → output `http://***@example.com:secret@evil.com/x` (password `secret` leaks). Doc 05 V8 (L40) is unconditional: "URLs have credentials stripped". Why it matters: error messages built from media/playlist URLs (untrusted input, V7) reach the UI, logs, and — for `media.get-info` failures — unauthenticated read-only control peers. What correct looks like: scan for the LAST `@` in `tail[..limit]` and mask everything from `://` to it. Confidence: HIGH — Verification: read/traced; NOT executed (no executable harness without building outside budget — cargo-build-budget). The 8 in-tree tests only exercise single-`@` URLs, so they cannot catch this.
F-002 [S3]
Where: crates/commands/src/error.rs:238-249 (+ module doc 144-151)
Code:
```
let boundary = out.is_empty() || out.ends_with(char::is_whitespace);
if rest.starts_with('/') && boundary {
```
What is wrong: path collapse only triggers at string start or after whitespace. (a) `file:///home/alice/secret.mkv` passes `mask_userinfo` untouched (no `@`) and `collapse_paths` untouched (the `/`s follow `file:`, not whitespace) — the FULL local path survives `redact` verbatim; (b) `open(/home/alice/x.mkv)`, `path=/home/...`, `"/home/..."` — any punctuation-adjacent path leaks; (c) Windows `C:\Users\...` paths are not handled at all. The module comment (L146-150) claims `message` is "display-safe by construction" and "collapses absolute paths to their final component" — an overclaim. Doc 05 V8 (L40) permits absolute paths to local users but forbids them "over the control server to unauthenticated peers": that channel exists (read-only mode serves `media.get-info` failures to unauthenticated peers, server.rs F-001 context), though the control server is not shipped today (product gate off). What correct looks like: redact `file://` paths and paths after punctuation, or scope the doc-comment claim to what the code does. Confidence: HIGH — Verification: read/traced (mask_userinfo fall-through: when `://` is found but no `@` precedes the limit, control falls to the one-char copy, L221-224 — verified by sed).
F-003 [S3]
Where: crates/commands/src/error.rs:61-62, 13-14 (serde derives); doc 05 §6.2
Code:
```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
```
What is wrong: doc 05 §6.2 ("Additive-change rules (within a major)") allows "new `ErrorCode`/`Stage` values (clients must degrade unknown codes to a generic failure with the message)" and states "a v1 client that ignores everything it does not recognize must keep working against any v1.x engine". `Deserialize` here rejects unknown variant strings, so a v1.x-only code fails parsing of the WHOLE `CommandError` (and any enclosing `WireResponse`) — there is no degradation path, no `#[serde(other)]`/unknown catch-all, and no version guard. The additive-change contract is unimplemented in the one type both ends share. Latent today (only v1 codes exist in tree), guaranteed violation the day a code is added. What correct looks like: an `Unknown(String)`/`#[serde(other)]`-style fallback or a lossy parse mode. Confidence: HIGH — Verification: read; doc 05 §6.2 quoted via sed.
F-004 [S4]
Where: crates/commands/src/error.rs:243
Code:
```
let basename = path.rsplit('/').next().unwrap_or(path);
```
What is wrong: `rsplit` on any `&str` always yields at least one element; `unwrap_or(path)` is dead code that suggests a None case that cannot exist. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: crates/commands/src/error.rs:261-263
Code:
```
fn truncate(text: &str, cap: usize) -> String {
    if text.len() <= cap {
        return text.to_string();
```
What is wrong: the common path (message under cap — i.e. nearly every error constructed in the engine) pays a fresh `String` allocation per redaction; `mask_userinfo`/`collapse_paths` likewise return owned Strings on the no-match path. Cosmetic for error rates, but this is the hottest constructor in the command layer. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Stage::as_str | 33-49 | REAL | — | 13 variants; test-pinned |
| Display for Stage | 52-56 | REAL | — | delegates to as_str |
| ErrorCode::as_str | 107-128 | REAL | — | 18 variants; test-pinned |
| Display for ErrorCode | 131-135 | REAL | — | delegates |
| CommandError::new | 163-170 | REAL | — | redacts message at construction (the V8 choke point — good design) |
| CommandError::with_detail | 174-177 | REAL | — | redacts key AND value (tested) |
| CommandError::summary | 181-183 | REAL | — | `[stage.code]` |
| Display for CommandError | 186-190 | REAL | — | |
| std::error::Error for CommandError | 192 | REAL | — | no source (raw chains belong to log sink per doc) |
| redact | 197-201 | REAL | S2/S3 | pipeline; gaps in F-001/F-002 |
| mask_userinfo | 205-227 | REAL | S2 | first-`@`-only masking (F-001); char-boundary-safe slicing (no panics) |
| collapse_paths | 234-258 | REAL | S3 | whitespace-boundary-only (F-002); no panic paths |
| truncate | 261-272 | REAL | S4 | clones on the common path (F-005); boundary-safe |
| VariantCount (test-only) | 275-286 | REAL | — | forces conscious variant-count updates |
| test stage_and_code_wire_names_are_stable | 292-305 | HONEST | — | fails on rename or silent variant add |
| test serde_wire_names_match_as_str | 309-352 | HONEST | — | full enum coverage, both enums |
| test command_error_roundtrips_on_the_wire | 354-371 | HONEST | — | golden JSON string |
| test error_construction_redacts_userinfo | 373-385 | SHALLOW-ADJACENT | — | single-`@` only; blind to F-001 |
| test error_construction_collapses_absolute_paths | 387-396 | SHALLOW-ADJACENT | — | whitespace-delimited only; blind to F-002 |
| test details_are_redacted_too | 398-405 | HONEST | — | |
| test long_messages_are_capped_without_panicking | 407-417 | HONEST | — | incl. CJK boundary |
| test plain_messages_pass_through_verbatim | 419-423 | HONEST | — | no false positives |

## 5. Test honesty (if tests exist)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| stage_and_code_wire_names_are_stable | YES | pins spellings + counts 13/18 |
| serde_wire_names_match_as_str | YES | serde vs as_str, all variants |
| command_error_roundtrips_on_the_wire | YES | byte-exact golden |
| error_construction_redacts_userinfo | YES (for single-`@`) | blind to multi-`@` (F-001) |
| error_construction_collapses_absolute_paths | YES (for "…: /path") | blind to file://, punctuation, Windows (F-002) |
| details_are_redacted_too | YES | |
| long_messages_are_capped_without_panicking | YES | cap + multibyte |
| plain_messages_pass_through_verbatim | YES | guards over-redaction |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| Stage/ErrorCode enums are "the §2.13 common error set" | doc 05 §2.13 (L338-357) | REAL | variant-for-variant match; executed test pins wire names |
| "`message` is redaction-safe display text (V8)" | doc 05 §2.13 rule 2 (L363) | PARTIAL | single-`@` URLs + whitespace POSIX paths covered (tests); multi-`@` creds leak (F-001); file:/// + punctuation + Windows paths leak (F-002) |
| "URLs have credentials stripped" | doc 05 V8 (L40) | FALSE in edge cases | F-001 trace |
| "absolute paths … never emitted over the control server to unauthenticated peers" | doc 05 V8 (L40) | UNENFORCED at this layer | redact() lets file:/// and punctuation-adjacent paths through; the control-server channel exists (read-only media.get-info) but is unshipped (product gate) |
| "new ErrorCode/Stage values (clients must degrade unknown codes to a generic failure)" | doc 05 §6.2 (L~2 of section) | NOT IMPLEMENTED | no serde fallback (F-003) |
| Strings ≤ 1024 (names) / 4096 (paths) `[PROPOSAL]` | doc 05 V6 (L38) | REAL (partial) | MESSAGE_CAP=1024 on messages (L139); per-kind path/name caps live elsewhere |

## 7. Dead / duplicated / unreachable code
- `unwrap_or(path)` at L243 — dead branch (F-004).
- No unreachable/unused items otherwise; `Stage`/`ErrorCode` both consumed widely (rg: CommandError referenced in apps/cli, apps/desktop, apps/web, apps/control-server, crates/*).

## 8. Dependencies & flags
schemars (JsonSchema — feeds schemas/commands.schema.json), serde/serde_json. No feature flags; no cfg gates except `#[cfg(test)]` VariantCount. No unsafe, no unwrap/expect outside tests (tests use unreachable! on impossible branches). Crate `commands` is in the release-lane dependency graph via lumen-cli.

## 9. Open questions for the next agent
- Does any engine site format error messages with `format!("open({}) failed", path)` (parenthesis-adjacent) — i.e. is F-002's punctuation gap live in real messages? Audit the error-construction sites in crates/engine*/apps/cli.
- §6.2 degradation: does client-side (web/UI) parsing of WireResponse have its own unknown-code fallback that rescues F-003? apps/web/src/protocol.rs is the place to check.
- Who enforces the V6 4096-byte path cap? If nowhere, note it as a declared-but-unread config rule for the hygiene ledger.
