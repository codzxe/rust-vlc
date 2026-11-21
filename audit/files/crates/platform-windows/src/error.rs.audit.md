# AUDIT: crates/platform-windows/src/error.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-21 |
| Lines / functions | 114 / 3 units (8-variant enum, Display impl, Error impl) |
| File verdict | UNREACHABLE (the crate has no dependents — manifest scan matches only platform-linux's header *comment* and the crate itself; `WindowsPlatform::discover` is caller-free; the file itself IS compiled on Linux (unconditional `pub mod error;`) and its vocabulary is executed by the crate's tests) |
| Reachable from shipped entry point? | NO — no workspace member or app depends on `platform-windows`; only internal chain: seam fallbacks (`UnavailableAudioDevices::because` etc.) → `WindowsPlatform::discover()`, itself caller-free. Compiled on the Linux baseline (build log :242); tests executed (see §2). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | Read of full file; `sed -n` quote verification (1-8, 94-97); `diff` of the three `platform-*/src/error.rs` (byte-identical except line 1's card id: 7.A1/7.A2/7.A3 — diff exit 1 with only that line, macos same); manifest scan for dependents (none); baseline cites: `orchrun-test-workspace-nofailfast.log.txt:3712-3716` (platform_windows lib + `tests/seam_behaviour.rs` 16 tests ok) and the Display pin at seam_behaviour.rs:371. |

## 1. One-paragraph verdict
A pure typed-error vocabulary: 8 variants, one Display arm each, one empty `std::error::Error` impl — no logic to get wrong, and the parts that could drift (the Display formats) are test-pinned and executed in the no-fail-fast baseline (16 seam_behaviour tests ok, including a verbatim Display assertion). The findings are all hygiene: the header comment is a copy-paste artifact that names **itself** ("mirrored in `platform-windows`/`platform-macos`" — written from platform-linux's perspective, wrong in platform-windows's own file, and identical in all three copies because the files are byte-identical except the card number); the same triple duplication is the documented `[PROPOSAL]` Lead-unification debt; and the `CallFailed` Display renders HRESULTs as signed decimals (`-2147417850` instead of `0x80010106`), which is deliberately pinned by test but produces diagnostics that cannot be pasted into Microsoft's documentation search. The crate is dead weight in the shipped graph like its two siblings — nothing depends on it.

## 2. What actually works (verified)
— Vocabulary executed in tests: baseline `orchrun-test-workspace-nofailfast.log.txt:3712-3716` — `tests/seam_behaviour.rs` (16 tests, all ok) constructs `NotCompiledForThisOs`, `CallFailed { api: "CreateDXGIFactory1", code: Some(0x8007_000E), … }`, `LibraryUnavailable`, `SymbolMissing` and asserts they round-trip through the typed fallbacks (`assert_eq!(err, reason)` — the enum's `PartialEq` is exercised).
— Display format pinned verbatim: seam_behaviour.rs:371 asserts `"CoInitializeEx failed with status -2147417850: changed mode"` — the `CallFailed` arm's exact rendering (0x80010106 as i64), so the format cannot silently drift.
— All 8 variants have Display arms (checked exhaustively — the match is total with no `_` arm), and the enum derives `Debug/Clone/PartialEq/Eq` — the machine-checkable-truth claim holds.
— `impl std::error::Error for PlatformError {}` — leaf error, no `source()` — correct for a terminal platform vocabulary.

## 3. Findings

### F-001 [S4]
Where: crates/platform-windows/src/error.rs:6-7
Code:
```
//! — never a bare string, never a fake success (MP §8: "errors are
//! data, and the error path is a first-class outcome"). The same
//! shape is mirrored in `platform-windows`/`platform-macos` so the
//! Lead can later unify them into a shared L5 vocabulary crate
```
What is wrong: this is platform-windows's **own** error.rs, and the sentence names platform-windows and platform-macos as the mirrors — i.e., it names itself and omits platform-linux. The sentence is correct only in platform-linux's copy (its origin, card 7.A2); it was copied verbatim into the windows and macos crates where it is false.
Why it matters: a reader of platform-windows/src/error.rs is told the vocabulary is shared with macos (true) and not told it is shared with linux (also true) — and is implicitly told platform-windows is *elsewhere* in the tree. Small, but it proves no copy of the triple-duplicated text was ever re-read at its destination.
What correct looks like: per-crate accurate header ("mirrored in `platform-linux` and `platform-macos`" here), or the shared-crate unification the `[PROPOSAL]` already anticipates.
Confidence: HIGH — Verification: read + three-way diff (byte-identical except card id).

### F-002 [S4]
Where: crates/platform-windows/src/error.rs:1-114 (with platform-linux/src/error.rs and platform-macos/src/error.rs)
Code:
```
//! The same shape is mirrored in `platform-windows`/`platform-macos` so the
//! Lead can later unify them into a shared L5 vocabulary crate
//! `[PROPOSAL]`.
```
What is wrong: 114 lines of identical vocabulary × 3 crates (diff-verified byte-identical except line 1). The duplication is documented and Lead-gated — but three independent `PlatformError` *types* (one per crate, no shared root) mean the future unification is a breaking change for every signature that mentions any of them, and any fix applied to one copy (including F-001's header) must be manually replicated twice.
Why it matters: checklist duplication; the risk is not hypothetical — F-001 is a defect that exists in exactly 3 places because of this shape.
What correct looks like: the `[PROPOSAL]` itself (a shared L5 vocabulary crate), or at minimum a cross-link comment in each copy naming its two siblings' exact paths so drift is visible.
Confidence: HIGH — Verification: three-way diff.

### F-003 [S4]
Where: crates/platform-windows/src/error.rs:94-97 (pinned at tests/seam_behaviour.rs:371)
Code:
```
            PlatformError::CallFailed { api, code, detail } => match code {
                Some(code) => write!(f, "{api} failed with status {code}: {detail}"),
                None => write!(f, "{api} failed: {detail}"),
            },
```
What is wrong: Windows HRESULTs are unsigned 32-bit conventions rendered in hex everywhere in Microsoft's documentation; this Display renders the `i64` code as a signed decimal — `0x80010106` becomes `-2147417850` (the pinned test string proves it is deliberate).
Why it matters: a user pasting the error message into a search engine or Microsoft's docs gets no hits; HRESULT-in-hex is the platform's own diagnostic convention and this crate is the Windows adapter. Diagnostic quality only — the machine-readable `code` field keeps the exact value.
What correct looks like: `Some(code) => write!(f, "{api} failed with status {code:#010X}: {detail}")` (or render both), with the test pin updated.
Confidence: HIGH — Verification: read + test string at seam_behaviour.rs:371.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PlatformError` enum (8 variants) | 14-74 | REAL | — | NotCompiledForThisOs, LibraryUnavailable, SymbolMissing, ServiceUnavailable, CallFailed, TimedOut, InvalidEncoding, SpawnFailed — every field documented; no bare-string escape hatch. |
| `Display for PlatformError` | 76-112 | REAL | S4 | Total match, all 8 arms; **HRESULT signed-decimal rendering (F-003)**; formats test-pinned. |
| `impl Error` | 114 | REAL | — | Leaf error, correct. |
| module docs | 1-8 | — | S4 | **Self-naming copy-paste header (F-001)** + triple duplication (F-002). |

## 5. Test honesty (tests live in crates/platform-windows/tests/seam_behaviour.rs, not in-file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `unavailable_audio_devices_returns_the_typed_reason` (33-47) + gpu/dialog siblings | YES | Constructs variants, asserts the fallbacks return *exactly* the reason (`assert_eq!(err, reason)`) — a vocabulary change or a fallback that invents its own error fails. All ran ok (baseline :3716+, 16 tests). |
| Display-format table (≈360-375, asserting `"CoInitializeEx failed with status -2147417850: changed mode"` etc.) | YES | Verbatim Display strings — a format change fails. Pins F-003's rendering as intentional. |
| Mutation check (reasoned): drop the `TimedOut` Display arm | CAUGHT (compile) | The match is total — a missing arm is a compile error, not a runtime gap. (Rust exhaustiveness does the honesty work here.) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every failure a platform seam can produce is one of these variants — never a bare string, never a fake success" | module docs 3-5 | REAL | The FFI modules (`windows::wasapi`/`dxgi`/`dialogs`, cfg-gated out on this host) return these variants (spot-checked via lib.rs seam builders); the fallbacks return the reason verbatim (tests). |
| "The same shape is mirrored in `platform-windows`/`platform-macos`" | module docs 6-7 | FALSE (here) | In this crate's copy the sentence names itself and omits platform-linux (F-001); the three files are byte-identical otherwise (diff). |
| "compile-verified via `cargo check --target x86_64-pc-windows-msvc` … runtime … `[VERIFY]`" | lib.rs (crate docs, not this file) | UNVERIFIED | No msvc-target run exists in `audit/runs/baseline/` (INDEX.md reviewed) — same class as platform-macos's F-004. |

## 7. Dead / duplicated / unreachable code
- The crate is unreachable from shipped entry points (no dependents; `WindowsPlatform::discover` caller-free).
- F-002: the file is a byte-identical triplicate.

## 8. Dependencies & flags
- `error.rs` is unconditional (compiled on every OS — the Linux baseline compiled it, build log :242); the `windows` FFI modules are `#[cfg(target_os = "windows")]` (lib.rs:61+).
- Std-only crate (deps: audio-output, media-types — mirror of the platform-linux manifest shape); no feature gates; no unsafe in this file.

## 9. Open questions for the next agent
- `crates/platform-windows/src/windows/{wasapi,dxgi,dialogs}.rs` (the COM/FFI surface) are unaudited — same never-compiled-in-recorded-runs status as platform-macos's metal.rs (the lib.rs "compile-verified via cargo check --target x86_64-pc-windows-msvc" claim is not reproducible from baseline evidence). REQUEST-ORCH-RUN (optional, logged): `cargo check -p platform-windows --target x86_64-pc-windows-msvc`.
- `capability.rs` (`Capability::from_probe_error` mapping these variants) is where the vocabulary becomes the platform report — unaudited here.
