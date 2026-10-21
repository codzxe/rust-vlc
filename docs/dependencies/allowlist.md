# Dependency Allowlist — Phase 1 snapshot (task 1.E5-R)

| | |
|---|---|
| **Implements** | doc 11 §3.1 (allowlist, one row per approved crate), §1.3 (intake), §2 (audit-record pointers), §3.7 (pinning/re-audit); doc 13 §4 row 1.E5; `allowlist-format.md` (schema v1 field rules + verdict vocabulary) |
| **Status** | Populated 2026-10-20 against the committed `Cargo.lock`; extended by card 1.D4 (same day) for the schemars family — the lock now resolves 39 packages: 28 registry + 11 workspace-local (the 1.E5-R snapshot's 31 = 22 + 9 predates the `commands`/`plugin-api` member entries). Compiled by task 1.E5-R; Lead ratification = merge of this file with `deny.toml` + the activated `cargo-deny` CI job (AGENTS §1: dependency approval is Lead-only). |
| **Enforcement** | `deny.toml` + the `cargo-deny` CI job (G3/G4, doc 10 §4.6). This document is the human-reviewable approval record; the machine-checked TOML form (`deps/allowlist.toml` per doc 11 §3.1) is deferred to the G3-checker card (see §6). |
| **Verdict vocabulary** | per `allowlist-format.md`: `pass` (pure Rust, permissive license) · `conditional` (E2-class OS-ABI bindings per target) · `fail` (forbidden) · `exception-ADR-XXXX`. `quick-pass` below = the preaudit-foundation quick method (crates.io API metadata: license/repository/provenance), **not** a full R1–R10 record — marked `[VERIFY]` until the tarball audit closes it. |

## 1. Path decision (records the deferral in allowlist-format.md)

`allowlist-format.md` left the allowlist's location open (`deps/allowlist.toml`
per doc 11 §3.1 vs `docs/dependencies/`). Resolved by this card **for the
document form**: doc 13 §4 row 1.E5 names `docs/dependencies/allowlist.md` as
the deliverable, and this file is it. The **machine-checkable** TOML stays
deferred: nothing consumes it until the G3 checker (doc 11 §3.6 tree-diff +
§3.1 row checker, `tools/dependency-audit`) exists, so shipping an unchecked
TOML now would only create a second drift surface. `[PROPOSAL: Lead confirms —
when the G3 checker card lands, it generates/owns `deps/allowlist.toml` from
this document or replaces it wholesale.]`

## 2. Method and evidence

- **Quick-pass method** (same as `preaudit-foundation.md`, 0.C4-R): crates.io
  REST API, `https://crates.io/api/v1/crates/<name>/<locked-version>`,
  User-Agent `lumen-deny-audit/1.0 (task 1.E5-R)`, fetched 2026-10-20. Raw
  JSON preserved in `tmp/preaudit-e5r/` (gitignored, same convention as
  `tmp/preaudit2/`). License column below is the API's `license` field for the
  exact locked version — not the docs page, not the repository.
- **Automated cross-checks from this card's tooling** (all 22 crates):
  `cargo deny --workspace check licenses sources bans advisories` = ok
  (SPDX evaluation against the minimal allow set; crates.io-only sources;
  RustSec advisory DB + yanked scan; named-crate ban list).
- **Not yet done** (owed, §7): full R1–R10 tarball records (F1–F7 scans,
  build.rs token analysis, sha256 provenance pins). A crate API does not
  prove purity — every quick-pass row below carries `[VERIFY]` for exactly
  that reason (AGENTS §3.4).

## 3. The allowlist (22 registry crates in Cargo.lock)

Direct workspace consumers named per `cargo tree --workspace` (2026-10-20;
`commands` added by card 1.D4): serde (product-config, settings-schema,
commands) · serde_json (product-config, theme-tokens, commands) · toml
(localization, product-config, settings-schema) · thiserror (localization,
product-config, settings-schema) · schemars (commands — the doc 05 §5.1
schema-derivation dep behind `cargo xtask codegen`). All other rows are
transitive-only. `scope = "all"` (normal+build edges; the workspace has no
registry dev-dependencies — `testkit` is a path dev-dep, §4).

| crate | locked | version-range | license (verified 2026-10-20) | purity-class | verdict / evidence |
|---|---|---|---|---|---|
| serde | 1.0.229 | >=1.0.229, <2.0.0 | MIT OR Apache-2.0 | pure-rust | **pass** — inherited preaudit-foundation (2026-10-19) |
| serde_json | 1.0.154 | >=1.0.154, <2.0.0 | MIT OR Apache-2.0 | pure-rust | **pass** — inherited preaudit-foundation |
| toml | 1.1.8+spec-1.1.0 | >=1.1.8, <2.0.0 | MIT OR Apache-2.0 | pure-rust | **pass** — inherited preaudit-foundation; its `[VERIFY-P1: license null at version endpoint]` is **closed** by this card's API check + green cargo-deny licenses run |
| thiserror | 2.0.21 | >=2.0.21, <3.0.0 | MIT OR Apache-2.0 | pure-rust | **pass** — inherited preaudit-foundation |
| schemars | 1.2.3 | >=1.2.3, <2.0.0 | MIT | pure-rust | **pass** — inherited preaudit-foundation (2026-10-12); card 1.D4 re-verified the license from the vendored crate manifest via `cargo metadata` (the crate is MIT-only, not dual-licensed). The doc 05 §5.1 schema-derivation dep for `commands` (catalog codegen). |
| dyn-clone | 1.0.20 | >=1.0.20, <2.0.0 | MIT OR Apache-2.0 | pure-rust | **quick-pass** `[VERIFY]` — dtolnay; via schemars (card 1.D4) |
| equivalent | 1.0.3 | >=1.0.3, <2.0.0 | Apache-2.0 OR MIT | pure-rust | **quick-pass** `[VERIFY]` — indexmap support (indexmap-rs org); via indexmap |
| hashbrown | 0.17.1 | >=0.17.1, <0.18.0 | MIT OR Apache-2.0 | pure-rust | **quick-pass** `[VERIFY]` — rust-lang org; via indexmap; 0.x ⇒ minor pin |
| indexmap | 2.14.2 | >=2.14.2, <3.0.0 | Apache-2.0 OR MIT | pure-rust | **quick-pass** `[VERIFY]` — via toml |
| itoa | 1.0.18 | >=1.0.18, <2.0.0 | MIT OR Apache-2.0 | pure-rust | **quick-pass** `[VERIFY]` — dtolnay; via serde_json |
| memchr | 2.8.3 | >=2.8.3, <3.0.0 | Unlicense OR MIT | pure-rust | **quick-pass** `[VERIFY]` — BurntSushi (std-adjacent); via serde_json; the one Unlicense-or-MIT crate ⇒ Unlicense in deny.toml's allow set |
| proc-macro2 | 1.0.107 | >=1.0.107, <2.0.0 | MIT OR Apache-2.0 | pure-rust (proc-macro) | **quick-pass** `[VERIFY]` — dtolnay; via serde_derive/thiserror-impl/syn/quote; R3 build.rs disposition owed (expected rustc-probe, unverified) |
| quote | 1.0.47 | >=1.0.47, <2.0.0 | MIT OR Apache-2.0 | pure-rust (proc-macro) | **quick-pass** `[VERIFY]` — dtolnay |
| ref-cast | 1.0.27 | >=1.0.27, <2.0.0 | MIT OR Apache-2.0 | pure-rust | **quick-pass** `[VERIFY]` — dtolnay; via schemars (card 1.D4); its `build.rs` was inspected by 1.D4: rustc-version probe + `OUT_DIR` module write only — no network, no native toolchain |
| ref-cast-impl | 1.0.27 | >=1.0.27, <2.0.0 | MIT OR Apache-2.0 | pure-rust (proc-macro) | **quick-pass** `[VERIFY]` — dtolnay; via ref-cast (card 1.D4) |
| schemars_derive | 1.2.3 | >=1.2.3, <2.0.0 | MIT | pure-rust (proc-macro) | **quick-pass** `[VERIFY]` — GREsau/schemars org; via schemars (card 1.D4) |
| serde_core | 1.0.229 | >=1.0.229, <2.0.0 | MIT OR Apache-2.0 | pure-rust | **quick-pass** `[VERIFY]` — serde-rs org; via serde/serde_json/toml family |
| serde_derive | 1.0.229 | >=1.0.229, <2.0.0 | MIT OR Apache-2.0 | pure-rust (proc-macro) | **quick-pass** `[VERIFY]` — serde-rs org |
| serde_derive_internals | 0.30.0 | >=0.30.0, <1.0.0 | MIT OR Apache-2.0 | pure-rust | **quick-pass** `[VERIFY]` — serde-rs org; via schemars_derive (card 1.D4) |
| serde_spanned | 1.1.2 | >=1.1.2, <2.0.0 | MIT OR Apache-2.0 | pure-rust | **quick-pass** `[VERIFY]` — toml-rs org; via toml |
| syn | 3.0.9 | >=3.0.9, <4.0.0 | MIT OR Apache-2.0 | pure-rust (proc-macro) | **quick-pass** `[VERIFY]` — dtolnay |
| thiserror-impl | 2.0.21 | >=2.0.21, <3.0.0 | MIT OR Apache-2.0 | pure-rust (proc-macro) | **quick-pass** `[VERIFY]` — dtolnay; via thiserror |
| toml_datetime | 1.1.2+spec-1.1.0 | >=1.1.2, <2.0.0 | MIT OR Apache-2.0 | pure-rust | **quick-pass** `[VERIFY]` — toml-rs org; via toml |
| toml_parser | 1.1.5+spec-1.1.0 | >=1.1.5, <2.0.0 | MIT OR Apache-2.0 | pure-rust | **quick-pass** `[VERIFY]` — toml-rs org; via toml |
| toml_writer | 1.1.3+spec-1.1.0 | >=1.1.3, <2.0.0 | MIT OR Apache-2.0 | pure-rust | **quick-pass** `[VERIFY]` — toml-rs org; via toml |
| unicode-ident | 1.0.27 | >=1.0.27, <2.0.0 | (MIT OR Apache-2.0) AND Unicode-3.0 | pure-rust | **quick-pass** `[VERIFY]` — dtolnay; via proc-macro2/syn; the one Unicode-3.0 crate ⇒ Unicode-3.0 in deny.toml's allow set |
| winnow | 1.0.4 | >=1.0.4, <2.0.0 | MIT | pure-rust | **quick-pass** `[VERIFY]` — winnow-rs org; via toml/toml_parser |
| zmij | 1.0.23 | >=1.0.23, <2.0.0 | MIT | pure-rust | **quick-pass** `[VERIFY]` — dtolnay; via serde_json (float writer) |

Common row fields (allowlist-format.md §1): `approver = "Lead"` (this
snapshot's compiler is task 1.E5-R; approval is the Lead's merge decision —
AGENTS §1, doc 11 §10 Q10) · `date = "2026-10-20"` · `expires = ""` ·
`scope = "all"` · `features =` unset (no conditional feature pins in this
set; the first one — rav1d asm-off, doc 11 §2.4 — is future work).

Re-audit triggers, all rows (doc 11 §3.7): version change · new transitive
dep · build.rs/links= change across the bump · license-file change ·
RustSec advisory touching the locked version · feature-set change. The
tree-diff gate (doc 11 §3.6) is what automates detection; until it exists the
same-PR rule is procedural: any `Cargo.lock` delta must co-occur with an edit
to this file.

## 4. Workspace-local packages (not allowlist rows)

9 path packages (`codec-api`, `localization`, `media-types`,
`product-config`, `settings-schema`, `template`, `testkit`, `theme-tokens`,
`xtask`) are `publish = false` and never enter the registry graph as
externals: they are the consumers, not consumed. Their licensing posture is
deliberately unresolved (ADR-006, doc 11 §8.2) — covered in cargo-deny via
`[licenses.private] ignore = true` (registry crates are unaffected; the skip
is scoped to unpublished workspace members).

## 5. Approved-but-not-imported and rejected (cross-references)

- **Pre-audited, not in the lockfile** (preaudit-foundation.md verdicts stand;
  they get rows here only when actually added): tracing /
  tracing-subscriber (pass), raw-window-handle (pass), wasm-bindgen (pass),
  cpal / wgpu / winit / windows / windows-sys / objc2 / core-foundation
  (conditional, E2), tauri / wry (exception-ADR-004). Pre-audit/verification
  records exist for four of these: `records/wasmi.md` (full R1–R10, ADR-002
  decision input — not yet imported; `plugin-host-wasm` is Phase 7),
  `records/cpal-0.18.2.md` (P0 verification, conditional-E2, NOT admitted —
  ADR-005 input), `records/wgpu-30.0.1.md` (deferral record, conditional-E2,
  NOT imported — admission-time obligations listed in its §6),
  `records/tauri-wry-2.x.md` (pre-audit, exception-ADR-004, NOT imported —
  the ADR-004 admission vehicle, criteria in its §"Admission criteria").
- **Rejected by policy** (deny.toml bans, 2026-10-20, every name verified to
  exist on crates.io before banning): the FFmpeg/libav family, codec C
  wrappers (libopus/minimp3/libvorbis/openh264/dav1d/x264/libvpx/libde265/
  libass/freetype bindings), the GStreamer family (21 core + 21 -sys + the
  101-crate gst-plugin-* enumeration), libVLC bindings, plus the
  build-machinery tripwires `libloading` and `bindgen`. Single candidate
  skipped: `gstreamer-utils-sys` (404 on crates.io — `gstreamer-utils` has no
  -sys sibling). Full list with provenance: `deny.toml`.

## 6. Enforcement wiring (this card)

- `deny.toml` (repo root) — cargo-deny 0.20.2 config: licenses (minimal allow
  set: MIT, Apache-2.0, Unicode-3.0, Unlicense — verified to be exactly what
  the lockfile consumes; adding a license class needs a same-PR edit, which
  cargo-deny itself polices via its `license-not-encountered` warning —
  observed 2026-10-20 by inserting a dummy `Zlib` entry) · advisories
  (`yanked = "deny"`) · sources (crates.io only) · bans (above).
- `.github/workflows/ci.yml` job `cargo-deny` — blocking, SHA-256-pinned
  cargo-deny 0.20.2 release binary, final step
  `cargo deny --workspace check licenses sources bans advisories`.
- Honest gap (doc 11 §3.2 note): cargo-deny cannot see inside `build.rs`.
  The F1–F7 tarball scanner (§4.2) and the tree-diff gate (§3.6) remain
  future cards; until then the quick-pass `[VERIFY]` rows above are the
  accepted residual risk, recorded as such.

## 7. Audit queue (owed records)

1. Full R1–R10 records for the 23 quick-pass crates (five added by card 1.D4's schemars family; method:
   `audit-method.md`; priority: proc-macro2 first — it is the only one
   expected to carry a build script).
2. Re-record R2 graphs when the lockfile moves (tree-diff gate automates the
   detection half).
3. `deps/allowlist.toml` machine form + checker when the G3 tool card lands
   (§1).

## 8. Evidence posture (honest state, AGENTS §3.4)

All verification in this file was executed **locally** in the build sandbox
on 2026-10-20 with cargo-deny 0.20.2 (SHA-256-pinned release binary) and
rustc 1.99.1:

- `cargo deny --workspace check licenses sources bans advisories` →
  `advisories ok, bans ok, licenses ok, sources ok`, exit 0 (2 expected
  wildcard warnings from intra-workspace path deps; 28 license notes, of
  which the 9 visible at `--log-level info` are
  `skipped-private-workspace-crate` for the publish=false members — i.e.
  `[licenses.private] ignore` behaves exactly as scoped).
- Negative fixture (throwaway project in /tmp, NOT this workspace):
  `ffmpeg-sys-next` added + deny.toml copied in → `cargo deny check bans`
  **fails exit 2** with `error[banned]` for ffmpeg-sys-next 9.0.1, bindgen
  0.73.2, libloading 0.8.9. Second fixture: `gst-plugin-cdg` → exit 2 with
  8 `error[banned]` hits (plugin + version-helper + gstreamer core + -sys
  family).
- CI-side sha pin negative control: the install script with a tampered
  digest → `sha256sum -c` FAILED, exit 1 (tarball never extracted).
- `cargo test --workspace --all-features` → 370 passed / 0 failed.
- actionlint 1.7.12 over `.github/workflows/ci.yml` → exit 0.

**The CI green-run link is PENDING first push**: this sandbox has no git
remote (`git remote -v` is empty — the 2026-10-20 rollback context, see
worklog 1.WAVE-RESTORE). Every statement above is local-run evidence; the
first real GitHub Actions execution of the `cargo-deny` job is
`[VERIFY on push]`, as with 1.E3-R's wasm.yml.
