//! Drift gate for the committed token outputs (j18 F3).
//!
//! `lib.rs`, `Cargo.toml` and both generated headers claim the committed
//! outputs are "diff-tested by `crates/theme-tokens/tests/diff.rs`" — this
//! file IS that test, wiring the drift check into `cargo test --workspace`
//! (ci.yml `gate` job, full_gate.sh #3) so a repin of `themes/tokens.json`
//! without `cargo xtask tokens` (+ `npm run tokens:generate` for the UI
//! bridge copy) fails the gate instead of shipping silently.
//!
//! Compared against a fresh emission from the committed source:
//!
//!   * `themes/generated/tokens.css`              (crate CSS output)
//!   * `crates/theme-tokens/src/generated/tokens.rs` (crate Rust output)
//!   * `ui/src/tokens/generated/tokens.css`       (the verbatim UI-side
//!     bridge copy written by `ui/scripts/gen-tokens.mjs`)
//!
//! Byte equality, first differing line reported — the same contract as
//! `tokens-gen --check`, but running everywhere `cargo test` runs.

use std::fs;
use std::path::PathBuf;

use theme_tokens::emit::emit_css;
use theme_tokens::emit::emit_rust;
use theme_tokens::parse;

/// The Lumen workspace root (this file lives in crates/theme-tokens/tests).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn committed_source() -> Result<String, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(
        workspace_root().join("themes/tokens.json"),
    )?)
}

/// First line at which two documents differ (1-based; None when equal or
/// when one is a prefix of the other and lengths differ).
fn first_diff_line(a: &str, b: &str) -> Option<usize> {
    if a == b {
        return None;
    }
    let mut line = 1;
    for (x, y) in a.lines().zip(b.lines()) {
        if x != y {
            return Some(line);
        }
        line += 1;
    }
    Some(line)
}

fn assert_matches(path: &PathBuf, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let committed = fs::read_to_string(path).unwrap_or_else(|e| {
        panic_wrapper(&format!(
            "committed token output {} is unreadable ({e}) — run `cargo xtask tokens` and commit it",
            path.display()
        ))
    });
    let diff_line = first_diff_line(&committed, expected);
    assert!(
        diff_line.is_none(),
        "{} differs from the regenerated output at line {} — run `cargo xtask tokens` \
         (+ `npm run tokens:generate` in ui/) and commit the refreshed files",
        path.display(),
        diff_line.unwrap_or(0)
    );
    Ok(())
}

/// `unwrap_or_else` needs a value, not a `!`-typed panic — route through this
/// tiny helper so the `clippy::panic` deny (workspace-wide, tests included)
/// stays satisfied while the message stays actionable.
#[allow(clippy::panic)]
fn panic_wrapper(message: &str) -> ! {
    panic!("{message}")
}

#[test]
fn committed_outputs_match_regeneration() -> Result<(), Box<dyn std::error::Error>> {
    let set = parse(&committed_source()?)?;
    let mut css = Vec::new();
    let mut rust = Vec::new();
    emit_css(&set, &mut css)?;
    emit_rust(&set, &mut rust)?;
    let css = String::from_utf8(css)?;
    let rust = String::from_utf8(rust)?;

    assert_matches(&workspace_root().join("themes/generated/tokens.css"), &css)?;
    assert_matches(
        &workspace_root().join("crates/theme-tokens/src/generated/tokens.rs"),
        &rust,
    )?;
    // The UI-side bridge copy is a VERBATIM copy of the crate CSS output
    // (ui/scripts/gen-tokens.mjs) — stale copies after a crate-side
    // regeneration are caught here, closing the ui.yml gap (no tokens step)
    // from the Rust side of the fence.
    assert_matches(
        &workspace_root().join("ui/src/tokens/generated/tokens.css"),
        &css,
    )?;
    Ok(())
}

#[test]
fn the_comparison_detects_drift() -> Result<(), Box<dyn std::error::Error>> {
    // Sensitivity proof (the equality above must not be vacuous): mutate ONE
    // primitive value in the committed source (gray.700 #191919 — a unique
    // `$value`, unlike bare hex which also occurs in descriptions), re-emit,
    // and assert the committed output now differs from the mutated emission.
    let mutated =
        committed_source()?.replacen("\"$value\": \"#191919\"", "\"$value\": \"#1a1a1a\"", 1);
    assert_ne!(mutated, committed_source()?, "mutation must apply");
    let set = parse(&mutated)?;
    let mut css = Vec::new();
    emit_css(&set, &mut css)?;
    let css = String::from_utf8(css)?;

    let committed_css = fs::read_to_string(workspace_root().join("themes/generated/tokens.css"))?;
    assert_ne!(
        committed_css, css,
        "a mutated source must produce different output — the drift gate would be vacuous"
    );
    Ok(())
}
