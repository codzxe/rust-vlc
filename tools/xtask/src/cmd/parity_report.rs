//! `xtask parity-report` — the derived parity table for
//! docs/feature-matrix.toml (issue #20, card 8.F1).
//!
//! The matrix header is explicit: "Parity percentages are computed by the
//! tool, never stored." This is that tool. Parity (per the issue-#20 card)
//! is the share of rows whose platform status is `implemented` or
//! `verified` — computed per category and overall, per platform column,
//! straight from the parsed matrix. A strict `verified`-only line is also
//! emitted (the MASTER-PROMPT §1 wording: "verified by automated
//! evidence") so nobody can mistake shipped-but-unverified work for done.
//!
//! Nothing here is hardcoded: every number is derived from the matrix file
//! at run time (anti-fabrication rule, MP §15).
//!
//! Output: a markdown table on stdout plus a machine-readable summary
//! line. Exit 0 on success; 2 on xtask error; a matrix that does not parse
//! is exit 1 with a `parse:` line (same contract as feature-audit).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::XtaskError;
use crate::cmd::toml_lite::{self, Value};

const USAGE: &str = "\
Usage: cargo xtask parity-report [--root <dir>] [--matrix <relpath>]

Computes the parity table DERIVED from docs/feature-matrix.toml
(issue #20, card 8.F1; the matrix header: \"Parity percentages are
computed by the tool, never stored\"):

  - per category and overall, per platform column: rows whose status is
    `implemented` or `verified` over total rows (markdown table);
  - a strict `verified`-only summary (MP §1: \"verified by automated
    evidence\").

  --root <dir>      workspace root (default: walk up from the cwd)
  --matrix <rel>    matrix path relative to the root
                    (default: docs/feature-matrix.toml)

Exit code: 0 success | 1 matrix parse failure | 2 xtask error.";

const PLATFORMS: [&str; 4] = ["windows", "linux", "macos", "web"];

/// Per-category accumulators.
#[derive(Default)]
struct CategoryStats {
    rows: usize,
    /// implemented-or-verified row counts per platform.
    imp_ver: [usize; 4],
    /// verified-only row counts per platform.
    verified: [usize; 4],
}

pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut root_override: Option<PathBuf> = None;
    let mut matrix_rel = "docs/feature-matrix.toml".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(ExitCode::SUCCESS);
            }
            "--root" => {
                i += 1;
                let Some(dir) = args.get(i) else {
                    return Err(XtaskError::Usage(format!(
                        "--root requires a directory\n{USAGE}"
                    )));
                };
                root_override = Some(PathBuf::from(dir));
            }
            "--matrix" => {
                i += 1;
                let Some(p) = args.get(i) else {
                    return Err(XtaskError::Usage(format!(
                        "--matrix requires a path\n{USAGE}"
                    )));
                };
                matrix_rel = p.clone();
            }
            other => {
                return Err(XtaskError::Usage(format!(
                    "unexpected argument `{other}`\n{USAGE}"
                )));
            }
        }
        i += 1;
    }

    let root = match root_override {
        Some(dir) => dir,
        None => find_root()?,
    };
    let matrix_path = root.join(&matrix_rel);
    let text = std::fs::read_to_string(&matrix_path)
        .map_err(|e| XtaskError::io(format!("reading {}", matrix_path.display()), e))?;
    let doc = match toml_lite::parse(&text) {
        Ok(doc) => doc,
        Err(err) => {
            println!("parity: cannot parse {}: {err}", matrix_path.display());
            return Ok(ExitCode::from(1));
        }
    };

    let table = compute_table(&doc);
    print!("{table}");
    Ok(ExitCode::SUCCESS)
}

/// Compute the full markdown report from a parsed matrix. Exposed for the
/// fixture test (numbers derived from the fixture, asserted exactly).
pub fn compute_table(doc: &toml_lite::Document) -> String {
    let mut categories: BTreeMap<String, CategoryStats> = BTreeMap::new();
    for row in doc.array_rows("feature") {
        let category = row
            .entries
            .iter()
            .find(|(k, _)| k == "category")
            .and_then(|(_, v)| v.as_str())
            .unwrap_or("?")
            .to_string();
        let stats = categories.entry(category).or_default();
        stats.rows += 1;
        let Some(Value::Table(status)) = row
            .entries
            .iter()
            .find(|(k, _)| k == "status")
            .map(|(_, v)| v)
        else {
            continue;
        };
        for (idx, platform) in PLATFORMS.iter().enumerate() {
            let Some(value) = status
                .iter()
                .find(|(k, _)| k == platform)
                .and_then(|(_, v)| v.as_str())
            else {
                continue;
            };
            if value == "implemented" || value == "verified" {
                stats.imp_ver[idx] += 1;
            }
            if value == "verified" {
                stats.verified[idx] += 1;
            }
        }
    }

    let mut total = CategoryStats::default();
    for stats in categories.values() {
        total.rows += stats.rows;
        for i in 0..4 {
            total.imp_ver[i] += stats.imp_ver[i];
            total.verified[i] += stats.verified[i];
        }
    }

    let mut out = String::new();
    out.push_str("# Lumen parity report (derived from the feature matrix)\n\n");
    out.push_str("Share of rows with `implemented` or `verified` status, per platform\n");
    out.push_str("column. Computed by `cargo xtask parity-report`; never stored.\n\n");
    out.push_str("| Category | Rows | windows | linux | macos | web |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for (category, stats) in &categories {
        let mut line = format!("| {category} | {} ", stats.rows);
        for i in 0..4 {
            line.push_str(&format!("| {} ", cell(stats.imp_ver[i], stats.rows)));
        }
        line.push_str("|\n");
        out.push_str(&line);
    }
    let mut line = format!("| **TOTAL** | {} ", total.rows);
    for i in 0..4 {
        line.push_str(&format!("| {} ", cell(total.imp_ver[i], total.rows)));
    }
    line.push_str("|\n");
    out.push_str(&line);

    out.push_str("\nverified-only (MP §1 strict definition — automated evidence):\n\n");
    let mut line =
        String::from("| Scope | windows | linux | macos | web |\n|---|---|---|---|---|\n");
    for (label, stats) in categories
        .iter()
        .map(|(c, s)| (c.clone(), s))
        .chain(std::iter::once(("TOTAL".to_string(), &total)))
    {
        let mut row = format!("| {label} ");
        for i in 0..4 {
            row.push_str(&format!("| {} ", cell(stats.verified[i], stats.rows)));
        }
        row.push_str("|\n");
        line.push_str(&row);
    }
    out.push_str(&line);

    let mut machine = String::from("parity: rows=");
    machine.push_str(&total.rows.to_string());
    for (i, platform) in PLATFORMS.iter().enumerate() {
        machine.push_str(&format!(" {platform}={}/{}", total.imp_ver[i], total.rows));
    }
    out.push('\n');
    out.push_str(&machine);
    out.push('\n');
    out
}

/// `n/total (pp.p%)`; `n/a` when the denominator is zero.
fn cell(n: usize, total: usize) -> String {
    if total == 0 {
        return "n/a".to_string();
    }
    let share = (n as f64) * 100.0 / (total as f64);
    format!("{n}/{total} ({share:.1}%)")
}

fn find_root() -> Result<PathBuf, XtaskError> {
    let cwd =
        std::env::current_dir().map_err(|e| XtaskError::io("determining current directory", e))?;
    let mut dir: &std::path::Path = &cwd;
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("docs").is_dir() {
            return Ok(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => {
                return Err(XtaskError::internal(
                    "workspace root not found — run inside the Lumen workspace or pass --root",
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_formats_share() {
        assert_eq!(cell(1, 4), "1/4 (25.0%)");
        assert_eq!(cell(3, 3), "3/3 (100.0%)");
        assert_eq!(cell(0, 7), "0/7 (0.0%)");
        assert_eq!(cell(0, 0), "n/a");
    }

    #[test]
    fn table_derives_from_a_small_matrix() {
        let doc = toml_lite::parse(
            "[meta]\nschema_version = 1\n\
             [[feature]]\n\
             id = \"a-one\"\ncategory = \"alpha\"\npriority = \"P0\"\nphase_target = 5\n\
             status = { windows = \"implemented\", linux = \"verified\", macos = \"implemented\", web = \"not_started\" }\n\
             evidence = \"x\"\n\
             [[feature]]\n\
             id = \"a-two\"\ncategory = \"alpha\"\npriority = \"P1\"\nphase_target = 5\n\
             status = { windows = \"not_started\", linux = \"implemented\", macos = \"partial\", web = \"not_started\" }\n\
             evidence = \"\"\n\
             [[feature]]\n\
             id = \"b-one\"\ncategory = \"beta\"\npriority = \"P2\"\nphase_target = 6\n\
             status = { windows = \"verified\", linux = \"verified\", macos = \"verified\", web = \"verified\" }\n\
             evidence = \"y\"\n",
        )
        .expect("fixture parses");
        let table = compute_table(&doc);
        // alpha: windows 1/2, linux 2/2, macos 1/2, web 0/2.
        assert!(
            table.contains("| alpha | 2 | 1/2 (50.0%) | 2/2 (100.0%) | 1/2 (50.0%) | 0/2 (0.0%) |"),
            "{table}"
        );
        // beta: all platforms 1/1.
        assert!(
            table.contains(
                "| beta | 1 | 1/1 (100.0%) | 1/1 (100.0%) | 1/1 (100.0%) | 1/1 (100.0%) |"
            ),
            "{table}"
        );
        // totals: windows 2/3, linux 3/3, macos 2/3, web 1/3.
        assert!(
            table.contains(
                "| **TOTAL** | 3 | 2/3 (66.7%) | 3/3 (100.0%) | 2/3 (66.7%) | 1/3 (33.3%) |"
            ),
            "{table}"
        );
        // verified-only: windows 1/3, linux 2/3, macos 1/3, web 1/3.
        assert!(
            table.contains("| TOTAL | 1/3 (33.3%) | 2/3 (66.7%) | 1/3 (33.3%) | 1/3 (33.3%) |"),
            "{table}"
        );
        // machine line.
        assert!(
            table.contains("parity: rows=3 windows=2/3 linux=3/3 macos=2/3 web=1/3"),
            "{table}"
        );
    }
}
