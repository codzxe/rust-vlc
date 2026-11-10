//! The WS ↔ command-layer bridge — card 7.J1 (doc 01 §2.8 (d)).
//!
//! [`CatalogDispatch`] is the host-side [`control_protocol::server::Dispatch`]
//! the event loop hands every VALIDATED, POLICY-CHECKED request to.
//! v1 embeds no engine (the desktop composition card owns that join),
//! so the served subset is catalog introspection:
//!
//! * `system.get-engine-info` — the product-identity-derived block;
//! * `system.list-commands` — the live catalog table (every id, class,
//!   since — built from `commands::catalog::spec`, never a copy).
//!
//! Everything else answers with the command layer's typed `not-ready`
//! (stage `control`) — an honest gap, never a silent no-op and never a
//! fake success. The wire vocabulary is 100% the ONE command layer's
//! (ARCH-5): this module constructs `WireResponse`s, it invents nothing.

use commands::Command;
use commands::WireRequest;
use commands::WireResponse;
use commands::catalog::command::CommandListing;
use commands::catalog::command::CommandResult;
use commands::catalog::ids::CommandId;
use commands::catalog::spec::command_spec;
use commands::catalog::types::EngineInfo;
use commands::envelope::CommandOutcome;
use commands::envelope::Generation;
use commands::error::CommandError;
use commands::error::ErrorCode;
use commands::error::Stage;

use control_protocol::server::Dispatch;

/// The not-ready message every non-introspection command answers with.
const NOT_READY: &str = "this control-server build embeds no engine: only catalog \
     introspection (system.get-engine-info, system.list-commands) is served; every \
     other command is not-ready until the engine-embedding composition lands";

/// The catalog-introspection dispatch (v1 host).
#[derive(Debug, Clone)]
pub struct CatalogDispatch {
    /// The `engineInfo` block served for `system.get-engine-info`
    /// (product-identity-derived; built once at startup).
    info: EngineInfo,
}

impl CatalogDispatch {
    /// New dispatch advertising `info`.
    pub fn new(info: EngineInfo) -> CatalogDispatch {
        CatalogDispatch { info }
    }
}

impl Dispatch for CatalogDispatch {
    fn dispatch(&mut self, request: WireRequest) -> WireResponse {
        let result = match request.command {
            Command::SystemGetEngineInfo => {
                Ok(CommandOutcome::Completed(CommandResult::EngineInfo {
                    info: self.info.clone(),
                }))
            }
            Command::SystemListCommands => {
                Ok(CommandOutcome::Completed(CommandResult::CommandListings {
                    commands: catalog_listings(),
                }))
            }
            _ => Err(CommandError::new(
                Stage::Control,
                ErrorCode::NotReady,
                NOT_READY,
            )),
        };
        WireResponse {
            request_id: request.request_id,
            session: None,
            generation: Generation(0),
            result,
        }
    }
}

/// Every catalog command as a `system.list-commands` row, straight from
/// the live spec table (doc 05 §2.12: UIs and the control server build
/// menus and validate keymaps without hardcoding the catalog).
pub fn catalog_listings() -> Vec<CommandListing> {
    CommandId::ALL
        .iter()
        .filter_map(|id| {
            command_spec(*id).map(|spec| CommandListing {
                id: id.as_str().to_owned(),
                class: spec.class,
                // The v1 namespace: every command appeared in catalog
                // major 1 (doc 05 §6.1); no deprecations, no
                // cross-command requirements in v1.
                // lumen-allow-literal: catalog major version tag (doc 05 §6.1 wire vocabulary)
                since: "1".to_owned(),
                deprecated: None,
                requires: Vec::new(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::envelope::RequestId;

    fn engine_info() -> EngineInfo {
        EngineInfo {
            engine_version: "0.1.0-test".to_owned(),
            catalog_version: commands::catalog::spec::catalog_checksum().to_string(),
            // lumen-allow-literal: protocol version string is wire vocabulary (doc 05 §5.4)
            protocol_version: "lumen-control/1".to_owned(),
            host: "control-server".to_owned(),
            features: Vec::new(),
            plugins: Vec::new(),
            decoders: Vec::new(),
            sessions: Vec::new(),
        }
    }

    #[test]
    fn the_catalog_lists_all_64_commands_from_the_live_spec() {
        let listings = catalog_listings();
        assert_eq!(listings.len(), CommandId::ALL.len());
        assert_eq!(listings.len(), 64, "doc 05 §2.1's v1 table");
        // Ordered by the catalog table, every row well-formed.
        let ids: Vec<&str> = listings.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids.first().copied(), Some("session.open"));
        assert!(ids.contains(&"system.list-commands"));
        assert!(ids.contains(&"config.set-setting"));
        for row in &listings {
            assert_eq!(row.since, "1");
            assert_eq!(row.deprecated, None);
            assert!(row.requires.is_empty());
        }
    }

    #[test]
    fn engine_info_and_list_commands_serve_the_catalog_subset() {
        let mut dispatch = CatalogDispatch::new(engine_info());
        let response = dispatch.dispatch(WireRequest {
            request_id: RequestId(1),
            command: Command::SystemGetEngineInfo,
        });
        assert_eq!(response.request_id, RequestId(1));
        match response.result {
            Ok(CommandOutcome::Completed(CommandResult::EngineInfo { info })) => {
                assert_eq!(info.host, "control-server");
            }
            other => unreachable!("engine-info result: {other:?}"),
        }
        let response = dispatch.dispatch(WireRequest {
            request_id: RequestId(2),
            command: Command::SystemListCommands,
        });
        match response.result {
            Ok(CommandOutcome::Completed(CommandResult::CommandListings { commands })) => {
                assert_eq!(commands.len(), 64);
            }
            other => unreachable!("list-commands result: {other:?}"),
        }
    }

    #[test]
    fn everything_else_is_a_typed_not_ready_never_a_fake_success() {
        let mut dispatch = CatalogDispatch::new(engine_info());
        let commands = [
            Command::PlaybackPlay,
            Command::SessionOpen {
                source: commands::catalog::types::SourceSpec::Url {
                    url: "http://127.0.0.1/never-fetched".to_owned(),
                },
                options: commands::catalog::types::OpenOptions::default(),
            },
        ];
        for command in commands {
            let response = dispatch.dispatch(WireRequest {
                request_id: RequestId(1),
                command: command.clone(),
            });
            match response.result {
                Err(err) => {
                    assert_eq!(err.code, ErrorCode::NotReady);
                    assert_eq!(err.stage, Stage::Control);
                    assert!(err.message.contains("no engine"), "{}", err.message);
                }
                other => unreachable!("{command:?} answered {other:?}"),
            }
        }
    }
}
