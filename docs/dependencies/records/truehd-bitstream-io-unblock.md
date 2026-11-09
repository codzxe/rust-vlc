# docs/dependencies/records/truehd-bitstream-io-unblock.md — TrueHD admission unblock decision (issue #7, W16-EE)

> **Class:** decision record (admissions-wave follow-up, issue #7).
> Owner of the decision: **Lead** — everything here is
> **[PROPOSAL — Lead]**; this record changes no Cargo.toml, no code,
> no allowlist. Parent record: `truehd-0.7.2.md` (purity PASS,
> admission BLOCKED at R10; root cause isolated at R11). Date:
> 2026-11-09, branch w15/07-codec-admissions at main 62453fb.

## The blocker (from the parent record, not re-measured here)

`truehd 0.7.2` cannot compile under the workspace toolchain (rustc
1.99.1): the transitive `bitstream-io = "=4.10.0"` `define_huffman_tree!`
macro's **`ToBits` (write-tree) expansion** (`compile_write_tree_nodes!`,
`bitstream-io 4.10.0 src/huffman.rs:129+`) OOM-kills rustc — the OOM
fires at ~3.4–3.6 GB anon-RSS on the 4 GB / no-swap sandbox (dmesg:
`total-vm:20.6 GB, anon-rss:3.58 GB`), even for `cargo check`.
Bisection, a ~20-line minimal repro, and the escape hatch are all in
the parent record's R11. The decode-quality question was already
answered **YES in scratch**: four-stream ffmpeg differential, authored
streams bit-exact (R6, MD5s quoted there) — behind the same ~60-line
substitution this decision evaluates.

## The three options (drafted at parent R10; assessed here)

### (a) Upstream `bitstream-io` fix, then re-run the record

- **What:** file/track an upstream issue (+ PR if accepted) against
  `bitstream-io` for the `ToBits` expansion blow-up; wait for a
  release; re-verify the parent record against the same `truehd`
  0.7.2 pin.
- **Pros:** zero permanent maintenance for us; the fix lands for every
  `bitstream-io` consumer; no deviation from crates.io provenance; the
  parent record re-runs unchanged.
- **Cons / risk:** latency is outside our control (4.10.0 is the
  newest release, 2026-04-14 — the maintainer cadence is unknown to
  us); an upstream fix may land as a breaking 5.x, re-triggering the
  whole record anyway (doc 11 §3.7); until then **TrueHD admission is
  blocked indefinitely**.
- **Verdict:** right long-term play, wrong critical path.

### (b) ~60-line `[patch.crates-io]` vendor fork of `bitstream-io`

- **What:** a Lead-approved workspace `[patch.crates-io]` entry
  pointing at an in-repo (or vendored) copy of `bitstream-io` 4.10.0
  in which `define_huffman_tree!` emits only the **read-only
  `FromBits`** tree (the decoder never calls `ToBits`) — exactly the
  substitution already proven in scratch: full `truehd 0.7.2` compiles
  clean and produces the R6 bit-exact differential behind it.
- **Pros:** **the only option already proven end-to-end** (compile +
  4/4 bit-exact differential); bounded, reviewed diff (~60 lines);
  unblocks admission evaluation on current 4 GB-class hardware — no
  host-size lottery; stays pure Rust (no unsafe added); reversible —
  delete the patch the day an upstream fix ships.
- **Cons / risk:** we carry a divergence from the crates.io tarball —
  a supply-chain surface the R1–R9 scan did not audit (the fork must
  be re-scanned + sha256-recorded at creation); every `bitstream-io`
  or `truehd` bump re-triggers the patch review (doc 11 §3.7 applies
  to the patched graph, not just the pinned versions); `[patch]`
  directives live in the Lead-owned workspace manifest; fork rot if
  upstream refactors huffman.rs.
- **Verdict:** fastest honest unblock with the strongest evidence
  already in hand.

### (c) Retest on a ≥8 GB host

- **What:** `[VERIFY: retest on a ≥8 GB host]` — run the R11 minimal
  repro / full build on a bigger machine.
- **Pros:** zero code change; also answers whether this is a
  rustc-regression class (older rustc untested — the sandbox lacked
  disk for a second toolchain).
- **Cons / risk:** **does not fix the blow-up** — the expansion's
  20.6 GB total-VM demand gives no evidence 8 GB is sufficient (it
  OOM'd at 3.58 GB anon-RSS with the OOM killer, not a graceful
  bound); even if it passes, it makes ≥8 GB a hidden platform
  requirement for every contributor/CI host that builds the
  workspace, contradicting the 4 GB sandbox-class dev environment;
  no numbers exist yet — pure hope.
- **Verdict:** cheap to keep as a parallel probe, unacceptable as the
  plan.

## Recommendation [PROPOSAL — Lead]

**Adopt (b), with (a) ridden in parallel and (c) kept as a one-command
probe** (the R11 minimal repro is the check on any host). Rationale:
(b) is the only option whose full path — compile + bit-exact
differential — is *already measured* rather than hoped for; it is
bounded (~60 lines), reversible, and independent of upstream release
cadence; (a) retires the patch later and (c) costs nothing to run but
must not gate anything.

**If the Lead ratifies (b), the implementation card must include
(minimum):**

1. The fork copy carries a header comment citing this record + the
   parent R11 root cause; diff vs the crates.io 4.10.0 tarball
   recorded (expect: read-tree-only `define_huffman_tree!`).
2. Full doc-11 R1–R9 scan + sha256 of the forked source recorded in
   the parent record's addendum (the current scan covers the
   unpatched tarball only).
3. `[patch.crates-io]` entry is Lead-owned (wave rules: no agent adds
   it); the parent record re-runs its R2 graph against the patched
   graph.
4. Re-trigger rule: any `bitstream-io` / `truehd` version bump
   re-opens the patch review (doc 11 §3.7).
5. Upstream issue (option a) filed and linked in the parent record —
   the patch is a bridge, not a home.

**If the Lead declines (b):** TrueHD stays honestly BLOCKED at the
parent record's R10 verdict, with (a) as the only exit; the
`codec-audio-truehd-mlp` matrix row stays `blocked` with this record
linked as the unblock plan.

*Evidence posture (AGENTS §3.4): no new measurements in this record —
every number and claim above is cited from the parent record
`truehd-0.7.2.md` (R6/R10/R11, measured 2026-10-24) or from the
in-tree codecs suite evidence of 2026-11-09.*
