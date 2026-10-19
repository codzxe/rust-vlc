# Dependency Allowlist — FORMAT (schema v1, not yet populated)

| | |
|---|---|
| **Implements** | doc 11 §3.1 (allowlist file), §2 (audit records R1–R10), §1.3 (intake), doc 13 §3 card 0.C4-I ("initial allowlist *format* per doc 11 method") |
| **Status** | FORMAT ONLY. No Cargo workspace or lockfile exists in Phase 0, so there is nothing to allowlist yet; populating it is Phase 1 task 1.E5 (deny.toml + CI job). The single filled example record (`records/wasmi.md`) is a pre-audit record, not an approval. |
| **Decisions this file encodes** | Doc 11 §3.1 proposes `deps/allowlist.toml`; this card (doc 13 §3 0.C4-I) places the format at `docs/dependencies/`. **Path decision deferred to the Lead/1.E5** — the schema below is path-agnostic; if `deps/` is chosen, move this file wholesale and update the `record =` links. `[PROPOSAL]` |

---

## 1. The allowlist file (TOML schema)

File: `deps/allowlist.toml` `[PROPOSAL: path per doc 11 §3.1; alternative
`docs/dependencies/allowlist.toml`]`. One `[[crate]]` row per approved crate
version-range. A crate present in `Cargo.lock` **without a row fails G3** (doc 11
§3.1); a version outside `version-range` fails; a feature-set mismatch fails.

```toml
version = 1  # allowlist schema version (bump on breaking schema change)

# ── Example row (fields are normative; values shown for wasmi are the
# ── pre-audit proposal — NOT an approval; see records/wasmi.md) ──────────
[[crate]]
name = "wasmi"                                   # crates.io name (required, unique)
version-range = ">=2.0.0, <2.1"                  # semver range approved (required)
record = "docs/dependencies/records/wasmi.md"    # R1–R10 audit record (required;
                                                 # relative path from repo root)
features = { default-features = false, enable = ["std"] }
                                                 # (optional) if present, the ONLY
                                                 # resolved feature set allowed —
                                                 # used for conditional verdicts
                                                 # (doc 11 §2.4 rav1d pattern)
license = "MIT/Apache-2.0"                       # SPDX from the record's R7 (required)
purity-class = "pure-rust"                       # pure-rust | os-exception-adr-XXX |
                                                 # exception-adr-004 (webview) | rejected
approver = "Lead"                                # must be Lead (AGENTS §1; doc 11 §10 Q10)
date = "2026-10-19"                              # approval date (required)
expires = ""                                     # (optional) re-audit deadline for
                                                 # conditional/aged rows
notes = "WASM extension host interpreter; ADR-002 pending"
                                                 # free text ≤1 line; detail lives in
                                                 # the record
ree-audit-triggers = [                           # events that reopen the record
  "version-change",                              # any bump of this crate
  "new-transitive-dep",                          # tree-diff delta (doc 11 §3.6)
  "buildrs-or-links-change",                     # scanner diff across the bump
  "license-file-change",
  "rustsec-advisory",                            # advisory touching the pinned version
  "feature-set-change",                          # deviation from `features` above
]
```

### 1.1 Field rules (testable by the G3 checker)

1. `name` matches `^[a-z0-9_-]+$` (crates.io normalization: `-` and `_` are the
   same crate; the checker normalizes both before matching).
2. `version-range` is a valid semver range; the lockfile version must satisfy it.
3. `record` must exist in the repo and must reference this row's exact name (the
   checker correlates tree-diff ↔ record ↔ row per doc 11 §3.6 — "fails on
   orphans").
4. `features`, when present, is matched against `cargo metadata`'s *resolved*
   features for that package (not just the manifest's declared features) —
   this is the rav1d-asm control of doc 11 §4.3.
5. `purity-class`:
   - `pure-rust` — F1–F7 all NO (normal row);
   - `os-exception-adr-XXX` — E2 class: OS-service/ABI binding covered by the named
     ADR (e.g. `os-exception-adr-005` for audio, `os-exception-adr-003` for
     windowing/GPU); the ADR must exist and list this crate's exact API surface;
   - `exception-adr-004` — E1 class: the OS-webview exception (Tauri stack);
   - `rejected` — recorded poison (kept for history/CI-ban correlation; a rejected
     row can never be imported — the checker denies it outright).
6. `approver` must be exactly `Lead` (delegation is doc 11 §10 Q10, unresolved).
7. `date`/`expires` are ISO-8601; the CI job fails on expired rows.
8. Dev-dependencies follow the same policy (doc 11 §3.1) — rows carry
   `scope = "normal" | "dev" | "all"` `[PROPOSAL: add field; default "all"]`.

### 1.2 Deviation from doc 11 §3.1 sketch (recorded)

Doc 11 §3.1's sketch had `version`, `record`, `version-range`, `features`,
`approver`, `date`, `notes`. This format adds `license`, `purity-class`,
`expires`, `ree-audit-triggers`, `scope` and renames nothing — additive only,
consistent with doc 11's `[PROPOSAL: layout]` markers. `[PROPOSAL — Lead
confirms at 1.E5]`.

## 2. The audit-record file template (R1–R10, per doc 11 §2)

File: `docs/dependencies/records/<crate>.md` (doc 11 §2.1 proposed
`deps/records/<crate>-<version>.md`; this card uses `docs/dependencies/records/`
per doc 13 §3 0.C4-I deliverables. **Version-in-filename vs not: Lead decides at
1.E5** — recommendation: keep the version *inside* the record (R-header) so
renames don't break links; multiple majors get `<crate>-<major>.md`.)

```markdown
# deps/records/<crate>.md — <crate> <version> (<pre-audit | full> record, filled <date>)

> **Class:** <Phase 0 pre-audit | Phase 1 full audit>. Evidence gathered <date>
> by <task id> using docs/dependencies/audit-method.md (crates.io API + sparse
> index + static tarball inspection; crate never built or executed).
> `[VERIFY-P0]` = needs Phase 0 close-out; `[VERIFY-P1]` = needs the Phase 1
> workspace (cargo tree / cargo-deny / vet). Acceptance is a Lead decision.

## R1 — Language & source
   <Rust/pure claim vs reality; repo URL; maintainers; crates.io publisher;
    tarball native-file count; extern "C"/#[link]/blob scan results;
    build.rs presence + one-line disposition>

## R2 — Transitive graph
   <declared deps table (kind/target/optional); closure count for the pinned
    feature set; notable entries; [VERIFY-P1: cargo tree at lockfile]>

## R3 — Build scripts
   <build.rs presence; token-level disposition (compiler? download? cfg-only?
    rustc-probe?); manifest `build` key>

## R4 — Native links
   <links= keys; extern "C" classification by file; binary blobs; F1–F7 answers>

## R5 — Targets
   <our desktop triples; wasm32 status; target-conditional native code cfg gates
    recorded verbatim>

## R6 — Actual feature coverage
   <what the crate verifiably does vs what our spec needs, with source citations
    (tarball paths/lines); corpus/tests/limitations; "a Rust API does not prove
    purity, a version number does not prove coverage">

## R7 — License
   <SPDX from manifest; license-file presence in tarball; allowlist compatibility;
    transitive license posture [VERIFY-P1: cargo-deny]>

## R8 — Maintenance & security record
   <newest release + cadence; downloads; advisory count/IDs/fixed-versions;
    open advisories affecting the pinned version (must be 0); maintainer count/
    bus factor; [VERIFY-P0: GitHub activity if API blocked]>

## R9 — Provenance
   <tarball URL + size + sha256; sparse-index cksum match; publisher identity;
    cargo-vet entry [VERIFY-P1]>

## R10 — Verdict
   <accepted (version range + feature pin) | conditional (the pin that makes it
    compliant) | rejected (which F-checks failed) | exception-ADR-required (which
    ADR)> — Approved by: — (Lead decision pending <date>)
```

## 3. Filled example

See **`docs/dependencies/records/wasmi.md`** — wasmi 2.0.0, filled 2026-10-19 from
the worked example in `audit-method.md` §4 (sha256-verified tarball, F1–F7 all NO,
MIT/Apache-2.0, pre-audit verdict PASS with the proposed Phase 1 pin
`>=2.0.0, <2.1` / no-default-features + `std`). It demonstrates every field,
including honest `[VERIFY-P0]`/`[VERIFY-P1]` markers (GitHub 403, cargo tree,
license-file absence in tarball).

## 4. Relationship to the other Phase 0 artifacts

- `audit-method.md` — how records are produced (M1–M9).
- `records/*.md` — the records (1 exists: wasmi; the other pre-audited candidates
  live as table rows in `docs/adr/ADR-002-evidence.md` and
  `docs/dependencies/preaudit-foundation.md` until Phase 1 promotes them).
- `allowlist.toml` — Phase 1 (1.E5): machine-checked by CI gate G3 together with
  `deny.toml` (doc 11 §3.2) and `tools/dependency-audit --deny-native` (§4.2).
- ADR-002 (`docs/adr/ADR-002-evidence.md`) — consumes the WASM-runtime candidate
  records for the Lead's 0.D3 decision.
