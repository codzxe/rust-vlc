//! Doc 05 §7.4 item 3 — the anti-bypass architecture test.
//!
//! "An architecture test reads the desktop shell's `invoke_handler`
//! registration and fails if any command other than `lumen_dispatch` is
//! registered."
//!
//! Source-scanning (not compiling) the wrapper keeps the test honest on
//! every host — including hosts without webkit (this crate is
//! workspace-excluded; `cargo test` inside it still runs the full
//! battery of the shell library it links). The single-command rule is a
//! REGISTRATION rule, so the test reads the registration site in
//! `src/app.rs` exactly as §7.4 specifies. `check_architecture` covers
//! the workspace-boundary half of §7.4 (item 5); the UI-import halves
//! (items 1/2/4) live in the ESLint/boundary lanes.
//!
//! Comment lines (module docs quoting the promised verbatim signature,
//! the `run` docs) are stripped before scanning: §7.4 audits the
//! REGISTRATION — code — not prose.

/// The registration source with comment lines stripped (`//`, `//!`,
/// `///`): the code the compiler sees.
fn registration_source() -> String {
    let source = include_str!("../src/app.rs");
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//")
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

/// The shell registers exactly one command, and it is `lumen_dispatch`.
/// Any second command — however convenient — is a §5.1 bypass of the
/// generic entrypoint (per-command Rust glue that can drift from the
/// catalog) and fails here.
#[test]
fn the_shell_registers_exactly_one_command_and_it_is_lumen_dispatch() {
    let source = registration_source();

    let handler = source
        .find("generate_handler!")
        .unwrap_or_else(|| panic!("src/app.rs must register commands via generate_handler![…]"));
    let array_open = source[handler..]
        .find('[')
        .unwrap_or_else(|| panic!("generate_handler![…] array form (doc 05 §5.1 verbatim)"));
    let array_close = source[handler + array_open..]
        .find(']')
        .unwrap_or_else(|| panic!("generate_handler![…] closes its array"));
    let body = &source[handler + array_open + 1..handler + array_open + array_close];

    let registered: Vec<&str> = body
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .collect();

    assert_eq!(
        registered,
        vec!["lumen_dispatch"],
        "doc 05 §5.1 single-command rule / §7.4 anti-bypass: the shell may \
         register exactly one command, `lumen_dispatch`; found {registered:?}"
    );
}

/// The wrapper's registration is the ONLY `generate_handler` site in the
/// crate's code — a second site (smuggling its own handler) would
/// bypass the single-command audit above.
#[test]
fn the_registration_site_is_unique_in_the_crate() {
    let source = registration_source();
    let sites = source.matches("generate_handler!").count();
    assert_eq!(
        sites, 1,
        "exactly one `generate_handler!` call site may exist in src/app.rs code"
    );
}
