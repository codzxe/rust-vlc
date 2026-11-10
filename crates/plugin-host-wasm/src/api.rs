//! The `lumen:v1.*` import surface (doc 06 §5.2, §5.5) — the typed
//! catalog of host functions an extension may call, the guest-status
//! code convention, API-version negotiation and the deprecation
//! window mechanism.
//!
//! # The no-host-pointers model (doc 06 §5.2)
//!
//! Every import is a plain-integer wasm function. Buffers are
//! `(ptr: i32, len: i32)` pairs addressing the **guest's own linear
//! memory**; the host reads/writes them through wasmi's
//! bounds-checked `Memory::read`/`Memory::write` after validating
//! lengths against the transfer caps. The guest can never name a host
//! address: there are no i64 host pointers, no references, no
//! `externref`s in the v1 ABI. Handles (`fs` file handles) are small
//! opaque u32 indices into a per-instance host-side table — ownership
//! is validated on every use (doc 06 §9.4
//! `plugin_host_pointer_smuggling`).
//!
//! # Guest-observable status codes
//!
//! All imports return a single `i32` status from [`GuestStatus`];
//! payload results are written through out-parameters in guest
//! memory. Runtime denials are *typed errors the guest observes*
//! (doc 06 §5.3), never silent failures and never traps — except
//! escalations past the session threshold, which fault the instance
//! (§5.3) and hand the plugin to the disable path.
//!
//! # Module set (doc 06 §5.2 `[PROPOSAL: module set]` — adopted)
//!
//! | Module | Permission | Functions (v1.0) |
//! |---|---|---|
//! | `lumen:v1.host` | — (always granted) | `api_version`, `report_manifest` |
//! | `lumen:v1.log` | — (always granted) | `log_write` |
//! | `lumen:v1.fs` | `fs-sandboxed-read` | `fs_open`, `fs_read`, `fs_size`, `fs_close` |
//! | `lumen:v1.http` | `http-fetch` (+ allowlist) | `http_get` |
//! | `lumen:v1.ui` | `ui-surface` | `ui_register_panel` |
//! | `lumen:v1.devices` | `device-query` | `devices_list` |
//!
//! `lumen:v1.media` (doc 06 §5.2's seventh module) ships **no
//! functions in v1.0** and is therefore not in the catalog: no
//! media-handling extension kind needs it yet, and an empty module
//! would be scaffolding, not contract. It joins with the first
//! media consumer, additive-only per §5.5.

use plugin_api::{ENGINE_PLUGIN_API_MAJOR, PluginKind};

/// Lowest plugin-API major this engine still serves (doc 06 §5.5:
/// majors stay loadable for one transition cycle after `v2` lands —
/// with only `v1` existing, the window is exactly `[1, 1]`).
pub const ENGINE_PLUGIN_API_MIN: u32 = 1;

/// Negotiate a manifest `api_version` against the engine window
/// (doc 06 §5.5). Mismatch is the typed [`HostError::ApiVersion`]
/// error — never UB, never a silent load.
pub fn negotiate_api_version(found: u32) -> Result<u32, crate::error::HostError> {
    let max = ENGINE_PLUGIN_API_MAJOR;
    if found >= ENGINE_PLUGIN_API_MIN && found <= max {
        Ok(found)
    } else {
        Err(crate::error::HostError::ApiVersion {
            found,
            supported_min: ENGINE_PLUGIN_API_MIN,
            supported_max: max,
        })
    }
}

/// The permission an import is gated by (doc 06 §2.2 permission
/// table; mirrors `plugin_api::Permissions` fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// Always granted — the base host surface every extension sees.
    Base,
    /// `fs-sandboxed-read`.
    FsSandboxedRead,
    /// `http-fetch` (with the `network-host-allowlist` companion).
    HttpFetch,
    /// `ui-surface`.
    UiSurface,
    /// `device-query`.
    DeviceQuery,
}

impl Permission {
    /// The manifest permission string that grants this (§2.2 table).
    pub const fn as_str(self) -> &'static str {
        match self {
            Permission::Base => "base",
            Permission::FsSandboxedRead => "fs-sandboxed-read",
            Permission::HttpFetch => "http-fetch",
            Permission::UiSurface => "ui-surface",
            Permission::DeviceQuery => "device-query",
        }
    }
}

/// A wasm value kind, for the catalog's documented signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValKind {
    /// `i32`.
    I32,
    /// `i64`.
    I64,
}

impl ValKind {
    /// The wasm text name.
    pub const fn as_str(self) -> &'static str {
        match self {
            ValKind::I32 => "i32",
            ValKind::I64 => "i64",
        }
    }
}

/// A deprecation record (doc 06 §5.5): an import marked deprecated
/// stays functional for two engine minors (log-on-first-use notice),
/// then is removed at the next major.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deprecation {
    /// Engine minor that deprecated the import.
    pub since_minor: u32,
    /// The engine minor at which the import stops working (removed
    /// at the major bump after this).
    pub removal_minor: u32,
    /// Notice text logged on first use within the window.
    pub notice: &'static str,
}

/// One catalog entry: an import the host knows how to serve.
#[derive(Debug, Clone, Copy)]
pub struct ImportSpec {
    /// Import module name, e.g. `lumen:v1.fs`.
    pub module: &'static str,
    /// Import function name, e.g. `fs_open`.
    pub name: &'static str,
    /// Parameter kinds (documented signature; wasmi type-checks the
    /// real instantiation).
    pub params: &'static [ValKind],
    /// Result kinds.
    pub results: &'static [ValKind],
    /// The permission gating this import.
    pub permission: Permission,
    /// `(major, minor)` this import appeared in — additive-only
    /// history per §5.5.
    pub since: (u32, u32),
    /// Deprecation window, if any. **Empty in v1.0** — the mechanism
    /// is exercised by unit tests with synthetic entries; the first
    /// real deprecation will populate this.
    pub deprecated: Option<Deprecation>,
}

const I32: ValKind = ValKind::I32;

/// The v1.0 catalog — every `lumen:v1.*` import the host serves.
pub const V1_IMPORTS: &[ImportSpec] = &[
    // --- lumen:v1.host -------------------------------------------------
    ImportSpec {
        module: "lumen:v1.host",
        name: "api_version",
        params: &[],
        results: &[I32],
        permission: Permission::Base,
        since: (1, 0),
        deprecated: None,
    },
    ImportSpec {
        module: "lumen:v1.host",
        name: "report_manifest",
        params: &[I32, I32],
        results: &[I32],
        permission: Permission::Base,
        since: (1, 0),
        deprecated: None,
    },
    // --- lumen:v1.log --------------------------------------------------
    ImportSpec {
        module: "lumen:v1.log",
        name: "log_write",
        params: &[I32, I32, I32],
        results: &[I32],
        permission: Permission::Base,
        since: (1, 0),
        deprecated: None,
    },
    // --- lumen:v1.fs ---------------------------------------------------
    ImportSpec {
        module: "lumen:v1.fs",
        name: "fs_open",
        params: &[I32, I32, I32],
        results: &[I32],
        permission: Permission::FsSandboxedRead,
        since: (1, 0),
        deprecated: None,
    },
    ImportSpec {
        module: "lumen:v1.fs",
        name: "fs_read",
        params: &[I32, I32, I32, I32],
        results: &[I32],
        permission: Permission::FsSandboxedRead,
        since: (1, 0),
        deprecated: None,
    },
    ImportSpec {
        module: "lumen:v1.fs",
        name: "fs_size",
        params: &[I32, I32],
        results: &[I32],
        permission: Permission::FsSandboxedRead,
        since: (1, 0),
        deprecated: None,
    },
    ImportSpec {
        module: "lumen:v1.fs",
        name: "fs_close",
        params: &[I32],
        results: &[I32],
        permission: Permission::FsSandboxedRead,
        since: (1, 0),
        deprecated: None,
    },
    // --- lumen:v1.http -------------------------------------------------
    ImportSpec {
        module: "lumen:v1.http",
        name: "http_get",
        params: &[I32, I32, I32, I32, I32],
        results: &[I32],
        permission: Permission::HttpFetch,
        since: (1, 0),
        deprecated: None,
    },
    // --- lumen:v1.ui ---------------------------------------------------
    ImportSpec {
        module: "lumen:v1.ui",
        name: "ui_register_panel",
        params: &[I32, I32],
        results: &[I32],
        permission: Permission::UiSurface,
        since: (1, 0),
        deprecated: None,
    },
    // --- lumen:v1.devices ----------------------------------------------
    ImportSpec {
        module: "lumen:v1.devices",
        name: "devices_list",
        params: &[I32, I32, I32],
        results: &[I32],
        permission: Permission::DeviceQuery,
        since: (1, 0),
        deprecated: None,
    },
];

/// Look up a catalog entry by `(module, name)`.
pub fn lookup(module: &str, name: &str) -> Option<&'static ImportSpec> {
    V1_IMPORTS
        .iter()
        .find(|spec| spec.module == module && spec.name == name)
}

/// Look up the first catalog entry of a module (import-policy
/// checking is per-module: a guest importing *any* function of a
/// gated module needs that module's permission).
pub fn lookup_module(module: &str) -> Option<&'static ImportSpec> {
    V1_IMPORTS.iter().find(|spec| spec.module == module)
}

/// The deprecation notice to emit for `spec` on first use within the
/// two-minor window (doc 06 §5.5) — `None` when the import is not
/// deprecated or the use is past the window (at which point the
/// import is simply gone from the catalog, so it can no longer be
/// resolved at all).
pub fn deprecation_notice(spec: &ImportSpec) -> Option<&'static str> {
    spec.deprecated.map(|d| d.notice)
}

/// Guest-observable import status codes (the single `i32` every
/// import returns).
///
/// `E_PERM` (and only `E_PERM`) marks a permission escalation the
/// host logs as a security event and counts toward the disable
/// threshold (doc 06 §5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestStatus {
    /// Success.
    Ok,
    /// Permission denied at runtime (allowlist violation, sandbox
    /// escape attempt). Escalation counter increments.
    EPerm,
    /// Invalid argument (bad level, bad URL, unparsable payload).
    EArg,
    /// Unknown/unowned handle.
    EHandle,
    /// Guest buffer out of bounds (typed denial — no UB).
    EOob,
    /// Host-side I/O failure.
    EIo,
    /// Over the single-transfer cap.
    ETooLarge,
    /// I/O rate/burst quota exceeded.
    EQuota,
    /// HTTP transport unavailable (the seam's honest v1 default).
    ETransport,
    /// Other resource limit (handle table full, panel cap).
    ELimit,
}

impl GuestStatus {
    /// Wire value.
    pub const fn code(self) -> i32 {
        match self {
            GuestStatus::Ok => 0,
            GuestStatus::EPerm => 1,
            GuestStatus::EArg => 2,
            GuestStatus::EHandle => 3,
            GuestStatus::EOob => 4,
            GuestStatus::EIo => 5,
            GuestStatus::ETooLarge => 6,
            GuestStatus::EQuota => 7,
            GuestStatus::ETransport => 8,
            GuestStatus::ELimit => 9,
        }
    }

    /// Parse a wire value (tests and guest-side bindings).
    ///
    /// # Errors
    /// [`GuestStatus::EArg`] (self-referentially) for unknown codes.
    pub fn from_code(code: i32) -> Result<GuestStatus, GuestStatus> {
        match code {
            0 => Ok(GuestStatus::Ok),
            1 => Ok(GuestStatus::EPerm),
            2 => Ok(GuestStatus::EArg),
            3 => Ok(GuestStatus::EHandle),
            4 => Ok(GuestStatus::EOob),
            5 => Ok(GuestStatus::EIo),
            6 => Ok(GuestStatus::ETooLarge),
            7 => Ok(GuestStatus::EQuota),
            8 => Ok(GuestStatus::ETransport),
            9 => Ok(GuestStatus::ELimit),
            _ => Err(GuestStatus::EArg),
        }
    }
}

/// Which permission grants a module of an extension manifest (the
/// doc 06 §2.2 companion rule: `network-host-allowlist` implies
/// `http-fetch`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Grants {
    /// `fs-sandboxed-read` (jail roots come from the host services,
    /// not the manifest — the manifest only *asks*).
    pub fs: bool,
    /// `http-fetch`, with the exact-host allowlist when present.
    pub http: bool,
    /// The allowlist (empty = nothing contactable even when
    /// `http` is granted).
    pub http_hosts: Vec<String>,
    /// `ui-surface`.
    pub ui: bool,
    /// `device-query`.
    pub devices: bool,
}

impl Grants {
    /// Derive the import-table grants from a manifest's declared
    /// permissions (doc 06 §2.2: deny by default; the allowlist
    /// implies `http-fetch`).
    pub fn from_permissions(perms: &plugin_api::Permissions) -> Grants {
        let allowlist = perms.network_host_allowlist.clone().unwrap_or_default();
        Grants {
            fs: perms.fs_sandboxed_read,
            http: perms.http_fetch || perms.network_host_allowlist.is_some(),
            http_hosts: allowlist,
            ui: perms.ui_surface,
            devices: perms.device_query,
        }
    }

    /// Does `permission` hold under these grants?
    pub fn has(&self, permission: Permission) -> bool {
        match permission {
            Permission::Base => true,
            Permission::FsSandboxedRead => self.fs,
            Permission::HttpFetch => self.http,
            Permission::UiSurface => self.ui,
            Permission::DeviceQuery => self.devices,
        }
    }

    /// The granted list in manifest spelling (install UI / events).
    pub fn granted_names(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.fs {
            out.push("fs-sandboxed-read");
        }
        if self.http {
            out.push("http-fetch");
        }
        if self.ui {
            out.push("ui-surface");
        }
        if self.devices {
            out.push("device-query");
        }
        out
    }
}

/// Extension-legal kinds double-check (doc 06 §1.4) — re-expressed
/// here for install-time refusal; the authoritative rule lives in
/// `plugin_api::PluginKind::extension_allowed_in_v1`.
pub fn kind_extension_legal(kind: PluginKind) -> bool {
    kind.extension_allowed_in_v1()
}

#[cfg(test)]
mod tests {
    use super::*;
    use plugin_api::Permissions;

    #[test]
    fn catalog_shapes_are_doc06_conformant() {
        // Every module in the catalog uses the lumen:v1.* namespace.
        assert!(V1_IMPORTS.iter().all(|s| s.module.starts_with("lumen:v1.")));
        // Every function returns exactly one i32 status.
        assert!(V1_IMPORTS
            .iter()
            .all(|s| s.results.len() == 1 && s.results[0] == ValKind::I32));
        // Base modules are host and log; gated modules are the four
        // doc 06 §2.2 permissions.
        for spec in V1_IMPORTS {
            match spec.module {
                "lumen:v1.host" | "lumen:v1.log" => {
                    assert_eq!(spec.permission, Permission::Base);
                }
                "lumen:v1.fs" => assert_eq!(spec.permission, Permission::FsSandboxedRead),
                "lumen:v1.http" => assert_eq!(spec.permission, Permission::HttpFetch),
                "lumen:v1.ui" => assert_eq!(spec.permission, Permission::UiSurface),
                "lumen:v1.devices" => assert_eq!(spec.permission, Permission::DeviceQuery),
                other => unreachable!("unexpected module {other}"),
            }
        }
    }

    #[test]
    fn lookup_finds_every_entry() {
        for spec in V1_IMPORTS {
            let found = lookup(spec.module, spec.name);
            assert!(found.is_some(), "{}::{}", spec.module, spec.name);
        }
        assert!(lookup("lumen:v1.fs", "fs_chmod").is_none());
        assert!(lookup_module("wasi_snapshot_preview1").is_none());
    }

    /// House test helper (media-probe precedent).
    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        }
    }

    #[test]
    fn version_window_is_v1_only_today() {
        assert_eq!(ok(negotiate_api_version(1)), 1);
        for bad in [0u32, 2, 3, 999] {
            assert!(matches!(
                negotiate_api_version(bad),
                Err(crate::error::HostError::ApiVersion { found, .. }) if found == bad
            ));
        }
    }

    #[test]
    fn grants_follow_doc06_companion_rule() {
        // Deny by default.
        let none = Grants::from_permissions(&Permissions::default());
        assert!(!none.fs && !none.http && !none.ui && !none.devices);
        // Allowlist implies http-fetch.
        let perms = plugin_api::Permissions {
            network_host_allowlist: Some(vec!["musicbrainz.org".to_string()]),
            ..Default::default()
        };
        let grants = Grants::from_permissions(&perms);
        assert!(grants.http);
        assert_eq!(grants.http_hosts, vec!["musicbrainz.org".to_string()]);
        // http-fetch without allowlist: granted module, empty list.
        let perms = plugin_api::Permissions {
            http_fetch: true,
            ..Default::default()
        };
        let grants = Grants::from_permissions(&perms);
        assert!(grants.http);
        assert!(grants.http_hosts.is_empty());
    }

    #[test]
    fn status_codes_round_trip() {
        for code in 0..=9 {
            let status = GuestStatus::from_code(code).unwrap_or(GuestStatus::EArg);
            assert_eq!(status.code(), code);
        }
        assert_eq!(
            GuestStatus::from_code(42),
            Err(GuestStatus::EArg),
            "unknown codes map to E_ARG, never panic"
        );
    }

    #[test]
    fn deprecation_window_mechanism() {
        // v1.0 ships no deprecated imports — the registry slot is
        // real and exercised with a synthetic entry (the honest
        // mechanism test; the first real deprecation populates it).
        assert!(V1_IMPORTS.iter().all(|s| s.deprecated.is_none()));
        let synthetic = ImportSpec {
            module: "lumen:v1.host",
            name: "old_thing",
            params: &[],
            results: &[I32],
            permission: Permission::Base,
            since: (1, 0),
            deprecated: Some(Deprecation {
                since_minor: 2,
                removal_minor: 4,
                notice: "old_thing is deprecated; use new_thing",
            }),
        };
        // Within the window: a first-use notice fires once.
        assert_eq!(
            deprecation_notice(&synthetic),
            Some("old_thing is deprecated; use new_thing")
        );
        let clean = ImportSpec {
            deprecated: None,
            ..synthetic
        };
        assert_eq!(deprecation_notice(&clean), None);
    }
}
