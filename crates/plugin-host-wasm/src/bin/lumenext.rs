//! `lumenext` — the `.lumenext` packaging and inspection CLI (card
//! 7.G1, issue #46): the developer-side tool the example-extension
//! walkthrough uses. It WRITES packages (stamping the real module
//! digest into the manifest's provenance) and VALIDATES them (the
//! same structural chain the installer runs); it never installs and
//! never executes guest code (install is the app's user-consented
//! flow, doc 06 §5.7).
//!
//! Exit codes: 0 success, 1 usage/validation failure, 2 I/O failure.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use plugin_api::PluginManifest;
use plugin_host_wasm::crc32::crc32;
use plugin_host_wasm::package::{
    ENTRY_MANIFEST, ENTRY_MODULE, module_digest, read_package, stamp_manifest_hash,
};

/// The action this invocation failed on (context for messages).
enum Failure {
    /// Bad command line.
    Usage(String),
    /// Validation refused the input.
    Validation(String),
    /// Filesystem failure.
    Io(String),
}

impl Failure {
    /// The process exit code for this failure class.
    fn code(&self) -> u8 {
        match self {
            Failure::Usage(_) | Failure::Validation(_) => 1,
            Failure::Io(_) => 2,
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Usage(msg) => write!(f, "usage: {msg}"),
            Failure::Validation(msg) => write!(f, "refused: {msg}"),
            Failure::Io(msg) => write!(f, "io: {msg}"),
        }
    }
}

const USAGE: &str = "lumenext — package and inspect .lumenext extension archives

USAGE:
    lumenext package <ext-dir> [--wasm <module>] [--out <file>]
        Reads <ext-dir>/manifest.toml and the module (the --wasm path,
        defaulting to <ext-dir>/plugin.wasm), stamps the manifest's
        [provenance] hash with the module's real sha256 (a doc
        placeholder is required; a diverging full hash is refused),
        and writes the stored-entry ZIP.

    lumenext inspect <pkg>
        Validates an archive end to end (structure, CRCs, wasm magic,
        manifest schema, digest equality) and prints its identity.
        Never installs, never executes.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Usage(msg)) => {
            eprintln!("lumenext: {msg}\n\n{USAGE}");
            ExitCode::from(1)
        }
        Err(other) => {
            eprintln!("lumenext: {other}");
            ExitCode::from(other.code())
        }
    }
}

fn run(args: &[String]) -> Result<(), Failure> {
    match args.first().map(String::as_str) {
        Some("package") => package(&args[1..]),
        Some("inspect") => inspect(&args[1..]),
        Some(other) => Err(Failure::Usage(format!("unknown command `{other}`"))),
        None => Err(Failure::Usage("a subcommand is required".to_string())),
    }
}

// -- package -----------------------------------------------------------------

fn package(args: &[String]) -> Result<(), Failure> {
    let mut dir: Option<PathBuf> = None;
    let mut wasm: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--wasm" => {
                wasm = Some(
                    args.get(i + 1)
                        .ok_or_else(|| Failure::Usage("--wasm needs a path".to_string()))?
                        .into(),
                );
                i += 2;
            }
            "--out" => {
                out = Some(
                    args.get(i + 1)
                        .ok_or_else(|| Failure::Usage("--out needs a path".to_string()))?
                        .into(),
                );
                i += 2;
            }
            other if !other.starts_with("--") && dir.is_none() => {
                dir = Some(other.into());
                i += 1;
            }
            other => {
                return Err(Failure::Usage(format!("unexpected argument `{other}`")));
            }
        }
    }
    let dir = dir.ok_or_else(|| Failure::Usage("package needs <ext-dir>".to_string()))?;
    let manifest_path = dir.join("manifest.toml");
    let manifest_bytes = read(&manifest_path)?;
    let manifest_text = String::from_utf8(manifest_bytes)
        .map_err(|_| Failure::Validation("manifest.toml is not UTF-8".to_string()))?;
    let wasm_path = wasm.unwrap_or_else(|| dir.join(ENTRY_MODULE));
    let module = read(&wasm_path)?;

    // Stamp the provenance hash with the real digest (placeholder in,
    // full digest out — a diverging full hash is a tamper refusal).
    // The one library implementation (package.rs) is the same one the
    // installer's digest-equality check consumes.
    let digest = module_digest(&module);
    let stamped = stamp_manifest_hash(&manifest_text, &digest)
        .map_err(|e| Failure::Validation(e.to_string()))?;
    // The stamped manifest must parse as an extension manifest the
    // installer would accept (origin, provenance, full digest).
    let manifest = PluginManifest::from_toml(&stamped)
        .map_err(|e| Failure::Validation(format!("manifest.toml: {e}")))?;

    let out = out.unwrap_or_else(|| {
        dir.join(format!(
            "{}-{}.lumenext",
            manifest.id.as_str(),
            manifest.version
        ))
    });
    let bytes = zip_stored(&[
        (ENTRY_MANIFEST, stamped.as_bytes()),
        (ENTRY_MODULE, &module),
    ]);
    write(&out, &bytes)?;
    println!(
        "lumenext: packaged {} v{} (kind {}, api v{})",
        manifest.id.as_str(),
        manifest.version,
        manifest.kind.as_str(),
        manifest.api_version
    );
    println!(
        "  module : {} ({} bytes)",
        wasm_path.display(),
        module.len()
    );
    println!("  digest : {digest}");
    println!("  output : {}", out.display());
    Ok(())
}

// -- inspect -----------------------------------------------------------------

fn inspect(args: &[String]) -> Result<(), Failure> {
    let pkg = args
        .first()
        .ok_or_else(|| Failure::Usage("inspect needs <pkg>".to_string()))?;
    let bytes = read(Path::new(pkg))?;
    // The structural chain: ZIP shape, stored method, CRCs, caps, the
    // wasm-magic native-code fence (read_package, doc 06 §5.7 step 2).
    let package = read_package(&bytes).map_err(|e| Failure::Validation(format!("{e}")))?;
    let manifest_text = String::from_utf8(package.manifest_toml.clone())
        .map_err(|_| Failure::Validation("manifest.toml is not UTF-8".to_string()))?;
    let manifest = PluginManifest::from_toml(&manifest_text)
        .map_err(|e| Failure::Validation(format!("manifest.toml: {e}")))?;
    // Digest equality (doc 06 §5.7 step 3): the manifest's full
    // `sha256:<hex>` value must be exactly the module's digest (the
    // same string `lumenext package` stamps and install compares).
    let digest = module_digest(&package.module_wasm);
    if manifest
        .provenance
        .as_ref()
        .is_none_or(|p| p.hash.as_str() != digest)
    {
        return Err(Failure::Validation(format!(
            "provenance.hash does not match the module (module digests to {digest})"
        )));
    }
    let permissions = manifest.permissions.granted();
    let signature = match &package.signature {
        None => "absent (unsigned package)".to_string(),
        Some(block) => match block.status() {
            plugin_host_wasm::package::SignatureStatus::Unsigned => "unsigned".to_string(),
            plugin_host_wasm::package::SignatureStatus::Unverified { suite } => {
                format!("unverified ({suite}) — v1 has no verifier yet")
            }
            plugin_host_wasm::package::SignatureStatus::UnknownSuite { suite } => {
                format!("unknown suite `{suite}`")
            }
        },
    };
    println!(
        "lumenext: {} v{} VALID (kind {}, origin {}, api v{})",
        manifest.id.as_str(),
        manifest.version,
        manifest.kind.as_str(),
        manifest.origin.as_str(),
        manifest.api_version
    );
    println!(
        "  module    : {} bytes, sha256 matches",
        package.module_wasm.len()
    );
    println!("  signature : {signature}");
    if permissions.is_empty() {
        println!("  permissions: (none declared)");
    } else {
        println!("  permissions: {}", permissions.join(", "));
    }
    Ok(())
}

// -- std helpers (typed, no panics) -------------------------------------------

fn read(path: &Path) -> Result<Vec<u8>, Failure> {
    fs::read(path).map_err(|e| Failure::Io(format!("reading {}: {e}", path.display())))
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
    fs::write(path, bytes).map_err(|e| Failure::Io(format!("writing {}: {e}", path.display())))
}

/// A stored-entry ZIP writer (local headers + central directory +
/// EOCD), CRC-32 via the crate's own implementation — the writer
/// twin of the reader in `src/package.rs` (doc 06 §5.7: v1 accepts
/// stored entries only, so the packager writes exactly that shape).
fn zip_stored(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut offsets = Vec::new();
    for (name, data) in entries {
        offsets.push(out.len() as u32);
        let crc = crc32(data);
        out.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]);
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&0u16.to_le_bytes()); // method: stored
        out.extend_from_slice(&0u16.to_le_bytes()); // mod time
        out.extend_from_slice(&0u16.to_le_bytes()); // mod date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
    }
    let cd_offset = out.len() as u32;
    let mut central = Vec::new();
    for ((name, data), offset) in entries.iter().zip(&offsets) {
        let crc = crc32(data);
        central.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02]);
        central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        central.extend_from_slice(&0u16.to_le_bytes()); // flags
        central.extend_from_slice(&0u16.to_le_bytes()); // method: stored
        central.extend_from_slice(&0u16.to_le_bytes()); // time
        central.extend_from_slice(&0u16.to_le_bytes()); // date
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra
        central.extend_from_slice(&0u16.to_le_bytes()); // comment
        central.extend_from_slice(&0u16.to_le_bytes()); // disk start
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let cd_size = central.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06]);
    out.extend_from_slice(&0u16.to_le_bytes()); // this disk
    out.extend_from_slice(&0u16.to_le_bytes()); // cd start disk
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment len
    out
}
