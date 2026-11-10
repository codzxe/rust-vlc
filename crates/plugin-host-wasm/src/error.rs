//! Typed errors for the WASM extension host (doc 06 §5).
//!
//! Every failure mode the host can produce is a variant here — never a
//! panic, never a swallowed `Result` (AGENTS §3.5; MP §8 typed
//! stage-named errors). Guest-side failures surface as trap codes or
//! import status codes ([`crate::api::GuestStatus`]); host-side
//! failures surface as [`HostError`].

use std::fmt;
use std::path::PathBuf;

/// The one error type crossing the host's public API.
#[derive(Debug)]
pub enum HostError {
    /// `api_version` negotiation failed at load (doc 06 §5.5): the
    /// manifest's major is outside the engine's supported window.
    /// Typed and actionable — never UB, never a silent load.
    ApiVersion {
        /// The major the extension was built against.
        found: u32,
        /// The lowest major this engine still serves.
        supported_min: u32,
        /// The engine's current plugin API major.
        supported_max: u32,
    },
    /// The module imports a `lumen:v1.*` function whose gating
    /// permission the manifest did not declare (doc 06 §5.3): the
    /// import is absent from the instance's table, so instantiation
    /// fails before any guest code runs. Recorded as a security event.
    UndeclaredImport {
        /// Import module name (e.g. `lumen:v1.fs`).
        module: String,
        /// Import function name (e.g. `fs_open`).
        name: String,
        /// The permission that would have granted it.
        permission: &'static str,
    },
    /// The module imports from a namespace the host does not serve at
    /// all (no WASI, no ambient modules — doc 06 §5.2, ADR-002
    /// no-WASI posture).
    UnknownImportModule {
        /// The module name the guest asked for.
        module: String,
    },
    /// A runtime permission denial the guest observed as a typed
    /// status code (doc 06 §5.3: allowlist-violating fetch and kin).
    PermissionDenied {
        /// Human-readable detail (shapes, never raw buffers — §5.3).
        detail: String,
    },
    /// Linear-memory ceiling breached: a `memory.grow` past the
    /// per-kind cap traps (doc 06 §5.4 `MemoryOom`). The instance is
    /// faulted; the host is unaffected.
    MemoryLimit {
        /// The cap in bytes that was breached.
        cap_bytes: u64,
    },
    /// Fuel exhausted: the dispatched call (or instantiation) ran past
    /// its CPU budget (doc 06 §5.4, ADR-002 fuel metering). The
    /// instance is faulted; the host is unaffected.
    CpuLimit,
    /// The instance already faulted earlier (trap, limit breach or
    /// escalation disable); further dispatch is refused (doc 06 §5.6).
    InstanceFaulted {
        /// Why the instance was faulted.
        reason: String,
    },
    /// The exported entry point was not found or had the wrong shape.
    MissingExport {
        /// Export name that was requested.
        name: String,
    },
    /// Guest memory access failed the bounds check (doc 06 §9.4
    /// `plugin_sandbox_escape`): out-of-bounds offsets in import
    /// arguments produce this typed error — never UB, never a host
    /// crash. wasmi bounds-checks every read/write the host performs.
    GuestMemoryOob {
        /// Guest offset requested.
        ptr: u64,
        /// Length requested.
        len: u64,
        /// Current guest memory size in bytes.
        size: u64,
    },
    /// The extension kind is refused in v1 (doc 06 §5.9): access,
    /// network-protocol, demuxer, decoder, encoder, audio-output,
    /// video-output extensions are rejected at install with this
    /// error naming the policy.
    KindRefused {
        /// The refused kind's manifest string.
        kind: String,
    },
    /// wasmi failed to compile or instantiate the module for a reason
        /// not covered by a more specific variant.
    Instantiation(String),
    /// A dispatch returned an unexpected wasmi error (trap codes the
    /// host does not specially classify).
    Wasm(String),
    /// Host-side filesystem failure (install/remove/layout).
    Io(String),
    /// The on-disk extensions layout is inconsistent (missing
    /// manifest, stray files, corrupt record).
    Layout(String),
    /// The `.lumenext` package failed structural validation
    /// (doc 06 §5.7 step 2). Carries the specific reason.
    Package(Box<PackageError>),
    /// The extension is installed but disabled (by user or by the
    /// escalation policy, doc 06 §5.3); spawn is refused.
    ExtensionDisabled {
        /// Why the extension is disabled.
        reason: String,
    },
    /// The extension id is not installed on this host.
    NotInstalled {
        /// The requested plugin id.
        id: String,
    },
    /// A host-level resource ceiling was hit while spawning: too many
    /// active instances or the total extension-heap budget is
    /// exhausted (doc 06 §5.4 host accounting).
    HostCapacity {
        /// Human-readable accounting detail.
        detail: String,
    },
    /// Install/remove was attempted without explicit user consent
        /// (doc 06 §5.7 step 1, MP §6: install is user-initiated).
    ConsentRequired,
}

/// `.lumenext` package-format failures (doc 06 §5.7; F3).
#[derive(Debug)]
pub enum PackageError {
    /// The archive is not a ZIP (no end-of-central-directory record).
    NotZip,
    /// The archive exceeds the 64 MiB package cap.
    TooLarge {
        /// Observed size in bytes.
        size: u64,
    },
    /// An archive entry exceeds the 64 MiB per-entry cap, or an
    /// import argument exceeded its transfer cap.
    EntryTooLarge {
        /// Which entry / which cap.
        detail: String,
    },
    /// A required root entry is missing (`manifest.toml`, `plugin.wasm`).
    MissingEntry {
        /// The missing entry name.
        name: String,
    },
    /// The payload does not start with the wasm magic `\0asm` — the
    /// native-code fence for extensions (doc 06 §6: ELF/Mach-O/PE
    /// payloads are rejected here).
    BadMagic {
        /// The first four observed bytes, hex-encoded.
        observed: String,
    },
    /// The entry is compressed; v1 accepts stored (method 0) entries
    /// only `[PROPOSAL: deflate support with a future in-crate
    /// inflater, Lead decision]`.
    CompressedEntry {
        /// The ZIP method code.
        method: u16,
    },
    /// The entry's CRC-32 does not match its content.
    CrcMismatch {
        /// The entry name.
        name: String,
    },
    /// The central directory is malformed or carries an unsupported
    /// shape (too many entries, bad offsets, mismatched local header).
    Malformed {
        /// Specific reason.
        detail: String,
    },
    /// The manifest inside failed schema validation (delegates to
    /// `plugin_api`'s typed manifest errors, message included).
    Manifest(String),
    /// The manifest's `provenance.hash` does not match the package's
    /// `plugin.wasm` bytes (doc 06 §5.7 step 3 — refused, not warned).
    HashMismatch {
        /// The manifest-declared digest text.
        declared: String,
        /// The digest the host computed.
        computed: String,
    },
    /// A disk manifest claims `origin = "builtin"` — provenance
    /// forgery (doc 06 §2.2, §9.4 `plugin_malicious_manifest`).
    OriginForgery,
    /// The manifest is not an extension-shaped manifest at all.
    NotAnExtension {
        /// Specific reason.
        detail: String,
    },
    /// The detached signature block is present but unparseable.
    SignatureUnparsable {
        /// Specific reason.
        detail: String,
    },
    /// The `plugin.wasm` bytes do not hash to what is already
    /// installed under this id+version (tamper after install).
    InstalledHashMismatch {
        /// Where the diverging copy lives.
        path: PathBuf,
    },
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostError::ApiVersion {
                found,
                supported_min,
                supported_max,
            } => write!(
                f,
                "api_version {found} is outside the supported window \
                 [{supported_min}, {supported_max}] — rebuild the \
                 extension against a supported plugin API major \
                 (doc 06 §5.5)"
            ),
            HostError::UndeclaredImport {
                module,
                name,
                permission,
            } => write!(
                f,
                "module imports `{module}::{name}` without declaring \
                 the `{permission}` permission — import absent from the \
                 instance table (doc 06 §5.3)"
            ),
            HostError::UnknownImportModule { module } => write!(
                f,
                // lumen-allow-literal: plugin ABI namespace in an actionable error message — protocol vocabulary, not a brand surface
                "module imports from unknown module `{module}` — only \
                 `lumen:v1.*` imports exist (no WASI, doc 06 §5.2)"
            ),
            HostError::PermissionDenied { detail } => {
                write!(f, "permission denied: {detail}")
            }
            HostError::MemoryLimit { cap_bytes } => write!(
                f,
                "linear-memory cap of {cap_bytes} bytes breached — \
                 instance faulted (doc 06 §5.4 MemoryOom)"
            ),
            HostError::CpuLimit => f.write_str(
                "fuel exhausted — CPU budget for the dispatched call \
                 spent, instance faulted (doc 06 §5.4)",
            ),
            HostError::InstanceFaulted { reason } => {
                write!(f, "instance already faulted: {reason}")
            }
            HostError::MissingExport { name } => {
                write!(f, "module does not export `{name}`")
            }
            HostError::GuestMemoryOob { ptr, len, size } => write!(
                f,
                "guest buffer [{ptr}..{ptr}+{len}) is out of bounds \
                 (memory size {size} bytes) — typed denial, no UB \
                 (doc 06 §9.4)"
            ),
            HostError::KindRefused { kind } => write!(
                f,
                "extensions of kind `{kind}` are refused in v1 — \
                 byte-source, codec and output kinds stay builtin \
                 (doc 06 §5.9)"
            ),
            HostError::Instantiation(msg) => {
                write!(f, "instantiation failed: {msg}")
            }
            HostError::Wasm(msg) => write!(f, "wasm error: {msg}"),
            HostError::Io(msg) => write!(f, "host I/O error: {msg}"),
            HostError::Layout(msg) => write!(f, "extensions layout error: {msg}"),
            HostError::Package(err) => write!(f, "package rejected: {err}"),
            HostError::ExtensionDisabled { reason } => {
                write!(f, "extension disabled: {reason}")
            }
            HostError::NotInstalled { id } => write!(f, "extension `{id}` is not installed"),
            HostError::HostCapacity { detail } => {
                write!(f, "host capacity ceiling: {detail}")
            }
            HostError::ConsentRequired => f.write_str(
                "install/remove requires explicit user consent — \
                 no code path installs or executes an extension without \
                 it (doc 06 §5.7, MP §6)",
            ),
        }
    }
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageError::NotZip => f.write_str("not a ZIP archive"),
            PackageError::TooLarge { size } => write!(
                f,
                "package is {size} bytes; the cap is {} (doc 06 §5.7)",
                crate::package::PACKAGE_CAP_BYTES
            ),
            PackageError::EntryTooLarge { detail } => {
                write!(f, "entry exceeds its cap: {detail}")
            }
            PackageError::MissingEntry { name } => {
                write!(f, "required root entry `{name}` is missing")
            }
            PackageError::BadMagic { observed } => write!(
                f,
                "plugin.wasm does not start with the wasm magic \
                 `\\0asm` (observed {observed}) — native payloads are \
                 refused (doc 06 §6)"
            ),
            PackageError::CompressedEntry { method } => write!(
                f,
                "ZIP entry uses compression method {method}; v1 \
                 accepts stored (method 0) entries only"
            ),
            PackageError::CrcMismatch { name } => {
                write!(f, "CRC-32 mismatch for entry `{name}`")
            }
            PackageError::Malformed { detail } => {
                write!(f, "malformed ZIP structure: {detail}")
            }
            PackageError::Manifest(msg) => {
                write!(f, "manifest validation failed: {msg}")
            }
            PackageError::HashMismatch { declared, computed } => write!(
                f,
                "provenance hash mismatch: manifest declares {declared}, \
                 plugin.wasm hashes to {computed}"
            ),
            PackageError::OriginForgery => f.write_str(
                "disk manifest claims origin = \"builtin\" — provenance \
                 forgery attempt (doc 06 §2.2)",
            ),
            PackageError::NotAnExtension { detail } => {
                write!(f, "not an extension manifest: {detail}")
            }
            PackageError::SignatureUnparsable { detail } => {
                write!(f, "signature block unparseable: {detail}")
            }
            PackageError::InstalledHashMismatch { path } => write!(
                f,
                "installed copy at {} no longer matches its recorded \
                 digest — refusing to load a tampered extension",
                path.display()
            ),
        }
    }
}

impl std::error::Error for HostError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HostError::Package(err) => Some(err),
            _ => None,
        }
    }
}

impl std::error::Error for PackageError {}

impl From<std::io::Error> for HostError {
    fn from(err: std::io::Error) -> Self {
        HostError::Io(err.to_string())
    }
}

impl From<PackageError> for HostError {
    fn from(err: PackageError) -> Self {
        HostError::Package(Box::new(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_displays_actably() {
        // MP §8: error strings name the rule and the fix path.
        let cases: Vec<HostError> = vec![
            HostError::ApiVersion {
                found: 2,
                supported_min: 1,
                supported_max: 1,
            },
            HostError::UndeclaredImport {
                module: crate::api::MOD_FS.to_string(),
                name: "fs_open".into(),
                permission: "fs-sandboxed-read",
            },
            HostError::GuestMemoryOob {
                ptr: 8,
                len: u32::MAX as u64,
                size: 64 * 1024,
            },
            HostError::ConsentRequired,
        ];
        for err in cases {
            let text = err.to_string();
            assert!(text.len() > 20, "actionable message: {text}");
            assert!(!text.is_empty());
        }
    }
}
