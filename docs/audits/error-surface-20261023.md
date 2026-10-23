# Engine-Crate Error-Surface Audit — 2026-10-23

- **Card:** W5-20 (audit + gate preparation; report-only, zero source edits)
- **Scope:** `crates/{playback-engine, media-clock, packet-queue, demux, codecs,
  audio-engine, video-engine, input, media-probe}/src` — non-test code only.
- **Method:** evidence-only. Every claim cites a `file:line`, a worklog Task ID,
  or a command run in this sandbox (recorded verbatim below). Nothing was edited.
- **Baseline:** `main @ a63fe22` (the wave-5 dispatch base; branch `wip/w5-20`).

## 0. Method and tooling (reproducible)

**Raw sweep** (verbatim, run from the worktree root, 2026-10-23):

```
grep -rEn '\.unwrap\(|\.expect\(|panic!\(|unreachable!\(|todo!\(|unimplemented!\(' \
  crates/playback-engine/src crates/media-clock/src crates/packet-queue/src \
  crates/demux/src crates/codecs/src crates/audio-engine/src crates/video-engine/src \
  crates/input/src crates/media-probe/src
```

Result: **452 raw matches** (per-crate column "raw" in §2).

**Classification** (script `/tmp/classify2.py`, algorithm reproduced here so the
split is re-derivable without the ephemeral file):

1. *Test context* — a match is test code if it sits inside an inline
   `#[cfg(test)] mod/fn` brace range (attribute gathered across continuation
   lines; range closed by brace tracking), **or** inside a file pulled in by a
   file-level `#[cfg(test)] mod NAME;` declaration (e.g. `demux/src/avi/mod.rs:1257-1258`
   gates the whole `avi/tests.rs`; `video-engine/src/lib.rs:180-181` gates
   `test_support.rs`). 412 matches.
2. *Comment context* — the match text sits after `//`, inside `/* */`, or on a
   `///` doc line. 12 matches (11 inside fenced doc examples that the doctest
   phase compiles and runs — e.g. `Doc-tests demux: 1 test`,
   `Doc-tests codecs: 2 tests` in `docs/gates/logs/wave3-merge.test.log` — plus
   1 line of policy prose, `video-engine/src/policy.rs:184`).
3. *Non-test code* — the remainder. **28 matches: 1 `expect` + 27 `unreachable!`;
   zero `unwrap()`, zero `panic!()`, zero `todo!()`, zero `unimplemented!()` in
   non-test code across all nine crates.**

A supplementary sweep of the assert family
(`assert!|assert_eq!|assert_ne!|debug_assert*!|process::abort`, same classifier)
found 46 non-test matches: **3 `debug_assert!`** (compiled out of release
builds) and **43 inside the two executable-contract modules** (§1.8, §1.9).

**Lint posture:** the workspace denies the panic surface at
`Cargo.toml:49-56` (`unwrap_used`, `expect_used`, `panic`, `todo`,
`unimplemented` — all `deny`). One sanctioned local allow exists in the nine
crates: `playback-engine/src/pipeline.rs:1062`
(`#[allow(clippy::expect_used, reason = "wired kinds always have a lane")]`).

**Verified clippy gap (controlled experiment, this sandbox, 2026-10-23):** a
throwaway crate at `/tmp/clippytest` with the same five denied lints compiled
three functions: `panic!("boom")` → clippy error "`panic` should not be present
in production code"; `unreachable!("provably dead")` → **no diagnostic**;
`x.unwrap_or_else(|e| unreachable!("dead: {e}"))` → **no diagnostic**. Clippy's
`panic` lint does not cover `unreachable!`, and `unwrap_or_else(..)` hides the
`unwrap`. The 27 non-test `unreachable!` sites below therefore pass the gate's
clippy step by lint blind spot, not by proof — the house convention
(`unwrap_or_else(|e| unreachable!("{e}"))`, seen throughout the corpus) is a
deliberate dodge of exactly this deny block. Recorded as fix-list item F-5.

## 1. Per-crate findings

### 1.1 playback-engine — 2 non-test code sites

| Site | Form | Classification | Notes |
|---|---|---|---|
| `src/pipeline.rs:1062-1065` | `.expect("wired kinds always have a queue lane")` under a reasoned `#[allow]` | **REAL RISK (latent)** | see below |
| `src/pipeline.rs:2373-2377` | `CodecDescriptor::new("unknown").unwrap_or_else(\|error\| unreachable!("the literal `unknown` is a valid descriptor: {error}"))` | DOC-COMMENTED INVARIANT | `CodecDescriptor::new` rejects only empty/control-char strings (`media-types/src/ids.rs:449-458`); the 7-ASCII-char literal cannot fail under the current rule. Drift coupling: if the registry ever reserves/rejects the name `unknown`, this panics on the cached-decoder recycle path. |

**REAL RISK analysis (pipeline.rs:1065).** `build_limb` is called from (a)
`Pipeline::wire` (`pipeline.rs:1033`), which iterates the fixed array
`[MediaKind::Video, Audio, Subtitle]` (`pipeline.rs:1017`) — all `Some` in
`QueuePlan::for_kind` (`pipeline.rs:354-361`); and (b) the track-switch install
path (`pipeline.rs:1763`), whose kind comes from `SessionCommandKind::selection_kind`
(`session.rs:175-181`, mapping `SelectVideo/SelectAudio/SelectSubtitle` to the
three wired kinds). The invariant is real today but enforced by **convention
across three modules**; a future `MediaKind` variant, a data-kind selection
command, or a new `build_limb` caller compiles clean and panics on a
user-facing open/switch path. **Fix proposal (never applied — owned by the
playback-engine agent):** introduce a `WiredKind` enum (`Audio | Video |
Subtitle`) in `tracks.rs`, change `for_kind` to
`fn for_kind(WiredKind) -> QueueLimits` (total — the `Option` disappears), and
convert `MediaKind -> WiredKind` exactly once at each guard point
(`wire`'s array, `selection_kind`), returning the typed
`NoDecodableKnown`/config error for `Data`. The `expect`, the `allow`, and the
cross-module convention all vanish; the type system carries the invariant.

**D-4 (never fabricate).** Clean: seek contract D4 "never a fake success" +
rollback (`seek.rs:2, 17, 82-87, 388`); per-frame-damage concealment repeats
the **last good video frame** (never new content) and audio **skips** —
"never synthesizing silence; 3.C2 G3" (`pipeline.rs:573-587, 590-604`);
`VIDEO_CONCEALMENT_BUDGET = 1000` bounds the run and every concealed unit is
observable as a `DamagedData` fact (`pipeline.rs:601-608`); absence is data,
never a guess (`pipeline.rs:2379-2380`).

**Typed errors.** Clean. `error.rs` carries `SessionError` (66),
`SessionConfigError` (122), `NoDecodableKnown` (35); `stop`/`close` return
`Result<(), SessionError>` (`session.rs:326, 334`).

### 1.2 media-clock — 0 non-test code sites

All 36 raw matches live in inline `#[cfg(test)]` modules. One `debug_assert!`
in production code (`drift.rs:164`, `a > 0 && b > 0`) — the sanctioned
debug-only invariant form (compiled out of release; D-8 hot-path discipline).

**D-4.** Clean: `drift.rs:77` ("never a division by zero, never a fabricated
maximum"); `DriftCorrection::ZERO` is honest degradation, not a fabricated
correction (`audio-engine/src/governor.rs:48`, the consumer-side mirror).

**Typed errors.** Clean — mapping/drift constructors are typed; no `()`/`bool`
error paths found (predicate sweep: only `is_paused`, `permits`).

### 1.3 packet-queue — 1 non-test code site (+2 doc-example)

| Site | Form | Classification | Notes |
|---|---|---|---|
| `src/limits.rs:21-26` | `cap(literal)`: `None => unreachable!("cap literal {literal} is non-zero by construction")` | DOC-COMMENTED INVARIANT | Module doc (lines 15-20) states the contract: only literals, "broken-invariant abort, not a runtime condition". Verified: all three callers pass literals (`limits.rs:150` `cap(200)`, `:162` `cap(300)`, `:176` `cap(128)` — the doc 03 §4.4(b) defaults). Optional hardening: the const-position `None => loop {}` trick video-engine already uses (fix F-3). |

Doc-example sites `lib.rs:107, 117` sit inside the crate's single fenced
example — compiled and run by the doctest phase (`Doc-tests packet_queue` in
`docs/gates/logs/wave3-merge.test.log`).

**D-4.** Clean: `close()` never destroys resident data without a signal —
producers/consumers observe `PushOutcome::Closed` / `PopOutcome::Closed`
(`queue.rs:303-310`).

**Typed errors.** Clean: `QueueConfigError` for bad caps; push/pop report
`PushOutcome`/`PopOutcome`/`PushAborted` typed enums; `flush` returns a
`Discontinuity` report (not a `Result` — nothing fallible). No `()`/`bool`
error paths.

### 1.4 demux — 0 non-test code sites (+1 doc-example)

198 of 200 raw matches are test code (167 in `avi/tests.rs`, which is wholly
`#[cfg(test)]` via `mod.rs:1257-1258`; 31 more in inline test modules); 1 is
the doc example at `avi/mod.rs:221` (doctest-run, `Doc-tests demux: 1 test`,
`docs/gates/logs/wave3-merge.test.log`); 1 more is a comment inside test code.
The pub test seam `avi/test_support.rs` (lines 1-129, non-test) is panic-free
and propagates typed errors.

**D-4.** Clean and heavily pinned: timebase from `strh` is typed, never
defaulted (`header.rs:268` `TimeBase::new(..).map_err(.. bad_stream_params)`);
missing span end "never guessed" (`header.rs:245`); no fabricated language
(`header.rs:297`); a lying/absent layout skips the stream rather than
fabricating one (`header.rs:346-347, 755`); stamps "format-canonical, not
invented (invariant D1)" (`avi/mod.rs:76`); the EOS terminal is never a
fabricated empty packet (`avi/tests.rs:1712`, test-pinned).
`MAX_PCM_CHANNELS = 64` (`header.rs:349`) accepted the corpus's 8-channel
f32le case byte-exactly (worklog W3-16/W3-08).

**Typed errors.** Clean: `error.rs` mints 14 typed constructors at stage
`Demux` (`not_avi`, `truncated`, `bad_structure`, `bad_stream_params`,
`walk_resync_failed`, `no_supported_streams`, `payload_too_large`,
`index_too_large`, `no_movi`, `not_started`, `seek_out_of_range`,
`seek_unindexed`, `scan_too_slow`);
source-stream errors propagate unchanged with their original stage
(`error.rs:10-14`). Empirical panic-freedom: fuzz harness
`fuzz/fuzz_targets/fuzz_demux_avi.rs` + the 88-file adversarial sweep
(0 panics, worklog W4-R2) + 137/0/4 suite (worklog W5-14).

### 1.5 codecs — 0 non-test code sites (+7 doc-example, +2 debug_assert)

All 7 non-test comment-context matches are inside the two crate-level fenced
examples (`pcm/decoder.rs` one example containing lines 416/420/426/450;
`mjpeg/mod.rs` one example containing 208/212/218) — doctest-run
(`Doc-tests codecs: 2 tests`, `docs/gates/logs/wave3-merge.test.log`). Two
`debug_assert!` in `mjpeg/bitreader.rs:135, 163` (debug-only hot-path
invariants).

**D-4.** Clean: PCM "fabricated silence is a lie about the [missing bytes]"
— typed partial-frame error instead (`pcm/decoder.rs:54-55` +
`frame_to_f32_planes` `# Errors` in `audio-engine/src/convert.rs:135-138`);
MJPEG "returns a typed error: fabricating a frame from nothing would [lie]"
(`mjpeg/decoder.rs:19`); Huffman exhaustion reports `Exhausted`, "never
fabricate the symbol" (`mjpeg/huffman.rs:133, 402-406`); the bit reader
fabricates no bits past end-of-data (`mjpeg/bitreader.rs:12`).

**Typed errors.** Clean: decode outcomes are `Result<DecodeOutcome,
EngineError>`; PCM has a typed error module (`pcm/error.rs`); fuzz targets
`fuzz_codec_pcm.rs` / `fuzz_codec_mjpeg.rs` exist.

### 1.6 audio-engine — 1 non-test code site

| Site | Form | Classification | Notes |
|---|---|---|---|
| `src/pipeline.rs:432-436` | `fresh.get_mut()` → `None => unreachable!("fresh SharedBytes has refcount 1")` | DOC-COMMENTED INVARIANT | `SharedBytes` is `Arc<[u8]>` (`media-types/src/packet.rs:12`); `from_vec` moves a fresh allocation in (no interning/caching anywhere in the type), so a just-constructed handle has refcount 1 and `get_mut` is `Some` by Rust's `Arc` semantics (`packet.rs:44-50` documents "None while any other handle is alive"). Message itself states the invariant. |

**D-4.** Clean: drift estimation "never a fabricated estimate"
(`pipeline.rs:184`); the resampler's defensive backstop "never a fabricated
value" (`resample.rs:201`), test-pinned ("the stream never fabricates samples
past the kernel window", `resample.rs:438`).

**Typed errors.** Clean: `error.rs` mints 7 typed constructors (`bad_gain`,
`channel_range`, `unsupported_format`, `partial_frame`, `plane_mismatch`,
`track_range`, `chunk_alignment`).

### 1.7 video-engine — 0 non-test code sites (+1 policy prose)

The single non-test comment-context match (`policy.rs:184`) is documentation
prose explaining why the scoped `#[allow(clippy::empty_loop)]` exists. The
adjacent constant is the **best practice this audit found**:
`DEFAULT_MAX_PENDING_NZ` uses a const-position `None => loop {}` arm
(`policy.rs:186-190`) — a `loop` in const position never yields a value, so
zeroing the literal *fails the build* instead of panicking at runtime. No
panic surface at all, by construction.

**D-4.** Clean: "this module never guesses a base" (`policy.rs:17`); unknown
duration → no guess (`policy.rs:638`); no frame duration → no guess, the
`Auto` policy requires it typed (`policy.rs:744` + `SchedulerCfgError::
AutoWithoutFrameDuration`). Both tunables are `[PROPOSAL]`-marked with
rationale (`DEFAULT_MAX_PENDING = 16` at `policy.rs:170-174`;
`DEFAULT_MAX_REPEAT = 3` at `policy.rs:192-196`).

**Typed errors.** Clean: `SchedulerCfgError` typed; predicates only
(`is_empty`, `holds_missing`).

### 1.8 input — 23 non-test code sites (all in `src/contract.rs`)

All 23 `unreachable!` (plus 39 `assert!`-family) live in the **executable
contract** module: `pub fn assert_stream_contract` / `assert_truncated` /
`assert_seek_contract`-family helpers (`contract.rs:247-256, 261-262` and
following), which encode the `SourceStream` conformance rules as runnable
proofs. Callers verified: only test contexts — `tests/file_stream.rs:9, 30`,
and the module's own trailing `#[cfg(test)] mod tests` (`contract.rs:538`).
No production call site exists.

**Classification: DOC-COMMENTED INVARIANT** (the executable-contract pattern;
module doc `contract.rs:1-23` states the reuse intent for the future
conformance kit, doc 06 §7.2; the twin house rule is written verbatim in
`media-probe/src/lib.rs:66-70`). A violated contract *is* the failure signal —
these arms fire only when an implementation under test breaks the contract,
which has no recovery semantics.

**Residual observation (rank F-2):** the module is `pub mod contract;`
(`lib.rs:57`) — compiled into **release** binaries with panicky paths inside,
reachable by any future non-test caller by mistake. Fix proposal (owner: the
input-crate agent): gate as `#[cfg(any(test, feature = "contract"))]` (or move
to a `contract` feature used by downstream conformance kits), keeping the API
identical for test builds; `#[track_caller]` already present.

**D-4.** Clean: `peek` "return[s] exactly what is buffered, never more"
(`buffered.rs:288`, test-pinned at `:576`); `contract.rs:502` makes
peek-past-window an explicit fabrication failure.

**Typed errors.** Clean: `error.rs` mints 19 typed constructors
(`missing`, `permission`, `not_a_regular_file`, `truncated`,
`seek_out_of_range`, `seek_before_start`, `seek_overflow`, budget family,
io wrappers …). Seek-past-length/before-start are typed errors, asserted by
the contract itself (`contract.rs:344, 359`).

### 1.9 media-probe — 1 non-test code site (+71 test, +4 assert-family)

`src/contract.rs:79` — `unreachable!("literal URI is valid: {err}")` inside
`assert_*` contract helpers, same executable-contract pattern as §1.8; house
rule documented at `lib.rs:66-70` ("`unreachable!` only on provably-dead
literal-contract arms, the testkit `must` precedent"). Callers verified:
`tests/probe_io.rs` and the `#[cfg(test)] mod tests` at `lib.rs:184-185`.
Classification: DOC-COMMENTED INVARIANT; same F-2 hardening proposal.

**D-4.** Clean: "Hints are sniffed, never assumed" (`lib.rs:53`); "unknown
fourccs stay unhinted rather than invented" (`lib.rs:59-62`;
`signature/avi.rs:517`); unknown format is "an honest typed unknown, not a
guess" (`lib.rs:254`, `error.rs:16`).

**Typed errors.** Clean: `error.rs` (`no_demuxer`, `lookahead_zero`,
`lookahead_over_ceiling`, `min_score_out_of_range`, …).

## 2. Summary table (100% of the 9 crates)

Raw = the §0 grep, verifiable. comment = doc-example/prose (doctest-run unless
prose). test = inline `#[cfg(test)]` or file-level-gated. **NT-code** = the
audit's subject: panic-surface matches compiled into non-test code.

| Crate | raw | comment | test | NT-code | NT forms | REAL RISK | D-4 verdict | typed-error verdict |
|---|---|---|---|---|---|---|---|---|
| playback-engine | 9 | 0 | 7 | **2** | 1 expect + 1 unreachable | **1** (latent) | clean | clean |
| media-clock | 36 | 0 | 36 | **0** | — | 0 | clean | clean |
| packet-queue | 3 | 2 | 0 | **1** | 1 unreachable | 0 | clean | clean |
| demux | 200 | 2 | 198 | **0** | — | 0 | clean | clean |
| codecs | 72 | 7 | 65 | **0** | — | 0 | clean | clean |
| audio-engine | 2 | 0 | 1 | **1** | 1 unreachable | 0 | clean | clean |
| video-engine | 21 | 1 | 20 | **0** | — | 0 | clean | clean |
| input | 37 | 0 | 14 | **23** | 23 unreachable | 0 (residual F-2) | clean | clean |
| media-probe | 72 | 0 | 71 | **1** | 1 unreachable | 0 (residual F-2) | clean | clean |
| **TOTAL** | **452** | **12** | **412** | **28** | 1 expect + 27 unreachable | **1** | 0 violations | 0 violations |

Cross-checks: per-crate `raw = comment + test + NT-code` holds for every row;
the demux row reproduces with the §0 command (`grep … crates/demux/src` → 200).
Supplementary: 3 `debug_assert!` (media-clock 1, codecs 2) + 43 assert-family
in the two contract modules + 0 `process::abort` anywhere.

## 3. Ranked fix list (proposals only — no files were edited)

1. **F-1 (REAL RISK, playback-engine owner):** `pipeline.rs:1065` — replace the
   cross-module `wired kinds` convention with a total `for_kind(WiredKind) ->
   QueueLimits` (new `WiredKind` = Audio|Video|Subtitle in `tracks.rs`;
   convert once at `wire`'s kind array and `selection_kind`; typed error for
   `Data`). Removes the only production `expect` and its `allow`.
2. **F-2 (hardening, input + media-probe owners):** feature-gate the
   executable-contract modules (`#[cfg(any(test, feature = "contract"))]`) so
   release binaries carry zero panicky paths; test/conformance-kit builds are
   unchanged. 24 unreachable! + 43 assert-family sites retire from release.
3. **F-3 (hardening, packet-queue owner):** `limits.rs:21-26` `cap()` → the
   const-position `None => loop {}` form (video-engine `policy.rs:186-190`
   precedent): a zeroed literal becomes a build failure, deleting the
   runtime abort arm.
4. **F-4 (low, playback-engine owner):** `pipeline.rs:2373-2377` — replace the
   `CodecDescriptor::new("unknown")` dance with a crate-const descriptor (or a
   `CodecDescriptor::well_known("unknown")` constructor that cannot fail),
   decoupling the recycle path from the registry's validation rules.
5. **F-5 (gate hardening, Lead/xtask owner):** close the verified clippy blind
   spot (`unreachable!` not covered by the `panic` deny — §0 experiment) by
   adding a grep-based step to the gate (e.g. in `check-architecture` or a new
   xtask subcommand) that fails on `unreachable!` outside `#[cfg(test)]` /
   the two contract modules / doc examples — encoding this audit as CI.

## 4. Honest gaps (what this audit did NOT prove)

- **Direct indexing panics** (~245 heuristic non-test sites, e.g.
  `audio-engine/src/convert.rs:60-96`, `codecs/src/audio/pcm/decoder.rs:261-312`)
  were *not* dataflow-proven individually; the empirical cover is the three
  fuzz targets (`fuzz/fuzz_targets/`), the 88-file adversarial sweep with 0
  panics (worklog W4-R2), and the corpus suites. A full indexing audit is a
  separate card.
- **Arithmetic overflow**: no `[profile]` overrides exist in the root
  `Cargo.toml`, so release builds wrap (cargo default) — only `debug_assert!`
  and explicit checked arithmetic (extensive `saturating_*` use) guard hot
  paths; division-by-zero sites were spot-checked (documented at
  `media-clock/src/drift.rs:77`), not exhaustively enumerated.
- Wave-5 sibling branches (wip/w5-01..19) are mid-flight; this audit covers
  the wave-5 dispatch base `main @ a63fe22` only.
