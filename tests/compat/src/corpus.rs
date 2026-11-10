//! Corpus assembly: the committed fixture roots (walked) plus the
//! regenerable manifest-anchored set (the corpus-big/D-11 pattern —
//! the manifest is committed, the bytes regenerate on demand from
//! recorded ffmpeg argvs, sha256-verified for deterministic rows).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ToolError;
use crate::config::Config;
use crate::proc;
use crate::sha256;

/// The committed regen manifest (schema `compat-matrix/corpus-manifest/1`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegenManifest {
    /// Schema discriminator.
    pub schema: String,
    /// Provenance prose (cited in the report).
    pub provenance: String,
    /// The entries.
    pub entries: Vec<RegenEntry>,
}

/// One regenerable file.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegenEntry {
    /// Path inside the cache dir.
    pub rel: String,
    /// ffmpeg argvs WITHOUT the binary (element 0 of each argv is
    /// the first ffmpeg argument; `__OUT__` = the output file).
    pub argvs: Vec<Vec<String>>,
    /// The committed sha256 for deterministic inputs; `null` for
    /// runtime-random sources (fresh realization each regen).
    pub sha256: Option<String>,
    /// Whether regeneration reproduces the recorded bytes.
    pub deterministic: bool,
    /// Where the entry's argvs come from (committed record citation).
    pub source: String,
    /// Why the row exists (report prose).
    pub purpose: String,
    /// VLC `afile` format for oracle runs (`u8`/`s16`/`float32`);
    /// `null` = not oracle-comparable.
    pub audiofile_format: Option<String>,
    /// The committed oracle record's key for this row
    /// (`media/0022-avi-av.avi` — the record's rel namespace), when
    /// the row is one of the lossless-comparable family.
    pub oracle_rel: Option<String>,
    /// Whether the runner retains the decoded PCM for comparison.
    pub dump_pcm: bool,
}

/// One corpus file to run, with its class.
#[derive(Debug, Clone)]
pub struct CorpusFile {
    /// Corpus-relative id (stable across runs).
    pub rel: String,
    /// Absolute path.
    pub path: PathBuf,
    /// The corpus family it came from.
    pub class: CorpusClass,
    /// Retain the decoded audio payload (oracle rows).
    pub dump_pcm: bool,
}

/// Where a corpus file came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusClass {
    /// A committed fixture under a config-listed root.
    Committed,
    /// A committed malformed/error-contract derivative.
    Malformed,
    /// A regenerated file from the manifest-anchored cache.
    Regen,
}

/// One regen step's outcome (for the run metadata).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum RegenOutcome {
    /// Already present + sha256 verified (deterministic row).
    PresentVerified,
    /// Already present (random row; the realization's sha256 recorded).
    PresentRandom { sha256: String },
    /// Regenerated now; sha256 matches the manifest.
    RegeneratedVerified,
    /// Regenerated now (random source; fresh realization).
    RegeneratedRandom { sha256: String },
    /// Regeneration failed (the ffmpeg step's outcome is the record).
    Failed { error: String },
}

/// Walk the committed roots (sorted for determinism: BTreeMap by rel).
pub fn collect_committed(config: &Config) -> Result<Vec<CorpusFile>, ToolError> {
    let mut files: BTreeMap<String, CorpusFile> = BTreeMap::new();
    for root_rel in &config.corpus.committed_roots {
        let root = config.rel(root_rel);
        walk_root(config, &root, CorpusClass::Committed, &mut files)?;
    }
    let malformed_root = config.rel(&config.corpus.malformed_root);
    walk_root(config, &malformed_root, CorpusClass::Malformed, &mut files)?;
    Ok(files.into_values().collect())
}

/// Walk one root recursively, collecting media files.
fn walk_root(
    config: &Config,
    root: &Path,
    class: CorpusClass,
    files: &mut BTreeMap<String, CorpusFile>,
) -> Result<(), ToolError> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(err) => {
            return Err(ToolError::io(
                format!("reading corpus root {}", root.display()),
                err,
            ));
        }
    };
    for entry in entries {
        let entry = entry.map_err(|e| ToolError::io(format!("iterating {}", root.display()), e))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| ToolError::io(format!("stating {}", path.display()), e))?;
        if file_type.is_dir() {
            // Recurse (bounded by the tree's own depth).
            walk_root(config, &path, class, files)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if config
            .corpus
            .exclude_suffixes
            .iter()
            .any(|suffix| name.ends_with(suffix.as_str()))
        {
            continue;
        }
        let is_media = config.corpus.media_extensions.iter().any(|ext| {
            name.to_ascii_lowercase()
                .ends_with(&ext.to_ascii_lowercase())
        });
        if !is_media {
            continue;
        }
        // The corpus-relative id: the workspace-relative path (stable,
        // unique across subdirectories — hls/ts_vod/pl0.ts vs
        // hls/adts_vod/pl0.aac must not collide).
        let rel = path
            .strip_prefix(&config.root)
            .map(|rel| rel.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));
        files.insert(
            rel.clone(),
            CorpusFile {
                rel,
                path,
                class,
                dump_pcm: false,
            },
        );
    }
    Ok(())
}

/// Read + schema-check the regen manifest (shared by the corpus
/// assembly and the report's comparable-row lookups).
pub fn read_manifest(config: &Config) -> Result<RegenManifest, ToolError> {
    let manifest_path = config.rel(&config.corpus.regen.manifest);
    let text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| ToolError::io(format!("reading {}", manifest_path.display()), e))?;
    let manifest: RegenManifest = serde_json::from_str(&text)
        .map_err(|e| ToolError::config(format!("parsing {}: {e}", manifest_path.display())))?;
    if manifest.schema != "compat-matrix/corpus-manifest/1" {
        return Err(ToolError::config(format!(
            "unsupported manifest schema {:?}",
            manifest.schema
        )));
    }
    Ok(manifest)
}

/// Per-entry regeneration outcomes (rel → outcome), in manifest order.
pub type RegenOutcomes = Vec<(String, RegenOutcome)>;

/// Ensure the regen corpus exists (regenerate missing files), answer
/// the ready corpus files + per-entry outcomes.
pub fn ensure_regen_corpus(config: &Config) -> Result<(Vec<CorpusFile>, RegenOutcomes), ToolError> {
    let manifest = read_manifest(config)?;
    let cache = config.rel(&config.corpus.regen.cache_dir);
    std::fs::create_dir_all(&cache)
        .map_err(|e| ToolError::io(format!("creating {}", cache.display()), e))?;

    let mut files = Vec::new();
    let mut outcomes = Vec::new();
    for entry in &manifest.entries {
        let target = cache.join(&entry.rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::io(format!("creating {}", parent.display()), e))?;
        }
        let outcome = match std::fs::read(&target) {
            Ok(bytes) => {
                let fresh = sha256::hex_digest(&bytes);
                match (&entry.sha256, entry.deterministic) {
                    // A deterministic row whose bytes match the pin.
                    (Some(expected), true) if &fresh == expected => RegenOutcome::PresentVerified,
                    // A deterministic row whose bytes drifted: never
                    // use them silently — regenerate and re-verify.
                    (expected, true) => match regenerate(config, entry, &target)? {
                        Some(bytes) => match verified(&bytes, expected.as_deref()) {
                            Ok(()) => RegenOutcome::RegeneratedVerified,
                            Err(error) => RegenOutcome::Failed { error },
                        },
                        None => regen_failed(entry),
                    },
                    // A random row already on disk: a prior fresh
                    // realization — a real file; its sha256 recorded.
                    _ => RegenOutcome::PresentRandom { sha256: fresh },
                }
            }
            Err(_) => match regenerate(config, entry, &target)? {
                Some(bytes) => {
                    let fresh = sha256::hex_digest(&bytes);
                    if entry.deterministic {
                        match verified(&bytes, entry.sha256.as_deref()) {
                            Ok(()) => RegenOutcome::RegeneratedVerified,
                            Err(error) => RegenOutcome::Failed { error },
                        }
                    } else {
                        RegenOutcome::RegeneratedRandom { sha256: fresh }
                    }
                }
                None => regen_failed(entry),
            },
        };
        let failed = matches!(outcome, RegenOutcome::Failed { .. });
        outcomes.push((entry.rel.clone(), outcome));
        if failed {
            // A failed regeneration is a recorded fact; the row is
            // NOT offered to the decode sweep (its bytes are absent
            // or unverified — the report counts it honestly).
            continue;
        }
        files.push(CorpusFile {
            rel: format!("{}/{}", config.corpus.regen.cache_dir, entry.rel),
            path: target,
            class: CorpusClass::Regen,
            dump_pcm: entry.dump_pcm,
        });
    }
    Ok((files, outcomes))
}

/// The typed failure record for a regeneration whose ffmpeg step
/// itself failed (nonzero exit / timeout — the step's outcome is the
/// honest record; stderr head is folded in by the caller's budget).
fn regen_failed(entry: &RegenEntry) -> RegenOutcome {
    RegenOutcome::Failed {
        error: format!(
            "ffmpeg step failed for {} (argvs: {} recorded in the manifest; \
             the run's raw-rows file carries the step record)",
            entry.rel,
            entry.argvs.len()
        ),
    }
}

/// Verify regenerated bytes against the manifest pin.
fn verified(bytes: &[u8], expected: Option<&str>) -> Result<(), String> {
    let fresh = sha256::hex_digest(bytes);
    match expected {
        Some(expected) if expected == fresh => Ok(()),
        Some(expected) => Err(format!(
            "regenerated sha256 {fresh} != manifest {expected} — the ffmpeg \
             or source changed; re-pin the manifest"
        )),
        None => Err("deterministic row carries no manifest sha256 to verify against".to_string()),
    }
}

/// Regenerate one entry (run its argvs in order). `None` = a step
/// failed (the caller records the typed failure).
fn regenerate(
    config: &Config,
    entry: &RegenEntry,
    target: &Path,
) -> Result<Option<Vec<u8>>, ToolError> {
    let ffmpeg = &config.corpus.regen.ffmpeg;
    let target_text = target
        .to_str()
        .ok_or_else(|| ToolError::config(format!("cache path is not UTF-8: {}", target.display())))?
        .to_string();
    for argv in &entry.argvs {
        let mut full = vec![ffmpeg.clone()];
        for arg in argv {
            full.push(if arg == "__OUT__" {
                target_text.clone()
            } else {
                arg.clone()
            });
        }
        let record = proc::run_bounded(&full, None, &[], config.budgets.regen_s)?;
        if !record.outcome.starts_with("exit 0") {
            return Ok(None);
        }
    }
    match std::fs::read(target) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) => Err(ToolError::io(
            format!("reading regenerated {}", target.display()),
            err,
        )),
    }
}
