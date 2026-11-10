//! `lumen-control-server` — the binary (card 7.J1).
//!
//! Process contract:
//!
//! * exit 0 — `--help`, or the session cap (`--sessions`) was served;
//! * exit 2 — config/product trouble (unreadable/unparsable config,
//!   bad policy grants, product.toml diagnostics, feature flag off);
//! * exit 3 — bind trouble (non-loopback refusals included);
//! * exit 4 — the serve loop failed.
//!
//! Startup prints the bound `ws://` address on STDOUT (script-friendly)
//! and, when the token is generated per start, the token on STDERR (the
//! single sanctioned reader path — doc 05 §5.4 "printed to the
//! controlling terminal"). Structured session records are JSON lines on
//! STDERR. No signal handling (std-only): SIGTERM/SIGINT end the
//! process; a control surface dies with its product, by design.

use std::path::Path;
use std::path::PathBuf;

use control_protocol::ControlServer;
use control_protocol::PROTOCOL_VERSION;
use control_protocol::TokenSource;
use control_protocol::token::EntropySource;

use control_server::AppError;
use control_server::CatalogDispatch;
use control_server::JsonStderr;
use control_server::load_config_file;
use control_server::resolve_server_config;
use control_server::serve_loop;

/// Default operator config path (missing file = strict defaults).
// lumen-allow-literal: default operator config file name (doc 05 §5.4) — a path convention, not product identity
const DEFAULT_CONFIG: &str = "control-server.toml";
/// Default product model path (repo root / product working dir).
const DEFAULT_PRODUCT: &str = "product.toml";
/// Default product schema path.
const DEFAULT_SCHEMA: &str = "product.schema.json";

/// The parsed command line.
struct Args {
    config: PathBuf,
    product: PathBuf,
    schema: PathBuf,
    sessions: Option<u32>,
}

/// What `parse_args` produced.
enum Parsed {
    /// `--help` was given.
    Help,
    /// A usable command line.
    Args(Args),
    /// A usage error (the detail; empty detail = plain `--help` form).
    Usage(String),
}

/// Read the value after the flag at `*index`, advancing `*index` past it.
fn value_after(argv: &[String], index: &mut usize, flag: &str) -> Result<String, String> {
    *index += 1;
    match argv.get(*index) {
        Some(value) => Ok(value.clone()),
        None => Err(format!("{flag} needs a value")),
    }
}

/// Parse argv (unknown flags are usage errors; `--help` prints usage).
fn parse_args(argv: &[String]) -> Parsed {
    let mut config = PathBuf::from(DEFAULT_CONFIG);
    let mut product = PathBuf::from(DEFAULT_PRODUCT);
    let mut schema = PathBuf::from(DEFAULT_SCHEMA);
    let mut sessions: Option<u32> = None;
    let mut index = 0;
    while index < argv.len() {
        let arg = argv[index].as_str();
        let outcome: Result<(), String> = match arg {
            "--help" | "-h" => return Parsed::Help,
            "--config" => {
                value_after(argv, &mut index, "--config").map(|value| config = PathBuf::from(value))
            }
            "--product" => value_after(argv, &mut index, "--product")
                .map(|value| product = PathBuf::from(value)),
            "--schema" => {
                value_after(argv, &mut index, "--schema").map(|value| schema = PathBuf::from(value))
            }
            "--sessions" => value_after(argv, &mut index, "--sessions").and_then(|raw| {
                raw.parse::<u32>()
                    .map(|parsed| sessions = Some(parsed))
                    .map_err(|_| format!("--sessions needs a non-negative integer, got {raw:?}"))
            }),
            other => Err(format!("unknown argument {other:?}")),
        };
        if let Err(detail) = outcome {
            return Parsed::Usage(detail);
        }
        index += 1;
    }
    Parsed::Args(Args {
        config,
        product,
        schema,
        sessions,
    })
}

/// The usage text (also the `--help` output).
fn usage() -> String {
    format!(
        "lumen-control-server — the Lumen loopback control surface (card 7.J1)\n\
         \n\
         USAGE:\n\
         \x20   lumen-control-server [--config <control-server.toml>] [--product <product.toml>]\n\
         \x20                        [--schema <product.schema.json>] [--sessions <n>]\n\
         \n\
         OPTIONS:\n\
         \x20   --config    operator config [default: {DEFAULT_CONFIG}; missing = strict defaults]\n\
         \x20   --product   product model [default: {DEFAULT_PRODUCT}]\n\
         \x20   --schema    product schema [default: {DEFAULT_SCHEMA}]\n\
         \x20   --sessions  serve at most <n> sessions then exit 0 [default: unlimited]\n\
         \x20   --help      this text\n\
         \n\
         The server binds loopback only (127.0.0.1/::1 — anything else is a\n\
         typed startup error), authenticates a per-start token, and denies\n\
         every sensitive catalog command unless the config grants it.\n\
         lumen-control/{PROTOCOL_VERSION} over WebSocket (RFC 6455).\n"
    )
}

/// Load `product.toml` through the schema pipeline.
fn load_product(
    path: &Path,
    schema_path: &Path,
) -> Result<product_config::ProductConfig, AppError> {
    let schema_text = match std::fs::read_to_string(schema_path) {
        Ok(text) => text,
        Err(source) => {
            return Err(AppError::ConfigRead {
                path: schema_path.to_owned(),
                source,
            });
        }
    };
    let schema = match schema_text.parse::<product_config::Schema>() {
        Ok(schema) => schema,
        Err(source) => {
            return Err(AppError::ConfigParse {
                path: schema_path.to_owned(),
                detail: source.to_string(),
            });
        }
    };
    product_config::load_file(path, &schema)
        .map_err(|diagnostics| AppError::ProductLoad { diagnostics })
}

/// Main. Returns the process exit code; the caller exits with it only
/// when non-zero (falling off `main` is the natural exit-0 path — no
/// `process::exit` shenanigans on the happy path).
fn run(argv: &[String]) -> i32 {
    let args = match parse_args(argv) {
        Parsed::Help => {
            print!("{}", usage());
            return 0;
        }
        Parsed::Usage(detail) => {
            eprintln!("usage error: {detail}\n\n{}", usage());
            return 2;
        }
        Parsed::Args(args) => args,
    };
    // 1. Product model (rule F2 gate runs inside resolve_server_config).
    let product = match load_product(&args.product, &args.schema) {
        Ok(product) => product,
        Err(err) => {
            eprintln!("{err}");
            return err.exit_code();
        }
    };
    // 2. Operator config (missing file = strict defaults).
    let file = match std::fs::read_to_string(&args.config) {
        Ok(_) => match load_config_file(&args.config) {
            Ok(file) => file,
            Err(err) => {
                eprintln!("{err}");
                return err.exit_code();
            }
        },
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            control_protocol::policy::PolicyFile::default()
        }
        Err(source) => {
            let err = AppError::ConfigRead {
                path: args.config.clone(),
                source,
            };
            eprintln!("{err}");
            return err.exit_code();
        }
    };
    // 3. Resolve + bind. The dispatch reuses the SAME engineInfo the
    // welcome will carry (one source, no drift).
    let config = match resolve_server_config(&product, &file) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            return err.exit_code();
        }
    };
    let token_generated = matches!(config.token, TokenSource::Generated);
    let engine_info = config.engine_info.clone();
    let mut server = match ControlServer::bind(config) {
        Ok(server) => server,
        Err(source) => {
            let err = AppError::Bind { source };
            eprintln!("{err}");
            return err.exit_code();
        }
    };
    let address = match server.local_addr() {
        Ok(address) => address,
        Err(source) => {
            let err = AppError::Bind { source };
            eprintln!("{err}");
            return err.exit_code();
        }
    };
    // The token's single sanctioned reader: the controlling terminal.
    // The structured log NEVER carries it (V8).
    if token_generated {
        eprintln!(
            "control-server session token (keep private): {}",
            server.token()
        );
        match server.entropy_source() {
            EntropySource::OsRandom => {}
            EntropySource::WeakFallback => {
                eprintln!("warning: OS randomness unavailable; token is time-derived (weak)");
            }
        }
    }
    println!("lumen-control/{PROTOCOL_VERSION} listening on ws://{address}");
    // 4. Serve.
    let mut dispatch = CatalogDispatch::new(engine_info);
    let mut sink = JsonStderr::new();
    match serve_loop(&mut server, &mut dispatch, &mut sink, args.sessions) {
        Ok(outcome) => {
            eprintln!(
                "control-server served {} session(s){}",
                outcome.sessions,
                if outcome.capped { " (cap reached)" } else { "" }
            );
            0
        }
        Err(source) => {
            eprintln!("serve loop failed: {source}");
            4
        }
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let code = run(&argv);
    if code != 0 {
        std::process::exit(code);
    }
}
