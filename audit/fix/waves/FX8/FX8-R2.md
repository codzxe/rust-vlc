<!--FIXV1
agent: FX8-R2
title: remedial — fx8_a08_same_named_keys_outside_labels_are_not_normalized red at harness.rs:866 is a test-expectation drift (spurious trailing comma in the expected substring); normalizer is correct, assertion fixed to the verbatim payload line
verdict: DONE
changed:
- tests/integration/first-vertical/src/harness.rs
tests:
- fx8_a08_same_named_keys_outside_labels_are_not_normalized
evidence_read:
- audit/fix/RULES.md
- audit/fix/wave-FX8.tsv
- audit/fix/waves/FX8/FX8-A08.md
- audit/fix/waves/FX8/FX8-R1.md
- audit/files/tests/integration/first-vertical/src/harness.rs.audit.md
- tests/integration/first-vertical/evidence/transcripts/open.json
-->

# FX8-R2 — remedial: the one red fx8_a08_* test (harness.rs:866) is a TEST-EXPECTATION defect

## Approach

The central run's single red is
`harness::tests::fx8_a08_same_named_keys_outside_labels_are_not_normalized`
panicking at `harness.rs:866` on
`assert!(normalized.contains("    \"environment_kind\": \"ci-vm\",\n"))`.
Task: decide with code evidence whether the normalizer
(`normalize_run_specific`, harness.rs:738-782) or the test's expected
string is wrong, then fix the wrong side surgically — without weakening
the test's contract (same-named keys OUTSIDE labels are NOT normalized
away; F-004, `harness.rs.audit.md` §3).

1. Read the normalizer and all four `fx8_a08_*` tests
   (harness.rs:813/846/882/898) plus the `transcript_with_labels`
   helper; resolved every Rust `\`-line-continuation in the failing
   test's `format!` input by hand to recover the exact bytes fed to the
   normalizer.
2. Traced the normalizer's `in_labels` state machine line-by-line over
   that input (a python3 line-for-line simulation, /tmp scratch — no
   cargo, RULES.md exception).
3. Cross-checked the emitter's real shape against a committed artifact
   (`evidence/transcripts/open.json`) and re-verified the OTHER three
   fx8_a08_* tests pass unchanged (trace: their expectations all match
   the algorithm's output).

## Root cause: TEST-EXPECTATION (normalizer is correct)

The test input's payload object is (harness.rs:855-857, continuations
resolved):

```
  "payload": {
    "os": "plan9",
    "environment_kind": "ci-vm"     <- LAST member: NO trailing comma
  }
```

`"environment_kind"` is the payload's last field, so its input line is
`    "environment_kind": "ci-vm"` + `\n` with **no comma** — exactly as
the real emitter renders last members (`open.json`: the last array
member `"software-render"` and every closing member before `}` carry no
comma).

The normalizer's trace over the input: `in_labels` turns true on
`"labels": {` (harness.rs:775-776), rewrites only the two labels axes
(:862/:863 pass — `    "environment_kind": "per-run",` and
`    "os": "per-run",`), turns false on the labels-closing `  },` line
(:777-778), and therefore pushes the payload lines through the
`else { out.push_str(line); }` branch (:772-774) VERBATIM
(:865 passes — the payload `os` line DOES carry its comma). The
payload's environment_kind line emerges as
`    "environment_kind": "ci-vm"\n` — real bytes preserved, which is
precisely the contract the test exists to pin (F-004: outside-labels
same-named keys must keep their real bytes so drift fails the currency
compare).

The assertion at :866 expects
`    "environment_kind": "ci-vm",\n` — **with a trailing comma that
never existed in the input**. A stray-comma drift in the expected
substring: the sibling :865 line, whose payload `os` field DOES carry a
comma (a member follows it), passes — the comma was carried over onto
the last-field line where it does not apply. A python3 simulation of
the algorithm reproduces exactly this: :862/:863/:865 true, :866 false
— one red, matching the central run's single failure byte-for-byte,
while the :869 drift `assert_ne!` still holds (a payload `os` drift
stays visible after normalization).

Verdict: **the normalizer is correct** — "fixing" it to satisfy :866 as
written would mean mutating outside-labels bytes (adding a comma), i.e.
breaking the very contract all four tests pin. The test's expected
substring is the defect.

## The fix

`tests/integration/first-vertical/src/harness.rs` (was :866, now :868
after the comment grew) — drop the spurious trailing comma from the
expected substring:

```rust
// before
assert!(normalized.contains("    \"environment_kind\": \"ci-vm\",\n"));
// after
assert!(normalized.contains("    \"environment_kind\": \"ci-vm\"\n"));
```

One-token change (`,\n` → `\n`) plus a three-line comment making the
no-comma expectation explicit (last member of the payload object); the
normalizer and the test input are untouched. The corrected needle is strictly STRONGER as a pin, not
weaker: `contains` demands the exact bytes
`    "environment_kind": "ci-vm"` + `\n`, so it fails if the normalizer
either (a) rewrites the value (to `"per-run"`) — the F-004 regression
the test exists to catch — or (b) alters the line's punctuation in any
way (an injected comma breaks the `\n`-terminated substring match
exactly the way the current red proves). The test still asserts the
labels axes ARE rewritten (:862-863), the payload `os` keeps its real
bytes (:865), and a payload os drift stays visible after normalization
(assert_ne!, :869-874) — the SAME behavior contract, unchanged in
strength.

## Verification performed here (no cargo — python3 simulation)

A line-for-line python3 emulation of `normalize_run_specific`, fed the
exact test inputs (continuations resolved by hand), shows all four
fx8_a08_* tests GREEN with the fixed assertion — including the drift
`assert_ne!` — and both regression guards for the corrected needle:
if the payload's `environment_kind` is wrongly rewritten to
`"per-run"` the needle stops matching (F-004 caught), and if a comma
is injected into that line the needle also stops matching
(punctuation tampering caught). Scratch scripts deleted afterwards
(/tmp only, RULES.md).

## Expected central-run outcome

`cargo test -p first-vertical --no-fail-fast --lib`:
`fx8_a08_same_named_keys_outside_labels_are_not_normalized` turns
GREEN; the other three fx8_a08_* tests and the rest of the lib suite
are untouched (the edit lives inside one assertion's string literal
plus its comment, in that single test fn). The lib suite should read
14/14 green.

## Risks & followups

- The expected substring is matched with `contains`, so it pins the
  payload line's exact bytes but not the whole payload block; a future
  edit could `assert_eq!` the full normalized output to pin everything
  at once. Not done here (surgical scope).
- rustfmt: UNAVAILABLE on this box — no rustfmt binary anywhere on
  PATH and `/home/z/my-project/.cargo/bin/` does not exist (same
  finding as FX8-A08's report). The edit follows the file's existing
  style; verified zero tab characters and zero trailing-whitespace
  lines in the file (grep -c → 0/0).
