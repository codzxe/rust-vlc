# AUDIT: crates/platform-windows/src/audio.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-21 |
| Lines / functions | 114 / 5 items |
| File verdict | UNREACHABLE (real, tested OS-neutral seam; no shipped entry point reaches the crate) |
| Reachable from shipped entry point? | NO — no workspace member depends on platform-windows (established in my dialogs.rs audit: apps/cli's 16 deps and apps/desktop-tauri's deps contain no platform crate). Internal consumers exist (windows/wasapi.rs:50 imports the trait + hash) but wasapi.rs is itself cfg(windows) and equally orphaned. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `rg "stable_device_id" crates/platform-*` → triplication confirmed; `diff` of the 96-114 range across the three platform crates → code byte-identical (only a doc-line platform order differs); `sed -n '300,310p' tests/seam_behaviour.rs` → pinned FNV vector; FNV-1a-64 constants hand-checked against the published algorithm; `sed -n '30,60p'` + `'196,250p' tests/seam_behaviour.rs` → fallback/queue tests |

## 1. One-paragraph verdict
A small, clean, OS-neutral seam: the `AudioDevices` trait, an honest typed fallback that fails every method with its reason (never a fake success — tested), and an FNV-1a-64 device-id minter whose constants and pinned test vector are correct and whose non-collision caveat is documented rather than hidden. The crate's own test suite genuinely exercises the fallback contract and the event-queue plumbing, and passed in the no-fail-fast baseline. Two structural defects: the entire crate is unreachable from every shipped binary (zero dependents), and the id-minting function is a verbatim triplicate across the three platform crates when its natural home — the `audio-output` crate all three already depend on — would give it one home.

## 2. What actually works (verified)
- `stable_device_id` is a correct FNV-1a-64: offset basis `0xcbf29ce4842222325` and prime `0x00000100000001b3` match the published algorithm (lines 108-111); the pinned test `stable_device_id("a").handle() == 0xaf63_dc4c_8601_ec8c` (seam_behaviour.rs:300-301) is the standard FNV-1a-64 test vector for "a" — implementation AND test are right.
- Honest collision caveat (101-104): "documented as **not collision-free**" with the birthday-bound argument and the ids-are-per-backend scoping — correct engineering honesty, not overclaim.
- The typed fallback is real degradation, not a stub: every method returns the constructed reason (82-94), and `unavailable_audio_devices_returns_the_typed_reason` (seam_behaviour.rs:31-47) asserts all three methods fail with exactly that reason.
- Queue collaborators tested in the same suite: FIFO drain order, bounded capacity with counted drops, capacity clamped to ≥1 (seam_behaviour.rs:203-250) — mutation-sensitive (drop counting, newest-survive semantics asserted exactly).
- Suite green: platform-windows tests ran in `orchrun-test-workspace-nofailfast.log.txt` (not among the 7 failing binaries; this file's seam compiles on Linux since it is not cfg-gated).
- Trait design: `subscribe` replaces the previous queue (documented 40-41); `poll` distinguishes push-style (Windows COM, 0 iterations) from poll-style (PulseAudio) integrations (48-52) — a correct abstraction over the two OS event models.

## 3. Findings

### F-001 [S3]
Where: crates/platform-windows/src/audio.rs:29-57 (the trait) — crate-wide fact
Code:
```rust
pub trait AudioDevices: Send {
    /// Devices visible right now. An empty list is honest ("no
    /// devices"), matching [`AudioBackend::enumerate`]'s contract.
```
What is wrong: the seam, its fallback, and its hash have no consumer in any shipped binary. No crate or app depends on platform-windows (proof in my dialogs.rs audit F-001: rg over every Cargo.toml). The internal consumer (wasapi.rs) is cfg(windows) and equally orphaned.
Why it matters: identical structural finding to dialogs.rs — finished, tested platform code wired to nothing; every baseline build/test minute spent on it produces no shippable capability. The module doc's future tense (10-12: "When that gate opens, the real backend consumes this seam's DeviceDescs directly") is accurate about the ADR-005 admission gate but the seam is currently product-dead.
What correct looks like: the desktop shell gains a platform-crate dependency (cfg-gated), or the capability report carries "not yet integrated" for the whole crate.
Confidence: HIGH — Verification: cross-referenced dialogs.rs F-001 evidence (Cargo.toml sweep).

### F-002 [S3]
Where: crates/platform-windows/src/audio.rs:106-114 (identical at platform-linux/src/audio.rs:106 and platform-macos/src/audio.rs:106)
Code:
```rust
pub fn stable_device_id(os_identifier: &str) -> DeviceId {
    // FNV-1a 64-bit over the UTF-8 bytes.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in os_identifier.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    DeviceId::new(hash)
}
```
What is wrong: the function is triplicated verbatim across the three platform crates (diff of the 96-114 range: code byte-identical; only the doc comment's platform-list order differs). All three crates already depend on `audio_output` (line 16 imports `DeviceDesc, DeviceId` from it) — the shared dependency that could host one copy.
Why it matters: the module doc itself promises "same [`DeviceId`] minting, same names" for the future backend (10-12). Three copies of the minting function make that promise drift-prone: a prime/basis change or a prefix tweak in one copy silently changes that platform's ids while the doc still says "same minting". Each copy also carries its own pinned-vector test — three tests to keep in sync for one algorithm.
What correct looks like: `stable_device_id` lives in `audio-output` next to `DeviceId` (it is pure, std-only, and OS-independent); the platform crates re-export or call it.
Confidence: HIGH — Verification: diff across all three files; dependency direction confirmed (all three import from audio_output).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `AudioDevices` (trait) | 29-57 | REAL | S3 | Sound contract (enumerate/subscribe/poll with push-vs-poll semantics documented); unreachable from shipped binaries (F-001) |
| `UnavailableAudioDevices` (struct) + `because`/`reason` | 63-80 | REAL | — | Typed-refusal fallback; tested |
| `impl AudioDevices for UnavailableAudioDevices` | 82-94 | REAL | — | Every method returns the reason — tested at seam_behaviour.rs:31-47 |
| `stable_device_id` | 106-114 | REAL | S3 | Correct FNV-1a-64 (constants + pinned vector verified); triplicated (F-002) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `unavailable_audio_devices_returns_the_typed_reason` (31-47) | YES | All three methods asserted to fail with the exact reason — a fake success (Ok(vec![])) fails it |
| `stable_device_id_is_deterministic_and_pinned` (300-310) | YES | Published FNV-1a-64 vector for "a", a real WASAPI-shaped id pinned, same-input equality, different-input inequality |
| `event_queue_drains_fifo_and_empties` / `event_queue_is_bounded_and_counts_drops` / capacity clamp (203-250+) | YES | Order, drop-count, and newest-survive semantics asserted exactly (exercises the events.rs collaborator, another agent's file) |
| Baseline: green — platform-windows not among the 7 failures in `orchrun-test-workspace-nofailfast.log.txt` | — | cited |
| No test covers: the WASAPI implementation itself (cfg(windows), never compiled here) | — | inherent, honestly documented in windows/mod.rs |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the streaming half (`AudioBackend::open`, the render callback) stays behind the ADR-005 admission gate" | module doc 6-9 | REAL | No open/render implementation exists in this crate (wasapi.rs is enumeration/notifications only — imports at wasapi.rs:50 are the trait + hash) |
| "When that gate opens, the real backend consumes this seam's DeviceDescs directly: same DeviceId minting" | module doc 10-12 | REAL-UNPROVEN (future) | The minting exists (106-114) but is triplicated (F-002) — "same minting" rests on three copies staying identical |
| "never a fake success" (fallback doctrine) | doc 26-28 | REAL | Tested (31-47) |
| "documented as not collision-free" | doc 101 | REAL | Verbatim at 101-104 |

## 7. Dead / duplicated / unreachable code
- Unreachable: the whole crate from shipped entry points (F-001).
- Duplicated: `stable_device_id` × 3 (F-002).

## 8. Dependencies & flags
- Not cfg-gated (compiles everywhere — the seam files are OS-neutral by ARCH-6); the Windows half of the crate is gated.
- Depends on `audio-output` (DeviceDesc/DeviceId) and crate-internal error/events. No unsafe in this file (the crate's unsafe lives in windows/com.rs/wasapi.rs).

## 9. Open questions for the next agent
- Q-1 (audio-output owner): would hosting `stable_device_id` there break the layering (audio-output is L3; platform crates L5 depend on it — hosting is legal)? F-002's fix needs that call.
- Q-2 (ADR-005/cpal record owner): when the streaming gate opens, does the plan actually route through this seam, or through audio-output directly? If the latter, this crate's audio half is permanently dead and should be marked so.
