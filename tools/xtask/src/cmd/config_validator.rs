//! `xtask config-validator` — product.toml vs product.schema.json (issue #20,
//! card 8.F1). This is the subcommand product.toml's own header names
//! ("Validated by product.schema.json via `cargo xtask config-validator`",
//! MP §5) — until now it did not exist (CI covered product.toml only via the
//! `product-config` crate's committed test suite).
//!
//! # Checks
//!
//! | # | Group | What it proves |
//! |---|-------|----------------|
//!   S | structural | JSON-Schema subset walk of product.toml against product.schema.json: required keys, `additionalProperties` (false or per-key schema), type mismatches, `enum`, `not.enum` (reserved names), `minLength`/`maxLength`, `minimum`/`maximum`, `minItems`/`maxItems` + `items`, `$ref` into `$defs` |
//!   X | cross-file | brand asset paths exist; `defaults.locale` resolves to `locales/<tag>.toml`; `defaults.theme`/`profile` validate against the `themes/`/`profiles/` listings (empty listing = honest note, not a violation — those planes are parked, doc 07 §9 C4); `identity.description_key` + `window.title_idle_key` exist in the reference catalog `locales/en-US.toml`; `title_template` variables ⊆ {product_name, media_title}; `telemetry.enabled = true` ⇒ `urls.privacy` present; `schema_version` == max(migrations `to`) or 1; app ids == `reverse_dns` + "." + PascalCase(name) (doc 07 §3.3(b)) |
//!
//! # Honest v0 limits (notes, printed every run)
//!
//! * `pattern` constraints are NOT enforced — std has no regex. The
//!   `product-config` crate's committed CI suite owns them (msgid/bcp47/
//!   https/hex shapes); this tool owns the existence/join checks above.
//! * Rule F1/F2 (feature ids vs the engine feature registry) needs the
//!   registry surface; v0 checks that every `features.*` value is boolean.
//! * Magic-byte checks for brand assets are the crate's, not this tool's.
//!
//! Exit codes: 0 pass | 1 violations | 2 xtask error (unreadable files).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::XtaskError;
use crate::cmd::toml_lite::{self, Value as TValue};

const USAGE: &str = "\
Usage: cargo xtask config-validator [--root <dir>] [--product <path>] [--schema <path>]

Validates product.toml against product.schema.json plus the cross-file
conventions of doc 07 §3.3 (issue #20, card 8.F1):

  S  structural   schema walk: required/unknown keys, types, enums, ranges
  X  cross-file   asset paths, locale/theme/profile joins, msgid existence,
                  title variables, telemetry->privacy, schema_version,
                  reverse-dns app ids

  --root <dir>     workspace root (default: walk up from the cwd)
  --product <p>    product TOML (default <root>/product.toml)
  --schema <p>     schema JSON (default <root>/product.schema.json)

Exit code: 0 pass | 1 violations | 2 xtask error.";

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut root_override: Option<PathBuf> = None;
    let mut product_path: Option<PathBuf> = None;
    let mut schema_path: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(ExitCode::SUCCESS);
            }
            "--root" => {
                i += 1;
                let Some(dir) = args.get(i) else {
                    return Err(XtaskError::Usage(format!(
                        "--root requires a directory\n{USAGE}"
                    )));
                };
                root_override = Some(PathBuf::from(dir));
            }
            "--product" => {
                i += 1;
                let Some(p) = args.get(i) else {
                    return Err(XtaskError::Usage(format!(
                        "--product requires a path\n{USAGE}"
                    )));
                };
                product_path = Some(PathBuf::from(p));
            }
            "--schema" => {
                i += 1;
                let Some(p) = args.get(i) else {
                    return Err(XtaskError::Usage(format!(
                        "--schema requires a path\n{USAGE}"
                    )));
                };
                schema_path = Some(PathBuf::from(p));
            }
            other => {
                return Err(XtaskError::Usage(format!(
                    "unexpected argument `{other}`\n{USAGE}"
                )));
            }
        }
        i += 1;
    }

    let root = match root_override {
        Some(dir) => dir,
        None => find_root()?,
    };
    let product = product_path.unwrap_or_else(|| root.join("product.toml"));
    let schema = schema_path.unwrap_or_else(|| root.join("product.schema.json"));

    let product_text = std::fs::read_to_string(&product)
        .map_err(|e| XtaskError::io(format!("reading {}", product.display()), e))?;
    let schema_text = std::fs::read_to_string(&schema)
        .map_err(|e| XtaskError::io(format!("reading {}", schema.display()), e))?;

    println!("=== LUMEN CONFIG-VALIDATOR REPORT ===");
    println!("product: {}", product.display());
    println!("schema:  {}", schema.display());

    let mut violations: Vec<String> = Vec::new();
    let mut notes: Vec<String> = vec![
        "pattern constraints not enforced by this v0 (std has no regex; the product-config crate's CI suite owns them)".into(),
    ];

    let doc = match toml_lite::parse(&product_text) {
        Ok(doc) => doc,
        Err(err) => {
            println!("parse: product.toml: {err}");
            println!("result: FAILURE");
            return Ok(ExitCode::from(1));
        }
    };
    let schema_json = match json::parse(&schema_text) {
        Ok(v) => v,
        Err(err) => {
            println!("parse: product.schema.json: {err}");
            println!("result: FAILURE");
            return Ok(ExitCode::from(1));
        }
    };

    // S: structural walk over the merged table tree.
    let tree = build_tree(&doc);
    if let json::Json::Obj(schema_root) = &schema_json {
        walk(schema_root, &tree, "", &schema_json, &mut violations);
    } else {
        violations.push("product.schema.json: root is not an object".to_string());
    }

    // X: cross-file checks.
    cross_file(&root, &doc, &mut violations, &mut notes);

    println!("tables: {}", doc.tables.len());
    for note in &notes {
        println!("note: {note}");
    }
    if violations.is_empty() {
        println!("result: PASS");
        Ok(ExitCode::SUCCESS)
    } else {
        for v in &violations {
            println!("violation: {v}");
        }
        println!("result: FAILURE ({} violation(s))", violations.len());
        Ok(ExitCode::from(1))
    }
}

// ---------------------------------------------------------------------------
// TOML tree building
// ---------------------------------------------------------------------------

/// Merge the flat table list (+ bare root entries) into one nested value
/// tree so the schema walk can descend `identity.ids.windows` naturally.
fn build_tree(doc: &toml_lite::Document) -> TValue {
    let mut root: Vec<(String, TValue)> = doc
        .root_entries
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for table in &doc.tables {
        insert_table(&mut root, &table.path, &table.entries);
    }
    TValue::Table(root)
}

fn insert_table(into: &mut Vec<(String, TValue)>, path: &[String], entries: &[(String, TValue)]) {
    let Some((head, rest)) = path.split_first() else {
        return;
    };
    let existing_idx = into.iter().position(|(k, _)| k == head);
    match existing_idx {
        Some(idx) if matches!(into[idx].1, TValue::Table(_)) && rest.is_empty() => {
            if let TValue::Table(obj) = &mut into[idx].1 {
                for (k, v) in entries {
                    obj.push((k.clone(), v.clone()));
                }
            }
        }
        Some(idx) if matches!(into[idx].1, TValue::Table(_)) => {
            if let TValue::Table(obj) = &mut into[idx].1 {
                insert_table(obj, rest, entries);
            }
        }
        Some(idx) => {
            // Was a scalar, now a table: overwrite (malformed input is the
            // schema walk's job to flag via type mismatch).
            into[idx].1 = TValue::Table(vec![]);
            insert_table(into, path, entries);
        }
        None => {
            let mut obj: Vec<(String, TValue)> = Vec::new();
            if rest.is_empty() {
                for (k, v) in entries {
                    obj.push((k.clone(), v.clone()));
                }
                into.push((head.clone(), TValue::Table(obj)));
            } else {
                insert_table(&mut obj, rest, entries);
                into.push((head.clone(), TValue::Table(obj)));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Schema walk (S)
// ---------------------------------------------------------------------------

fn lookup<'a>(obj: &'a [(String, json::Json)], key: &str) -> Option<&'a json::Json> {
    obj.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn resolve_ref<'a>(schema: &'a json::Json, r#ref: &str) -> Option<&'a json::Json> {
    // Only internal refs: "#/$defs/name".
    let Some(suffix) = r#ref.strip_prefix("#/") else {
        return None;
    };
    let mut current = schema;
    for segment in suffix.split('/') {
        let json::Json::Obj(obj) = current else {
            return None;
        };
        current = lookup(obj, segment)?;
    }
    Some(current)
}

fn walk(
    schema: &[(String, json::Json)],
    value: &TValue,
    path: &str,
    root_schema: &json::Json,
    violations: &mut Vec<String>,
) {
    let schema_obj = schema;
    // $ref indirection.
    let resolved;
    let mut schema_obj = schema_obj;
    if let Some(json::Json::Str(r#ref)) = lookup(schema_obj, "$ref") {
        if let Some(target) = resolve_ref(root_schema, r#ref) {
            if let json::Json::Obj(t) = target {
                resolved = t;
                schema_obj = resolved;
            }
        }
    }
    let at = if path.is_empty() {
        "<root>".to_string()
    } else {
        path.to_string()
    };

    // type
    if let Some(json::Json::Str(want)) = lookup(schema_obj, "type") {
        let got = value.type_name();
        let ok = match (want.as_str(), value) {
            ("string", TValue::Str(_)) => true,
            ("integer", TValue::Int(_)) => true,
            ("number", TValue::Int(_) | TValue::Float(_)) => true,
            ("boolean", TValue::Bool(_)) => true,
            ("array", TValue::Array(_)) => true,
            ("object", TValue::Table(_)) => true,
            _ => false,
        };
        if !ok {
            violations.push(format!(
                "{at}: type mismatch — schema wants {want}, product.toml has {got}"
            ));
            return; // further constraints are meaningless on the wrong type
        }
    }
    match value {
        TValue::Str(s) => {
            if let Some(json::Json::Arr(items)) = lookup(schema_obj, "enum") {
                if !items.is_empty() && !items.iter().any(|j| j.as_str() == Some(s.as_str())) {
                    let allowed: Vec<String> = items
                        .iter()
                        .filter_map(|j| j.as_str().map(|s| format!("{s:?}")))
                        .collect();
                    violations.push(format!(
                        "{at}: {s:?} not in enum [{alloweds}]",
                        alloweds = allowed.join(", ")
                    ));
                }
            }
            if let Some(json::Json::Obj(not)) = lookup(schema_obj, "not") {
                if let Some(json::Json::Arr(banned)) = lookup(not, "enum") {
                    if banned.iter().any(|j| j.as_str() == Some(s.as_str())) {
                        violations.push(format!("{at}: {s:?} is a reserved/banned value"));
                    }
                }
            }
            if let Some(json::Json::Num(n)) = lookup(schema_obj, "minLength") {
                if (s.chars().count() as f64) < *n {
                    violations.push(format!(
                        "{at}: length {} < minLength {n}",
                        s.chars().count()
                    ));
                }
            }
            if let Some(json::Json::Num(n)) = lookup(schema_obj, "maxLength") {
                if (s.chars().count() as f64) > *n {
                    violations.push(format!(
                        "{at}: length {} > maxLength {n}",
                        s.chars().count()
                    ));
                }
            }
        }
        TValue::Int(i) => {
            if let Some(json::Json::Num(n)) = lookup(schema_obj, "minimum") {
                if (*i as f64) < *n {
                    violations.push(format!("{at}: {i} < minimum {n}"));
                }
            }
            if let Some(json::Json::Num(n)) = lookup(schema_obj, "maximum") {
                if (*i as f64) > *n {
                    violations.push(format!("{at}: {i} > maximum {n}"));
                }
            }
        }
        TValue::Float(f) => {
            if let Some(json::Json::Num(n)) = lookup(schema_obj, "minimum") {
                if f < n {
                    violations.push(format!("{at}: {f} < minimum {n}"));
                }
            }
            if let Some(json::Json::Num(n)) = lookup(schema_obj, "maximum") {
                if f > n {
                    violations.push(format!("{at}: {f} > maximum {n}"));
                }
            }
        }
        TValue::Array(items) => {
            if let Some(json::Json::Num(n)) = lookup(schema_obj, "minItems") {
                if (items.len() as f64) < *n {
                    violations.push(format!("{at}: {} items < minItems {n}", items.len()));
                }
            }
            if let Some(json::Json::Num(n)) = lookup(schema_obj, "maxItems") {
                if (items.len() as f64) > *n {
                    violations.push(format!("{at}: {} items > maxItems {n}", items.len()));
                }
            }
            if let Some(item_schema) = lookup(schema_obj, "items").and_then(|j| j.as_obj()) {
                for (idx, item) in items.iter().enumerate() {
                    walk(
                        item_schema,
                        item,
                        &format!("{at}[{idx}]"),
                        root_schema,
                        violations,
                    );
                }
            }
        }
        TValue::Table(obj) => {
            if let Some(json::Json::Arr(required)) = lookup(schema_obj, "required") {
                for want in required.iter().filter_map(|j| j.as_str()) {
                    if !obj.iter().any(|(k, _)| k == want) {
                        violations.push(format!("{at}: missing required key `{want}`"));
                    }
                }
            }
            let properties = lookup(schema_obj, "properties").and_then(|j| j.as_obj());
            match lookup(schema_obj, "additionalProperties") {
                Some(json::Json::Bool(false)) => {
                    for (k, _) in obj {
                        let known = properties.is_some_and(|props| lookup(props, k).is_some());
                        if !known {
                            violations.push(format!(
                                "{at}: unknown key `{k}` (additionalProperties: false)"
                            ));
                        }
                    }
                }
                Some(other) if !matches!(other, json::Json::Bool(true)) => {
                    // additionalProperties is a schema (e.g. features: {type: boolean}).
                    let extra_schema = other.as_obj().unwrap_or(&[]);
                    for (k, v) in obj {
                        let known = properties.is_some_and(|props| lookup(props, k).is_some());
                        if !known {
                            walk(
                                extra_schema,
                                v,
                                &format!("{at}.{k}"),
                                root_schema,
                                violations,
                            );
                        }
                    }
                }
                _ => {}
            }
            if let Some(props) = properties {
                for (k, v) in obj {
                    if let Some(prop_schema) = lookup(props, k) {
                        let prop_schema = prop_schema.as_obj().unwrap_or(&[]);
                        walk(
                            prop_schema,
                            v,
                            &format!("{at}.{k}"),
                            root_schema,
                            violations,
                        );
                    }
                }
            }
        }
        TValue::Bool(_) => {}
    }
}

// ---------------------------------------------------------------------------
// Cross-file checks (X)
// ---------------------------------------------------------------------------

fn cross_file(
    root: &Path,
    doc: &toml_lite::Document,
    violations: &mut Vec<String>,
    notes: &mut Vec<String>,
) {
    let get_str = |dotted: &str| doc.get(dotted).and_then(TValue::as_str);

    // Brand assets must exist (doc 07 §2.3/§3.3).
    for key in [
        "brand.icon_ico",
        "brand.icon_icns",
        "brand.icon_png_512",
        "brand.logo_svg",
    ] {
        if let Some(p) = get_str(key) {
            if !root.join(p).exists() {
                violations.push(format!(
                    "{key}: asset path `{p}` does not exist under the root"
                ));
            }
        }
    }

    // defaults joins (doc 07 §3.2 injected enums).
    let locale = get_str("defaults.locale").unwrap_or("");
    if !locale.is_empty() && !root.join(format!("locales/{locale}.toml")).exists() {
        violations.push(format!(
            "defaults.locale: locales/{locale}.toml does not exist"
        ));
    }
    for (key, dir) in [
        ("defaults.theme", "themes"),
        ("defaults.profile", "profiles"),
    ] {
        if let Some(value) = get_str(key) {
            let candidates = list_toml_ids(&root.join(dir));
            match candidates {
                None => notes.push(format!("{key}: `{dir}/` does not exist yet — enum injection skipped (parked plane, doc 07 §9 C4)")),
                Some(ids) if ids.is_empty() => notes.push(format!(
                    "{key}: `{dir}/` has no *.toml candidates — enum injection skipped until the plane lands (doc 07 §9 C4)"
                )),
                Some(ids) => {
                    if !ids.iter().any(|id| id == value) {
                        violations.push(format!("{key}: {value:?} not in `{dir}/` listing {ids:?}"));
                    }
                }
            }
        }
    }

    // Reference catalog: msgids must exist in locales/en-US.toml (doc 07 §6.3 rule 4).
    let reference = root.join("locales/en-US.toml");
    let catalog = std::fs::read_to_string(&reference)
        .ok()
        .and_then(|text| toml_lite::parse(&text).ok());
    for key in ["identity.description_key", "window.title_idle_key"] {
        let Some(msgid) = get_str(key) else { continue };
        match &catalog {
            Some(cat) => {
                if !cat.has_root_key(msgid) {
                    violations.push(format!("{key}: msgid {msgid:?} not in the reference catalog locales/en-US.toml"));
                }
            }
            None => notes.push(format!("{key}: reference catalog locales/en-US.toml unreadable — msgid existence not checked")),
        }
    }

    // title_template variables (doc 07 §2.2).
    if let Some(template) = get_str("window.title_template") {
        let mut rest: &str = template;
        while let Some(open) = rest.find('{') {
            let Some(close_rel) = rest[open..].find('}') else {
                break;
            };
            let variable = &rest[open + 1..open + close_rel];
            if variable != "product_name" && variable != "media_title" {
                violations.push(format!(
                    "window.title_template: unknown variable {{{variable}}} (only {{product_name}} and {{media_title}} are defined)"
                ));
            }
            rest = &rest[open + close_rel + 1..];
        }
    }

    // telemetry.enabled = true requires urls.privacy (doc 07 §3.3 cross-field).
    if doc.get("telemetry.enabled") == Some(&TValue::Bool(true))
        && doc.get("urls.privacy").is_none()
    {
        violations
            .push("telemetry.enabled = true but urls.privacy is absent (doc 07 §3.3)".to_string());
    }

    // schema_version == max(migrations.entries[].to), or 1 when empty.
    if let Some(TValue::Int(version)) = doc.get("schema_version") {
        let entries = doc
            .get("migrations.entries")
            .and_then(|v| match v {
                TValue::Array(a) => Some(a.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let mut max_to: Option<i64> = None;
        let mut prev_to: Option<i64> = None;
        for entry in &entries {
            let get = |k: &str| -> Option<i64> {
                entry
                    .as_table()
                    .and_then(|t| t.iter().find(|(kk, _)| kk == k))
                    .and_then(|(_, v)| v.as_int())
            };
            let (Some(from), Some(to)) = (get("from"), get("to")) else {
                violations.push("migrations.entries: row missing from/to".to_string());
                continue;
            };
            if from >= to {
                violations.push(format!("migrations.entries: from {from} !< to {to}"));
            }
            if let Some(prev) = prev_to {
                if to != prev + 1 {
                    violations.push(format!("migrations.entries: to {to} does not continue ascending from {prev} (no gaps)"));
                }
            }
            prev_to = Some(to);
            max_to = Some(max_to.map_or(to, |m| m.max(to)));
        }
        let expected = max_to.unwrap_or(1);
        if *version != expected {
            violations.push(format!(
                "schema_version {version} != {expected} (max migrations to, or 1 when empty)"
            ));
        }
    }

    // App ids == reverse_dns + "." + PascalCase(name) (doc 07 §3.3(b)).
    let name = get_str("identity.name").unwrap_or("");
    let reverse_dns = get_str("identity.ids.reverse_dns").unwrap_or("");
    if !name.is_empty() && !reverse_dns.is_empty() {
        let expected = format!("{reverse_dns}.{}", pascal_case(name));
        for key in [
            "identity.ids.windows.app_user_model_id",
            "identity.ids.macos.bundle_id",
            "identity.ids.linux.appstream_id",
            "identity.ids.web.app_id",
        ] {
            if let Some(id) = get_str(key) {
                if id != expected {
                    violations.push(format!(
                        "{key}: {id:?} != reverse_dns + \".\" + PascalCase(name) ({expected:?})"
                    ));
                }
            }
        }
    }
}

/// List `<dir>/*.toml` file stems, or `None` when the directory is absent.
fn list_toml_ids(dir: &Path) -> Option<Vec<String>> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut ids = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "toml") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                ids.push(stem.to_string());
            }
        }
    }
    ids.sort();
    Some(ids)
}

/// PascalCase of a product name: capitalize each alphanumeric run's first
/// letter ("Lumen" -> "Lumen", "My Player" -> "MyPlayer").
fn pascal_case(name: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for c in name.chars() {
        if c.is_alphanumeric() {
            if capitalize {
                out.extend(c.to_uppercase());
                capitalize = false;
            } else {
                out.push(c);
            }
        } else {
            capitalize = true;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Root discovery
// ---------------------------------------------------------------------------

fn find_root() -> Result<PathBuf, XtaskError> {
    let cwd =
        std::env::current_dir().map_err(|e| XtaskError::io("determining current directory", e))?;
    let mut dir: &Path = &cwd;
    loop {
        if dir.join("product.toml").is_file() {
            return Ok(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => {
                return Err(XtaskError::internal(
                    "workspace root not found — run inside the Lumen workspace or pass --root",
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// json: minimal std-only JSON parser (schema files only)
// ---------------------------------------------------------------------------

mod json {
    /// A parsed JSON value.
    #[derive(Debug, Clone, PartialEq)]
    pub enum Json {
        Null,
        Bool(bool),
        Num(f64),
        Str(String),
        Arr(Vec<Json>),
        Obj(Vec<(String, Json)>),
    }

    impl Json {
        pub fn as_str(&self) -> Option<&str> {
            match self {
                Json::Str(s) => Some(s),
                _ => None,
            }
        }

        pub fn as_obj(&self) -> Option<&[(String, Json)]> {
            match self {
                Json::Obj(o) => Some(o),
                _ => None,
            }
        }
    }

    /// Parse failure with a 1-based line and column.
    #[derive(Debug)]
    pub struct ParseError {
        pub line: usize,
        pub col: usize,
        pub message: String,
    }

    impl std::fmt::Display for ParseError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "line {}:{}: {}", self.line, self.col, self.message)
        }
    }

    pub fn parse(text: &str) -> Result<Json, ParseError> {
        let mut p = Parser {
            bytes: text.as_bytes(),
            pos: 0,
        };
        p.skip_ws();
        let value = p.value(0)?;
        p.skip_ws();
        if p.pos != p.bytes.len() {
            return Err(p.err("trailing content after the JSON value"));
        }
        Ok(value)
    }

    struct Parser<'a> {
        bytes: &'a [u8],
        pos: usize,
    }

    impl<'a> Parser<'a> {
        fn err(&self, msg: &str) -> ParseError {
            let mut line = 1;
            let mut col = 1;
            for &b in &self.bytes[..self.pos.min(self.bytes.len())] {
                if b == b'\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
            }
            ParseError {
                line,
                col,
                message: msg.to_string(),
            }
        }

        fn peek(&self) -> Option<u8> {
            self.bytes.get(self.pos).copied()
        }

        fn skip_ws(&mut self) {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.pos += 1;
            }
        }

        fn expect(&mut self, b: u8) -> Result<(), ParseError> {
            if self.peek() == Some(b) {
                self.pos += 1;
                Ok(())
            } else {
                Err(self.err(&format!("expected `{}`", b as char)))
            }
        }

        fn literal(&mut self, want: &str) -> Result<(), ParseError> {
            if self.bytes[self.pos..].starts_with(want.as_bytes()) {
                self.pos += want.len();
                Ok(())
            } else {
                Err(self.err(&format!("expected `{want}`")))
            }
        }

        fn value(&mut self, depth: usize) -> Result<Json, ParseError> {
            if depth > 64 {
                return Err(self.err("nesting deeper than 64"));
            }
            match self.peek() {
                Some(b'{') => self.object(depth),
                Some(b'[') => self.array(depth),
                Some(b'"') => Ok(Json::Str(self.string()?)),
                Some(b't') => {
                    self.literal("true")?;
                    Ok(Json::Bool(true))
                }
                Some(b'f') => {
                    self.literal("false")?;
                    Ok(Json::Bool(false))
                }
                Some(b'n') => {
                    self.literal("null")?;
                    Ok(Json::Null)
                }
                Some(b'-' | b'0'..=b'9') => self.number(),
                _ => Err(self.err("unexpected character")),
            }
        }

        fn object(&mut self, depth: usize) -> Result<Json, ParseError> {
            self.expect(b'{')?;
            let mut entries = Vec::new();
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.pos += 1;
                return Ok(Json::Obj(entries));
            }
            loop {
                self.skip_ws();
                let key = self.string()?;
                self.skip_ws();
                self.expect(b':')?;
                self.skip_ws();
                let value = self.value(depth + 1)?;
                entries.push((key, value));
                self.skip_ws();
                match self.peek() {
                    Some(b',') => {
                        self.pos += 1;
                    }
                    Some(b'}') => {
                        self.pos += 1;
                        return Ok(Json::Obj(entries));
                    }
                    _ => return Err(self.err("expected `,` or `}`")),
                }
            }
        }

        fn array(&mut self, depth: usize) -> Result<Json, ParseError> {
            self.expect(b'[')?;
            let mut items = Vec::new();
            self.skip_ws();
            if self.peek() == Some(b']') {
                self.pos += 1;
                return Ok(Json::Arr(items));
            }
            loop {
                self.skip_ws();
                items.push(self.value(depth + 1)?);
                self.skip_ws();
                match self.peek() {
                    Some(b',') => {
                        self.pos += 1;
                    }
                    Some(b']') => {
                        self.pos += 1;
                        return Ok(Json::Arr(items));
                    }
                    _ => return Err(self.err("expected `,` or `]`")),
                }
            }
        }

        fn string(&mut self) -> Result<String, ParseError> {
            self.expect(b'"')?;
            let mut out = String::new();
            loop {
                match self.peek() {
                    None => return Err(self.err("unterminated string")),
                    Some(b'"') => {
                        self.pos += 1;
                        return Ok(out);
                    }
                    Some(b'\\') => {
                        self.pos += 1;
                        match self.peek() {
                            Some(b'"') => out.push('"'),
                            Some(b'\\') => out.push('\\'),
                            Some(b'/') => out.push('/'),
                            Some(b'n') => out.push('\n'),
                            Some(b't') => out.push('\t'),
                            Some(b'r') => out.push('\r'),
                            Some(b'b') => out.push('\u{0008}'),
                            Some(b'f') => out.push('\u{000C}'),
                            Some(b'u') => {
                                self.pos += 1;
                                let cp = self.hex4()?;
                                // Surrogate pairs (schema files use BMP only,
                                // but handle honestly anyway).
                                if (0xD800..0xDC00).contains(&cp) {
                                    if self.peek() == Some(b'\\') {
                                        self.pos += 2; // consume \u
                                        let low = self.hex4()?;
                                        let combined =
                                            0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00);
                                        out.push(char::from_u32(combined).unwrap_or('\u{FFFD}'));
                                    } else {
                                        out.push('\u{FFFD}');
                                    }
                                } else {
                                    out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                                }
                                continue; // hex4 advanced pos already
                            }
                            _ => return Err(self.err("bad escape")),
                        }
                        self.pos += 1;
                    }
                    Some(_) => {
                        // Copy one UTF-8 scalar.
                        let start = self.pos;
                        let len = utf8_len(self.bytes[start]);
                        let end = (start + len).min(self.bytes.len());
                        match std::str::from_utf8(&self.bytes[start..end]) {
                            Ok(s) => out.push_str(s),
                            Err(_) => out.push('\u{FFFD}'),
                        }
                        self.pos = end;
                    }
                }
            }
        }

        fn hex4(&mut self) -> Result<u32, ParseError> {
            let mut value = 0u32;
            for _ in 0..4 {
                let Some(b) = self.peek() else {
                    return Err(self.err("truncated \\u escape"));
                };
                let digit = (b as char)
                    .to_digit(16)
                    .ok_or_else(|| self.err("bad hex digit"))?;
                value = value * 16 + digit;
                self.pos += 1;
            }
            Ok(value)
        }

        fn number(&mut self) -> Result<Json, ParseError> {
            let start = self.pos;
            if self.peek() == Some(b'-') {
                self.pos += 1;
            }
            while matches!(
                self.peek(),
                Some(b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
            ) {
                self.pos += 1;
            }
            let text = std::str::from_utf8(&self.bytes[start..self.pos])
                .map_err(|_| self.err("bad number"))?;
            text.parse::<f64>()
                .map(Json::Num)
                .map_err(|_| self.err("bad number"))
        }
    }

    fn utf8_len(first: u8) -> usize {
        match first {
            0x00..=0x7F => 1,
            0xC0..=0xDF => 2,
            0xE0..=0xEF => 3,
            _ => 4,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_the_schema_shapes() {
            let v = parse(r#"{"a": [1, -2.5, true, null], "b": {"c": "x\ny"}}"#).expect("parses");
            let Json::Obj(obj) = &v else { panic!("obj") };
            assert_eq!(obj[0].0, "a");
            let Json::Arr(items) = &obj[0].1 else {
                panic!("arr")
            };
            assert_eq!(items.len(), 4);
            assert_eq!(items[1], Json::Num(-2.5));
            assert_eq!(obj[1].1.as_obj().unwrap()[0].1.as_str(), Some("x\ny"));
        }

        #[test]
        fn rejects_truncated_json() {
            assert!(parse("{\"a\": ").is_err());
            assert!(parse("[1, 2").is_err());
            assert!(parse("12x").is_err());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_case_capitalizes_runs() {
        assert_eq!(pascal_case("Lumen"), "Lumen");
        assert_eq!(pascal_case("my player"), "MyPlayer");
        assert_eq!(pascal_case("lumen-tv"), "LumenTv");
    }
}
