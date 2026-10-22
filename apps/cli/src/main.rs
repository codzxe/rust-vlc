//! The `lumen` binary (card 2.D2; doc 05 §5.3): thin `main` — the
//! whole player is the library, so every mode is testable in-memory
//! (see [`lumen_cli::run`]). Std-only, no panics: every failure is a
//! typed `CommandError` printed by the one honest printer and mapped
//! to a sysexits-class code.

use std::io::{IsTerminal, Write};

fn main() -> std::process::ExitCode {
    // The wall-clock origin for `decode-bench`'s wall_s (true process
    // wall; the bench falls back to its own start when the library
    // is driven without a main — the in-memory test path).
    lumen_cli::bench::mark_process_start();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let interactive = std::io::stdin().is_terminal();
    let mut input = std::io::stdin().lock();
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    let code = lumen_cli::run::run(&argv, &mut input, &mut out, &mut err, interactive);
    let _ = out.flush();
    let _ = err.flush();
    std::process::ExitCode::from(code)
}
