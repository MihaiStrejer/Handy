//! A model proposes memory; only a verified local field change admits it.
use super::session::{
    Captured, FeedbackEffect, InputContext, Prediction, ResolvedContext, TextOperation,
};
use super::{CaptureService, SessionId, SessionStore};
use tauri::{Emitter, Manager};

fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

// Match words/phrases, not an accidental substring inside another word.
fn contains_phrase(text: &str, phrase: &str) -> bool {
    let text = normalize(text);
    let phrase = normalize(phrase);
    !phrase.is_empty()
        && text.match_indices(&phrase).any(|(at, _)| {
            let end = at + phrase.len();
            !text[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
                && !text[end..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_')
        })
}

pub(super) fn memory_text(
    context: &ResolvedContext,
    prediction: &Prediction,
    transcript: &str,
) -> Option<String> {
    match &prediction.effect {
        FeedbackEffect::None {} => None,
        // New corrections need not have a long-term dictionary entry. The
        // endpoint is instructed to propose only explicit user corrections.
        FeedbackEffect::Remember { text } => {
            (!text.trim().is_empty() && text.chars().count() <= 500).then(|| text.trim().to_owned())
        }
        FeedbackEffect::AddMisheardForm { keyword_id, phrase } => {
            let keyword = context.dictionary.iter().find(|k| &k.id == keyword_id)?;
            let normalized = normalize(phrase);
            if normalized.is_empty()
                || phrase.chars().count() > 120
                || normalized == normalize(&keyword.canonical)
                || !contains_phrase(&prediction.text, &keyword.canonical)
                || !(contains_phrase(transcript, phrase)
                    || matches!(&context.input.selection, Captured::Present(text) if contains_phrase(text, phrase)))
                || context.dictionary.iter().any(|k| {
                    k.id != *keyword_id
                        && (normalize(&k.canonical) == normalized
                            || k.misheard_forms
                                .iter()
                                .any(|form| normalize(form) == normalized))
                })
            {
                return None;
            }
            // The legacy effect remains accepted, but never mutates a keyword.
            Some(format!(
                "Interpret {} as {} in this conversation.",
                serde_json::to_string(phrase.trim()).ok()?,
                serde_json::to_string(&keyword.canonical).ok()?
            ))
            .filter(|text| text.chars().count() <= 500)
        }
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
        return Ok(());
    };
    let Some(target) = store.target(id)? else {
        return Ok(());
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1000);
    while std::time::Instant::now() < deadline {
        if store.current(audio.cancel_generation())? != Some(id)
            || super::capture_target().as_ref() != Some(&target)
        {
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
            let added =
                store.commit_feedback(id, audio.cancel_generation(), |context, text, source| {
                    super::storage::admit_memory(app, context, text, source)
                })?;
            if added {
                // Only an ID crosses the event bus; the UI fetches current memory.
                app.emit("profile-memory-updated", &context.profile_id)
                    .map_err(|_| "Could not refresh profile memory")?;
            }
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
    }
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
            effect: FeedbackEffect::Remember {
                text: "Use BOM (bill of materials) when bomb refers to this term.".into(),
            },
        }
    }

    #[test]
    fn explicit_correction_without_dictionary_returns_text_and_memory() {
        let context = context("Review the bomb today", 11, 15);
        assert!(context.dictionary.is_empty());
        assert_eq!(correction().text, "BOM");
        assert!(memory_text(
            &context,
            &correction(),
            "thats not bomb its BOM from bill of materials"
        )
        .unwrap()
        .contains("bill of materials"));
        let mut ordinary = correction();
        ordinary.effect = FeedbackEffect::None {};
        assert!(memory_text(&context, &ordinary, "change my mind").is_none());
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
    fn legacy_mishearing_requires_existing_unambiguous_observed_keyword() {
        use super::super::session::DictionaryEntry;
        let mut context = context("bomb", 0, 4);
        context.dictionary.push(DictionaryEntry {
            id: "bom".into(),
            canonical: "BOM".into(),
            misheard_forms: vec![],
        });
        let mut prediction = correction();
        prediction.effect = FeedbackEffect::AddMisheardForm {
            keyword_id: "bom".into(),
            phrase: "bomb".into(),
        };
        assert!(memory_text(&context, &prediction, "correct it").is_some());
        prediction.text = "BOMBER".into();
        assert!(memory_text(&context, &prediction, "correct it").is_none());
        prediction.text = "BOM".into();
        context.input.selection = Captured::Empty;
        assert!(memory_text(&context, &prediction, "bombard").is_none());
        assert!(memory_text(&context, &prediction, "that's not bomb").is_some());
        context.dictionary.push(DictionaryEntry {
            id: "other".into(),
            canonical: "Other".into(),
            misheard_forms: vec!["bomb".into()],
        });
        assert!(memory_text(&context, &prediction, "bomb").is_none());
        context.dictionary.clear();
        assert!(memory_text(&context, &prediction, "bomb").is_none());
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
            .commit_feedback(id, 0, |c, t, s| Ok(memory.admit(
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
