//! Single-definition rules D1–D6 (doc 07 §5.2) plus the product.toml
//! cross-checks that need the wider world.
//!
//! * **D1** — `default` xor `default_ref` is enforced at parse time
//!   ([`crate::parse`]); this module joins the schema to `product.toml`:
//!   `default_ref` targets must exist (and type-check against the entry),
//!   and a literal default that duplicates a same-area `product.toml` value
//!   fails with a "reference it instead" hint (the doc's `[PROPOSAL`
//!   heuristic: same file+area).
//! * **D2** — setting keys globally unique.
//! * **D3** — CLI flags globally unique and disjoint from the fixed
//!   reserved-flag table and the doc 05 §5.3 subcommand words; short flags
//!   unique and disjoint from the reserved shorts.
//! * **D4** — description msgids unique 1:1 with entries.
//! * **D5** — placement orders unique within `(screen, section)`; screens
//!   must be known; `ui = false` entries must set `advanced` (the doc's
//!   "or a documented reason comment" is not parseable data, so the
//!   mechanical form is `advanced = true`).
//! * **D6** — no two entries may write the same product-decision slot: the
//!   Phase-1 slot is the `default_ref` path (the `product.toml` key the
//!   entry drives). The engine runtime-slot owner table arrives with the
//!   settings framework (card 4.E1) `[PROPOSAL]`.
//!
//! The `product.toml` tree is passed in as parsed data (`toml::Value`) —
//! the cross-check lives in the validator, not as a crate edge (doc 01
//! §2.3); the 1.E2 `config-validator` xtask wiring feeds both files.

use crate::diagnostic::SchemaDiagnostic;
use crate::model::DefaultValue;
use crate::model::SettingDef;
use crate::model::SettingsSchema;
use crate::model::check_value;

/// Reserved flag names a generated flag must not shadow (doc 05 §5.3
/// machine-mode flags + doc 07 §5.3 G2 `--generate-manifest` + universal).
pub const RESERVED_CLI_FLAGS: &[&str] = &[
    "--help",
    "-h",
    "--version",
    "-v",
    "--json",
    "--control-stdio",
    "--generate-manifest",
];

/// Reserved short flags (doc 07 §5.2: short flags come from a fixed
/// reserved table; `-h`/`-v` are the documented universal ones).
pub const RESERVED_CLI_SHORTS: &[&str] = &["-h", "-v"];

/// The doc 05 §5.3 CLI subcommand words (the "top-30" table). A generated
/// long flag whose body equals one of these words would collide with the
/// subcommand namespace (rule D3).
pub const RESERVED_CLI_SUBCOMMANDS: &[&str] = &[
    "open",
    "close",
    "play",
    "pause",
    "toggle",
    "stop",
    "seek",
    "rate",
    "step",
    "volume",
    "mute",
    "unmute",
    "tracks",
    "audio-track",
    "video-track",
    "subtitle-track",
    "subtitle-add",
    "info",
    "time",
    "duration",
    "chapters",
    "chapter",
    "devices",
    "audio-device",
    "screenshot",
    "enqueue",
    "next",
    "prev",
    "playlist",
    "plugins",
    "engine-info",
    "setting",
    "log-level",
    "shuffle",
    "repeat",
];

/// Run every D-rule and product cross-check; returns all findings.
pub fn validate(schema: &SettingsSchema, product: Option<&toml::Value>) -> Vec<SchemaDiagnostic> {
    let mut diags = Vec::new();
    d2_keys_unique(schema, &mut diags);
    d3_cli_unique_and_reserved(schema, &mut diags);
    d4_descriptions_unique(schema, &mut diags);
    d5_placements(schema, &mut diags);
    d6_default_ref_slots(schema, &mut diags);
    if let Some(product) = product {
        d1_product_join(schema, product, &mut diags);
        e_gate_features(schema, product, &mut diags);
    }
    diags
}

fn entry_err(
    diags: &mut Vec<SchemaDiagnostic>,
    rule: &str,
    def: &SettingDef,
    field: &str,
    message: String,
) {
    diags.push(SchemaDiagnostic::error(
        rule,
        Some(def.line),
        &format!("setting[{}].{}", def.index, field),
        message,
    ));
}

// --- D2 ---------------------------------------------------------------------

fn d2_keys_unique(schema: &SettingsSchema, diags: &mut Vec<SchemaDiagnostic>) {
    let mut seen: Vec<(&str, u32)> = Vec::new();
    for def in &schema.settings {
        let first = seen
            .iter()
            .find(|(key, _)| *key == def.key.as_str())
            .copied();
        match first {
            Some((first_key, first_line)) => entry_err(
                diags,
                "D2",
                def,
                "key",
                format!(
                    "duplicate setting key `{}` (first defined at line {first_line} as part \
                     of the `{first_key}` group) — setting keys are globally unique; rename \
                     one of the entries",
                    def.key
                ),
            ),
            None => seen.push((def.key.as_str(), def.line)),
        }
    }
}

// --- D3 ---------------------------------------------------------------------

fn d3_cli_unique_and_reserved(schema: &SettingsSchema, diags: &mut Vec<SchemaDiagnostic>) {
    let mut flags: Vec<(&str, &str, u32)> = Vec::new();
    let mut shorts: Vec<(&str, u32)> = Vec::new();
    for def in &schema.settings {
        let first = flags
            .iter()
            .find(|(flag, _, _)| *flag == def.cli.flag.as_str())
            .copied();
        match first {
            Some((flag, owner_key, owner_line)) => entry_err(
                diags,
                "D3",
                def,
                "cli.flag",
                format!(
                    "duplicate CLI flag `{flag}` (first defined at line {owner_line} by \
                     setting `{owner_key}`) — flags are globally unique"
                ),
            ),
            None => flags.push((def.cli.flag.as_str(), def.key.as_str(), def.line)),
        }
        if RESERVED_CLI_FLAGS.contains(&def.cli.flag.as_str()) {
            entry_err(
                diags,
                "D3",
                def,
                "cli.flag",
                format!(
                    "CLI flag `{}` is reserved ({}) — generated flags must be disjoint from \
                     the reserved table",
                    def.cli.flag,
                    RESERVED_CLI_FLAGS.join(", ")
                ),
            );
        }
        let flag_body = def.cli.flag.trim_start_matches('-');
        if RESERVED_CLI_SUBCOMMANDS.contains(&flag_body) {
            entry_err(
                diags,
                "D3",
                def,
                "cli.flag",
                format!(
                    "CLI flag `{}` collides with the doc 05 §5.3 subcommand `{flag_body}` — \
                     prefix the flag with its area (rule D3)",
                    def.cli.flag
                ),
            );
        }
        if !def.cli.short.is_empty() {
            let first_short = shorts
                .iter()
                .find(|(short, _)| *short == def.cli.short.as_str())
                .copied();
            match first_short {
                Some((_, first_line)) => entry_err(
                    diags,
                    "D3",
                    def,
                    "cli.short",
                    format!(
                        "duplicate short flag `{}` (first defined at line {first_line}) — \
                         short flags are globally unique",
                        def.cli.short
                    ),
                ),
                None => shorts.push((def.cli.short.as_str(), def.line)),
            }
            if RESERVED_CLI_SHORTS.contains(&def.cli.short.as_str()) {
                entry_err(
                    diags,
                    "D3",
                    def,
                    "cli.short",
                    format!(
                        "short flag `{}` is reserved ({})",
                        def.cli.short,
                        RESERVED_CLI_SHORTS.join(", ")
                    ),
                );
            }
        }
    }
}

// --- D4 ---------------------------------------------------------------------

fn d4_descriptions_unique(schema: &SettingsSchema, diags: &mut Vec<SchemaDiagnostic>) {
    let mut seen: Vec<(&str, u32)> = Vec::new();
    for def in &schema.settings {
        let first = seen
            .iter()
            .find(|(desc, _)| *desc == def.description.as_str())
            .copied();
        match first {
            Some((_, first_line)) => entry_err(
                diags,
                "D4",
                def,
                "description",
                format!(
                    "description msgid `{}` is already used (line {first_line}) — msgids are \
                     1:1 with entries, no shared prose keys across settings",
                    def.description
                ),
            ),
            None => seen.push((def.description.as_str(), def.line)),
        }
    }
}

// --- D5 ---------------------------------------------------------------------

fn d5_placements(schema: &SettingsSchema, diags: &mut Vec<SchemaDiagnostic>) {
    let mut seen: Vec<(String, String, u32)> = Vec::new();
    for def in &schema.settings {
        if !schema.areas.contains(&def.placement.screen) {
            entry_err(
                diags,
                "D5",
                def,
                "placement.screen",
                format!(
                    "screen `{}` is not a known area — add it to `[areas].known` (doc 07 \
                     §5.4; later: layouts/settings-areas.toml)",
                    def.placement.screen
                ),
            );
        }
        if !def.placement.ui && !def.advanced {
            entry_err(
                diags,
                "D5",
                def,
                "placement.ui",
                "entry has `ui = false` without `advanced = true` — engine-internal entries \
                 must set `advanced` (or carry a documented reason; doc 07 §5.2)"
                    .to_owned(),
            );
        }
        let slot = (
            def.placement.screen.clone(),
            def.placement.section.clone(),
            def.placement.order,
        );
        if seen.contains(&slot) {
            entry_err(
                diags,
                "D5",
                def,
                "placement.order",
                format!(
                    "order {} is already used in screen `{}` section `{}` — placement orders \
                     are unique within (screen, section)",
                    def.placement.order, def.placement.screen, def.placement.section
                ),
            );
        } else {
            seen.push(slot);
        }
    }
}

// --- D6 ---------------------------------------------------------------------

fn d6_default_ref_slots(schema: &SettingsSchema, diags: &mut Vec<SchemaDiagnostic>) {
    let mut seen: Vec<(&str, u32)> = Vec::new();
    for def in &schema.settings {
        let DefaultValue::Ref(reference) = &def.default else {
            continue;
        };
        let first = seen
            .iter()
            .find(|(slot, _)| *slot == reference.as_str())
            .copied();
        match first {
            Some((_, first_line)) => entry_err(
                diags,
                "D6",
                def,
                "default_ref",
                format!(
                    "`{reference}` is already driven by another entry (line {first_line}) — no \
                     two entries may write the same product-decision slot"
                ),
            ),
            None => seen.push((reference.as_str(), def.line)),
        }
    }
}

// --- D1 (product.toml join) --------------------------------------------------

fn d1_product_join(
    schema: &SettingsSchema,
    product: &toml::Value,
    diags: &mut Vec<SchemaDiagnostic>,
) {
    let mut leaves: Vec<(String, toml::Value)> = Vec::new();
    collect_scalar_leaves(product, "", &mut leaves);
    for def in &schema.settings {
        match &def.default {
            DefaultValue::Ref(reference) => match resolve_product_path(product, reference) {
                Some(target) => {
                    if target.as_table().is_some() {
                        entry_err(
                            diags,
                            "D1",
                            def,
                            "default_ref",
                            format!(
                                "`{reference}` resolves to a table, not a value — default_ref \
                                 must point at a product.toml value"
                            ),
                        );
                    } else if let Err(v) = check_value(def.ty, &def.domain, target) {
                        entry_err(
                            diags,
                            "D1",
                            def,
                            "default_ref",
                            format!(
                                "product.toml value at `{reference}` does not satisfy this \
                                 entry's type/domain ({}): expected {}, found {} — {}",
                                v.rule, v.expected, v.found, v.hint
                            ),
                        );
                    }
                }
                None => {
                    entry_err(
                        diags,
                        "D1",
                        def,
                        "default_ref",
                        format!(
                            "`{reference}` does not exist in product.toml — a default_ref \
                             pointing at a nonexistent snake_case path is an error (rule D1, \
                             doc 07 §5.2)"
                        ),
                    );
                }
            },
            DefaultValue::Literal(value) => {
                // [PROPOSAL heuristic: same file+area] — a literal default
                // that duplicates a product.toml value under the same area
                // should be a default_ref instead.
                let area = def.key.split('.').next().unwrap_or_default();
                for (path, leaf) in &leaves {
                    let leaf_area = path.split('.').next().unwrap_or_default();
                    if leaf_area == area && leaf == value {
                        entry_err(
                            diags,
                            "D1",
                            def,
                            "default",
                            format!(
                                "literal default `{value}` duplicates the product.toml value \
                                 at `{path}` — product-level defaults are stated once, in \
                                 product.toml: use `default_ref = \"{path}\"` (rule D1, \
                                 heuristic: same file+area)"
                            ),
                        );
                    }
                }
            }
        }
    }
}

/// E-GATE: a nonempty feature_gate must be a feature id that product.toml
/// [features] actually declares (rule F1 — you cannot even name an
/// unimplemented feature, doc 07 §2.7).
fn e_gate_features(
    schema: &SettingsSchema,
    product: &toml::Value,
    diags: &mut Vec<SchemaDiagnostic>,
) {
    let features = product.get("features").and_then(toml::Value::as_table);
    for def in &schema.settings {
        if def.feature_gate.is_empty() {
            continue;
        }
        match features {
            None => {
                entry_err(
                    diags,
                    "E-GATE",
                    def,
                    "feature_gate",
                    "product.toml has no [features] table to check the gate against".to_owned(),
                );
            }
            Some(table) => {
                if !table.contains_key(&def.feature_gate) {
                    entry_err(
                        diags,
                        "E-GATE",
                        def,
                        "feature_gate",
                        format!(
                            "feature_gate `{}` is not a feature id in product.toml \
                             [features] (rule F1, doc 07 §2.7)",
                            def.feature_gate
                        ),
                    );
                }
            }
        }
    }
}

/// Walk a TOML tree collecting value leaves (scalars and scalar arrays)
/// with their dotted paths.
fn collect_scalar_leaves(value: &toml::Value, prefix: &str, out: &mut Vec<(String, toml::Value)>) {
    match value {
        toml::Value::Table(table) => {
            for (key, child) in table {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                collect_scalar_leaves(child, &path, out);
            }
        }
        _ => out.push((prefix.to_owned(), value.clone())),
    }
}

/// Resolve a dotted snake_case path inside a TOML tree.
fn resolve_product_path<'a>(product: &'a toml::Value, path: &str) -> Option<&'a toml::Value> {
    let mut current = product;
    for segment in path.split('.') {
        current = current.as_table()?.get(segment)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two clean entries + areas; every test mutates exactly one thing.
    fn fixture() -> String {
        let entry = |key: &str, flag: &str, order: u32| {
            format!(
                "\n[[setting]]\nkey = \"{key}\"\ntype = \"bool\"\ndefault = false\n\
                 description = \"settings.{key}\"\nrestart_required = false\n\
                 placement = {{ screen = \"playback\", section = \"tracks\", order = {order}, ui = true }}\n\
                 cli = {{ flag = \"{flag}\", short = \"\" }}\nfeature_gate = \"\"\nsince = \"0.1.0\"\n"
            )
        };
        format!(
            "[areas]\nknown = [\"playback\"]\n{}{}",
            entry("playback.one", "--playback-one", 10),
            entry("playback.two", "--playback-two", 20)
        )
    }

    fn parse_fixture(src: &str) -> SettingsSchema {
        match crate::parse::parse_str(src) {
            Ok(schema) => schema,
            Err(diags) => {
                assert_eq!(
                    "",
                    crate::diagnostic::render_all(&diags),
                    "fixture failed to parse"
                );
                SettingsSchema {
                    areas: Vec::new(),
                    settings: Vec::new(),
                }
            }
        }
    }

    fn parse_product(src: &str) -> Option<toml::Value> {
        toml::from_str::<toml::Table>(src)
            .ok()
            .map(toml::Value::Table)
    }

    fn validate_fixture(src: &str, product: Option<&toml::Value>) -> Vec<SchemaDiagnostic> {
        let schema = parse_fixture(src);
        validate(&schema, product)
    }

    const PRODUCT: &str = r#"
[defaults]
theme = "dark"

[features]
core-playback = true

[playback]
subtitle_mode = "auto"
"#;

    #[test]
    fn clean_fixture_has_no_findings() {
        assert_eq!(
            validate_fixture(&fixture(), None),
            Vec::<SchemaDiagnostic>::new()
        );
        let product = parse_product(PRODUCT);
        assert_eq!(
            validate_fixture(&fixture(), product.as_ref()),
            Vec::<SchemaDiagnostic>::new()
        );
    }

    #[test]
    fn d2_duplicate_key_fails_with_rule_and_first_line() {
        let src = fixture().replace("key = \"playback.two\"", "key = \"playback.one\"");
        let diags = validate_fixture(&src, None);
        assert_eq!(diags.len(), 1, "only D2 fires: {diags:?}");
        assert_eq!(diags[0].rule, "D2");
        assert!(
            diags[0]
                .message
                .contains("duplicate setting key `playback.one`"),
            "D2 must name the duplicate: {}",
            diags[0]
        );
        assert!(diags[0].message.contains("first defined at line"));
    }

    #[test]
    fn d3_duplicate_flag_fails() {
        let src = fixture().replace("--playback-two", "--playback-one");
        let diags = validate_fixture(&src, None);
        assert_eq!(diags.len(), 1, "only D3 fires: {diags:?}");
        assert!(diags[0].message.contains("duplicate CLI flag"));
    }

    #[test]
    fn d3_reserved_flag_fails() {
        let src = fixture().replace("--playback-one", "--json");
        assert_eq!(
            validate_fixture(&src, None)
                .iter()
                .map(|d| d.rule.as_str())
                .collect::<Vec<_>>(),
            vec!["D3"]
        );
    }

    #[test]
    fn d3_subcommand_collision_fails() {
        let src = fixture().replace("--playback-one", "--seek");
        let diags = validate_fixture(&src, None);
        assert!(
            diags
                .iter()
                .any(|d| d.rule == "D3" && d.message.contains("subcommand")),
            "flag body colliding with a subcommand must fail D3: {diags:?}"
        );
    }

    #[test]
    fn d3_duplicate_short_fails() {
        let src = fixture()
            .replace(
                "cli = { flag = \"--playback-one\", short = \"\" }",
                "cli = { flag = \"--playback-one\", short = \"-x\" }",
            )
            .replace(
                "cli = { flag = \"--playback-two\", short = \"\" }",
                "cli = { flag = \"--playback-two\", short = \"-x\" }",
            );
        assert_eq!(
            validate_fixture(&src, None)
                .iter()
                .map(|d| d.rule.as_str())
                .collect::<Vec<_>>(),
            vec!["D3"]
        );
    }

    #[test]
    fn d4_duplicate_description_fails() {
        let src = fixture().replace(
            "description = \"settings.playback.two\"",
            "description = \"settings.playback.one\"",
        );
        let diags = validate_fixture(&src, None);
        assert_eq!(diags.len(), 1, "only D4 fires: {diags:?}");
        assert_eq!(diags[0].rule, "D4");
        assert!(diags[0].message.contains("1:1 with entries"));
    }

    #[test]
    fn d5_duplicate_order_within_screen_section_fails() {
        let src = fixture().replace("order = 20", "order = 10");
        let diags = validate_fixture(&src, None);
        assert_eq!(diags.len(), 1, "only D5 fires: {diags:?}");
        assert_eq!(diags[0].rule, "D5");
        assert!(diags[0].message.contains("unique within (screen, section)"));
    }

    #[test]
    fn d5_unknown_screen_fails() {
        let src = fixture().replace("screen = \"playback\"", "screen = \"bogus\"");
        let diags = validate_fixture(&src, None);
        // Both entries move to the bogus screen; both orders stay unique.
        assert_eq!(diags.len(), 2, "both entries flagged: {diags:?}");
        assert!(diags.iter().all(|d| d.rule == "D5"));
    }

    #[test]
    fn d5_ui_false_without_advanced_fails() {
        let src = fixture().replace("order = 20, ui = true", "order = 20, ui = false");
        let diags = validate_fixture(&src, None);
        assert_eq!(diags.len(), 1, "only D5 fires: {diags:?}");
        assert!(
            diags[0]
                .message
                .contains("`ui = false` without `advanced = true`")
        );
    }

    #[test]
    fn d6_two_entries_same_default_ref_fails() {
        let to_enum_ref = |key: &str| {
            format!(
                "type = \"enum\"\nenumeration = [\"dark\", \"light\"]\ndefault_ref = \
                 \"defaults.theme\"\ndescription = \"settings.{key}\""
            )
        };
        let src = fixture()
            .replace(
                "type = \"bool\"\ndefault = false\ndescription = \"settings.playback.one\"",
                &to_enum_ref("playback.one"),
            )
            .replace(
                "type = \"bool\"\ndefault = false\ndescription = \"settings.playback.two\"",
                &to_enum_ref("playback.two"),
            );
        let product = parse_product(PRODUCT);
        let diags = validate_fixture(&src, product.as_ref());
        assert_eq!(diags.len(), 1, "only D6 fires: {diags:?}");
        assert_eq!(diags[0].rule, "D6");
        assert!(diags[0].message.contains("same product-decision slot"));
    }

    #[test]
    fn d1_default_ref_must_exist_in_product() {
        let src = fixture().replace(
            "default = false\ndescription = \"settings.playback.one\"",
            "default_ref = \"defaults.nonexistent\"\ndescription = \"settings.playback.one\"",
        );
        let product = parse_product(PRODUCT);
        let diags = validate_fixture(&src, product.as_ref());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, "D1");
        assert!(diags[0].message.contains("does not exist in product.toml"));
    }

    #[test]
    fn d1_default_ref_type_mismatch_fails() {
        // bool entry referencing a string product value.
        let src = fixture().replace(
            "default = false\ndescription = \"settings.playback.one\"",
            "default_ref = \"defaults.theme\"\ndescription = \"settings.playback.one\"",
        );
        let product = parse_product(PRODUCT);
        let diags = validate_fixture(&src, product.as_ref());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, "D1");
        assert!(diags[0].message.contains("does not satisfy"));
    }

    #[test]
    fn d1_same_area_literal_duplication_heuristic_fires() {
        // playback.* entry with literal "auto" duplicating
        // [playback] subtitle_mode = "auto" — must suggest the ref.
        let src = fixture().replace(
            "key = \"playback.one\"\ntype = \"bool\"\ndefault = false",
            "key = \"playback.one\"\ntype = \"enum\"\nenumeration = [\"auto\", \"off\"]\ndefault = \"auto\"",
        );
        let product = parse_product(PRODUCT);
        let diags = validate_fixture(&src, product.as_ref());
        let d1: Vec<&SchemaDiagnostic> = diags.iter().filter(|d| d.rule == "D1").collect();
        assert_eq!(
            d1.len(),
            1,
            "got: {}",
            crate::diagnostic::render_all(&diags)
        );
        assert!(d1[0].message.contains("playback.subtitle_mode"));
        assert!(d1[0].message.contains("default_ref"));
    }

    #[test]
    fn e_gate_unknown_feature_fails() {
        let src = fixture().replace("feature_gate = \"\"", "feature_gate = \"no-such-feature\"");
        let product = parse_product(PRODUCT);
        let diags = validate_fixture(&src, product.as_ref());
        assert_eq!(diags.len(), 2, "both entries flagged: {diags:?}");
        assert!(diags.iter().all(|d| d.rule == "E-GATE"));
        assert!(diags[0].message.contains("rule F1"));
    }

    #[test]
    fn literal_cross_area_default_is_not_flagged() {
        // privacy.* literal false vs [playback] values — different area, no
        // heuristic fire (doc 07 §5.4 seeds privacy literals cross-area).
        let src = fixture().replace("key = \"playback.one\"", "key = \"privacy.telemetry\"");
        let product = parse_product(PRODUCT);
        assert_eq!(
            validate_fixture(&src, product.as_ref()),
            Vec::<SchemaDiagnostic>::new()
        );
    }

    #[test]
    fn reserved_tables_are_the_doc_05_set() {
        assert!(RESERVED_CLI_FLAGS.contains(&"--json"));
        assert!(RESERVED_CLI_SUBCOMMANDS.contains(&"log-level"));
        assert!(RESERVED_CLI_SHORTS.contains(&"-h"));
    }
}
