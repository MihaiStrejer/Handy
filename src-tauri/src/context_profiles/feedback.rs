//! A model proposes memory; only a verified local field change admits it.
#[cfg(test)]
use super::session::ResolvedContext;
use super::session::{Captured, InputContext, Prediction, TextOperation};
use super::{CaptureService, SessionId, SessionStore};
use tauri::{Emitter, Manager};

pub(crate) fn skip_pending(app: &tauri::AppHandle, id: SessionId, reason: &str) {
    if let Ok(Some((context, _))) = app.state::<SessionStore>().feedback(id) {
        super::storage::report_memory_skip(app, &context.profile_id, reason);
    }
}

fn field(input: &InputContext) -> Option<&str> {
    if input.truncated {
        return None;
    }
    match &input.surrounding_text {
        Captured::Present(text) => Some(text),
        Captured::Empty => Some(""),
        _ => None,
    }
}

pub(super) struct ExpectedChange {
    text: String,
    caret: u32,
}

impl ExpectedChange {
    pub(super) fn new(
        input: &InputContext,
        prediction: &Prediction,
        inserted: &str,
    ) -> Option<Self> {
        let original = field(input)?;
        let raw: Vec<u16> = original.encode_utf16().collect();
        let (start, end) = input.selection_range_utf16?;
        if start > end || end as usize > raw.len() {
            return None;
        }
        let selected = String::from_utf16(&raw[start as usize..end as usize]).ok()?;
        match (&prediction.operation, &input.selection) {
            (TextOperation::ReplaceSelection, Captured::Present(text))
                if !text.is_empty() && *text == selected => {}
            (TextOperation::Insert, Captured::Empty)
                if start == end && input.caret_utf16 == Captured::Present(start) => {}
            _ => return None,
        }
        // Both boundaries must be valid UTF-16 positions, including empty selections.
        let prefix = String::from_utf16(&raw[..start as usize]).ok()?;
        let suffix = String::from_utf16(&raw[end as usize..]).ok()?;
        let text = format!("{prefix}{inserted}{suffix}");
        if text == original {
            return None;
        }
        let caret = start.checked_add(u32::try_from(inserted.encode_utf16().count()).ok()?)?;
        Some(Self { text, caret })
    }

    pub(super) fn matches(&self, actual: &InputContext) -> bool {
        field(actual) == Some(self.text.as_str())
            && actual.selection == Captured::Empty
            && actual.caret_utf16 == Captured::Present(self.caret)
            && actual.selection_range_utf16 == Some((self.caret, self.caret))
    }
}

/// Runs on a blocking worker after successful dispatch. The caller retains the
/// session lifetime guard until this returns. Unsupported readback never learns.
pub(crate) fn verify_and_remember(
    app: &tauri::AppHandle,
    id: SessionId,
    inserted: &str,
    audio: &crate::managers::audio::AudioRecordingManager,
) -> Result<(), String> {
    let store = app.state::<SessionStore>();
    let Some((context, prediction)) = store.feedback(id)? else {
        return Ok(());
    };
    let Some(expected) = ExpectedChange::new(&context.input, &prediction, inserted) else {
        super::storage::report_memory_skip(app, &context.profile_id, "readback_unavailable");
        return Ok(());
    };
    let Some(target) = store.target(id)? else {
        super::storage::report_memory_skip(app, &context.profile_id, "readback_unavailable");
        return Ok(());
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1000);
    while std::time::Instant::now() < deadline {
        if store.current(audio.cancel_generation())? != Some(id)
            || super::capture_target().as_ref() != Some(&target)
        {
            super::storage::report_memory_skip(app, &context.profile_id, "session_changed");
            return Ok(());
        }
        let actual = app
            .state::<CaptureService>()
            .request(Some(target.clone()))
            .wait();
        if expected.matches(&actual) {
            if super::capture_target().as_ref() != Some(&target) {
                return Ok(());
            }
            let added = store.commit_feedback(
                id,
                audio.cancel_generation(),
                |context, batch, source| super::storage::admit_memory(app, context, batch, source),
            )?;
            if added {
                // Only an ID crosses the event bus; the UI fetches current memory.
                app.emit("profile-memory-updated", &context.profile_id)
                    .map_err(|_| "Could not refresh profile memory")?;
            } else {
                super::storage::report_memory_skip(app, &context.profile_id, "context_changed");
            }
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
    }
    super::storage::report_memory_skip(app, &context.profile_id, "readback_failed");
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::{
        routing,
        storage::{MemoryState, ProfileCatalog, ProfileSnapshot},
    };
    use super::*;
    use std::{collections::HashMap, sync::Arc};

    pub(in crate::context_profiles) fn context(
        text: &str,
        start: u32,
        end: u32,
    ) -> ResolvedContext {
        let raw: Vec<u16> = text.encode_utf16().collect();
        let input = InputContext {
            provider: crate::context_profiles::providers::ProviderContext::default(),
            application: Captured::Present("test.exe".into()),
            workspace: Captured::Unavailable,
            selection: if start == end {
                Captured::Empty
            } else {
                Captured::Present(String::from_utf16(&raw[start as usize..end as usize]).unwrap())
            },
            surrounding_text: if text.is_empty() {
                Captured::Empty
            } else {
                Captured::Present(text.into())
            },
            caret_utf16: if start == end {
                Captured::Present(start)
            } else {
                Captured::Unavailable
            },
            selection_range_utf16: Some((start, end)),
            captured_at_ms: 1,
            truncated: false,
        };
        routing::resolve(
            &ProfileSnapshot {
                catalog: Arc::new(ProfileCatalog::seed(
                    &crate::settings::get_default_settings(),
                )),
                memory: HashMap::new(),
                memory_epochs: HashMap::new(),
            },
            input,
        )
        .unwrap()
    }

    pub(in crate::context_profiles) fn correction() -> Prediction {
        Prediction {
            text: "BOM".into(),
            operation: TextOperation::ReplaceSelection,
            memory_changes: vec![super::super::memory_proposals::correction()],
            memory_skip_reason: None,
        }
    }

    #[test]
    fn explicit_correction_requires_grounded_transcript() {
        let context = context("Review the bomb today", 11, 15);
        assert!(super::super::memory_proposals::validate(
            &context,
            "not bomb, BOM",
            &correction().memory_changes
        )
        .is_ok());
        assert!(super::super::memory_proposals::validate(
            &context,
            "ordinary rewriting",
            &correction().memory_changes
        )
        .is_err());
    }

    #[test]
    fn exact_readback_requires_changed_field_caret_and_full_context() {
        let before = context("Review the bomb today", 11, 15);
        let expected = ExpectedChange::new(&before.input, &correction(), "BOM").unwrap();
        assert!(!expected.matches(&before.input));
        let after = context("Review the BOM today", 14, 14);
        assert!(expected.matches(&after.input));
        for mut bad in [
            context("BOM", 3, 3).input,
            context("Review the BOM today!", 14, 14).input,
            context("Review the BOM today", 0, 0).input,
            context("Review the BOM today", 11, 14).input,
        ] {
            assert!(!expected.matches(&bad));
            bad.truncated = true;
            assert!(!expected.matches(&bad));
        }
        let mut bad = before.input.clone();
        bad.selection_range_utf16 = None;
        assert!(ExpectedChange::new(&bad, &correction(), "BOM").is_none());
        bad = before.input.clone();
        bad.truncated = true;
        assert!(ExpectedChange::new(&bad, &correction(), "BOM").is_none());
        assert!(ExpectedChange::new(&before.input, &correction(), "bomb").is_none());
    }

    #[test]
    fn utf16_repeated_selection_and_trailing_space_are_exact() {
        let before = context("😀 bomb bomb", 8, 12);
        let expected = ExpectedChange::new(&before.input, &correction(), "BOM ").unwrap();
        assert!(expected.matches(&context("😀 bomb BOM ", 12, 12).input));
        assert!(!expected.matches(&context("😀 BOM  bomb", 7, 7).input));
        let mut bad = before.input;
        bad.selection_range_utf16 = Some((1, 2));
        assert!(ExpectedChange::new(&bad, &correction(), "BOM").is_none());
    }

    #[test]
    fn insertion_can_learn_only_after_verified_caret_insertion() {
        let mut prediction = correction();
        prediction.operation = TextOperation::Insert;
        let before = context("", 0, 0);
        let expected = ExpectedChange::new(&before.input, &prediction, "BOM").unwrap();
        assert!(expected.matches(&context("BOM", 3, 3).input));
        let mut bad = before.input;
        bad.caret_utf16 = Captured::Unavailable;
        assert!(ExpectedChange::new(&bad, &prediction, "BOM").is_none());
    }

    #[test]
    fn explicit_statement_without_readback_cannot_establish_completion() {
        let mut before = context("bomb", 0, 4);
        let prediction = correction();
        assert!(super::super::memory_proposals::validate(
            &before,
            "not bomb, BOM",
            &prediction.memory_changes
        )
        .is_ok());
        before.input.surrounding_text = Captured::Unavailable;
        assert!(ExpectedChange::new(&before.input, &prediction, "BOM").is_none());
    }

    #[test]
    fn cancelled_stale_duplicate_or_unaccepted_prediction_cannot_commit() {
        let store = SessionStore::default();
        let id = store.begin(0, None, true).unwrap();
        assert!(!store
            .commit_feedback(id, 0, |_, _, _| panic!("unaccepted"))
            .unwrap());
        store.resolve(id, 0, context("bomb", 0, 4)).unwrap();
        store.begin_request(id, 0).unwrap();
        store
            .accept_prediction(id, 0, correction(), "not bomb, BOM")
            .unwrap();
        assert!(!store
            .commit_feedback(id, 1, |_, _, _| panic!("cancel generation"))
            .unwrap());
        let mut memory = MemoryState::default();
        assert!(store
            .commit_feedback(id, 0, |c, t, s| Ok(memory.admit_batch(
                &c.profile_id,
                c.memory_epoch,
                t,
                s
            )))
            .unwrap());
        assert!(!store
            .commit_feedback(id, 0, |_, _, _| panic!("duplicate"))
            .unwrap());
        let newer = store.begin(0, None, true).unwrap();
        assert!(!store
            .commit_feedback(id, 0, |_, _, _| panic!("stale"))
            .unwrap());
        store.resolve(newer, 0, context("bomb", 0, 4)).unwrap();
        store.begin_request(newer, 0).unwrap();
        store
            .accept_prediction(newer, 0, correction(), "not bomb, BOM")
            .unwrap();
        store.cancel().unwrap();
        assert!(!store
            .commit_feedback(newer, 0, |_, _, _| panic!("cancelled"))
            .unwrap());
        assert_eq!(memory.items["general"].len(), 1);
    }
}
