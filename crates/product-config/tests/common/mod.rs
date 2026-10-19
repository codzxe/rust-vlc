//! Shared helpers for the integration test suites.
//!
//! Failure paths assert first and then diverge (no `unwrap`/`expect`/
//! `panic!` — workspace lints, doc 10 §3.1/§3.3); the divergence arms are
//! unreachable in practice because the assertion has already failed the
//! test.

#![allow(dead_code)] // each suite uses a subset

use std::path::PathBuf;

use product_config::{Diagnostic, ProductConfig, Schema, load_file, load_str};

/// The Lumen workspace root (this file lives in
/// `crates/product-config/tests/common`).
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The committed root `product.schema.json` (doc 07 §3).
pub fn root_schema() -> Schema {
    let path = repo_root().join("product.schema.json");
    let json = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => unreachable!("cannot read {}: {err}", path.display()),
    };
    match json.parse::<Schema>() {
        Ok(schema) => schema,
        Err(err) => unreachable!("root schema failed to load: {err}"),
    }
}

/// The committed root `product.toml` (doc 07 §2 draft), validated against
/// the root schema.
pub fn load_root_product() -> Result<ProductConfig, Vec<Diagnostic>> {
    let path = repo_root().join("product.toml");
    load_file(&path, &root_schema())
}

/// Load a fixture document (file label = the fixture name).
pub fn load_fixture(name: &str) -> Result<ProductConfig, Vec<Diagnostic>> {
    load_file(&fixture_path(name), &root_schema())
}

pub fn fixture_path(name: &str) -> PathBuf {
    repo_root()
        .join("crates/product-config/tests/fixtures")
        .join(name)
}

pub fn read_fixture(name: &str) -> String {
    match std::fs::read_to_string(fixture_path(name)) {
        Ok(text) => text,
        Err(err) => unreachable!("cannot read fixture {name}: {err}"),
    }
}

/// Load a fixture's source under a chosen file label (used to reproduce the
/// doc 07 §3.5 example, whose file is `product.toml`).
pub fn load_fixture_as(file: &str, name: &str) -> Result<ProductConfig, Vec<Diagnostic>> {
    load_str(file, &read_fixture(name), &root_schema())
}

/// 1-based line of the first line containing `marker`.
pub fn marker_line(src: &str, marker: &str) -> Option<u32> {
    src.lines()
        .position(|line| line.contains(marker))
        .map(|i| i as u32 + 1)
}

/// The diagnostics whose key path is exactly `key`.
pub fn for_key<'a>(diags: &'a [Diagnostic], key: &str) -> Vec<&'a Diagnostic> {
    diags.iter().filter(|d| d.key_path == key).collect()
}
