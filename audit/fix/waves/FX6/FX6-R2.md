<!--FIXV1
agent: FX6-R2
title: remedial — E0308 at fx6_frame_lane.rs:520 set_present_tap Arc coercion
verdict: DONE
changed:
- apps/desktop/tests/fx6_frame_lane.rs
tests: []
evidence_read:
- audit/fix/RULES.md
- audit/fix/FIX-PLAN.md
- audit/fix/waves/FX6/FX6-A04.md
- audit/fix/waves/FX6/FX6-R1.md
- apps/desktop/tests/fx6_frame_lane.rs
- apps/desktop/src/shell.rs
- apps/cli/src/host.rs
- apps/cli/src/pipeline_host.rs
-->

# FX6-R2 — the E0308 at fx6_frame_lane.rs:520 set_present_tap Arc coercion

## Approach

The wave's single compile error, as delivered: the test passed
`Arc::clone(&tap)` (`tap: Arc<CountingTap>`) into
`set_present_tap`, and rustc answered E0308 with the caret on the
`&tap` sub-expression — "expected reference `&Arc<dyn PresentTap>`,
found reference `&Arc<CountingTap>`".

Per the assignment's instruction, the API's exact signature was read
FIRST, at its definition sites:

- `apps/cli/src/host.rs:284` — `pub fn set_present_tap(&mut self, tap:
  Arc<dyn PresentTap>)` (the `CliHost` injector the test reaches via
  `ShellRuntime::engine_host`, shell.rs:139);
- `apps/cli/src/pipeline_host.rs:1532` — `pub fn set_present_tap(&self,
  tap: Arc<dyn PresentTap>)` (the consumer-side twin it forwards to).

So the API takes the trait object **by value** — not `&Arc<dyn …>`.
The note's reference types are the compiler's view of the `&tap`
argument *inside* `Arc::clone(&tap)`: the expected `Arc<dyn PresentTap>`
pins `Arc::clone`'s `?Sized` type parameter to the trait object, after
which `&tap: &Arc<CountingTap>` cannot meet the instantiated formal
`&Arc<dyn PresentTap>` — the unsizing coercion does not pass through
`&Arc<T>`. (The assignment's suggested `set_present_tap(&dyn_tap)` shape
would therefore itself be wrong — the parameter is owned; the passed
binding must be the `Arc<dyn PresentTap>` itself.) This is a drift
between the situation text and the real signature, recorded here per
RULES; the fix targets the real signature.

Production's own call to the same injector (`apps/desktop/src/shell.rs:187`)
passes a plain concrete binding by value and compiles — the argument-site
unsizing is the proven pattern. The test was restructured the same way:
the concrete `Arc<CountingTap>` handle stays for the assertions
(`tap.drain_records` at :566/:580, `tap.mailbox_stats()` at :613), while
a separately coerced clone rides the engine seam. `tap.clone()` (a
method call — the receiver pins the clone's `Self`, leaving no free type
parameter for the expected type to capture) is bound through a let with
the trait-object annotation, so the unsizing happens at the let's
coercion site and the call passes an exactly-typed binding — no coercion
remains anywhere near the argument.

## Changes per file

- **apps/desktop/tests/fx6_frame_lane.rs** — the one cited call site
  (old :517-520, in `fx6_a04_frame_counter_matches_decode_count_at_
  normal_pace`): the comment block grew by four lines explaining the
  two-handle structure, and
  `runtime.engine_host().set_present_tap(Arc::clone(&tap));` became
  ```
  let dyn_tap: Arc<dyn PresentTap> = tap.clone();
  runtime.engine_host().set_present_tap(dyn_tap);
  ```
  (now :525-526; the cited :520 line is comment after the restructure).
  Nothing else in the file's 721 lines moves: the five `fx6_a04_*`
  tests, the `CountingTap`/`Lane` harness, and every assertion are
  untouched — only the tap-install mechanics of the acceptance test.
  rustfmt `--edition 2021` exit 0 (the allowed exception; note the
  binary lives at
  `/home/z/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/rustfmt`
  — RULES' `/home/z/my-project/.cargo/bin` PATH hint carries cargo but
  no rustfmt, matching FX6-R1's PATH observation).

Sweep per the assignment: repo-wide rg for `set_present_tap` — src
sites (host.rs, pipeline_host.rs, shell.rs:187) are the shipped,
compiling production path; the ONLY test call site is the one fixed
here. No other `&Arc<Concrete>`-into-`&Arc<dyn Trait>` shape of this
API exists anywhere (the file's remaining `Arc::clone(` occurrences
are gone entirely — `Arc::clone` no longer appears in the test file).

## Test design

`tests: []` deliberately — this is a remedial compile fix of FX6-A04's
existing five tests, not a new proof. The regression pin for the audit
finding is FX6-A04's own suite, unchanged in semantics; nothing about
the proven properties (counters, conservation, content) moved. The
mechanics change is compile-level only: which Arc handle the engine
sees is identical (same underlying `CountingTap` object, shared via
Arc).

## Acceptance vs FIX-PLAN

The remedial scope is the wave's compile gate, not a FIX-PLAN §3 item
of its own: F1.2's in-process acceptance proof (FX6-A04's five
`fx6_a04_*` tests) now compiles against the injector's real, by-value
`Arc<dyn PresentTap>` signature, keeping the assignment's constraint
(tests-only, no src change, no dependency edits). Definition of done:
the code fix (minimal, existing style — the same passing shape
production shell.rs:187 uses); no new test required by the remedial
assignment; this report.

## Risks & followups

1. **Not compiled locally** (cargo forbidden; the orchestrator compiles
   centrally). The fix uses only the canonical let-site unsizing
   (`let dyn_tap: Arc<dyn PresentTap> = tap.clone();` — the book
   pattern) and passes an exactly-typed binding by value, mirroring the
   shipped, compiling call at shell.rs:187; residual risk is minimal.
   If the central compile still objects at this line, the next lever is
   the explicit-cast form (`Arc::clone(&tap) as Arc<dyn PresentTap>`),
   which removes inference from the coercion entirely.
2. The lint surface of the edit is clean: no unwrap/expect, no unsafe,
   no new imports (`Arc` and `PresentTap` were already imported),
   `dyn_tap` is consumed, `tap` still feeds the later concrete reads.
3. For the orchestrator: the situation brief's "API takes `&Arc<dyn
   PresentTap>`" is a misread of the compiler note — the real parameter
   is owned `Arc<dyn PresentTap>` (host.rs:284); worth carrying into any
   future remedial brief for this seam.
