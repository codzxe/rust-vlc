//! The typed settings model (doc 07 §5.2 entry format — ten mandated fields,
//! four `[PROPOSAL]` extensions) plus the value domain shared by parsing,
//! validation and every generator.

/// A whole parsed schema: the known areas plus every setting entry, in
/// source order (generators re-sort deterministically).
#[derive(Debug, Clone)]
pub struct SettingsSchema {
    /// Known screen ids (doc 07 §5.4; seeds `[areas].known`).
    pub areas: Vec<String>,
    /// Entries in `[[setting]]` source order.
    pub settings: Vec<SettingDef>,
}

impl SettingsSchema {
    /// Look an entry up by canonical key.
    pub fn entry(&self, key: &str) -> Option<&SettingDef> {
        self.settings.iter().find(|s| s.key == key)
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.settings.len()
    }

    /// Whether the schema has no entries.
    pub fn is_empty(&self) -> bool {
        self.settings.is_empty()
    }

    /// Screen ids that carry at least one entry, sorted and deduplicated —
    /// the screen iteration order used by every generator.
    pub fn screens_in_use(&self) -> Vec<String> {
        let mut screens: Vec<String> = self
            .settings
            .iter()
            .map(|s| s.placement.screen.clone())
            .collect();
        screens.sort();
        screens.dedup();
        screens
    }

    /// Entries of one screen sorted by `(section, order, key)` — the doc 07
    /// §5.4 navigation order.
    pub fn screen_entries(&self, screen: &str) -> Vec<&SettingDef> {
        let mut entries: Vec<&SettingDef> = self
            .settings
            .iter()
            .filter(|s| s.placement.screen == screen)
            .collect();
        entries.sort_by(|a, b| {
            a.placement
                .section
                .cmp(&b.placement.section)
                .then(a.placement.order.cmp(&b.placement.order))
                .then(a.key.cmp(&b.key))
        });
        entries
    }
}

/// One `[[setting]]` entry (doc 07 §5.2). All ten mandated slots are always
/// populated after parsing (so generators never special-case); the
/// `[PROPOSAL]` extensions carry their documented defaults.
#[derive(Debug, Clone)]
pub struct SettingDef {
    /// Canonical dotted kebab-case id (D2: globally unique).
    pub key: String,
    /// Value type.
    pub ty: SettingType,
    /// Value domain: range, enumeration, or unconstrained.
    pub domain: Domain,
    /// Literal default, or a `product.toml` reference (D1: exactly one).
    pub default: DefaultValue,
    /// Description msgid (D4: unique 1:1 with entries; never literal prose).
    pub description: String,
    /// `true` ⇒ UI shows a restart hint; CLI exits "applied, restart
    /// required" (doc 05 §2.11).
    pub restart_required: bool,
    /// UI placement.
    pub placement: UiPlacement,
    /// CLI surface.
    pub cli: CliSpec,
    /// Feature gate id; empty = ungated (doc 05 §6.4).
    pub feature_gate: String,
    /// First product version shipping this entry.
    pub since: String,
    /// `user` (persisted, layer 4) or `session` (layer 6) — `[PROPOSAL]`.
    pub scope: Scope,
    /// Array merge policy — `[PROPOSAL]` (doc 07 §4.4 rule 3).
    pub array_policy: ArrayPolicy,
    /// VLC-style "show advanced" folding.
    pub advanced: bool,
    /// Version that deprecated this entry; empty = active.
    pub deprecated_in: String,
    /// 1-based source line of the `[[setting]]` header (diagnostics).
    pub line: u32,
    /// Source index of the entry (diagnostics: `setting[<index>]`).
    pub index: usize,
}

/// Setting value types (doc 07 §5.2): `bool|int|float|string|enum|path|url|
/// list<T>|duration-ms|percent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingType {
    Bool,
    Int,
    Float,
    String,
    Enum,
    Path,
    Url,
    /// `list<T>` — homogeneous list of `T`.
    List(ListElement),
    DurationMs,
    Percent,
}

/// Element type of `list<T>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListElement {
    String,
    Path,
    Url,
    Enum,
}

impl SettingType {
    /// Parse the TOML type tag. Returns `None` for unknown tags.
    pub fn parse(tag: &str) -> Option<Self> {
        match tag {
            "bool" => Some(SettingType::Bool),
            "int" => Some(SettingType::Int),
            "float" => Some(SettingType::Float),
            "string" => Some(SettingType::String),
            "enum" => Some(SettingType::Enum),
            "path" => Some(SettingType::Path),
            "url" => Some(SettingType::Url),
            "duration-ms" => Some(SettingType::DurationMs),
            "percent" => Some(SettingType::Percent),
            "list<string>" => Some(SettingType::List(ListElement::String)),
            "list<path>" => Some(SettingType::List(ListElement::Path)),
            "list<url>" => Some(SettingType::List(ListElement::Url)),
            "list<enum>" => Some(SettingType::List(ListElement::Enum)),
            _ => None,
        }
    }

    /// The canonical TOML tag (round-trips through [`parse`]).
    pub fn as_str(self) -> &'static str {
        match self {
            SettingType::Bool => "bool",
            SettingType::Int => "int",
            SettingType::Float => "float",
            SettingType::String => "string",
            SettingType::Enum => "enum",
            SettingType::Path => "path",
            SettingType::Url => "url",
            SettingType::DurationMs => "duration-ms",
            SettingType::Percent => "percent",
            SettingType::List(ListElement::String) => "list<string>",
            SettingType::List(ListElement::Path) => "list<path>",
            SettingType::List(ListElement::Url) => "list<url>",
            SettingType::List(ListElement::Enum) => "list<enum>",
        }
    }

    /// Whether numeric values (with a [`Domain::Range`]) are integers only.
    pub fn is_integer_valued(self) -> bool {
        matches!(self, SettingType::Int | SettingType::DurationMs)
    }

    /// Whether the type carries a numeric range domain.
    pub fn is_ranged(self) -> bool {
        matches!(
            self,
            SettingType::Int | SettingType::Float | SettingType::DurationMs | SettingType::Percent
        )
    }

    /// The UI control hint (doc 07 §5.3 G1 control selection):
    /// bool→toggle, enum→select, ranged numeric→slider, string/url→input,
    /// list→tag editor, path→path picker, duration-ms→duration input.
    pub fn control(self) -> Control {
        match self {
            SettingType::Bool => Control::Toggle,
            SettingType::Enum => Control::Select,
            SettingType::Int | SettingType::Float | SettingType::Percent => Control::Slider,
            SettingType::String | SettingType::Url => Control::Input,
            SettingType::Path => Control::PathPicker,
            SettingType::List(_) => Control::List,
            SettingType::DurationMs => Control::Duration,
        }
    }
}

/// UI control hint derived from the type (doc 07 §5.3 G1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Toggle,
    Select,
    Slider,
    Input,
    List,
    PathPicker,
    Duration,
}

impl Control {
    /// The wire name used in generated artifacts.
    pub fn as_str(self) -> &'static str {
        match self {
            Control::Toggle => "toggle",
            Control::Select => "select",
            Control::Slider => "slider",
            Control::Input => "input",
            Control::List => "list",
            Control::PathPicker => "path-picker",
            Control::Duration => "duration",
        }
    }
}

/// The value domain of a setting.
#[derive(Debug, Clone, PartialEq)]
pub enum Domain {
    /// Numeric bounds (int/float/duration-ms/percent).
    Range(ValueRange),
    /// Closed set of members (enum, and `list<enum>` elements).
    Enumeration(Vec<String>),
    /// No domain constraint (bool/string/path/url/list of scalars).
    Unconstrained,
}

/// Inclusive numeric bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValueRange {
    pub min: f64,
    pub max: f64,
}

/// A setting's default: a literal, or a reference into `product.toml`
/// (D1: exactly one; product-level defaults are stated once, in
/// `product.toml`, and referenced by the schema).
#[derive(Debug, Clone, PartialEq)]
pub enum DefaultValue {
    Literal(toml::Value),
    Ref(String),
}

/// UI placement (doc 07 §5.2): screen + section + order, and the `ui` flag
/// (`false` for engine-internal entries that never render).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPlacement {
    pub screen: String,
    pub section: String,
    pub order: u32,
    pub ui: bool,
}

/// CLI surface of a setting (doc 07 §5.2): one long flag (D3-unique,
/// disjoint from reserved names) plus an optional short flag from the fixed
/// reserved table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliSpec {
    pub flag: String,
    /// Empty when the entry has no short flag.
    pub short: String,
}

/// Persistence scope — `[PROPOSAL]` (doc 07 §4.1 layer 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scope {
    /// Persisted user preference (layer 4).
    #[default]
    User,
    /// Per-session transient value (layer 6).
    Session,
}

impl Scope {
    /// The TOML spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::User => "user",
            Scope::Session => "session",
        }
    }
}

/// Array merge policy — `[PROPOSAL]` (doc 07 §4.4 rule 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrayPolicy {
    /// Higher layer's list fully replaces the lower one (v1 default).
    #[default]
    Replace,
    /// Additive semantics where intended (plugin enable-lists).
    AppendUnique,
}

impl ArrayPolicy {
    /// The TOML spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            ArrayPolicy::Replace => "replace",
            ArrayPolicy::AppendUnique => "append-unique",
        }
    }
}

/// One runtime value violation (the G4 `config.set-setting` contract:
/// `invalid-argument`, never silent coercion — doc 07 §5.3, doc 05 §2.11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueViolation {
    /// `V-TYPE` (wrong TOML type), `V-RANGE` (out of bounds),
    /// `V-ENUM` (not a member) or `V-FORMAT` (path/url shape).
    pub rule: &'static str,
    /// What the entry's type/domain requires.
    pub expected: String,
    /// What was actually found (rendered TOML).
    pub found: String,
    /// Fix hint.
    pub hint: String,
}

impl std::fmt::Display for ValueViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "rule {}: expected {}, found {} — {}",
            self.rule, self.expected, self.found, self.hint
        )
    }
}

impl std::error::Error for ValueViolation {}

/// Check a TOML value against an entry's type and domain. Shared by the
/// parse-time literal-default check (`E-DEFAULT`) and the runtime
/// `config.set-setting` validation (G4) so both enforce the exact same rule.
pub fn check_value(
    ty: SettingType,
    domain: &Domain,
    value: &toml::Value,
) -> Result<(), ValueViolation> {
    match ty {
        SettingType::Bool => value
            .as_bool()
            .map(|_| ())
            .ok_or_else(|| violation("V-TYPE", "a boolean", value, "write `true` or `false`")),
        SettingType::Int | SettingType::DurationMs => {
            let n = value
                .as_integer()
                .ok_or_else(|| violation("V-TYPE", "an integer", value, "write a whole number"))?;
            check_range(domain, n as f64, value, "an integer")
        }
        SettingType::Float | SettingType::Percent => {
            let n = value_as_f64(value)
                .ok_or_else(|| violation("V-TYPE", "a number", value, "write a decimal number"))?;
            check_range(domain, n, value, "a number")
        }
        SettingType::String => value
            .as_str()
            .map(|_| ())
            .ok_or_else(|| violation("V-TYPE", "a string", value, "write a quoted string")),
        SettingType::Path => match value.as_str() {
            Some(s) if !s.is_empty() => Ok(()),
            Some(_) => Err(violation(
                "V-FORMAT",
                "a non-empty path string",
                value,
                "paths cannot be empty",
            )),
            None => Err(violation(
                "V-TYPE",
                "a path string",
                value,
                "write a quoted filesystem path",
            )),
        },
        SettingType::Url => match value.as_str() {
            Some(s) if s.starts_with("https://") || s.starts_with("http://") => Ok(()),
            Some(_) => Err(violation(
                "V-FORMAT",
                "an http(s) URL string",
                value,
                "URLs start with `https://` (product URLs must be https, doc 07 §2.10)",
            )),
            None => Err(violation(
                "V-TYPE",
                "a URL string",
                value,
                "write a quoted URL",
            )),
        },
        SettingType::Enum => {
            let Domain::Enumeration(members) = domain else {
                return Ok(());
            };
            match value.as_str() {
                Some(s) if members.iter().any(|m| m == s) => Ok(()),
                Some(_) => Err(violation(
                    "V-ENUM",
                    &format!("one of {}", members.join(", ")),
                    value,
                    "pick a member of the enumeration",
                )),
                None => Err(violation(
                    "V-TYPE",
                    "an enum member string",
                    value,
                    "write a quoted member of the enumeration",
                )),
            }
        }
        SettingType::List(element) => {
            let items = value
                .as_array()
                .ok_or_else(|| violation("V-TYPE", "an array", value, "write a TOML array"))?;
            let element_ty = match element {
                ListElement::String => SettingType::String,
                ListElement::Path => SettingType::Path,
                ListElement::Url => SettingType::Url,
                ListElement::Enum => SettingType::Enum,
            };
            for item in items {
                check_value(element_ty, domain, item)?;
            }
            Ok(())
        }
    }
}

fn check_range(
    domain: &Domain,
    n: f64,
    value: &toml::Value,
    type_name: &str,
) -> Result<(), ValueViolation> {
    let Domain::Range(range) = domain else {
        return Ok(());
    };
    if n < range.min || n > range.max {
        return Err(violation(
            "V-RANGE",
            &format!(
                "{type_name} in {}..={}",
                fmt_bound(range.min),
                fmt_bound(range.max)
            ),
            value,
            "out-of-range values are rejected, never clamped (MP §5)",
        ));
    }
    Ok(())
}

fn violation(
    rule: &'static str,
    expected: &str,
    value: &toml::Value,
    hint: &str,
) -> ValueViolation {
    ValueViolation {
        rule,
        expected: expected.to_owned(),
        found: render_toml_value(value),
        hint: hint.to_owned(),
    }
}

/// Render a TOML value for diagnostics.
pub fn render_toml_value(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => format!("string \"{s}\""),
        toml::Value::Integer(n) => format!("integer {n}"),
        toml::Value::Float(f) => format!("float {f}"),
        toml::Value::Boolean(b) => format!("boolean {b}"),
        toml::Value::Array(_) => "an array".to_owned(),
        toml::Value::Table(_) => "a table".to_owned(),
        toml::Value::Datetime(dt) => format!("datetime {dt}"),
    }
}

fn value_as_f64(value: &toml::Value) -> Option<f64> {
    match value {
        toml::Value::Integer(n) => Some(*n as f64),
        toml::Value::Float(f) => Some(*f),
        _ => None,
    }
}

/// Format a range bound without noise: whole numbers render bare (`4`),
/// fractions keep their decimals (`0.25`).
pub fn fmt_bound(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        let as_int = v as i64;
        if as_int as f64 == v {
            return format!("{as_int}");
        }
    }
    format!("{v}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_tags_round_trip() {
        for tag in [
            "bool",
            "int",
            "float",
            "string",
            "enum",
            "path",
            "url",
            "duration-ms",
            "percent",
            "list<string>",
            "list<path>",
            "list<url>",
            "list<enum>",
        ] {
            assert_eq!(
                SettingType::parse(tag).map(SettingType::as_str),
                Some(tag),
                "type tag `{tag}` must round-trip"
            );
        }
        assert!(SettingType::parse("list<bool>").is_none());
        assert!(SettingType::parse("bogus").is_none());
    }

    #[test]
    fn control_selection_follows_doc_07_g1() {
        assert_eq!(SettingType::Bool.control().as_str(), "toggle");
        assert_eq!(SettingType::Enum.control().as_str(), "select");
        assert_eq!(SettingType::Int.control().as_str(), "slider");
        assert_eq!(SettingType::Percent.control().as_str(), "slider");
        assert_eq!(SettingType::String.control().as_str(), "input");
        assert_eq!(
            SettingType::List(ListElement::String).control().as_str(),
            "list"
        );
        assert_eq!(SettingType::Path.control().as_str(), "path-picker");
        assert_eq!(SettingType::DurationMs.control().as_str(), "duration");
    }

    #[test]
    fn screens_in_use_sorts_and_dedups() {
        let schema = SettingsSchema {
            areas: vec!["a".into(), "b".into()],
            settings: vec![
                SettingDef {
                    key: "b.one".into(),
                    ty: SettingType::Bool,
                    domain: Domain::Unconstrained,
                    default: DefaultValue::Literal(toml::Value::Boolean(true)),
                    description: "settings.b.one".into(),
                    restart_required: false,
                    placement: UiPlacement {
                        screen: "b".into(),
                        section: "s".into(),
                        order: 20,
                        ui: true,
                    },
                    cli: CliSpec {
                        flag: "--b-one".into(),
                        short: String::new(),
                    },
                    feature_gate: String::new(),
                    since: "0.1.0".into(),
                    scope: Scope::User,
                    array_policy: ArrayPolicy::Replace,
                    advanced: false,
                    deprecated_in: String::new(),
                    line: 1,
                    index: 0,
                },
                SettingDef {
                    key: "a.two".into(),
                    ty: SettingType::Bool,
                    domain: Domain::Unconstrained,
                    default: DefaultValue::Literal(toml::Value::Boolean(false)),
                    description: "settings.a.two".into(),
                    restart_required: false,
                    placement: UiPlacement {
                        screen: "a".into(),
                        section: "s".into(),
                        order: 10,
                        ui: true,
                    },
                    cli: CliSpec {
                        flag: "--a-two".into(),
                        short: String::new(),
                    },
                    feature_gate: String::new(),
                    since: "0.1.0".into(),
                    scope: Scope::User,
                    array_policy: ArrayPolicy::Replace,
                    advanced: false,
                    deprecated_in: String::new(),
                    line: 9,
                    index: 1,
                },
            ],
        };
        assert_eq!(
            schema.screens_in_use(),
            vec!["a".to_string(), "b".to_string()]
        );
        let screen_a = schema.screen_entries("a");
        assert_eq!(screen_a.len(), 1);
        assert_eq!(screen_a[0].key, "a.two");
    }
}
