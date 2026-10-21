//! The argv/REPL grammar (card 2.D2): subcommand words → catalog
//! [`Command`] payloads, plus the seek-time grammar of doc 05 §5.3's
//! CLI table (`90s`, `+10s`, `-1m`, `50%`, `chapter 3`).
//!
//! The grammar is deliberately tiny and total: every token the CLI
//! accepts maps to a catalog command or a documented host verb, and
//! everything else is a typed `invalid-argument` with a usage hint —
//! never a guess, never a partial parse that silently drops tokens.
//!
//! # Host verbs (not catalog commands)
//!
//! `advance [ticks]` pumps the engine deterministically (the scripted
//! session's run-to-end verb — see [`crate::host`]); `quit`/`q` leaves
//! the REPL; `help` prints the subcommand table. These are host
//! conveniences over the same dispatch, documented as such — the
//! card's command surface itself is the catalog subset.

use commands::catalog::types::{OpenOptions, SeekTarget, SourceSpec, Timestamp};

/// One parsed CLI action (argv mode and REPL lines share the grammar).
#[derive(Debug, Clone, PartialEq)]
pub enum CliCommand {
    /// `open <path> [--start-time T] [--start-paused] [--no-video]
    /// [--no-audio]` → `session.open`.
    Open {
        /// The path (or URL) to open.
        source: String,
        /// `--start-time` (µs).
        start_time: Option<i64>,
        /// `--start-paused`.
        start_paused: bool,
        /// `--no-video`.
        no_video: bool,
        /// `--no-audio`.
        no_audio: bool,
    },
    /// `play` → `playback.play`.
    Play,
    /// `pause` → `playback.pause`.
    Pause,
    /// `toggle` → `playback.toggle-play-pause`.
    Toggle,
    /// `stop` → `playback.stop`.
    Stop,
    /// `close` → `session.close`.
    Close,
    /// `seek <target>` → `playback.seek`.
    Seek(SeekArg),
    /// `info` → `media.get-info`.
    Info,
    /// `time` → `media.get-current-time`.
    Time,
    /// `duration` → `media.get-duration`.
    Duration,
    /// `tracks` → `tracks.list`.
    Tracks,
    /// `engine-info` → `system.get-engine-info`.
    EngineInfo,
    /// `commands` → `system.list-commands`.
    Commands,
    /// `advance [ticks]` — host verb: pump the engine `ticks` times
    /// (deterministic, no wall clock).
    Advance { ticks: u32 },
    /// `setting get <area.key>` → `config.get-setting`.
    SettingGet { key: String },
    /// `setting set <area.key> <value>` → `config.set-setting`.
    SettingSet { key: String, value: String },
    /// `help` — host verb.
    Help,
    /// `quit` / `q` — host verb (REPL exit).
    Quit,
}

/// One seek argument, parsed per doc 05 §5.3's grammar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeekArg {
    /// `90s`, `1m30s`, `500ms`, plain `90` (seconds).
    Absolute { us: i64 },
    /// `+10s`, `-1m`.
    Relative { ms: i64 },
    /// `50%`, `12.5%`.
    Percent { fraction: f32 },
    /// `chapter 3`.
    Chapter { index: u32 },
}

impl SeekArg {
    /// Build the catalog [`SeekTarget`]. The host resolves
    /// relative/percent against the live position/duration at dispatch
    /// time; the pure grammar here only classifies.
    #[must_use]
    pub fn to_target(self) -> SeekTarget {
        match self {
            SeekArg::Absolute { us } => SeekTarget::Absolute {
                time: Timestamp(us),
            },
            SeekArg::Relative { ms } => SeekTarget::Relative { delta_ms: ms },
            SeekArg::Percent { fraction } => SeekTarget::Percent { fraction },
            SeekArg::Chapter { index } => SeekTarget::Chapter { index },
        }
    }
}

/// The whole argv invocation: global flags + at most one command.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Invocation {
    /// `--json`: machine output (NDJSON events + responses).
    pub json: bool,
    /// `--control-stdio`: NDJSON request/response machine mode.
    pub control_stdio: bool,
    /// `--help` or `--help <area>` (the settings area).
    pub help: Option<Option<String>>,
    /// `--version`.
    pub version: bool,
    /// The subcommand, when present (`None` = bare `lumen` → usage).
    pub command: Option<CliCommand>,
}

/// A grammar failure, already shaped for the honest printer.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// Human sentence (redaction-safe: argv is not echoed wholesale —
    /// only the offending token, capped).
    pub message: String,
    /// The usage hint printed beneath the error.
    pub hint: String,
}

impl ParseError {
    fn new(message: impl Into<String>, hint: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            hint: hint.into(),
        }
    }
}

/// The usage line every parse error carries.
pub const USAGE: &str = "usage: lumen [OPTIONS] <COMMAND> [ARGS] — try `lumen --help`";

/// Parse the argv (argv[0] already removed).
///
/// # Errors
/// [`ParseError`] for unknown verbs, missing/malformed arguments.
pub fn parse_argv(argv: &[String]) -> Result<Invocation, ParseError> {
    let mut invocation = Invocation::default();
    let mut command_words: Vec<String> = Vec::new();
    for (index, arg) in argv.iter().enumerate() {
        match arg.as_str() {
            "--json" => invocation.json = true,
            "--control-stdio" => invocation.control_stdio = true,
            "--version" | "-v" => invocation.version = true,
            "-h" | "--help" => {
                invocation.help = Some(None);
                // `lumen --help <area>`: the settings area is the FINAL
                // argv word (`--help open clip.avi` is help, not an
                // area named "open").
                if index + 2 == argv.len() && !argv[index + 1].starts_with('-') {
                    invocation.help = Some(Some(argv[index + 1].clone()));
                }
            }
            _ => command_words.push(arg.clone()),
        }
    }
    if invocation.help.is_none() && !invocation.version && !command_words.is_empty() {
        invocation.command = Some(parse_command(&command_words)?);
    }
    Ok(invocation)
}

/// Parse one command (a word list: argv after global flags, or one
/// REPL line split on whitespace).
///
/// # Errors
/// [`ParseError`] for unknown verbs / bad arguments.
pub fn parse_command(words: &[String]) -> Result<CliCommand, ParseError> {
    let Some((verb, args)) = words.split_first() else {
        return Err(ParseError::new("no command given", USAGE));
    };
    let no_args = |cmd| {
        if args.is_empty() {
            Ok(cmd)
        } else {
            Err(ParseError::new(
                format!("`{verb}` takes no arguments"),
                USAGE,
            ))
        }
    };
    match verb.as_str() {
        "open" => parse_open(args),
        "play" => no_args(CliCommand::Play),
        "pause" => no_args(CliCommand::Pause),
        "toggle" | "toggle-play-pause" => no_args(CliCommand::Toggle),
        "stop" => no_args(CliCommand::Stop),
        "close" => no_args(CliCommand::Close),
        "seek" => parse_seek(args),
        "info" => no_args(CliCommand::Info),
        "time" => no_args(CliCommand::Time),
        "duration" => no_args(CliCommand::Duration),
        "tracks" => no_args(CliCommand::Tracks),
        "engine-info" => no_args(CliCommand::EngineInfo),
        "commands" | "list-commands" => no_args(CliCommand::Commands),
        "advance" => parse_advance(args),
        "setting" => parse_setting(args),
        "help" => no_args(CliCommand::Help),
        "quit" | "q" => no_args(CliCommand::Quit),
        other => Err(ParseError::new(
            format!("unknown command `{other}`"),
            USAGE,
        )),
    }
}

/// `open <path> [--start-time T] [--start-paused] [--no-video]
/// [--no-audio]` (doc 05 §5.3's flag set).
fn parse_open(args: &[String]) -> Result<CliCommand, ParseError> {
    let mut source: Option<String> = None;
    let mut start_time: Option<i64> = None;
    let mut start_paused = false;
    let mut no_video = false;
    let mut no_audio = false;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--start-time" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    ParseError::new("--start-time needs a time argument", "e.g. --start-time 30s")
                })?;
                start_time = Some(parse_time_us(value)?);
                index += 2;
            }
            "--start-paused" => {
                start_paused = true;
                index += 1;
            }
            "--no-video" => {
                no_video = true;
                index += 1;
            }
            "--no-audio" => {
                no_audio = true;
                index += 1;
            }
            _ => {
                if source.is_some() {
                    return Err(ParseError::new(
                        "open takes exactly one source path",
                        "usage: lumen open <path> [flags]",
                    ));
                }
                source = Some(args[index].clone());
                index += 1;
            }
        }
    }
    let source = source.ok_or_else(|| {
        ParseError::new("open needs a source path", "usage: lumen open <path> [flags]")
    })?;
    Ok(CliCommand::Open {
        source,
        start_time,
        start_paused,
        no_video,
        no_audio,
    })
}

/// `seek <target>` with doc 05 §5.3's grammar; `chapter N` is
/// two-token.
fn parse_seek(args: &[String]) -> Result<CliCommand, ParseError> {
    if args.len() != 1 && !(args.len() == 2 && args[0] == "chapter") {
        return Err(ParseError::new(
            "seek takes one target",
            "usage: lumen seek 90s | +10s | -1m | 50% | chapter 3",
        ));
    }
    if args.len() == 2 && args[0] == "chapter" {
        let index: u32 = parse_number(&args[1])
            .ok_or_else(|| ParseError::new("chapter index must be a number", "e.g. chapter 3"))?;
        return Ok(CliCommand::Seek(SeekArg::Chapter { index }));
    }
    Ok(CliCommand::Seek(parse_seek_target(&args[0])?))
}

/// `advance [ticks]`.
fn parse_advance(args: &[String]) -> Result<CliCommand, ParseError> {
    match args.len() {
        0 => Ok(CliCommand::Advance { ticks: 64 }),
        1 => {
            let ticks: u32 = parse_number(&args[0]).ok_or_else(|| {
                ParseError::new("advance takes a tick count", "e.g. advance 128")
            })?;
            Ok(CliCommand::Advance { ticks })
        }
        _ => Err(ParseError::new("advance takes at most one count", USAGE)),
    }
}

/// `setting get <key>` / `setting set <key> <value>`.
fn parse_setting(args: &[String]) -> Result<CliCommand, ParseError> {
    let Some((action, rest)) = args.split_first() else {
        return Err(ParseError::new(
            "setting needs `get` or `set`",
            "usage: lumen setting get <area.key> | setting set <area.key> <value>",
        ));
    };
    match action.as_str() {
        "get" => {
            if rest.len() != 1 {
                return Err(ParseError::new(
                    "setting get takes one <area.key>",
                    "e.g. setting get playback.autoplay-next",
                ));
            }
            Ok(CliCommand::SettingGet { key: rest[0].clone() })
        }
        "set" => {
            if rest.len() != 2 {
                return Err(ParseError::new(
                    "setting set takes <area.key> <value>",
                    "e.g. setting set playback.autoplay-next off",
                ));
            }
            Ok(CliCommand::SettingSet {
                key: rest[0].clone(),
                value: rest[1].clone(),
            })
        }
        other => Err(ParseError::new(
            format!("unknown setting action `{other}`"),
            "usage: lumen setting get <area.key> | setting set <area.key> <value>",
        )),
    }
}

/// One seek target token (chapter handled by the caller).
fn parse_seek_target(token: &str) -> Result<SeekArg, ParseError> {
    let hint = "usage: lumen seek 90s | +10s | -1m | 50% | chapter 3";
    if let Some(percent) = token.strip_suffix('%') {
        let value: f64 = parse_f64(percent)
            .ok_or_else(|| ParseError::new(format!("bad percent `{token}`"), hint))?;
        if !(0.0..=100.0).contains(&value) {
            return Err(ParseError::new(
                format!("percent {value} outside 0..100"),
                hint,
            ));
        }
        return Ok(SeekArg::Percent {
            fraction: (value / 100.0) as f32,
        });
    }
    if let Some(signed) = token.strip_prefix('+') {
        let ms = parse_duration_ms(signed)
            .ok_or_else(|| ParseError::new(format!("bad relative time `{token}`"), hint))?;
        return Ok(SeekArg::Relative { ms });
    }
    if let Some(signed) = token.strip_prefix('-') {
        let ms = parse_duration_ms(signed)
            .ok_or_else(|| ParseError::new(format!("bad relative time `{token}`"), hint))?;
        return Ok(SeekArg::Relative { ms: -ms });
    }
    let us = parse_time_us(token)
        .ok_or_else(|| ParseError::new(format!("bad time `{token}`"), hint))?;
    Ok(SeekArg::Absolute { us })
}

/// An absolute time: `90` (seconds), `1m30s`, `500ms`, `2h`, `1.5s`.
/// Non-negative; the host clamps against the duration (doc 05 §2.4 —
/// clamping is truth, not failure). Saturating at `i64::MAX/2` µs
/// (~146k years) so later ms↔µs arithmetic cannot overflow.
fn parse_time_us(token: &str) -> Option<i64> {
    let ms = parse_duration_ms(token)?;
    ms.checked_mul(1000).map(|us| us.min(i64::MAX / 2))
}

/// A duration in milliseconds from `<num><unit>*` runs
/// (`h`, `m`, `s`, `ms`, `us`; also bare `<num>` = seconds).
fn parse_duration_ms(token: &str) -> Option<i64> {
    if token.is_empty() {
        return None;
    }
    // Fast path: a bare number is seconds.
    if parse_f64(token).is_some() {
        let secs = parse_f64(token)?;
        let ms = secs.checked_mul(1000.0)?;
        if !(0.0..=9.0e12).contains(&ms) {
            return None;
        }
        return Some(ms as i64);
    }
    let mut total_us: i128 = 0;
    let mut rest = token;
    while !rest.is_empty() {
        let split = rest
            .find(|c: char| c.is_ascii_alphabetic())
            .unwrap_or(rest.len());
        if split == 0 {
            return None; // unit with no number, or stray letters
        }
        let (number, tail) = rest.split_at(split);
        let value = parse_f64(number)?;
        let (unit_len, mult_us): (usize, f64) = if tail.starts_with("ms") {
            (2, 1000.0)
        } else if tail.starts_with("us") {
            (2, 1.0)
        } else if tail.starts_with('s') {
            (1, 1.0e6)
        } else if tail.starts_with('m') {
            (1, 60.0e6)
        } else if tail.starts_with('h') {
            (1, 3600.0e6)
        } else {
            return None;
        };
        total_us += (value * mult_us) as i128;
        rest = &tail[unit_len..];
    }
    let ms = total_us / 1000;
    if ms > i64::MAX as i128 / 2 {
        return Some(i64::MAX / 2);
    }
    i64::try_from(ms).ok()
}

fn parse_f64(token: &str) -> Option<f64> {
    if token.is_empty() || token.contains([':', ' ']) {
        return None;
    }
    token.parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0)
}

fn parse_number<T: std::str::FromStr>(token: &str) -> Option<T> {
    token.parse::<T>().ok()
}

/// Build `session.open`'s catalog payload from a parsed [`CliCommand::Open`].
#[must_use]
pub fn open_command(source: &str, start_time: Option<i64>, start_paused: bool, no_video: bool, no_audio: bool) -> commands::Command {
    commands::Command::SessionOpen {
        source: SourceSpec::File {
            path: std::path::PathBuf::from(source),
        },
        options: OpenOptions {
            start_time: start_time.map(Timestamp),
            start_paused,
            rate: None,
            no_video,
            no_audio,
            video_track: None,
            audio_track: None,
            subtitle_track: None,
            forced_subtitle_only: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn seek_grammar_covers_the_doc_table() {
        // `90s`, `+10s`, `-1m`, `50%`, `chapter 3` — doc 05 §5.3.
        assert_eq!(
            parse_command(&words("seek 90s")),
            Ok(CliCommand::Seek(SeekArg::Absolute { us: 90_000_000 }))
        );
        assert_eq!(
            parse_command(&words("seek +10s")),
            Ok(CliCommand::Seek(SeekArg::Relative { ms: 10_000 }))
        );
        assert_eq!(
            parse_command(&words("seek -1m")),
            Ok(CliCommand::Seek(SeekArg::Relative { ms: -60_000 }))
        );
        match parse_command(&words("seek 50%")) {
            Ok(CliCommand::Seek(SeekArg::Percent { fraction })) => {
                assert!((fraction - 0.5).abs() < 1e-6);
            }
            other => panic!("percent parse: {other:?}"),
        }
        assert_eq!(
            parse_command(&words("seek chapter 3")),
            Ok(CliCommand::Seek(SeekArg::Chapter { index: 3 }))
        );
    }

    #[test]
    fn time_grammar_composites_and_decimals() {
        assert_eq!(parse_time_us("1m30s"), Some(90_000_000));
        assert_eq!(parse_time_us("500ms"), Some(500_000));
        assert_eq!(parse_time_us("2h"), Some(7_200_000_000));
        assert_eq!(parse_time_us("1.5s"), Some(1_500_000));
        assert_eq!(parse_time_us("750us"), Some(750));
        assert_eq!(parse_time_us("90"), Some(90_000_000));
        assert_eq!(parse_time_us("1m30"), None, "bare number after unit is not seconds-seconds");
        assert_eq!(parse_time_us("-5s"), None, "negatives are relative-only");
        assert_eq!(parse_time_us(""), None);
        assert_eq!(parse_time_us("s"), None);
    }

    #[test]
    fn percent_bounds_are_enforced() {
        assert!(parse_command(&words("seek 101%")).is_err());
        assert!(parse_command(&words("seek -5%")).is_err());
        assert!(parse_command(&words("seek 100%")).is_ok());
        assert!(parse_command(&words("seek 0%")).is_ok());
    }

    #[test]
    fn open_flags_parse() {
        match parse_command(&words("open clip.avi --start-time 30s --no-audio")) {
            Ok(CliCommand::Open { source, start_time, start_paused, no_video, no_audio }) => {
                assert_eq!(source, "clip.avi");
                assert_eq!(start_time, Some(30_000_000));
                assert!(!start_paused);
                assert!(!no_video);
                assert!(no_audio);
            }
            other => panic!("open parse: {other:?}"),
        }
        match parse_command(&words("open x.avi --start-paused")) {
            Ok(CliCommand::Open { start_paused, .. }) => assert!(start_paused),
            other => panic!("open parse: {other:?}"),
        }
        assert!(parse_command(&words("open")).is_err(), "source is required");
        assert!(parse_command(&words("open a b")).is_err(), "one source only");
    }

    #[test]
    fn no_arg_verbs_and_unknowns() {
        for verb in ["play", "pause", "toggle", "stop", "close", "info", "time", "duration", "tracks", "engine-info", "commands", "quit", "help"] {
            assert!(parse_command(&words(verb)).is_ok(), "{verb}");
            assert!(parse_command(&words(&format!("{verb} extra"))).is_err(), "{verb} takes no args");
        }
        assert!(parse_command(&words("bogus")).is_err());
        assert!(parse_command(&words("")).is_err());
        assert_eq!(parse_command(&words("q")), Ok(CliCommand::Quit));
    }

    #[test]
    fn setting_grammar() {
        assert_eq!(
            parse_command(&words("setting get playback.autoplay-next")),
            Ok(CliCommand::SettingGet { key: "playback.autoplay-next".to_string() })
        );
        assert_eq!(
            parse_command(&words("setting set engine.log-level debug")),
            Ok(CliCommand::SettingSet {
                key: "engine.log-level".to_string(),
                value: "debug".to_string()
            })
        );
        assert!(parse_command(&words("setting")).is_err());
        assert!(parse_command(&words("setting bogus k")).is_err());
    }

    #[test]
    fn argv_global_flags_and_help_areas() {
        let invocation = parse_argv(&words("--json open clip.avi")).ok();
        assert!(matches!(invocation, Some(i) if i.json && i.command.is_some()));
        let invocation = parse_argv(&words("--help engine")).ok();
        assert!(matches!(invocation, Some(i) if i.help == Some(Some("engine".to_string())) && i.command.is_none()));
        let invocation = parse_argv(&words("--version")).ok();
        assert!(matches!(invocation, Some(i) if i.version));
        let invocation = parse_argv(&words("--help open clip.avi")).ok();
        // --help wins over a command (doc convention).
        assert!(matches!(invocation, Some(i) if i.help.is_some() && i.command.is_none()));
        let invocation = parse_argv(&words("--control-stdio")).ok();
        assert!(matches!(invocation, Some(i) if i.control_stdio && i.command.is_none()));
    }
}
