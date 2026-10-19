//! The doc 07 §3 fixture matrix: every fixture in `tests/fixtures/` must be
//! rejected with the documented key path, rule, expected-vs-found and line
//! (card 1.B1 AC). See `tests/fixtures/README.md` for the fixture ↔ rule map.

mod common;

use common::{for_key, load_fixture, marker_line, read_fixture};
use product_config::{Stage, render_all};

/// Load `name`, insist it is rejected, return its diagnostics.
fn rejected(name: &str) -> Vec<product_config::Diagnostic> {
    match load_fixture(name) {
        Ok(_) => unreachable!("{name} must be rejected"),
        Err(diags) => diags,
    }
}

fn one<'a>(
    diags: &'a [product_config::Diagnostic],
    key: &str,
    name: &str,
) -> &'a product_config::Diagnostic {
    let hits = for_key(diags, key);
    assert_eq!(
        hits.len(),
        1,
        "{name}: expected exactly one {key} diagnostic\n{}",
        render_all(diags)
    );
    match hits.first() {
        Some(d) => d,
        None => unreachable!("{name}: no {key} diagnostic"),
    }
}

// --- §3.4 stage 1: syntax ------------------------------------------------------

#[test]
fn invalid_syntax_is_a_parse_diagnostic_with_line() {
    let name = "invalid-syntax.toml";
    let src = read_fixture(name);
    let diags = rejected(name);
    let d = one(&diags, "<document>", name);
    assert_eq!(d.stage, Stage::Parse);
    assert_eq!(d.line, marker_line(&src, "width = "));
    assert!(d.rule.contains("well-formed TOML"), "{}", d);
}

// --- §3.3 required fields --------------------------------------------------------

#[test]
fn missing_identity_section_is_reported() {
    let name = "missing-section-identity.toml";
    let diags = rejected(name);
    let d = one(&diags, "identity", name);
    assert!(d.rule.contains("required key is missing"), "{}", d);
    assert_eq!(d.found, "absent");
    // A wholly missing top-level table has no line to point at.
    assert!(d.line.is_none(), "{}", d);
}

#[test]
fn missing_identity_field_points_at_the_table_header() {
    let name = "missing-identity-field.toml";
    let src = read_fixture(name);
    let diags = rejected(name);
    let d = one(&diags, "identity.description_key", name);
    assert!(d.rule.contains("required key is missing"), "{}", d);
    assert_eq!(d.line, marker_line(&src, "[identity]"), "{}", d);
}

#[test]
fn missing_window_fields_reports_every_violation() {
    let name = "missing-window-fields.toml";
    let src = read_fixture(name);
    let diags = rejected(name);
    // doc 07 §3.4 stage 2: report every violation, not just the first.
    assert_eq!(for_key(&diags, "window.title_template").len(), 1);
    assert_eq!(for_key(&diags, "window.title_idle_key").len(), 1);
    let header = marker_line(&src, "[window]");
    for key in ["window.title_template", "window.title_idle_key"] {
        let d = one(&diags, key, name);
        assert_eq!(d.line, header, "{}", d);
    }
}

#[test]
fn missing_brand_asset_is_reported() {
    let name = "missing-brand-asset.toml";
    let diags = rejected(name);
    let d = one(&diags, "brand.icon_png_512", name);
    assert!(d.rule.contains("required key is missing"), "{}", d);
}

#[test]
fn missing_defaults_key_is_reported() {
    let name = "missing-defaults-key.toml";
    let diags = rejected(name);
    let d = one(&diags, "defaults.locale", name);
    assert!(d.rule.contains("required key is missing"), "{}", d);
}

#[test]
fn missing_limits_key_is_reported() {
    let name = "missing-limits-key.toml";
    let diags = rejected(name);
    let d = one(&diags, "limits.max_open_sessions", name);
    assert!(d.rule.contains("required key is missing"), "{}", d);
}

// --- §3.3 enums -------------------------------------------------------------------

#[test]
fn enum_subtitle_mode_is_rejected_with_the_legal_values() {
    let name = "enum-subtitle-mode.toml";
    let src = read_fixture(name);
    let diags = rejected(name);
    let d = one(&diags, "playback.subtitle_mode", name);
    assert!(
        d.rule
            .contains("must be one of: auto, forced-only, on, off"),
        "{}",
        d
    );
    assert_eq!(d.found, "\"always\"");
    assert_eq!(
        d.line,
        marker_line(&src, "subtitle_mode = \"always\""),
        "{}",
        d
    );
}

/// ADR-001 was accepted 2026-10-19 (docs/adr/ADR-001-hardware-decode.md):
/// hw-first H.264/HEVC via OS decoders is the accepted policy, so the enum
/// carries its full pre-registered vocabulary (doc 07 §2.8 growth note:
/// "software-only" | "prefer-hardware" | "hardware-only"). The fixture
/// uses "hw-first" — the ADR's feature-matrix row term — which must stay
/// rejected: matrix vocabulary is not config-enum vocabulary.
#[test]
fn enum_hw_decode_policy_is_rejected_with_the_post_adr001_legal_values() {
    let name = "enum-hw-decode-policy.toml";
    let src = read_fixture(name);
    let diags = rejected(name);
    let d = one(&diags, "playback.hw_decode_policy", name);
    assert!(
        d.rule
            .contains("must be one of: software-only, prefer-hardware, hardware-only"),
        "{}",
        d
    );
    assert_eq!(d.found, "\"hw-first\"");
    assert_eq!(
        d.line,
        marker_line(&src, "hw_decode_policy = \"hw-first\""),
        "{}",
        d
    );
}

#[test]
fn enum_crash_reports_is_rejected() {
    let name = "enum-crash-reports.toml";
    let diags = rejected(name);
    let d = one(&diags, "telemetry.crash_reports", name);
    assert!(d.rule.contains("must be one of: off, ask, send"), "{}", d);
    assert_eq!(d.found, "\"always\"");
}

// --- §3.3 ranges --------------------------------------------------------------------

#[test]
fn range_window_width_is_rejected_with_line() {
    let name = "range-window-width.toml";
    let src = read_fixture(name);
    let diags = rejected(name);
    let d = one(&diags, "window.width", name);
    assert!(d.rule.contains("must be >= 480"), "{}", d);
    assert!(d.rule.contains("<= 3840"), "{}", d);
    assert_eq!(d.found, "100");
    assert!(d.expected.contains("480..=3840"), "{}", d);
    assert_eq!(d.line, marker_line(&src, "width = 100"), "{}", d);
}

#[test]
fn range_default_rate_is_rejected() {
    let name = "range-default-rate.toml";
    let diags = rejected(name);
    let d = one(&diags, "playback.default_rate", name);
    assert!(d.rule.contains("must be >= 0.25"), "{}", d);
    assert!(d.rule.contains("<= 4"), "{}", d);
    assert_eq!(d.found, "10");
}

#[test]
fn range_limits_multi_reports_all_four_violations() {
    let name = "range-limits-multi.toml";
    let src = read_fixture(name);
    let diags = rejected(name);
    for (key, bound) in [
        ("limits.cache_demux_mib", ">= 4"),
        ("limits.queue_packets_audio", "<= 8192"),
        ("limits.max_playlist_items", ">= 100"),
        ("limits.max_wasm_extension_memory_mib", "<= 256"),
    ] {
        let d = one(&diags, key, name);
        assert!(d.rule.contains(bound), "{}", d);
        assert!(d.line.is_some(), "{}", d);
    }
    let _ = src;
}

// --- §3.3 types ----------------------------------------------------------------------

#[test]
fn type_window_width_is_rejected() {
    let name = "type-window-width.toml";
    let diags = rejected(name);
    let d = one(&diags, "window.width", name);
    assert!(d.rule.contains("must be of type integer"), "{}", d);
    assert_eq!(d.found, "\"wide\"");
}

#[test]
fn type_features_value_is_rejected() {
    let name = "type-features-value.toml";
    let diags = rejected(name);
    let d = one(&diags, "features.core-playback", name);
    assert!(d.rule.contains("must be of type boolean"), "{}", d);
    assert_eq!(d.found, "\"yes\"");
}

// --- §3.3 patterns --------------------------------------------------------------------

#[test]
fn pattern_short_name_is_rejected() {
    let name = "pattern-short-name.toml";
    let diags = rejected(name);
    let d = one(&diags, "identity.short_name", name);
    assert!(d.rule.contains("must match the pattern"), "{}", d);
    assert_eq!(d.found, "\"Lumen Player\"");
}

#[test]
fn pattern_display_version_is_rejected() {
    let name = "pattern-display-version.toml";
    let diags = rejected(name);
    let d = one(&diags, "identity.display_version", name);
    assert!(d.rule.contains("must match the pattern"), "{}", d);
    assert_eq!(d.found, "\"version one\"");
}

#[test]
fn pattern_accent_color_is_rejected() {
    let name = "pattern-accent-color.toml";
    let diags = rejected(name);
    let d = one(&diags, "brand.accent_color", name);
    assert!(d.rule.contains("#"), "{}", d);
    assert_eq!(d.found, "\"4f8cff\"");
}

#[test]
fn pattern_reverse_dns_is_rejected() {
    let name = "pattern-reverse-dns.toml";
    let diags = rejected(name);
    let d = one(&diags, "identity.ids.reverse_dns", name);
    assert!(d.rule.contains("must match the pattern"), "{}", d);
    assert_eq!(d.found, "\"Dev.Lumen\"");
}

#[test]
fn pattern_guid_is_rejected() {
    let name = "pattern-guid.toml";
    let diags = rejected(name);
    let d = one(&diags, "identity.ids.windows.msi_upgrade_code", name);
    assert!(d.rule.contains("must match the pattern"), "{}", d);
    assert_eq!(d.found, "\"not-a-guid\"");
}

#[test]
fn pattern_title_template_unknown_variable_is_rejected() {
    let name = "pattern-title-template.toml";
    let src = read_fixture(name);
    let diags = rejected(name);
    let d = one(&diags, "window.title_template", name);
    assert!(d.rule.contains("must match the pattern"), "{}", d);
    assert!(d.hint.contains("{product_name}"), "{}", d);
    assert_eq!(
        d.line,
        marker_line(&src, "title_template = \"{unknown_var}"),
        "{}",
        d
    );
}

#[test]
fn pattern_msgid_description_key_is_rejected() {
    let name = "pattern-msgid.toml";
    let diags = rejected(name);
    let d = one(&diags, "identity.description_key", name);
    assert!(d.rule.contains("must match the pattern"), "{}", d);
    assert_eq!(d.found, "\"Product Description\"");
}

#[test]
fn pattern_https_url_is_rejected() {
    let name = "pattern-urls.toml";
    let diags = rejected(name);
    let d = one(&diags, "urls.help", name);
    assert!(d.rule.contains("https"), "{}", d);
    assert!(d.found.starts_with("\"http://"), "{}", d);
}

#[test]
fn pattern_bcp47_language_tag_is_rejected() {
    let name = "pattern-bcp47.toml";
    let src = read_fixture(name);
    let diags = rejected(name);
    let d = one(&diags, "playback.preferred_audio_languages.0", name);
    assert!(d.rule.contains("must match the pattern"), "{}", d);
    assert_eq!(d.found, "\"english\"");
    assert_eq!(
        d.line,
        marker_line(&src, "preferred_audio_languages = [\"english\"]"),
        "{}",
        d
    );
}

// --- §3.3 unique-identity (a): reserved names --------------------------------------------

#[test]
fn reserved_name_is_rejected() {
    let name = "reserved-name.toml";
    let src = read_fixture(name);
    let diags = rejected(name);
    let d = one(&diags, "identity.name", name);
    assert!(d.rule.contains("reserved"), "{}", d);
    assert!(d.rule.contains("VLC"), "{}", d);
    assert_eq!(d.found, "\"VLC\"");
    assert_eq!(d.line, marker_line(&src, "name = \"VLC\""), "{}", d);
}

// --- §3.5 (4): unknown keys + typo detection ------------------------------------------------

#[test]
fn unknown_key_typo_gets_a_did_you_mean_hint() {
    let name = "unknown-key-typo.toml";
    let src = read_fixture(name);
    let diags = rejected(name);
    // Two diagnostics: the unknown key itself, and the now-missing required
    // key (every violation is reported).
    let d = one(&diags, "limits.cache_demux_mb", name);
    assert!(d.rule.contains("unknown key"), "{}", d);
    assert!(
        d.hint.contains("did you mean `cache_demux_mib`"),
        "hint: {}",
        d.hint
    );
    assert_eq!(d.line, marker_line(&src, "cache_demux_mb = 32"), "{}", d);
    let missing = one(&diags, "limits.cache_demux_mib", name);
    assert!(
        missing.rule.contains("required key is missing"),
        "{}",
        missing
    );
}

#[test]
fn unknown_top_level_section_is_rejected() {
    let name = "unknown-top-level.toml";
    let src = read_fixture(name);
    let diags = rejected(name);
    let d = one(&diags, "extra", name);
    assert!(d.rule.contains("unknown key"), "{}", d);
    assert!(d.hint.contains("schema_version"), "{}", d);
    assert_eq!(d.line, marker_line(&src, "[extra]"), "{}", d);
}
