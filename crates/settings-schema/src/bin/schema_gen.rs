//! `schema-gen` — the regeneration entry point of the settings-schema
//! pipeline (card 1.B2, doc 07 §5.3).
//!
//! Reads the canonical schema source `schemas/settings.toml`, validates it
//! (entry format + D1–D6, joined against `product.toml` for rule D1) and
//! emits the three committed outputs deterministically:
//!
//! * `schemas/generated/settings.d.ts`     (G1 — TS type surface)
//! * `schemas/generated/settings-help.txt` (G2 — CLI `--help` golden)
//! * `schemas/generated/settings-docs.md`  (G3 — settings reference)
//!
//! The (frozen) xtask `schema-gen` subcommand shells out to this binary —
//! the Lead-ratified subprocess bridge pattern from card 1.B4:
//!
//! ```text
//! cargo run -p settings-schema --bin schema-gen --quiet -- [args]
//! ```
//!
//! Exit codes: `0` success (or no drift with `--check`), `1` drift with
//! `--check`, `2` usage, I/O or validation error (every D-rule violation
//! prints its full actionable diagnostic first). All errors are typed and
//! actionable on stderr; no panics, no unwraps.

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use settings_schema::GenTarget;
use settings_schema::LoadError;
use settings_schema::generate_str;

const USAGE: &str = "\
schema-gen — regenerate the Lumen settings-schema outputs (card 1.B2, doc 07 §5.3)

Usage: schema-gen [OPTIONS]

Options:
      --source <PATH>     schema source file
                           (default: <root>/schemas/settings.toml)
      --product <PATH>    product.toml for the rule-D1 join
                           (default: <root>/product.toml)
      --ts-out <PATH>     TS types output
                           (default: <root>/schemas/generated/settings.d.ts)
      --help-out <PATH>   CLI --help golden output
                           (default: <root>/schemas/generated/settings-help.txt)
      --docs-out <PATH>   docs page output
                           (default: <root>/schemas/generated/settings-docs.md)
      --out-dir <DIR>     write all three outputs into <DIR> using their
                           default names (conflicts with the --*-out flags)
      --check             compare the outputs against regeneration without
                           writing; exit 1 on drift (CI mode, doc 07 §9)
  -h, --help              print this help

The workspace root is found by walking up from the current directory,
looking for a Cargo.toml containing a [workspace] table. Exit codes:
0 success / no drift, 1 drift (--check), 2 usage, I/O or validation error.";

/// A terminal failure: an actionable message plus its exit code.
struct Failure {
    code: u8,
    message: String,
}

fn usage_error(message: String) -> Failure {
    Failure {
        code: 2,
        message: format!("{message} — see `schema-gen --help`"),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("schema-gen: {}", failure.message);
            ExitCode::from(failure.code)
        }
    }
}

fn run() -> Result<(), Failure> {
    let args: Vec<String> = env::args().skip(1).collect();
    let options = parse_args(&args)?;
    if options.help {
        println!("{USAGE}");
        return Ok(());
    }

    let root = find_workspace_root()?;
    let source_path = match &options.source {
        Some(path) => path.clone(),
        None => root.join("schemas").join("settings.toml"),
    };
    let product_path = match &options.product {
        Some(path) => path.clone(),
        None => root.join("product.toml"),
    };
    let outputs = output_paths(&options, &root)?;

    let schema = settings_schema::load(&source_path, Some(&product_path)).map_err(failure)?;
    let entries = schema.len();

    let ts = generate_str(&schema, GenTarget::TsTypes);
    let help = generate_str(&schema, GenTarget::CliHelp);
    let docs = generate_str(&schema, GenTarget::Docs);

    if options.check {
        let drift =
            drift(&outputs.ts, &ts)? | drift(&outputs.help, &help)? | drift(&outputs.docs, &docs)?;
        if drift {
            return Err(Failure {
                code: 1,
                message: "output drift — run `cargo xtask schema-gen` and commit the \
                          refreshed files"
                    .to_owned(),
            });
        }
        println!(
            "schema-gen: no drift — outputs match {} ({} entries)",
            source_path.display(),
            entries
        );
    } else {
        write_file(&outputs.ts, &ts)?;
        write_file(&outputs.help, &help)?;
        write_file(&outputs.docs, &docs)?;
        println!(
            "schema-gen: wrote {} ({} lines)",
            outputs.ts.display(),
            ts.lines().count()
        );
        println!(
            "schema-gen: wrote {} ({} lines)",
            outputs.help.display(),
            help.lines().count()
        );
        println!(
            "schema-gen: wrote {} ({} lines)",
            outputs.docs.display(),
            docs.lines().count()
        );
    }

    println!(
        "schema-gen: {} entries across {} screen(s) of {} known areas; validated against {}",
        entries,
        schema.screens_in_use().len(),
        schema.areas.len(),
        product_path.display(),
    );
    Ok(())
}

struct Options {
    source: Option<PathBuf>,
    product: Option<PathBuf>,
    ts_out: Option<PathBuf>,
    help_out: Option<PathBuf>,
    docs_out: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    check: bool,
    help: bool,
}

fn parse_args(args: &[String]) -> Result<Options, Failure> {
    let mut options = Options {
        source: None,
        product: None,
        ts_out: None,
        help_out: None,
        docs_out: None,
        out_dir: None,
        check: false,
        help: false,
    };
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "-h" | "--help" => options.help = true,
            "--check" => options.check = true,
            "--source" | "--product" | "--ts-out" | "--help-out" | "--docs-out" | "--out-dir" => {
                let value = value_of(args, i, arg)?;
                let path = PathBuf::from(value);
                match arg {
                    "--source" => options.source = Some(path),
                    "--product" => options.product = Some(path),
                    "--ts-out" => options.ts_out = Some(path),
                    "--help-out" => options.help_out = Some(path),
                    "--docs-out" => options.docs_out = Some(path),
                    _ => options.out_dir = Some(path),
                }
                i += 1;
            }
            other => {
                return Err(usage_error(format!("unexpected argument `{other}`")));
            }
        }
        i += 1;
    }
    if options.out_dir.is_some()
        && (options.ts_out.is_some() || options.help_out.is_some() || options.docs_out.is_some())
    {
        return Err(usage_error(
            "--out-dir cannot be combined with --ts-out/--help-out/--docs-out".to_owned(),
        ));
    }
    Ok(options)
}

/// The value following the flag at `index` (usage error when missing).
fn value_of<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str, Failure> {
    args.get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| usage_error(format!("missing value for {flag}")))
}

struct Outputs {
    ts: PathBuf,
    help: PathBuf,
    docs: PathBuf,
}

fn output_paths(options: &Options, root: &Path) -> Result<Outputs, Failure> {
    if let Some(dir) = &options.out_dir {
        return Ok(Outputs {
            ts: dir.join("settings.d.ts"),
            help: dir.join("settings-help.txt"),
            docs: dir.join("settings-docs.md"),
        });
    }
    let generated = root.join("schemas").join("generated");
    Ok(Outputs {
        ts: options
            .ts_out
            .clone()
            .unwrap_or_else(|| generated.join("settings.d.ts")),
        help: options
            .help_out
            .clone()
            .unwrap_or_else(|| generated.join("settings-help.txt")),
        docs: options
            .docs_out
            .clone()
            .unwrap_or_else(|| generated.join("settings-docs.md")),
    })
}

/// Walk up from the current directory to the workspace root (a Cargo.toml
/// containing a `[workspace]` table).
fn find_workspace_root() -> Result<PathBuf, Failure> {
    let mut dir = env::current_dir().map_err(|e| Failure {
        code: 2,
        message: format!("determining the current directory: {e}"),
    })?;
    loop {
        let manifest = dir.join("Cargo.toml");
        if let Ok(text) = fs::read_to_string(&manifest)
            && text.contains("[workspace]")
        {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err(Failure {
                code: 2,
                message: format!(
                    "workspace root not found (walked up from {}) — run from within the \
                     Lumen workspace, or pass --source/--product/--out-dir explicitly",
                    dir.display()
                ),
            });
        }
    }
}

/// Map a library load failure to the binary's failure shape (diagnostics
/// are already rendered, actionable text).
fn failure(err: LoadError) -> Failure {
    Failure {
        code: 2,
        message: err.to_string(),
    }
}

/// Whether `path` differs from `expected` (missing counts as drift; the
/// first differing line is reported).
fn drift(path: &Path, expected: &str) -> Result<bool, Failure> {
    let committed = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => {
            eprintln!(
                "schema-gen: {} is missing — run `cargo xtask schema-gen` and commit it",
                path.display()
            );
            return Ok(true);
        }
    };
    if committed == expected {
        return Ok(false);
    }
    let mut line = 1;
    for (old, new) in committed.lines().zip(expected.lines()) {
        if old != new {
            break;
        }
        line += 1;
    }
    eprintln!(
        "schema-gen: {} differs from the regenerated output at line {line} — run \
         `cargo xtask schema-gen` and commit the refreshed file",
        path.display()
    );
    Ok(true)
}

fn write_file(path: &Path, content: &str) -> Result<(), Failure> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| Failure {
            code: 2,
            message: format!("creating {}: {e}", parent.display()),
        })?;
    }
    fs::write(path, content).map_err(|e| Failure {
        code: 2,
        message: format!("writing {}: {e}", path.display()),
    })
}
