//! The Lumen WASM extension host (doc 06 §5; cards 7.F1–7.F3, issue
//! #45; runtime: **wasmi 2.0.0 per ADR-002**, the accepted Phase 0
//! decision — pure-Rust interpreter, fuel metering, no WASI).
//!
//! # What this crate is
//!
//! The sandboxed half of the plugin system: everything an *extension*
//! (a `.lumenext` WASM package) touches, from install to dispatch.
//! The registry/selection half (doc 06 §3) is not here. Layer L6
//! (doc 01 §2.8): nothing depends on this crate but the apps that
//! embed it.
//!
//! # The three cards
//!
//! - **F1 — runtime** ([`instance`], [`api`], [`imports`]): wasmi
//!   embedded; linear-memory / no-host-pointers model (every buffer is
//!   a `(ptr, len)` pair into the guest's own memory, every handle a
//!   u32 index into a per-instance host table); `lumen:v1.*` import
//!   dispatch through a typed catalog with API-version negotiation
//!   (window `[1, 1]` today) and a deprecation-window mechanism.
//! - **F2 — capability enforcement** ([`limits`], [`fsjail`],
//!   [`http`]): permissions on imports (fs-sandboxed-read path-prefix
//!   jail, http-fetch allowlist in front of an honest transport seam,
//!   ui-surface, device-query), memory/fuel ceilings (wasmi store
//!   limits + fuel metering), I/O quotas (token bucket, transfer
//!   caps) and the doc 06 §9.4 escalation tests.
//! - **F3 — the `.lumenext` package format** ([`package`],
//!   [`install`]): manifest + module + versioned signature suite
//!   (recorded, honestly **unverified** until a Lead-approved crypto
//!   ADR), user-initiated install/remove with consent, no
//!   auto-execution.
//!
//! # Crash isolation (issue #46 boundary)
//!
//! Trap containment comes free with wasmi: a trap destroys only that
//! instance's `Store`; the host process and every other instance are
//! untouched (typed [`error::HostError`] + `Faulted` status). The
//! restart/backoff/`Disabled` scheduler is card #46 — only the
//! escalation-threshold disable (doc 06 §5.3) is wired here because
//! it falls out of the import dispatch naturally.
//!
//! # No-unsafe, bounded, typed
//!
//! No `unsafe` (AGENTS §3.3); bounded memory everywhere (D-2); every
//! failure is a typed [`error::HostError`] with an actionable message
//! (MP §8) — never a panic, never a hang, never a swallowed `Result`.
//!
//! # Module map
//!
//! - [`api`]      — the `lumen:v1.*` import catalog, version
//!   negotiation, guest status codes, permission grants.
//! - [`error`]    — [`error::HostError`] / [`error::PackageError`].
//! - [`fsjail`]   — the `fs-sandboxed-read` canonicalizing path jail.
//! - [`http`]     — the `http-fetch` policy chain + transport seam.
//! - [`imports`]  — host implementations of every import + the
//!   per-instance linker builder (only granted modules).
//! - [`install`]  — the on-disk extension store: install/remove/load,
//!   consent, provenance verification, no-auto-exec.
//! - [`instance`] — [`instance::WasmInstance`]: spawn/invoke with
//!   ceilings, trap classification, fault containment.
//! - [`limits`]   — doc 06 §5.4 resource limits + the I/O token
//!   bucket.
//! - [`package`]  — the `.lumenext` reader (stored-ZIP, CRC-32,
//!   wasm-magic fence) + the signature-suite block.
//! - [`state`]    — per-instance `Store` data: handle table, security
//!   ledger, bounded log, panels.
//! - [`crc32`]    — std-only CRC-32 (ZIP check value).
//! - [`sha256`]   — std-only SHA-256 (FIPS 180-4), for
//!   `provenance.hash`.

pub mod api;
pub mod crc32;
pub mod error;
pub mod fsjail;
pub mod http;
pub mod imports;
pub mod install;
pub mod instance;
pub mod limits;
pub mod package;
pub mod sha256;
pub mod state;

// Headline re-exports: the surface an embedding app touches.
pub use api::{ENGINE_PLUGIN_API_MIN, Grants, Permission};
pub use error::{HostError, PackageError};
pub use fsjail::PathJail;
pub use http::{HttpTransport, NoTransport, TransportError};
pub use install::{ExtensionStore, InstallConsent, InstallReceipt};
pub use instance::{InstanceStatus, WasmInstance, engine};
pub use limits::ResourceLimits;
pub use package::{LumenextPackage, SignatureStatus, read_package};
pub use state::{HostServices, SecurityEvent, SecurityLedger};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headline_surface_is_wired() {
        // One-touch compile check: the embedding surface exists and the
        // version window matches plugin-api's contract major.
        assert_eq!(ENGINE_PLUGIN_API_MIN, 1);
        assert_eq!(
            plugin_api::ENGINE_PLUGIN_API_MAJOR,
            1,
            "the window is [1, 1] while only v1 exists (doc 06 §5.5)"
        );
        let _limits = ResourceLimits::for_kind(plugin_api::PluginKind::UiPanel);
        let _services = HostServices::empty();
    }
}
