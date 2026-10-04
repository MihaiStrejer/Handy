//! The only supported legacy dictionary boundary. Never runs a model.
use super::storage::{Profile, ProfileCatalog, ProfileIcon, RoutingRule};
use serde::Deserialize;
use std::io::Write;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Keyword {
    id: String,
    canonical: String,
    misheard_forms: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyProfile {
    id: String,
    name: String,
    revision: u32,
    dictionary_revision: u32,
    icon: ProfileIcon,
    rules: Vec<RoutingRule>,
    prompt: Option<String>,
    dictionary: Vec<Keyword>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyCatalog {
    schema_version: u32,
    revision: u32,
    next_id: u32,
    profiles: Vec<LegacyProfile>,
}

pub(super) fn placeholders(prompt: &str) -> String {
    let mut output = String::new();
    let mut rest = prompt;
    while let Some(start) = rest.find("{{") {
        let Some(relative_end) = rest[start + 2..].find("}}") else {
            break;
        };
        let end = start + 2 + relative_end;
        output.push_str(&rest[..start]);
        if rest[start + 2..end].trim() == "dictionary" {
            output.push_str("{{long_term_memory}}");
        } else {
            output.push_str(&rest[start..end + 2]);
        }
        rest = &rest[end + 2..];
    }
    output.push_str(rest);
    output
}

pub(super) fn convert(value: serde_json::Value) -> Result<ProfileCatalog, String> {
    let old: LegacyCatalog = serde_json::from_value(value)
        .map_err(|_| "Legacy profile store is invalid; existing data was retained")?;
    if old.schema_version != 1 || old.profiles.iter().filter(|p| p.id == "general").count() != 1 {
        return Err("Legacy profile store version or General profile is invalid".into());
    }
    let mut profiles = Vec::new();
    for p in old.profiles {
        let mut text = Vec::new();
        for k in p.dictionary {
            // JSON string literals preserve newlines, quotation marks, Unicode
            // and case without making the text ambiguous to a later model.
            let canonical = serde_json::to_string(&k.canonical).map_err(|_| "Migration failed")?;
            let aliases =
                serde_json::to_string(&k.misheard_forms).map_err(|_| "Migration failed")?;
            let _legacy_id = k.id; // Exact IDs remain in the raw backup.
            text.push(format!(
                "- Preferred term: {canonical}; misheard forms: {aliases}."
            ));
        }
        profiles.push(Profile {
            id: p.id,
            name: p.name,
            revision: p.revision,
            rewrite_context_revision: p.revision,
            long_term_memory_revision: p.dictionary_revision,
            icon: p.icon,
            rules: p.rules,
            prompt: p.prompt.map(|v| placeholders(&v)),
            long_term_memory: text.join("\n"),
            consolidation_instructions: None,
            long_term_undo: None,
            last_consolidation: None,
        });
    }
    let catalog = ProfileCatalog {
        schema_version: 3,
        revision: old.revision,
        next_id: old.next_id,
        profiles,
    };
    catalog.validate_identity()?;
    Ok(catalog)
}

/// Retain the exact file bytes before the first schema write. An existing
/// backup must decode to the same legacy catalog; it is never overwritten.
pub(super) fn backup(path: &Path) -> Result<(), String> {
    let raw = std::fs::read(path).map_err(|_| "Could not read legacy profile backup source")?;
    let backup = path.with_file_name("context-profiles.v1.backup.json");
    if backup.exists() {
        let previous =
            std::fs::read(&backup).map_err(|_| "Could not read legacy profile backup")?;
        let prior: serde_json::Value =
            serde_json::from_slice(&previous).map_err(|_| "Legacy backup is invalid")?;
        let current: serde_json::Value =
            serde_json::from_slice(&raw).map_err(|_| "Legacy source is invalid")?;
        if prior.get("profiles") != current.get("profiles") {
            return Err("Legacy backup differs; migration was not applied".into());
        }
        return Ok(());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup)
        .map_err(|_| "Could not create legacy profile backup")?;
    if file.write_all(&raw).and_then(|_| file.sync_all()).is_err() {
        drop(file);
        let _ = std::fs::remove_file(&backup);
        return Err("Could not save legacy profile backup".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn literals_and_oversized_legacy_data_survive_without_rewriting_prose() {
        let canonical = "Mihai \"BOM\"\n材料 😀";
        let aliases = vec!["BoM".to_owned(), "x".repeat(40_000)];
        let value = json!({"schema_version":1,"revision":7,"next_id":42,"profiles":[{
            "id":"general","name":"General","revision":6,"dictionary_revision":5,"icon":"generic",
            "rules":[],"prompt":"Keep dictionary prose {{ dictionary }} {{transcript}}",
            "dictionary":[{"id":"literal","canonical":canonical,"misheard_forms":aliases}]
        }]});
        let catalog = convert(value.clone()).unwrap();
        assert_eq!(catalog.schema_version, 3);
        assert_eq!(catalog.next_id, 42);
        assert!(catalog.profiles[0]
            .long_term_memory
            .contains(&serde_json::to_string(canonical).unwrap()));
        assert!(catalog.profiles[0]
            .long_term_memory
            .contains(&serde_json::to_string(&aliases).unwrap()));
        assert_eq!(
            catalog.profiles[0].prompt.as_deref(),
            Some("Keep dictionary prose {{long_term_memory}} {{transcript}}")
        );
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("context-profiles.json");
        let raw = format!("  {}\n", json!({"profiles":value}));
        std::fs::write(&path, &raw).unwrap();
        backup(&path).unwrap();
        backup(&path).unwrap();
        assert_eq!(
            std::fs::read(directory.path().join("context-profiles.v1.backup.json")).unwrap(),
            raw.as_bytes()
        );
        assert!(convert(json!({"schema_version":99})).is_err());
    }
}
