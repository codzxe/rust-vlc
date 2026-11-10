//! The on-disk extension store: install, remove, list, load (doc 06
//! §5.7, F3).
//!
//! # Layout `[PROPOSAL — doc 06 §5.7]`
//!
//! ```text
//! <root>/extensions/<id>/<version>/manifest.toml
//!                               /plugin.wasm
//!                               /signature      (optional)
//! ```
//!
//! Per-user only (the root is the caller's config dir); never a
//! system directory, never on `PATH`, no executable bit required.
//! Multiple versions coexist; [`ExtensionStore::load`] picks the
//! highest.
//!
//! # Consent is a value, not a flag (doc 06 §5.7 step 1, MP §6)
//!
//! Install and remove take an [`InstallConsent`] that records *what
//! the user saw and confirmed*: publisher, permission list, signature
//! status. The store refuses an install whose package does not match
//! the consent exactly — a UI cannot present one extension and slip
//! in another, and no code path exists that installs without a
//! consent value naming the package. Merely *reading* or *opening* a
//! `.lumenext` (this type's [`ExtensionStore::inspect`]) installs and
//! executes nothing — tested.
//!
//! # No execution at install time
//!
//! Install validates structure, manifest, provenance and signature
//! bookkeeping, then copies files. It never compiles or instantiates
//! the module: first execution happens only when something spawns the
//! extension through [`crate::instance::WasmInstance::spawn`] (doc 06
//! §5.7 step 5, tested with a trapping start section).

use std::path::{Path, PathBuf};

use plugin_api::PluginManifest;
use plugin_api::manifest::Origin;
use plugin_api::semver::Semver;

use crate::api::{self, Grants};
use crate::error::{HostError, PackageError};
use crate::package::{
    ENTRY_MANIFEST, ENTRY_MODULE, ENTRY_SIGNATURE, LumenextPackage, SignatureStatus, read_package,
};

/// User consent for an install or remove (doc 06 §5.7 step 1).
///
/// Constructed only through [`InstallConsent::confirm`] — the record
/// of what the confirmation dialog showed. The store verifies the
/// package matches before installing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallConsent {
    /// Publisher the user was shown (manifest `[provenance]`).
    pub(crate) publisher: String,
    /// Permission names the user was shown (plain language list).
    pub(crate) permissions: Vec<&'static str>,
    /// Signature status the user was shown.
    pub(crate) signature: SignatureStatus,
}

impl InstallConsent {
    /// The user confirmed installing `publisher`'s extension with
    /// exactly `permissions` and the stated signature posture.
    pub fn confirm(
        publisher: impl Into<String>,
        permissions: &[&'static str],
        signature: SignatureStatus,
    ) -> InstallConsent {
        InstallConsent {
            publisher: publisher.into(),
            permissions: permissions.to_vec(),
            signature,
        }
    }

    /// Whether this consent covers the given package facts.
    fn covers(&self, publisher: &str, grants: &Grants, signature: &SignatureStatus) -> bool {
        self.publisher == publisher
            && self.permissions == grants.granted_names()
            && self.signature == *signature
    }
}

/// What an install produced (the `plugins.loaded` seed, doc 06 §5.7
/// step 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReceipt {
    /// Installed extension id.
    pub id: String,
    /// Installed version.
    pub version: String,
    /// Where the extension now lives.
    pub path: PathBuf,
    /// Signature posture recorded for the install (honest: v1 never
    /// claims `verified`).
    pub signature: SignatureStatus,
    /// Permissions granted (the manifest's declared set — the only
    /// thing the import table will serve).
    pub permissions: Vec<&'static str>,
}

/// One installed version directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledVersion {
    /// Version string (directory name).
    pub version: String,
    /// The directory.
    pub path: PathBuf,
}

/// One installed extension (all its versions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledExtension {
    /// Extension id.
    pub id: String,
    /// Every version currently on disk (unsorted).
    pub versions: Vec<InstalledVersion>,
}

/// A loaded extension: verified bytes ready to spawn.
#[derive(Debug, Clone)]
pub struct LoadedExtension {
    /// The parsed, re-validated manifest.
    pub manifest: PluginManifest,
    /// The module bytes (digest re-verified against the manifest).
    pub wasm: Vec<u8>,
    /// Signature status (parsed at install, re-parsed at load).
    pub signature: SignatureStatus,
    /// The version directory this came from.
    pub path: PathBuf,
}

/// The per-user extension store (doc 06 §5.7).
#[derive(Debug, Clone)]
pub struct ExtensionStore {
    root: PathBuf,
}

impl ExtensionStore {
    /// Open (and create if needed) the store under `root`; the
    /// extensions live in `<root>/extensions/`.
    ///
    /// # Errors
    /// [`HostError::Io`] when the directory cannot be created.
    pub fn open(root: impl AsRef<Path>) -> Result<ExtensionStore, HostError> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root).map_err(|e| {
            HostError::Io(format!(
                "extension store root {} cannot be created: {e}",
                root.display()
            ))
        })?;
        Ok(ExtensionStore { root })
    }

    /// The store root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// **Inspect only** — parse a package and report what an install
    /// would need consent for (publisher, permissions, signature
    /// posture). Installs nothing, executes nothing (doc 06 §5.7:
    /// double-opening a `.lumenext` never installs; MP §6: no
    /// auto-exec of downloaded packages — tested).
    ///
    /// # Errors
    /// The same typed package/manifest errors [`Self::install`]
    /// would produce — surfaced to the UI before any confirmation.
    pub fn inspect(&self, pkg: &[u8]) -> Result<InstallPreview, HostError> {
        let parsed = validate_package(pkg)?;
        Ok(InstallPreview {
            manifest: parsed.manifest,
            signature: parsed.signature,
            module_bytes: parsed.package.module_wasm.len() as u64,
        })
    }

    /// Install a `.lumenext` package (doc 06 §5.7 steps 2–4). Never
    /// executes the module.
    ///
    /// # Errors
    /// [`HostError::ConsentRequired`] without a matching consent;
    /// typed [`PackageError`]s for every structural/manifest/
    /// provenance refusal — see [`validate_package`].
    pub fn install(
        &self,
        pkg: &[u8],
        consent: &InstallConsent,
    ) -> Result<InstallReceipt, HostError> {
        let parsed = validate_package(pkg)?;
        let ValidatedPackage {
            manifest,
            package,
            signature,
        } = parsed;
        let grants = Grants::from_permissions(&manifest.permissions);
        // Consent must match the package exactly (doc 06 §5.7 step 3:
        // publisher + permissions surfaced in plain language before
        // the final confirm; this is the grant moment).
        let publisher = manifest
            .provenance
            .as_ref()
            .map(|p| p.publisher.clone())
            .unwrap_or_default();
        if !consent.covers(&publisher, &grants, &signature) {
            return Err(HostError::ConsentRequired);
        }
        // Layout: <root>/extensions/<id>/<version>/…
        let dir = self.version_dir(manifest.id.as_str(), &manifest.version.to_string())?;
        if dir.exists() {
            // Same version already installed: idempotent only when the
            // bytes are identical; a silent overwrite would break the
            // provenance story (the recorded digest belongs to what
            // the user consented to).
            let existing = std::fs::read(dir.join(ENTRY_MODULE)).map_err(HostError::from)?;
            if existing != package.module_wasm {
                return Err(HostError::Layout(format!(
                    "version {} of `{}` is already installed with \
                     different bytes — remove it first (doc 06 §5.7)",
                    manifest.version, manifest.id
                )));
            }
        } else {
            std::fs::create_dir_all(&dir)
                .map_err(|e| HostError::Io(format!("cannot create {}: {e}", dir.display())))?;
        }
        std::fs::write(dir.join(ENTRY_MANIFEST), &package.manifest_toml)
            .map_err(|e| HostError::Io(format!("write manifest: {e}")))?;
        std::fs::write(dir.join(ENTRY_MODULE), &package.module_wasm)
            .map_err(|e| HostError::Io(format!("write module: {e}")))?;
        if let Some(sig) = package.signature.as_ref() {
            let text = format!(
                "lumenext-signature-v1\nsuite: {}\nkey: {}\nvalue: {}\n",
                sig.suite, sig.key, sig.value
            );
            std::fs::write(dir.join(ENTRY_SIGNATURE), text)
                .map_err(|e| HostError::Io(format!("write signature: {e}")))?;
        }
        Ok(InstallReceipt {
            id: manifest.id.as_str().to_string(),
            version: manifest.version.to_string(),
            path: dir,
            signature,
            permissions: grants.granted_names(),
        })
    }

    /// Remove an installed extension (doc 06 §5.7 remove): the whole
    /// id (`version = None`) or one version. Active instances are a
    /// host-scheduler concern (issue #46); this store owns only the
    /// disk state.
    ///
    /// # Errors
    /// [`HostError::ConsentRequired`] without consent;
    /// [`HostError::NotInstalled`] when nothing matches.
    pub fn remove(
        &self,
        id: &str,
        version: Option<&str>,
        _consent: &InstallConsent,
    ) -> Result<Removed, HostError> {
        // _consent: the parameter is mandatory so no call site can be
        // written without deciding the consent story; the current
        // body only needs it to exist (doc 06 §5.7: user-initiated).
        let target = match version {
            Some(v) => self.version_dir(id, v)?,
            None => self.extension_dir(id)?,
        };
        if !target.exists() {
            return Err(HostError::NotInstalled { id: id.to_string() });
        }
        std::fs::remove_dir_all(&target)
            .map_err(|e| HostError::Io(format!("cannot remove {}: {e}", target.display())))?;
        Ok(Removed { path: target })
    }

    /// Every installed extension with every version (registry seed).
    ///
    /// # Errors
    /// [`HostError::Io`] on unreadable directories.
    pub fn installed(&self) -> Result<Vec<InstalledExtension>, HostError> {
        let mut out = Vec::new();
        let extensions = self.root.join("extensions");
        let ids = match std::fs::read_dir(&extensions) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => {
                return Err(HostError::Io(format!(
                    "cannot read {}: {e}",
                    extensions.display()
                )));
            }
        };
        for id_entry in ids {
            let id_entry = id_entry.map_err(HostError::from)?;
            if !id_entry.file_type().map_err(HostError::from)?.is_dir() {
                continue;
            }
            let id = id_entry.file_name().to_string_lossy().to_string();
            let mut versions = Vec::new();
            for version_entry in std::fs::read_dir(id_entry.path()).map_err(HostError::from)? {
                let version_entry = version_entry.map_err(HostError::from)?;
                if !version_entry.file_type().map_err(HostError::from)?.is_dir() {
                    continue;
                }
                versions.push(InstalledVersion {
                    version: version_entry.file_name().to_string_lossy().to_string(),
                    path: version_entry.path(),
                });
            }
            out.push(InstalledExtension { id, versions });
        }
        Ok(out)
    }

    /// Load an extension for spawning: the highest installed version,
    /// re-validated end to end (manifest, origin, kind, api window,
    /// **digest** — a tampered on-disk copy refuses to load, doc 06
    /// §5.7 step 3 enforced at every load, not just install).
    ///
    /// # Errors
    /// [`HostError::NotInstalled`]; every typed validation error of
    /// [`validate_package`]; [`PackageError::InstalledHashMismatch`]
    /// when the on-disk module diverges from the recorded digest.
    pub fn load(&self, id: &str) -> Result<LoadedExtension, HostError> {
        let extension = self
            .installed()?
            .into_iter()
            .find(|e| e.id == id)
            .ok_or_else(|| HostError::NotInstalled { id: id.to_string() })?;
        // Highest version wins (doc 06 §5.7). Precedence via
        // plugin-api's semver; unparsable directory names are
        // skipped as layout noise (a partially written install).
        let mut best: Option<(Semver, InstalledVersion)> = None;
        for version in extension.versions {
            let text = version.version.clone();
            let Ok(parsed) = Semver::parse(&text) else {
                continue;
            };
            let better = match &best {
                None => true,
                Some((current, _)) => parsed.precedence(current) == std::cmp::Ordering::Greater,
            };
            if better {
                best = Some((parsed, version));
            }
        }
        let (_, highest) = best.ok_or_else(|| {
            HostError::Layout(format!(
                "extension `{id}` has no parsable version directory"
            ))
        })?;
        let manifest_bytes = std::fs::read(highest.path.join(ENTRY_MANIFEST)).map_err(|e| {
            HostError::Layout(format!(
                "manifest at {}: {e}",
                highest.path.join(ENTRY_MANIFEST).display()
            ))
        })?;
        let module_bytes = std::fs::read(highest.path.join(ENTRY_MODULE)).map_err(|e| {
            HostError::Layout(format!(
                "module at {}: {e}",
                highest.path.join(ENTRY_MODULE).display()
            ))
        })?;
        let signature_block = std::fs::read_to_string(highest.path.join(ENTRY_SIGNATURE))
            .ok()
            .map(|t| crate::package::SignatureBlock::parse(&t))
            .transpose()
            .map_err(HostError::from)?;
        let signature = signature_block
            .as_ref()
            .map(|block| block.status())
            .unwrap_or(SignatureStatus::Unsigned);
        // Re-validate exactly like install (structural rules come
        // free: the files were extracted from a package once; the
        // manifest, origin, kind, window and hash are re-checked).
        let manifest = parse_extension_manifest(&String::from_utf8_lossy(&manifest_bytes))?;
        api::negotiate_api_version(manifest.api_version)?;
        let provenance = manifest
            .provenance
            .as_ref()
            .ok_or_else(|| HostError::Layout("extension manifest without provenance".into()))?;
        let computed = crate::package::module_digest(&module_bytes);
        if provenance.hash.as_str() != computed {
            return Err(HostError::Package(Box::new(
                PackageError::InstalledHashMismatch {
                    path: highest.path.join(ENTRY_MODULE),
                },
            )));
        }
        if let Some(declared) = provenance.signature.as_ref() {
            let recorded = signature_block.as_ref().map(|b| b.suite.as_str());
            if recorded != Some(declared.scheme.as_str()) {
                return Err(HostError::Package(Box::new(
                    PackageError::SignatureUnparsable {
                        detail: format!(
                            "manifest declares signature scheme `{}` but the \
                         stored signature block records `{}`",
                            declared.scheme,
                            recorded.unwrap_or("<absent>")
                        ),
                    },
                )));
            }
        }
        Ok(LoadedExtension {
            manifest,
            wasm: module_bytes,
            signature,
            path: highest.path,
        })
    }

    fn extension_dir(&self, id: &str) -> Result<PathBuf, HostError> {
        safe_component(id)?;
        Ok(self.root.join("extensions").join(id))
    }

    fn version_dir(&self, id: &str, version: &str) -> Result<PathBuf, HostError> {
        safe_component(id)?;
        safe_component(version)?;
        Ok(self.root.join("extensions").join(id).join(version))
    }
}

/// What an [`ExtensionStore::inspect`] preview showed (the consent
/// dialog's data, doc 06 §5.7 step 3).
#[derive(Debug, Clone, PartialEq)]
pub struct InstallPreview {
    /// The package's manifest (validated).
    pub manifest: PluginManifest,
    /// Signature posture.
    pub signature: SignatureStatus,
    /// Module size in bytes (for the size warning).
    pub module_bytes: u64,
}

/// A completed removal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removed {
    /// The directory that was deleted.
    pub path: PathBuf,
}

/// A package that passed every validation step.
struct ValidatedPackage {
    manifest: PluginManifest,
    package: LumenextPackage,
    signature: SignatureStatus,
}

/// The full validation chain (doc 06 §5.7 steps 2–3), shared by
/// install and inspect:
///
/// 1. structural: ZIP shape, caps, stored method, CRCs, wasm magic
///    (the native-code fence — ELF/Mach-O/PE payloads die here);
/// 2. manifest: `plugin_api` schema validation + extension shape
///    (`origin = "extension"` — a disk manifest claiming `builtin`
///    is an origin forgery, doc 06 §2.2/§9.4);
/// 3. kind: v1 extension-legal kinds only (doc 06 §5.9);
/// 4. api window: negotiated now, typed mismatch (doc 06 §5.5);
/// 5. provenance: full `sha256:<64hex>` digest that must equal the
///    module's actual digest (doc 06 §2.2, §5.7 step 3);
/// 6. signature bookkeeping: the manifest's declared signature
///    scheme must agree with the package's detached block; the
///    status is recorded honestly (never `verified` in v1 — the
///    verifier is a Lead-approved crypto ADR away, doc 06 §10 Q5).
fn validate_package(pkg: &[u8]) -> Result<ValidatedPackage, HostError> {
    let package = read_package(pkg)?;
    let manifest = parse_extension_manifest(&String::from_utf8_lossy(&package.manifest_toml))?;
    // (2) origin: disk/package manifests are extensions, always.
    if manifest.origin != Origin::Extension {
        return Err(HostError::Package(Box::new(PackageError::OriginForgery)));
    }
    // (3) kind: v1 fence.
    if !api::kind_extension_legal(manifest.kind) {
        return Err(HostError::KindRefused {
            kind: manifest.kind.to_string(),
        });
    }
    // (4) api window.
    api::negotiate_api_version(manifest.api_version)?;
    // (5) provenance hash: full form, exact match.
    let provenance = manifest.provenance.as_ref().ok_or_else(|| {
        HostError::Package(Box::new(PackageError::NotAnExtension {
            detail: "missing [provenance] (plugin-api requires it; the \
                     package manifest validator agrees)"
                .to_string(),
        }))
    })?;
    if !provenance.hash.is_full() {
        return Err(HostError::Package(Box::new(PackageError::HashMismatch {
            declared: provenance.hash.as_str().to_string(),
            computed: "a full sha256:<64 hex> digest is required at \
                       install (doc placeholders are documentation, not \
                       packages)"
                .to_string(),
        })));
    }
    let computed = crate::package::module_digest(&package.module_wasm);
    if provenance.hash.as_str() != computed {
        return Err(HostError::Package(Box::new(PackageError::HashMismatch {
            declared: provenance.hash.as_str().to_string(),
            computed,
        })));
    }
    // (6) signature bookkeeping (honest status, cross-checked).
    let signature = match (&package.signature, &provenance.signature) {
        (None, None) => SignatureStatus::Unsigned,
        (None, Some(declared)) => {
            return Err(HostError::Package(Box::new(
                PackageError::SignatureUnparsable {
                    detail: format!(
                        "manifest declares signature scheme `{}` but the \
                         package carries no signature block",
                        declared.scheme
                    ),
                },
            )));
        }
        (Some(_), None) => {
            return Err(HostError::Package(Box::new(
                PackageError::SignatureUnparsable {
                    detail: "package carries a signature block but the \
                             manifest declares none"
                        .to_string(),
                },
            )));
        }
        (Some(block), Some(declared)) => {
            if block.suite != declared.scheme {
                return Err(HostError::Package(Box::new(
                    PackageError::SignatureUnparsable {
                        detail: format!(
                            "manifest declares signature scheme `{}` but \
                             the block says `{}`",
                            declared.scheme, block.suite
                        ),
                    },
                )));
            }
            if block.key != declared.key.as_str() {
                return Err(HostError::Package(Box::new(
                    PackageError::SignatureUnparsable {
                        detail: "signature block key differs from the \
                                 manifest's declared key"
                            .to_string(),
                    },
                )));
            }
            block.status()
        }
    };
    Ok(ValidatedPackage {
        manifest,
        package,
        signature,
    })
}

/// Parse the package manifest through the plugin-api validator and
/// require the extension shape.
fn parse_extension_manifest(text: &str) -> Result<PluginManifest, HostError> {
    let manifest =
        PluginManifest::from_toml(text).map_err(|e| HostError::Package(Box::new(e.into())))?;
    if manifest.origin != Origin::Extension {
        return Err(HostError::Package(Box::new(PackageError::OriginForgery)));
    }
    Ok(manifest)
}

impl From<plugin_api::manifest::ManifestError> for PackageError {
    fn from(err: plugin_api::manifest::ManifestError) -> PackageError {
        PackageError::Manifest(err.to_string())
    }
}

/// A path component that is safe to join: non-empty, no separators,
/// no `.`/`..`, no NUL (defense in depth — the id grammar already
/// forbids these, doc 06 §2.2).
fn safe_component(component: &str) -> Result<(), HostError> {
    const MSG: &str = "unsafe path component";
    if component.is_empty()
        || component == "."
        || component == ".."
        || component.bytes().any(|b| b == b'/' || b == b'\\' || b == 0)
    {
        return Err(HostError::Layout(format!("{MSG}: `{component}`")));
    }
    Ok(())
}

/// The permissions a manifest asks for (install UI helper; the
/// [`Grants::from_permissions`] derivation is the enforced one).
#[must_use]
pub fn declared_permissions(manifest: &PluginManifest) -> plugin_api::Permissions {
    manifest.permissions.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_component_rejects_traversal() {
        assert!(safe_component("ext.acme.x").is_ok());
        assert!(safe_component("0.1.0").is_ok());
        for bad in ["", ".", "..", "a/b", "a\\b", "a\0b"] {
            assert!(safe_component(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn consent_must_match_exactly() {
        let grants = Grants::default();
        let sig = SignatureStatus::Unsigned;
        let consent = InstallConsent::confirm("Acme", &[], sig.clone());
        assert!(consent.covers("Acme", &grants, &sig));
        // Wrong publisher / permissions / posture all refuse.
        assert!(!consent.covers("Evil", &grants, &sig));
        let perms = crate::api::Grants {
            fs: true,
            ..Default::default()
        };
        assert!(!consent.covers("Acme", &perms, &sig));
        assert!(!consent.covers(
            "Acme",
            &grants,
            &SignatureStatus::Unverified {
                suite: "ed25519-detached-v1".into()
            }
        ));
    }
}
