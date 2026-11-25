<!--FIXV1
agent: FX6-R1
title: custodial KEEP — whitespace-only rustfmt import-order hunk in desktop tests/common/mod.rs (no semantic change)
verdict: DONE
changed:
- apps/desktop/tests/common/mod.rs
tests: []
evidence_read:
- audit/fix/RULES.md
- audit/fix/waves/FX2/FX2-R1.md
- audit/fix/FIX-PLAN.md
- apps/desktop/tests/common/mod.rs
-->

# FX6-R1 — custodial KEEP of a whitespace-only rustfmt import-order hunk

One human section (per assignment).

**Class & FX2-R1 precedent.** An uncommitted, whitespace-only rustfmt hunk
left in the tree by an unclaimed sibling run, claimed custodially: reviewed
and KEPT here with zero edits by this agent — the hunk was already in the
tree, so FX6-R1's working-tree contribution to
`apps/desktop/tests/common/mod.rs` is nothing, and the file appears in
`changed:` as an ADOPTION (FX2-R1 did the same for the split `assert!` in
`apps/cli/src/containers/ogg.rs`: formatting-only → KEEP, no revert).
`tests: []` deliberately — an import re-order has no semantics to pin
(FX2-R1: "a whitespace reformat has no semantics to pin"); the desktop
suites that compile this shared fixture are the oracle.

**The hunk is present exactly as described** (tree :18-19
`std::sync::atomic` now precedes `std::sync::Arc`; :22 media_clock list is
`{MediaClock, MonotonicSource, PlaybackClock, NANO_BASE}`; :29
`use media_types::GenerationId;` moved below the
`media_types::frame`/`packet`/`time` block).

**Verification — mechanical, not by eye (scratch in `/tmp/fx6r1` only; the
real file never touched; no git/cargo; rustfmt run ONLY on /tmp copies,
since RULES permits it on files edited and I edited none):**
1. Same import set: HEAD reconstruction vs tree — 12 `use` statements each;
   as a multiset of (module path, symbol set) they are EQUAL — no symbol
   gained or lost, order changed only. (No `audit/files/` audit report
   exists for this path; FIX-PLAN.md carries no scoped item for it — this is
   a custodial claim only.)
2. Idempotency: `rustfmt --edition 2021` on a /tmp copy of the current file
   → zero changes — the tree is an e2021 fixed point, so there is no OTHER
   formatting drift anywhere in the 151-line file (left as found; nothing
   to restyle).
3. Round-trip: `rustfmt --edition 2024` on the tree copy reproduces exactly
   HEAD's import block (Arc first, NANO_BASE third, GenerationId right
   after media_clock — the diff's `-` side); `rustfmt --edition 2021` on
   that HEAD reconstruction → `cmp` **byte-identical** to the current tree.
   The uncommitted hunk is exactly and only what a rustfmt run with the
   2021 style edition does to HEAD's imports; nothing else in the file moves.
4. Every imported symbol (24: 16 from braced lists + 8 single-name) has a
   body use — `Must` via the `.must()` trait method at :65 — so the
   deny(unused_imports) compile surface is identical to HEAD's.

**Disposition: KEPT** (formatting-only, per the assignment's rule).

**Risks & followups.** Risk: none expected — the hunk is proven
order-only (multiset-equal imports, round-trip-identical under rustfmt), so
the file compiles or not exactly as HEAD did. Origin observation for the
orchestrator: `apps/desktop` is edition 2024 (`Cargo.toml:21`) and HEAD is
rustfmt-e2024-style; the drift is exactly rustfmt-**e2021**-style — i.e., a
run that used `--edition 2021`, the literal edition in RULES.md's rustfmt
exception (e2021/e2024 order imports differently). Worth fixing in RULES.md
alongside FX2-R1's rustfmt-PATH note, or edition-2024 crates will keep
reproducing this drift class. Scratch is confined to `/tmp/fx6r1`; nothing
outside this report was left behind.
