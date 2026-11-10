//! `xtask sbom` — std-only CycloneDX 1.5 SBOM generator (issue #52,
//! doc 11 §3.4; task 7.w23f / agent w23-f).
//!
//! # What this is
//!
//! Emits `docs/security/sbom/cyclonedx.json` — a CycloneDX 1.5 JSON SBOM
//! of the **resolved Cargo.lock graph** (all 156 lockfile packages:
//! registry dependencies and workspace members alike), with the product
//! identity from `product.toml` as the BOM subject. This closes the last
//! deliverable of issue #52 named by the supply-chain sweep
//! (`docs/security/sweep-supply-chain.md`).
//!
//! # Why hand-rolled, not `cargo-cyclonedx`
//!
//! Doc 11 §3.4 sketches `cargo-cyclonedx` + `cargo auditable`. Both are
//! new third-party tools; AGENTS.md §3.2 (clean-room, dependency approval)
//! and the xtask C5 posture (std-only, zero deps — nothing to approve,
//! nothing to audit) prefer a generator whose entire trust base is this
//! file. The deviation is deliberate and recorded here: the lockfile is a
//! fully-resolved, checksum-carrying dependency list, so a faithful
//! CycloneDX rendering of it needs no cargo metadata subprocess and no
//! network. [PROPOSAL] a future card may swap in cargo-cyclonedx if the
//! Lead prefers the ecosystem tool; the committed artifact's shape
//! (format 1.5, subject metadata, per-component hashes) is compatible.
//!
//! # What the SBOM carries (and honestly does not)
//!
//! Per component (one per `[[package]]` row, sorted by name then version
//! for byte-stability across lockfile re-orders):
//!
//! * `type: "library"`, `name`, `version`, `scope: "required"` (every
//!   lockfile row is in the resolved build graph);
//! * `bom-ref`: `pkg:cargo/<name>@<version>` for registry packages, the
//!   opaque `<name>@<version>` for path packages (workspace members have
//!   no package-manager identity — a purl would claim one they lack);
//! * `purl` for registry packages (crates.io is the `cargo` purl type);
//! * `externalReferences` → `distribution` → the crates.io API URL for
//!   the exact version, derived from the lockfile `source` registry URL;
//! * `hashes`: the lockfile `checksum` as `SHA-256` where present
//!   (registry packages only — path packages carry none);
//! * `properties`: `lumen:cargo-lock-source` (the raw lockfile `source`
//!   string, or `path` when absent — exactly what the lockfile encodes).
//!
//! NOT carried, because `Cargo.lock` does not contain it (never
//! fabricate): **licenses** (G3 cargo-deny owns license policy — recorded
//! in a metadata property), authors, CPEs, and any timestamp. The
//! `serialNumber` is a deterministic content-derived UUIDv8, NOT a random
//! UUID: same lockfile + product identity → byte-identical SBOM, which is
//! what the `--check` drift gate (below) and CI need.
//!
//! # Modes (mirrors fixtures / schema-gen drift conventions)
//!
//! * `cargo xtask sbom` — (re)generate and write the committed artifact.
//! * `cargo xtask sbom --check` — regenerate in memory and byte-compare
//!   against the committed file: exit 0 no drift / 1 drift (with the
//!   "run `cargo xtask sbom` and commit" remediation) / 2 xtask error.
//!
//! Wired into CI (cargo-deny job, after the deny check) and
//! `scripts/full_gate.sh` step 9, so an unregenerated SBOM after a
//! dependency change fails the merge gate.
//!
//! # Ownership
//!
//! Registered under task 7.w23f's granted ownership of `tools/xtask/src/**`
//! (issue #52), same precedent as check-panics/7-e, visual/4.G1 and the
//! 7.w21a gates. Reuses `cmd::toml_lite` (the shared std-only TOML-subset
//! reader, issue #20) for both `Cargo.lock` and `product.toml`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use super::toml_lite::{self, Value};
use crate::XtaskError;

// ---------------------------------------------------------------------------
// Usage / constants
// ---------------------------------------------------------------------------

const USAGE: &str = "\
Usage: cargo xtask sbom [--check] [--root <dir>] [--lock <path>] [--out <path>]

Generates the CycloneDX 1.5 SBOM (issue #52, doc 11 §3.4) from the resolved
Cargo.lock graph + the product identity in product.toml, std-only (C5: no
cargo-cyclonedx, no cargo metadata subprocess, no network):

  components    one per [[package]] row: name, version, registry source,
                checksum (SHA-256) where present, purl for registry packages
  metadata      product name / version / publisher as the BOM subject
  dependencies  the lockfile dependency graph (resolved bom-refs)
  serialNumber  deterministic content-derived UUIDv8 (byte-stable output)

Licenses are NOT included (Cargo.lock carries none; G3 cargo-deny owns
license policy). No timestamp is emitted — determinism is a feature here.

  --check       verify the committed SBOM matches regeneration byte-for-byte
                (exit 0 = no drift, 1 = drift, 2 = error)
  --root <dir>  workspace root (default: walk up from the cwd)
  --lock <p>    lockfile (default <root>/Cargo.lock)
  --out <p>     output path (default <root>/docs/security/sbom/cyclonedx.json)

Exit code: 0 success/no drift | 1 drift | 2 xtask error.";

/// Where the committed SBOM lives, relative to the workspace root
/// (issue #52's named evidence directory, `docs/security/sbom/`).
const SBOM_REL: &str = "docs/security/sbom/cyclonedx.json";

/// The crates.io sparse-index registry URL as it appears in Cargo.lock.
const CRATES_IO_INDEX: &str = "registry+https://github.com/rust-lang/crates.io-index";

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Entry point for `cargo xtask sbom`.
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut check_only = false;
    let mut root_override: Option<PathBuf> = None;
    let mut lock_override: Option<PathBuf> = None;
    let mut out_override: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(ExitCode::SUCCESS);
            }
            "--check" => check_only = true,
            "--root" | "--lock" | "--out" => {
                let flag = args[i].as_str();
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err(XtaskError::Usage(format!(
                        "{flag} requires a path\n{USAGE}"
                    )));
                };
                let slot = match flag {
                    "--root" => &mut root_override,
                    "--lock" => &mut lock_override,
                    _ => &mut out_override,
                };
                *slot = Some(PathBuf::from(value));
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
    let lock_path = lock_override.unwrap_or_else(|| root.join("Cargo.lock"));
    let out_path = out_override.unwrap_or_else(|| root.join(SBOM_REL));

    let lock_text = std::fs::read_to_string(&lock_path)
        .map_err(|e| XtaskError::io(format!("reading {}", lock_path.display()), e))?;
    let product_path = root.join("product.toml");
    let product_text = std::fs::read_to_string(&product_path)
        .map_err(|e| XtaskError::io(format!("reading {}", product_path.display()), e))?;

    println!("=== LUMEN SBOM REPORT ===");
    println!("lock: {}", lock_path.display());
    println!("out:  {}", out_path.display());

    let lock = parse_lock(&lock_text).map_err(XtaskError::internal)?;
    let identity = read_product_identity(&product_text).map_err(XtaskError::internal)?;
    let sbom = build_sbom(&identity, &lock).map_err(XtaskError::internal)?;

    let registry_count = lock.packages.iter().filter(|p| p.source.is_some()).count();
    let path_count = lock.packages.len() - registry_count;
    println!(
        "components: {} ({} registry, {} workspace/path), lockfile format v{}",
        lock.packages.len(),
        registry_count,
        path_count,
        lock.format_version
    );
    println!("subject:    {} {}", identity.name, identity.version);

    if check_only {
        let existing = match std::fs::read(&out_path) {
            Ok(bytes) => bytes,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                println!(
                    "missing: {} — run `cargo xtask sbom` and commit the generated SBOM",
                    SBOM_REL
                );
                println!("result: DRIFT — regenerate with `cargo xtask sbom` and commit");
                return Ok(ExitCode::from(1));
            }
            Err(source) => {
                return Err(XtaskError::io(
                    format!("reading {}", out_path.display()),
                    source,
                ));
            }
        };
        if existing == sbom.as_bytes() {
            println!("result: no drift — {} bytes verified", existing.len());
            Ok(ExitCode::SUCCESS)
        } else {
            println!(
                "drift: {} differs from the regenerated output — run `cargo xtask sbom` and commit the refreshed SBOM",
                SBOM_REL
            );
            println!("result: DRIFT — regenerate with `cargo xtask sbom` and commit");
            Ok(ExitCode::from(1))
        }
    } else {
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| XtaskError::io(format!("creating {}", parent.display()), e))?;
        }
        std::fs::write(&out_path, &sbom)
            .map_err(|e| XtaskError::io(format!("writing {}", out_path.display()), e))?;
        println!(
            "wrote: {} ({} bytes) — commit it; drift gate: `cargo xtask sbom --check`",
            SBOM_REL,
            sbom.len()
        );
        Ok(ExitCode::SUCCESS)
    }
}

// ---------------------------------------------------------------------------
// Root discovery (config_validator's pattern)
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
// Cargo.lock model (toml_lite)
// ---------------------------------------------------------------------------

/// One `[[package]]` row of the lockfile.
#[derive(Debug, Clone, PartialEq)]
struct LockPackage {
    name: String,
    version: String,
    /// Raw `source` string (`registry+https://…`); `None` for path packages.
    source: Option<String>,
    /// SHA-256 hex from the lockfile, where present.
    checksum: Option<String>,
    /// Dependency descriptors: `"name"` or `"name version"`, source order.
    dependencies: Vec<String>,
}

/// The parsed lockfile.
#[derive(Debug)]
struct LockFile {
    /// The lockfile `version = <n>` root entry (format version).
    format_version: u64,
    packages: Vec<LockPackage>,
}

/// Parse `Cargo.lock` text via the shared TOML-subset reader.
fn parse_lock(text: &str) -> Result<LockFile, String> {
    let doc = toml_lite::parse(text).map_err(|e| format!("Cargo.lock: {e}"))?;
    let format_version = match doc.get("version") {
        Some(Value::Int(v)) if *v >= 1 => *v as u64,
        Some(other) => {
            return Err(format!(
                "Cargo.lock: `version` must be an integer, got {}",
                other.type_name()
            ));
        }
        None => return Err("Cargo.lock: missing `version` root entry".to_string()),
    };
    let mut packages = Vec::new();
    for row in doc.array_rows("package") {
        let mut name = None;
        let mut version = None;
        let mut source = None;
        let mut checksum = None;
        let mut dependencies = Vec::new();
        for (key, value) in &row.entries {
            match key.as_str() {
                "name" | "version" | "source" | "checksum" => {
                    let text = value.as_str().ok_or_else(|| {
                        format!(
                            "package `{key}`: expected a string, got {}",
                            value.type_name()
                        )
                    })?;
                    match key.as_str() {
                        "name" => name = Some(text.to_string()),
                        "version" => version = Some(text.to_string()),
                        "source" => source = Some(text.to_string()),
                        _ => checksum = Some(text.to_string()),
                    }
                }
                "dependencies" => {
                    let items = match value {
                        Value::Array(items) => items,
                        other => {
                            return Err(format!(
                                "package `dependencies`: expected an array, got {}",
                                other.type_name()
                            ));
                        }
                    };
                    for item in items {
                        match item {
                            Value::Str(dep) => dependencies.push(dep.clone()),
                            other => {
                                return Err(format!(
                                    "package `dependencies`: unsupported entry shape ({}) — \
                                     lockfile dialect not understood by this reader",
                                    other.type_name()
                                ));
                            }
                        }
                    }
                }
                // Future lockfile keys (e.g. v5 additions) must fail loudly,
                // not silently drop information from an SBOM.
                other => {
                    return Err(format!(
                        "package: unknown lockfile key `{other}` — extend the sbom reader"
                    ));
                }
            }
        }
        let name = name.ok_or("package: missing `name`")?;
        let version = version.ok_or_else(|| format!("package `{name}`: missing `version`"))?;
        packages.push(LockPackage {
            name,
            version,
            source,
            checksum,
            dependencies,
        });
    }
    if packages.is_empty() {
        return Err("Cargo.lock: no [[package]] rows found".to_string());
    }
    Ok(LockFile {
        format_version,
        packages,
    })
}

// ---------------------------------------------------------------------------
// Product identity (product.toml via toml_lite)
// ---------------------------------------------------------------------------

/// The BOM subject, from `product.toml [identity]`.
#[derive(Debug, Clone, PartialEq)]
struct ProductIdentity {
    /// `identity.name` — the display name.
    name: String,
    /// `identity.display_version` — the human-facing version.
    version: String,
    /// `identity.publisher` — the legal entity.
    publisher: String,
}

fn read_product_identity(text: &str) -> Result<ProductIdentity, String> {
    let doc = toml_lite::parse(text).map_err(|e| format!("product.toml: {e}"))?;
    let str_at = |key: &str| -> Result<String, String> {
        match doc.get(key) {
            Some(Value::Str(s)) => Ok(s.clone()),
            Some(other) => Err(format!(
                "product.toml: `{key}` must be a string, got {}",
                other.type_name()
            )),
            None => Err(format!("product.toml: missing `{key}`")),
        }
    };
    Ok(ProductIdentity {
        name: str_at("identity.name")?,
        version: str_at("identity.display_version")?,
        publisher: str_at("identity.publisher")?,
    })
}

// ---------------------------------------------------------------------------
// Determinism: content fingerprint → UUIDv8 serial number
// ---------------------------------------------------------------------------

/// FNV-1a 64-bit with a caller-chosen offset basis (two different seeds
/// give two independent 64-bit fingerprints → a 128-bit serial).
fn fnv1a_64(seed: u64, bytes: &[u8]) -> u64 {
    let mut hash = seed;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// Deterministic RFC 9562 UUIDv8 (custom version) from the SBOM inputs.
///
/// Non-cryptographic by design: it only needs uniqueness-per-content for
/// the drift gate, never unpredictability. Version nibble 8 and variant
/// bits `10` are set so the string is a structurally valid UUID.
fn serial_uuid(inputs: &str) -> String {
    let h1 = fnv1a_64(0xcbf2_9ce4_8422_2325, inputs.as_bytes());
    let h2 = fnv1a_64(0xd6e8_feb8_6659_8473, inputs.as_bytes());
    let mut bits = (u128::from(h1) << 64) | u128::from(h2);
    // Nibble math: hex digit k of the 32-digit string sits at bits
    // 124-4k. Version (3rd group, digit 12) at shift 76; variant (4th
    // group, digit 16) at shift 60.
    bits = (bits & !(0xf_u128 << 76)) | (8_u128 << 76);
    let d16 = (bits >> 60) & 0xf;
    bits = (bits & !(0xf_u128 << 60)) | ((d16 & 0x3 | 0x8) << 60);
    let hex = format!("{bits:032x}");
    format!(
        "urn:uuid:{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

// ---------------------------------------------------------------------------
// Minimal deterministic JSON writer (std-only, fixed key order)
// ---------------------------------------------------------------------------

/// String-building JSON emitter with 2-space pretty printing. Key order is
/// the caller's — deterministic by construction, unlike a HashMap walk.
struct Json {
    out: String,
    indent: usize,
    /// Per open container: whether no value has been emitted yet.
    first: Vec<bool>,
    /// A key was just written; the value that follows owns no separator.
    pending_key: bool,
}

impl Json {
    fn new() -> Self {
        Json {
            out: String::new(),
            indent: 0,
            first: Vec::new(),
            pending_key: false,
        }
    }

    fn open(&mut self, bracket: char) {
        self.separator();
        self.out.push(bracket);
        self.first.push(true);
        self.indent += 1;
    }

    fn close(&mut self, bracket: char) {
        let had_items = match self.first.pop() {
            Some(first) => !first,
            None => false,
        };
        self.indent -= 1;
        if had_items {
            self.newline();
        }
        self.out.push(bracket);
    }

    fn newline(&mut self) {
        self.out.push('\n');
        for _ in 0..self.indent {
            self.out.push_str("  ");
        }
    }

    /// Emit the value separator (and line break) when needed. A value
    /// directly after its key shares the key's separator.
    fn separator(&mut self) {
        if self.pending_key {
            self.pending_key = false;
            return;
        }
        // At the root (no open container) there is nothing to separate.
        let Some(flag) = self.first.last_mut() else {
            return;
        };
        let is_first = *flag;
        *flag = false;
        if !is_first {
            self.out.push(',');
        }
        self.newline();
    }

    fn key(&mut self, name: &str) {
        self.separator();
        let _ = write!(self.out, "\"{}\": ", json_escape(name));
        self.pending_key = true;
    }

    fn str_value(&mut self, value: &str) {
        self.separator();
        let _ = write!(self.out, "\"{}\"", json_escape(value));
    }

    fn int_value(&mut self, value: u64) {
        self.separator();
        let _ = write!(self.out, "{value}");
    }

    /// Open an array of strings and emit each element on its own line.
    fn str_array(&mut self, values: &[String]) {
        self.open('[');
        for value in values {
            self.str_value(value);
        }
        self.close(']');
    }
}

/// RFC 8259 string escaping (control characters, quote, backslash;
/// non-ASCII passes through as UTF-8).
fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// SBOM construction
// ---------------------------------------------------------------------------

/// purl / bom-ref / distribution URL for a lockfile package.
struct Refs {
    bom_ref: String,
    purl: Option<String>,
    distribution_url: Option<String>,
}

fn package_refs(package: &LockPackage) -> Result<Refs, String> {
    match &package.source {
        Some(source) => {
            if source == CRATES_IO_INDEX {
                // crates.io packages: the canonical cargo purl + the
                // version-exact distribution endpoint.
                let purl = format!("pkg:cargo/{}@{}", package.name, package.version);
                Ok(Refs {
                    bom_ref: purl.clone(),
                    purl: Some(purl),
                    distribution_url: Some(format!(
                        "https://crates.io/api/v1/crates/{}/{}",
                        package.name, package.version
                    )),
                })
            } else {
                // A non-crates.io registry or git source: keep the raw
                // source string as the identity (properties, below) and use
                // the opaque ref — inventing a purl for an unknown registry
                // would fabricate an identity.
                Ok(Refs {
                    bom_ref: format!("{}@{}", package.name, package.version),
                    purl: None,
                    distribution_url: None,
                })
            }
        }
        None => Ok(Refs {
            bom_ref: format!("{}@{}", package.name, package.version),
            purl: None,
            distribution_url: None,
        }),
    }
}

/// Resolve a lockfile dependency descriptor (`"name"` or `"name version"`)
/// to the package it names. Lockfiles are fully resolved by construction,
/// so an unresolvable edge means the reader missed a dialect shape — a
/// loud tool error, never a silent graph hole.
fn resolve_dep(descriptor: &str, packages: &[LockPackage]) -> Result<usize, String> {
    let (name, want_version) = match descriptor.rsplit_once(' ') {
        Some((name, version)) if version.starts_with(|c: char| c.is_ascii_digit()) => {
            (name, Some(version))
        }
        _ => (descriptor, None),
    };
    let matches: Vec<usize> = packages
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            p.name == name
                && match want_version {
                    Some(v) => p.version == v,
                    None => true,
                }
        })
        .map(|(idx, _)| idx)
        .collect();
    match matches.as_slice() {
        [only] => Ok(*only),
        [] => Err(format!(
            "dependency `{descriptor}` matches no lockfile package — \
             the graph is not fully resolved or the reader missed a shape"
        )),
        many => Err(format!(
            "dependency `{descriptor}` is ambiguous between {} packages — \
             the lockfile should have disambiguated with a version",
            many.len()
        )),
    }
}

/// Canonical fingerprint input: subject identity + every package's
/// identity fields + the resolved edges. Order-independent (sorted).
fn fingerprint_input(identity: &ProductIdentity, lock: &LockFile) -> String {
    let mut lines = vec![format!(
        "subject\t{}\t{}\t{}",
        identity.name, identity.version, identity.publisher
    )];
    let mut rows: Vec<String> = lock
        .packages
        .iter()
        .map(|p| {
            format!(
                "pkg\t{}\t{}\t{}\t{}",
                p.name,
                p.version,
                p.source.as_deref().unwrap_or("path"),
                p.checksum.as_deref().unwrap_or("-")
            )
        })
        .collect();
    rows.sort();
    lines.extend(rows);
    lines.join("\n")
}

/// Build the full CycloneDX 1.5 document (deterministic bytes).
fn build_sbom(identity: &ProductIdentity, lock: &LockFile) -> Result<String, String> {
    // Sorted package list (name, version) — byte-stable across lockfile
    // re-orders; the drift gate must fire on set changes, not reorders.
    // Owned clones: the resolver indexes the sorted order.
    let mut sorted: Vec<LockPackage> = lock.packages.to_vec();
    sorted.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));

    let refs: Vec<Refs> = sorted.iter().map(package_refs).collect::<Result<_, _>>()?;

    // Resolve every dependency edge up front (loud failure on gaps).
    let mut edges: Vec<Vec<usize>> = Vec::with_capacity(sorted.len());
    for package in &sorted {
        let mut targets = Vec::with_capacity(package.dependencies.len());
        for descriptor in &package.dependencies {
            targets.push(resolve_dep(descriptor, &sorted)?);
        }
        targets.sort_unstable();
        targets.dedup();
        edges.push(targets);
    }

    // The subject depends on every path-source package (the workspace
    // member set). Shipped-app vs dev-tool is NOT encoded in Cargo.lock;
    // this SBOM describes the whole resolved graph, stated in the module
    // docs and the artifact's README.
    let mut subject_deps: Vec<usize> = sorted
        .iter()
        .enumerate()
        .filter(|(_, p)| p.source.is_none())
        .map(|(idx, _)| idx)
        .collect();
    subject_deps.sort_unstable();
    subject_deps.dedup();

    let subject_ref = format!("{}@{}", identity.name, identity.version);
    let serial = serial_uuid(&fingerprint_input(identity, lock));

    let mut json = Json::new();
    json.open('{');
    json.key("$schema");
    json.str_value("http://cyclonedx.org/schema/bom-1.5.schema.json");
    json.key("bomFormat");
    json.str_value("CycloneDX");
    json.key("specVersion");
    json.str_value("1.5");
    json.key("serialNumber");
    json.str_value(&serial);
    json.key("version");
    json.int_value(1);

    // --- metadata -----------------------------------------------------
    json.key("metadata");
    json.open('{');
    json.key("component");
    json.open('{');
    json.key("type");
    json.str_value("application");
    json.key("bom-ref");
    json.str_value(&subject_ref);
    json.key("name");
    json.str_value(&identity.name);
    json.key("version");
    json.str_value(&identity.version);
    json.key("publisher");
    json.str_value(&identity.publisher);
    json.close('}');
    json.key("properties");
    json.open('[');
    let properties = [
        (
            "lumen:generator",
            "xtask sbom (std-only; Cargo.lock + product.toml; no cargo-cyclonedx)",
        ),
        (
            "lumen:cargo-lock-version",
            &format!("v{}", lock.format_version),
        ),
        (
            "lumen:license-coverage",
            "omitted — Cargo.lock carries no license fields; G3 cargo-deny owns license policy",
        ),
    ];
    for (name, value) in properties {
        json.open('{');
        json.key("name");
        json.str_value(name);
        json.key("value");
        json.str_value(value);
        json.close('}');
    }
    json.close(']');
    json.close('}');

    // --- components -----------------------------------------------------
    json.key("components");
    json.open('[');
    for (idx, package) in sorted.iter().enumerate() {
        let r = &refs[idx];
        json.open('{');
        json.key("type");
        json.str_value("library");
        json.key("bom-ref");
        json.str_value(&r.bom_ref);
        json.key("name");
        json.str_value(&package.name);
        json.key("version");
        json.str_value(&package.version);
        json.key("scope");
        json.str_value("required");
        if let Some(purl) = &r.purl {
            json.key("purl");
            json.str_value(purl);
        }
        if let Some(url) = &r.distribution_url {
            json.key("externalReferences");
            json.open('[');
            json.open('{');
            json.key("type");
            json.str_value("distribution");
            json.key("url");
            json.str_value(url);
            json.close('}');
            json.close(']');
        }
        if let Some(checksum) = &package.checksum {
            json.key("hashes");
            json.open('[');
            json.open('{');
            json.key("alg");
            json.str_value("SHA-256");
            json.key("content");
            json.str_value(checksum);
            json.close('}');
            json.close(']');
        }
        json.key("properties");
        json.open('[');
        json.open('{');
        json.key("name");
        json.str_value("lumen:cargo-lock-source");
        json.key("value");
        json.str_value(package.source.as_deref().unwrap_or("path"));
        json.close('}');
        json.close(']');
        json.close('}');
    }
    json.close(']');

    // --- dependencies ---------------------------------------------------
    json.key("dependencies");
    json.open('[');
    fn dep_entry(json: &mut Json, bom_ref: &str, targets: &[usize], refs: &[Refs]) {
        json.open('{');
        json.key("ref");
        json.str_value(bom_ref);
        json.key("dependsOn");
        let refs_out: Vec<String> = targets
            .iter()
            .map(|idx| refs[*idx].bom_ref.clone())
            .collect();
        json.str_array(&refs_out);
        json.close('}');
    }
    dep_entry(&mut json, &subject_ref, &subject_deps, &refs);
    for (idx, _package) in sorted.iter().enumerate() {
        dep_entry(&mut json, &refs[idx].bom_ref, &edges[idx], &refs);
    }
    json.close(']');

    json.close('}');
    json.out.push('\n');
    Ok(json.out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    trait Must {
        type Value;
        #[track_caller]
        fn must(self) -> Self::Value;
    }
    impl<T, E: std::fmt::Display> Must for Result<T, E> {
        type Value = T;
        #[track_caller]
        fn must(self) -> T {
            match self {
                Ok(v) => v,
                Err(e) => unreachable!("test must(): {e}"),
            }
        }
    }
    impl<T> Must for Option<T> {
        type Value = T;
        #[track_caller]
        fn must(self) -> T {
            match self {
                Some(v) => v,
                None => unreachable!("test must(): None"),
            }
        }
    }

    const MINI_LOCK: &str = "\
# This file is automatically @generated by Cargo.
version = 4

[[package]]
name = \"alpha\"
version = \"1.0.0\"
source = \"registry+https://github.com/rust-lang/crates.io-index\"
checksum = \"0000000000000000000000000000000000000000000000000000000000000000\"
dependencies = [
 \"beta 2.0.0\",
 \"lumen-engine\",
]

[[package]]
name = \"beta\"
version = \"2.0.0\"
source = \"registry+https://github.com/rust-lang/crates.io-index\"
checksum = \"1111111111111111111111111111111111111111111111111111111111111111\"

[[package]]
name = \"beta\"
version = \"1.5.0\"
source = \"registry+https://github.com/rust-lang/crates.io-index\"
checksum = \"2222222222222222222222222222222222222222222222222222222222222222\"

[[package]]
name = \"lumen-engine\"
version = \"0.1.0\"
dependencies = [
 \"beta 1.5.0\",
]
";

    const MINI_PRODUCT: &str = "\
schema_version = 1

[identity]
name = \"Lumen\"
short_name = \"Lumen\"
publisher = \"Lumen Contributors\"
display_version = \"0.1.0-dev\"
version_code = 1
";

    #[test]
    fn parses_lock_packages_and_classifies_sources() {
        let lock = parse_lock(MINI_LOCK).must();
        assert_eq!(lock.format_version, 4);
        assert_eq!(lock.packages.len(), 4);
        let engine = lock
            .packages
            .iter()
            .find(|p| p.name == "lumen-engine")
            .must();
        assert_eq!(engine.source, None);
        assert_eq!(engine.checksum, None);
        let alpha = lock.packages.iter().find(|p| p.name == "alpha").must();
        assert_eq!(
            alpha.source.as_deref(),
            Some("registry+https://github.com/rust-lang/crates.io-index")
        );
        assert_eq!(alpha.dependencies, vec!["beta 2.0.0", "lumen-engine"]);
        // Two versions of `beta` coexist.
        assert_eq!(lock.packages.iter().filter(|p| p.name == "beta").count(), 2);
    }

    #[test]
    fn rejects_unknown_lockfile_keys_loudly() {
        assert!(
            parse_lock("version = 4\n[[package]]\nname = \"x\"\nversion = \"1\"\nweird = true\n")
                .is_err()
        );
        assert!(parse_lock("version = 4\n").is_err(), "no packages");
        assert!(
            parse_lock("[[package]]\nname = \"x\"\n").is_err(),
            "no version root"
        );
    }

    #[test]
    fn reads_product_identity() {
        let identity = read_product_identity(MINI_PRODUCT).must();
        assert_eq!(identity.name, "Lumen");
        assert_eq!(identity.version, "0.1.0-dev");
        assert_eq!(identity.publisher, "Lumen Contributors");
        assert!(read_product_identity("[identity]\nname = \"x\"\n").is_err());
    }

    #[test]
    fn dependency_resolution_handles_version_suffixes() {
        let lock = parse_lock(MINI_LOCK).must();
        let beta2 = resolve_dep("beta 2.0.0", &lock.packages).must();
        assert_eq!(lock.packages[beta2].version, "2.0.0");
        let beta15 = resolve_dep("beta 1.5.0", &lock.packages).must();
        assert_eq!(lock.packages[beta15].version, "1.5.0");
        // Ambiguous bare name when two versions exist: loud error.
        assert!(resolve_dep("beta", &lock.packages).is_err());
        assert!(resolve_dep("ghost", &lock.packages).is_err());
    }

    #[test]
    fn sbom_is_deterministic_and_reorder_stable() {
        let identity = read_product_identity(MINI_PRODUCT).must();
        let lock = parse_lock(MINI_LOCK).must();
        let first = build_sbom(&identity, &lock).must();
        let second = build_sbom(&identity, &lock).must();
        assert_eq!(first, second, "same inputs must give identical bytes");

        // Reordering the [[package]] rows must not change the artifact.
        let reordered = {
            let mut rows: Vec<&str> = MINI_LOCK.split("\n\n").collect();
            // rows: [header, alpha, beta 2.0, beta 1.5, engine] — swap two.
            rows.swap(1, 4);
            rows.join("\n\n")
        };
        let lock2 = parse_lock(&reordered).must();
        let third = build_sbom(&identity, &lock2).must();
        assert_eq!(
            first, third,
            "lockfile row order must not leak into the SBOM"
        );
    }

    #[test]
    fn sbom_carries_expected_cyclonedx_fields() {
        let identity = read_product_identity(MINI_PRODUCT).must();
        let lock = parse_lock(MINI_LOCK).must();
        let sbom = build_sbom(&identity, &lock).must();
        for needle in [
            "\"bomFormat\": \"CycloneDX\"",
            "\"specVersion\": \"1.5\"",
            "\"serialNumber\": \"urn:uuid:",
            "\"type\": \"application\"",
            "\"name\": \"Lumen\"",
            "\"version\": \"0.1.0-dev\"",
            "\"publisher\": \"Lumen Contributors\"",
            "\"pkg:cargo/alpha@1.0.0\"",
            "\"pkg:cargo/beta@2.0.0\"",
            "\"lumen-engine@0.1.0\"",
            "\"alg\": \"SHA-256\"",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "\"type\": \"distribution\"",
            "https://crates.io/api/v1/crates/beta/2.0.0",
            "\"lumen:cargo-lock-source\"",
            "\"lumen:license-coverage\"",
            "\"dependsOn\"",
        ] {
            assert!(sbom.contains(needle), "SBOM missing {needle}");
        }
        // Path packages must NOT claim a purl.
        let engine_pos = sbom.find("\"lumen-engine\"").must();
        let next_component = sbom[engine_pos..]
            .find("\"properties\"")
            .map(|off| &sbom[engine_pos..engine_pos + off])
            .must();
        assert!(!next_component.contains("\"purl\""));
        // The subject depends on the path (workspace) package.
        let subject_pos = sbom.find("\"ref\": \"Lumen@0.1.0-dev\"").must();
        let dep_block = &sbom[subject_pos..subject_pos + 200];
        assert!(dep_block.contains("lumen-engine@0.1.0"));
    }

    #[test]
    fn serial_uuid_is_rfc9562_shaped_and_content_sensitive() {
        let a = serial_uuid("content-a");
        let b = serial_uuid("content-b");
        assert_ne!(a, b);
        assert!(a.starts_with("urn:uuid:"));
        let uuid = &a["urn:uuid:".len()..];
        let groups: Vec<&str> = uuid.split('-').collect();
        assert_eq!(
            groups.iter().map(|g| g.len()).collect::<Vec<_>>(),
            vec![8, 4, 4, 4, 12]
        );
        assert!(
            groups[2].starts_with('8'),
            "version nibble must be 8 (custom), got {groups:?}"
        );
        assert!(
            matches!(groups[3].as_bytes()[0], b'8'..=b'b'),
            "variant bits must be 10xx"
        );
        assert!(groups.iter().all(|g| {
            g.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        }));
        // Deterministic.
        assert_eq!(a, serial_uuid("content-a"));
    }

    #[test]
    fn json_escape_covers_rfc8259() {
        assert_eq!(json_escape("plain"), "plain");
        assert_eq!(json_escape("a\"b\\c"), "a\\\"b\\\\c");
        assert_eq!(json_escape("nl\ntab\tcr\r"), "nl\\ntab\\tcr\\r");
        assert_eq!(json_escape("\u{0001}\u{001F}"), "\\u0001\\u001f");
        assert_eq!(json_escape("héllo — ünïcode"), "héllo — ünïcode");
    }

    #[test]
    fn json_writer_pretty_prints_and_handles_empty_containers() {
        let mut json = Json::new();
        json.open('{');
        json.key("a");
        json.str_value("x");
        json.key("empty");
        json.str_array(&[]);
        json.key("list");
        json.str_array(&["one".to_string(), "two".to_string()]);
        json.close('}');
        assert_eq!(
            json.out,
            "{\n  \"a\": \"x\",\n  \"empty\": [],\n  \"list\": [\n    \"one\",\n    \"two\"\n  ]\n}"
        );
    }

    #[test]
    fn non_crates_io_source_stays_opaque() {
        let lock = parse_lock(
            "version = 4\n\n[[package]]\nname = \"gitdep\"\nversion = \"0.9.0\"\nsource = \"git+https://example.com/dep?rev=abc#abc\"\n",
        )
        .must();
        let identity = read_product_identity(MINI_PRODUCT).must();
        let sbom = build_sbom(&identity, &lock).must();
        assert!(sbom.contains("\"gitdep@0.9.0\""));
        assert!(!sbom.contains("pkg:cargo/gitdep"));
        assert!(sbom.contains("git+https://example.com/dep?rev=abc#abc"));
    }
}
