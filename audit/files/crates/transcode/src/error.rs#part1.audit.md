# AUDIT: crates/transcode/src/error.rs#part1 (lines 1-411)
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-21 |
| Chunk boundaries | part1 = 1-411 (module docs, `mint`, all graph/copy/wav/sink/job/mp4-empty-set constructors through `mp4_track_index`); part2/W10-A05 = 412-806 (`mp4_codec_unsupported` … `mp4_sample_entry_over_budget` + the test module). Equal-split anchor 1-403, adjusted +8 to the function boundary. |
| Lines / functions | 411 / 39 (1 private helper + 37 constructors + FactsMismatch leaf with Display/Error) |
| File verdict | UNREACHABLE (real, baseline-tested error vocabulary of a crate no shipped binary links against) |
| Reachable from shipped entry point? | NO. The whole `transcode` crate (13 modules) has zero consumers: no app or crate Cargo.toml lists it (`rg -l "^transcode" */Cargo.toml` → nothing) and no `use transcode`/`transcode::` outside the crate itself (only its own lib.rs/record.rs doc comments and its own tests). The shipped `lumen` CLI has no convert/stream command (`rg -i "convert|transcode"` over apps/cli → nothing). Matrix is honest about this: `ui-dialog-convert` = not_started, empty evidence, phase 7 (feature-matrix.toml:2913-2922). Executed in baseline as library code: orchrun-test-workspace-nofailfast.log.txt:5824 (`error::tests::constructors_mint_stage_transcode_kinds_and_codes … ok`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "^(pub |)fn…" error.rs` (outline/split); `rg -rln "use transcode|transcode::"` (crate-local only); `rg -l "^transcode"` in Cargo.tomls (absent); `rg -in "transcode|convert"` in feature-matrix + apps/cli; `rg -n "let _ ="` in error.rs (two hits — the ignored params); caller checks (wav.rs:111, runner.rs:1045 pass real values); baseline log grep; `sed -n` on all quoted ranges (verbatim). |

## 1. One-paragraph verdict
A disciplined, table-driven mint vocabulary — 37 one-purpose constructors, stable dotted codes, kinds delegated to the media-types taxonomy (never redefined), and a 36-case code/kind pin test that ran green in baseline. Two genuine defects: every I/O failure (sink write, sink seek, worker spawn) is classified `Inaccessible { reason: PermissionDenied }` regardless of what the OS actually said — a full disk or an exhausted thread budget will be reported to users as a permissions problem; and two constructors accept a diagnostic value (`found`, `channels`) computed by real callers and throw it away with `let _ =`, so the module's entire purpose — informative typed errors — is defeated at exactly the two sites where the number matters. All of it is dead weight in the shipped binary: the transcode crate has no consumer at all.

## 2. What actually works (verified)
— verified by baseline run `orchrun-test-workspace-nofailfast.log.txt:5824`: the constructor table test passes — every code/kind pairing in this range is pinned.
— verified by read: the budget constructors (`filter_stages_over_budget`, `limit_over_budget`, `tracks_over_budget`, `wav_size_overflow`) genuinely carry their values into `ResourceLimit { budget, attempted }` — the ignored-parameter disease is confined to the two `Unsupported`-shaped sites; `copy_facts_mismatch` chains a dedicated `FactsMismatch` source leaf preserving both strings.

## 3. Findings

### F-001 [S3]
Where: crates/transcode/src/error.rs:309-342 (three constructors; quote from 309-318)
Code:
```
pub fn sink_write_io(err: std::io::Error) -> EngineError {
    mint(
        ErrorKind::Inaccessible {
            reason: AccessDenied::PermissionDenied,
        },
        "transcode.sink.write-io",
    )
    .with_source(err)
}
```
What is wrong: `sink_write_io`, `sink_seek_io` (320-329), and `worker_spawn_io` (331-342) map ANY `std::io::Error` unconditionally to `AccessDenied::PermissionDenied`. No inspection of `err.kind()`: ENOSPC (disk full — the most common transcode-sink failure), EPIPE, EIO, EAGAIN (thread-spawn resource exhaustion) all surface as "permissions refuse it". The taxonomy itself offers no better variant (AccessDenied is permission/missing/locked/auth/disc — media-types error.rs:100-114), but `PermissionDenied` specifically asserts a permission refusal that did not happen.
Why it matters: the kind/reason pair is the machine-readable surface UIs and docs key on — a full-disk WAV conversion telling the user "permission denied" sends them chasing the wrong fix. The OS error survives in the source chain, but the classification is the contract. Muted today (crate unreachable; phase-7 target).
What correct looks like: match on `err.kind()` at least separating `PermissionDenied` from everything else (a new `AccessDenied::Io` variant or an `Inaccessible` reason for raw I/O, matching how the input crate folds read errors).
Confidence: HIGH — Verification: read of all three bodies + the taxonomy.

### F-002 [S3]
Where: crates/transcode/src/error.rs:121-128 (and identically 267-274)
Code:
```
pub fn audio_track_ambiguous(found: usize) -> EngineError {
    let _ = found;
    mint(
        ErrorKind::Unsupported { format: None },
        "transcode.source.audio-track-ambiguous",
    )
}
```
What is wrong: `audio_track_ambiguous(found)` and `wav_channels_out_of_range(channels)` (268: `let _ = channels;`) accept a diagnostic value and discard it. Real callers compute and pass the actual numbers — runner.rs:1045 (`found => Err(audio_track_ambiguous(found))`) and wav.rs:111 (`wav_channels_out_of_range(u32::from(channels))`) — and the value dies inside the constructor. The error says "ambiguous"/"out of range" without HOW MANY tracks or WHICH channel count.
Why it matters: this module's entire job is informative typed errors ("one table, tested in one place", line 2-3); the checklist's ignored-parameter smell at the exact sites where the number is the diagnosis. The budget constructors prove the authors know how to carry values.
What correct looks like: carry the number — a `FormatInfo`-sniffed string, or a dedicated source leaf like `FactsMismatch` (the in-file precedent at 192-206), or extending the taxonomy shape.
Confidence: HIGH — Verification: read + caller grep (real values passed); the 36-case pin test asserts only kind+code, so the discard is invisible to it.

### F-003 [S4]
Where: crates/transcode/src/error.rs:176-190 (and 367-376)
Code:
```
pub fn copy_facts_mismatch(descriptor: &str, facts: &str) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::InvalidStructure { offset: None },
        },
```
What is wrong: `copy_facts_mismatch` is a caller-config-vs-track disagreement, and `sink_bad_state` (367) is an internal invariant violation — both classified `Corrupt { InvalidStructure }`, the kind the taxonomy reserves for "the media data itself is malformed" (media-types error.rs:199-200). Neither is media corruption.
Why it matters: kind-level misclassification again (same family as F-001); a UI keying on `Corrupt` would tell the user their file is broken when the job config was.
What correct looks like: `Unsupported` for the facts mismatch (it is a shape the sink cannot consume) and a dedicated internal/state kind or `Decoder{BadState}`-style choice for `sink_bad_state`.
Confidence: MEDIUM — Verification: read; classification philosophy is a judgment call, but the taxonomy doc's own wording points the other way.

### F-004 [S4]
Where: crates/transcode/src/error.rs:309-342
Code:
```
mint(
    ErrorKind::Inaccessible {
        reason: AccessDenied::PermissionDenied,
    },
    "transcode.sink.write-io",
)
.with_source(err)
```
What is wrong: the three I/O constructors are copy-paste twins differing only in the code string — the same mint-with-source shape repeated where one `io_inaccessible(code, err)` helper would do (the crate already has the `mint` precedent for exactly this dedup).
Why it matters: hygiene; three sites to fix when F-001 is fixed (and the fix will be forgotten at one of them).
What correct looks like: one private helper taking the code, called three times.
Confidence: HIGH — Verification: read.

## 4. Per-function table
39 items — the table is uniform by construction; deviations are the findings above.
| Function group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint (helper) | 16-19 | REAL | — | stage+kind+code; codes are literals → ErrorContext::new's constructor panic is unreachable here |
| graph_decode_required / decode_forbidden / copy_facts_missing / selection_tracks_mismatch | 22-57 | REAL | — | Unsupported, shape refusals; codes pinned by test |
| filter_unimplemented | 59-65 | REAL | — | honest-capability refusal (v1 implements no filters) |
| filter_stages_over_budget | 67-77 | REAL | — | carries requested/budget — correct |
| limit_zero / limit_over_budget / tracks_over_budget | 79-110 | REAL | — | ResourceLimit with values — correct; MP §12 posture |
| audio_track_missing | 112-119 | REAL | — | plain refusal |
| audio_track_ambiguous | 121-128 | REAL (defect) | S3 | F-002: `found` discarded |
| track_not_found / track_not_audio | 130-144 | REAL | — | plain refusals |
| copy_not_pcm / copy_byte_order | 146-173 | REAL | — | descriptor carried in FormatInfo — correct |
| copy_facts_mismatch | 176-190 | REAL (defect) | S4 | F-003 kind; strings preserved via FactsMismatch leaf |
| FactsMismatch + Display + Error | 192-206 | REAL | — | source-chaining leaf precedent, correct |
| wav_codec_not_pcm / frame_not_audio / wav_conversion_unsupported | 210-246 | REAL | — | Unsupported, descriptors carried |
| wav_format_change / wav_partial_frame | 248-265 | REAL | — | Corrupt kinds — these ARE data-shaped, fine |
| wav_channels_out_of_range | 267-274 | REAL (defect) | S3 | F-002: `channels` discarded |
| wav_rate_zero | 276-283 | REAL | — | Unsupported |
| wav_byte_rate_overflow / wav_size_overflow | 285-307 | REAL | — | Corrupt/ResourceLimit; size carries attempted — correct |
| sink_write_io / sink_seek_io / worker_spawn_io | 309-342 | REAL (defects) | S3/S4 | F-001 unconditional PermissionDenied; F-004 copy-paste trio; OS error preserved via with_source |
| worker_panic / flush_undrained | 344-365 | REAL | — | Decoder{BadState} — honest panic-as-fault folding |
| sink_bad_state | 367-376 | REAL (defect) | S4 | F-003 Corrupt for an internal invariant |
| mp4_tracks_empty / track_duplicate / config_track_missing / track_index | 378-410 | REAL | — | config-shape refusals, Unsupported |

## 5. Test honesty
Tests (496-806) belong to part2/W10-A05. Read for evidence: `constructors_mint_stage_transcode_kinds_and_codes` (504) is a 36-case table pinning (constructor, kind, code) triples — load-bearing against kind/code drift, blind to F-001/F-002 (it never asserts values are carried, because they aren't). Mutation check (reasoned): change a code string → test fails; swap two kinds → test fails; delete a `let _ =` → test still passes (the gap).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the crate's whole mint vocabulary, one table, tested in one place" | error.rs:1-3 | REAL | 36-case test (part2), baseline :5824 |
| "Codes are part of the compatibility surface once a release ships, so additions are additive-only" | error.rs:8-9 | REAL (policy) | all codes literal, dotted, stable — enforced shape by ErrorContext::new |
| `ui-dialog-convert` not_started, empty evidence, "Conversion UI: WAV (PCM encode) + MP4 remux (stream copy) targets ONLY" | feature-matrix.toml:2913-2922 | REAL (honest) | transcode crate exists (WAV writer + mp4mux + runner) but no consumer; phase-7 target; no contradiction, unlike the network-auth row |

## 7. Dead / duplicated / unreachable code
Entire crate unreachable from shipped entry points (see header) — compiled into the workspace, linked by nothing. Within the file: the F-004 trio duplication; otherwise no dead code (every constructor has an in-crate caller — spot-checked the two F-002 sites and wav.rs/runner.rs imports).

## 8. Dependencies & flags
Depends only on media-types (the taxonomy — correct layering: kinds never redefined here). std-only otherwise. No feature gates. No unsafe.

## 9. Open questions for the next agent
- Part2 owner (W10-A05): do the mp4_* constructors (413-495) repeat the F-002 ignored-parameter pattern? (`mp4_samples_over_budget(found, budget)` at 459 carries values per the outline — check the rest.)
- Wave owner: when the Convert UI (7.E3) wires this crate, F-001 becomes user-visible on day one — fix before wiring, same posture as buffer.rs F-001.
- media-types owner: `AccessDenied` lacks an I/O-shaped variant — the root cause of F-001 lives in the taxonomy, not this file.
