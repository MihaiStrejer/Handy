use super::target::TargetIdentity;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// The audio cancellation generation is not unique across successful sessions.
/// The serial also distinguishes two recordings with no intervening cancel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SessionId {
    serial: u64,
    cancel_generation: u64,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "availability", content = "value", rename_all = "snake_case")]
pub(super) enum Captured<T> {
    Present(T),
    Empty,
    Unavailable,
    Uncertain,
    Protected,
    TimedOut,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct InputContext {
    pub application: Captured<String>,
    pub workspace: Captured<String>,
    pub selection: Captured<String>,
    pub surrounding_text: Captured<String>,
    /// UTF-16 code units within the captured surrounding text, when verified.
    pub caret_utf16: Captured<u32>,
    /// Local evidence for exact replacement; never sent to the endpoint.
    #[serde(skip)]
    pub selection_range_utf16: Option<(u32, u32)>,
    pub captured_at_ms: u64,
    pub truncated: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct DictionaryEntry {
    pub id: String,
    pub canonical: String,
    pub misheard_forms: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct MemoryItem {
    pub id: String,
    pub text: String,
    pub source_session: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PromptSource {
    General,
    Inherited,
    Custom,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum MatchBasis {
    Workspace,
    Application,
    General,
    Ambiguous,
}

/// Owned values freeze data independently of later edits to settings or focus.
/// This is the only session type intended for request serialization.
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct ResolvedContext {
    pub profile_id: String,
    pub profile_name: String,
    pub profile_revision: u64,
    pub icon: String,
    pub match_basis: MatchBasis,
    pub prompt: String,
    pub prompt_source: PromptSource,
    pub prompt_revision: u64,
    pub dictionary_revision: u64,
    pub dictionary: Vec<DictionaryEntry>,
    pub memory: Vec<MemoryItem>,
    #[serde(skip)]
    pub memory_epoch: u64,
    pub input: InputContext,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum TextOperation {
    Insert,
    ReplaceSelection,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum FeedbackEffect {
    None {},
    AddMisheardForm { keyword_id: String, phrase: String },
    Remember { text: String },
}

/// A model proposes an operation and an effect; it never supplies destination,
/// profile, session, or completion authority.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Prediction {
    pub text: String,
    pub operation: TextOperation,
    pub effect: FeedbackEffect,
}

#[derive(PartialEq, Eq)]
enum Phase {
    Resolving,
    Resolved,
    Requesting,
    Predicted,
    FeedbackConsumed,
}

struct Session {
    id: SessionId,
    profile_mode: bool,
    // Retain this locally so enrichment must use the original target.
    target: Option<TargetIdentity>,
    captured: Option<InputContext>,
    context: Option<ResolvedContext>,
    prediction: Option<Prediction>,
    feedback_text: Option<String>,
    phase: Phase,
}

#[derive(Default)]
struct State {
    serial: u64,
    active: Option<Session>,
}

#[derive(Default)]
pub(crate) struct SessionStore(Mutex<State>);

/// Public widget metadata deliberately excludes captured text and native handles.
#[derive(Clone, Serialize, specta::Type, tauri_specta::Event)]
pub(crate) struct SessionDisplay {
    pub session_id: String,
    pub enabled: bool,
    pub profile: Option<ProfileIndicator>,
}

#[derive(Clone, Serialize, specta::Type)]
pub(crate) struct ProfileIndicator {
    pub name: String,
    pub icon: String,
    pub input_mode: String,
}

impl SessionStore {
    pub(crate) fn begin(
        &self,
        cancel_generation: u64,
        target: Option<TargetIdentity>,
        profile_mode: bool,
    ) -> Result<SessionId, &'static str> {
        let mut state = self.0.lock().map_err(|_| "Context session lock poisoned")?;
        state.serial = state.serial.checked_add(1).ok_or("Session ID exhausted")?;
        let id = SessionId {
            serial: state.serial,
            cancel_generation,
        };
        state.active = Some(Session {
            id,
            profile_mode,
            target,
            captured: None,
            context: None,
            prediction: None,
            feedback_text: None,
            phase: Phase::Resolving,
        });
        Ok(id)
    }

    pub(crate) fn display(&self) -> Result<Option<SessionDisplay>, &'static str> {
        let state = self.0.lock().map_err(|_| "Context session lock poisoned")?;
        Ok(state.active.as_ref().map(|session| SessionDisplay {
            session_id: session.id.serial.to_string(),
            enabled: session.profile_mode,
            profile: session.context.as_ref().map(|context| ProfileIndicator {
                name: context.profile_name.clone(),
                icon: context.icon.clone(),
                input_mode: match &context.input.selection {
                    Captured::Present(_) => "edit",
                    Captured::Empty => "compose",
                    Captured::Protected => "protected",
                    _ => "unknown",
                }
                .into(),
            }),
        }))
    }

    pub(crate) fn current(&self, generation: u64) -> Result<Option<SessionId>, &'static str> {
        let state = self.0.lock().map_err(|_| "Context session lock poisoned")?;
        Ok(state
            .active
            .as_ref()
            .filter(|s| s.id.cancel_generation == generation)
            .map(|s| s.id))
    }

    pub(crate) fn uses_profiles(&self, id: SessionId) -> bool {
        self.0.lock().ok().is_some_and(|state| {
            state
                .active
                .as_ref()
                .is_some_and(|s| s.id == id && s.profile_mode)
        })
    }

    pub(super) fn target(&self, id: SessionId) -> Result<Option<TargetIdentity>, &'static str> {
        let state = self.0.lock().map_err(|_| "Context session lock poisoned")?;
        Ok(state
            .active
            .as_ref()
            .filter(|s| s.id == id)
            .and_then(|s| s.target.clone()))
    }

    pub(super) fn attach_capture(
        &self,
        id: SessionId,
        generation: u64,
        captured: InputContext,
    ) -> Result<bool, &'static str> {
        let mut state = self.0.lock().map_err(|_| "Context session lock poisoned")?;
        let Some(session) = state.active.as_mut().filter(|s| {
            s.id == id
                && id.cancel_generation == generation
                && s.phase == Phase::Resolving
                && s.captured.is_none()
        }) else {
            return Ok(false);
        };
        session.captured = Some(captured);
        Ok(true)
    }

    /// Accept enrichment once, and only for the current recording generation.
    pub(super) fn resolve(
        &self,
        id: SessionId,
        generation: u64,
        context: ResolvedContext,
    ) -> Result<bool, &'static str> {
        let mut state = self.0.lock().map_err(|_| "Context session lock poisoned")?;
        let Some(session) = state.active.as_mut().filter(|s| {
            s.id == id && id.cancel_generation == generation && s.phase == Phase::Resolving
        }) else {
            return Ok(false);
        };
        session.context = Some(context);
        session.phase = Phase::Resolved;
        Ok(true)
    }

    /// A duplicate invocation cannot issue a second request for this session.
    pub(super) fn begin_request(
        &self,
        id: SessionId,
        generation: u64,
    ) -> Result<Option<ResolvedContext>, &'static str> {
        let mut state = self.0.lock().map_err(|_| "Context session lock poisoned")?;
        let Some(session) = state.active.as_mut().filter(|s| {
            s.id == id && id.cancel_generation == generation && s.phase == Phase::Resolved
        }) else {
            return Ok(None);
        };
        session.phase = Phase::Requesting;
        Ok(session.context.clone())
    }

    pub(super) fn accept_prediction(
        &self,
        id: SessionId,
        generation: u64,
        prediction: Prediction,
        transcript: &str,
    ) -> Result<bool, &'static str> {
        let mut state = self.0.lock().map_err(|_| "Context session lock poisoned")?;
        let Some(session) = state.active.as_mut().filter(|s| {
            s.id == id && id.cancel_generation == generation && s.phase == Phase::Requesting
        }) else {
            return Ok(false);
        };
        session.feedback_text = session
            .context
            .as_ref()
            .and_then(|context| super::feedback::memory_text(context, &prediction, transcript));
        session.prediction = Some(prediction);
        session.phase = Phase::Predicted;
        Ok(true)
    }

    pub(super) fn feedback(
        &self,
        id: SessionId,
    ) -> Result<Option<(ResolvedContext, Prediction)>, &'static str> {
        let state = self.0.lock().map_err(|_| "Context session lock poisoned")?;
        Ok(state
            .active
            .as_ref()
            .filter(|s| s.id == id && s.phase == Phase::Predicted && s.feedback_text.is_some())
            .and_then(|s| Some((s.context.clone()?, s.prediction.clone()?))))
    }

    /// Serialize the one-shot commit with cancellation and newer recordings.
    pub(super) fn commit_feedback(
        &self,
        id: SessionId,
        generation: u64,
        commit: impl FnOnce(&ResolvedContext, &str, &str) -> Result<bool, String>,
    ) -> Result<bool, String> {
        let mut state = self.0.lock().map_err(|_| "Context session lock poisoned")?;
        let Some(session) = state.active.as_mut().filter(|s| {
            s.id == id && id.cancel_generation == generation && s.phase == Phase::Predicted
        }) else {
            return Ok(false);
        };
        session.phase = Phase::FeedbackConsumed;
        match (&session.context, session.feedback_text.take()) {
            (Some(context), Some(text)) => commit(context, &text, &id.serial.to_string()),
            _ => Ok(false),
        }
    }

    pub(crate) fn finish(&self, id: SessionId) -> Result<(), &'static str> {
        let mut state = self.0.lock().map_err(|_| "Context session lock poisoned")?;
        if state.active.as_ref().is_some_and(|s| s.id == id) {
            state.active = None;
        }
        Ok(())
    }

    pub(crate) fn cancel(&self) -> Result<(), &'static str> {
        self.0
            .lock()
            .map_err(|_| "Context session lock poisoned")?
            .active = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(name: &str) -> ResolvedContext {
        ResolvedContext {
            profile_id: name.into(),
            profile_name: name.into(),
            profile_revision: 1,
            icon: "terminal".into(),
            match_basis: MatchBasis::Application,
            prompt: "Rewrite {{transcript}}".into(),
            prompt_source: PromptSource::Inherited,
            prompt_revision: 2,
            dictionary_revision: 3,
            dictionary: vec![DictionaryEntry {
                id: "codex".into(),
                canonical: "Codex".into(),
                misheard_forms: vec!["codecks".into()],
            }],
            memory: vec![],
            memory_epoch: 0,
            input: InputContext {
                application: Captured::Present("terminal".into()),
                workspace: Captured::Unavailable,
                selection: Captured::Empty,
                surrounding_text: Captured::Unavailable,
                caret_utf16: Captured::Unavailable,
                selection_range_utf16: None,
                captured_at_ms: 1,
                truncated: false,
            },
        }
    }

    fn prediction() -> Prediction {
        Prediction {
            text: "Codex".into(),
            operation: TextOperation::Insert,
            effect: FeedbackEffect::None {},
        }
    }

    #[test]
    fn late_resolver_after_cancel_cannot_issue_request() {
        let store = SessionStore::default();
        let old = store.begin(0, None, true).unwrap();
        store.cancel().unwrap();
        assert!(!store.attach_capture(old, 1, context("old").input).unwrap());
        assert!(!store.resolve(old, 1, context("old")).unwrap());
        assert!(store.begin_request(old, 1).unwrap().is_none());
        assert!(!store
            .accept_prediction(old, 1, prediction(), "speech")
            .unwrap());
    }

    #[test]
    fn late_resolver_and_cleanup_cannot_replace_newer_recording() {
        let store = SessionStore::default();
        let old = store.begin(0, None, true).unwrap();
        let current = store.begin(0, None, true).unwrap();
        assert!(!store.attach_capture(old, 0, context("old").input).unwrap());
        assert!(!store.resolve(old, 0, context("old")).unwrap());
        store.finish(old).unwrap();
        assert_eq!(store.current(0).unwrap(), Some(current));
        assert!(store.resolve(current, 0, context("new")).unwrap());
        assert_eq!(
            store.begin_request(current, 0).unwrap().unwrap().profile_id,
            "new"
        );
    }

    #[test]
    fn cancellation_generation_blocks_result_before_cleanup() {
        let store = SessionStore::default();
        let id = store.begin(4, None, true).unwrap();
        assert!(!store.resolve(id, 5, context("general")).unwrap());
        assert!(store.resolve(id, 4, context("general")).unwrap());
        assert!(store.begin_request(id, 5).unwrap().is_none());
        assert!(store.begin_request(id, 4).unwrap().is_some());
        assert!(!store
            .accept_prediction(id, 5, prediction(), "speech")
            .unwrap());
    }

    #[test]
    fn snapshot_is_frozen_and_request_and_prediction_are_single_use() {
        let store = SessionStore::default();
        let id = store.begin(0, None, true).unwrap();
        let mut original = context("general");
        assert!(store.resolve(id, 0, original.clone()).unwrap());
        original.dictionary[0].canonical = "changed".into();
        assert!(!store.resolve(id, 0, original).unwrap());
        let snapshot = store.begin_request(id, 0).unwrap().unwrap();
        assert_eq!(snapshot.dictionary[0].canonical, "Codex");
        assert!(store.begin_request(id, 0).unwrap().is_none());
        assert!(store
            .accept_prediction(id, 0, prediction(), "speech")
            .unwrap());
        assert!(!store
            .accept_prediction(id, 0, prediction(), "speech")
            .unwrap());
    }

    #[test]
    fn private_target_is_not_part_of_serialized_context() {
        let store = SessionStore::default();
        let target = TargetIdentity {
            window: 111,
            focused_control: 222,
            process_id: 333,
            thread_id: 444,
        };
        let id = store.begin(0, Some(target.clone()), true).unwrap();
        assert!(store.target(id).unwrap() == Some(target));
        store.resolve(id, 0, context("general")).unwrap();
        let json = serde_json::to_value(store.begin_request(id, 0).unwrap().unwrap()).unwrap();
        assert!(json.get("target").is_none());
        assert!(json.get("window").is_none());
        assert!(json.get("process_id").is_none());
        assert_eq!(json["input"]["selection"]["availability"], "empty");
        assert_eq!(json["input"]["workspace"]["availability"], "unavailable");
    }

    #[test]
    fn prediction_cannot_assert_profile_or_output_completion() {
        assert!(serde_json::from_str::<Prediction>(
            r#"{"text":"x","operation":"insert","effect":{"type":"none"},"completed":true}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Prediction>(
            r#"{"text":"x","operation":"insert","effect":{"type":"none","profile_id":"other"}}"#
        )
        .is_err());
    }
}
