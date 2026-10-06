use super::template;
use crate::settings::{self, AppSettings};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

// Profile data has a separate owner/store. Existing whole-settings writes and
// debug dumps cannot overwrite or disclose profile memory content.
pub(crate) static PROFILE_WRITES: Mutex<()> = Mutex::new(());
const STORE_PATH: &str = "context-profiles.json";
const GENERAL: &str = "general";

#[derive(Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
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
    pub rewrite_context_revision: u32,
    pub long_term_memory_revision: u32,
    pub icon: ProfileIcon,
    pub rules: Vec<RoutingRule>,
    /// General must own a template. None on other profiles inherits General.
    pub prompt: Option<String>,
    pub long_term_memory: String,
    pub consolidation_instructions: Option<String>,
    #[serde(default)]
    pub long_term_undo: Option<LongTermUndo>,
    #[serde(default)]
    pub last_consolidation: Option<super::consolidation::ConsolidationOperation>,
}

#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct LongTermUndo {
    pub text: String,
    pub committed_revision: u32,
}

/// The ordinary editor cannot supply memory or backend-owned revisions.
#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileEdit {
    pub id: String,
    pub name: String,
    pub icon: ProfileIcon,
    pub rules: Vec<RoutingRule>,
    pub prompt: Option<String>,
    pub consolidation_instructions: Option<String>,
}

impl From<Profile> for ProfileEdit {
    fn from(p: Profile) -> Self {
        Self {
            id: p.id,
            name: p.name,
            icon: p.icon,
            rules: p.rules,
            prompt: p.prompt,
            consolidation_instructions: p.consolidation_instructions,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileCatalog {
    pub schema_version: u32,
    pub revision: u32,
    pub(super) next_id: u32,
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
    pub(super) fn validate_identity(&self) -> Result<(), String> {
        let mut ids = std::collections::HashSet::new();
        if self.schema_version != 3
            || self.profiles.is_empty()
            || self.profiles.len() > 64
            || self.profiles.iter().filter(|p| p.id == GENERAL).count() != 1
            || self
                .profiles
                .iter()
                .any(|p| p.id.is_empty() || !ids.insert(&p.id))
            || ids.contains(&format!("profile-{}", self.next_id))
        {
            return Err(
                "Profile store identity or version is invalid; existing data was retained".into(),
            );
        }
        Ok(())
    }
    pub(super) fn seed(settings: &AppSettings) -> Self {
        let prompt = settings
            .post_process_selected_prompt_id
            .as_ref()
            .and_then(|id| settings.post_process_prompts.iter().find(|p| &p.id == id))
            .map(|p| super::migration::placeholders(p.prompt.replace("${output}", "").trim()))
            .filter(|prompt| template::validate(prompt).is_ok())
            .unwrap_or_else(|| "Rewrite {{transcript}} accurately using {{long_term_memory}} and {{short_term_memory}}. Use {{input_context}} when available.".into());
        Self {
            schema_version: 3,
            revision: 0,
            next_id: 1,
            profiles: vec![Profile {
                id: GENERAL.into(),
                name: "General".into(),
                revision: 0,
                rewrite_context_revision: 0,
                long_term_memory_revision: 0,
                icon: ProfileIcon::Generic,
                rules: vec![],
                prompt: Some(prompt),
                long_term_memory: String::new(),
                consolidation_instructions: None,
                long_term_undo: None,
                last_consolidation: None,
            }],
        }
    }

    fn check_revision(&self, expected: u32) -> Result<(), String> {
        if expected != self.revision {
            return Err("Profiles changed; reload before saving".into());
        }
        Ok(())
    }

    fn save_profile(&mut self, edit: ProfileEdit, expected: u32) -> Result<(), String> {
        self.check_revision(expected)?;
        let existing = self.profiles.iter().find(|p| p.id == edit.id);
        if !edit.id.is_empty() && existing.is_none() {
            return Err("Profile no longer exists".into());
        }
        let mut profile = Profile {
            id: edit.id,
            name: edit.name,
            icon: edit.icon,
            rules: edit.rules,
            prompt: edit.prompt,
            consolidation_instructions: edit.consolidation_instructions,
            revision: existing.map_or(0, |p| p.revision),
            rewrite_context_revision: existing.map_or(0, |p| p.rewrite_context_revision),
            long_term_memory_revision: existing.map_or(0, |p| p.long_term_memory_revision),
            long_term_memory: existing.map_or_else(String::new, |p| p.long_term_memory.clone()),
            long_term_undo: existing.and_then(|p| p.long_term_undo.clone()),
            last_consolidation: existing.and_then(|p| p.last_consolidation.clone()),
        };
        // None selects the built-in default; an override holds 1 to 4,000 characters.
        if profile
            .consolidation_instructions
            .as_ref()
            .is_some_and(|v| !bounded(v, 4000))
        {
            return Err("Consolidation instructions must contain 1 to 4,000 characters".into());
        }
        if let ProfileIcon::Custom(data) = &profile.icon {
            super::icons::validate_custom(data)?;
        }
        if !bounded(&profile.name, 80) {
            return Err("Profile name must contain 1 to 80 characters".into());
        }
        if profile.rules.len() > 16 {
            return Err("Profile exceeds its routing rule limit".into());
        }
        if profile.id == GENERAL {
            if !profile.rules.is_empty() {
                return Err("General cannot have routing rules".into());
            }
            if profile.prompt.is_none() {
                return Err("General must define the default prompt".into());
            }
        } else if profile.rules.is_empty() {
            return Err("A profile requires a routing rule".into());
        }
        if let Some(prompt) = &profile.prompt {
            template::validate(prompt)?;
        }
        for rule in &profile.rules {
            if rule.workspace.is_none() && !bounded(&rule.application, 260)
                || rule.application.chars().count() > 260
                || rule.application.contains(['/', '\\'])
            {
                return Err("Application must be an executable name".into());
            }
            if rule
                .workspace
                .as_ref()
                .is_some_and(|w| !bounded(w, 1024) || super::providers::directory_key(w).is_none())
            {
                return Err("Invalid workspace directory".into());
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
        if existing.is_none_or(|p| p.rules != profile.rules || p.prompt != profile.prompt) {
            profile.rewrite_context_revision = next_revision;
        }
        if profile.id.is_empty() {
            if self.profiles.len() >= 64 {
                return Err("At most 64 profiles are supported".into());
            }
            // Skip IDs already in use, including ones above next_id, so two
            // profiles can never share one memory owner.
            let mut candidate = self.next_id;
            while self
                .profiles
                .iter()
                .any(|p| p.id == format!("profile-{candidate}"))
            {
                candidate = candidate
                    .checked_add(1)
                    .ok_or("Profile identity exhausted")?;
            }
            let next_id = candidate
                .checked_add(1)
                .ok_or("Profile identity exhausted")?;
            profile.id = format!("profile-{candidate}");
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

pub(super) fn load(app: &AppHandle) -> Result<ProfileCatalog, String> {
    let path = crate::portable::resolve_app_data(app, STORE_PATH)
        .map_err(|_| "Could not resolve profile store")?;
    let root = read_store(&path)?;
    if let Some(value) = root.get("profiles").cloned() {
        let version = value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64);
        let migrated = if version == Some(1) {
            let catalog = super::migration::convert(value.clone())?;
            let path = crate::portable::resolve_app_data(app, STORE_PATH)
                .map_err(|_| "Could not resolve profile store")?;
            super::migration::backup(&path)?;
            persist(app, &catalog)?;
            return Ok(catalog);
        } else {
            value
        };
        let catalog: ProfileCatalog = serde_json::from_value(migrated)
            .map_err(|_| "Profile store is invalid; existing data was retained")?;
        catalog.validate_identity()?;
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

pub(super) fn persist(app: &AppHandle, catalog: &ProfileCatalog) -> Result<(), String> {
    // Acquire publication authority before touching disk. A poisoned cache
    // cannot turn a successfully committed write into a reported failure.
    let state = app.state::<ProfileCache>();
    let mut cache = state.0.lock().map_err(|_| "Profile cache lock poisoned")?;
    let path = crate::portable::resolve_app_data(app, STORE_PATH)
        .map_err(|_| "Could not resolve profile store")?;
    persist_file(&path, catalog)?;
    cache.replace(Arc::new(catalog.clone()));
    Ok(())
}

fn read_store(path: &Path) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|_| "Profile store is invalid; existing data was retained".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::Map::new()),
        Err(_) => Err("Could not read profile store; existing data was retained".into()),
    }
}

pub(super) fn persist_file(path: &Path, catalog: &ProfileCatalog) -> Result<(), String> {
    // Preserve unrelated top-level values. This dedicated owner intentionally
    // does not register the file with the plugin's non-atomic autosave/exit path.
    let mut root = read_store(path)?;
    root.insert(
        "profiles".into(),
        serde_json::to_value(catalog).map_err(|_| "Could not serialize profiles")?,
    );
    let bytes = serde_json::to_vec_pretty(&root).map_err(|_| "Could not serialize profiles")?;
    atomic_write_with(path, |file| file.write_all(&bytes))
        .map_err(|_| "Could not save profiles; changes were not applied".into())
}

fn atomic_write_with(
    path: &Path,
    write: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("Invalid profile path"))?;
    std::fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    write(temporary.as_file_mut())?;
    temporary.as_file().sync_all()?;
    // Same-directory replacement keeps old bytes authoritative until commit.
    // On any pre-commit error, NamedTempFile removes only its owned partial file.
    temporary.persist(path).map_err(|error| error.error)?;
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
    profile: ProfileEdit,
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
    super::consolidation::discard_profile_job(&app, &id)?;
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
    pub proposal_index: u32,
    pub revision: u32,
    pub evidence_quote: String,
    pub provenance: String,
    pub wrong: Option<String>,
    pub corrected: Option<String>,
    pub scope: Option<String>,
}

/// Process-local memory. No raw field contents are persisted as memory.
#[derive(Default)]
pub(crate) struct ProfileMemory(pub(super) Mutex<MemoryState>);

const MEMORY_ITEMS: usize = 20;
const MEMORY_CHARACTERS: usize = 6000;

#[derive(Clone, Default)]
pub(super) struct MemoryState {
    pub(super) items: HashMap<String, Vec<MemoryRecord>>,
    pub(super) epochs: HashMap<String, u64>,
    next_id: u64,
    consumed: std::collections::VecDeque<(String, String)>,
    skips: HashMap<String, String>,
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

    pub(super) fn admit_batch(
        &mut self,
        profile: &str,
        epoch: u64,
        batch: &[super::memory_proposals::MemoryProposal],
        session: &str,
    ) -> bool {
        use super::memory_proposals::{inverse_terms, normalized, MemoryAction};
        if self.epochs.get(profile).copied().unwrap_or_default() != epoch
            || batch.is_empty()
            || batch.len() > 4
            || self.consumed.contains(&(profile.into(), session.into()))
        {
            return false;
        }
        let mut candidate = self.clone();
        let records = candidate.items.entry(profile.into()).or_default();
        let mut replaced = false;
        let mut touched = std::collections::HashSet::new();
        for (index, p) in batch.iter().enumerate() {
            if !bounded(&p.text, 500) || !bounded(&p.evidence_quote, 500) {
                return false;
            }
            if records.iter().any(|r| {
                normalized(&r.text) == normalized(&p.text) && p.target_id.as_deref() != Some(&r.id)
            }) {
                return false;
            }
            if p.wrong.is_some()
                && records.iter().any(|r| {
                    r.wrong.as_deref().map(normalized) == p.wrong.as_deref().map(normalized)
                        && r.scope.as_deref().map(normalized) == p.scope.as_deref().map(normalized)
                        && r.corrected != p.corrected
                        && p.target_id.as_deref() != Some(&r.id)
                })
            {
                return false;
            }
            // A reversed pair (bomb->BOM vs BOM->bomb) contradicts the same
            // record; only a replace of that record may resolve it.
            if let (Some(wrong), Some(corrected)) = (&p.wrong, &p.corrected) {
                if records.iter().any(|r| {
                    inverse_terms(r.wrong.as_deref(), r.corrected.as_deref(), wrong, corrected)
                        && r.scope.as_deref().map(normalized) == p.scope.as_deref().map(normalized)
                        && p.target_id.as_deref() != Some(&r.id)
                }) {
                    return false;
                }
            }
            let (id, revision, target) = match p.action {
                MemoryAction::Add if p.target_id.is_none() && p.expected_revision.is_none() => {
                    let Some(id) = candidate.next_id.checked_add(1) else {
                        return false;
                    };
                    candidate.next_id = id;
                    (id.to_string(), 1, None)
                }
                MemoryAction::Replace => {
                    let Some(target) = records
                        .iter()
                        .position(|r| Some(&r.id) == p.target_id.as_ref())
                    else {
                        return false;
                    };
                    let r = &records[target];
                    if p.expected_revision != Some(r.revision) {
                        return false;
                    }
                    let Some(revision) = r.revision.checked_add(1) else {
                        return false;
                    };
                    replaced = true;
                    (r.id.clone(), revision, Some(target))
                }
                _ => return false,
            };
            let record = MemoryRecord {
                id,
                revision,
                text: p.text.trim().into(),
                source_session: session.into(),
                proposal_index: index as u32,
                evidence_quote: p.evidence_quote.clone(),
                provenance: "verified_readback".into(),
                wrong: p.wrong.clone(),
                corrected: p.corrected.clone(),
                scope: p.scope.clone(),
            };
            touched.insert(record.id.clone());
            if let Some(target) = target {
                records[target] = record;
            } else {
                records.push(record);
            }
        }
        // Evict the oldest record this batch did not add or replace. A batch
        // whose own records cannot fit is rejected instead of partly applied.
        while records.len() > MEMORY_ITEMS
            || records
                .iter()
                .map(|r| r.text.chars().count())
                .sum::<usize>()
                > MEMORY_CHARACTERS
        {
            let Some(oldest) = records.iter().position(|r| !touched.contains(&r.id)) else {
                return false;
            };
            records.remove(oldest);
        }
        if replaced {
            *candidate.epochs.entry(profile.into()).or_default() = epoch.saturating_add(1);
        }
        candidate
            .consumed
            .push_back((profile.into(), session.into()));
        while candidate.consumed.len() > 1024 {
            candidate.consumed.pop_front();
        }
        *self = candidate;
        self.skips.remove(profile);
        true
    }

    #[cfg(test)]
    pub(super) fn admit(&mut self, profile: &str, epoch: u64, text: &str, session: &str) -> bool {
        use super::memory_proposals::{MemoryAction, MemoryProposal};
        self.admit_batch(
            profile,
            epoch,
            &[MemoryProposal {
                action: MemoryAction::Add,
                text: text.into(),
                evidence_quote: text.into(),
                target_id: None,
                expected_revision: None,
                wrong: None,
                corrected: None,
                scope: None,
            }],
            session,
        )
    }
}

/// Events contain profile identity only. Quotes and captured text stay local.
pub(super) fn report_memory_skip(app: &AppHandle, profile: &str, reason: &str) {
    use tauri::Emitter;
    if let Ok(mut state) = app.state::<ProfileMemory>().0.lock() {
        state.skips.insert(profile.into(), reason.into());
    }
    let _ = app.emit("profile-memory-updated", profile);
}

#[tauri::command]
#[specta::specta]
pub(crate) fn get_profile_memory_skip_reason(
    app: AppHandle,
    profile_id: String,
) -> Result<Option<String>, String> {
    let memory = app.state::<ProfileMemory>();
    let state = memory
        .0
        .lock()
        .map_err(|_| "Profile memory lock poisoned")?;
    Ok(state.skips.get(&profile_id).cloned())
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
    u64::from(profile.rewrite_context_revision) == context.profile_revision
        && u64::from(profile.long_term_memory_revision) == context.long_term_memory_revision
        && prompt_owner
            .is_some_and(|p| u64::from(p.rewrite_context_revision) == context.prompt_revision)
}

/// No profile/catalog writes: feedback is admitted only to process-local memory.
pub(super) fn admit_memory(
    app: &AppHandle,
    context: &super::session::ResolvedContext,
    batch: &[super::memory_proposals::MemoryProposal],
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
    Ok(state.admit_batch(&context.profile_id, context.memory_epoch, batch, session))
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
    fn corrupted_profile_identities_cannot_alias_memory_owners() {
        let mut catalog = seed();
        assert!(catalog.validate_identity().is_ok());
        catalog.profiles.push(catalog.profiles[0].clone());
        assert!(catalog.validate_identity().is_err());
        catalog.profiles[1].id = "profile-1".into();
        // A reused next ID would make two profiles share one memory owner.
        assert!(catalog.validate_identity().is_err());
        catalog.next_id = 2;
        assert!(catalog.validate_identity().is_ok());
    }

    #[test]
    fn failed_atomic_write_retains_exact_original_and_removes_partial_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("context-profiles.json");
        let original = b"{ \"profiles\": {\"schema_version\": 1}, \"unrelated\": true }\n";
        std::fs::write(&path, original).unwrap();
        assert!(atomic_write_with(&path, |file| {
            file.write_all(b"partially written replacement")?;
            Err(std::io::Error::other("injected write failure"))
        })
        .is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
        let absent = directory.path().join("absent.json");
        assert!(atomic_write_with(&absent, |_| Err(std::io::Error::other("injected"))).is_err());
        assert!(!absent.exists());
        persist_file(&path, &seed()).unwrap();
        let root = read_store(&path).unwrap();
        assert_eq!(root["unrelated"], true);
        assert_eq!(root["profiles"]["schema_version"], 3);
        let restarted: ProfileCatalog = serde_json::from_value(root["profiles"].clone()).unwrap();
        assert_eq!(
            restarted.profiles[0].long_term_memory,
            seed().profiles[0].long_term_memory
        );
    }

    #[test]
    fn batches_commit_once_atomically_and_replacements_keep_identity() {
        use super::super::memory_proposals::{correction, MemoryAction};
        let mut memory = MemoryState::default();
        let batch: Vec<_> = (0..4)
            .map(|i| {
                let mut p = correction();
                p.text = format!("Correction {i}");
                p.wrong = Some(format!("wrong{i}"));
                p.corrected = Some(format!("right{i}"));
                p
            })
            .collect();
        assert!(memory.admit_batch("general", 0, &batch, "one"));
        assert_eq!(memory.items["general"].len(), 4);
        assert!(!memory.admit_batch("general", 0, &batch, "one"));
        let original = memory.items["general"][0].clone();
        let mut replace = batch[0].clone();
        replace.action = MemoryAction::Replace;
        replace.target_id = Some(original.id.clone());
        replace.expected_revision = Some(original.revision);
        replace.text = "Corrected rule".into();
        replace.corrected = Some("updated".into());
        let mut invalid = batch[1].clone();
        invalid.target_id = Some("missing".into());
        assert!(!memory.admit_batch("general", 0, &[replace.clone(), invalid], "two"));
        assert_eq!(memory.items["general"][0].revision, 1);
        assert!(memory.admit_batch("general", 0, &[replace.clone()], "two"));
        assert_eq!(memory.items["general"][0].id, original.id);
        assert_eq!(memory.items["general"][0].revision, 2);
        assert_eq!(memory.epochs["general"], 1);
        assert!(!memory.admit_batch("general", 1, &[replace], "three"));
        // FIFO eviction is not a destructive source invalidation.
        assert!(memory.admit("general", 1, "Late note", "four"));
        assert_eq!(memory.epochs["general"], 1);
    }

    #[test]
    fn eviction_keeps_records_replaced_or_added_by_the_same_batch() {
        use super::super::memory_proposals::{MemoryAction, MemoryProposal};
        let mut memory = MemoryState::default();
        for i in 0..20 {
            assert!(memory.admit("general", 0, &format!("Note {i}"), &format!("s{i}")));
        }
        let oldest = memory.items["general"][0].clone();
        let second = memory.items["general"][1].id.clone();
        let replace = MemoryProposal {
            action: MemoryAction::Replace,
            text: "Note zero, revised".into(),
            evidence_quote: "Note zero, revised".into(),
            target_id: Some(oldest.id.clone()),
            expected_revision: Some(oldest.revision),
            wrong: None,
            corrected: None,
            scope: None,
        };
        let add = MemoryProposal {
            action: MemoryAction::Add,
            text: "Note twenty".into(),
            evidence_quote: "Note twenty".into(),
            target_id: None,
            expected_revision: None,
            wrong: None,
            corrected: None,
            scope: None,
        };
        assert!(memory.admit_batch("general", 0, &[replace, add], "batch"));
        let records = &memory.items["general"];
        assert_eq!(records.len(), 20);
        let kept = records.iter().find(|r| r.id == oldest.id).unwrap();
        assert_eq!(kept.text, "Note zero, revised");
        assert_eq!(kept.revision, oldest.revision + 1);
        assert!(records.iter().any(|r| r.text == "Note twenty"));
        assert!(!records.iter().any(|r| r.id == second));
    }

    #[test]
    fn reversed_term_pair_needs_a_replace_of_the_existing_record() {
        use super::super::memory_proposals::{correction, MemoryAction};
        let mut memory = MemoryState::default();
        assert!(memory.admit_batch("general", 0, &[correction()], "one"));
        let existing = memory.items["general"][0].clone();
        let mut reverse = correction();
        reverse.text = "Use bomb when BOM refers to this term.".into();
        reverse.evidence_quote = "not BOM, bomb".into();
        reverse.wrong = Some("BOM".into());
        reverse.corrected = Some("bomb".into());
        assert!(!memory.admit_batch("general", 0, &[reverse.clone()], "two"));
        assert_eq!(memory.items["general"].len(), 1);
        reverse.action = MemoryAction::Replace;
        reverse.target_id = Some(existing.id.clone());
        reverse.expected_revision = Some(existing.revision);
        assert!(memory.admit_batch("general", 0, &[reverse], "two"));
        assert_eq!(memory.items["general"].len(), 1);
        assert_eq!(memory.items["general"][0].wrong.as_deref(), Some("BOM"));
    }

    #[test]
    fn consolidation_override_requires_one_to_four_thousand_characters() {
        let mut catalog = seed();
        for invalid in [String::new(), "  \n ".into(), "x".repeat(4001)] {
            let mut edit = ProfileEdit::from(catalog.profiles[0].clone());
            edit.consolidation_instructions = Some(invalid);
            assert!(catalog.save_profile(edit, catalog.revision).is_err());
        }
        assert_eq!(catalog.revision, 0);
        let mut edit = ProfileEdit::from(catalog.profiles[0].clone());
        edit.consolidation_instructions = Some("x".repeat(4000));
        catalog.save_profile(edit.clone(), 0).unwrap();
        edit.consolidation_instructions = None;
        catalog.save_profile(edit, 1).unwrap();
        assert!(catalog.profiles[0].consolidation_instructions.is_none());
    }

    #[test]
    fn new_profile_ids_skip_identities_above_next_id() {
        let mut catalog = seed();
        catalog.save_profile(profile().into(), 0).unwrap();
        // A catalog may already hold an ID above next_id.
        catalog.profiles[1].id = "profile-3".into();
        assert_eq!(catalog.next_id, 2);
        for name in ["Second", "Third"] {
            let mut edit = ProfileEdit::from(profile());
            edit.name = name.into();
            catalog.save_profile(edit, catalog.revision).unwrap();
        }
        let ids: Vec<_> = catalog.profiles.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["general", "profile-3", "profile-2", "profile-4"]);
        assert_eq!(catalog.next_id, 5);
        assert!(catalog.validate_identity().is_ok());
    }

    #[test]
    fn feedback_rejects_deleted_or_changed_originating_profile() {
        let context = super::super::feedback::tests::context("bomb", 0, 4);
        let catalog = ProfileCatalog::seed(&crate::settings::get_default_settings());
        assert!(feedback_profile_is_current(&catalog, &context));
        let mut changed = catalog.clone();
        changed.profiles[0].rewrite_context_revision += 1;
        assert!(!feedback_profile_is_current(&changed, &context));
        changed = catalog.clone();
        changed.profiles[0].long_term_memory_revision += 1;
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
            rewrite_context_revision: 0,
            long_term_memory_revision: 0,
            icon: ProfileIcon::Terminal,
            rules: vec![RoutingRule {
                application: "WindowsTerminal.exe".into(),
                workspace: None,
            }],
            prompt: None,
            long_term_memory: "Use Codex for codecks.".into(),
            consolidation_instructions: None,
            long_term_undo: None,
            last_consolidation: None,
        }
    }

    #[test]
    fn directory_rules_save_without_application_and_reject_relative_paths() {
        let mut catalog = seed();
        let mut edit = ProfileEdit::from(profile());
        edit.rules[0].application.clear();
        edit.rules[0].workspace = Some("D:\\Work\\Project".into());
        catalog.save_profile(edit, 0).unwrap();
        assert!(catalog.profiles[1].rules[0].application.is_empty());
        let mut invalid = ProfileEdit::from(catalog.profiles[1].clone());
        invalid.rules[0].workspace = Some("relative/project".into());
        assert!(catalog.save_profile(invalid, catalog.revision).is_err());
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
        assert!(catalog.save_profile(invalid.into(), 0).is_err());
        assert_eq!(catalog.profiles.len(), 1);
        assert_eq!(catalog.revision, 0);
    }

    #[test]
    fn round_trip_and_conflicting_edits_preserve_data() {
        let mut catalog = seed();
        catalog.save_profile(profile().into(), 0).unwrap();
        let disk = tempfile::NamedTempFile::new().unwrap();
        serde_json::to_writer(disk.as_file(), &catalog).unwrap();
        let mut loaded: ProfileCatalog =
            serde_json::from_reader(std::fs::File::open(disk.path()).unwrap()).unwrap();
        let mut edited = loaded.profiles[1].clone();
        edited.name = "Workspace".into();
        loaded.save_profile(edited.clone().into(), 1).unwrap();
        assert!(loaded.save_profile(edited.into(), 1).is_err());
        assert!(loaded.profiles[1].long_term_memory.is_empty());
        assert_eq!(loaded.profiles[1].name, "Workspace");
        loaded.delete_profile("profile-1", 2).unwrap();
        assert_eq!(loaded.profiles.len(), 1);
        assert!(loaded.delete_profile(GENERAL, 3).is_err());
    }

    #[test]
    fn prompt_override_reset_and_inheritance_are_separate() {
        let mut catalog = seed();
        catalog.save_profile(profile().into(), 0).unwrap();
        assert!(catalog.profiles[1].prompt.is_none());
        let mut custom = catalog.profiles[1].clone();
        custom.prompt = Some("Custom {{long_term_memory}}".into());
        catalog.save_profile(custom.clone().into(), 1).unwrap();
        custom.prompt = None;
        catalog.save_profile(custom.into(), 2).unwrap();
        assert!(catalog.profiles[1].prompt.is_none());
        assert_eq!(
            catalog.profiles[0].prompt.as_deref(),
            Some("Rewrite {{transcript}}")
        );
    }

    #[test]
    fn editor_has_no_memory_authority_and_metadata_preserves_rewrite_revision() {
        let mut catalog = seed();
        let original = catalog.profiles[0].clone();
        let mut edit: ProfileEdit = original.clone().into();
        edit.name = "Renamed".into();
        edit.consolidation_instructions = Some("Preserve terms".into());
        catalog.save_profile(edit, 0).unwrap();
        assert_eq!(
            catalog.profiles[0].rewrite_context_revision,
            original.rewrite_context_revision
        );
        assert_eq!(
            catalog.profiles[0].long_term_memory,
            original.long_term_memory
        );
        let mut json = serde_json::to_value(ProfileEdit::from(original)).unwrap();
        json["long_term_memory"] = serde_json::json!("Injected");
        assert!(serde_json::from_value::<ProfileEdit>(json).is_err());
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
        assert!(seeded.profiles[0].long_term_memory.is_empty());
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
