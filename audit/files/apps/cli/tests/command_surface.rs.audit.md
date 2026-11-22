# AUDIT: apps/cli/tests/command_surface.rs

| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-22 |
| Lines / functions | 171 / 8 (1 helper + 7 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test file) — it drives the REAL shipped binary (`CARGO_BIN_EXE_lumen`, line 11) through argv + piped stdin, i.e. it exercises `main.rs` end to end including process exit codes. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:4 |

Commands run:
- Full Read of the file (171 lines) + `rg -n "area|Areas" apps/cli/src/help.rs` (help is schema-generated; the test's 6-area pin is a presence check)
- Not executed (no cargo); W00 baseline (1751 pass / 2 fail / 25 ignored) is the only run evidence.

## 1. One-paragraph verdict

A genuine real-binary integration suite: every test spawns the actual `lumen` executable, feeds it argv and (for machine mode) a piped NDJSON script, and asserts on stdout/stderr AND the process exit code — no mocks of the subject anywhere. The assertions are specific (version string equality, exit code 64 for a bad help area, the wire envelope's camelCase bytes, the not-ready error triple). The defects are all test-quality tier: a brittle hard-coded catalog count ("65") whose own comment admits it was wrong (64) across multiple waves before being caught; the engine-info test enshrining host.rs's static string table instead of deriving expectations; no machine-mode SUCCESS-payload assertion (only the error envelope and an event-kind presence check); and an under-specified areas pin (6 presence checks, no completeness).

## 2. What actually works (verified)

- The `run` helper (10-28) exercises the true process boundary: `Command::new(env!("CARGO_BIN_EXE_lumen"))` with piped stdio — these tests fail if the binary fails to build, print, or exit-code correctly.
- Exit-code assertions are real (`assert_eq!(code, 0, …)`, `assert_eq!(code, 64, …)` — the sysexits-class usage error).
- The wire-envelope test asserts the byte shape (`"requestId":42`, `"code":"not-ready"`, `"stage":"session"`) — it pins serde field naming, not just "some JSON came out".
- The `--json` machine-mode test opens the committed AVI fixture through the real open path and asserts the state-changed and tracks-list-ready events appear on the NDJSON stream.

## 3. Findings

### F-001 [S4]
Where: apps/cli/tests/command_surface.rs:95-100
Code:
```rust
    // 65 since w31.1 minted session.set-frame-channel (the frame
    // channel bind for the GUI canvas presenter); the wave-8
    // playback-operations verbs (frame-step, chapter nav, Go-to-Time,
    // bookmarks) had made it 64. w31.1 missed THIS pin (its checksum
    // sibling was fixed in w31.6); the consolidation round catches it.
    assert!(out.contains("ok: 65 command(s)"), "{out}");
```
What is wrong: the catalog cardinality is pinned as a prose substring with a hand-maintained number — and the comment itself documents that this exact pin was WRONG (said 64 after the catalog grew to 65) across at least two waves before someone caught it. A count that must be edited in lockstep with the catalog is a drift machine; it has already drifted once.
What correct looks like: derive the expected count from `commands::catalog::catalog_data().len()` (the test links against the same crate) and assert equality — the per-id loop below already does the presence half correctly.
Confidence: HIGH — Verification: read.

### F-002 [S4]
Where: apps/cli/tests/command_surface.rs:133-136
Code:
```rust
    assert!(out.contains("first-vertical:avi-mjpeg-pcm"), "{out}");
    assert!(out.contains("audio.null-device"), "{out}");
    assert!(out.contains("video.software-compositor"), "{out}");
    assert!(out.contains("mjpeg, pcm_s16le"), "{out}");
```
What is wrong: the engine-info test pins host.rs's HARDCODED string table (host.rs F-007: decoders and features are literals, not derived from the provider). The test enshrines the hardcoding — it verifies the strings match the strings, not that the build's decoder set is what the wire claims. When a decoder is added, this test stays green while `engine-info` understates reality.
Confidence: HIGH — Verification: read + host.rs:1631-1661.

### F-003 [S4]
Where: apps/cli/tests/command_surface.rs:141-171
Code:
```rust
    assert!(line.contains("\"requestId\":42"), "{line}");
    // A seek with no session is the honest not-ready, in the
    // documented error envelope shape.
    assert!(line.contains("\"error\":"), "{line}");
```
What is wrong: machine mode's SUCCESS envelope is never payload-asserted — the seek test pins only the error shape; the `--json` test asserts `requestId` echo and two event-kind substrings but never a `Completed` response body (e.g. `media.get-current-time` answering a number). A regression that broke every success payload while keeping error envelopes would pass this suite.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: apps/cli/tests/command_surface.rs:49-58
Code:
```rust
    for area in [
        "engine",
        "global",
        "interface",
        "performance",
        "playback",
        "privacy",
    ] {
        assert!(out.contains(area), "area {area} missing:\n{out}");
```
What is wrong: the generated-areas pin is a presence check of 6 area names — no completeness, no count, no ordering. If the schema drops or adds areas (or help regresses to printing a stale copy), only the 6 pinned names are protected.
Confidence: HIGH — Verification: read + help.rs's generated `areas_block`.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run` | 10-28 | REAL | — | real process spawn, piped stdin, exit code captured; lossy UTF-8 acceptable for CLI bytes |
| test `help_lists_commands_and_generated_areas` | 33-59 | REAL | S4 | version header + synopsis + 6-area presence pin (F-004) |
| test `help_area_prints_the_generated_flags` | 63-76 | REAL | — | flag syntax + schema default pinned; unknown-area exit 64 + "known areas:" pinned |
| test `version_prints_the_crate_version` | 80-87 | REAL | — | exact equality |
| test `commands_lists_the_whole_catalog` | 91-123 | REAL | S4 | served-subset presence + class letters good; brittle count pin (F-001) |
| test `engine_info_names_the_vertical` | 127-137 | REAL | S4 | enshrines the static table (F-002) |
| test `control_stdio_speaks_the_wire_envelope` | 141-153 | REAL | S4 | error envelope pinned; no success payload anywhere in the file (F-003) |
| test `control_stdio_with_json_emits_events` | 157-171 | REAL | — | real fixture, real open, event kinds on the NDJSON stream |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| help_lists… | YES | binary output + exit code; presence-only for areas |
| help_area… | YES | exact flag line + usage-error code |
| version… | YES | exact string equality |
| commands_lists… | YES for presence; NO for count correctness (the count is hand-maintained and historically wrong — F-001) |
| engine_info… | YES for string presence; NO for truthfulness (circular with the hardcoded table — F-002) |
| control_stdio… | YES | byte-shape pins on the error envelope |
| control_stdio_with_json… | YES | requestId echo + event kinds over a real open |
No mocks of the subject; the subject is the real binary.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "--help generated from settings-schema" | test docs 31-32 | REAL | the areas block is schema-generated (help.rs:162-166) |
| "the machine mode's envelope shape" (doc 05 §5.1) | test docs 2-3 | REAL (error half) | F-003 for the success half |
| "all 64 ids" → now 65 | test docs 89-90 vs 100 | STALE DOC + fixed pin | the doc comment's "64" contradicts the assertion's "65" in the same function — the comment was updated at 95-99 but line 89-90 still says "all 64 ids" |

## 7. Dead / duplicated / unreachable code

- None. Test-only file; every fn runs.

## 8. Dependencies & flags

No feature gates. Test-only deps: `testkit::Must`. Requires the `lumen` binary target (CARGO_BIN_EXE_lumen) — runs under `cargo test -p lumen-cli` only.

## 9. Open questions for the next agent

- Who else pins the catalog count? If feature-matrix or docs cite "64 commands" anywhere, they are stale the same way this test was (Wave 18/19 claim ledgers).
- The `--json` events test asserts `tracks.list-ready` — note that dispatch.rs F-001 (TracksList's empty-success shape) is NOT covered here; a `tracks.list` call in machine mode pre-session would answer `[]` and no test would notice.
