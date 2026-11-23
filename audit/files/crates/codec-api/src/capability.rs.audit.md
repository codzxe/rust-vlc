# AUDIT: crates/codec-api/src/capability.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 353 / 13 production fns + 3 tests |
| File verdict | REAL code, **PAPER-ONLY wiring**: vocabulary + pure predicates, zero consumers anywhere in the repo |
| Reachable from shipped entry point? | **NO** — `CodecCapability`/`HwPreference`/`DimensionLimits`/`ProfileDescriptor` are constructed and read **nowhere** outside this file and its tests (rg over the whole tree: matches only in capability.rs + lib.rs re-exports) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## 1. One-paragraph verdict
The module itself is competently written: opaque profile/level descriptors with honest validation (non-empty, no control chars, typed errors), per-axis dimension limits with a 65 535 cap and min≤max enforcement, absence-means-unconstrained predicates, and an ADR-001 posture enum — all pure, all unit-tested (3/3 pass live). But it is a vocabulary for a registry that does not exist: no codec declares a `CodecCapability`, no code reads `HwPreference`, nothing matches a Track against these predicates. The doc's "the data the registry matches a [`Track`] against" describes a consumer that is absent from the entire repository — the module doc partially admits this ("Selection policy ... is the registry's (MP §6) — deliberately absent here"), but the registry itself (not just its policy) is absent. 353 lines of exported, tested, dead API.

## 2. What actually works (verified)
- `timeout 60 ./target/debug/deps/codec_api-8bf0f988e15216e7 capability --test-threads=2` → `3 passed; 0 failed`.
- rg whole-tree (excluding audit/): `CodecCapability|HwPreference|DimensionLimits|ProfileDescriptor` → only capability.rs definitions/tests + lib.rs:107-135 re-export lists (`pub use` / prelude). Zero downstream users.

## 3. Findings

F-001 [S3 — exported capability vocabulary with zero consumers; the doc names a registry that does not exist]
Where: capability.rs:198-205 (doc), 206-220 (struct)
Code:
```
/// What one codec implementation offers (doc 01 §2.4 "capability
/// descriptors") — the data the registry matches a
/// [`Track`](media_types::Track) against.
```
What is wrong: no registry exists in the repo. The shipped decode path is a hard-coded descriptor match in apps/cli/src/pipeline_host.rs (per-codec match arms) that never consults profiles, levels, dimensions, or hardware posture. Every decoder (h264 sw, vp9, av1, mjpeg, …) constructs none of this. The vocabulary is re-exported at codec-api's top level and in the prelude as if it were the contract surface, so it compiles into every shipped binary while being called by nobody.
Why it matters: the capability card (1.D1-R) is implemented as types-without-a-system; any matrix/doc row claiming capability-driven codec selection is describing code that does not exist. Also invites drift: the real selection behavior (CLI match arms) duplicates knowledge these types were meant to centralize.
What correct looks like: either the registration card (doc 01 §2.5(a)) lands a registry consuming this, or the module is cfg-gated/documented as reserved vocabulary.
Confidence: HIGH — Verification: rg (whole tree) + read.

F-002 [S4 — `validate_opaque` accepts arbitrary non-control junk (whitespace, giant strings) without length cap]
Where: capability.rs:87-92
Code:
```
fn validate_opaque(value: &str, code: &'static str) -> Result<String, EngineError> {
    if value.is_empty() || value.chars().any(|c| c.is_ascii_control()) {
        return Err(unsupported(code));
    }
    Ok(value.to_string())
}
```
What is wrong: a 10 MiB "profile" string or `"  "` passes validation; there is no length bound. Cosmetic today (no consumers), but a registry consuming this would inherit an unbounded-string attack surface from a module whose sibling (`CodecDescriptor`) is the established precedent for such caps.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| unsupported (helper) | 18-24 | REAL | — | Decode-stage Unsupported mint |
| ProfileDescriptor::new / as_str / Display | 37-57 | REAL | S4 | validation honest; F-002 no length cap |
| LevelDescriptor::new / as_str / Display | 61-84 | REAL | — | same shape |
| validate_opaque | 87-92 | REAL | S4 | F-002 |
| DimensionLimits::new | 120-141 | REAL | — | per-axis 1..=65535, min≤max, tested incl. cap edges |
| min_*/max_* accessors | 144-161 | REAL | — | |
| DimensionLimits::accepts | 165-170 | REAL | — | pure, tested |
| HwPreference enum | 182-196 | REAL (def) | — | zero readers in tree |
| CodecCapability::new | 225-233 | REAL | S3 | F-001: never constructed outside tests |
| supports_profile / supports_level / accepts_dimensions | 237-255 | REAL | — | absence = unconstrained, tested |
| tests ×3 | 262-352 | REAL | — | pass live |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| opaque_descriptors_reject_empty_and_control_bytes | YES | empty/`\n`/NUL rejected with exact codes; equality round-trip |
| dimension_limits_validate_per_axis | YES | 6 bad tuples + cap-edge acceptance |
| capability_predicates_treat_absence_as_unconstrained | YES | before/after declaration narrowing |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the data the registry matches a Track against" | capability.rs:199-200 | FALSE (no registry exists) | F-001, rg |
| "the registry combines the two" (capability + product.toml policy) | capability.rs:178-181 | FALSE (no registry) | same |
| "Selection policy ... is the registry's (MP §6) — deliberately absent here" | capability.rs:5-8 | TRUE as stated | absent — and so is everything downstream of it |
| "65 535 px covers every codec family's documented maximum" | capability.rs:98-100 | UNVERIFIED | plausible (16-bit container fields); not checked per family |

## 7. Dead / duplicated / unreachable code
The entire public surface is dead-in-tree (F-001): re-exported by lib.rs:115-118 + prelude, consumed by nobody. Real selection lives in apps/cli/src/pipeline_host.rs match arms.

## 8. Dependencies & flags
media-types error/ids only; part of codec-api (unconditional, shipped in every binary via the codecs dependency chain).

## 9. Open questions for the next agent
- Doc/matrix auditors: do any rows cite "capability descriptors" or "registry" as landed functionality? If yes, those rows are FALSE — this file is the entire implementation.
- If the registration card lands, F-001 flips to wired; the length-cap gap (F-002) should be fixed first.
