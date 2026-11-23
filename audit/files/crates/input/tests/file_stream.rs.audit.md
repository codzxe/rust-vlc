# AUDIT: crates/input/tests/file_stream.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-23 |
| Lines / functions | 219 / 12 (1 helper fn, 11 tests — 2 feature-gated) |
| File verdict | REAL (exacting integration suite over real files; both feature configurations executed by me) |
| Reachable from shipped entry point? | N/A (test binary). Runs in CI: `.github/workflows/ci.yml:234 cargo test --workspace --all-features` (covers the 2 `#[cfg(feature = "contract")]` tests). Locally verified both ways: default → 9 passed; `--features contract` → 11 passed. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
A model integration suite: every refusal is asserted down to the dotted code AND the `ErrorKind` (missing → `input.file.missing`/NotFound, directory and `/dev/null` → `not-a-regular-file`/Unsupported, http/rtsp/dvd schemes → `plugin.access.scheme-not-served`, mode-000 → `permission`/PermissionDenied with an honest privileged-bypass guard that FAILS if open succeeds without detected privilege). The behavioral tests are equally exact: a file that grows after open serves the growth (no false truncation), the 64 MiB read ceiling refuses typed without moving position and leaves the stream usable, and the read-ahead budget join clamps the 32 MiB request to the 16 MiB ceiling. I ran it in both feature configurations — 11/11 with contract. The only notes: the two contract tests are invisible to a plain `cargo test -p input` (documented, CI covers it), and the 32 MiB product default silently halving at the 16 MiB ceiling deserves a product-schema cross-check.

## 2. What actually works (verified)
— `cargo test -p input --test file_stream` → `9 passed; 0 failed`; `cargo test -p input --test file_stream --features contract` → `11 passed; 0 failed` (both executed in this audit). Mutation reasoning: weakening the typed-refusal mapping, the growth semantics, the ceiling check, or the clamp would each fail an exact assertion here.

## 3. Findings
F-001 [S4]
Where: crates/input/tests/file_stream.rs:29-36 (and :194-201)
Code:
```
#[cfg(feature = "contract")]
#[test]
fn real_file_passes_the_stream_contract() {
```
What is wrong: the two contract tests (stream contract + read_into contract) compile only under `--features contract`; a developer running plain `cargo test -p input` sees "9 passed" with no signal that the two strongest checks (varied chunk sizes, position consistency, sticky EOF, borrow-twin byte agreement) did not run. The gate is deliberate (audit F-2: keep conformance vocabulary out of release binaries) and CI runs `--all-features` (ci.yml:234), so nothing is dead — this is a discoverability note, not fakeness.
Confidence: HIGH — Verification: executed both configurations (9 vs 11).
F-002 [S4]
Where: crates/input/tests/file_stream.rs:203-212 (cross-file: crates/input/src/buffered.rs:49, product.toml:201)
Code:
```
    let access = input::FileAccess::with_read_ahead_budget(32 * 1024 * 1024).must();
    assert_eq!(access.buffer_capacity(), input::MAX_BUFFER_BYTES);
```
What is wrong: the product default (`product.toml:201 cache_demux_mib = 32 # demux read-ahead. schema 4..=512`) is double the hard ceiling (`MAX_BUFFER_BYTES = 16 MiB`, buffered.rs:49) — the request silently clamps, which this test asserts as correct behavior. Any user setting between 17 and 512 (the schema's advertised range) is inert: the schema promises a range the code will not honor. The test comment is honest about the clamp; the product schema is the dishonest half. What correct looks like: schema max 16, or the ceiling raised, or a typed refusal above the ceiling.
Confidence: HIGH — Verification: read both files + this test executed (clamping behavior confirmed by the passing assert).

## 4. Per-function table (test file)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open (helper) | 17-21 | REAL | — | FileAccess over parsed SourceUri |
| real_file_passes_the_stream_contract | 30-36 | REAL | S4 | F-001 feature-gated; ran under contract: pass |
| missing_file_is_typed_not_found | 39-49 | REAL | — | code + kind exact |
| directory_is_a_typed_rejection | 53-59 | REAL | — | |
| device_node_is_a_typed_rejection | 63-69 | REAL | — | /dev/null, unix |
| non_file_scheme_is_refused_typed | 73-86 | REAL | — | http/rtsp/dvd all `plugin.access.scheme-not-served` — corroborates the CLI scheme story (http handled earlier by ranged lane; others exit 69) |
| unreadable_file_is_typed_permission_denied | 90-119 | REAL | — | mode-000 + loud privileged-bypass guard (fails if open succeeds unprivileged) |
| growing_file_reads_past_the_open_time_length | 124-141 | REAL | — | real append, exact tail bytes, live size |
| empty_file_is_a_clean_immediate_eof | 144-153 | REAL | — | sticky EOF + seek |
| oversized_read_requests_are_typed_limit_errors | 159-185 | REAL | — | budget/attempted exact, position unmoved, stream usable |
| real_file_passes_the_read_into_contract | 194-201 | REAL | S4 | F-001 feature-gated; pass under contract |
| read_ahead_budget_opens_and_serves | 207-219 | REAL | S4 | F-002 clamp pinned |

## 5. Test honesty
All 11 tests would fail on the corresponding regression — every assertion is an exact code/kind/byte/length match; no tautologies; no mocks (real files, real permissions, real growth). The permission test explicitly refuses to silently pass under privilege.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "CI's --all-features runs it" | file_stream.rs:28 | REAL | ci.yml:234 `cargo test --workspace --all-features` |
| "the product.toml default (32 MiB) opens cleanly at the 16 MiB ceiling" | file_stream.rs:205-206 | REAL | executed; product.toml:201 confirmed |
| "growth is served, not truncated" (truncation only on shrink) | file_stream.rs:121-123 | REAL | executed |

## 7. Dead / duplicated / unreachable code
None. All 11 tests run (9 by default, 11 with the contract feature — both verified).

## 8. Dependencies & flags
`input` (FileAccess, MAX_READ_BYTES, contract feature), `media_types`, `plugin_api` (Access/SourceStream), `testkit`, `common` module (TempDir/TempFile/pattern/running_privileged — audited lane: not in my assignment). Feature `contract` is test/conformance-only by design.

## 9. Open questions for the next agent
- Who consumes `cache_demux_mib` end-to-end (product-config model/merge found; the FileAccess wiring site)? Does any typed refusal exist for settings over the 16 MiB ceiling, or is it clamp-only everywhere?
- common/mod.rs helpers (TempFile append, running_privileged) — separate audit lane if assigned.
