//! Card 1.D4 — catalog shape tests: the committed schema's structure,
//! the id grammar, the enum ↔ spec-table coverage and the checksum.
//!
//! These pin the *shape* invariants doc 05 §2–§3 define: 61 command
//! entries (+ the issue #32 bookmark trio = 64) / 23 event entries (the
//! §2.1 / §3.2 tables), ids of the form
//! `<area>.<verb-noun>`, no orphan and no missing rows between the
//! `CommandId`/`EventId` enums, the payload enums' wire tags and the
//! `catalog_data()` metadata table — and that the committed checksum
//! recomputes from the same source.

use std::error::Error;
use std::path::Path;

use commands::CommandId;
use commands::EventId;
use commands::catalog::codegen::default_paths;
use commands::catalog::spec::catalog_checksum;
use commands::catalog::spec::catalog_data;
use commands::catalog::spec::event_catalog_data;
use serde_json::Value;

/// The Lumen workspace root (this crate lives in `<root>/crates/commands`).
fn workspace_root() -> &'static Path {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().and_then(Path::parent).unwrap_or(manifest)
}

fn committed_schema() -> Result<Value, Box<dyn Error>> {
    let path = default_paths(workspace_root()).schema;
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("committed schema unreadable at {}: {err}", path.display()))?;
    Ok(serde_json::from_str(&text)?)
}

/// The `cmd` wire tags the schema's request section carries (one per
/// `Command` variant — the payload enum's own surface).
fn schema_command_tags(schema: &Value) -> Vec<String> {
    schema["$defs"]["CommandRequest"]["oneOf"]
        .as_array()
        .map(|variants| {
            variants
                .iter()
                .filter_map(|variant| {
                    variant["properties"]["cmd"]["const"]
                        .as_str()
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The `kind` wire tags the schema's event section carries (one per
/// `EventKind` variant).
fn schema_event_tags(schema: &Value) -> Vec<String> {
    schema["$defs"]["Event"]["oneOf"]
        .as_array()
        .map(|variants| {
            variants
                .iter()
                .filter_map(|variant| {
                    variant["properties"]["kind"]["const"]
                        .as_str()
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The committed schema is valid JSON with the §5.1 document shape and
/// the doc 05 table counts: 61 command entries (64 with the issue #32
/// bookmark trio) + 23 event entries.
#[test]
fn schema_has_the_doc_05_structure_and_counts() -> Result<(), Box<dyn Error>> {
    let schema = committed_schema()?;
    assert!(
        schema["$schema"]
            .as_str()
            .is_some_and(|s| s.contains("2020-12")),
        "schema must declare a JSON Schema draft 2020-12 dialect"
    );
    assert_eq!(schema["title"].as_str(), Some("CatalogDocument"));
    assert_eq!(schema["type"].as_str(), Some("object"));
    for section in ["request", "response", "event"] {
        assert!(
            schema["properties"][section].is_object(),
            "schema root must carry the {section} envelope"
        );
    }
    // Doc 05 §2.1 lists 61 command entries; issue #32 adds the three
    // media.*-bookmark commands (64); §3.2 lists 23 events.
    assert_eq!(schema_command_tags(&schema).len(), 64, "§2.1 count (+3 bookmarks, issue #32)");
    assert_eq!(schema_event_tags(&schema).len(), 23, "§3.2 count");
    // The §2.3–§2.12 tables classify 19 commands A and 42 S; issue #32's
    // bookmark trio is class S (64 total).
    let a = catalog_data()
        .iter()
        .filter(|spec| spec.class.as_str() == "a")
        .count();
    let s = catalog_data()
        .iter()
        .filter(|spec| spec.class.as_str() == "s")
        .count();
    assert_eq!((a, s), (19, 45), "§1.5 classification counts (+3 S bookmarks, issue #32)");
    // §3.2's "gen" column: exactly the 13 pipeline-scoped events.
    let scoped = event_catalog_data()
        .iter()
        .filter(|spec| spec.pipeline_scoped)
        .count();
    assert_eq!(scoped, 13, "§3.2 pipeline-scoped count");
    Ok(())
}

/// Every `CommandId`/`EventId` round-trips through its stable wire
/// string, and every id follows the §2.1 grammar `<area>.<verb-noun>`
/// (an area token, a dot, one or more kebab-case tokens — all
/// lowercase ASCII alphanumerics).
#[test]
fn every_id_parses_and_follows_the_grammar() -> Result<(), Box<dyn Error>> {
    fn check_grammar(id: &str) {
        match id.split_once('.') {
            Some((area, rest)) => {
                assert!(!area.is_empty(), "empty area in {id}");
                for token in std::iter::once(area).chain(rest.split('-')) {
                    assert!(
                        !token.is_empty()
                            && token
                                .chars()
                                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                        "token {token:?} of {id} is not kebab-case"
                    );
                }
            }
            // No separator at all: the doc-grammar assertion below cannot
            // hold, so it fails with the actionable message.
            None => assert!(
                id.contains('.'),
                "id {id} does not follow `<area>.<verb-noun>` (no area separator)"
            ),
        }
    }

    for (variant, id) in CommandId::ALL.iter().zip(CommandId::ALL_IDS) {
        assert_eq!(variant.as_str(), *id);
        assert_eq!(
            (*id).parse::<CommandId>().ok(),
            Some(*variant),
            "{id} parse"
        );
        check_grammar(id);
    }
    for (variant, id) in EventId::ALL.iter().zip(EventId::ALL_IDS) {
        assert_eq!(variant.as_str(), *id);
        assert_eq!((*id).parse::<EventId>().ok(), Some(*variant), "{id} parse");
        check_grammar(id);
    }
    assert_eq!(CommandId::ALL.len(), 64, "§2.1: 61 commands + issue #32 bookmark trio");
    assert_eq!(EventId::ALL.len(), 23, "§3.2: 23 events");
    Ok(())
}

/// The spec table covers EXACTLY the payload enums' variants — the
/// schema's wire tags (the `Command`/`EventKind` surface), the id enums
/// and the `catalog_data()` rows are the same sets, with no orphan row
/// and no missing row on any side.
#[test]
fn spec_table_covers_exactly_the_enum_variants() -> Result<(), Box<dyn Error>> {
    let schema = committed_schema()?;
    let schema_command_tags = schema_command_tags(&schema);
    let schema_event_tags = schema_event_tags(&schema);
    let mut schema_commands: Vec<&str> = schema_command_tags.iter().map(String::as_str).collect();
    let mut schema_events: Vec<&str> = schema_event_tags.iter().map(String::as_str).collect();
    let mut id_commands: Vec<&str> = CommandId::ALL_IDS.to_vec();
    let mut id_events: Vec<&str> = EventId::ALL_IDS.to_vec();
    let mut spec_commands: Vec<&str> = catalog_data().iter().map(|s| s.id.as_str()).collect();
    let mut spec_events: Vec<&str> = event_catalog_data().iter().map(|s| s.id.as_str()).collect();
    schema_commands.sort_unstable();
    schema_events.sort_unstable();
    id_commands.sort_unstable();
    id_events.sort_unstable();
    spec_commands.sort_unstable();
    spec_events.sort_unstable();
    assert_eq!(
        schema_commands, id_commands,
        "Command enum variants and CommandId ids disagree"
    );
    assert_eq!(
        id_commands, spec_commands,
        "CommandId ids and catalog_data() rows disagree (orphan or missing row)"
    );
    assert_eq!(
        schema_events, id_events,
        "EventKind variants and EventId ids disagree"
    );
    assert_eq!(
        id_events, spec_events,
        "EventId ids and event_catalog_data() rows disagree (orphan or missing row)"
    );
    Ok(())
}

/// The committed checksum file recomputes from the live catalog: the
/// `catalog-fnv1a64` line matches `catalog_checksum()` and the counts
/// line matches both tables.
#[test]
fn committed_checksum_recomputes() -> Result<(), Box<dyn Error>> {
    let path = default_paths(workspace_root()).checksum;
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("committed checksum unreadable at {}: {err}", path.display()))?;
    let expected_catalog = format!("catalog-fnv1a64: {:016x}", catalog_checksum());
    assert!(
        text.lines().any(|line| line == expected_catalog),
        "committed checksum does not carry the recomputed catalog digest \
         {expected_catalog}:\n{text}"
    );
    assert!(
        text.contains("counts: 64 commands / 23 events"),
        "committed checksum counts line drifted:\n{text}"
    );
    Ok(())
}
