# Security sweep — supply chain (issue #52, card W23-F sweep 4 of 4)

Task ID `7.w23f` (agent `w23-f`, branch `w23/f`). Scope: the workspace
dependency graph per doc 11 §3 — `cargo deny --workspace check
licenses sources bans advisories` (0.20.2, SHA-256-pinned per ci.yml)
plus a `cargo tree --workspace` review. cargo-audit is **[PROPOSAL]
only** (§4). The SBOM deliverable of issue #52 (doc 11 §9) is
`docs/security/sbom/` + the xtask subcommand — see its own section in
the worklog; this document records the audit posture.

## 1. Tooling provenance (verified, not assumed)

cargo-deny 0.20.2 installed from the release tarball exactly as
`.github/workflows/ci.yml` pins it:

```
curl … cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz
sha256 9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f  ✓ (re-downloaded this sweep, matches the ci.yml pin)
binary sha256 b329e25933d01c36dd7c47d84ea5716694f9b7caf53a5003d45674703a8ed54a
cargo-deny 0.20.2
```

The pre-extracted binary at `/tmp/cargodeny/…/cargo-deny` is
byte-identical (sha256) to the freshly verified tarball's payload.

## 2. ACTUAL results — cargo-deny (all five gates)

`cargo deny --workspace check licenses sources bans advisories`
(0.20.2, worktree `w23/f` at commit 784c6e6):

```
advisories ok, bans ok, licenses ok, sources ok
```

**Zero errors.** Warning taxonomy (all warn-level, per the documented
deny.toml posture):

- **29 `wildcard` warnings across 25 crates** — machine-verified this
  sweep: every warned crate is a workspace member; every wildcard is
  an intra-workspace `path = "../x"` edge (version-less by design,
  never published). Zero registry `*` requirements. *Doc-drift note:*
  `deny.toml`'s bans comment still says "the only two expected
  warnings are the two workspace path edges" — that count has rotted
  (25 members carry path deps now). The protected invariant (no
  registry wildcard) holds; the comment needs a Lead refresh
  [PROPOSAL: reword to "every wildcard warning must name a workspace
  member; a registry name is a review stopper", or re-tighten to
  `deny` when cargo-deny gains path-dep exemption].
- **2 `duplicate` warnings** (multiple-versions = "warn" per doc 11
  §3.2 sketch): `getrandom` 0.2.17 (via ring 0.17/rustls/ureq) vs
  0.4.3 (via jobserver/nasm-rs/rav1d build path); `syn` 2.0.119
  (serde/strum/thiserror macros) vs 3.0.9 (atomig-macro via rav1d).
  Both are transitive build/macro dichotomies with no host-code
  duplication; dedup is upstream-gated (ring → getrandom 0.4).

**Advisories:** the single RustSec hit is the documented acceptance
`RUSTSEC-2024-0436` (`paste` 1.0.15 via rav1d 1.1.0 — unmaintained,
not vulnerable, no fixed upstream; drop with the next rav1d bump).
`yanked = "deny"` held: no locked crate is yanked.

## 3. cargo tree review (graph posture)

`Cargo.lock`: **156 packages = 38 workspace members (publish=false,
path deps) + 118 registry entries**; every `source` field is
`registry+https://github.com/rust-lang/crates.io-index` — **zero git
dependencies, zero path-source externals, no vendored tarballs**
(cargo-deny `sources ok` enforces the same). External crates in the
non-dev Linux graph: ~60 unique (serde/toml/thiserror families,
ureq→rustls/ring/webpki-roots for TLS, wasmi 2.0 for the plugin host,
rav1d + symphonia + filmcraft/oxideav/ruopus/rusty_vp9 decoder set,
cpal audio, log/parking_lot/bitflags utilities). Native-code edges
(`cc`, `nasm-rs`, `find-msvc-tools`, `libc`, `ring`'s asm) are all
inside the ADR-admitted crates (rav1d build, ring TLS) — no new
native surface since the 6.A1b/W18-KK admissions; every external
crate of consequence has an admission record under
`docs/dependencies/records/` (21 files).

## 4. cargo-audit — [PROPOSAL] only

Not run (no pinned binary, no Lead admission). Proposal for the Lead:
cargo-deny's `advisories` check already queries the same RustSec DB
with `yanked = "deny"`; a separate cargo-audit job would add only (a)
the `cargo audit` JSON output format for GitHub advisory triage and
(b) `--no-fetch` offline mode for hermetic CI. Recommend NOT adding it
unless the advisory-triage workflow (doc 10 §4.6 G4 auto-issue flow)
specifically needs the JSON — one tool, one DB, one pin is the
smaller supply-chain surface.

## 5. Verdict

Supply chain clean at first-pass depth: five cargo-deny gates green
on the pinned 0.20.2, registry-only lockfile, per-crate admission
records current, warning posture exactly as documented (modulo the
benign wildcard-comment drift recorded in §2).

## 6. Remaining for 8.A2 / 8.E3

1. Lead decision on the deny.toml wildcard-comment refresh (§2) and
   the `multiple-versions → deny` flip date (doc 11 §3.2).
2. `paste`/RUSTSEC-2024-0436 removal rides the next rav1d upgrade.
3. SBOM per-platform finalization (8.E3): the CycloneDX generator
   landed this sweep covers the Cargo.lock graph; attaching SBOMs to
   release artifacts (checksums, per-OS builds) is 8.E3's card.
4. `cargo vet` (doc 11 §3.4 sketch) — still unadmitted; the audit
   method doc (`docs/dependencies/audit-method.md`) is the standing
   substitute (manual records); revisit with the Lead.
