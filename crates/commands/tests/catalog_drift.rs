//! Card 1.D4 — golden-file drift tests for the committed catalog
//! artifacts (the card's drift acceptance criterion, in test form).
//!
//! The three committed files are regenerated IN MEMORY through the same
//! `commands::catalog::codegen` API the `catalog-gen` bin calls (so the
//! bin, the tests and `cargo xtask codegen` can never disagree about
//! what "regenerated" means) and compared byte-for-byte. A mismatch
//! reports the artifact, the first differing line and both texts — fix
//! by running `cargo xtask codegen` and committing the refreshed files,
//! never by hand-editing the artifacts.

use std::error::Error;
use std::fs;
use std::path::Path;

use commands::catalog::codegen::default_paths;
use commands::catalog::codegen::generate_outputs;

/// The Lumen workspace root (this crate lives in `<root>/crates/commands`).
fn workspace_root() -> &'static Path {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().and_then(Path::parent).unwrap_or(manifest)
}

/// The first differing line of two texts, as `(line number, a, b)` —
/// `None` when the texts are identical (then `a`/`b` are placeholders).
fn first_line_diff(committed: &str, regenerated: &str) -> Option<(usize, String, String)> {
    let committed_lines: Vec<&str> = committed.lines().collect();
    let regenerated_lines: Vec<&str> = regenerated.lines().collect();
    let shared = committed_lines.len().min(regenerated_lines.len());
    for index in 0..shared {
        if committed_lines[index] != regenerated_lines[index] {
            return Some((
                index + 1,
                committed_lines[index].to_owned(),
                regenerated_lines[index].to_owned(),
            ));
        }
    }
    if committed_lines.len() != regenerated_lines.len() {
        let line = shared + 1;
        let a = committed_lines
            .get(shared)
            .map(|line: &&str| (*line).to_owned());
        let b = regenerated_lines
            .get(shared)
            .map(|line: &&str| (*line).to_owned());
        return Some((
            line,
            a.unwrap_or_else(|| "<end of file>".to_owned()),
            b.unwrap_or_else(|| "<end of file>".to_owned()),
        ));
    }
    None
}

/// The three committed artifacts must be byte-identical to what the
/// current catalog types regenerate — the CI drift gate in test form.
#[test]
fn committed_artifacts_match_regeneration() -> Result<(), Box<dyn Error>> {
    let root = workspace_root();
    let paths = default_paths(root);
    let outputs = generate_outputs()?;
    for (label, path, regenerated) in [
        ("schema", paths.schema, &outputs.schema),
        ("typescript", paths.typescript, &outputs.typescript),
        ("checksum", paths.checksum, &outputs.checksum),
    ] {
        let committed = fs::read_to_string(path).map_err(|err| {
            format!(
                "committed artifact {label} missing at {}: {err} — run `cargo xtask codegen` \
                 and commit it",
                path.display()
            )
        })?;
        if let Some((line, was, now)) = first_line_diff(&committed, regenerated) {
            return Err(format!(
                "{label} artifact {} drifted from the catalog: first difference at \
                 line {line}\n  committed:    {was}\n  regenerated:  {now}\nrun \
                 `cargo xtask codegen` and commit the refreshed file",
                path.display()
            )
            .into());
        }
    }
    Ok(())
}

/// Regeneration is deterministic: two independent runs agree byte for
/// byte (the precondition that makes the byte-for-byte comparison above
/// meaningful at all).
#[test]
fn regeneration_is_deterministic() -> Result<(), Box<dyn Error>> {
    let first = generate_outputs()?;
    let second = generate_outputs()?;
    assert_eq!(first, second, "two regenerations must agree");
    Ok(())
}
