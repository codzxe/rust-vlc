//! `--help` and `--version` generation (card 2.D2).
//!
//! **The card's requirement**: "`--help` generated from settings-schema"
//! (doc 05 §5.3: "`--help` text for settings-derived flags is generated
//! from the settings schema per MP §5"; doc 01 §2.8 apps/cli: "Flags
//! and `--help` are generated from `settings-schema`").
//!
//! The mechanism (honest interpretation, recorded): the settings-schema
//! crate owns generator **G2** (`GenTarget::CliHelp` → the committed
//! golden `schemas/generated/settings-help.txt`), and embeds the
//! authored schema (`schemas/settings.toml`) at compile time
//! (`runtime::embedded`). This module calls G2 over the embedded
//! schema — so the CLI's help is *generated from the same schema
//! source* the xtask golden is, with zero duplication of flag
//! metadata. The committed artifact doubles as the golden: the test
//! suite pins this module's output against it, so schema edits that
//! are not regenerated fail the CLI gate too.
//!
//! The help text follows the G2 artifact's own documented shape: a
//! usage/synopsis header (this module: the command catalog's playing
//! subset), then `lumen --help` lists **areas with counts**;
//! `lumen --help <area>` lists that area's flags — the VLC
//! `--longhelp` pattern, generated (doc 07 §5.3).
//!
//! # What is NOT generated here
//!
//! The subcommand synopsis is rendered from the command catalog *data*
//! (the served subset — see [`crate::dispatch`]), not a second table.
//! Product identity (`product.toml` name/logo) is not wired in v0 —
//! `--version` reports the crate version and says so `[PROPOSAL:
//! product-config joins with the rebrand-propagation card]`.

use commands::CommandId;
use settings_schema::{GenTarget, generate_str};

/// The served subcommand rows: (verb, catalog id, one-line help).
/// Source of truth for both `--help` and the REPL `help` verb; the
/// ids are cross-checked against the dispatch table by test.
pub const SUBCOMMANDS: &[(&str, &str, &str)] = &[
    ("open <path> [--start-time T] [--start-paused] [--no-video] [--no-audio]", "session.open", "open a media file (starts the interactive session)"),
    ("close", "session.close", "close the session"),
    ("play", "playback.play", "play / resume"),
    ("pause", "playback.pause", "pause"),
    ("toggle", "playback.toggle-play-pause", "toggle play/pause"),
    ("stop", "playback.stop", "stop playback (the session survives)"),
    ("seek 90s | +10s | -1m | 50% | chapter 3", "playback.seek", "seek (v0: reopen at the target through the engine's real transitions)"),
    ("info", "media.get-info", "the media snapshot (container, duration, tracks)"),
    ("time", "media.get-current-time", "current position"),
    ("duration", "media.get-duration", "media duration"),
    ("tracks", "tracks.list", "the track list"),
    ("engine-info", "system.get-engine-info", "engine self-description"),
    ("commands", "system.list-commands", "the full command catalog (introspection)"),
    ("setting get <area.key> | setting set <area.key> <value>", "config.get-setting / config.set-setting", "read/write a settings-schema key"),
    ("advance [ticks]", "(host verb)", "pump the engine N ticks — deterministic, no wall clock"),
    ("help", "(host verb)", "this command list"),
    ("quit | q", "(host verb)", "leave the interactive session"),
];

/// The `--version` line.
#[must_use]
pub fn version_text() -> String {
    // [PROPOSAL: product.toml identity (name/brand) joins with the
    // rebrand-propagation wiring; v0 reports the crate's own version.]
    format!("lumen {}\n", env!("CARGO_PKG_VERSION"))
}

/// The full `--help` text: synopsis + the generated areas index.
///
/// # Errors
/// [`String`] (an honest sentence) when the embedded schema is invalid
/// — a build-breaking state surfaced loudly rather than half-printed.
pub fn help_text() -> Result<String, String> {
    let generated = generated_help()?;
    let mut out = String::new();
    out.push_str(&version_text());
    out.push_str("The Lumen command-line player — headless host of the engine's first\n");
    out.push_str("vertical, driven through the one command layer (doc 05 §5.3).\n\n");
    out.push_str("Usage:\n");
    out.push_str("  lumen [OPTIONS] <COMMAND> [ARGS]     one-shot / interactive\n");
    out.push_str("  lumen --control-stdio                NDJSON machine mode\n");
    out.push_str("  lumen --help [AREA]                  this help / one settings area\n\n");
    out.push_str("Commands (the playing subset of the command catalog):\n");
    for (verb, id, help) in SUBCOMMANDS {
        out.push_str(&format!("  {verb:<58} {id}\n      {help}\n"));
    }
    out.push_str("\nSettings-derived flags (--<area>-<key> <value>):\n");
    out.push_str("  `lumen --help` lists areas with counts; `lumen --help <area>` lists that\n");
    out.push_str("  area's flags (the VLC --longhelp pattern, generated).\n");
    out.push_str("  Reserved (non-settings) flags: -h/--help, -v/--version, --json,\n");
    out.push_str("  --control-stdio.\n\n");
    out.push_str(&areas_block(&generated));
    Ok(out)
}

/// The `--help <area>` text: the synopsis header + that area's
/// generated flag section.
///
/// # Errors
/// [`String`] naming the known areas when `area` is not one of them.
pub fn help_for_area(area: &str) -> Result<String, String> {
    let generated = generated_help()?;
    let Some(section) = area_section(&generated, area) else {
        let known = known_areas(&generated).join(", ");
        return Err(format!(
            "unknown settings area `{area}` — known areas: {known}"
        ));
    };
    let mut out = String::new();
    out.push_str(&version_text());
    out.push_str("Settings area `");
    out.push_str(area);
    out.push_str("` (generated from schemas/settings.toml — the G2 golden text).\n\n");
    out.push_str(&section);
    Ok(out)
}

/// The G2 generator over the embedded schema (the single call every
/// help path funnels through).
fn generated_help() -> Result<String, String> {
    let schema = settings_schema::runtime::embedded()
        .map_err(|err| format!("the embedded settings schema is invalid: {err}"))?;
    Ok(generate_str(schema, GenTarget::CliHelp))
}

/// The `Areas:` index block of the generated text (from the `Areas:`
/// marker to the blank line that ends the block).
#[must_use]
pub fn areas_block(generated: &str) -> String {
    let mut out = String::from("Areas:\n");
    let mut in_block = false;
    for line in generated.lines() {
        if !in_block {
            in_block = line == "Areas:";
            continue;
        }
        if line.is_empty() {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// One area's flag section: from the area's header line (the bare
/// `screen` or `screen (annotation)` line) to the next area header or
/// the end — the G2 emitter's own section shape.
#[must_use]
pub fn area_section(generated: &str, area: &str) -> Option<String> {
    let areas = known_areas(generated);
    if !areas.iter().any(|known| known == area) {
        return None;
    }
    let header_exact = format!("{area}\n");
    let header_annotated = format!("{area} (");
    let mut out = String::new();
    let mut collecting = false;
    for line in generated.lines() {
        let is_header = line == area
            || (line.starts_with(&header_annotated) && line.ends_with(')'));
        if collecting {
            // The next area's header ends this section.
            if areas
                .iter()
                .any(|other| other != area && (line == other.as_str() || (line.starts_with(&format!("{other} (")) && line.ends_with(')'))))
            {
                break;
            }
            out.push_str(line);
            out.push('\n');
        } else if is_header || format!("{line}\n") == header_exact {
            collecting = true;
            out.push_str(line);
            out.push('\n');
        }
    }
    if collecting {
        while out.ends_with("\n\n") {
            out.pop();
        }
        Some(out)
    } else {
        None
    }
}

/// The known area ids, parsed from the generated `Areas:` block.
#[must_use]
pub fn known_areas(generated: &str) -> Vec<String> {
    let mut areas = Vec::new();
    let mut in_block = false;
    for line in generated.lines() {
        if !in_block {
            in_block = line == "Areas:";
            continue;
        }
        if line.is_empty() {
            break;
        }
        // Rows: "  <area>  <count> options[  (annotation)]".
        let trimmed = line.trim_start();
        if let Some((name, _rest)) = trimmed.split_once(char::is_whitespace) {
            areas.push(name.to_string());
        }
    }
    areas
}

/// The catalog ids the synopsis claims — the dispatch table's served
/// set is tested against this (see `dispatch::SERVED`). The settings
/// row names two ids (get and set share one synopsis line).
#[must_use]
pub fn synopsis_catalog_ids() -> Vec<CommandId> {
    SUBCOMMANDS
        .iter()
        .flat_map(|(_, id, _)| catalog_ids(id))
        .collect()
}

/// Parse a synopsis id string into the catalog enum(s) it names.
fn catalog_ids(id: &str) -> Vec<CommandId> {
    match id {
        "session.open" => vec![CommandId::SessionOpen],
        "session.close" => vec![CommandId::SessionClose],
        "playback.play" => vec![CommandId::PlaybackPlay],
        "playback.pause" => vec![CommandId::PlaybackPause],
        "playback.toggle-play-pause" => vec![CommandId::PlaybackTogglePlayPause],
        "playback.stop" => vec![CommandId::PlaybackStop],
        "playback.seek" => vec![CommandId::PlaybackSeek],
        "media.get-info" => vec![CommandId::MediaGetInfo],
        "media.get-current-time" => vec![CommandId::MediaGetCurrentTime],
        "media.get-duration" => vec![CommandId::MediaGetDuration],
        "tracks.list" => vec![CommandId::TracksList],
        "system.get-engine-info" => vec![CommandId::SystemGetEngineInfo],
        "system.list-commands" => vec![CommandId::SystemListCommands],
        "config.get-setting / config.set-setting" => {
            vec![CommandId::ConfigGetSetting, CommandId::ConfigSetSetting]
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The embedded schema's G2 output IS the committed golden's byte
    /// stream (drift-tie: editing settings.toml without regenerating
    /// fails here AND the xtask `schema-gen --check`).
    #[test]
    fn generated_help_matches_the_committed_artifact() {
        let artifact = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../schemas/generated/settings-help.txt"
        ))
        .unwrap_or_default();
        assert!(!artifact.is_empty(), "the committed G2 artifact is missing");
        let generated = match generated_help() {
            Ok(text) => text,
            Err(err) => panic!("{err}"),
        };
        assert_eq!(generated, artifact);
    }

    #[test]
    fn areas_block_lists_the_six_areas() {
        let generated = match generated_help() {
            Ok(text) => text,
            Err(err) => panic!("{err}"),
        };
        let areas = known_areas(&generated);
        assert_eq!(
            areas,
            vec![
                "engine", "global", "interface", "performance", "playback", "privacy"
            ]
        );
        let block = areas_block(&generated);
        assert!(block.starts_with("Areas:\n  engine"));
        assert!(block.contains("6 options"));
    }

    #[test]
    fn area_section_slices_engine_and_rejects_unknown() {
        let generated = match generated_help() {
            Ok(text) => text,
            Err(err) => panic!("{err}"),
        };
        let section = match area_section(&generated, "engine") {
            Some(text) => text,
            None => panic!("engine section missing"),
        };
        assert!(section.starts_with("engine (engine-internal: ui = false)"));
        assert!(section.contains("--engine-log-level <error|warn|info|debug|trace>"));
        assert!(section.contains("settings.engine.log-level"));
        // The section ends before the next area header.
        assert!(!section.contains("--global-language"));
        assert!(area_section(&generated, "no-such-area").is_none());
        // An area whose name prefixes another still slices exactly.
        let playback = match area_section(&generated, "playback") {
            Some(text) => text,
            None => panic!("playback section missing"),
        };
        assert!(playback.starts_with("playback\n"));
    }

    #[test]
    fn help_for_area_names_known_areas_on_error() {
        match help_for_area("bogus") {
            Err(message) => assert!(message.contains("playback"), "{message}"),
            Ok(_) => panic!("bogus area must be refused"),
        }
        let text = match help_for_area("global") {
            Ok(text) => text,
            Err(err) => panic!("{err}"),
        };
        assert!(text.contains("--global-language <en-US>"));
    }

    #[test]
    fn synopsis_ids_parse_and_cover_the_served_set() {
        let ids = synopsis_catalog_ids();
        assert_eq!(ids.len(), 15, "one per catalog-serving synopsis row (the settings row counts twice)");
        // Distinct by construction of the table; spot-pin two.
        assert!(ids.contains(&CommandId::SessionOpen));
        assert!(ids.contains(&CommandId::PlaybackSeek));
        assert!(ids.contains(&CommandId::ConfigSetSetting));
    }
}
