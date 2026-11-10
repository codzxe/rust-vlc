//! The `.lumenext` package format (doc 06 §5.7, F3).
//!
//! A `.lumenext` is a ZIP archive with `manifest.toml` at the root,
//! `plugin.wasm` (wasm magic required — the native-code fence), an
//! optional detached `signature` block, and a 64 MiB size cap. v1
//! reads **stored** (method 0) entries and integrity-checks every
//! entry's CRC-32; compressed entries are refused with a typed
//! error `[PROPOSAL: deflate inflater if ever needed — Lead
//! decision]`.
//!
//! # Signature suite (doc 06 §10 Q5 — `[PROPOSAL]`)
//!
//! The detached signature file is a small versioned text block:
//!
//! ```text
//! lumenext-signature-v1
//! suite: ed25519-detached-v1
//! key: <publisher public key, suite-defined text form>
//! value: <detached signature, hex, over manifest.toml || plugin.wasm>
//! ```
//!
//! v1 parses and records the block but does **not** verify
//! cryptographic signatures: a real verifier needs a crypto
//! dependency (ed25519) that only a Lead-approved ADR may add. The
//! honest path ships now: signature present → trust recorded as
//! [`SignatureStatus::Unverified`]; absent →
//! [`SignatureStatus::Unsigned`]; *either way* install demands
//! explicit user consent, and the `provenance.hash` sha256 check
//! (which we CAN do, in-crate) is fully enforced.

use crate::crc32::crc32;
use crate::error::PackageError;
use crate::sha256::sha256_hex;

/// Package size cap (doc 06 §5.7: 64 MiB).
pub const PACKAGE_CAP_BYTES: u64 = 64 * 1024 * 1024;

/// Per-entry cap (same number; a single entry cannot exceed the
/// whole package).
pub const ENTRY_CAP_BYTES: u64 = PACKAGE_CAP_BYTES;

/// Entry-count cap (central-directory DoS guard).
pub const MAX_ENTRIES: usize = 1024;

/// Root entries of the format.
pub const ENTRY_MANIFEST: &str = "manifest.toml";
pub const ENTRY_MODULE: &str = "plugin.wasm";
pub const ENTRY_SIGNATURE: &str = "signature";

/// The parsed contents of a `.lumenext` archive.
#[derive(Debug, Clone)]
pub struct LumenextPackage {
    /// Raw `manifest.toml` bytes (validated by the caller through
    /// `plugin_api`).
    pub manifest_toml: Vec<u8>,
    /// Raw `plugin.wasm` bytes (wasm magic checked here).
    pub module_wasm: Vec<u8>,
    /// The detached signature block, when present.
    pub signature: Option<SignatureBlock>,
}

/// The versioned detached-signature block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureBlock {
    /// Magic line (`lumenext-signature-v1`).
    pub version_line: String,
    /// Suite id (e.g. `ed25519-detached-v1`); unknown suites are
    /// recorded, never silently trusted.
    pub suite: String,
    /// Publisher public key text (suite-defined grammar).
    pub key: String,
    /// Signature value (hex text).
    pub value: String,
}

/// What the host could honestly determine about a package's
/// signature (doc 06 §5.7 step 3 + §10 Q5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureStatus {
    /// No signature block in the package.
    Unsigned,
    /// A block parsed; its suite is known by name but v1 has no
    /// verifier — the package installs only with explicit user
    /// consent, recorded as such.
    Unverified { suite: String },
    /// A block parsed but names a suite the host has never heard of.
    UnknownSuite { suite: String },
}

/// Suites the format documents (v1: exactly one, unverified).
pub const KNOWN_SUITES: &[&str] = &["ed25519-detached-v1"];

impl SignatureBlock {
    /// Parse the text block.
    ///
    /// # Errors
    /// [`PackageError::SignatureUnparsable`] for wrong magic,
    /// missing fields or non-hex values.
    pub fn parse(text: &str) -> Result<SignatureBlock, PackageError> {
        let mut lines = text.lines();
        let version_line = lines
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        if version_line != "lumenext-signature-v1" {
            return Err(PackageError::SignatureUnparsable {
                detail: format!("first line must be `lumenext-signature-v1`, got `{version_line}`"),
            });
        }
        let mut suite = None;
        let mut key = None;
        let mut value = None;
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let (field, rest) = line
                .split_once(':')
                .ok_or_else(|| PackageError::SignatureUnparsable {
                    detail: format!("line `{line}` is not `field: value`"),
                })?;
            let rest = rest.trim();
            match field.trim() {
                "suite" => suite = Some(rest.to_string()),
                "key" => key = Some(rest.to_string()),
                "value" => value = Some(rest.to_string()),
                other => {
                    return Err(PackageError::SignatureUnparsable {
                        detail: format!("unknown field `{other}`"),
                    });
                }
            }
        }
        let suite = suite.ok_or_else(|| PackageError::SignatureUnparsable {
            detail: "missing `suite:` field".to_string(),
        })?;
        let key = key.ok_or_else(|| PackageError::SignatureUnparsable {
            detail: "missing `key:` field".to_string(),
        })?;
        let value = value.ok_or_else(|| PackageError::SignatureUnparsable {
            detail: "missing `value:` field".to_string(),
        })?;
        if !value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(PackageError::SignatureUnparsable {
                detail: "`value:` is not hex".to_string(),
            });
        }
        Ok(SignatureBlock {
            version_line,
            suite,
            key,
            value,
        })
    }

    /// The trust status this block yields under v1's honest policy.
    pub fn status(&self) -> SignatureStatus {
        if KNOWN_SUITES.contains(&self.suite.as_str()) {
            SignatureStatus::Unverified {
                suite: self.suite.clone(),
            }
        } else {
            SignatureStatus::UnknownSuite {
                suite: self.suite.clone(),
            }
        }
    }
}

/// One central-directory entry.
#[derive(Debug, Clone)]
struct ZipEntry {
    name: String,
    method: u16,
    compressed_size: u32,
    uncompressed_size: u32,
    crc32: u32,
    local_offset: u32,
}

/// Parse and extract a `.lumenext` archive from bytes.
///
/// Structure checks only (doc 06 §5.7 step 2): ZIP shape, stored
/// method, per-entry CRC, caps. Manifest *schema* validation and
/// provenance checks are the installer's job ([`crate::install`]).
///
/// # Errors
/// [`PackageError`] for every structural failure — typed, never a
/// panic.
pub fn read_package(bytes: &[u8]) -> Result<LumenextPackage, PackageError> {
    let len = bytes.len() as u64;
    if len > PACKAGE_CAP_BYTES {
        return Err(PackageError::TooLarge { size: len });
    }
    let entries = read_central_directory(bytes)?;
    let manifest = extract(bytes, &entries, ENTRY_MANIFEST)?;
    let module = extract(bytes, &entries, ENTRY_MODULE)?;
    if module.len() < 4 || module[..4] != [0x00, 0x61, 0x73, 0x6d] {
        return Err(PackageError::BadMagic {
            observed: module
                .iter()
                .take(4)
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        });
    }
    let signature = match extract(bytes, &entries, ENTRY_SIGNATURE) {
        Ok(sig_bytes) => {
            let text = String::from_utf8_lossy(&sig_bytes).to_string();
            Some(SignatureBlock::parse(&text)?)
        }
        Err(PackageError::MissingEntry { .. }) => None,
        Err(other) => return Err(other),
    };
    Ok(LumenextPackage {
        manifest_toml: manifest,
        module_wasm: module,
        signature,
    })
}

/// Locate the end-of-central-directory record and parse the central
/// directory.
fn read_central_directory(bytes: &[u8]) -> Result<Vec<ZipEntry>, PackageError> {
    // EOCD signature 0x06054b50; scan backwards over the (≤64 KiB)
    // comment.
    let scan_start = bytes.len().saturating_sub(66_000);
    let mut eocd = None;
    let mut i = bytes.len();
    while i > scan_start {
        i -= 1;
        if bytes[i..].len() >= 22
            && bytes[i..i + 4] == [0x50, 0x4b, 0x05, 0x06]
        {
            eocd = Some(i);
            break;
        }
    }
    let eocd = eocd.ok_or(PackageError::NotZip)?;
    let u16le = |off: usize| -> u16 {
        u16::from_le_bytes([bytes[off], bytes[off + 1]])
    };
    let u32le = |off: usize| -> u32 {
        u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
    };
    let entry_count = u16le(eocd + 10) as usize;
    if entry_count > MAX_ENTRIES {
        return Err(PackageError::Malformed {
            detail: format!("{entry_count} entries (cap {MAX_ENTRIES})"),
        });
    }
    let cd_offset = u32le(eocd + 16) as usize;
    let mut entries = Vec::with_capacity(entry_count);
    let mut off = cd_offset;
    for _ in 0..entry_count {
        if off + 46 > bytes.len() || bytes[off..off + 4] != [0x50, 0x4b, 0x01, 0x02] {
            return Err(PackageError::Malformed {
                detail: format!("bad central-directory header at byte {off}"),
            });
        }
        let method = u16le(off + 10);
        let crc = u32le(off + 16);
        let compressed = u32le(off + 20);
        let uncompressed = u32le(off + 24);
        let name_len = u16le(off + 28) as usize;
        let extra_len = u16le(off + 30) as usize;
        let comment_len = u16le(off + 32) as usize;
        let local_offset = u32le(off + 42);
        let name_start = off + 46;
        let name_end = name_start + name_len;
        if name_end > bytes.len() {
            return Err(PackageError::Malformed {
                detail: "central-directory name overruns the archive".to_string(),
            });
        }
        let name = String::from_utf8_lossy(&bytes[name_start..name_end]).to_string();
        entries.push(ZipEntry {
            name,
            method,
            compressed_size: compressed,
            uncompressed_size: uncompressed,
            crc32: crc,
            local_offset,
        });
        off = name_end + extra_len + comment_len;
    }
    Ok(entries)
}

/// Extract one named entry (stored method only, CRC-verified).
fn extract(bytes: &[u8], entries: &[ZipEntry], name: &str) -> Result<Vec<u8>, PackageError> {
    let entry = entries
        .iter()
        .find(|e| e.name == name)
        .ok_or_else(|| PackageError::MissingEntry {
            name: name.to_string(),
        })?;
    if entry.method != 0 {
        return Err(PackageError::CompressedEntry {
            method: entry.method,
        });
    }
    let size = entry.uncompressed_size as u64;
    if size > ENTRY_CAP_BYTES {
        return Err(PackageError::EntryTooLarge {
            detail: format!("`{name}` is {size} bytes (cap {ENTRY_CAP_BYTES})"),
        });
    }
    if entry.compressed_size != entry.uncompressed_size {
        return Err(PackageError::Malformed {
            detail: format!("stored entry `{name}` has mismatched sizes"),
        });
    }
    // Local file header: signature(4) + fields(22) then name +
    // extra. Name and extra lengths can differ from the central
    // directory's, so read them here.
    let lo = entry.local_offset as usize;
    if lo + 30 > bytes.len() || bytes[lo..lo + 4] != [0x50, 0x4b, 0x03, 0x04] {
        return Err(PackageError::Malformed {
            detail: format!("bad local header for `{name}`"),
        });
    }
    let name_len = u16::from_le_bytes([bytes[lo + 26], bytes[lo + 27]]) as usize;
    let extra_len = u16::from_le_bytes([bytes[lo + 28], bytes[lo + 29]]) as usize;
    let data_start = lo + 30 + name_len + extra_len;
    let data_end = data_start + size as usize;
    if data_end > bytes.len() {
        return Err(PackageError::Malformed {
            detail: format!("entry `{name}` data overruns the archive"),
        });
    }
    let data = &bytes[data_start..data_end];
    let computed = crc32(data);
    if computed != entry.crc32 {
        return Err(PackageError::CrcMismatch {
            name: name.to_string(),
        });
    }
    Ok(data.to_vec())
}

/// The `sha256:<hex>` provenance digest of a module (doc 06 §2.2).
pub fn module_digest(wasm: &[u8]) -> String {
    format!("sha256:{}", sha256_hex(wasm))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// House test helper (media-probe precedent).
    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        }
    }

    #[test]
    fn signature_block_parses_and_classifies() {
        let text = "lumenext-signature-v1\nsuite: ed25519-detached-v1\nkey: 9f00cafe\nvalue: deadbeef01\n";
        let block = ok(SignatureBlock::parse(text));
        assert_eq!(block.suite, "ed25519-detached-v1");
        assert_eq!(
            block.status(),
            SignatureStatus::Unverified {
                suite: "ed25519-detached-v1".to_string()
            }
        );
        let unknown = ok(SignatureBlock::parse(
            "lumenext-signature-v1\nsuite: rot13-rsa-v9\nkey: x\nvalue: ff\n",
        ));
        assert_eq!(
            unknown.status(),
            SignatureStatus::UnknownSuite {
                suite: "rot13-rsa-v9".to_string()
            }
        );
        for bad in [
            "",
            "not-the-magic\nsuite: x\nkey: y\nvalue: ff\n",
            "lumenext-signature-v1\nsuite: x\nkey: y\nvalue: zznothex\n",
            "lumenext-signature-v1\nkey: y\nvalue: ff\n",
            "lumenext-signature-v1\nsuite: x\nkey: y\nvalue: ff\nextra: 1\n",
        ] {
            assert!(SignatureBlock::parse(bad).is_err(), "rejected: {bad:?}");
        }
    }

    #[test]
    fn garbage_is_not_a_zip() {
        assert!(matches!(
            read_package(b"not a zip at all"),
            Err(PackageError::NotZip)
        ));
    }

    #[test]
    fn module_digest_is_the_provenance_form() {
        let digest = module_digest(&[]);
        assert!(digest.starts_with("sha256:"));
        assert_eq!(digest.len(), "sha256:".len() + 64);
        assert_eq!(
            digest,
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
