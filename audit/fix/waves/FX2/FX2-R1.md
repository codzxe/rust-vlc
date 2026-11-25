<!--FIXV1
agent: FX2-R1
title: remedial — host.rs unused `Demuxer` import (FX2-A01 fallout) removed; custodial KEEP of the whitespace-only rustfmt assert hunk in containers/ogg.rs tests
verdict: DONE
changed:
- apps/cli/src/host.rs
- apps/cli/src/containers/ogg.rs
tests:
- none — unused-import compile remediation + formatting-only custodial claim: no runtime surface to pin; the central build (deny unused_imports) and the pre-existing ogg tests are the oracles; no test authored rather than fabricate a name
evidence_read:
- audit/fix/RULES.md
- audit/fix/FIX-PLAN.md
- audit/fix/wave-FX2.tsv
- audit/fix/waves/FX2/FX2-A01.md
- audit/fix/waves/FX1/FX1-R1.md
- audit/files/apps/cli/src/host.rs.audit.md
- audit/files/apps/cli/src/containers/ogg.rs.audit.md
-->

# FX2-R1 — remedial batch: host.rs unused import + ogg.rs custodial claim

**Custodial note (item 2):** `apps/cli/src/containers/ogg.rs` appears in
`changed:` as an ADOPTION, not an edit — the uncommitted whitespace-only
rustfmt hunk left by an unclaimed sibling run is reviewed here and KEPT
(verification below). FX2-R1 made zero edits to ogg.rs; my working-tree
contribution to that file is nothing (the hunk was already in the tree).

## Approach

**Item 1 — `apps/cli/src/host.rs:95:18` (compile error).** The workspace
denies `unused_imports`; rustc flags the `Demuxer` token (col 18) of
`use plugin_api::{Demuxer, SeekSupport};` — the import FX2-A01 added for
its F0.3 fix ("the trait must be in scope for the `dyn Demuxer` method
call" — disproven by rustc itself: trait-object method calls resolve
without the trait name in scope; the same class FX1-R1 documented for
fx1_a05). FX2-A01's registry-truth edit routes the read through
`opened.demuxer.seekability()` (host.rs:924) on the `Box<dyn Demuxer>`
inside `OpenedMedia`, so no `Demuxer` token remained.

Fix: `use plugin_api::SeekSupport;` — `Demuxer` dropped, `SeekSupport`
kept. Every remaining name on that line verified to have a user by rg:
`SeekSupport` is used as a path (`SeekSupport::None`) at host.rs:924
(non-comment; lines 169/920/1328 are doc references). rg `\bDemuxer\b`
on host.rs now yields ZERO hits. Nothing else in host.rs was touched —
exactly one line replaced one line (file stays 1982 lines), and a /tmp
copy of the edited file came back from rustfmt byte-identical, so the
real file was never rewritten (my working-tree diff = the one import
line).

**Item 2 — custodial, `apps/cli/src/containers/ogg.rs` near :759.**
The described hunk is the `short`-fixture assert of the pre-existing
test `fx1_a08_flac_extra_data_is_the_stripped_34_byte_body`
(ogg.rs:728), now sitting at ogg.rs:762-765; a unified diff with 3
context lines starts at :759 and counts 5 diff lines (1 removed + 4
added) — matching the assignment's description exactly:

- HEAD (single line): `assert!(flac_streaminfo_body(Some(&short)).is_empty(), "33-byte body");`
- Tree (4 lines): `assert!(` / `flac_streaminfo_body(Some(&short)).is_empty(),` / `"33-byte body"` / `);`

## Verification

- **host.rs:** rg `\bDemuxer\b` → 0 hits (was 1: the import). rg
  `\bSeekSupport\b` → import :95, real use :924, three doc-comment
  mentions. The deny-lint error at :95:18 is thereby discharged; the
  only remaining name resolves and is used.
- **ogg.rs hunk is formatting-only — shown mechanically, not by eye
  (all scratch in `/tmp/fx2r1`; the real file never touched by me):**
  1. Token identity: same macro (`assert!`), same argument ORDER
     (condition first, message literal second), identical condition
     `flac_streaminfo_body(Some(&short)).is_empty()`, identical message
     `"33-byte body"`; joining removes only whitespace/newlines. No
     operator, literal, or operand movement — no semantic difference of
     any kind found.
  2. rustfmt idempotency: `rustfmt --edition 2024` (toolchain binary,
     default config; no rustfmt.toml exists in the source tree) on a
     copy of the CURRENT file → zero changes, i.e. the tree state is
     rustfmt-clean.
  3. Round-trip: I reconstructed the pre-hunk single-line assert
     (python join in /tmp) and ran rustfmt on it → the output is
     byte-identical to the current file (`cmp`: IDENTICAL). So the tree
     hunk is exactly and only what rustfmt does to that single-line
     assert; nothing else in the 796-line file moves — consistent with
     the claimed one-hunk, 5-diff-line, whitespace-only diff.
  - Note for the disposition rule: the neighbouring split asserts
    (`wrong_type` :771-774, `lying_len` :780-783, plus `None` :784
    single-line) are also token-identical to their single-line forms, so
    the keep decision is insensitive to which of the region's asserts
    the sibling's run actually split.
- **Disposition: KEPT** (formatting-only, per the assignment's rule),
  claimed custodially in `changed:`. The assert's semantics remain
  pinned by its own pre-existing test fn (it is inside
  `fx1_a08_flac_extra_data_is_the_stripped_34_byte_body`, unchanged
  token-for-token).
- rustfmt was run ONLY on /tmp copies (RULES allows rustfmt on files I
  edited; the host.rs copy was already clean, and ogg.rs I did not edit).

## Risks & followups

- **Compile-unverified locally** (cargo forbidden): the unused-import
  fix is rg-proven and the proof-of-compile is the orchestrator's
  central gate. Residual risk near zero — one name removed, the
  surviving name has a located user (:924).
- **rustfmt path drift:** RULES cites `/home/z/my-project/.cargo/bin`
  for rustfmt, but no rustfmt exists there (only cargo/rustc/clippy
  proxies); the binary is at
  `.rustup/toolchains/1.99.1-.../bin/rustfmt` (reports 1.10.0-stable).
  Worth fixing in RULES or the PATH so future agents don't lose the
  rustfmt exception silently.
- **No test authored (deliberate):** an unused-import fix has no
  runtime oracle an integration test can pin (the deny-lint compile
  step IS the oracle), and a whitespace reformat has no semantics to
  pin; the existing ogg tests cover the adopted hunk's test fn. If the
  gate demands a named test regardless, flag it — none exists to name
  honestly.
- **Custodial hunk identification:** git is forbidden for me, so "which
  assert is the uncommitted hunk" was derived from the assignment's
  description (5 diff lines, split assert, ~:759) plus the mechanical
  round-trip above; the orchestrator's `git diff` remains the
  authoritative statement of the hunk's extent. If the real diff turns
  out to contain anything semantic beyond the formatting-only hunk
  described, this adoption does not cover it — reassign.
- FX2-A01's report still carries the stale claim that the `Demuxer`
  trait import was required in scope; harmless now (the line is gone),
  but the sibling report text is not mine to edit.
