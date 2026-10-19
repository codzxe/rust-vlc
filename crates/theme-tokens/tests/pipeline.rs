//! Library-surface pipeline tests (card 1.B4; doc 08 §3): parse/validate
//! the committed token source, spot-check resolved values against doc 08
//! §1.2, and prove emission determinism.

use std::fs;
use std::path::PathBuf;

use theme_tokens::color::Rgb8;
use theme_tokens::emit::emit_css;
use theme_tokens::emit::emit_rust;
use theme_tokens::model::ModeValues;
use theme_tokens::model::ResolvedValue;
use theme_tokens::parse;

/// The Lumen workspace root (this file lives in crates/theme-tokens/tests).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn committed_source() -> Result<String, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(
        workspace_root().join("themes/tokens.json"),
    )?)
}

fn committed_set() -> Result<theme_tokens::TokenSet, Box<dyn std::error::Error>> {
    Ok(parse(&committed_source()?)?)
}

fn color_of(
    set: &theme_tokens::TokenSet,
    css_name: &str,
    mode: Option<&str>,
) -> Result<Rgb8, Box<dyn std::error::Error>> {
    let token = set
        .semantic_by_css_name(css_name)
        .ok_or_else(|| format!("missing semantic token `{css_name}`"))?;
    match token.values.value_for(mode) {
        Some(ResolvedValue::Color(c)) => Ok(*c),
        other => Err(format!("`{css_name}` is not a color: {other:?}").into()),
    }
}

#[test]
fn committed_source_parses_with_the_documented_shape() -> Result<(), Box<dyn std::error::Error>> {
    let set = committed_set()?;
    let counts = set.counts();
    assert_eq!(counts.primitives, 92, "primitive-layer token count");
    assert_eq!(counts.aliases, 70, "alias-layer token count");
    assert_eq!(counts.semantic, 70, "semantic-layer token count");
    assert_eq!(set.modes(), ["dark", "light"]);
    assert_eq!(set.default_mode(), "dark");
    assert_eq!(
        set.accents(),
        [
            "orange", "red", "yellow", "green", "cyan", "blue", "violet", "pink", "gray"
        ]
    );
    assert_eq!(set.default_accent(), "orange");
    // Doc 08 §3.3: exactly the six accent-family semantic tokens.
    assert_eq!(
        set.accent_family_css(),
        [
            "--color-accent",
            "--color-accent-bright",
            "--color-accent-icon",
            "--color-accent-strong",
            "--color-accent-tinted",
            "--color-content-on-accent",
        ]
    );
    assert_eq!(set.accent_ramps().len(), 9);
    Ok(())
}

#[test]
fn resolved_values_match_doc_08_section_1_2() -> Result<(), Box<dyn std::error::Error>> {
    let set = committed_set()?;
    // Dark neutral ramp anchors.
    assert_eq!(
        color_of(&set, "--color-surface-deep", Some("dark"))?,
        Rgb8::new(0x09, 0x09, 0x09)
    );
    assert_eq!(
        color_of(&set, "--color-surface", Some("dark"))?,
        Rgb8::new(0x19, 0x19, 0x19)
    );
    assert_eq!(
        color_of(&set, "--color-content-primary", Some("dark"))?,
        Rgb8::new(0x8f, 0x8f, 0x8f)
    );
    assert_eq!(
        color_of(&set, "--color-content-bright", Some("dark"))?,
        Rgb8::new(0xea, 0xea, 0xea)
    );
    // Light table.
    assert_eq!(
        color_of(&set, "--color-surface", Some("light"))?,
        Rgb8::new(0xd5, 0xd5, 0xd5)
    );
    assert_eq!(
        color_of(&set, "--color-surface-deep", Some("light"))?,
        Rgb8::new(0xde, 0xde, 0xde)
    );
    assert_eq!(
        color_of(&set, "--color-content-primary", Some("light"))?,
        Rgb8::new(0x2e, 0x2e, 0x2e)
    );
    // Orange family + per-mode accent roles.
    assert_eq!(
        color_of(&set, "--color-accent", None)?,
        Rgb8::new(0xd3, 0x79, 0x18)
    );
    assert_eq!(
        color_of(&set, "--color-accent-strong", None)?,
        Rgb8::new(0xab, 0x63, 0x14)
    );
    assert_eq!(
        color_of(&set, "--color-accent-tinted", Some("dark"))?,
        Rgb8::new(0x37, 0x1f, 0x06)
    );
    assert_eq!(
        color_of(&set, "--color-accent-tinted", Some("light"))?,
        Rgb8::new(0xd8, 0xba, 0x9a)
    );
    assert_eq!(
        color_of(&set, "--color-accent-bright", Some("light"))?,
        Rgb8::new(0xe8, 0x85, 0x1a)
    );
    // Modeless tokens really are modeless.
    let record = set
        .semantic_by_css_name("--color-record")
        .ok_or("missing --color-record")?;
    assert!(matches!(record.values, ModeValues::Single(_)));
    Ok(())
}

#[test]
fn accent_ramps_follow_the_contrast_rule() -> Result<(), Box<dyn std::error::Error>> {
    let set = committed_set()?;
    // Default ramp carries the mined orange family.
    let orange = set.ramp_for("orange").ok_or("missing orange ramp")?;
    assert_eq!(orange.primary, Rgb8::new(0xd3, 0x79, 0x18));
    assert_eq!(orange.tinted_dark, Rgb8::new(0x37, 0x1f, 0x06));
    assert_eq!(orange.on_accent, Rgb8::new(0x16, 0x16, 0x16));
    // Every ramp's on-accent glyph meets doc 08 §3.3 (≥ 4.5:1).
    for ramp in set.accent_ramps() {
        let ratio = theme_tokens::color::contrast_ratio(ramp.on_accent, ramp.primary);
        assert!(
            ratio >= theme_tokens::color::MIN_ON_ACCENT_CONTRAST,
            "accent `{}` on-accent contrast {ratio:.2} < 4.5",
            ramp.id
        );
    }
    // Blue falls through to the brightest text, violet to the white fallback.
    assert_eq!(
        set.ramp_for("blue").ok_or("missing blue ramp")?.on_accent,
        Rgb8::new(0xea, 0xea, 0xea)
    );
    assert_eq!(
        set.ramp_for("violet")
            .ok_or("missing violet ramp")?
            .on_accent,
        Rgb8::new(0xff, 0xff, 0xff)
    );
    // Gray stays achromatic (doc 08 §1.2 gray swatch).
    let gray = set.ramp_for("gray").ok_or("missing gray ramp")?;
    assert_eq!(gray.strong.r, gray.strong.b);
    assert_eq!(gray.tinted_light.r, gray.tinted_light.b);
    Ok(())
}

#[test]
fn emission_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    let set = committed_set()?;
    let mut css_a = Vec::new();
    let mut css_b = Vec::new();
    let mut rust_a = Vec::new();
    let mut rust_b = Vec::new();
    emit_css(&set, &mut css_a)?;
    emit_css(&set, &mut css_b)?;
    emit_rust(&set, &mut rust_a)?;
    emit_rust(&set, &mut rust_b)?;
    assert_eq!(css_a, css_b, "CSS emission must be byte-stable");
    assert_eq!(rust_a, rust_b, "Rust emission must be byte-stable");
    Ok(())
}

#[test]
fn emitted_documents_have_the_documented_structure() -> Result<(), Box<dyn std::error::Error>> {
    let set = committed_set()?;
    let mut css = Vec::new();
    let mut rust = Vec::new();
    emit_css(&set, &mut css)?;
    emit_rust(&set, &mut rust)?;
    let css = String::from_utf8(css)?;
    let rust = String::from_utf8(rust)?;

    assert!(css.starts_with("/* GENERATED FILE — DO NOT EDIT."));
    assert!(css.contains(":root {\n"));
    assert!(css.contains(":root[data-theme=\"light\"] {\n"));
    for accent in [
        "red", "yellow", "green", "cyan", "blue", "violet", "pink", "gray",
    ] {
        assert!(
            css.contains(&format!(":root[data-accent=\"{accent}\"] {{\n")),
            "missing accent block for `{accent}`"
        );
        assert!(
            css.contains(&format!(
                ":root[data-theme=\"light\"][data-accent=\"{accent}\"] {{\n"
            )),
            "missing light-accent block for `{accent}`"
        );
    }
    // The default accent's values live in :root / the light block instead.
    assert!(!css.contains("data-accent=\"orange\""));
    // A representative declaration per layer concern (doc 08 §3.2 examples).
    for decl in [
        "--color-surface: #191919;",
        "--color-surface: #d5d5d5;",
        "--radius-control: 6px;",
        "--space-inset: 8px;",
        "--font-size-body: 14px;",
        "--shadow-overlay: 0px 4.76px 5.9px 0px rgba(0, 0, 0, 0.7);",
        "--blur-backdrop: 25px;",
    ] {
        assert!(css.contains(decl), "CSS output missing `{decl}`");
    }

    assert!(rust.starts_with("//! GENERATED FILE — DO NOT EDIT."));
    for line in [
        "pub const MODES: &[&str] = &[\"dark\", \"light\"];",
        "pub const DEFAULT_MODE: &str = \"dark\";",
        "pub const COLOR_SURFACE_DARK: Rgb8 = Rgb8::new(0x19, 0x19, 0x19);",
        "pub const COLOR_SURFACE_LIGHT: Rgb8 = Rgb8::new(0xd5, 0xd5, 0xd5);",
        "pub const RADIUS_CARD_LG: f32 = 30.0;",
        "pub const SPACE_TRACK_INSET: f32 = 2.0;",
        "pub const OPACITY_SWATCH_REST: f32 = 0.7;",
        "pub const FONT_FAMILY_SANS: &[&str] = &[\"Inter\", \"system-ui\", \"sans-serif\"];",
        "pub const ACCENT_ORANGE_PRIMARY: Rgb8 = Rgb8::new(0xd3, 0x79, 0x18);",
        "pub const ACCENT_VIOLET_ON_ACCENT: Rgb8 = Rgb8::new(0xff, 0xff, 0xff);",
        "pub const ACCENT_GRAY_TINTED_DARK: Rgb8 = Rgb8::new(0x1e, 0x1e, 0x1e);",
    ] {
        assert!(rust.contains(line), "Rust output missing `{line}`");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Validation errors (doc 08 §3.2/§3.4)
// ---------------------------------------------------------------------------

/// A minimal valid source to mutate; r## delimiters because the JSON
/// contains `"#…` sequences.
const MINI: &str = r##"{
  "$extensions": { "lumen.tokens": {
      "modes": ["dark", "light"],
      "default-mode": "dark",
      "accents": ["orange"],
      "default-accent": "orange",
      "accent-primaries": { "orange": "{primitive.color.orange.500}" },
      "on-accent-candidates": ["{primitive.color.gray.800}"]
  } },
  "primitive": { "color": { "$type": "color",
      "orange": { "500": { "$value": "#D37918" } },
      "gray": { "800": { "$value": "#161616" }, "25": { "$value": "#EAEAEA" } }
  } },
  "alias": { "color": { "$type": "color",
      "accent": { "$value": "{primitive.color.orange.500}" },
      "accent.bright": { "$value": { "dark": "{primitive.color.orange.500}", "light": "{primitive.color.gray.25}" } },
      "accent.icon": { "$value": "{primitive.color.orange.500}" },
      "accent.strong": { "$value": "{primitive.color.orange.500}" },
      "accent.tinted": { "$value": "{primitive.color.gray.800}" },
      "content.on-accent": { "$value": "{primitive.color.gray.800}" },
      "surface": { "$value": { "dark": "{primitive.color.gray.800}", "light": "{primitive.color.gray.25}" } }
  } },
  "semantic": { "color": { "$type": "color",
      "accent": { "$value": "{alias.color.accent}" },
      "accent.bright": { "$value": "{alias.color.accent.bright}" },
      "accent.icon": { "$value": "{alias.color.accent.icon}" },
      "accent.strong": { "$value": "{alias.color.accent.strong}" },
      "accent.tinted": { "$value": "{alias.color.accent.tinted}" },
      "content.on-accent": { "$value": "{alias.color.content.on-accent}" },
      "surface": { "$value": "{alias.color.surface}" }
  } }
}
"##;

/// Assert that an already-mutated MINI source fails to parse with a
/// diagnostic containing `needle`.
fn expect_error(source: &str, needle: &str) -> Result<(), Box<dyn std::error::Error>> {
    let err = parse(source).err().ok_or("expected a parse error")?;
    assert!(
        err.to_string().contains(needle),
        "error `{err}` does not contain `{needle}`"
    );
    Ok(())
}

#[test]
fn mini_source_is_valid_and_counts_survive() -> Result<(), Box<dyn std::error::Error>> {
    let set = parse(MINI)?;
    assert_eq!(set.counts().semantic, 7);
    assert_eq!(set.counts().primitives, 3);
    // MINI defines exactly 7 alias `$value`s (accent, accent.bright,
    // accent.icon, accent.strong, accent.tinted, content.on-accent,
    // surface); the parser counts one alias per `$value`, so 7 — not 8.
    assert_eq!(set.counts().aliases, 7);
    Ok(())
}

#[test]
fn layer_rules_are_enforced() -> Result<(), Box<dyn std::error::Error>> {
    // Semantic may not skip the alias layer (doc 08 §3.2).
    expect_error(
        &MINI.replacen(
            "\"{alias.color.accent}\"",
            "\"{primitive.color.orange.500}\"",
            1,
        ),
        "must resolve through the alias layer",
    )?;
    // Primitives must be raw.
    expect_error(
        &MINI.replacen(
            "\"orange\": { \"500\": { \"$value\": \"#D37918\" } }",
            "\"orange\": { \"500\": { \"$value\": \"{primitive.color.gray.800}\" } }",
            1,
        ),
        "primitives must be raw values",
    )?;
    // Missing layer group.
    expect_error(
        &MINI.replacen("\"alias\": {", "\"aliass\": {", 1),
        "missing required layer group",
    )?;
    Ok(())
}

#[test]
fn reference_errors_are_actionable() -> Result<(), Box<dyn std::error::Error>> {
    // Unknown reference target.
    expect_error(
        &MINI.replacen(
            "\"surface\": { \"$value\": { \"dark\": \"{primitive.color.gray.800}\"",
            "\"surface\": { \"$value\": { \"dark\": \"{primitive.color.gray.999}\"",
            1,
        ),
        "references unknown token `primitive.color.gray.999`",
    )?;
    // Reference cycle alias.a -> alias.b -> alias.a.
    expect_error(
        &MINI
            .replacen(
                "\"accent.strong\": { \"$value\": \"{primitive.color.orange.500}\" }",
                "\"accent.strong\": { \"$value\": \"{alias.color.accent.verystrong}\" },\n      \"accent.verystrong\": { \"$value\": \"{alias.color.accent.strong}\" }",
                1,
            ),
        "reference cycle",
    )?;
    Ok(())
}

#[test]
fn per_mode_coverage_is_enforced() -> Result<(), Box<dyn std::error::Error>> {
    expect_error(
        &MINI
            .replacen(
                "\"accent.bright\": { \"$value\": { \"dark\": \"{primitive.color.orange.500}\", \"light\": \"{primitive.color.gray.25}\" } }",
                "\"accent.bright\": { \"$value\": { \"dark\": \"{primitive.color.orange.500}\" } }",
                1,
            ),
        "per-mode token is missing mode `light`",
    )
}

#[test]
fn value_parsing_rejects_bad_values() -> Result<(), Box<dyn std::error::Error>> {
    expect_error(&MINI.replacen("#D37918", "#D3791", 1), "bad color `#D3791`")?;
    expect_error(&MINI.replacen("#D37918", "D37918", 1), "bad color `D37918`")
}

#[test]
fn meta_errors_are_actionable() -> Result<(), Box<dyn std::error::Error>> {
    expect_error(
        &MINI.replacen(
            "\"default-mode\": \"dark\"",
            "\"default-mode\": \"amoled\"",
            1,
        ),
        "`default-mode` `amoled` is not a registered mode",
    )?;
    expect_error(
        &MINI.replacen(
            "\"$extensions\": { \"lumen.tokens\": {",
            "\"$extensionsx\": { \"lumen.tokens\": {",
            1,
        ),
        "missing `$extensions",
    )
}

#[test]
fn unsupported_types_and_categories_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    expect_error(
        &MINI.replacen(
            "\"primitive\": { \"color\": { \"$type\": \"color\",",
            "\"primitive\": { \"color\": { \"$type\": \"duration\",",
            1,
        ),
        "unsupported $type `duration`",
    )?;
    expect_error(
        &MINI
            .replacen(
                "\"semantic\": { \"color\": { \"$type\": \"color\",",
                "\"semantic\": { \"elevation\": { \"$type\": \"dimension\",",
                1,
            )
            .replacen(
                "\"accent\": { \"$value\": \"{alias.color.accent}\" }",
                "\"x\": { \"$value\": \"{alias.color.accent}\" }",
                1,
            ),
        "unsupported semantic category `elevation`",
    )
}
