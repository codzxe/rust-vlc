# AUDIT: crates/audio-output/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-21 |
| Lines / functions | 42 / 7 config keys ([package]×5, [lints], [dependencies]×3, [dev-dependencies]×1) |
| File verdict | REAL (cleanest manifest so far: edges exact both directions, zero unsafe, zero escape hatches, honest typed-refusal seam — and the crate actually ships and runs in the baseline binary) |
| Reachable from shipped entry point? | YES (call chain: `lumen` bin → lumen-cli lib (apps/cli/Cargo.toml:49 `audio-output`) → audio_output NullBackend; executed: baseline #20 rc-smoke PASS on the null device, engine-info features list `audio.null-device`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Grep boundaries.toml `[crates.audio-output]` → internal_deps ["audio-engine","media-clock","media-types"], dev_deps ["testkit"], external_allowlist [] (:408-413) = exact manifest match; Grep `unsafe\|#\[allow\(` in crates/audio-output → only prose mentions, zero real sites; Read src/cpal.rs:1-60 (the admission-gate seam: `AUDIO_ADR005_ADMITTED: bool = false` + compile-time assert); `rg -n 'audio-output.*L4' docs/01-architecture.md` → :243 (registry row includes media-clock — reconciled); Grep `use testkit` in tests → callback_budget.rs:50, clock_truth.rs:23; Read ADR-005-audio-output.md (194 lines); `sed -n '4,20p;82,86p'` quote verifies; baseline #1/#3/#5/#11/#20 |

## 1. One-paragraph verdict
This is what an honest manifest looks like: three internal deps, all downward, all declared in boundaries.toml exactly (no over- or under-declaration in either direction); `[lints] workspace = true` with zero `#[allow(` escape hatches and zero unsafe in the whole crate (verified — the "wait-free ring is atomics-only" claim is literally true); the cpal decision is recorded in two dependency records and one ADR, and the not-admitted state is enforced by a compile-time assertion on a named const with a typed `BackendNotAdmitted` marker instead of fake success (src/cpal.rs:43-52). Unlike the four previous files in this batch, the crate is not an orphan — it is compiled into the shipped `lumen` binary and its null backend executed in the baseline smoke. The only findings are two stale-prose items (a fixed registry gap described as current, and an ADR that says "zero dependencies" where it means "zero external dependencies").

## 2. What actually works (verified)
— Reachability + execution: `lumen` depends on audio-output (apps/cli/Cargo.toml:49); baseline #20 `RC_SMOKE_SKIP_WEB=1 bash scripts/rc-smoke.sh` → PASS 6/6 engine checks with the AVI first vertical (`v_decoded=60 a_decoded=60`, mp4-smoke-bench-avi-mjpeg-pcm.txt), and `lumen engine-info` self-reports `audio.null-device` (INDEX.md:46-47) — the null backend is the shipped, executed audio path.
— Manifest ↔ boundaries exact two-way match: boundaries.toml:408-413 lists exactly audio-engine, media-clock, media-types + dev testkit + empty externals; the manifest declares exactly those.
— "no unsafe" verified: Grep for `unsafe` and `#[allow(` in crates/audio-output → prose mentions only (src/pattern.rs:3 "std-only, no `unsafe`", tests/callback_budget.rs:28 etc.); zero actual blocks/attributes. Crate clippy-clean under the inherited denies (baseline #5).
— The admission gate is real code, not decoration: src/cpal.rs:43 `pub const AUDIO_ADR005_ADMITTED: bool = false;` + :48-52 `const _: () = assert!(!AUDIO_ADR005_ADMITTED, …)` — the gate cannot flip without editing this file, exactly as documented.
— testkit dev-dep is used (tests/callback_budget.rs:50 `use testkit::Must;`, clock_truth.rs:23).

## 3. Findings
### F-001 [S4]
Where: crates/audio-output/Cargo.toml:4-6
Code:
```
# Registry: doc 01 §2.6 (`audio-output`, layer L4, deps: media-types,
# audio-engine). This card adds the media-clock edge the registry
# under-specifies: the device sample counter IS the clock's truth
```
What is wrong: Stale prose describing a fixed problem as current. The registry row no longer under-specifies: doc 01's L4 row reads `| audio-output | crate | L4 | media-types, audio-engine, media-clock | 2 | audio outputs |` (01-architecture.md:243) — the media-clock edge was reconciled by 3.DOC-1, as ADR-005:109-111 itself records ("The registry delta stands: audio-output's `media-clock` edge … is recorded in `boundaries.toml`; doc 01 §2.6's row is reconciled by 3.DOC-1").
Why it matters: A reader of this header today concludes doc 01 and the manifest disagree about media-clock; they do not. Same stale-prose class as the device-discovery header (its F-003) — manifest comments are frozen at card time and never re-baselined.
What correct looks like: "(registry row since reconciled by 3.DOC-1: it now lists media-clock)".
Confidence: HIGH — Verification: read of manifest, 01-architecture.md:243, ADR-005:109-111.

### F-002 [S4]
Where: crates/audio-output/Cargo.toml:32-35 (cross-ref: docs/adr/ADR-005-audio-output.md:84)
Code:
```
[dependencies]
audio-engine = { path = "../audio-engine" }
media-clock = { path = "../media-clock" }
media-types = { path = "../media-types" }
```
(Cross-referenced claim: ADR-005:84 "crates/audio-output — 3,097 LOC, std-only, zero dependencies; 32 tests".)
What is wrong: The ADR's accepted-half summary says "zero dependencies" while the crate carries three internal path dependencies (and the manifest itself words it correctly: "zero external dependencies", line 19). The ADR wording is the sloppier twin of the manifest's correct claim; quoted as-is it contradicts every manifest in the workspace (all members have path deps).
Why it matters: ADR-005 is the governance record for this crate's architecture; a factually wrong dependency statement in the "What is already implemented and verified" section degrades the record's precision, and future audits quoting the ADR propagate the error.
What correct looks like: "std-only, zero external dependencies (3 internal: audio-engine, media-clock, media-types)".
Confidence: HIGH — Verification: read of both files; recorded here because the manifest is the anchor file, the fix belongs to ADR-005's owner.

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package]` name/version/edition/publish | 22-27 | REAL | — | `audio-output` 0.1.0, edition 2024, publish=false; description matches actual contents (trait, ring, callback, underruns, null backend, typed stubs). |
| `[lints] workspace = true` | 29-30 | REAL | — | Binds (baseline #11); zero escape hatches in crate (Grep → 0) — the lints' promises hold here without any allow. |
| `audio-engine` dep | 33 | REAL | — | L4→L3 downward; declared (boundaries:410); used (AudioSink extension per ADR-005 decision 1). |
| `media-clock` dep | 34 | REAL | — | L4→L2 downward; declared; the registry-delta edge (ADR-005:109-111); used for SampleCounterSource truth. |
| `media-types` dep | 35 | REAL | — | L4→L0 downward; declared; used (AudioFormat vocabulary). |
| `testkit` dev-dep | 41 | REAL | — | Matches boundaries dev_deps:411; used in tests (callback_budget.rs:50, clock_truth.rs:23). |

## 5. Test honesty (if tests exist in/for this file)
No tests in this file. The crate's suites (23 lib + 4 callback-budget + 1 clock-truth + 3 sine-sink-hash + 1 doctest per ADR-005:84-86) ran inside baseline #3 (1681 passed workspace-wide); the sine-hash AC (byte-identical sink buffer vs pinned f32 conversion) is the strongest form — a mutation in the conversion or ring would break the hash. Per-crate breakdown not in baseline; src auditor to confirm the hash oracle is not self-generated (F-005 checklist: goldens generated by the code under test).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "cpal is conditional-E2, NOT admitted" | manifest:12-13 / ADR-005 / records/cpal-0.18.2.md | REAL | src/cpal.rs:43 const false + :48-52 compile-time assert; no cpal anywhere in Cargo.lock main graph (deny.toml posture unchanged). |
| "typed not-supported stubs for real devices" | manifest:17 | REAL | src/cpal.rs BackendNotAdmitted marker; ADR-005:58-60 "enumerates nothing, never fakes success". |
| "no unsafe (the wait-free ring is atomics-only)" | manifest:19-20 | REAL | Grep → zero unsafe blocks / zero #[allow(] in crate; clippy CLEAN (baseline #5). |
| "zero dependencies" | ADR-005:84 | FALSE (wording) | 3 internal path deps (manifest:33-35) — see F-002. |
| "the registry under-specifies" (media-clock) | manifest:5-6 | FALSE (stale) | Registry row reconciled (01-architecture.md:243 includes media-clock) — see F-001. |

## 7. Dead / duplicated / unreachable code
- None. The crate is reachable and executed; no orphan deps; no bins; no build.rs; no escape hatches.

## 8. Dependencies & flags
- Internal: audio-engine (L3), media-clock (L2), media-types (L0) — all downward, all declared both directions. Dev: testkit. External: NONE (external_allowlist = [], boundaries:412) — nothing for deny.toml to govern.
- Features: none. The cpal seam (src/cpal.rs) anticipates a future external but imports nothing today.

## 9. Open questions for the next agent
- Cross-crate design tension for the audio owner: audio-output's seam says the future admission lands "the real `platform-*` cpal adapter behind [`cpal_backend`]" (src/cpal.rs:39-40), while platform-linux (7.A2, this batch's file 5) implemented a hand-rolled PulseAudio client for enumeration. If enumeration goes via the PulseAudio server but output via cpal/ALSA, the two substrate stories must be reconciled in ADR-005 before either admission flips — otherwise the seam's "one change" promise (cpal.rs:14-16) breaks.
- Src auditor: tests/zero_alloc_null_sink.rs exists here while apps/desktop/tests/zero_alloc_transport.rs FAILED in baseline #3 (608 bytes allocated) — check whether the same measurement methodology is used and why one passes and the other drifts.
