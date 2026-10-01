use super::template;
use crate::settings::{self, AppSettings};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

// Profile data has a separate owner/store. Existing whole-settings writes and
// debug dumps cannot overwrite or disclose captured dictionary/memory content.
pub(crate) static PROFILE_WRITES: Mutex<()> = Mutex::new(());
const STORE_PATH: &str = "context-profiles.json";
const GENERAL: &str = "general";

#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct Keyword {
    pub id: String,
    pub canonical: String,
    pub misheard_forms: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct RoutingRule {
    /// Exact executable basename; normalized for Windows matching later.
    pub application: String,
    pub workspace: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProfileIcon {
    Generic,
    Terminal,
    Code,
    Mail,
    Chrome,
    Firefox,
    Vscode,
    Intellij,
    Claude,
    Codex,
    Pi,
    Opencode,
    Github,
    Slack,
    Notes,
    Custom(String),
}

impl ProfileIcon {
    pub(super) fn display_value(&self) -> String {
        match self {
            Self::Generic => "generic",
            Self::Terminal => "terminal",
            Self::Code => "code",
            Self::Mail => "mail",
            Self::Chrome => "chrome",
            Self::Firefox => "firefox",
            Self::Vscode => "vscode",
            Self::Intellij => "intellij",
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Pi => "pi",
            Self::Opencode => "opencode",
            Self::Github => "github",
            Self::Slack => "slack",
            Self::Notes => "notes",
            Self::Custom(data) => return data.clone(),
        }
        .into()
    }
}

#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct Profile {
    pub id: String,
    pub name: String,
    pub revision: u32,
    pub dictionary_revision: u32,
    pub icon: ProfileIcon,
    pub rules: Vec<RoutingRule>,
    /// General must own a template. None on other profiles inherits General.
    pub prompt: Option<String>,
    pub dictionary: Vec<Keyword>,
}

#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileCatalog {
    pub schema_version: u32,
    pub revision: u32,
    next_id: u32,
    pub profiles: Vec<Profile>,
}

fn normalized(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn bounded(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.chars().count() <= limit
}

impl ProfileCatalog {
    pub(super) fn seed(settings: &AppSettings) -> Self {
        let prompt = settings
            .post_process_selected_prompt_id
            .as_ref()
            .and_then(|id| settings.post_process_prompts.iter().find(|p| &p.id == id))
            .map(|p| p.prompt.replace("${output}", "").trim().to_owned())
            .filter(|prompt| template::validate(prompt).is_ok())
            .unwrap_or_else(|| "Rewrite {{transcript}} accurately using {{dictionary}} and {{short_term_memory}}. Use {{input_context}} when available.".into());
        Self {
            schema_version: 1,
            revision: 0,
            next_id: 1,
            profiles: vec![Profile {
                id: GENERAL.into(),
                name: "General".into(),
                revision: 0,
                dictionary_revision: 0,
                icon: ProfileIcon::Generic,
                rules: vec![],
                prompt: Some(prompt),
                dictionary: vec![],
            }],
        }
    }

    fn check_revision(&self, expected: u32) -> Result<(), String> {
        if expected != self.revision {
            return Err("Profiles changed; reload before saving".into());
        }
        Ok(())
    }

    fn save_profile(&mut self, mut profile: Profile, expected: u32) -> Result<(), String> {
        self.check_revision(expected)?;
        if let ProfileIcon::Custom(data) = &profile.icon {
            super::icons::validate_custom(data)?;
        }
        if !bounded(&profile.name, 80) {
            return Err("Profile name must contain 1 to 80 characters".into());
        }
        if profile.rules.len() > 16 || profile.dictionary.len() > 500 {
            return Err("Profile exceeds its rule or dictionary limit".into());
        }
        if profile.id == GENERAL {
            if !profile.rules.is_empty() {
                return Err("General cannot have routing rules".into());
            }
            if profile.prompt.is_none() {
                return Err("General must define the default prompt".into());
            }
        } else if profile.rules.is_empty() {
            return Err("A profile requires an application rule".into());
        }
        if let Some(prompt) = &profile.prompt {
            template::validate(prompt)?;
        }
        for rule in &profile.rules {
            if !bounded(&rule.application, 260) || rule.application.contains(['/', '\\']) {
                return Err("Application must be an executable name".into());
            }
            if rule.workspace.as_ref().is_some_and(|w| !bounded(w, 1024)) {
                return Err("Invalid workspace directory".into());
            }
        }
        let mut ids = HashSet::new();
        let mut phrases = HashMap::new();
        for keyword in &profile.dictionary {
            if !bounded(&keyword.id, 80)
                || !ids.insert(&keyword.id)
                || !bounded(&keyword.canonical, 120)
                || keyword.misheard_forms.len() > 50
            {
                return Err("Invalid or duplicate keyword, or too many misheard forms".into());
            }
            for phrase in std::iter::once(&keyword.canonical).chain(&keyword.misheard_forms) {
                if !bounded(phrase, 120)
                    || phrases.insert(normalized(phrase), &keyword.id).is_some()
                {
                    return Err("A keyword or misheard form is empty, duplicated, or assigned more than once".into());
                }
            }
        }
        if self
            .profiles
            .iter()
            .any(|p| p.id != profile.id && normalized(&p.name) == normalized(&profile.name))
        {
            return Err("Profile name is already used".into());
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or("Profile revision exhausted")?;
        profile.name = profile.name.trim().into();
        profile.revision = next_revision;
        profile.dictionary_revision = next_revision;
        if profile.id.is_empty() {
            if self.profiles.len() >= 64 {
                return Err("At most 64 profiles are supported".into());
            }
            let next_id = self
                .next_id
                .checked_add(1)
                .ok_or("Profile identity exhausted")?;
            profile.id = format!("profile-{}", self.next_id);
            self.next_id = next_id;
            self.profiles.push(profile);
        } else {
            let existing = self
                .profiles
                .iter_mut()
                .find(|p| p.id == profile.id)
                .ok_or("Profile no longer exists")?;
            // Clients cannot choose revisions; all changes advance the catalog.
            *existing = profile;
        }
        self.revision = next_revision;
        Ok(())
    }

    fn delete_profile(&mut self, id: &str, expected: u32) -> Result<(), String> {
        self.check_revision(expected)?;
        if id == GENERAL {
            return Err("General cannot be deleted".into());
        }
        let index = self
            .profiles
            .iter()
            .position(|p| p.id == id)
            .ok_or("Profile no longer exists")?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or("Profile revision exhausted")?;
        self.profiles.remove(index);
        self.revision = revision;
        Ok(())
    }
}

fn load(app: &AppHandle) -> Result<ProfileCatalog, String> {
    let store = app
        .store(crate::portable::store_path(STORE_PATH))
        .map_err(|_| "Could not open profile store")?;
    if let Some(value) = store.get("profiles") {
        let catalog: ProfileCatalog = serde_json::from_value(value)
            .map_err(|_| "Profile store is invalid; existing data was retained")?;
        if catalog.schema_version != 1
            || catalog.profiles.iter().filter(|p| p.id == GENERAL).count() != 1
        {
            return Err("Profile store version or General profile is invalid".into());
        }
        app.state::<ProfileCache>()
            .0
            .lock()
            .map_err(|_| "Profile cache lock poisoned")?
            .replace(Arc::new(catalog.clone()));
        Ok(catalog)
    } else {
        let catalog = ProfileCatalog::seed(&settings::get_settings(app));
        persist(app, &catalog)?;
        Ok(catalog)
    }
}

fn persist(app: &AppHandle, catalog: &ProfileCatalog) -> Result<(), String> {
    let store = app
        .store(crate::portable::store_path(STORE_PATH))
        .map_err(|_| "Could not open profile store")?;
    let previous = store.get("profiles");
    let value = serde_json::to_value(catalog).map_err(|_| "Could not serialize profiles")?;
    store.set("profiles", value);
    if store.save().is_err() {
        if let Some(previous) = previous {
            store.set("profiles", previous);
        } else {
            store.delete("profiles");
        }
        return Err("Could not save profiles; changes were not applied".into());
    }
    app.state::<ProfileCache>()
        .0
        .lock()
        .map_err(|_| "Profile cache lock poisoned")?
        .replace(Arc::new(catalog.clone()));
    Ok(())
}

#[derive(Default)]
pub(crate) struct ProfileCache(Mutex<Option<Arc<ProfileCatalog>>>);

pub(super) struct ProfileSnapshot {
    pub catalog: Arc<ProfileCatalog>,
    pub memory: HashMap<String, Vec<MemoryRecord>>,
    pub memory_epochs: HashMap<String, u64>,
}

/// Invocation takes an immutable catalog reference. Persistence and UIA are
/// never performed on this path; edits publish a new reference after saving.
pub(super) fn snapshot(app: &AppHandle) -> Result<ProfileSnapshot, &'static str> {
    let catalog = app
        .state::<ProfileCache>()
        .0
        .lock()
        .map_err(|_| "Profile cache lock poisoned")?
        .clone()
        .ok_or("Profile catalog is not loaded")?;
    let memory = app
        .state::<ProfileMemory>()
        .0
        .lock()
        .map_err(|_| "Profile memory lock poisoned")?
        .clone();
    Ok(ProfileSnapshot {
        catalog,
        memory: memory.items,
        memory_epochs: memory.epochs,
    })
}

#[tauri::command]
#[specta::specta]
pub(crate) fn get_context_profiles(app: AppHandle) -> Result<ProfileCatalog, String> {
    let _lock = PROFILE_WRITES
        .lock()
        .map_err(|_| "Profile store lock poisoned")?;
    load(&app)
}

#[tauri::command]
#[specta::specta]
pub(crate) fn save_context_profile(
    app: AppHandle,
    profile: Profile,
    expected_revision: u32,
) -> Result<ProfileCatalog, String> {
    let _lock = PROFILE_WRITES
        .lock()
        .map_err(|_| "Profile store lock poisoned")?;
    let mut catalog = load(&app)?;
    catalog.save_profile(profile, expected_revision)?;
    persist(&app, &catalog)?;
    Ok(catalog)
}

#[tauri::command]
#[specta::specta]
pub(crate) fn delete_context_profile(
    app: AppHandle,
    id: String,
    expected_revision: u32,
) -> Result<ProfileCatalog, String> {
    let _lock = PROFILE_WRITES
        .lock()
        .map_err(|_| "Profile store lock poisoned")?;
    let mut catalog = load(&app)?;
    catalog.delete_profile(&id, expected_revision)?;
    persist(&app, &catalog)?;
    app.state::<ProfileMemory>()
        .0
        .lock()
        .map_err(|_| "Profile memory lock poisoned")?
        .remove(&id, None);
    Ok(catalog)
}

#[derive(Clone, Serialize, Deserialize, Type)]
pub(crate) struct MemoryRecord {
    pub id: String,
    pub text: String,
    pub source_session: String,
}

/// Process-local memory. No raw field contents are persisted as memory.
#[derive(Default)]
pub(crate) struct ProfileMemory(Mutex<MemoryState>);

const MEMORY_ITEMS: usize = 20;
const MEMORY_CHARACTERS: usize = 6000;

#[derive(Clone, Default)]
pub(super) struct MemoryState {
    pub(super) items: HashMap<String, Vec<MemoryRecord>>,
    pub(super) epochs: HashMap<String, u64>,
    next_id: u64,
}

impl MemoryState {
    pub(super) fn remove(&mut self, profile: &str, item: Option<&str>) {
        // In-flight requests may still contain a removed correction. Their
        // feedback must not resurrect it, even when the list is already empty.
        let epoch = self.epochs.entry(profile.into()).or_default();
        *epoch = epoch.saturating_add(1);
        if let Some(id) = item {
            if let Some(records) = self.items.get_mut(profile) {
                records.retain(|r| r.id != id);
            }
        } else {
            self.items.remove(profile);
        }
    }

    pub(super) fn admit(&mut self, profile: &str, epoch: u64, text: &str, session: &str) -> bool {
        if self.epochs.get(profile).copied().unwrap_or_default() != epoch || !bounded(text, 500) {
            return false;
        }
        let records = self.items.entry(profile.into()).or_default();
        if records
            .iter()
            .any(|r| r.source_session == session || normalized(&r.text) == normalized(text))
        {
            return false;
        }
        let Some(id) = self.next_id.checked_add(1) else {
            return false;
        };
        self.next_id = id;
        records.push(MemoryRecord {
            id: id.to_string(),
            text: text.trim().into(),
            source_session: session.into(),
        });
        while records.len() > MEMORY_ITEMS
            || records
                .iter()
                .map(|r| r.text.chars().count())
                .sum::<usize>()
                > MEMORY_CHARACTERS
        {
            records.remove(0);
        }
        true
    }
}

fn feedback_profile_is_current(
    catalog: &ProfileCatalog,
    context: &super::session::ResolvedContext,
) -> bool {
    let Some(profile) = catalog.profiles.iter().find(|p| p.id == context.profile_id) else {
        return false;
    };
    let prompt_owner = if profile.prompt.is_some() {
        Some(profile)
    } else {
        catalog.profiles.iter().find(|p| p.id == GENERAL)
    };
    u64::from(profile.revision) == context.profile_revision
        && u64::from(profile.dictionary_revision) == context.dictionary_revision
        && prompt_owner.is_some_and(|p| u64::from(p.revision) == context.prompt_revision)
}

/// No profile/catalog writes: feedback is admitted only to process-local memory.
pub(super) fn admit_memory(
    app: &AppHandle,
    context: &super::session::ResolvedContext,
    text: &str,
    session: &str,
) -> Result<bool, String> {
    let _lock = PROFILE_WRITES
        .lock()
        .map_err(|_| "Profile store lock poisoned")?;
    let cache = app.state::<ProfileCache>();
    let catalog = cache.0.lock().map_err(|_| "Profile cache lock poisoned")?;
    if !catalog
        .as_ref()
        .is_some_and(|c| feedback_profile_is_current(c, context))
    {
        return Ok(false);
    }
    let memory = app.state::<ProfileMemory>();
    let mut state = memory
        .0
        .lock()
        .map_err(|_| "Profile memory lock poisoned")?;
    Ok(state.admit(&context.profile_id, context.memory_epoch, text, session))
}

#[tauri::command]
#[specta::specta]
pub(crate) fn get_profile_memory(
    app: AppHandle,
    profile_id: String,
) -> Result<Vec<MemoryRecord>, String> {
    let memory = app.state::<ProfileMemory>();
    let items = memory
        .0
        .lock()
        .map_err(|_| "Profile memory lock poisoned")?;
    Ok(items.items.get(&profile_id).cloned().unwrap_or_default())
}

#[tauri::command]
#[specta::specta]
pub(crate) fn remove_profile_memory(
    app: AppHandle,
    profile_id: String,
    item_id: Option<String>,
) -> Result<(), String> {
    let memory = app.state::<ProfileMemory>();
    let mut items = memory
        .0
        .lock()
        .map_err(|_| "Profile memory lock poisoned")?;
    items.remove(&profile_id, item_id.as_deref());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn change_post_process_profiles_setting(
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    let _lock = PROFILE_WRITES
        .lock()
        .map_err(|_| "Profile store lock poisoned")?;
    let mut current = settings::get_settings(&app);
    if enabled {
        if !current.post_process_enabled {
            return Err("Enable post-processing before enabling profiles".into());
        }
        // Profile setup is local. Endpoint validation belongs to request::process,
        // so users can prepare profiles before choosing a provider or model.
        let catalog = load(&app)?;
        let general = catalog
            .profiles
            .iter()
            .find(|p| p.id == GENERAL)
            .ok_or("General profile is missing")?;
        template::validate(general.prompt.as_deref().unwrap_or_default())?;
    }
    current.post_process_profiles = enabled;
    settings::write_settings(&app, current);
    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn feedback_rejects_deleted_or_changed_originating_profile() {
        let context = super::super::feedback::tests::context("bomb", 0, 4);
        let catalog = ProfileCatalog::seed(&crate::settings::get_default_settings());
        assert!(feedback_profile_is_current(&catalog, &context));
        let mut changed = catalog.clone();
        changed.profiles[0].revision += 1;
        assert!(!feedback_profile_is_current(&changed, &context));
        changed = catalog.clone();
        changed.profiles[0].dictionary_revision += 1;
        assert!(!feedback_profile_is_current(&changed, &context));
        changed.profiles.clear();
        assert!(!feedback_profile_is_current(&changed, &context));
    }

    #[test]
    fn correction_memory_is_bounded_deduplicated_isolated_and_clear_cancels_inflight() {
        let mut memory = MemoryState::default();
        assert!(memory.admit("general", 0, "Use BOM", "1"));
        assert!(!memory.admit("general", 0, "  use   bom ", "2"));
        assert!(!memory.admit("general", 0, "Other correction", "1"));
        assert!(memory.admit("work", 0, "Use BOM", "2"));
        let id = memory.items["general"][0].id.clone();
        memory.remove("general", Some(&id));
        assert!(!memory.admit("general", 0, "Use BOM", "3"));
        assert!(memory.admit("general", 1, "Use BOM", "3"));
        memory.remove("general", None);
        assert!(!memory.admit("general", 1, "Use BOM", "4"));
        assert_eq!(memory.items["work"].len(), 1);
        assert!(!memory.admit("general", 2, " ", "4"));
        assert!(!memory.admit("general", 2, &"\u{1f600}".repeat(501), "4"));
        for i in 0..30 {
            assert!(memory.admit("general", 2, &format!("Correction {i}"), &format!("s{i}")));
        }
        assert_eq!(memory.items["general"].len(), 20);
        assert_eq!(memory.items["general"][0].text, "Correction 10");
        for i in 0..20 {
            assert!(memory.admit(
                "general",
                2,
                &format!("{i}:{}", "\u{1f600}".repeat(490)),
                &format!("large{i}")
            ));
        }
        assert!(
            memory.items["general"]
                .iter()
                .map(|r| r.text.chars().count())
                .sum::<usize>()
                <= 6000
        );
        assert!(memory.items["general"].len() < 20);
    }
    use super::*;

    fn seed() -> ProfileCatalog {
        let mut catalog = ProfileCatalog::seed(&settings::get_default_settings());
        catalog.profiles[0].prompt = Some("Rewrite {{transcript}}".into());
        catalog
    }

    fn profile() -> Profile {
        Profile {
            id: String::new(),
            name: "Terminal".into(),
            revision: 0,
            dictionary_revision: 0,
            icon: ProfileIcon::Terminal,
            rules: vec![RoutingRule {
                application: "WindowsTerminal.exe".into(),
                workspace: None,
            }],
            prompt: None,
            dictionary: vec![Keyword {
                id: "codex".into(),
                canonical: "Codex".into(),
                misheard_forms: vec!["codecks".into()],
            }],
        }
    }

    #[test]
    fn legacy_and_app_icons_round_trip_and_invalid_custom_icons_cannot_save() {
        for name in [
            "generic", "terminal", "code", "mail", "chrome", "firefox", "vscode", "intellij",
            "claude", "codex", "pi", "opencode", "github", "slack", "notes",
        ] {
            let encoded = format!("\"{name}\"");
            let icon: ProfileIcon = serde_json::from_str(&encoded).unwrap();
            assert_eq!(icon.display_value(), name);
            assert_eq!(serde_json::to_string(&icon).unwrap(), encoded);
        }
        let mut catalog = seed();
        let mut invalid = profile();
        invalid.icon = ProfileIcon::Custom("https://example.com/icon.svg".into());
        assert!(catalog.save_profile(invalid, 0).is_err());
        assert_eq!(catalog.profiles.len(), 1);
        assert_eq!(catalog.revision, 0);
    }

    #[test]
    fn round_trip_and_conflicting_edits_preserve_data() {
        let mut catalog = seed();
        catalog.save_profile(profile(), 0).unwrap();
        let disk = tempfile::NamedTempFile::new().unwrap();
        serde_json::to_writer(disk.as_file(), &catalog).unwrap();
        let mut loaded: ProfileCatalog =
            serde_json::from_reader(std::fs::File::open(disk.path()).unwrap()).unwrap();
        let mut edited = loaded.profiles[1].clone();
        edited.name = "Workspace".into();
        loaded.save_profile(edited.clone(), 1).unwrap();
        assert!(loaded.save_profile(edited, 1).is_err());
        assert_eq!(loaded.profiles[1].dictionary[0].canonical, "Codex");
        assert_eq!(loaded.profiles[1].name, "Workspace");
        loaded.delete_profile("profile-1", 2).unwrap();
        assert_eq!(loaded.profiles.len(), 1);
        assert!(loaded.delete_profile(GENERAL, 3).is_err());
    }

    #[test]
    fn prompt_override_reset_and_inheritance_are_separate() {
        let mut catalog = seed();
        catalog.save_profile(profile(), 0).unwrap();
        assert!(catalog.profiles[1].prompt.is_none());
        let mut custom = catalog.profiles[1].clone();
        custom.prompt = Some("Custom {{dictionary}}".into());
        catalog.save_profile(custom.clone(), 1).unwrap();
        custom.prompt = None;
        catalog.save_profile(custom, 2).unwrap();
        assert!(catalog.profiles[1].prompt.is_none());
        assert_eq!(
            catalog.profiles[0].prompt.as_deref(),
            Some("Rewrite {{transcript}}")
        );
    }

    #[test]
    fn dictionary_collisions_are_rejected_without_mutation() {
        let mut catalog = seed();
        let mut p = profile();
        p.dictionary.push(Keyword {
            id: "other".into(),
            canonical: "Other".into(),
            misheard_forms: vec![" CODECKS ".into()],
        });
        assert!(catalog.save_profile(p, 0).is_err());
        assert_eq!(catalog.revision, 0);
        assert_eq!(catalog.profiles.len(), 1);
    }

    #[test]
    fn initialization_copies_legacy_prompt_and_leaves_asr_alone() {
        let mut settings = settings::get_default_settings();
        settings.custom_words = vec!["ASR only".into()];
        settings.post_process_prompts[0].prompt = "Rewrite ${output}".into();
        settings.post_process_selected_prompt_id =
            Some(settings.post_process_prompts[0].id.clone());
        let seeded = ProfileCatalog::seed(&settings);
        assert_eq!(seeded.profiles[0].prompt.as_deref(), Some("Rewrite"));
        assert!(seeded.profiles[0].dictionary.is_empty());
        settings.post_process_prompts[0].prompt = "Changed".into();
        let reloaded: ProfileCatalog =
            serde_json::from_value(serde_json::to_value(&seeded).unwrap()).unwrap();
        assert_eq!(reloaded.profiles[0].prompt.as_deref(), Some("Rewrite"));
        assert_eq!(settings.custom_words, ["ASR only"]);

        settings.post_process_selected_prompt_id = None;
        let fallback = ProfileCatalog::seed(&settings);
        assert!(template::validate(fallback.profiles[0].prompt.as_deref().unwrap()).is_ok());
    }
}
