//! `xtask web-budget` — the doc 09 §2.4 wasm size-budget gate (card 7.K1,
//! issue #49).
//!
//! # What this is
//!
//! The regression gate over `docs/web/size-record.toml` (the tree-shake
//! evidence of the 7.K1 size analysis): it re-measures the record's
//! `rebuild = true` rows with a REAL `cargo build` (never trusting a stale
//! number) and enforces doc 09 §2.4's two rules —
//!
//! * **budget breach**: a row's gzipped size over its budget class's
//!   configured ceiling fails; ceilings are READ FROM CONFIG
//!   (`product.toml [web.size_budgets]`, validated by
//!   `product.schema.json`) — no budget literal exists in this file, by
//!   design (raise a ceiling only via the doc 09 §2.4 review, never to
//!   make a gate green);
//! * **>5% growth**: a re-measured row whose gzip size grew more than
//!   1/20 over its recorded value fails with the "justify or optimize"
//!   message (doc 09 §2.4's regression rule; integer-exact comparison
//!   `measured*20 > recorded*21`).
//!
//! Rows with `rebuild = false` (the per-codec analysis rows) are checked
//! against their budgets from their RECORDED values — their environment
//! and method live in `docs/web/size-report.md`; the gate does not
//! rebuild them.
//!
//! # Modes (mirrors the sbom/fixtures drift conventions)
//!
//! * `cargo xtask web-budget` — re-measure and REWRITE the record's
//!   rebuilt rows byte-stably (only their `raw_bytes` / `gzip_bytes` /
//!   `sha256` lines change), then report the verdict: a breach still
//!   exits 1 — the record carries reality, the human justifies or
//!   optimizes it.
//! * `cargo xtask web-budget --check` — re-measure and COMPARE against
//!   the committed record: budget breach or >5% growth exits 1; a
//!   within-tolerance sha drift is a note, not a failure (refresh the
//!   record with the plain mode). Wired into `scripts/full_gate.sh`
//!   (step 10) and the CI `wasm-build` job.
//!
//! # Honesty posture
//!
//! gzip is measured by running the record's documented compressor
//! (`gzip -n -c`) as a subprocess — the gate never re-implements the
//! compressor it reports. sha256 IS implemented here (std-only, C5: no
//! external deps to approve) against the FIPS 180-4 vectors in the tests
//! below. The artifact is checked for the `\0asm` magic before measuring
//! (the CI canary's convention). Build failures are tool errors (exit
//! 2); budget/growth verdicts are gate failures (exit 1) — the stable
//! xtask exit-code contract.
//!
//! # Ownership
//!
//! Registered under card 7.K1's granted ownership of the xtask
//! subcommand surface (issue #49), same precedent as check-panics/7-e,
//! visual/4.G1, the 7.w21a gates and sbom/7.w23f. Reuses
//! `cmd::toml_lite` (the shared std-only TOML-subset reader) for both
//! `product.toml` and the size record.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use super::toml_lite::{self, Value};
use crate::XtaskError;

// ---------------------------------------------------------------------------
// Usage / constants
// ---------------------------------------------------------------------------

const USAGE: &str = "\
Usage: cargo xtask web-budget [--check] [--root <dir>] [--record <path>]

The doc 09 §2.4 wasm size-budget gate (card 7.K1, issue #49): re-measures
the rebuild = true rows of docs/web/size-record.toml with a real
`cargo build -p <package> --target wasm32-unknown-unknown --release
--locked` and enforces the ceilings from product.toml
[web.size_budgets] (NO budget literal lives in this gate):

  budget breach   a row's gzip -n size over its budget class ceiling fails
                  (classes: phase2-vertical -> phase2_vertical_max_mib,
                  full-p0 -> full_p0_max_mib)
  floor growth    a rebuilt row's gzip size >5% (1/20) over its recorded
                  value fails with the \"justify or optimize\" message

  (no flag)       re-measure + rewrite the record's rebuilt rows
                  byte-stably, then report (a breach exits 1 — the record
                  carries reality, the human justifies it)
  --check         re-measure + compare against the committed record
                  (full_gate step 10 / CI wasm-build job; exit 1 on
                  breach or growth)
  --root <dir>    workspace root (default: walk up from the cwd)
  --record <p>    size record (default <root>/docs/web/size-record.toml)

Exit code: 0 pass/no breach | 1 gate FAILURE | 2 xtask error.";

/// Where the committed size record lives, relative to the workspace root.
const RECORD_REL: &str = "docs/web/size-record.toml";

/// The record schema this gate understands (bump on a shape change).
const RECORD_SCHEMA: &str = "lumen.web.size-record/1";

/// The wasm target + profile the record documents (doc 09 §2.4: the
/// release-profile lane; the CI canary keeps the debug lane).
const WASM_TARGET: &str = "wasm32-unknown-unknown";
const PROFILE_DIR: &str = "release";

/// The growth tolerance as a fraction: doc 09 §2.4's ">5% on the baseline
/// fails" is evaluated integer-exactly as `measured * 20 > recorded * 21`
/// (1/20 = 5%), so no float ever decides a gate verdict.
const GROWTH_DEN: u128 = 20;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Entry point for `cargo xtask web-budget`.
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut check_only = false;
    let mut root_override: Option<PathBuf> = None;
    let mut record_override: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(ExitCode::SUCCESS);
            }
            "--check" => check_only = true,
            "--root" | "--record" => {
                let flag = args[i].as_str();
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err(XtaskError::Usage(format!(
                        "{flag} requires a path\n{USAGE}"
                    )));
                };
                let slot = if flag == "--root" {
                    &mut root_override
                } else {
                    &mut record_override
                };
                *slot = Some(PathBuf::from(value));
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
    let record_path = record_override.unwrap_or_else(|| root.join(RECORD_REL));
    let product_path = root.join("product.toml");

    let product_text = std::fs::read_to_string(&product_path)
        .map_err(|e| XtaskError::io(format!("reading {}", product_path.display()), e))?;
    let budgets = Budgets::from_product_toml(&product_text).map_err(XtaskError::internal)?;
    let record_text = std::fs::read_to_string(&record_path).map_err(|e| {
        XtaskError::io(
            format!(
                "reading {} — the size record is a committed artifact; \
                 restore it or point --record at it",
                record_path.display()
            ),
            e,
        )
    })?;
    let record = parse_record(&record_text).map_err(XtaskError::internal)?;

    println!("=== LUMEN WEB-BUDGET GATE (doc 09 §2.4, card 7.K1) ===");
    println!(
        "budgets: {} (product.toml [web.size_budgets])",
        budgets.describe()
    );
    println!("record:  {}", record_path.display());

    // Re-measure every rebuild row with a real build (the gate never
    // trusts a stale recorded number for its own rows).
    let mut measured: BTreeMap<String, Measured> = BTreeMap::new();
    for row in record.rows.iter().filter(|row| row.rebuild) {
        let (m, secs) = rebuild_and_measure(&root, row)?;
        println!(
            "rebuild: `{}` — raw {} B, gzip -n {} B, sha256 {}… ({:.1}s)",
            row.name,
            m.raw_bytes,
            m.gzip_bytes,
            m.sha256.get(..12).unwrap_or(""),
            secs
        );
        measured.insert(row.name.clone(), m);
    }

    let (verdicts, failures, notes) =
        evaluate(&budgets, &record.rows, &measured).map_err(XtaskError::internal)?;

    println!();
    println!(
        "{:<18} {:<16} {:>12} {:>12} {:>7}  source",
        "row", "class", "gzip B", "ceiling B", "pct"
    );
    for v in &verdicts {
        let pct = if v.ceiling_bytes == 0 {
            "n/a".to_string()
        } else {
            format!(
                "{:.1}%",
                v.gzip_bytes as f64 / v.ceiling_bytes as f64 * 100.0
            )
        };
        println!(
            "{:<18} {:<16} {:>12} {:>12} {:>7}  {}",
            v.name, v.budget_class, v.gzip_bytes, v.ceiling_bytes, pct, v.source
        );
    }
    for note in &notes {
        println!("{note}");
    }

    if check_only {
        if failures.is_empty() {
            println!();
            println!(
                "result: PASS — no budget breach, no >5% growth on rebuilt rows ({} rows checked)",
                verdicts.len()
            );
            Ok(ExitCode::SUCCESS)
        } else {
            println!();
            for failure in &failures {
                println!("FAIL {failure}");
            }
            println!();
            println!(
                "result: FAILURE — {} finding(s) — justify or optimize per doc 09 §2.4 \
                 (ceilings: product.toml [web.size_budgets]; never raise one to make this green)",
                failures.len()
            );
            Ok(ExitCode::from(1))
        }
    } else {
        // Maintain mode: write reality FIRST (the record must be able to
        // carry a breach for the human to see), then give the verdict.
        let rewritten =
            rewrite_measured_rows(&record_text, &measured).map_err(XtaskError::internal)?;
        std::fs::write(&record_path, &rewritten)
            .map_err(|e| XtaskError::io(format!("writing {}", record_path.display()), e))?;
        println!();
        println!(
            "wrote: {} — rebuilt rows re-measured; commit the refreshed record",
            record_path.display()
        );
        if failures.is_empty() {
            println!("result: PASS — no budget breach, no >5% growth on rebuilt rows");
            Ok(ExitCode::SUCCESS)
        } else {
            for failure in &failures {
                println!("FAIL {failure}");
            }
            println!();
            println!(
                "result: FAILURE — the record now carries the breach honestly; justify or \
                 optimize per doc 09 §2.4 before merging"
            );
            Ok(ExitCode::from(1))
        }
    }
}

// ---------------------------------------------------------------------------
// Root discovery (config_validator's pattern)
// ---------------------------------------------------------------------------

fn find_root() -> Result<PathBuf, XtaskError> {
    let cwd =
        std::env::current_dir().map_err(|e| XtaskError::io("determining current directory", e))?;
    let mut dir: &Path = &cwd;
    loop {
        if dir.join("product.toml").is_file() {
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

// ---------------------------------------------------------------------------
// Budgets (product.toml [web.size_budgets] via toml_lite)
// ---------------------------------------------------------------------------

/// The configured ceilings (doc 09 §2.4). Values come from
/// `product.toml [web.size_budgets]` — NEVER from this file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Budgets {
    phase2_vertical_max_mib: u64,
    full_p0_max_mib: u64,
}

impl Budgets {
    /// Parse from `product.toml` text. A document without the section is
    /// a loud error, not a default: budgets are a product decision with
    /// no code-side fallback that would silently bypass the review that
    /// owns the number (model.rs; doc 09 §2.4).
    fn from_product_toml(text: &str) -> Result<Self, String> {
        let doc = toml_lite::parse(text).map_err(|e| format!("product.toml: {e}"))?;
        let int_at = |key: &str| -> Result<u64, String> {
            match doc.get(key) {
                Some(Value::Int(v)) if *v > 0 => Ok(*v as u64),
                Some(Value::Int(v)) => Err(format!(
                    "product.toml: {key} must be a positive integer (got {v} MiB)"
                )),
                Some(other) => Err(format!(
                    "product.toml: {key} must be an integer (MiB), got {}",
                    other.type_name()
                )),
                None => Err(format!(
                    "product.toml: missing {key} — the committed product.toml must carry \
                     [web.size_budgets] (doc 09 §2.4; product.schema.json); the web-budget \
                     gate has no code-side default by design"
                )),
            }
        };
        let budgets = Budgets {
            phase2_vertical_max_mib: int_at("web.size_budgets.phase2_vertical_max_mib")?,
            full_p0_max_mib: int_at("web.size_budgets.full_p0_max_mib")?,
        };
        budgets.validate()?;
        Ok(budgets)
    }

    /// The cross-field rule the schema documents and the gate enforces:
    /// every full-P0 build is a Phase-2 superset, so its ceiling cannot
    /// sit below the Phase-2 vertical ceiling.
    fn validate(&self) -> Result<(), String> {
        if self.full_p0_max_mib < self.phase2_vertical_max_mib {
            return Err(format!(
                "product.toml [web.size_budgets]: full_p0_max_mib ({}) must be >= \
                 phase2_vertical_max_mib ({}) — the full P0 codec set is a superset of the \
                 Phase-2 vertical (doc 09 §2.4)",
                self.full_p0_max_mib, self.phase2_vertical_max_mib
            ));
        }
        Ok(())
    }

    /// Map a record row's `budget_class` to its configured byte ceiling
    /// (MiB is binary: 1 MiB = 1 << 20 bytes).
    fn ceiling_bytes(&self, budget_class: &str) -> Result<u64, String> {
        let mib = match budget_class {
            "phase2-vertical" => self.phase2_vertical_max_mib,
            "full-p0" => self.full_p0_max_mib,
            other => {
                return Err(format!(
                    "size record: unknown budget_class `{other}` — expected \
                     `phase2-vertical` or `full-p0` (the two classes product.toml \
                     [web.size_budgets] configures)"
                ));
            }
        };
        Ok(mib.saturating_mul(1024u64 * 1024))
    }

    /// One-line description for the report header.
    fn describe(&self) -> String {
        format!(
            "phase2-vertical <= {} MiB ({} B), full-p0 <= {} MiB ({} B)",
            self.phase2_vertical_max_mib,
            self.phase2_vertical_max_mib * 1024 * 1024,
            self.full_p0_max_mib,
            self.full_p0_max_mib * 1024 * 1024
        )
    }
}

// ---------------------------------------------------------------------------
// The size record (docs/web/size-record.toml via toml_lite)
// ---------------------------------------------------------------------------

/// One `[[config]]` row of the size record.
#[derive(Debug, Clone, PartialEq)]
struct SizeRow {
    name: String,
    budget_class: String,
    rebuild: bool,
    /// Required on rebuild rows: the cargo package to build.
    package: Option<String>,
    /// Required on rebuild rows: the `--features` list ([] = the floor).
    cargo_features: Vec<String>,
    raw_bytes: u64,
    gzip_bytes: u64,
    sha256: String,
}

/// The parsed record.
#[derive(Debug)]
struct SizeRecord {
    rows: Vec<SizeRow>,
}

/// Parse the size record text via the shared TOML-subset reader.
fn parse_record(text: &str) -> Result<SizeRecord, String> {
    let doc = toml_lite::parse(text).map_err(|e| format!("size record: {e}"))?;
    match doc.get("schema") {
        Some(Value::Str(schema)) if schema == RECORD_SCHEMA => {}
        Some(other) => {
            return Err(format!(
                "size record: `schema` must be \"{RECORD_SCHEMA}\", got {}",
                other.type_name()
            ));
        }
        None => return Err("size record: missing `schema`".to_string()),
    }
    let mut rows = Vec::new();
    for row in doc.array_rows("config") {
        let mut name = None;
        let mut budget_class = None;
        let mut rebuild = false;
        let mut package = None;
        let mut cargo_features: Vec<String> = Vec::new();
        let mut raw_bytes = None;
        let mut gzip_bytes = None;
        let mut sha256 = None;
        for (key, value) in &row.entries {
            match key.as_str() {
                "name" | "budget_class" | "sha256" => {
                    let text = value.as_str().ok_or_else(|| {
                        format!(
                            "size record: `{key}` must be a string, got {}",
                            value.type_name()
                        )
                    })?;
                    match key.as_str() {
                        "name" => name = Some(text.to_string()),
                        "budget_class" => budget_class = Some(text.to_string()),
                        _ => sha256 = Some(text.to_string()),
                    }
                }
                "features" | "measured_by" => {
                    // Human documentation; the gate does not consume it,
                    // but it must still be a string (strictness).
                    if value.as_str().is_none() {
                        return Err(format!(
                            "size record: `{key}` must be a string, got {}",
                            value.type_name()
                        ));
                    }
                }
                "rebuild" => {
                    rebuild = match value {
                        Value::Bool(flag) => *flag,
                        other => {
                            return Err(format!(
                                "size record: `rebuild` must be a boolean, got {}",
                                other.type_name()
                            ));
                        }
                    };
                }
                "package" => {
                    package = value.as_str().map(str::to_string);
                }
                "cargo_features" => {
                    let items = match value {
                        Value::Array(items) => items,
                        other => {
                            return Err(format!(
                                "size record: `cargo_features` must be an array of strings, got {}",
                                other.type_name()
                            ));
                        }
                    };
                    for item in items {
                        match item.as_str() {
                            Some(feature) => cargo_features.push(feature.to_string()),
                            None => {
                                return Err(
                                    "size record: `cargo_features` must contain only strings"
                                        .to_string(),
                                );
                            }
                        }
                    }
                }
                "raw_bytes" | "gzip_bytes" => {
                    let n = value.as_int().ok_or_else(|| {
                        format!(
                            "size record: `{key}` must be an integer, got {}",
                            value.type_name()
                        )
                    })?;
                    if n < 0 {
                        return Err(format!("size record: `{key}` must be >= 0 (got {n})"));
                    }
                    match key.as_str() {
                        "raw_bytes" => raw_bytes = Some(n as u64),
                        _ => gzip_bytes = Some(n as u64),
                    }
                }
                other => {
                    return Err(format!(
                        "size record: unknown [[config]] key `{other}` — extend the \
                         web-budget reader (or fix the record)"
                    ));
                }
            }
        }
        let name = name.ok_or("size record: a [[config]] row is missing `name`")?;
        let budget_class = budget_class
            .ok_or_else(|| format!("size record: row `{name}` is missing `budget_class`"))?;
        let raw_bytes =
            raw_bytes.ok_or_else(|| format!("size record: row `{name}` is missing `raw_bytes`"))?;
        let gzip_bytes = gzip_bytes
            .ok_or_else(|| format!("size record: row `{name}` is missing `gzip_bytes`"))?;
        let sha256 =
            sha256.ok_or_else(|| format!("size record: row `{name}` is missing `sha256`"))?;
        if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!(
                "size record: row `{name}`: `sha256` must be 64 hex characters"
            ));
        }
        if rebuild && package.is_none() {
            return Err(format!(
                "size record: row `{name}` has rebuild = true but no `package` — the \
                 rebuild contract (record header) requires package + cargo_features"
            ));
        }
        rows.push(SizeRow {
            name,
            budget_class,
            rebuild,
            package,
            cargo_features,
            raw_bytes,
            gzip_bytes,
            sha256,
        });
    }
    if rows.is_empty() {
        return Err("size record: no [[config]] rows found".to_string());
    }
    if rows.iter().all(|row| !row.rebuild) {
        return Err(
            "size record: no `rebuild = true` row — the gate re-measures its own rows; a \
             record without one cannot be checked"
                .to_string(),
        );
    }
    for (i, row) in rows.iter().enumerate() {
        if rows[i + 1..].iter().any(|other| other.name == row.name) {
            return Err(format!("size record: duplicate row name `{}`", row.name));
        }
    }
    Ok(SizeRecord { rows })
}

// ---------------------------------------------------------------------------
// Rebuild + measure
// ---------------------------------------------------------------------------

/// A fresh measurement of one rebuild row.
#[derive(Debug, Clone, PartialEq)]
struct Measured {
    raw_bytes: u64,
    gzip_bytes: u64,
    sha256: String,
}

/// Rebuild one row's configuration with a real `cargo build` and measure
/// the produced module (raw size, `gzip -n -c` size, sha256). Returns the
/// measurement and the build's wall-clock seconds (reporting only).
fn rebuild_and_measure(root: &Path, row: &SizeRow) -> Result<(Measured, f64), XtaskError> {
    let package = match row.package.as_deref() {
        Some(package) => package,
        None => {
            return Err(XtaskError::internal(format!(
                "row `{}`: rebuild = true without a package (parse should have caught this)",
                row.name
            )));
        }
    };
    let mut cmd = Command::new("cargo");
    cmd.current_dir(root).args([
        "build",
        "-p",
        package,
        "--target",
        WASM_TARGET,
        "--release",
        "--locked",
    ]);
    if !row.cargo_features.is_empty() {
        cmd.arg("--features").arg(row.cargo_features.join(","));
    }
    let started = Instant::now();
    let output = cmd
        .output()
        .map_err(|e| XtaskError::io("spawning `cargo build`", e))?;
    let secs = started.elapsed().as_secs_f64();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let lines: Vec<&str> = stderr.lines().collect();
        let start = lines.len().saturating_sub(15);
        return Err(XtaskError::internal(format!(
            "cargo build for row `{}` failed ({}):\n{}",
            row.name,
            output.status,
            lines[start..].join("\n")
        )));
    }
    let artifact = wasm_artifact_path(root, package)?;
    let bytes = std::fs::read(&artifact)
        .map_err(|e| XtaskError::io(format!("reading {}", artifact.display()), e))?;
    // A real wasm module starts with the 4-byte magic \0asm (the CI
    // canary's convention — measure nothing else).
    let magic_ok = bytes.len() >= 4
        && bytes[0] == 0x00
        && bytes[1] == b'a'
        && bytes[2] == b's'
        && bytes[3] == b'm';
    if !magic_ok {
        return Err(XtaskError::internal(format!(
            "{} is not a wasm module (missing the \\0asm magic) — the build produced \
             something the gate refuses to measure",
            artifact.display()
        )));
    }
    let gzip_bytes = gzip_size(&artifact)?;
    Ok((
        Measured {
            raw_bytes: bytes.len() as u64,
            gzip_bytes,
            sha256: sha256_hex(&bytes),
        },
        secs,
    ))
}

/// Where cargo puts the row's cdylib module: `<target
/// dir>/<target>/<profile>/<package with _>.wasm`. `CARGO_TARGET_DIR`
/// wins (the parallel-worktree build environment exports it); otherwise
/// the workspace's repo-relative `target/`.
fn wasm_artifact_path(root: &Path, package: &str) -> Result<PathBuf, XtaskError> {
    let target_dir = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(dir) => {
            let path = PathBuf::from(dir);
            if path.is_absolute() {
                path
            } else {
                root.join(path)
            }
        }
        None => root.join("target"),
    };
    let file = format!("{}.wasm", package.replace('-', "_"));
    Ok(target_dir.join(WASM_TARGET).join(PROFILE_DIR).join(file))
}

/// Measure `gzip -n -c` size with the record's documented compressor.
fn gzip_size(path: &Path) -> Result<u64, XtaskError> {
    let output = Command::new("gzip")
        .args(["-n", "-c"])
        .arg(path)
        .output()
        .map_err(|e| {
            XtaskError::io(
                "spawning `gzip -n -c` (the record's documented compressor — install \
                 gzip or fix PATH)",
                e,
            )
        })?;
    if !output.status.success() {
        return Err(XtaskError::internal(format!(
            "gzip -n -c failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(output.stdout.len() as u64)
}

// ---------------------------------------------------------------------------
// The verdict (pure — unit-tested with synthetic inputs)
// ---------------------------------------------------------------------------

/// One row's checkable outcome, for the report table.
#[derive(Debug, Clone, PartialEq)]
struct RowVerdict {
    name: String,
    budget_class: String,
    gzip_bytes: u64,
    ceiling_bytes: u64,
    /// "measured" for rebuilt rows, "recorded" for analysis rows.
    source: &'static str,
}

/// The outcome of [`evaluate`]: the per-row verdicts for the report
/// table, the failures (empty = pass) and the non-fatal notes.
type Evaluation = (Vec<RowVerdict>, Vec<String>, Vec<String>);

/// Pure evaluation of doc 09 §2.4's two rules. `measured` carries the
/// fresh measurements for the record's rebuild rows (a row present here
/// is checked from its MEASURED gzip; every other row from its RECORDED
/// gzip — the record's own contract). Returns the per-row verdicts, the
/// failures (empty = pass) and the non-fatal notes.
fn evaluate(
    budgets: &Budgets,
    rows: &[SizeRow],
    measured: &BTreeMap<String, Measured>,
) -> Result<Evaluation, String> {
    budgets.validate()?;
    let mut verdicts = Vec::new();
    let mut failures = Vec::new();
    let mut notes = Vec::new();
    for row in rows {
        let ceiling = budgets.ceiling_bytes(&row.budget_class)?;
        let (gzip_bytes, source): (u64, &'static str) = match measured.get(&row.name) {
            Some(m) => (m.gzip_bytes, "measured"),
            None => (row.gzip_bytes, "recorded"),
        };
        verdicts.push(RowVerdict {
            name: row.name.clone(),
            budget_class: row.budget_class.clone(),
            gzip_bytes,
            ceiling_bytes: ceiling,
            source,
        });
        if gzip_bytes > ceiling {
            failures.push(format!(
                "budget breach: row `{}` (class `{}`, {}) gzip {} B > ceiling {} B — \
                 justify or optimize (doc 09 §2.4; ceiling from product.toml \
                 [web.size_budgets])",
                row.name, row.budget_class, source, gzip_bytes, ceiling
            ));
        }
        if let Some(m) = measured.get(&row.name) {
            // Integer-exact >5%: measured exceeds recorded + recorded/20.
            let grown = u128::from(m.gzip_bytes) * GROWTH_DEN
                > u128::from(row.gzip_bytes) * (GROWTH_DEN + 1);
            if grown {
                failures.push(format!(
                    "growth: row `{}` re-measured gzip {} B is >5% over the recorded {} B \
                     ({} — the committed baseline) — justify or optimize per doc 09 §2.4, \
                     then refresh the record with `cargo xtask web-budget`",
                    row.name, m.gzip_bytes, row.gzip_bytes, RECORD_REL
                ));
            } else if m.sha256 != row.sha256 {
                notes.push(format!(
                    "note: row `{}` sha256 {}… differs from the record {}… (size within \
                     tolerance) — the record predates this build; refresh it with \
                     `cargo xtask web-budget`",
                    row.name,
                    m.sha256.get(..12).unwrap_or(""),
                    row.sha256.get(..12).unwrap_or("")
                ));
            }
        }
    }
    Ok((verdicts, failures, notes))
}

// ---------------------------------------------------------------------------
// Byte-stable rewrite of the rebuilt rows
// ---------------------------------------------------------------------------

/// Rewrite the `raw_bytes` / `gzip_bytes` / `sha256` lines of the named
/// `[[config]]` blocks in place, preserving every other byte of the file
/// (comments, ordering, blank lines). Only bare `key = <value>` lines are
/// replaced — an inline comment on one is a loud error, not a silent
/// clobber. Preserves the original line's indentation.
fn rewrite_measured_rows(
    text: &str,
    updates: &BTreeMap<String, Measured>,
) -> Result<String, String> {
    let mut seen: BTreeMap<&str, bool> =
        updates.keys().map(|name| (name.as_str(), false)).collect();
    let mut out = String::with_capacity(text.len() + 128);
    let mut in_config_block = false;
    // (block name, measurement, raw done, gzip done, sha done)
    let mut target: Option<(String, &Measured, bool, bool, bool)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let mut new_line: Option<String> = None;
        if trimmed.starts_with("[[") {
            finish_block(&target)?;
            in_config_block = trimmed.starts_with("[[config]]");
            target = None;
        } else if trimmed.starts_with('[') {
            finish_block(&target)?;
            in_config_block = false;
            target = None;
        } else if in_config_block {
            if let Some(name) = parse_name_value(trimmed) {
                if let Some(m) = updates.get(name) {
                    seen.insert(name, true);
                    target = Some((name.to_string(), m, false, false, false));
                } else {
                    target = None;
                }
            } else if let Some((_, m, raw_done, gzip_done, sha_done)) = &mut target {
                if !*raw_done {
                    new_line = replace_int_value(line, "raw_bytes", m.raw_bytes);
                    *raw_done = new_line.is_some();
                }
                if new_line.is_none() && !*gzip_done {
                    new_line = replace_int_value(line, "gzip_bytes", m.gzip_bytes);
                    *gzip_done = new_line.is_some();
                }
                if new_line.is_none()
                    && !*sha_done
                    && let Some(r) = replace_str_value(line, "sha256", &m.sha256)
                {
                    new_line = Some(r);
                    *sha_done = true;
                }
            }
        }
        match new_line {
            Some(replacement) => {
                out.push_str(&replacement);
                out.push('\n');
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    finish_block(&target)?;
    for (name, found) in &seen {
        if !found {
            return Err(format!(
                "size record: row `{name}` was measured but has no [[config]] block — \
                 the record and the measurement disagree"
            ));
        }
    }
    Ok(out)
}

/// A target block that ends (new header or EOF) must have had all three of
/// its measured lines rewritten.
fn finish_block(target: &Option<(String, &Measured, bool, bool, bool)>) -> Result<(), String> {
    if let Some((name, _, raw, gzip, sha)) = target
        && !(*raw && *gzip && *sha)
    {
        return Err(format!(
            "size record: block `{name}` ended before raw_bytes/gzip_bytes/sha256 \
             were all rewritten (raw={raw}, gzip={gzip}, sha={sha}) — the measured \
             lines must be bare `key = value` lines"
        ));
    }
    Ok(())
}

/// Extract the string value of a `name = "…"` line, if it is one.
fn parse_name_value(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("name")?.trim_start();
    let value = rest.strip_prefix('=')?.trim();
    let inner = value.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner)
}

/// Replace a bare `key = <integer>` line (any indentation), preserving the
/// indentation. Returns `None` when the line is not that key.
fn replace_int_value(line: &str, key: &str, new_value: u64) -> Option<String> {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    let rest = trimmed.strip_prefix(key)?.trim_start();
    let value = rest.strip_prefix('=')?.trim();
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(format!("{indent}{key} = {new_value}"))
}

/// Replace a bare `key = "<string>"` line (any indentation), preserving
/// the indentation. Returns `None` when the line is not that key.
fn replace_str_value(line: &str, key: &str, new_value: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    let rest = trimmed.strip_prefix(key)?.trim_start();
    let value = rest.strip_prefix('=')?.trim();
    let inner = value.strip_prefix('"')?.strip_suffix('"')?;
    if inner.bytes().any(|b| b == b'"') {
        return None;
    }
    Some(format!("{indent}{key} = \"{new_value}\""))
}

// ---------------------------------------------------------------------------
// sha256 (std-only, FIPS 180-4)
// ---------------------------------------------------------------------------

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256 digest as lowercase hex (FIPS 180-4). Std-only by design (C5):
/// the xtask takes no dependency for one hash, and the implementation is
/// pinned by the standard test vectors below.
fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let (chunks, remainder) = data.as_chunks::<64>();
    for block in chunks {
        compress(&mut h, block);
    }
    // Padding: 0x80, zeros, then the 8-byte big-endian bit length — in one
    // or two final 64-byte blocks depending on the remainder.
    let mut tail = [0u8; 128];
    let rem_len = remainder.len();
    tail[..rem_len].copy_from_slice(remainder);
    tail[rem_len] = 0x80;
    let tail_total = if rem_len + 9 <= 64 { 64 } else { 128 };
    let len_at = tail_total - 8;
    tail[len_at..tail_total].copy_from_slice(&bit_len.to_be_bytes());
    let (tail_chunks, _) = tail[..tail_total].as_chunks::<64>();
    for block in tail_chunks {
        compress(&mut h, block);
    }
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = vec![0u8; 64];
    for (i, word) in h.iter().enumerate() {
        for j in 0..8 {
            let nibble = (word >> (28 - j * 4)) & 0xf;
            out[i * 8 + j] = HEX[nibble as usize];
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

/// The SHA-256 compression function over one 64-byte block.
fn compress(h: &mut [u32; 8], block: &[u8]) {
    let mut w = [0u32; 64];
    for (i, word) in w.iter_mut().take(16).enumerate() {
        let base = i * 4;
        let mut quad = [0u8; 4];
        for (dst, src) in quad
            .iter_mut()
            .zip(block.get(base..base + 4).unwrap_or(&[]))
        {
            *dst = *src;
        }
        *word = u32::from_be_bytes(quad);
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = hh
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(SHA256_K[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        hh = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    h[0] = h[0].wrapping_add(a);
    h[1] = h[1].wrapping_add(b);
    h[2] = h[2].wrapping_add(c);
    h[3] = h[3].wrapping_add(d);
    h[4] = h[4].wrapping_add(e);
    h[5] = h[5].wrapping_add(f);
    h[6] = h[6].wrapping_add(g);
    h[7] = h[7].wrapping_add(hh);
}

// ---------------------------------------------------------------------------
// Tests (synthetic breaches; the live pass runs via the entry point)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn budgets() -> Budgets {
        Budgets {
            phase2_vertical_max_mib: 3,
            full_p0_max_mib: 6,
        }
    }

    fn row(name: &str, class: &str, gzip: u64) -> SizeRow {
        SizeRow {
            name: name.to_string(),
            budget_class: class.to_string(),
            rebuild: false,
            package: None,
            cargo_features: Vec::new(),
            raw_bytes: gzip + 900_000,
            gzip_bytes: gzip,
            sha256: "a".repeat(64),
        }
    }

    // -- sha256: FIPS 180-4 vectors (cross-checked with coreutils
    //    sha256sum in this sandbox) --------------------------------------

    #[test]
    fn sha256_empty_string() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_abc() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_448_bits() {
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn sha256_multi_block_and_two_block_tail() {
        // 100 bytes of 'x': exercises multiple 64-byte blocks AND the
        // 128-byte tail path (remainder 36 + 9 > 64).
        assert_eq!(
            sha256_hex(&[b'x'; 100]),
            "09ecb6ebc8bcefc733f6f2ec44f791abeed6a99edf0cc31519637898aebd52d8"
        );
        // The classic million-'a' vector: the full multi-block path.
        assert_eq!(
            sha256_hex(&vec![b'a'; 1_000_000]),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    // -- budgets -----------------------------------------------------------

    #[test]
    fn budgets_parse_from_product_toml_shape() {
        let product = "[web.size_budgets]\nphase2_vertical_max_mib = 3\nfull_p0_max_mib = 6\n";
        let parsed = Budgets::from_product_toml(product);
        assert_eq!(parsed, Ok(budgets()));
    }

    #[test]
    fn budgets_missing_section_is_a_loud_error() {
        let err = Budgets::from_product_toml("[identity]\nname = \"x\"\n");
        assert!(
            err.is_err(),
            "no code-side default may silently bypass the config: {err:?}"
        );
    }

    #[test]
    fn budgets_full_p0_below_phase2_is_rejected() {
        let err = Budgets {
            phase2_vertical_max_mib: 6,
            full_p0_max_mib: 3,
        }
        .validate();
        assert!(err.is_err(), "superset ceiling cannot sit below");
    }

    #[test]
    fn ceiling_bytes_maps_classes_and_rejects_unknowns() {
        let b = budgets();
        assert_eq!(b.ceiling_bytes("phase2-vertical"), Ok(3 * 1024 * 1024));
        assert_eq!(b.ceiling_bytes("full-p0"), Ok(6 * 1024 * 1024));
        assert!(b.ceiling_bytes("phase-3").is_err());
    }

    // -- evaluate: the synthetic-breach proof -------------------------------

    #[test]
    fn evaluate_passes_the_committed_shape() {
        let rows = vec![
            row("floor", "phase2-vertical", 300_250),
            row("external-all", "full-p0", 704_996),
        ];
        let mut measured = BTreeMap::new();
        measured.insert(
            "floor".to_string(),
            Measured {
                raw_bytes: 1_294_311,
                gzip_bytes: 300_250,
                sha256: "a".repeat(64),
            },
        );
        let (verdicts, failures, notes) =
            evaluate(&budgets(), &rows, &measured).unwrap_or_default();
        assert!(failures.is_empty(), "no breach, no growth: {failures:?}");
        assert!(notes.is_empty());
        assert_eq!(verdicts.len(), 2);
        assert_eq!(verdicts[0].source, "measured");
        assert_eq!(verdicts[1].source, "recorded");
    }

    #[test]
    fn evaluate_fails_a_synthetic_budget_breach() {
        // 4 MiB gzipped against a 3 MiB phase2-vertical ceiling — the
        // synthetic breach: the gate MUST name the row, the class and the
        // configured ceiling, and MUST fail.
        let rows = vec![row("synthetic-fat", "phase2-vertical", 4 * 1024 * 1024)];
        let (_, failures, _) = evaluate(&budgets(), &rows, &BTreeMap::new()).unwrap_or_default();
        assert_eq!(failures.len(), 1, "exactly the breach: {failures:?}");
        assert!(
            failures[0].contains("budget breach") && failures[0].contains("synthetic-fat"),
            "the failure must name the breach and the row: {failures:?}"
        );
        assert!(
            failures[0].contains(&format!("{}", 3 * 1024 * 1024)),
            "the failure must name the configured ceiling: {failures:?}"
        );
    }

    #[test]
    fn evaluate_fails_synthetic_growth_just_over_five_percent() {
        let rows = vec![row("floor", "phase2-vertical", 300_000)];
        let mut measured = BTreeMap::new();
        measured.insert(
            "floor".to_string(),
            Measured {
                raw_bytes: 1_300_000,
                // 300_000 + 300_000/20 + 1 = 315_001: the first integer
                // size that is >5% over the recorded baseline.
                gzip_bytes: 300_000 + 300_000 / 20 + 1,
                sha256: "a".repeat(64),
            },
        );
        let (_, failures, _) = evaluate(&budgets(), &rows, &measured).unwrap_or_default();
        assert_eq!(failures.len(), 1, "just over 5% must fail: {failures:?}");
        assert!(failures[0].contains("growth"));
    }

    #[test]
    fn evaluate_passes_growth_exactly_five_percent_and_notes_sha_drift() {
        let rows = vec![row("floor", "phase2-vertical", 300_000)];
        let mut measured = BTreeMap::new();
        measured.insert(
            "floor".to_string(),
            Measured {
                raw_bytes: 1_300_000,
                // Exactly +5% (315_000 = 300_000 * 21/20): passes; the
                // rule is >5%, not >=5%.
                gzip_bytes: 300_000 + 300_000 / 20,
                sha256: "b".repeat(64),
            },
        );
        let (_, failures, notes) = evaluate(&budgets(), &rows, &measured).unwrap_or_default();
        assert!(failures.is_empty(), "exactly 5% is not >5%: {failures:?}");
        assert_eq!(notes.len(), 1, "the sha drift is a note, not a failure");
        assert!(notes[0].contains("sha256"));
    }

    // -- record parsing ------------------------------------------------------

    #[test]
    fn parse_record_reads_the_committed_shape() {
        let text = "\
schema = \"lumen.web.size-record/1\"

[environment]
toolchain = \"rustc test\"

[[config]]
name = \"floor\"
budget_class = \"phase2-vertical\"
rebuild = true
package = \"lumen-web\"
cargo_features = []
measured_by = \"cargo xtask web-budget\"
raw_bytes = 100
gzip_bytes = 50
sha256 = \"4a0f33831d6411dfef20b13925bb82a2f658d79c843dd1ec1ebfab78ca478d87\"

[[config]]
name = \"flac\"
budget_class = \"full-p0\"
rebuild = false
raw_bytes = 200
gzip_bytes = 60
sha256 = \"4d4cfa10f80e80133b1a13377597cb995d1a4cde613f7488a5fab5aa4bf9a13c\"
";
        let record = match parse_record(text) {
            Ok(record) => record,
            Err(err) => unreachable!("the committed shape must parse: {err}"),
        };
        assert_eq!(record.rows.len(), 2);
        assert_eq!(record.rows[0].package.as_deref(), Some("lumen-web"));
        assert!(record.rows[0].cargo_features.is_empty());
        assert_eq!(record.rows[1].budget_class, "full-p0");
    }

    #[test]
    fn parse_record_rejects_wrong_schema_and_rebuild_without_package() {
        let bad_schema = "schema = \"lumen.web.size-record/2\"\n\n[[config]]\nname = \"x\"\nbudget_class = \"full-p0\"\nrebuild = false\nraw_bytes = 1\ngzip_bytes = 1\nsha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\n";
        assert!(parse_record(bad_schema).is_err());
        let no_package = "schema = \"lumen.web.size-record/1\"\n\n[[config]]\nname = \"x\"\nbudget_class = \"full-p0\"\nrebuild = true\nraw_bytes = 1\ngzip_bytes = 1\nsha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\n";
        assert!(parse_record(no_package).is_err());
    }

    // -- the byte-stable rewrite ---------------------------------------------

    #[test]
    fn rewrite_touches_only_the_named_block() {
        let text = "\
# header comment
schema = \"lumen.web.size-record/1\"

[[config]]
name = \"floor\"
raw_bytes = 1
gzip_bytes = 2
sha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"

[[config]]
name = \"other\"
raw_bytes = 7
gzip_bytes = 8
sha256 = \"1111111111111111111111111111111111111111111111111111111111111111\"
";
        let mut updates = BTreeMap::new();
        updates.insert(
            "floor".to_string(),
            Measured {
                raw_bytes: 100,
                gzip_bytes: 42,
                sha256: "2222222222222222222222222222222222222222222222222222222222222222"
                    .to_string(),
            },
        );
        let rewritten = rewrite_measured_rows(text, &updates);
        assert!(rewritten.is_ok(), "{rewritten:?}");
        let rewritten = rewritten.unwrap_or_default();
        assert!(
            rewritten.contains("# header comment"),
            "comments preserved: {rewritten}"
        );
        assert!(
            rewritten
                .contains("name = \"floor\"\nraw_bytes = 100\ngzip_bytes = 42\nsha256 = \"2222"),
            "the floor row is rewritten: {rewritten}"
        );
        assert!(
            rewritten.contains("name = \"other\"\nraw_bytes = 7\ngzip_bytes = 8"),
            "the other row is untouched: {rewritten}"
        );
    }

    #[test]
    fn rewrite_is_byte_stable_for_identical_measurements() {
        let text = "schema = \"lumen.web.size-record/1\"\n\n[[config]]\nname = \"floor\"\nraw_bytes = 100\ngzip_bytes = 42\nsha256 = \"2222222222222222222222222222222222222222222222222222222222222222\"\n";
        let mut updates = BTreeMap::new();
        updates.insert(
            "floor".to_string(),
            Measured {
                raw_bytes: 100,
                gzip_bytes: 42,
                sha256: "2222222222222222222222222222222222222222222222222222222222222222"
                    .to_string(),
            },
        );
        let rewritten = rewrite_measured_rows(text, &updates).unwrap_or_default();
        assert_eq!(rewritten, text, "same measurements -> same bytes");
    }

    #[test]
    fn rewrite_fails_when_a_measured_row_has_no_block() {
        let text = "schema = \"lumen.web.size-record/1\"\n\n[[config]]\nname = \"floor\"\nraw_bytes = 1\ngzip_bytes = 2\nsha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\n";
        let mut updates = BTreeMap::new();
        updates.insert(
            "ghost".to_string(),
            Measured {
                raw_bytes: 1,
                gzip_bytes: 1,
                sha256: "0".repeat(64),
            },
        );
        assert!(rewrite_measured_rows(text, &updates).is_err());
    }

    // -- line helpers ----------------------------------------------------------

    #[test]
    fn int_line_replacement_preserves_indent_and_rejects_lookalikes() {
        assert_eq!(
            replace_int_value("  raw_bytes = 1294311", "raw_bytes", 5),
            Some("  raw_bytes = 5".to_string())
        );
        assert_eq!(
            replace_int_value("raw_bytes_extra = 1", "raw_bytes", 5),
            None
        );
        assert_eq!(
            replace_int_value("gzip_bytes = 12 # comment", "gzip_bytes", 5),
            None
        );
        assert_eq!(
            replace_str_value("sha256 = \"abc\"", "sha256", "def"),
            Some("sha256 = \"def\"".to_string())
        );
    }
}
