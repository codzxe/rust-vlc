# ADR-011: TLS / HTTPS Strategy — brief (decision deferred)

- **Status:** proposed (2026-10-19; decision due before Phase 6 network wave, or earlier if any HTTPS need arises)
- **Owner:** Lead decision, user-informed (it touches the §9 language policy)
- **Evidence:** `docs/feasibility/sections/subsystems.md` (0.C3-I finding),
  `docs/inventory/security-phase0.md` (SEC-07 material change), doc 11 §6

## The finding

Phase 0 discovered there is **no production pure-Rust HTTPS client stack**
today: rustls routes its crypto to aws-lc-rs (C/C++) or ring (contains
C/asm); native-tls binds OpenSSL (C) on Linux. This is a §9 material gap that
planning docs did not anticipate (doc 04's audit focused on media crates).

## Options

1. **rustls + rustls-with-* pure-Rust crypto backends** — verify whether any
   pure-Rust provider (e.g. RustCrypto-based `rustls-rustcrypto` provider
   work) has reached usable maturity [VERIFY-P1: current provider matrix].
   Status: possibly viable for our needs (TLS 1.2/1.3 client-only), perf
   acceptable for media byte-streams.
2. **OS TLS as an E2 OS-service binding** — Schannel (windows-rs),
   SecureTransport/CoreFoundation (objc2), and on Linux… there is no OS TLS
   service userspace ABI (OpenSSL/GnuTLS/NSS are libraries, not OS services)
   → Linux would remain unserved → weak option.
3. **HTTP-only v1 with HTTPS blocked** — honest but painful (most streams are
   HTTPS); feature-matrix rows blocked-with-reason.
4. **Wait** — the RustCrypto TLS ecosystem is active; re-audit at Phase 5 gate
   before deciding.

## Interim posture (until decided)

- No HTTPS dependency merges. Phase 6 cards that need HTTPS carry a
  `[DECISION-PENDING ADR-011]` marker (doc 13 already uses this pattern).
- The audit list item is tracked as R-44-class; re-verification happens at
  every phase gate from Phase 4 onward.

## Recommendation framing [PROPOSAL]

Option 1 if the provider matrix verifies at Phase 5 gate; option 4 posture
until then; option 3 only as a last resort with honest matrix rows.
