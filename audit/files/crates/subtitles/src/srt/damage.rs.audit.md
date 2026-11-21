# AUDIT: crates/subtitles/src/srt/damage.rs
| Field | Value |
|---|---|
| Auditor | W09-A02 |
| Date | 2026-11-21 |
| Lines / functions | 213 / 9 items (const, enum + 2 impls, struct ×2, sink methods ×3) + 2 tests |
| File verdict | UNREACHABLE (real, minimal, correct; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `DamageSink` consumed by `srt/parser.rs`, `srt/encoding.rs`, `srt/tags.rs` (all in-crate) + integration tests (`srt_fuzz_survival.rs:25` imports `damage::MAX_DAMAGE_EVENTS`); crate-level proof in ass/tags.rs audit §2 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | per-variant construction census: `rg -c "SrtDamageKind::<k>" parser.rs encoding.rs tags.rs mod.rs render.rs` for all 20 kinds (every kind ≥ 2 hits — no dead variants) · `sed -n` quote verification · baseline citation (below) |

## 1. One-paragraph verdict
This is what a damage ledger should be: 20 `Copy + Eq` enum variants, one
compiler-forced exhaustive `describe()`, a capped Vec (256 × 8 B) with a
saturating dropped-counter so hostile files cannot grow memory, and two
tests that pin the cap arithmetic and the description plumbing. All 20
variants are actually constructed somewhere in the srt module (census run,
no dead variants), and the module doc's `dropped_damage_events` reference
resolves to the real public field on `SrtDocument` (parser.rs:56, wired at
parser.rs:81/85 and 97/101 from `into_parts`) — verified during the
srt/mod.rs audit, retracting my initial suspicion of a phantom name. The
single defect is cosmetic: the "exhaustive" test array is hand-maintained
redundancy that cannot catch a new kind if `describe()` ever grows a
wildcard arm. Unreachable from any shipped binary like the rest of the
crate.

## 2. What actually works (verified)
- Cap arithmetic pinned by test: baseline
  `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` shows
  `test srt::damage::tests::ledger_caps_and_counts ... ok` and
  `every_kind_describes_itself ... ok` (inside the 267/267 subtitles lib
  result). The test records MAX+100 events and asserts exactly
  `events.len() == 256`, `dropped == 100` — a real mutation-killer.
- Variant liveness: census command above — every one of the 20 kinds has
  at least one construction/doc site in parser.rs/encoding.rs/tags.rs; no
  dead enum arms.
- `record()` memory bound: `events` can never exceed 256 entries
  (guard at line 148); `dropped` saturates (line 151). Worst-case ledger
  ≈ 2 KB — the D-2 doc claim holds.
- `describe()`'s match is exhaustive against the enum (no wildcard), so
  adding a variant breaks compilation — the descriptions cannot silently
  rot (this is what actually protects the invariant the test claims to
  protect).

## 3. Findings

### F-001 [S4] "Exhaustive at the time of writing" test is hand-maintained and non-self-enforcing
Where: crates/subtitles/src/srt/damage.rs:183-212
Code:
```
    #[test]
    fn every_kind_describes_itself() {
        // Exhaustive at the time of writing; new kinds must extend it.
        let kinds = [
```
What is wrong: the test enumerates the 20 kinds in a hand-written array.
Adding variant #21 compiles cleanly as long as `describe()` is extended
(match exhaustiveness forces that anyway), and the array silently stops
being exhaustive — the comment's "new kinds must extend it" is a hope, not
a mechanism.
Why it matters: the test's stated purpose (exhaustiveness) is already
guaranteed by the compiler at the `describe()` match; the array adds only
the `!describe().is_empty()` check, which a `""` arm would defeat for ALL
kinds at once. Belt-and-suspenders that misdescribes itself as a guard.
What correct looks like: drop the array, or generate it
(`#[test] fn describe_is_never_empty_for_all_kinds` via a macro/strum-like
walk if ever available std-only).
Confidence: HIGH — Verification: read; behavior reasoning (no mutation
run).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_DAMAGE_EVENTS` (const) | 15-17 | REAL | — | 256; pub; mirrors the per-crate convention (pgs/dvb/vobsub/cc have their own). |
| `SrtDamageKind` (enum) | 23-80 | REAL | — | 20 variants, `#[non_exhaustive]`, Copy/Eq/Hash — all live (census in §2). Per-variant docs are specific and honest (each says what was tolerated and what happened to the cue). |
| `SrtDamageKind::describe` | 84-108 | REAL | — | `const fn`, exhaustive match (compiler-enforced); 20 strings, none empty (pinned by test). |
| `Display for SrtDamageKind` | 111-115 | REAL | — | Delegates to `describe()` — consistent by construction. |
| `SrtDamage` (struct) | 120-127 | REAL | — | kind + 1-based line_no (0 = byte-decode stage) — semantics documented at the field. |
| `DamageSink` (struct) | 133-137 | REAL | — | Private Vec + `dropped` counter; surfaces publicly as `SrtDocument.dropped_damage_events` (parser.rs:56, wired at 81/85, 97/101) — the module doc's field reference is accurate. |
| `DamageSink::new` | 141-143 | REAL | — | Default delegate; `Default` derived — `new` without args is clippy-conventional here. |
| `DamageSink::record` | 147-153 | REAL | — | Cap guard + `saturating_add` — the D-2 core; correct. |
| `DamageSink::into_parts` | 156-158 | REAL | — | Consuming split; used by tests and (per parser docs) the document. |
| tests ×2 | 165-212 | REAL | S4 (F-001) | Cap test excellent; exhaustiveness test mislabeled. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ledger_caps_and_counts | YES | Exact len/dropped/first-event assertions; breaks on cap-off-by-one, on non-saturating add (debug overflow), on event shape change. |
| every_kind_describes_itself | PARTIALLY | Catches an empty `describe()` string for the 20 listed kinds; cannot catch kind #21 (array is manual) — see F-001. The Display==describe check is real. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The SRT parser NEVER aborts a file and NEVER panics — damage is data" | damage.rs:3-4 | REAL (this file's share) / crate-level | The ledger provides the never-fail record channel; the no-panic half is parser.rs's burden — its fuzz-survival lane passed in baseline (`srt_fuzz_survival` ok in no-failfast log). |
| "the ledger is capped at MAX_DAMAGE_EVENTS entries; further events are counted in dropped_damage_events" | damage.rs:9-13 | REAL | record():147-153; pinned by test; `dropped_damage_events` is the real public field of `SrtDocument` (parser.rs:56, wired at 81/85 and 97/101). |
| "the differential ACs assert exact damage sequences, not just counts" | damage.rs:21-22 | REAL (in-crate) | srt/tags.rs tests assert `Vec<SrtDamageKind>` equality (e.g. unknown_and_unterminated_tags). |
| matrix row `subtitle-srt` "105/0 at card, 230/0 current tree" | feature-matrix.toml:901 | REAL (in-crate tests) / UNREACHABLE (product) | Baseline no-failfast: subtitles lib 267/267; nothing shipped links the crate. |

## 7. Dead / duplicated / unreachable code
- Unreachable from shipped entry points (crate proof, ass/tags.rs audit §2).
- No dead variants (census), no dead functions. Duplication: each format
  crate (pgs/dvb/vobsub/cc) carries its own MAX_DAMAGE_EVENTS + sink-shaped
  pattern — consistent convention, not copy-paste rot (values and types
  differ per crate).

## 8. Dependencies & flags
- std-only, no features, no unsafe, no panics possible (Vec push on ≤256
  elements; saturating add). `#[non_exhaustive]` on the public enum is the
  right call for a diagnostics vocabulary.

## 9. Open questions for the next agent
- parser.rs owner: confirm every kind's doc sentence matches the parser's
  actual disposition (e.g. `ReversedInterval` says "cue skipped" — the
  census shows the recording site, but the skip logic is yours to verify).
- If a shared damage-vocabulary crate is ever extracted, the per-format
  MAX_DAMAGE_EVENTS convention should get one home.
