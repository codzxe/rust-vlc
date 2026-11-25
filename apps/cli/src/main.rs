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
    // argv in its OS-native form: a non-UTF-8 token (legal on Linux,
    // where filenames are byte strings) is the grammar's typed
    // refusal — never `std::env::args()`'s panic (its iterator
    // unwraps the `OsString`→`String` conversion and aborts with
    // exit 101 before a single player line runs).
    let argv = match std::env::args_os()
        .skip(1)
        .map(|raw| lumen_cli::parse::argv_string(&raw))
        .collect::<Result<Vec<String>, _>>()
    {
        Ok(argv) => argv,
        Err(failure) => {
            // The parse-refusal lane `run` itself takes: the runner's
            // parse-error printer + its exit code (64 EX_USAGE,
            // `exit::exit_code(InvalidArgument)`).
            let mut err = std::io::stderr().lock();
            lumen_cli::run::print_parse_error(&mut err, &failure);
            return std::process::ExitCode::from(lumen_cli::exit::exit_code(
                commands::ErrorCode::InvalidArgument,
            ));
        }
    };
    let interactive = std::io::stdin().is_terminal();
    let mut input = std::io::stdin().lock();
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    let code = lumen_cli::run::run(&argv, &mut input, &mut out, &mut err, interactive);
    let _ = out.flush();
    let _ = err.flush();
    std::process::ExitCode::from(code)
}
