use super::session::{
    Captured, DictionaryEntry, InputContext, MatchBasis, MemoryItem, PromptSource, ResolvedContext,
};
use super::storage::{Profile, ProfileSnapshot};

fn windows_path(value: &str) -> String {
    value
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

fn specificity(profile: &Profile, context: &InputContext) -> u8 {
    let Captured::Present(application) = &context.application else {
        return 0;
    };
    profile
        .rules
        .iter()
        .filter_map(|rule| {
            if !rule.application.eq_ignore_ascii_case(application) {
                return None;
            }
            match &rule.workspace {
                Some(directory) => match &context.workspace {
                    Captured::Present(workspace)
                        if windows_path(directory) == windows_path(workspace) =>
                    {
                        Some(2)
                    }
                    _ => None,
                },
                None => Some(1),
            }
        })
        .max()
        .unwrap_or(0)
}

/// Only the capture adapter can supply workspace evidence. Window titles and
/// neighboring processes are deliberately absent from the matcher contract.
pub(super) fn resolve(
    snapshot: &ProfileSnapshot,
    input: InputContext,
) -> Result<ResolvedContext, &'static str> {
    let general = snapshot
        .catalog
        .profiles
        .iter()
        .find(|p| p.id == "general")
        .ok_or("General profile is missing")?;
    let matches: Vec<_> = snapshot
        .catalog
        .profiles
        .iter()
        .filter(|p| p.id != "general")
        .map(|p| (p, specificity(p, &input)))
        .filter(|(_, score)| *score > 0)
        .collect();
    let best = matches.iter().map(|(_, score)| *score).max().unwrap_or(0);
    let winners: Vec<_> = matches.iter().filter(|(_, score)| *score == best).collect();
    // An ambiguous rule never silently chooses a different profile by list
    // order. General is deterministic and its match basis records the reason.
    let (profile, match_basis) = match winners.as_slice() {
        [(profile, 2)] => (*profile, MatchBasis::Workspace),
        [(profile, 1)] => (*profile, MatchBasis::Application),
        [] => (general, MatchBasis::General),
        _ => (general, MatchBasis::Ambiguous),
    };
    let (prompt_owner, prompt_source) = if profile.id == "general" {
        (general, PromptSource::General)
    } else if profile.prompt.is_some() {
        (profile, PromptSource::Custom)
    } else {
        (general, PromptSource::Inherited)
    };
    let prompt = prompt_owner
        .prompt
        .clone()
        .ok_or("General prompt is missing")?;
    super::template::validate(&prompt).map_err(|_| "Effective profile prompt is invalid")?;
    Ok(ResolvedContext {
        profile_id: profile.id.clone(),
        profile_name: profile.name.clone(),
        profile_revision: profile.revision.into(),
        icon: profile.icon.display_value(),
        match_basis,
        prompt,
        prompt_source,
        prompt_revision: prompt_owner.revision.into(),
        dictionary_revision: profile.dictionary_revision.into(),
        dictionary: profile
            .dictionary
            .iter()
            .map(|k| DictionaryEntry {
                id: k.id.clone(),
                canonical: k.canonical.clone(),
                misheard_forms: k.misheard_forms.clone(),
            })
            .collect(),
        memory_epoch: snapshot
            .memory_epochs
            .get(&profile.id)
            .copied()
            .unwrap_or_default(),
        memory: snapshot
            .memory
            .get(&profile.id)
            .into_iter()
            .flatten()
            .map(|m| MemoryItem {
                id: m.id.clone(),
                text: m.text.clone(),
                source_session: m.source_session.clone(),
            })
            .collect(),
        input,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_profiles::storage::{ProfileCatalog, ProfileIcon, RoutingRule};
    use std::{collections::HashMap, sync::Arc};

    fn fixtures() -> (ProfileSnapshot, InputContext) {
        let mut catalog = ProfileCatalog::seed(&crate::settings::get_default_settings());
        catalog.profiles[0].prompt = Some("General {{transcript}}".into());
        for (id, path) in [
            ("terminal", None),
            ("handy", Some("D:\\rust\\Handy")),
            ("other", Some("D:\\other")),
        ] {
            catalog.profiles.push(Profile {
                id: id.into(),
                name: id.into(),
                revision: 1,
                dictionary_revision: 1,
                icon: ProfileIcon::Terminal,
                rules: vec![RoutingRule {
                    application: "WindowsTerminal.exe".into(),
                    workspace: path.map(str::to_owned),
                }],
                prompt: None,
                dictionary: vec![],
            });
        }
        (
            ProfileSnapshot {
                catalog: Arc::new(catalog),
                memory: HashMap::new(),
                memory_epochs: HashMap::new(),
            },
            InputContext {
                application: Captured::Present("WindowsTerminal.exe".into()),
                workspace: Captured::Unavailable,
                selection: Captured::Unavailable,
                surrounding_text: Captured::Unavailable,
                caret_utf16: Captured::Unavailable,
                selection_range_utf16: None,
                captured_at_ms: 0,
                truncated: false,
            },
        )
    }

    #[test]
    fn workspace_then_application_then_general_and_no_path_prefix_guess() {
        let (snapshot, mut input) = fixtures();
        assert_eq!(
            resolve(&snapshot, input.clone()).unwrap().profile_id,
            "terminal"
        );
        input.workspace = Captured::Present("d:/RUST/handy/".into());
        assert_eq!(
            resolve(&snapshot, input.clone()).unwrap().profile_id,
            "handy"
        );
        input.workspace = Captured::Present("D:/other".into());
        assert_eq!(
            resolve(&snapshot, input.clone()).unwrap().profile_id,
            "other"
        );
        input.workspace = Captured::Present("D:/rust/Handy-unrelated".into());
        assert_eq!(
            resolve(&snapshot, input.clone()).unwrap().profile_id,
            "terminal"
        );
        input.application = Captured::Unavailable;
        assert_eq!(resolve(&snapshot, input).unwrap().profile_id, "general");
    }

    #[test]
    fn equally_specific_rules_use_general_regardless_of_order() {
        let (mut snapshot, input) = fixtures();
        let catalog = Arc::make_mut(&mut snapshot.catalog);
        let mut duplicate = catalog.profiles[1].clone();
        duplicate.id = "duplicate".into();
        catalog.profiles.push(duplicate);
        assert!(matches!(
            resolve(&snapshot, input.clone()).unwrap().match_basis,
            MatchBasis::Ambiguous
        ));
        Arc::make_mut(&mut snapshot.catalog).profiles.reverse();
        assert_eq!(resolve(&snapshot, input).unwrap().profile_id, "general");
    }

    #[test]
    fn snapshots_keep_inherited_and_custom_prompts_after_edits_or_deletion() {
        let (snapshot, input) = fixtures();
        let frozen = resolve(&snapshot, input.clone()).unwrap();
        let mut changed = ProfileSnapshot {
            catalog: Arc::clone(&snapshot.catalog),
            memory: HashMap::new(),
            memory_epochs: HashMap::new(),
        };
        Arc::make_mut(&mut changed.catalog).profiles[0].prompt = Some("New General".into());
        assert_eq!(
            resolve(&changed, input.clone()).unwrap().prompt,
            "New General"
        );
        assert_eq!(frozen.prompt, "General {{transcript}}");
        Arc::make_mut(&mut changed.catalog).profiles[1].prompt = Some("Custom".into());
        assert!(matches!(
            resolve(&changed, input.clone()).unwrap().prompt_source,
            PromptSource::Custom
        ));
        Arc::make_mut(&mut changed.catalog).profiles.remove(1);
        assert_eq!(
            resolve(&changed, input.clone()).unwrap().profile_id,
            "general"
        );
        assert_eq!(resolve(&snapshot, input).unwrap().profile_id, "terminal");
    }
}
