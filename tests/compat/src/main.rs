//! `compat-matrix` — the full-corpus compatibility matrix runner
//! (issue #53, cards 8.B1–8.B2; `docs/plans/phase8-cards.md` P8-2).
//!
//! What this tool IS: the honest evidence engine for the Phase-8
//! compatibility matrix. It iterates the fixture corpus (the committed
//! gate/codec fixtures plus the regenerable manifest-anchored set that
//! reproduces the VLC 3.0.23 differential inputs), runs every file
//! through the **real** Lumen open/decode path — the same composition
//! the player uses: [`lumen_cli::demux_host::open_source`] → the
//! container-host registry → [`lumen_cli::pipeline_host`]'s
//! [`HostDecoderProvider`] behind the [`codec_api::Decoder`] contract —
//! and records, per file and per track: probe result, demux result,
//! decode counts, concealed damage, and the decoded-audio MD5 (the
//! documented digest of the parity method, RFC 1321 in-repo via
//! `bench-vs-vlc`).
//!
//! For the lossless-comparable rows it then diffs against the pinned
//! VLC 3.0.23 oracle using the **documented MD5-prefix method**
//! (`docs/corpus/vlc-diff-report-20261023.md`: VLC's `afile` dump is a
//! bit-exact prefix of the decoded payload; the shortfall is the
//! unflushed tail) and the **ratified f32 tolerance path**
//! (`docs/corpus/vlc-f32-tolerance-20261109.json`: RMS ≤ 1e-4 AND
//! peak ≤ 1e-3 — thresholds come from the config, never code).
//!
//! What this tool is NOT: it never edits `docs/feature-matrix.toml`.
//! Matrix status transitions are applied ONLY by
//! `cargo xtask compat-matrix apply-transitions` (the parity tool's
//! rule-enforcing writer; `tools/xtask/src/cmd/compat_matrix.rs`),
//! from the transitions outbox this runner emits — the doc 10 §7.3
//! shape (runner collects → outbox → rule-checked application →
//! `feature-audit` judges). Nothing here self-verifies (MP §15).
//!
//! Everything user-visible is config-driven (`compat-matrix.json`,
//! schema `compat-matrix/config/1`): corpus roots, media extensions,
//! the regenerable-corpus manifest + cache, oracle invocation + record
//! paths, tolerance thresholds, lane step commands, report paths.
//!
//! # Exit codes
//!
//! * `0` — the run completed (rows are recorded honestly whatever they
//!   say; a failed file is a RESULT, not a tool failure).
//! * `2` — tool error (bad usage, unreadable config/corpus, a lane
//!   step that could not run). Partial rows already written stay on
//!   disk (the raw row file is appended incrementally).
#![forbid(unsafe_code)]

mod config;
mod corpus;
mod decode;
mod oracle;
mod proc;
mod report;
mod sha256;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::config::Config;

const USAGE: &str = "\
Usage: compat-matrix <subcommand> [args]

Subcommands:
  run --config <compat-matrix.json>
      The full run: ensure the regenerable corpus, decode every corpus
      file through the real open/decode path, run the configured lane
      steps (cargo build/test records), run the VLC oracle where
      configured, and render the reports + transitions outbox.
  ensure-corpus --config <compat-matrix.json>
      Only regenerate/verify the regenerable corpus from the committed
      manifest (ffmpeg argvs, sha256-checked for deterministic rows).
  compare-pcm <lumen.pcm> <oracle.pcm> --sample {u8|s16|f32}
      One-off comparison with the documented method: MD5-prefix for
      integer PCM, RMS/peak tolerance for f32 (thresholds printed,
      not decided, here).

Exit codes: 0 run completed | 2 tool error.";

/// The typed tool error. No panics, no unwraps (repo lint policy).
#[derive(Debug)]
enum ToolError {
    /// Bad command line.
    Usage(String),
    /// An I/O failure with context.
    Io {
        context: String,
        source: std::io::Error,
    },
    /// A config/schema problem.
    Config(String),
    /// A subprocess that could not produce a usable result.
    Subprocess(String),
}

impl ToolError {
    fn usage(msg: impl Into<String>) -> ToolError {
        ToolError::Usage(msg.into())
    }

    fn config(msg: impl Into<String>) -> ToolError {
        ToolError::Config(msg.into())
    }

    fn io(context: impl Into<String>, source: std::io::Error) -> ToolError {
        ToolError::Io {
            context: context.into(),
            source,
        }
    }
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolError::Usage(msg) => write!(f, "{msg}\n{USAGE}"),
            ToolError::Io { context, source } => write!(f, "{context}: {source}"),
            ToolError::Config(msg) => write!(f, "config: {msg}"),
            ToolError::Subprocess(msg) => write!(f, "subprocess: {msg}"),
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("compat-matrix error: {err}");
            ExitCode::from(2)
        }
    }
}

/// Parse argv and dispatch. Mirrors xtask's exit-code contract.
fn run(args: &[String]) -> Result<ExitCode, ToolError> {
    let Some(first) = args.first() else {
        return Err(ToolError::usage("no subcommand given"));
    };
    match first.as_str() {
        "run" => {
            let config = load_config_from(args)?;
            report::full_run(&config).map(|_| ExitCode::SUCCESS)
        }
        "ensure-corpus" => {
            let config = load_config_from(args)?;
            corpus::ensure_regen_corpus(&config).map(|_| ExitCode::SUCCESS)
        }
        "compare-pcm" => oracle::compare_pcm_cli(&args[1..]),
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        other => Err(ToolError::usage(format!("unknown subcommand `{other}`"))),
    }
}

/// Load the config named by `--config <path>` (required for `run` and
/// `ensure-corpus`).
fn load_config_from(args: &[String]) -> Result<Config, ToolError> {
    let mut path: Option<&str> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err(ToolError::usage("--config requires a path"));
                };
                path = Some(value);
            }
            other => {
                return Err(ToolError::usage(format!("unexpected argument `{other}`")));
            }
        }
        i += 1;
    }
    let Some(path) = path else {
        return Err(ToolError::usage("this subcommand requires --config <path>"));
    };
    config::load(Path::new(path))
}

/// Locate the workspace root (the directory holding `Cargo.toml` +
/// `docs/`) by walking up from the binary's cwd — the same discovery
/// rule xtask uses, so relative corpus/report paths in the config
/// resolve identically no matter where the tool is invoked from.
pub(crate) fn find_workspace_root() -> Result<PathBuf, ToolError> {
    let cwd =
        std::env::current_dir().map_err(|e| ToolError::io("determining current directory", e))?;
    let mut dir: &Path = &cwd;
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("docs").is_dir() {
            return Ok(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => {
                return Err(ToolError::config(
                    "workspace root not found — run inside the Lumen workspace",
                ));
            }
        }
    }
}
