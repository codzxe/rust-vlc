//! Fixture-based acceptance proof for `cargo xtask config-validator`
//! (issue #20, card 8.F1): a good product.toml passes against a schema
//! fixture mirroring product.schema.json's shapes, and each seeded failure
//! mode — unknown key, type mismatch, missing required key, enum violation,
//! out-of-range integer, dangling brand asset, missing locale file, missing
//! msgid, unknown title variable, wrong schema_version, wrong app id —
//! fails with exit 1.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;


/// Lint-legal expect for fixture setup (testkit is not an xtask
/// dev-dep — boundaries.toml dev_deps are Lead-owned): a broken
/// fixture means a broken test, never a product bug.
fn must<T, E: std::fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => unreachable!("fixture setup failed: {err}"),
    }
}

struct RunOutcome {
    code: Option<i32>,
    stdout: String,
}

fn run_validator(root: &Path) -> RunOutcome {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_xtask"));
    cmd.arg("config-validator").arg("--root").arg(root);
    match cmd.output() {
        Ok(out) => RunOutcome {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        },
        Err(err) => RunOutcome {
            code: None,
            stdout: format!("failed to spawn xtask binary: {err}"),
        },
    }
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "xtask-config-validator-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        must(fs::create_dir_all(root.join("assets/brand")));
        must(fs::create_dir_all(root.join("locales")));
        must(fs::create_dir_all(root.join("themes")));
        Fixture { root }
    }

    fn write_product(&self, body: &str) {
        must(fs::write(self.root.join("product.toml"), body));
    }

    fn write_good_product(&self) {
        self.write_product(GOOD_PRODUCT);
    }

    fn write_schema(&self) {
        must(fs::write(self.root.join("product.schema.json"), SCHEMA));
    }

    fn add_brand_assets(&self) {
        for name in ["icon.ico", "icon.icns", "icon-512.png", "logo.svg"] {
            must(fs::write(self.root.join("assets/brand").join(name), b"x"));
        }
    }

    fn add_catalog(&self) {
        must(fs::write(
            self.root.join("locales/en-US.toml"),
            "\"product.description\" = \"A player.\"\n\"window.title.idle\" = \"{product_name}\"\n",
        ));
    }

    fn add_theme(&self) {
        must(fs::write(self.root.join("themes/dark.toml"), "[theme]\n"));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const GOOD_PRODUCT: &str = r#"
schema_version = 1

[identity]
name = "Lumen"
short_name = "Lumen"
description_key = "product.description"

[identity.ids]
reverse_dns = "dev.lumen"

[identity.ids.windows]
app_user_model_id = "dev.lumen.Lumen"

[window]
width = 1280
title_template = "{media_title} — {product_name}"
title_idle_key = "window.title.idle"

[brand]
icon_ico = "assets/brand/icon.ico"
icon_icns = "assets/brand/icon.icns"
icon_png_512 = "assets/brand/icon-512.png"
logo_svg = "assets/brand/logo.svg"

[defaults]
theme = "dark"
locale = "en-US"

[features]
core-playback = true
streaming-hls = false

[telemetry]
enabled = false
"#;

const SCHEMA: &str = r##"{
  "type": "object",
  "required": ["schema_version", "identity", "window", "brand", "defaults", "features", "telemetry"],
  "additionalProperties": false,
  "properties": {
    "schema_version": { "type": "integer", "minimum": 1 },
    "identity": {
      "type": "object",
      "required": ["name", "short_name", "description_key", "ids"],
      "additionalProperties": false,
      "properties": {
        "name": { "type": "string", "not": { "enum": ["VLC", "VideoLAN", "vlc"] } },
        "short_name": { "type": "string", "minLength": 1, "maxLength": 12 },
        "description_key": { "$ref": "#/$defs/msgid" },
        "ids": { "type": "object" }
      }
    },
    "window": {
      "type": "object",
      "required": ["width", "title_template", "title_idle_key"],
      "additionalProperties": false,
      "properties": {
        "width": { "type": "integer", "minimum": 480, "maximum": 3840 },
        "title_template": { "type": "string" },
        "title_idle_key": { "$ref": "#/$defs/msgid" }
      }
    },
    "brand": {
      "type": "object",
      "required": ["icon_ico", "icon_icns", "icon_png_512", "logo_svg"],
      "additionalProperties": false,
      "properties": {
        "icon_ico": { "type": "string", "minLength": 1 },
        "icon_icns": { "type": "string", "minLength": 1 },
        "icon_png_512": { "type": "string", "minLength": 1 },
        "logo_svg": { "type": "string", "minLength": 1 }
      }
    },
    "defaults": {
      "type": "object",
      "required": ["theme", "locale"],
      "additionalProperties": false,
      "properties": {
        "theme": { "type": "string", "enum": [] },
        "locale": { "type": "string", "enum": [] }
      }
    },
    "features": {
      "type": "object",
      "additionalProperties": { "type": "boolean" }
    },
    "telemetry": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "enabled": { "type": "boolean" },
        "crash_reports": { "enum": ["off", "ask", "send"] }
      }
    }
  },
  "$defs": {
    "msgid": { "type": "string", "pattern": "^[a-z0-9-]+(\\.[a-z0-9-]+)+$" }
  }
}"##;

fn assert_fails(fx: &Fixture, needle: &str, label: &str) {
    let out = run_validator(&fx.root);
    assert_eq!(out.code, Some(1), "{label} must fail:\n{}", out.stdout);
    assert!(
        out.stdout.contains(needle),
        "{label} should mention `{needle}`:\n{}",
        out.stdout
    );
}

#[test]
fn good_product_passes() {
    let fx = Fixture::new("good");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_catalog();
    fx.add_theme();
    let out = run_validator(&fx.root);
    assert_eq!(
        out.code,
        Some(0),
        "clean fixture must pass:\n{}",
        out.stdout
    );
    assert!(out.stdout.contains("result: PASS"), "{}", out.stdout);
}

#[test]
fn unknown_key_fails() {
    let fx = Fixture::new("unknown-key");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_catalog();
    fx.add_theme();
    fx.write_product(&GOOD_PRODUCT.replace("short_name = \"Lumen\"", "short_nam = \"Lumen\""));
    assert_fails(&fx, "unknown key `short_nam`", "unknown key");
}

#[test]
fn type_mismatch_fails() {
    let fx = Fixture::new("type");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_catalog();
    fx.add_theme();
    fx.write_product(&GOOD_PRODUCT.replace("width = 1280", "width = \"wide\""));
    assert_fails(&fx, "type mismatch", "type mismatch");
}

#[test]
fn missing_required_key_fails() {
    let fx = Fixture::new("required");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_catalog();
    fx.add_theme();
    fx.write_product(&GOOD_PRODUCT.replace("short_name = \"Lumen\"\n", ""));
    assert_fails(&fx, "missing required key `short_name`", "missing required");
}

#[test]
fn reserved_name_fails() {
    let fx = Fixture::new("reserved");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_catalog();
    fx.add_theme();
    fx.write_product(&GOOD_PRODUCT.replace("name = \"Lumen\"", "name = \"VLC\""));
    assert_fails(&fx, "reserved/banned value", "reserved name");
}

#[test]
fn out_of_range_integer_fails() {
    let fx = Fixture::new("range");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_catalog();
    fx.add_theme();
    fx.write_product(&GOOD_PRODUCT.replace("width = 1280", "width = 100"));
    assert_fails(&fx, "< minimum 480", "integer range");
}

#[test]
fn enum_violation_fails() {
    let fx = Fixture::new("enum");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_catalog();
    fx.add_theme();
    fx.write_product(&GOOD_PRODUCT.replace(
        "enabled = false",
        "enabled = false\ncrash_reports = \"always\"",
    ));
    assert_fails(&fx, "not in enum", "enum violation");
}

#[test]
fn non_boolean_feature_flag_fails() {
    let fx = Fixture::new("feature-type");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_catalog();
    fx.add_theme();
    fx.write_product(&GOOD_PRODUCT.replace("streaming-hls = false", "streaming-hls = \"no\""));
    assert_fails(&fx, "type mismatch", "feature flag type");
}

#[test]
fn dangling_brand_asset_fails() {
    let fx = Fixture::new("asset");
    fx.write_good_product();
    fx.write_schema();
    fx.add_catalog();
    fx.add_theme();
    // No brand assets written.
    assert_fails(&fx, "does not exist under the root", "dangling asset");
}

#[test]
fn unknown_locale_fails() {
    let fx = Fixture::new("locale");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_catalog();
    fx.add_theme();
    fx.write_product(&GOOD_PRODUCT.replace("locale = \"en-US\"", "locale = \"xx-XX\""));
    assert_fails(&fx, "locales/xx-XX.toml does not exist", "unknown locale");
}

#[test]
fn theme_outside_directory_listing_fails() {
    let fx = Fixture::new("theme");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_catalog();
    // themes/ exists but is empty -> injection skipped (note, not violation).
    let out = run_validator(&fx.root);
    assert_eq!(
        out.code,
        Some(0),
        "empty themes/ listing is a note, not a violation:\n{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("enum injection skipped"),
        "{}",
        out.stdout
    );
    // With a theme present, a wrong id IS a violation.
    fx.add_theme();
    fx.write_product(&GOOD_PRODUCT.replace("theme = \"dark\"", "theme = \"neon\""));
    assert_fails(&fx, "not in `themes/` listing", "theme listing");
}

#[test]
fn missing_msgid_fails() {
    let fx = Fixture::new("msgid");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_theme();
    // Catalog present but unparseable -> honest note, not a violation.
    must(fs::write(fx.root.join("locales/en-US.toml"), "%%% not toml ["));
    let out = run_validator(&fx.root);
    assert_eq!(
        out.code,
        Some(0),
        "unreadable catalog is a note:\n{}",
        out.stdout
    );
    assert!(out.stdout.contains("unreadable"), "{}", out.stdout);
    // Catalog present but missing the key.
    fx.add_catalog();
    fx.write_product(&GOOD_PRODUCT.replace(
        "description_key = \"product.description\"",
        "description_key = \"product.missing\"",
    ));
    assert_fails(&fx, "not in the reference catalog", "missing msgid");
}

#[test]
fn unknown_title_variable_fails() {
    let fx = Fixture::new("title-var");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_catalog();
    fx.add_theme();
    fx.write_product(&GOOD_PRODUCT.replace(
        "title_template = \"{media_title} — {product_name}\"",
        "title_template = \"{media_title} — {app_name}\"",
    ));
    assert_fails(&fx, "unknown variable {app_name}", "title variable");
}

#[test]
fn wrong_schema_version_fails() {
    let fx = Fixture::new("version");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_catalog();
    fx.add_theme();
    fx.write_product(&GOOD_PRODUCT.replace("schema_version = 1", "schema_version = 7"));
    assert_fails(&fx, "schema_version 7 != 1", "schema version");
}

#[test]
fn wrong_app_id_fails() {
    let fx = Fixture::new("appid");
    fx.write_good_product();
    fx.write_schema();
    fx.add_brand_assets();
    fx.add_catalog();
    fx.add_theme();
    fx.write_product(&GOOD_PRODUCT.replace(
        "app_user_model_id = \"dev.lumen.Lumen\"",
        "app_user_model_id = \"dev.lumen.Other\"",
    ));
    assert_fails(&fx, "PascalCase", "app id rule");
}

#[test]
fn missing_files_are_xtask_errors() {
    let fx = Fixture::new("missing");
    let out = run_validator(&fx.root);
    assert_eq!(
        out.code,
        Some(2),
        "unreadable product/schema is an xtask error:\n{}",
        out.stdout
    );
}

#[test]
fn unparseable_product_toml_fails_with_exit_one() {
    let fx = Fixture::new("bad-toml");
    fx.write_schema();
    fx.write_product("[identity\nname = \"Lumen\"\n");
    let out = run_validator(&fx.root);
    assert_eq!(
        out.code,
        Some(1),
        "parse failure is a failed validation:\n{}",
        out.stdout
    );
    assert!(out.stdout.contains("parse:"), "{}", out.stdout);
}
