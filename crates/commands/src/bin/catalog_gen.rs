//! `catalog-gen` — the regeneration entry point of the command-catalog
//! codegen pipeline (card 1.D4, doc 05 §5.1 "Shared types").
//!
//! Derives the machine description of the whole catalog from the wire
//! types in `crates/commands/src/catalog` (schemars) and emits the three
//! committed outputs deterministically:
//!
//! * `schemas/commands.schema.json`           — the JSON Schema (§5.1a)
//! * `ui/src/generated/commands.ts`           — the TS types (§5.1b)
//! * `schemas/generated/commands.checksum.txt` — the catalog checksum
//!
//! The (frozen) xtask `codegen` subcommand shells out to this binary —
//! the Lead-ratified subprocess bridge pattern (cards 1.B2/1.B4; xtask
//! cannot take a path dependency without an edit to its frozen
//! manifest):
//!
//! ```text
//! cargo run -p commands --bin catalog-gen --quiet -- [args]
//! ```
//!
//! Exit codes: `0` success (or no drift with `--check`), `1` drift with
//! `--check`, `2` usage or I/O error. All errors are typed and
//! actionable on stderr; no panics, no unwraps.

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use commands::catalog::codegen::OutputPaths;
use commands::catalog::codegen::check_against;
use commands::catalog::codegen::generate_outputs;

const USAGE: &str = "\
catalog-gen — regenerate the Lumen command-catalog outputs (card 1.D4, doc 05 §5.1)

Usage: catalog-gen [OPTIONS]

Options:
      --schema-out <PATH>     JSON Schema output
                               (default: <root>/schemas/commands.schema.json)
      --ts-out <PATH>         TypeScript types output
                               (default: <root>/ui/src/generated/commands.ts)
      --checksum-out <PATH>   checksum file output
                               (default: <root>/schemas/generated/commands.checksum.txt)
      --out-dir <DIR>         write all three outputs into <DIR> using their
                               default names (conflicts with the --*-out flags)
      --check                 compare the outputs against regeneration without
                               writing; exit 1 on drift (CI mode)
  -h, --help                  print this help

The workspace root is found by walking up from the current directory,
looking for a Cargo.toml containing a [workspace] table. Exit codes:
0 success / no drift, 1 drift (--check), 2 usage or I/O error.";

/// A terminal failure: an actionable message plus its exit code.
struct Failure {
    code: u8,
    message: String,
}

fn usage_error(message: String) -> Failure {
    Failure {
        code: 2,
        message: format!("{message} — see `catalog-gen --help`"),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("catalog-gen: {}", failure.message);
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
    let paths = output_paths(&options, &root)?;

    let outputs = generate_outputs().map_err(|message| Failure { code: 2, message })?;

    if options.check {
        let drift = check_against(&paths).map_err(|e| Failure {
            code: 2,
            message: format!("reading committed outputs: {e}"),
        })?;
        if !drift.is_empty() {
            for line in &drift {
                eprintln!("catalog-gen: {line}");
            }
            return Err(Failure {
                code: 1,
                message: "output drift — run `cargo xtask codegen` and commit the \
                          refreshed files"
                    .to_owned(),
            });
        }
        println!(
            "catalog-gen: no drift — outputs match the catalog ({} commands / {} events)",
            commands::catalog::spec::catalog_data().len(),
            commands::catalog::spec::event_catalog_data().len(),
        );
    } else {
        write_file(paths.schema, &outputs.schema)?;
        write_file(paths.typescript, &outputs.typescript)?;
        write_file(paths.checksum, &outputs.checksum)?;
        println!(
            "catalog-gen: wrote {} ({} lines)",
            paths.schema.display(),
            outputs.schema.lines().count()
        );
        println!(
            "catalog-gen: wrote {} ({} lines)",
            paths.typescript.display(),
            outputs.typescript.lines().count()
        );
        println!(
            "catalog-gen: wrote {} ({} commands / {} events)",
            paths.checksum.display(),
            commands::catalog::spec::catalog_data().len(),
            commands::catalog::spec::event_catalog_data().len(),
        );
    }
    Ok(())
}

struct Options {
    schema_out: Option<PathBuf>,
    ts_out: Option<PathBuf>,
    checksum_out: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    check: bool,
    help: bool,
}

fn parse_args(args: &[String]) -> Result<Options, Failure> {
    let mut options = Options {
        schema_out: None,
        ts_out: None,
        checksum_out: None,
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
            "--schema-out" | "--ts-out" | "--checksum-out" | "--out-dir" => {
                let value = value_of(args, i, arg)?;
                let path = PathBuf::from(value);
                match arg {
                    "--schema-out" => options.schema_out = Some(path),
                    "--ts-out" => options.ts_out = Some(path),
                    "--checksum-out" => options.checksum_out = Some(path),
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
        && (options.schema_out.is_some()
            || options.ts_out.is_some()
            || options.checksum_out.is_some())
    {
        return Err(usage_error(
            "--out-dir cannot be combined with --schema-out/--ts-out/--checksum-out".to_owned(),
        ));
    }
    Ok(options)
}

/// The value following the flag at `index` (already validated to exist).
fn value_of<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str, Failure> {
    args.get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| usage_error(format!("missing value for {flag}")))
}

/// Resolve the three output paths from the options and workspace root.
fn output_paths(options: &Options, root: &Path) -> Result<OutputPaths<'static>, Failure> {
    if let Some(dir) = &options.out_dir {
        return Ok(OutputPaths {
            schema: &dir.join("commands.schema.json"),
            typescript: &dir.join("commands.ts"),
            checksum: &dir.join("commands.checksum.txt"),
        });
    }
    Ok(OutputPaths {
        schema: &options
            .schema_out
            .clone()
            .unwrap_or_else(|| root.join("schemas").join("commands.schema.json")),
        typescript: &options
            .ts_out
            .clone()
            .unwrap_or_else(|| root.join("ui").join("src").join("generated").join("commands.ts")),
        checksum: &options
            .checksum_out
            .clone()
            .unwrap_or_else(|| {
                root.join("schemas")
                    .join("generated")
                    .join("commands.checksum.txt")
            }),
    })
}

/// Walk up from the current directory to the workspace root (a
/// Cargo.toml containing a `[workspace]` table).
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
                     Lumen workspace, or pass --schema-out/--ts-out/--checksum-out explicitly",
                    dir.display()
                ),
            });
        }
    }
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
