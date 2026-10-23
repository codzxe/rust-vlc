# Error-surface fixes — closing the loop on F-1..F-5 (card 7-e, 2026-10-24)

Closing document for the fix list of
[`error-surface-20261023.md`](error-surface-20261023.md) §3. Every item
below names the commit, the verifying command, and the actual result
run on branch `wip/w7-e` at the commits cited. Honest status per item:
four **done**, zero partial, zero open.

| Item | Status | Commit | Evidence (command → result) |
|------|--------|--------|------------------------------|
| **F-1** — `pipeline.rs:1065` production `expect` retired via `WiredKind` + total `QueuePlan::for_kind`, typed `UnwiredKind` refusal for `Data` | **done** | `3c3ab10` | `git show 3c3ab10` touches `tracks.rs` (WiredKind, TryFrom, tests), `pipeline.rs` (wire/switch_limb/build_limb typed), `session.rs`, `tests/queue_policy.rs` (+35 lines of policy tests). The `expect` and its `allow` are gone (grep-clean). Tests: see the four-crate run below. |
| **F-2** — contract modules feature-gated out of release | **done** | `0d57502` | `crates/{input,media-probe}/src/contract.rs` under `#[cfg(any(test, feature = "contract"))]`; `resolve_origin` moved to production `seekable.rs`; `cargo run -p xtask -- check-panics` now counts both files under the "contract module" allowance (24 sites, release-invisible). Tests: see the four-crate run below. |
| **F-3** — `packet-queue` `cap()` const-position `loop {}` | **done** | `3c3ab10` | `git show 3c3ab10 -- crates/packet-queue/src/limits.rs`: the three doc-03 default literals are `const` items; a zeroed literal is now a **compile** failure, the runtime abort arm is deleted. |
| **F-4** — recycle-path `CodecDescriptor::new("unknown")` + `unreachable!` dance deleted | **done** | `3c3ab10` | `CachedDecoder` carries the parked track's descriptor; regression `parked_codec_rides_the_recycle_path` added; `cached_track_codec`/`empty_codec_descriptor` deleted. |
| **F-5** — grep gate closing the `unreachable!` clippy blind spot | **done** | `1235f35` + `914e528` | `cargo run -p xtask -- check-panics` → **`check-panics: PASS — 0 violations (audit F-5)`**. Seeded-violation proof below. |

## F-5 in detail

**The gate.** New `cargo xtask check-panics` (tools/xtask/src/cmd/check_panics.rs,
registered in `cmd::mod.rs` under card 7-e's granted xtask ownership):
lexical scan of the nine engine crates' `src/` trees for the six
panic-family forms (`.unwrap(` / `.expect(` / `panic!(` / `unreachable!(`
/ `todo!(` / `unimplemented!(`), matched on a string/comment-stripped,
char-aligned copy so doc examples and prose never fire. Sanctioned
zones: brace-tracked `#[cfg(test)]`/`#[test]` regions (including
file-level `#[cfg(test)] mod name;` targets and whole-file
`#![cfg(test)]`), the two F-2 contract modules, and `CITED_INVARIANTS`
— path **plus** raw-line message fragment, both must match.

**Predecessor review (relaunch, honest).** The dirty handoff had a
real bug: the cited-invariant check matched its fragment against the
*stripped* snippet, which blanks string literals — the one cited
invariant (audio-engine's `"fresh SharedBytes has refcount 1"`) could
never match, and the gate FAILed its own tree. Fixed by matching on
the raw source line (commit `1235f35`). The never-executed unit test
`cfg_test_regions_cover_their_items` was also RED as left
(`test_regions.len()==1` vs the nested-`#[test]` second region; a
line-number expectation off by the `\`-newline elision) — rewritten to
pin intent, not brittle counts (commit `914e528`).

**The three residual citations.** The audit's 28 non-test matches were
1 `expect` (F-1) + 27 `unreachable!` (24 in the F-2 contract modules);
the 3 residuals are cited in `CITED_INVARIANTS` with written proofs:
audio-engine `pipeline.rs` and codecs `pcm/decoder.rs` (same
`SharedBytes` just-constructed refcount-1 `Arc::get_mut` invariant,
audit §1.6) and playback-engine `recovery.rs` (`retrying()` ≔
`matches!(phase, Waiting{..})` with an early-return pinning the phase;
the audit counted but never rowed this site — the citation is the
reviewable act that makes it explicit). Retiring all three with typed
errors remains available follow-up if the Lead prefers zero citations;
nothing was silently passed.

**Seeded-violation demonstration** (requirement 2): appended
`fn _seeded_7e_demo() { unreachable!("seeded 7-e demo"); }` to
`crates/media-clock/src/lib.rs`, ran the gate, removed the seed:

```text
check-panics: FAIL — 1 violation(s) outside the sanctioned zones:
  crates/media-clock/src/lib.rs:145: unreachable!( in non-test code — fn _seeded_7e_demo() { unreachable!(                 ); }
  fix: return a typed error, or move the site into a #[cfg(test)] region; …
```

Exit 1, exact `file:line` named; `git checkout --` restored the file
(verified clean). The same proof runs permanently as fixture tests —
`tools/xtask/tests/check_panics.rs` spawns the real binary with
`--root` on scratch workspaces: seeded site fails (exit 1), sanctioned
zones pass (exit 0), a cited path with the *wrong* message fails
(scope proof), a missing scanned crate is exit 2 (no silent scope
shrink).

## Verification (actual commands and results)

- `cargo run -p xtask -- check-panics` → `PASS — 0 violations` (on the
  fixed tree; 3 cited-invariant + contract-module + cfg-test
  allowances itemized in the output).
- `cargo test -q -p xtask` → **109 + 12 + 4 + 12 passed, 0 failed**
  (3 ignored pre-existing), includes the 4 new fixture tests.
- `cargo clippy -p xtask --all-targets` → clean (0 warnings).
- `cargo fmt -p xtask` → applied.
- Four-crate suites (F-1/F-2/F-3/F-4 owners): `cargo test -q -p
  playback-engine -p packet-queue -p input -p media-probe` →
  **339 passed, 0 failed across 31 suites** (incl. the new
  `queue_policy.rs` WiredKind tests, `parked_codec_rides_the_recycle_path`,
  and the feature-off `file_stream.rs`/`truncation.rs`/`probe_io.rs`
  gates; run on the shared target dir, no other cargo/ffmpeg load
  observed competing at measure time).

## Honest limits

- The gate is lexical, not semantic: a `#[cfg(test)]` attribute on a
  statement inside a function body (nonexistent in this tree) would
  not open a region; `Option::unwrap(x)`-style free calls and spaced
  `unwrap (` shapes are outside both this net and the audit's raw
  sweep; direct-indexing panics (audit §4) remain unproven here.
- `assert!`/`assert_eq!` are not scanned (the audit kept them
  informational; all 43 non-test uses live in the gated contract
  modules).
- The three `CITED_INVARIANTS` entries are agent-cited (7-e), each
  with its written proof in the table; the Lead should review them as
  the intended reviewers.
