# SBOM — CycloneDX 1.5 (issue #52, doc 11 §3.4)

`cyclonedx.json` is the software bill of materials for the **resolved
Cargo.lock graph** (registry dependencies + workspace members), generated
by the std-only xtask subcommand:

```
cargo xtask sbom            # regenerate + write this file
cargo xtask sbom --check    # drift gate: exit 0 no drift / 1 drift / 2 error
```

Drift is byte-for-byte (same convention as `fixtures --check` /
`schema-gen --check`) and wired into CI (the cargo-deny job, after the
deny check) and `scripts/full_gate.sh` step 9. **Any change to
`Cargo.lock` or the `product.toml` identity must be followed by
`cargo xtask sbom` + commit**, or the merge gate fails.

## Why not cargo-cyclonedx (documented deviation)

Doc 11 §3.4 sketches `cargo-cyclonedx`. It is a third-party tool; under
the clean-room dependency rules (AGENTS.md §3.2) and the xtask std-only
posture (C5) the generator is hand-rolled in
`tools/xtask/src/cmd/sbom.rs` — its entire trust base is that one file.
[PROPOSAL] swap to cargo-cyclonedx later if the Lead prefers; the
artifact shape (1.5, subject metadata, per-component hashes) is
compatible. `cargo auditable` binary embedding (doc 11 §3.4) remains
unimplemented — no release pipeline consumption yet (8.E3's card).

## What it carries

- `metadata.component` — the product subject (`[identity]` name /
  display_version / publisher from `product.toml`).
- `components` — one per `[[package]]` row, sorted (name, version):
  `type: library`, `scope: required`, `bom-ref`
  (`pkg:cargo/<name>@<version>` for registry packages, opaque
  `<name>@<version>` for workspace/path packages), `purl` +
  `externalReferences[].url` (crates.io distribution endpoint) for
  registry packages, `hashes` (lockfile `checksum` as SHA-256) where
  present, and `lumen:cargo-lock-source` / `lumen:license-coverage`
  properties.
- `dependencies` — the lockfile dependency graph (`dependsOn`), plus the
  subject depending on every path-source package. Shipped-app vs
  dev-tool split is NOT encoded in `Cargo.lock`; this SBOM describes the
  whole resolved workspace graph.
- `serialNumber` — deterministic content-derived UUIDv8 (RFC 9562
  custom version; FNV-1a fingerprint of the inputs, not cryptographic).
  Same lockfile + identity → byte-identical SBOM, which the drift gate
  requires. No timestamp is emitted for the same reason.

## Known limits (honest register)

- **No licenses**: `Cargo.lock` carries no license fields; G3 cargo-deny
  owns license policy (recorded in `metadata.properties`).
- **No author/CPE/vex data** — same reason.
- Covers the Rust/Cargo graph only. The wasm build graph (doc 09) and
  the npm UI graph (doc 11 §3.4 `sbom-ui.json` [PROPOSAL]) are separate
  future SBOMs; per-platform finalization is card 8.E3.
- Schema-conformance is by construction + structural self-checks
  (valid JSON, unique bom-refs, all `dependsOn` targets resolve);
  validation against the official `bom-1.5.schema.json` with a JSON
  Schema validator has not been run in CI [VERIFY].
