# ADR-011: TLS / HTTPS Strategy — accepted (ureq+rustls(ring) ratified as the E2-class exception)

- **Status:** **accepted (2026-11-09).** The Lead ratified this ADR's
  recommended posture (decision ratified by the orchestrator,
  2026-11-09, activating issue #39): the ureq 3.5.0 +
  rustls(ring 0.17.14) deviation is admitted as the **E2/ADR-001-class
  exception** per this ADR's own audit (§"The audit" — rustls 0.23.46
  via ureq 3.5.0, the ring provider's 135 C/asm files documented as
  the recorded E2 deviation) — record `ureq-3.5.0.md` R10's
  conditional admission ("ADMITTED — conditionally, on the Lead
  accepting the recorded ring deviation") stands, its condition now
  discharged by this acceptance. Drafted 2026-10-19; decision record
  refreshed 2026-11-09 with the fresh rustls audit (issue #8);
  accepted 2026-11-09 (issue #39).
- **Owner:** Lead decision (it touches the §9 language policy and a
  recorded dependency deviation); user-informed.
- **Evidence:** fresh rustls 0.23 audit executed 2026-11-09 in this
  session (real `cargo` data — tree + lockfile, §"The audit" below);
  `docs/dependencies/records/ureq-3.5.0.md` (the 2026-10-30
  validate-then-admit record whose R10 verdict is *conditionally
  admitted on the Lead accepting the recorded ring deviation* — this
  ADR is that acceptance vehicle); `crates/streaming/Cargo.toml` +
  `src/client.rs` (the admitted transport: ureq 3.5.0, rustls,
  verification always on, webpki-roots); `docs/formats/network-http.md`
  (§6.A1b row); `docs/feasibility/sections/subsystems.md` (0.C3-I
  finding, the original Phase-0 gap); `docs/inventory/security-phase0.md`
  (SEC-07 material change); doc 11 §6/§SEC-07.
- **Supersedes:** the 2026-10-19 interim posture ("no HTTPS dependency
  merges") — superseded *as a fact* by card 6.A1b/6.A1c (merged, wave
  6): ureq 3.5.0 with `features = ["rustls", "rustls-webpki-roots"]`
  is in the tree, the adapter is live, and a real HTTPS roundtrip was
  measured (record R10). The Lead's ratification of the recorded
  crypto-provider deviation is now given (2026-11-09, above).

## Context

The 2026-10-19 brief deferred this decision to "before the Phase 6
network wave". Phase 6 happened: HLS (6.B), DASH (6.C), ranged
resources (6.A2), auth (6.E1) all landed on a transport that needs
TLS, and real-world streaming is https-dominated (the old brief said
so itself: "most streams are HTTPS"). The deadline passed with the
decision unmade, and the repo resolved the tension the honest way —
admit conditionally, record the deviation, park the ratification on
this ADR (record R10: "**ADMITTED — conditionally, on the Lead
accepting the recorded ring deviation**"; `crates/streaming/Cargo.toml`:
"Known deviation, Lead-ADR ask").

The §9 conflict is unchanged from the Phase-0 finding: there is **no
production pure-Rust TLS provider**. rustls routes its crypto to
aws-lc-rs (C/C++, CMake build) or ring (C/asm, cc build); native-tls
binds the platform's OpenSSL (C) — and is additionally a platform
*linkage* dependency, the §9 failure mode the old brief named. So
every https-capable option carries a language-policy deviation; the
question is which deviation, how small, and whether it is recorded the
way the repo already records such things (the E2/ADR-001-class
exception: audited native code in a security-critical core, the same
posture the rav1d asm admission took).

## The audit (fresh, real cargo data — 2026-11-09, this session)

Method: scratch crate, `rustls = "0.23"`, `cargo tree` + `Cargo.lock`,
Rust 1.99.1-compatible resolution, default features. No claims below
come from anywhere but that run plus the repo's own ureq record.

- **Resolution:** rustls **v0.23.46**; **35 packages locked**; the
  reachable build graph is **19 crates** including build-dependencies.
- **Default crypto provider: aws-lc-rs v1.19.0** → aws-lc-sys v0.47.0,
  whose *build* dependencies are cc v1.11.1, **cmake v0.1.58**,
  pkg-config v0.3.34, fs_extra v1.3.0, dunce v1.0.5 — i.e. a C/C++
  (CMake) toolchain is required to build the default provider.
  aws-lc is AWS's BoringSSL fork: C + asm, not Rust.
- Runtime/build graph (unique crates): rustls, aws-lc-rs, aws-lc-sys,
  cc, cmake, find-msvc-tools, jobserver, libc, shlex, dunce,
  fs_extra, pkg-config, zeroize v1.9.1, log v0.4.35, once_cell
  v1.21.4, rustls-pki-types v1.15.1, rustls-webpki v0.103.16,
  untrusted v0.9.0, subtle v2.6.1.
- **ring is in the lockfile but NOT in the default build graph** — it
  is rustls's opt-in alternate provider (locked because optional deps
  resolve, unactivated because the default features chose aws-lc-rs).
  The repo's admitted tree (via ureq's `rustls` feature) activates
  exactly this alternate: **ring 0.17.14 — 135 native C/asm files,
  cc-only build, 230 unsafe tokens** (ureq record R1/R8, measured on
  the vendored tarball 2026-10-30).
- **Conclusion the data forces:** both rustls providers are C/asm. The
  choice between them is *which* native core, not whether: aws-lc-rs
  (default, heavier build: cmake + pkg-config, large C surface) vs
  ring (opt-in, cc-only, smaller audited surface). Neither is §9-clean;
  a §9-clean TLS stack does not exist at production grade (the old
  brief's `[VERIFY-P1: provider matrix]` is resolved negative — by this
  audit for the default provider, by the ureq record for ring).

## Options

**(a) Ratify the recorded deviation: ureq 3.5.0 + rustls with ring
0.17.14 [RECOMMENDED].** This is the *status quo of the tree* —
admitted 2026-10-30 (record ureq-3.5.0.md, R10 "conditionally, on the
Lead accepting the recorded ring deviation"), exact-pinned
(`=3.5.0`), minimal features (no native-tls, no socks, no
gzip/brotli — raw bytes preserve range semantics), verification never
off, Mozilla webpki-roots compiled in (no OS store probing), sync by
design (doc 01 §4.2). Deviation surface: ring's 135 C/asm files
built by cc, 230 unsafe tokens, all inside the crypto core — the
E2/ADR-001-class exception posture (audited native code in a
security-critical core) already taken for rav1d's asm. Accepting this
ADR ratifies that exception; rejecting it reverts HTTPS (record R10:
"If the Lead rules ring out, HTTPS is out with it"). New data from
this audit *for* ring over the default: aws-lc-rs drags a cmake +
pkg-config C/C++ build chain — strictly more native toolchain than
ring's cc-only build — so even a hypothetical re-admission around
default-provider rustls would not reduce the deviation.

**(b) rustls with the default aws-lc-rs provider (direct client, no
ureq).** Same language deviation, heavier build chain (cmake,
pkg-config), a second HTTP layer to write and audit, and it abandons
the measured ureq admission (redirect policy, deadline mapping, live
roundtrip evidence). No advantage the audit can find — the default
provider is the *worse* deviation. Rejected.

**(c) native-tls.** Platform linkage (OpenSSL on Linux, Schannel/
SecureTransport elsewhere) — the §9 platform-linkage failure mode on
top of the C-code failure mode, and the old brief already noted Linux
has no OS TLS *service* ABI, only libraries. Explicitly banned by the
6.A1b card per the record. Rejected.

**(d) Pure-Rust provider (RustCrypto-class) / keep waiting.** The
2026-10-19 option-4 posture. The ecosystem has not produced a
production-grade pure-Rust TLS provider in the intervening window:
rustls 0.23.46's default is still aws-lc-rs and its alternate is
still ring (this audit). Waiting now additionally means un-shipping
the merged 6.A1b/6.A1c/6.B work or carrying the deviation unratified.
Rejected as a posture; retained only as the fallback if the Lead
rules ring out (record R10 already names it: doc-04 TLS-chapter
revisit, not another crate).

**(e) HTTP-only v1 (https blocked).** Honest but strands the
https-dominated reality the old brief itself acknowledged, and is now
a *regression* of admitted, tested, evidence-recorded capability.
Last resort, same as the old brief ranked it.

## Decision [ACCEPTED — Lead ratification 2026-11-09]

**Option (a): ratify the recorded ureq 3.5.0 + rustls(ring 0.17.14)
deviation as the E2/ADR-001-class exception** — audited native crypto
core, exact-pin discipline, record re-triggered on any version/feature
change (doc 11 §3.7), native-tls and verification-off remain hard-banned
(the record's standing rules). **Ratified by the Lead 2026-11-09**
(decision ratified by the orchestrator; activating issue #39), on the
basis of this ADR's 2026-11-09 audit: rustls 0.23.46 via ureq 3.5.0,
both providers C/asm, ring 0.17.14 (135 C/asm files, cc-only build,
230 unsafe tokens, all inside the crypto core) the smaller recorded
deviation — the E2/ADR-001-class posture the rav1d asm admission set.
Acceptance actions executed in the same change (2026-11-09): this ADR
flipped to **accepted**; the seam below **deleted** from
`crates/streaming/src/schemes.rs` (the `HTTPS_ADR011_ADMITTED` const,
the guarded arm, the refusal arm, and the `SchemeError::HttpsNotAdmitted`
vocabulary, per the TODO(admitted-impl) instruction); SEC-07's
admission-pending note closed (doc 11) and the proposed-admission
register row marked ratified. The rejection fallback (revert per record
R10: https out, doc-04 TLS chapter revisit) is retired with the seam —
any future removal is a fresh decision against this accepted record.

## The seam (deleted by the 2026-11-09 acceptance — `crates/streaming/src/schemes.rs`)

Historical record of the pre-ratification state (landed with the
2026-11-09 refresh, deleted the same day by the Lead's acceptance):
until the Lead ruled, the https dispatch carried the ADR-011 marker as
code, in the house typed-refusal style:

- `SchemeError::HttpsNotAdmitted` (`streaming.scheme.https-not-admitted`)
  — a typed refusal vocabulary with its `code()`/`Display`/engine
  bridge arms, pinned by test;
- the single flip point `HTTPS_ADR011_ADMITTED` (a documented `const`)
  at the scheme-dispatch arm: while `true` (the current,
  conditionally-admitted state per record R10) https classifies to
  [`AccessKind::Http`] exactly as `http` does; the Lead's rejection
  flips it to `false` and every `https://` source fails typed with
  `HttpsNotAdmitted` — TODO(admitted-impl): Lead ratification deletes
  the const, the dormant arm, and the variant.

This is deliberately *not* the always-refusing stub pattern (ADR-004
`WebviewPending`, ADR-005 `audio.os_backend_pending`): those mark
not-yet-implemented capability; https is implemented, admitted
conditionally, and live — MP §5 forbids claiming a working
implementation doesn't exist, so the seam records the *pending
ratification*, not a fake absence.

## Consequences

- SEC-07 (doc 11) points here and gains the proposed-admission
  register row for the rustls provider deviation (2026-11-09 audit
  data: rustls 0.23.46, 35 locked / 19-graph, aws-lc-rs default vs
  ring 0.17.14 activated).
- ROADMAP §6 ADR-011 index line: "decision record refreshed
  2026-11-09, pending Lead admission".
- `docs/feature-matrix.toml` tls/https row: `partial` at refresh time
  (seam + conditionally-admitted transport evidence); **flipped on
  acceptance (2026-11-09)** to implemented on desktop targets with the
  honest split — routing + composition + TLS config pinned in-tree, the
  networked TLS round-trip lane [VERIFY: needs a networked CI lane]
  (mock testkit is plaintext; the out-of-band measured round-trip is
  record `ureq-3.5.0.md` R10).
- The old interim rule "Phase 6 cards that need HTTPS carry a
  `[DECISION-PENDING ADR-011]` marker" is discharged: the marker now
  lives in code (the seam) and this file, not in per-card text.

## Revisit triggers

- A production pure-Rust TLS provider reaching rustls-provider
  maturity (then option (d) reopens; ring leaves the tree).
- Any ureq version bump or feature change (record ureq-3.5.0.md
  re-triggers per doc 11 §3.7, and this ADR re-reads the provider
  row).
- A rustls default-provider flip (aws-lc-rs ↔ ring upstream) — the
  audit table above is the 2026-11-09 snapshot to re-run.
