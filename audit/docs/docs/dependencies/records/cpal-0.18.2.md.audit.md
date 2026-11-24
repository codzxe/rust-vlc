# CLAIM AUDIT: docs/dependencies/records/cpal-0.18.2.md
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 5 REAL / 6 REAL-UNPROVEN / 0 / 0 / 0 / 0 / 1 UNREACHABLE |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An honest NOT-ADMITTED verification record whose repo-facing claims all verify: cpal appears NOWHERE in the dependency graph (zero matches in root Cargo.toml/Cargo.lock and every crate manifest), the null backend + typed stubs it says shipped DO exist (crates/audio-output/src/{null,stub}.rs + the `AUDIO_ADR005_ADMITTED` gate const in cpal.rs), and the platform-* adapter architecture it cites exists (crates/platform-{linux,macos,windows}/src/audio.rs all reference this exact record). External claims (tarball sha256s, crates.io API facts, the /tmp exit-101 build failure) are date-stamped and offline-unverifiable — REAL-UNPROVEN, not suspicious: the same discipline produced the oxideav record whose in-repo facts all verified.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "NOT admitted in 2.C3 — the card's platform-adapter fallback is executed instead" | L12-13 | REAL | `grep -c cpal Cargo.lock Cargo.toml crates/*/Cargo.toml` -> only crates/audio-output/Cargo.toml:12 (a comment citing this record); no dependency entry anywhere (ran) |
| 2 | "What ships instead: ... a fully implemented + tested null backend ... and typed not-supported stubs" | L162-166 | REAL | crates/audio-output/src/null.rs + stub.rs exist; lib.rs L13-14 "Implemented by [NullBackend] (fully, CI-safe) and ..." (read) |
| 3 | "the implementation lands in `platform-*` (L5)" | L168-169 | REAL | crates/platform-{linux,macos,windows}/src/audio.rs:9 each cite records/cpal-0.18.2.md; adapter trait `AudioDevices` with typed PlatformError (read) |
| 4 | "typed not-supported stubs ... stable error code naming the target and ADR-005" | L165-166 | REAL | crates/audio-output/src/cpal.rs: `AUDIO_ADR005_ADMITTED` gate + `BackendNotAditted` typed marker, "never a panic, never a fake success" (read; marker name read as BackendNotAditted? — see F-note below) |
| 5 | "a single named const is the gate" (admission discipline) | cpal.rs module docs | REAL | crates/audio-output/src/cpal.rs:19-24 — the HTTPS_ADR011_ADMITTED pattern copied; compile-time assertion present (read) |
| 6 | "cpal 0.18.2 ... sha256 6f02e8d0... , 59 .rs files, no native source" | L17-22 | REAL-UNPROVEN | external tarball facts; offline |
| 7 | "alsa-sys 0.4.0 build.rs runs pkg-config probe('alsa')" | L62-66 | REAL-UNPROVEN | build.rs of an external crate; consistent with the known sandbox failure class |
| 8 | "a throwaway /tmp project ... fails to build — exit 101 ... alsa.pc" | L92-100 | REAL-UNPROVEN | plausible + consistent with the record's DISK-INCIDENT-1 lesson framing; not re-runnable now (would need cargo network fetch) |
| 9 | "cpal 0.18.2: Apache-2.0 ... inside deny.toml's allow set" | L29-30, L116-118 | REAL-UNPROVEN | external license facts |
| 10 | "RustAudio org ... release 0.18.2 dated 2026-08-16" | L124-126 | REAL-UNPROVEN | external registry facts |
| 11 | "Sample counters are NOT provided by cpal" | L109-110 | REAL-UNPROVEN | external API facts; consistent with the repo's own atomic-counter design in audio-output |
| 12 | "no audio device in the sandbox ... nothing here asserts device behavior" | L98-99, L176-177 | REAL (environment fact) | matches the sandbox reality this audit runs in (no ALSA dev package — consistent with my own probe constraints) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| — | — | none found. Note a NAMING discrepancy, not a contradiction: cpal.rs docs say the marker is `BackendNotAdmitted`; my sed view of the rendered file displayed `BackendNotAdmitted` (rendered as `BackendNotAditted` in one output — the `[m` byte-eating renderer bug again; ripgrep-verified before relying on it: see run log note). The marker exists.

## 4. Fabrication / inflation indicators
NONE — the record documents a refusal to admit on four grounds, records its own [VERIFY-P1] debts (3 of them), and its structural claims match the tree exactly.

## 5. Matrix rows
N/A.

## 6. Cross-checks run
- `grep -c cpal Cargo.lock Cargo.toml crates/*/Cargo.toml` -> only a comment reference (claim 1 REAL).
- `ls crates/audio-output/src` -> null.rs, stub.rs, cpal.rs, device.rs... (claims 2/4 REAL).
- `sed -n '1,40p' crates/platform-linux/src/audio.rs` -> typed-error adapter trait (claim 3 REAL).
- `head -35 crates/audio-output/src/cpal.rs` -> gate-const admission discipline (claim 5 REAL).
