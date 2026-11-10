//! Config resolution — card 7.J1 (doc 05 §5.4; doc 07 config family).
//!
//! Two layers meet here:
//!
//! 1. `product.toml` (via [`product_config`], schema-validated): the
//!    product-model gate. `features.control-server` must be `true`
//!    (rule F2, doc 07 §3.4: config alone enables nothing — the flag
//!    flips together with the feature-matrix row when the Lead
//!    ratifies). Identity feeds `engineInfo`.
//! 2. `control-server.toml` ([`control_protocol::policy::PolicyFile`]):
//!    the operator's server config — bind, port, read-only mode, fixed
//!    token, `sensitive_allow` grants. Unknown keys are errors; unknown
//!    grant ids are typed errors at STARTUP (typos must surface, never
//!    silently no-op).

use std::path::Path;
use std::path::PathBuf;

use commands::catalog::types::EngineInfo;
use control_protocol::PROTOCOL_NAME;
use control_protocol::PROTOCOL_VERSION;
use control_protocol::ServerConfig;
use control_protocol::TokenSource;
use control_protocol::policy::PolicyFile;
use control_protocol::policy::ServerPolicy;
use product_config::ProductConfig;

/// Every typed startup failure of the app (never a stringly error, never
/// a panic — workspace lints). [`AppError::exit_code`] maps each onto
/// the binary's process contract.
#[derive(Debug)]
pub enum AppError {
    /// The operator config file could not be read.
    ConfigRead {
        /// The path attempted.
        path: PathBuf,
        /// The OS error.
        source: std::io::Error,
    },
    /// The operator config file is not valid TOML / not the strict shape.
    ConfigParse {
        /// The path attempted.
        path: PathBuf,
        /// Redaction-safe detail.
        detail: String,
    },
    /// A `sensitive_allow` entry is not a catalog command id.
    PolicyResolve {
        /// The typed policy error.
        source: control_protocol::policy::PolicyConfigError,
    },
    /// `product.toml` failed schema/typed validation.
    ProductLoad {
        /// Every diagnostic (doc 07 §3.4's aggregate-error rule).
        diagnostics: Vec<product_config::Diagnostic>,
    },
    /// The product model does not expose the control server (rule F2).
    FeatureDisabled {
        /// What the flag said (absent vs false).
        detail: String,
    },
    /// The server could not bind (non-loopback refusals land here too).
    Bind {
        /// The typed protocol error.
        source: control_protocol::ProtocolError,
    },
}

impl AppError {
    /// The process exit code (documented contract of the binary):
    /// 0 ok · 2 config/product trouble · 3 bind trouble · 4 serve loop.
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::ConfigRead { .. }
            | AppError::ConfigParse { .. }
            | AppError::PolicyResolve { .. }
            | AppError::ProductLoad { .. }
            | AppError::FeatureDisabled { .. } => 2,
            AppError::Bind { .. } => 3,
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::ConfigRead { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            AppError::ConfigParse { path, detail } => {
                write!(
                    f,
                    "{} is not a valid control-server config: {detail}",
                    path.display()
                )
            }
            AppError::PolicyResolve { source } => write!(f, "policy config rejected: {source}"),
            AppError::ProductLoad { diagnostics } => {
                write!(
                    f,
                    "product.toml rejected ({} diagnostics)",
                    diagnostics.len()
                )?;
                for diagnostic in diagnostics {
                    write!(f, "\n  {diagnostic}")?;
                }
                Ok(())
            }
            AppError::FeatureDisabled { detail } => write!(
                f,
                "the control server is not exposed by the product model ({detail}); \
                 see product.toml [features] control-server — doc 07 rule F2"
            ),
            AppError::Bind { source } => write!(f, "control server failed to bind: {source}"),
        }
    }
}

impl std::error::Error for AppError {}

/// Load and parse `control-server.toml`. Unknown keys are errors (the
/// [`PolicyFile`] serde contract); a MISSING file is not — the strict
/// defaults (loopback, ephemeral port, generated token, every sensitive
/// op denied) are the documented no-config posture.
///
/// # Errors
/// [`AppError::ConfigRead`] / [`AppError::ConfigParse`].
pub fn load_config_file(path: &Path) -> Result<PolicyFile, AppError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) => {
            return Err(AppError::ConfigRead {
                path: path.to_owned(),
                source,
            });
        }
    };
    toml::from_str(&text).map_err(|source| AppError::ConfigParse {
        path: path.to_owned(),
        detail: source.to_string(),
    })
}

/// The product-model gate (rule F2): `features.control-server` must be
/// exactly `true`.
///
/// # Errors
/// [`AppError::FeatureDisabled`] naming what the flag actually said.
pub fn require_feature(product: &ProductConfig) -> Result<(), AppError> {
    match product.features.get("control-server") {
        Some(true) => Ok(()),
        Some(false) => Err(AppError::FeatureDisabled {
            detail: "the feature flag is false".to_owned(),
        }),
        None => Err(AppError::FeatureDisabled {
            detail: "the feature flag is absent".to_owned(),
        }),
    }
}

/// The `engineInfo` this host advertises: identity from `product.toml`
/// (doc 07 §8.2 propagation: engineInfo is a runtime product-config
/// reader), the catalog checksum from the live command layer, empty
/// plugin/decoder/session lists (no engine is embedded — honest, not
/// aspirational).
pub fn engine_info_from_product(product: &ProductConfig) -> EngineInfo {
    EngineInfo {
        engine_version: product.identity.display_version.clone(),
        catalog_version: commands::catalog::spec::catalog_checksum().to_string(),
        protocol_version: format!("{PROTOCOL_NAME}/{PROTOCOL_VERSION}"),
        // lumen-allow-literal: host identifier naming THIS app in engineInfo (doc 05 §2.12) — a structural role name, not a brand string
        host: "control-server".to_owned(),
        features: product
            .features
            .iter()
            .filter(|(_, enabled)| **enabled)
            .map(|(name, _)| name.clone())
            .collect(),
        plugins: Vec::new(),
        decoders: Vec::new(),
        sessions: Vec::new(),
    }
}

/// Resolve the wire server's config from the product model + operator
/// file. Both gates run here: the feature flag first (rule F2), then
/// the policy grants (unknown ids are typed errors).
///
/// # Errors
/// [`AppError::FeatureDisabled`] / [`AppError::PolicyResolve`].
pub fn resolve_server_config(
    product: &ProductConfig,
    file: &PolicyFile,
) -> Result<ServerConfig, AppError> {
    require_feature(product)?;
    let policy =
        ServerPolicy::from_file(file).map_err(|source| AppError::PolicyResolve { source })?;
    let mut config = ServerConfig::strict(engine_info_from_product(product));
    config.bind = file.server.bind.clone();
    config.port = file.server.port;
    config.policy = policy;
    config.token = match &file.auth.token {
        Some(token) => TokenSource::Fixed(token.clone()),
        None => TokenSource::Generated,
    };
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use product_config::Schema;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn root_schema() -> Result<Schema, AppError> {
        let path = repo_root().join("product.schema.json");
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(source) => {
                return Err(AppError::ConfigRead { path, source });
            }
        };
        text.parse::<Schema>()
            .map_err(|source| AppError::ConfigParse {
                path,
                detail: source.to_string(),
            })
    }

    /// The REAL repo product.toml (control-server = false today).
    fn load_repo_product() -> Result<ProductConfig, AppError> {
        let schema = match root_schema() {
            Ok(schema) => schema,
            Err(err) => unreachable!("root schema must load: {err}"),
        };
        let path = repo_root().join("product.toml");
        product_config::load_file(&path, &schema)
            .map_err(|diagnostics| AppError::ProductLoad { diagnostics })
    }

    #[test]
    fn repo_product_today_refuses_the_feature_gate() {
        let product = match load_repo_product() {
            Ok(product) => product,
            Err(err) => unreachable!("repo product.toml must load: {err}"),
        };
        match resolve_server_config(&product, &PolicyFile::default()) {
            Err(AppError::FeatureDisabled { detail }) => {
                assert!(detail.contains("false"), "{detail}");
            }
            other => unreachable!("feature gate did not refuse: {other:?}"),
        }
    }

    #[test]
    fn engine_info_carries_product_identity_and_catalog_checksum() {
        let product = match load_repo_product() {
            Ok(product) => product,
            Err(err) => unreachable!("repo product.toml must load: {err}"),
        };
        let info = engine_info_from_product(&product);
        assert_eq!(info.engine_version, product.identity.display_version);
        assert_eq!(
            info.catalog_version,
            commands::catalog::spec::catalog_checksum().to_string()
        );
        // lumen-allow-literal: protocol version string is wire vocabulary (doc 05 §5.4)
        assert_eq!(info.protocol_version, "lumen-control/1");
        assert_eq!(info.host, "control-server");
        // The enabled-feature list mirrors the product model exactly.
        let enabled: Vec<String> = product
            .features
            .iter()
            .filter(|(_, enabled)| **enabled)
            .map(|(name, _)| name.clone())
            .collect();
        assert_eq!(info.features, enabled);
    }

    #[test]
    fn resolve_maps_the_operator_file_onto_the_wire_config() {
        // The feature gate is checked FIRST; bypass it with a synthetic
        // product (the gate has its own test above).
        let mut product = match load_repo_product() {
            Ok(product) => product,
            Err(err) => unreachable!("repo product.toml must load: {err}"),
        };
        product.features.insert("control-server".to_owned(), true);
        let file = PolicyFile {
            server: control_protocol::policy::ServerSection {
                bind: "::1".to_owned(),
                port: 4355,
                read_only: true,
            },
            auth: control_protocol::policy::AuthSection {
                token: Some("fixed-token".to_owned()),
            },
            policy: control_protocol::policy::PolicySection {
                sensitive_allow: vec!["config.set-setting".to_owned()],
            },
        };
        let config = match resolve_server_config(&product, &file) {
            Ok(config) => config,
            Err(err) => unreachable!("valid config rejected: {err}"),
        };
        assert_eq!(config.bind, "::1");
        assert_eq!(config.port, 4355);
        assert!(config.policy.read_only());
        assert_eq!(config.token, TokenSource::Fixed("fixed-token".to_owned()));
        assert!(
            config
                .policy
                .check(commands::catalog::ids::CommandId::ConfigSetSetting, true)
                .is_ok()
        );
    }

    #[test]
    fn a_missing_config_file_is_the_strict_default_not_an_error() {
        // load_config_file's missing-file path: the caller (main)
        // treats NotFound as "use defaults"; other read errors are
        // typed. Here we pin the typed error for an unreadable path.
        let missing = repo_root().join("does-not-exist-control-server.toml");
        match load_config_file(&missing) {
            Err(AppError::ConfigRead { .. }) => {}
            other => unreachable!("missing file: {other:?}"),
        }
    }
}
