# `ui/src/generated/` — generated, do not edit

Everything in this directory is **machine-generated** by the command-catalog
codegen pipeline (card 1.D4, doc 05 §5.1 "Shared types"):

- `commands.ts` — the TypeScript type surface the UI imports. Derived from
  the Rust wire types in `crates/commands/src/catalog/` via
  `schemas/commands.schema.json` (schemars). No TS type in this tree is ever
  hand-written.

## Regenerate

```sh
cargo xtask codegen
```

(Shells out to `cargo run -p commands --bin catalog-gen --quiet --`; the
same generator the drift tests call, so the committed files and the tests
cannot disagree about what "regenerated" means.)

CI fails on drift between the Rust catalog types and these artifacts
(`catalog-gen --check`, plus the golden-file tests in
`crates/commands/tests/`). If a check fails, run `cargo xtask codegen` and
commit the refreshed files — never hand-edit them.
