//! `tokens-gen` — the regeneration entry point of the theme-token pipeline
//! (card 1.B4, doc 08 §3; completed by salvage card 1.B4-R).
//!
//! Reads the canonical token source `themes/tokens.json`, validates it and
//! emits BOTH committed outputs deterministically:
//!
//! * `themes/generated/tokens.css` (CSS custom properties), and
//! * `crates/theme-tokens/src/generated/tokens.rs` (Rust constants).
//!
//! The (frozen) xtask `tokens` subcommand shells out to this binary — the
//! Lead-ratified subprocess bridge (doc 08 §3.1 "xtask calls
//! crates/theme-tokens"; xtask cannot take a path dependency without an
//! edit to its frozen manifest):
//!
//! ```text
//! cargo run -p theme-tokens --bin tokens-gen --quiet -- [args]
//! ```
//!
//! Exit codes: `0` success (or no drift with `--check`), `1` drift with
//! `--check`, `2` usage, I/O or parse/validation error. All errors are
//! typed and actionable on stderr.

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use theme_tokens::emit;
use theme_tokens::parse;

const USAGE: &str = "\
tokens-gen — regenerate the Lumen theme-token outputs (card 1.B4, doc 08 §3)

Usage: tokens-gen [OPTIONS]

Options:
      --source <PATH>    token source file
                         (default: <root>/themes/tokens.json)
      --css-out <PATH>   CSS output path
                         (default: <root>/themes/generated/tokens.css)
      --rust-out <PATH>  Rust output path
                         (default: <root>/crates/theme-tokens/src/generated/tokens.rs)
      --out-dir <DIR>    write both outputs into <DIR> as tokens.css and
                         tokens.rs (conflicts with --css-out/--rust-out)
      --check            compare the outputs against regeneration without
                         writing; exit 1 on drift (doc 08 §3.4)
  -h, --help             print this help

The workspace root is found by walking up from the current directory,
looking for a Cargo.toml containing a [workspace] table. Exit codes:
0 success / no drift, 1 drift (--check), 2 usage, I/O or parse error.";

/// A terminal failure: an actionable message plus its exit code.
struct Failure {
    code: u8,
    message: String,
}

fn usage_error(message: String) -> Failure {
    Failure {
        code: 2,
        message: format!("{message} — see `tokens-gen --help`"),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("tokens-gen: {}", failure.message);
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
        None => root.join("themes").join("tokens.json"),
    };
    let (css_path, rust_path) = output_paths(&options, &root)?;

    let source = fs::read_to_string(&source_path).map_err(|e| Failure {
        code: 2,
        message: format!("reading {}: {e}", source_path.display()),
    })?;
    let set = parse(&source).map_err(|e| Failure {
        code: 2,
        message: format!("{}: {e}", source_path.display()),
    })?;

    let mut css = Vec::new();
    emit::emit_css(&set, &mut css).map_err(|e| Failure {
        code: 2,
        message: e.to_string(),
    })?;
    let mut rust = Vec::new();
    emit::emit_rust(&set, &mut rust).map_err(|e| Failure {
        code: 2,
        message: e.to_string(),
    })?;
    let css = String::from_utf8(css).map_err(|e| Failure {
        code: 2,
        message: format!("generated CSS is not valid UTF-8: {e}"),
    })?;
    let rust = String::from_utf8(rust).map_err(|e| Failure {
        code: 2,
        message: format!("generated Rust is not valid UTF-8: {e}"),
    })?;

    if options.check {
        let css_drift = drift(&css_path, &css)?;
        let rust_drift = drift(&rust_path, &rust)?;
        if css_drift || rust_drift {
            return Err(Failure {
                code: 1,
                message: "output drift — run `cargo xtask tokens` and commit the \
                         refreshed files"
                    .to_owned(),
            });
        }
        println!(
            "tokens-gen: no drift — outputs match {}",
            source_path.display()
        );
    } else {
        write_file(&css_path, &css)?;
        write_file(&rust_path, &rust)?;
        println!(
            "tokens-gen: wrote {} ({} lines)",
            css_path.display(),
            css.lines().count()
        );
        println!(
            "tokens-gen: wrote {} ({} lines)",
            rust_path.display(),
            rust.lines().count()
        );
    }

    let counts = set.counts();
    println!(
        "tokens-gen: {} primitive / {} alias / {} semantic tokens; {} mode(s) ({} default); \
         {} accent(s) ({} default)",
        counts.primitives,
        counts.aliases,
        counts.semantic,
        set.modes().len(),
        set.default_mode(),
        set.accents().len(),
        set.default_accent(),
    );
    Ok(())
}

struct Options {
    source: Option<PathBuf>,
    css_out: Option<PathBuf>,
    rust_out: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    check: bool,
    help: bool,
}

fn parse_args(args: &[String]) -> Result<Options, Failure> {
    let mut options = Options {
        source: None,
        css_out: None,
        rust_out: None,
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
            "--source" | "--css-out" | "--rust-out" | "--out-dir" => {
                let value = value_of(args, i, arg)?;
                let path = PathBuf::from(value);
                match arg {
                    "--source" => options.source = Some(path),
                    "--css-out" => options.css_out = Some(path),
                    "--rust-out" => options.rust_out = Some(path),
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
    if options.out_dir.is_some() && (options.css_out.is_some() || options.rust_out.is_some()) {
        return Err(usage_error(
            "--out-dir cannot be combined with --css-out/--rust-out".to_owned(),
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

/// Resolve the CSS + Rust output paths from the options and workspace root.
fn output_paths(options: &Options, root: &Path) -> Result<(PathBuf, PathBuf), Failure> {
    if let Some(dir) = &options.out_dir {
        return Ok((dir.join("tokens.css"), dir.join("tokens.rs")));
    }
    let css = options
        .css_out
        .clone()
        .unwrap_or_else(|| root.join("themes").join("generated").join("tokens.css"));
    let rust = options.rust_out.clone().unwrap_or_else(|| {
        root.join("crates")
            .join("theme-tokens")
            .join("src")
            .join("generated")
            .join("tokens.rs")
    });
    Ok((css, rust))
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
                     Lumen workspace, or pass --source/--css-out/--rust-out explicitly",
                    dir.display()
                ),
            });
        }
    }
}

/// Whether `path` differs from `expected` (missing counts as drift; the
/// first differing line is reported).
fn drift(path: &Path, expected: &str) -> Result<bool, Failure> {
    let committed = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => {
            eprintln!(
                "tokens-gen: {} is missing — run `cargo xtask tokens` and commit it",
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
        "tokens-gen: {} differs from the regenerated output at line {line} — run \
         `cargo xtask tokens` and commit the refreshed file",
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
