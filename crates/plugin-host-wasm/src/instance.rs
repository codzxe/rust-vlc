//! The extension instance: one wasmi module instance with its own
//! store, memory ceiling, fuel budget and import policy (doc 06
//! §5.2–§5.4).
//!
//! Trap containment comes free with wasmi (ADR-002): a trap destroys
//! only this instance's `Store` — the host process and every other
//! instance are untouched. This card wires what falls out naturally
//! (typed fault classification + a `Faulted` status that refuses
//! further dispatch); the restart/backoff policy is issue #46.

use wasmi::errors::MemoryError;
use wasmi::{Config, Engine, Module, Store, TrapCode, Val};

use plugin_api::PluginManifest;

use crate::api::{self, Grants};
use crate::error::HostError;
use crate::imports::{self, check_import_policy, FaultKind, HostFault, HostState};
use crate::limits::ResourceLimits;
use crate::state::{HostServices, InstanceState, ManifestExpectation, SecurityLedger};

/// Instance lifecycle status (doc 06 §5.6, minus the backoff
/// machinery that belongs to issue #46).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceStatus {
    /// Instantiated and dispatchable.
    Ready,
    /// Faulted: a trap, a limit breach or the escalation disable
    /// stopped it. Further dispatch is refused with
    /// [`HostError::InstanceFaulted`].
    Faulted { reason: String },
}

/// A spawned extension instance.
pub struct WasmInstance {
    store: Store<HostState>,
    instance: wasmi::Instance,
    status: InstanceStatus,
    /// The manifest this instance was spawned from (provenance
    /// honesty anchor for `report_manifest`).
    manifest: PluginManifest,
}

impl WasmInstance {
    /// Spawn: wasm magic → module compile → api-version negotiation →
    /// import-policy check → store with memory ceiling + fuel →
    /// linker with only granted modules → instantiate (under the
    /// startup fuel budget).
    ///
    /// # Errors
    /// Typed [`HostError`]s, in the order the checks run: bad magic
    /// (`Package::BadMagic`), compile errors, [`HostError::ApiVersion`],
    /// [`HostError::UndeclaredImport`]/[`HostError::UnknownImportModule`],
    /// startup fuel exhaustion ([`HostError::CpuLimit`]), memory-cap
    /// breach during start ([`HostError::MemoryLimit`]).
    pub fn spawn(
        engine: &Engine,
        wasm: &[u8],
        manifest: PluginManifest,
        services: HostServices,
        limits: ResourceLimits,
        ledger: ArcLedger,
    ) -> Result<WasmInstance, HostError> {
        // Native-code fence (doc 06 §6): the payload must be wasm.
        if wasm.len() < 4 || wasm[..4] != [0x00, 0x61, 0x73, 0x6d] {
            return Err(HostError::Package(Box::new(
                crate::error::PackageError::BadMagic {
                    observed: wasm
                        .iter()
                        .take(4)
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>(),
                },
            )));
        }
        api::negotiate_api_version(manifest.api_version)?;
        let module = Module::new(engine, wasm)
            .map_err(|e| HostError::Instantiation(format!("module rejected: {e}")))?;
        let grants = Grants::from_permissions(&manifest.permissions);
        check_import_policy(&module, &grants)?;
        let store_limits = wasmi::StoreLimitsBuilder::new()
            .memory_size(limits.memory_bytes as usize)
            .table_elements(10_000)
            .instances(1)
            .tables(1)
            .memories(1)
            .trap_on_grow_failure(true)
            .build();
        let expectation = ManifestExpectation {
            id: manifest.id.as_str().to_string(),
            kind: manifest.kind,
            version: manifest.version.to_string(),
            api_version: manifest.api_version,
        };
        let mut store = Store::new(
            engine,
            InstanceState::new(
                limits.clone(),
                store_limits,
                expectation,
                &grants,
                ledger,
                services,
            ),
        );
        store.limiter(|state| &mut state.store_limits as &mut dyn wasmi::ResourceLimiter);
        store
            .set_fuel(limits.startup_fuel)
            .map_err(|e| HostError::Instantiation(e.to_string()))?;
        let linker = imports::build_linker(engine, &grants)?;
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .map_err(|e| classify_spawn_error(&e, limits.memory_bytes))?;
        Ok(WasmInstance {
            store,
            instance,
            status: InstanceStatus::Ready,
            manifest,
        })
    }

    /// The manifest this instance was spawned from.
    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    /// Current status.
    pub fn status(&self) -> &InstanceStatus {
        &self.status
    }

    /// A share on the escalation ledger (host-side observation).
    pub fn ledger(&self) -> ArcLedger {
        ArcLedger::clone(&self.store.data().ledger)
    }

    /// The guest's last `report_manifest` payload (F1: the
    /// hello-world handshake — `None` until the guest reported).
    pub fn manifest_report(&self) -> Option<&str> {
        self.store.data().manifest_report.as_deref()
    }

    /// Recent guest log records (`level, line`), oldest first
    /// (bounded clone of the 64-record ring).
    pub fn log_records(&self) -> Vec<(u8, String)> {
        self.store.data().logs.records()
    }

    /// Panels registered through `lumen:v1.ui` (the ui-surface seam).
    pub fn panels(&self) -> &[crate::state::PanelRecord] {
        &self.store.data().panels
    }

    /// Import call trace (`import`, `status`) for conformance runs.
    pub fn import_trace(&self) -> &[(String, i32)] {
        &self.store.data().import_trace
    }

    /// Dispatch an exported function with the per-call fuel budget.
    ///
    /// The v1 entry-point convention is integer parameters and
    /// results: `args` are i64 values narrowed to each parameter's
    /// declared integer type (i32/i64); a float- or reference-typed
    /// parameter/result is an honest typed error, not a silent
    /// reinterpretation.
    ///
    /// Traps, fuel exhaustion and memory-cap breaches fault *this
    /// instance only* (doc 06 §5.6): the typed error names the
    /// cause and the status moves to `Faulted`; the host and every
    /// other instance are unaffected.
    ///
    /// # Errors
    /// [`HostError::InstanceFaulted`] when already faulted;
    /// [`HostError::MissingExport`]; [`HostError::CpuLimit`];
    /// [`HostError::MemoryLimit`]; [`HostError::ExtensionDisabled`]
    /// for the escalation-disable fault; other traps as
    /// [`HostError::Wasm`].
    pub fn invoke(&mut self, export: &str, args: &[i64]) -> Result<Vec<i64>, HostError> {
        if let InstanceStatus::Faulted { reason } = &self.status {
            return Err(HostError::InstanceFaulted {
                reason: reason.clone(),
            });
        }
        let limits = self.store.data().limits.clone();
        self.store
            .set_fuel(limits.call_fuel)
            .map_err(|e| HostError::Wasm(e.to_string()))?;
        let func = self
            .instance
            .get_export(&mut self.store, export)
            .and_then(wasmi::Extern::into_func)
            .ok_or_else(|| HostError::MissingExport {
                name: export.to_string(),
            })?;
        let ty = func.ty(&self.store);
        if ty.params().len() != args.len() {
            return Err(HostError::Wasm(format!(
                "entry point `{export}` takes {} parameters, {} given",
                ty.params().len(),
                args.len()
            )));
        }
        let params: Vec<Val> = ty
            .params()
            .iter()
            .zip(args)
            .map(|(kind, value)| -> Result<Val, HostError> {
                match kind {
                    wasmi::ValType::I32 => match i32::try_from(*value) {
                        Ok(narrow) => Ok(Val::I32(narrow)),
                        Err(_) => Err(HostError::Wasm(format!(
                            "argument {value} does not fit the i32 parameter of `{export}`"
                        ))),
                    },
                    wasmi::ValType::I64 => Ok(Val::I64(*value)),
                    other => Err(HostError::Wasm(format!(
                        "entry point `{export}` has a non-integer parameter ({other:?}) \
                         — outside the v1 ABI"
                    ))),
                }
            })
            .collect::<Result<_, HostError>>()?;
        let mut results: Vec<Val> = ty
            .results()
            .iter()
            .map(|kind| -> Result<Val, HostError> {
                match kind {
                    wasmi::ValType::I32 => Ok(Val::I32(0)),
                    wasmi::ValType::I64 => Ok(Val::I64(0)),
                    other => Err(HostError::Wasm(format!(
                        "entry point `{export}` has a non-integer result ({other:?}) \
                         — outside the v1 ABI"
                    ))),
                }
            })
            .collect::<Result<_, HostError>>()?;
        match func.call(&mut self.store, &params, &mut results) {
            Ok(()) => {
                let mut out = Vec::with_capacity(results.len());
                for val in &results {
                    match val {
                        Val::I64(v) => out.push(*v),
                        Val::I32(v) => out.push(i64::from(*v)),
                        other => {
                            // Unreachable by the results construction
                            // above; reported honestly if wasmi ever
                            // hands back something else.
                            return Err(HostError::Wasm(format!(
                                "entry point `{export}` returned a \
                                 non-integer value ({other:?})"
                            )));
                        }
                    }
                }
                Ok(out)
            }
            Err(err) => {
                let host_err = classify_call_error(&err, limits.memory_bytes);
                self.status = InstanceStatus::Faulted {
                    reason: host_err.to_string(),
                };
                Err(host_err)
            }
        }
    }

    /// Read a range of the guest's linear memory (host-side result
    /// retrieval — the same bounds-checked path imports use).
    ///
    /// # Errors
    /// [`HostError::GuestMemoryOob`] for out-of-bounds ranges (typed
    /// §9.4 denial), [`HostError::Wasm`] when the module exports no
    /// memory.
    pub fn read_guest_memory(&self, ptr: u64, len: u64) -> Result<Vec<u8>, HostError> {
        let memory = self
            .instance
            .get_export(&self.store, "memory")
            .and_then(wasmi::Extern::into_memory)
            .ok_or_else(|| HostError::Wasm("module exports no memory".to_string()))?;
        let mut buf = vec![0u8; len as usize];
        memory
            .read(&self.store, ptr as usize, &mut buf)
            .map_err(|_| HostError::GuestMemoryOob {
                ptr,
                len,
                size: memory.size(&self.store) * 64 * 1024,
            })?;
        Ok(buf)
    }
}

/// Shared ledger handle.
pub type ArcLedger = std::sync::Arc<SecurityLedger>;

/// The engine every instance of one host shares (fuel metering on).
pub fn engine() -> Engine {
    let mut config = Config::default();
    config.consume_fuel(true);
    Engine::new(&config)
}

/// Map an instantiation/start error to the typed host error.
fn classify_spawn_error(err: &wasmi::Error, memory_cap: u64) -> HostError {
    if let Some(fault) = err.downcast_ref::<HostFault>() {
        return fault_to_host_error(fault);
    }
    if err.as_trap_code() == Some(TrapCode::OutOfFuel) {
        return HostError::CpuLimit;
    }
    if is_memory_denied(err) {
        return HostError::MemoryLimit { cap_bytes: memory_cap };
    }
    HostError::Instantiation(err.to_string())
}

/// Map a dispatch error to the typed host error.
fn classify_call_error(err: &wasmi::Error, memory_cap: u64) -> HostError {
    if let Some(fault) = err.downcast_ref::<HostFault>() {
        return fault_to_host_error(fault);
    }
    if err.as_trap_code() == Some(TrapCode::OutOfFuel) {
        return HostError::CpuLimit;
    }
    if is_memory_denied(err) {
        return HostError::MemoryLimit { cap_bytes: memory_cap };
    }
    if let Some(trap) = err.as_trap_code() {
        return HostError::Wasm(format!("trap: {trap:?}"));
    }
    HostError::Wasm(err.to_string())
}

fn fault_to_host_error(fault: &HostFault) -> HostError {
    match fault.kind() {
        FaultKind::EscalationDisabled { count } => HostError::ExtensionDisabled {
            reason: format!(
                "escalation threshold reached ({count} permission \
                 denials this session) — instance faulted (doc 06 §5.3)"
            ),
        },
    }
}

/// wasmi reports a store-limiter memory denial either as
/// `MemoryError::ResourceLimiterDeniedAllocation` (grow returning
/// `Err` at the API surface) or — under our `trap_on_grow_failure`
/// posture, doc 06 §5.4 `MemoryOom` — as the trap code
/// `GrowthOperationLimited` when the denial happens inside executing
/// guest code (memory.grow/table.grow trap instead of returning -1).
/// Both shapes are the same fact: the ceiling held.
fn is_memory_denied(err: &wasmi::Error) -> bool {
    if err.as_trap_code() == Some(TrapCode::GrowthOperationLimited) {
        return true;
    }
    matches!(
        err.kind(),
        wasmi::errors::ErrorKind::Memory(
            MemoryError::ResourceLimiterDeniedAllocation | MemoryError::OutOfSystemMemory
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_config_has_fuel_metering() {
        // Fuel metering is the CPU-limit primitive (ADR-002); the
        // engine we hand out must have it on. Observable indirectly:
        // set_fuel on a store succeeds only when the engine
        // consumes fuel.
        let engine = engine();
        let mut store = Store::new(&engine, ());
        assert!(store.set_fuel(1).is_ok());
    }
}
