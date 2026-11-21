# AUDIT: crates/audio-output/src/stub.rs
| Field | Value |
|---|---|
| Auditor | W08-A06 |
| Date | 2026-11-21 |
| Lines / functions | 165 / 13 (4 inherent + 8 trait-impl methods + 1 test) |
| File verdict | STUB (honest typed-refusal stub by documented design — and UNREACHABLE from every shipped entry point; see F-001) |
| Reachable from shipped entry point? | NO (only non-test construction site is `builtin_backends()` at src/lib.rs:152-157, which has ZERO callers repo-wide; `rg -n "builtin_backends"` → lib.rs:152 only. The shipped CLI hard-wires the null device: apps/cli/src/pipeline_host.rs:1598-1599 `NullBackend::open_stream(&format, RING_FRAMES, NullConfig { capture: false })`. apps/web/src/host.rs:75-76 imports null types only; apps/desktop mentions `UnsupportedBackend` in comments only (main.rs:8, host.rs:11, transport.rs:240). The module is compiled and its unit test executed (baseline #3), but no runtime path in any shipped binary constructs it.) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "UnsupportedBackend\|for_current_os\|os_backend_pending" --glob *.rs` (whole repo) → constructors: lib.rs:155 (dead fn) + stub.rs's own tests; prose refs in apps/desktop comments; `rg -n "builtin_backends"` → lib.rs:152 ONLY (zero callers); `rg -n "audio_output::\|use audio_output" apps/ crates/` → shipped users are null-only (apps/cli/pipeline_host.rs:52-53, apps/cli/host.rs:79, apps/web/host.rs:75-76); `rg -n "platform-" apps/cli/Cargo.toml` → no hit (CLI links NO platform-* crate, so not even enumeration code ships); `sed -n` verbatim verification of stub.rs:1-13, 26-33, 36-60, 61-74, 77-86, 87-100, 108-117, 119-133, 1580-1612 (pipeline_host), 320-332 (web capabilities); baseline #3 test-workspace.log.txt:155-196 → `stub::tests::every_operation_answers_typed_not_supported ... ok` inside audio-output lib 33/33 passed; baseline INDEX.md:46-47 (engine-info `audio.null-device`), :126 ("Audio 'plays' only into the null device"); docs/feature-matrix.toml:2352-2363 (row platform-aout-cpal); product.toml:148-150; audit/files/product.toml.audit.md F-003 [S0] cross-cite |

## 1. One-paragraph verdict
Hostile answer: **all seams, and this particular seam is not even wired in.** stub.rs is the honest tombstone for real-device audio output — every operation refuses with a typed `Unsupported` error (`audio.os_backend_pending`) naming the platform, exactly as card 2.C3 and ADR-005's not-admitted verdict dictate, and its unit test really ran in baseline #3. But it is dead weight in the shipped binary: the only non-test code that constructs `UnsupportedBackend` is `builtin_backends()` (lib.rs:152-157), which nothing in the repository calls — there is no backend-selection mechanism anywhere; the CLI hard-wires `NullBackend` at open time (pipeline_host.rs:1598). How does a user hear sound? **They cannot, at any point, on any platform.** The shipped chain is: 9+ live audio decoders (pipeline_host.rs:55-62, W07) → `AudioPipeline` → `NullSink` → `SpscRing` → `NullDevice::render` (driven deterministically by the pump purely for the audio-master clock) → discarded (`capture: false`). The CLI's dependency list contains no `platform-*` crate (apps/cli/Cargo.toml:44-63), so not even the device-enumeration code (pulse/coreaudio/wasapi) is linked; cpal is gated off by a const-false compile-time assert (cpal.rs:43, W01-A02 verified). The file itself is well-made — the defects are its doc comment claiming it "occupies the selection slot" (no such slot exists), an overbroad "every operation answers with a typed Unsupported error" sentence, and two test gaps.

## 2. What actually works (verified)
— The stub's refusals are real, typed, and platform-naming: verified by reading stub.rs:61-74 (`pending()` builds `ErrorKind::Unsupported` with `os-audio-backend/{platform}` + context `audio.os_backend_pending`) and by baseline #3: `test stub::tests::every_operation_answers_typed_not_supported ... ok` (audit/runs/baseline/test-workspace.log.txt, audio-output lib suite "33 passed; 0 failed").
— The type compiles into the shipped crate: baseline #1 (workspace build CLEAN) compiles audio-output; baseline #3 runs its 33 lib tests.
— No fake success on the error-capable operations: `write`/`open`/`pause`/`resume` all return `Err(self.pending())` (stub.rs:78-80, 93-108); `enumerate` returns empty per the trait's documented honesty rule (device.rs:89-90 "Empty is honest ('no devices'), never a placeholder entry").
— Zero `unsafe`, zero panics, zero unwraps in production code (whole-file read; W01-A02's crate-wide grep for `unsafe`/`#[allow(` corroborates).
— Honest advertisement elsewhere: `lumen engine-info` reports `audio.null-device` (baseline INDEX.md:46-47) and apps/web/src/capabilities.rs:327-330 declares `audio_output: "null device (the headless sample-counter clock; ...)"` — the ecosystem does not lie about this file's absence from the sound path.

## 3. Findings
### F-001 [S3]
Where: crates/audio-output/src/stub.rs:8-9 (impact site lib.rs:152-157)
Code:
```
//! cpal-vs-direct-API decision; the cpal P0 record lives at
//! `docs/dependencies/records/cpal-0.18.2.md`). Until those cards
//! land, this stub occupies the selection slot truthfully: it
```
What is wrong: The doc comment claims the stub "occupies the selection slot truthfully", but no selection slot exists in any shipped binary. The only non-test constructor call is `Box::new(UnsupportedBackend::for_current_os())` inside `builtin_backends()` (lib.rs:152-157), and `rg -n "builtin_backends"` over the entire repository returns exactly one hit — the definition. No shipped entry point (`lumen` CLI, `lumen-tauri`, web host) constructs `UnsupportedBackend`; the CLI's open path hard-wires the null device (`NullBackend::open_stream(&format, RING_FRAMES, NullConfig { capture: false })`, pipeline_host.rs:1598-1599), and apps/desktop references this type in comments only (apps/desktop/src/main.rs:8, host.rs:11, transport.rs:240).
Why it matters: The stub's entire raison d'être — standing in the selection slot so a future caller gets an honest typed refusal instead of silence — is unexercised. A reader of this file concludes the engine has a backend-selection mechanism that would surface `audio.os_backend_pending` to a user attempting device output; nothing of the sort exists. The stub is compiled dead weight plus a passing test.
What correct looks like: Either wire `builtin_backends()` into the session-open path (so `--audio-device` style selection encounters the typed refusal), or change the doc to "this stub exists for the future selection table; no selector is wired in v0" and mark `builtin_backends()` accordingly.
Confidence: HIGH — Verification: read + `rg -n "builtin_backends"` (1 hit: definition) + `rg` for all `UnsupportedBackend` constructions.

### F-002 [S4]
Where: crates/audio-output/src/stub.rs:10-13
Code:
```
//! enumerates **no devices** and every operation answers with a
//! typed [`Unsupported`](media_types::ErrorKind::Unsupported) error
//! naming the platform and the pending decision — never a fake
//! success, never a panic.
```
What is wrong: "Every operation answers with a typed Unsupported error" is false for 4 of the 9 trait operations: `enumerate` → `Vec::new()` (88-91), `latency` → `None` (82-84), `clock_source` → `None` (110-112), `underruns` → `0` (114-116). Those returns are correct per the trait contract (device.rs:89-90 documents empty-list-as-honest; :123-126 documents `None` as legitimate for `clock_source`), but they are success-shaped values, not errors.
Why it matters: The module doc overpromises; a reader auditing "does anything fake success here?" must reconcile the doc's absolute claim with four value-returns. In a file whose whole point is honesty, the doc sentence should be exact.
What correct looks like: "every fallible operation answers with a typed Unsupported error; the value-returning operations report honest empties (no devices, unknown latency, no clock, zero underruns)".
Confidence: HIGH — Verification: read of stub.rs:77-117 against device.rs:88-131.

### F-003 [S4]
Where: crates/audio-output/src/stub.rs:78-80 (test gap; test at 124-164)
Code:
```
impl AudioSink for UnsupportedBackend {
    fn write(&mut self, _buf: &AudioBuffer) -> Result<(), EngineError> {
        Err(self.pending())
    }
```
What is wrong: `AudioSink::write` — the operation that would actually move samples — is never exercised by any test in the repository. The unit test covers enumerate/clock_source/underruns/latency/pause/open/resume/named/platform, but not `write`. Additionally, `for_current_os`'s platform label is only prefix-checked (`rendered.contains("os-audio-backend/")`, line 138), never exact-asserted, so a mutation mislabeling linux as "windows" in `for_current_os` fails no test (the `named("plan9")` half IS exact-checked, lines 156-163).
Why it matters: Mutation analysis: `write → Ok(())` (a fake-success mutation — the exact sin this stub exists to avoid) passes the entire workspace suite because nothing calls `write` on this type anywhere. The strongest property of the file (never fake success) is untested on the one method that carries data.
What correct looks like: Add `let err = stub.write(&empty_buf).err().must(); assert_eq!(err.context.code, "audio.os_backend_pending");` and assert the exact `format!("os-audio-backend/{}", std::env::consts::OS)`-style label for `for_current_os`.
Confidence: HIGH — Verification: read of test body; `rg` shows no other caller of `write` on `UnsupportedBackend`.

### F-004 [S4]
Where: crates/audio-output/src/stub.rs:38-46
Code:
```
        let platform = if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            "other"
        };
```
What is wrong: An `if cfg!(...)` chain where `#[cfg]`-gated consts would be idiomatic (the chain type-checks all branches and reads as runtime dispatch when it is compile-time constant); the `"other"` catch-all silently labels every non-tier-1 target (freebsd, android, …) as `"other"` rather than `std::env::consts::OS`, so a refusal on such a target names a nonexistent platform bucket instead of the real OS.
Why it matters: Cosmetic today (the stub is unreachable, F-001), but the refusal message is this stub's only output; on a non-tier-1 target it would be `os-audio-backend/other` — vague exactly where precision is cheap.
What correct looks like: `#[cfg(target_os = "linux")] const OS: &str = "linux"; …` with a `#[cfg(not(any(...)))] const OS: &str = std::env::consts::OS;` fallback.
Confidence: HIGH — Verification: read; no execution possible (unreachable, and sandbox is linux anyway).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `UnsupportedBackend` (struct) | 29-32 | REAL (stub datum) | — | One `&'static str` field; derives Debug+Clone; no invariants to break. |
| `UnsupportedBackend::for_current_os` | 37-48 | REAL | S4 (F-004) | `cfg!` if-chain, compile-time folded; `"other"` catch-all for non-tier-1 OSes; unreachable in shipped binaries (F-001). |
| `UnsupportedBackend::named` | 52-54 | REAL | — | Test/future-table constructor; trivially correct. |
| `UnsupportedBackend::platform` | 57-59 | REAL | — | Trivial getter. |
| `UnsupportedBackend::pending` (private) | 61-74 | REAL | — | Builds the typed refusal: stage `AudioOutput`, `ErrorKind::Unsupported` with `FormatInfo { name: Some(format!("os-audio-backend/{platform}")), sniffed: vec![] }`, context `audio.os_backend_pending`. One `format!` allocation per call — cold path, irrelevant. No panic paths. |
| `AudioSink::write` | 78-80 | STUB (honest Err) | S4 (F-003) | Ignores `_buf` by design; returns `Err(pending)`. Untested anywhere (F-003). |
| `AudioSink::latency` | 82-84 | STUB (honest None) | — | `None` = unknown latency; matches `AudioSink` Option contract. Tested (line 130). |
| `AudioBackend::enumerate` | 88-91 | STUB (honest empty) | — | `Vec::new()` per device.rs:89-90 ("Empty is honest"). Tested (line 127). |
| `AudioBackend::open` | 93-100 | STUB (honest Err) | — | Ignores `_device`/`_format`/`_buffer_frames` by design; `Err(pending)`. Tested (lines 140-153). |
| `AudioBackend::pause` | 102-104 | STUB (honest Err) | — | `Err(pending)`. Tested (lines 132-138). |
| `AudioBackend::resume` | 106-108 | STUB (honest Err) | — | `Err(pending)`. Tested (lines 158-163, via `named`). |
| `AudioBackend::clock_source` | 110-112 | STUB (honest None) | — | `None` documented legitimate (device.rs:123-126). Tested (line 128). |
| `AudioBackend::underruns` | 114-116 | STUB (honest 0) | — | Vacuously true: never opens, so never underruns. Tested (line 129). |
| `tests::every_operation_answers_typed_not_supported` | 124-164 | REAL | S4 (F-003 gaps) | Genuinely discriminating for pause/open/resume/enumerate/context-code/named-platform (see §5); gaps: `write` untested, `for_current_os` label not exact-asserted. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `every_operation_answers_typed_not_supported` (124-164) | YES for most mutations; NO for two | Mutation `pause → Ok(())`: `.err().must()` panics (testkit `Must` unwraps Option/Result with context, testkit/src/lib.rs:44-69) ⇒ test FAILS — good. Mutation `context code → "x"`: `assert_eq!(err.context.code, "audio.os_backend_pending")` FAILS — good. Mutation `enumerate → placeholder entry`: `assert_eq!(stub.enumerate().len(), 0)` FAILS — good. Mutation `named` drops platform: `platform()`/`contains("os-audio-backend/plan9")` FAILS — good. Mutation `write → Ok(())`: NO test anywhere calls `write` on this type ⇒ PASSES — gap (F-003). Mutation `for_current_os` linux→"windows": only prefix `contains("os-audio-backend/")` asserted ⇒ PASSES — gap (F-003). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "real-device adapters stubbed with typed not-supported errors, honestly recorded" (card 2.C3 fallback, quoted at stub.rs:1-3) | card 2.C3 via module doc | REAL | stub.rs:61-116 implements exactly this; test ran in baseline #3 (test-workspace.log.txt: `stub::tests::every_operation_answers_typed_not_supported ... ok`). |
| "this stub occupies the selection slot truthfully" | stub.rs:8-9 | FALSE | No selection slot exists: `builtin_backends()` (lib.rs:152-157) has zero callers; CLI hard-wires NullBackend (pipeline_host.rs:1598-1599). See F-001. |
| "every operation answers with a typed Unsupported error … never a fake success, never a panic" | stub.rs:10-13 | SHALLOW (imprecise) | 4 of 9 operations return success-shaped honest empties (Vec::new()/None/None/0), not errors — F-002. The "never a panic" half is true (no panic path in the file). |
| `audio-output = true # default audio device output` | product.toml:150 (features) | FALSE | Shipped binary has no audio device output: CLI opens the null device with `capture: false` (pipeline_host.rs:1598-1599); CLI links no platform-* crate (apps/cli/Cargo.toml:44-63); cpal gated const-false (cpal.rs:43). Already recorded as F-003 [S0] by the product.toml audit (audit/files/product.toml.audit.md:106-130) — cross-cited, not re-claimed. |
| Row `platform-aout-cpal`: status partial (win/linux/macos), "no real-device evidence exists (typed marker refuses on every platform)" | docs/feature-matrix.toml:2352-2363 | REAL (as a self-description of a seam) | The evidence string accurately names this file's contents; the notes disclose the partial-seam status. The row does not claim audible output. |
| `audio.null-device` is a shipped feature | `lumen engine-info` (baseline INDEX.md:46-47) | REAL | The null backend is the shipped, executed audio path (baseline #20 rc-smoke PASS; INDEX.md:126 "Audio 'plays' only into the null device"). |

## 7. Dead / duplicated / unreachable code
- **Unreachable from shipped entry points: this entire file.** Sole non-test constructor site is `builtin_backends()` (lib.rs:152-157) with zero callers repo-wide (`rg -n "builtin_backends"` → 1 hit, the definition). The `pub use stub::UnsupportedBackend` re-export (lib.rs:143) is consumed by nothing outside the crate's own tests.
- Dead-adjacent (owned by W08-A07): `builtin_backends()` itself — and note its fallback ORDER is `[NullBackend, UnsupportedBackend]` (lib.rs:153-156), so even if a future caller wired it for device-loss fallback (doc 03 §4.9 d), the null backend would always be selected first and the stub's refusal would still never surface.
- No duplication; no `#[allow]`; no unsafe; no fixtures baked in.

## 8. Dependencies & flags
- Uses (all declared, all downward, L4→lower): `audio_engine` (AudioSink, buffer::AudioBuffer), `media_clock` (SampleCounterSource), `media_types` (AudioFormat, Duration, EngineError, ErrorKind, PipelineStage, error::FormatInfo/ErrorContext), `crate::device` (AudioBackend, DeviceDesc, DeviceId). Dev: `testkit` (Must). No external crates touched here — consistent with boundaries.toml:408-413 (external_allowlist = []), per W01-A02.
- No feature flags in this file. The cpal admission gate (`AUDIO_ADR005_ADMITTED = false`, cpal.rs:43, compile-time assert at :48-52 — W01-A02 verified) is the reason this stub exists; flipping it requires the Lead's ADR-005 admission, per docs/dependencies/records/cpal-0.18.2.md (exists; not re-audited here — W08-A08 owns cpal.rs).

## 9. Open questions for the next agent
- To W08-A07 (lib.rs): own the `builtin_backends()` dead-code finding (my F-001 documents the stub-side impact). Decide: is `builtin_backends()` a promised seam (then a caller must exist somewhere — engine? settings?) or dead code to delete? Also check whether any doc claims a backend-selection mechanism exists (doc 03 §4.9 d fallback policy) — that claim is currently FALSE for want of any caller.
- To W08-A08 (cpal.rs): this file's refusal vocabulary is `audio.os_backend_pending` (stub.rs:72); the cpal seam's is the `BackendNotAdmitted` marker (different code path). Confirm the two refusal vocabularies are documented as distinct on purpose, so a future caller doesn't catch one and miss the other.
- To platform-* owners: pulse.rs (platform-linux:41), coreaudio.rs (platform-macos:59), wasapi.rs (platform-windows:47,417) consume `DeviceDesc`/`DeviceId` from this crate — but the shipped CLI links NO platform-* crate (apps/cli/Cargo.toml has none), so device enumeration is as unreachable as this stub. Cross-check whether any shipped surface (tauri shell?) links them; the tauri build is UNVERIFIED-BUILD (baseline #2, missing GUI dev libs).
- Domain answer for the wave summary: user-audible output does not exist in any shipped binary; the only executed audio-output code is null/ring/render/pump/counter (peers' files), whose product is the audio-master sample clock, not sound.
