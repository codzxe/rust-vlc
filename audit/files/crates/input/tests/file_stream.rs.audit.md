# AUDIT: crates/input/tests/file_stream.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 219 / 9 tests + 1 helper |
| File verdict | REAL — integration tests over real files through the plugin-api trait; two of nine feature-gated but CI runs `--all-features` |
| Reachable from shipped entry point? | N/A (test binary). The tested surface (`FileAccess::open`) is the shipped CLI open path (demux_host.rs:394/422) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (quotes verified with unfiltered tools) |

## 1. One-paragraph verdict

Honest integration coverage of the file-access lane: typed rejections
for missing files, directories, device nodes, non-file schemes, and
permission-denied (with an explicit loud assert when running as root
rather than a silent pass, :109-117); growth-past-open-time-length
served (no false truncation, :124-141); empty-file EOF; the
64 MiB read ceiling as a public contract with position-preserved
refusal (:159-185); and the read-ahead budget join (:207-219), which
also documents-by-assert that a 32 MiB product budget clamps to the
16 MiB ceiling (`assert_eq!(access.buffer_capacity(),
input::MAX_BUFFER_BYTES)`). The two contract-suite tests are
feature-gated; CI's `--all-features` test step (ci.yml:234) covers
them. One hygiene note on the clamp-vs-reject posture.

## 2. What actually works (verified)

— All nine tests drive the real `FileAccess` → `BufferedReader` →
plugin-api `SourceStream` stack over real temp files (not mocks).
— The growth test verifies the exact bytes across the growth boundary
(:137-139) and the live `size()` (:140) — it would fail if the
truncation reference fired on growth.
— The ceiling test asserts the stream is still usable after the
refusal (:182-184) and that position never moved (:181).
— Feature gate cross-checked: `#[cfg(feature = "contract")]` (:29, :194)
and CI runs `cargo test --workspace --all-features`
(.github/workflows/ci.yml:234) — the gated tests do execute in CI.
(Whether the release workflow's feature set matches is Wave 17's
question.)

## 3. Findings

### F-001 [S4] The 32 MiB product read-ahead budget silently clamps to the 16 MiB ceiling — pinned as behavior, not decided as policy
Where: file_stream.rs:206-212

Code:
```
/// product.toml default (32 MiB) opens cleanly at the 16 MiB ceiling.
#[test]
fn read_ahead_budget_opens_and_serves() {
    let data = pattern(4096);
    let temp = TempFile::create("budget", &data).must();
    let access = input::FileAccess::with_read_ahead_budget(32 * 1024 * 1024).must();
    assert_eq!(access.buffer_capacity(), input::MAX_BUFFER_BYTES);
```
What is wrong: the test pins that the product's configured 32 MiB
budget silently becomes 16 MiB. Elsewhere this codebase's stated
posture for out-of-range product values is "reject, never clamp" (the
queue-plan doctrine, playback-engine/src/pipeline.rs:192-193; the
honest-parse rule, desktop-tauri app.rs:239-241). Here the clamp is
the chosen behavior and the test enshrines it — a config consumer
asking for 32 MiB gets 16 MiB with no error and no log. The test is
honest about what happens; the policy is inconsistent with the
project's own doctrine.
What correct looks like: either a typed refusal for budgets above the
ceiling (doctrine-consistent) or a documented clamp with an
observability note; the test then pins whichever is chosen.
Confidence: HIGH — Verification: read (clamp behavior asserted here;
`with_read_ahead_budget` lives in crates/input/src/file.rs — another
agent's file).

## 4. Per-function table (tests)

| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` (helper) | 17-21 | REAL | — | real FileAccess + SourceUri parse |
| `real_file_passes_the_stream_contract` | 29-36 | REAL | — | feature-gated; CI --all-features runs it |
| `missing_file_is_typed_not_found` | 39-49 | REAL | — | kind + code pinned |
| `directory_is_a_typed_rejection` | 53-59 | REAL | — | |
| `device_node_is_a_typed_rejection` | 63-69 | REAL | — | unix; /dev/null |
| `non_file_scheme_is_refused_typed` | 73-86 | REAL | — | http/rtsp/dvd all refused before fs access |
| `unreadable_file_is_typed_permission_denied` | 90-119 | REAL | — | root-bypass answered loudly, not silently |
| `growing_file_reads_past_the_open_time_length` | 124-141 | REAL | — | exact bytes across growth; live size |
| `empty_file_is_a_clean_immediate_eof` | 144-153 | REAL | — | sticky EOF + seek-to-0 |
| `oversized_read_requests_are_typed_limit_errors` | 159-185 | REAL | — | ceiling as public contract; position preserved; stream usable after |
| `real_file_passes_the_read_into_contract` | 194-201 | REAL | — | feature-gated |
| `read_ahead_budget_opens_and_serves` | 207-219 | REAL | S4 | pins the silent 32→16 MiB clamp (F-001) |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all nine ungated | YES | typed error kinds/codes, exact byte equality, position/size assertions over real files |
| the two gated | YES (when run) | shared contract suites (varied chunks, positions, EOF); executed by ci.yml:234 --all-features |

Not executed by me (no cargo); W00 ran `cargo test --workspace`
(1 751 pass). UNVERIFIED whether W00's run included --all-features
(the gated pair).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Feature-gated … CI's `--all-features` runs it" | :27-28, :192-193 | REAL | ci.yml:234 |
| "the product.toml default (32 MiB) opens cleanly at the 16 MiB ceiling" | :206 | REAL | the test asserts the clamp (F-001 discusses the policy) |

## 7. Dead / duplicated / unreachable code

None (the gated tests are CI-run, not dead).

## 8. Dependencies & flags

`input` (with `contract` feature under --all-features), `media_types`,
`plugin_api`, `testkit`, `common` module (tests/common — another
agent's file list may hold it; here it supplies TempDir/TempFile/
pattern/running_privileged).

## 9. Open questions for the next agent

- Wave 17 (CI): does the release workflow's feature set match
  ci.yml:234's --all-features, or can the `contract` pair silently
  skip a release gate?
- Wave 4: `with_read_ahead_budget`'s clamp lives in file.rs — confirm
  the clamp policy against product.toml's declared 32 MiB default
  (a product value that never takes effect is a config lie).
