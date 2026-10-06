//! Transport-independent correction proposals. Source data grants no authority.
use super::session::ResolvedContext;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum MemoryAction {
    Add,
    Replace,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MemoryProposal {
    pub action: MemoryAction,
    pub text: String,
    pub evidence_quote: String,
    pub target_id: Option<String>,
    pub expected_revision: Option<u32>,
    pub wrong: Option<String>,
    pub corrected: Option<String>,
    pub scope: Option<String>,
}

pub(super) fn normalized(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn bounded(text: &str) -> bool {
    !text.trim().is_empty() && text.chars().count() <= 500
}

fn phrase(text: &str, term: &str) -> bool {
    text.match_indices(term).any(|(start, _)| {
        let end = start + term.len();
        !text[..start]
            .chars()
            .next_back()
            .is_some_and(char::is_alphanumeric)
            && !text[end..]
                .chars()
                .next()
                .is_some_and(char::is_alphanumeric)
    })
}

/// True when an existing (wrong, corrected) pair is the reverse of a proposed
/// pair. Accepting both would leave contradictory notes side by side.
pub(super) fn inverse_terms(
    existing_wrong: Option<&str>,
    existing_corrected: Option<&str>,
    wrong: &str,
    corrected: &str,
) -> bool {
    existing_wrong.map(normalized) == Some(normalized(corrected))
        && existing_corrected.map(normalized) == Some(normalized(wrong))
        && existing_corrected != Some(corrected)
}

/// Initial delivery supports explicit direct statements. Quoted/reporting,
/// hypothetical and context-only corrections are deliberately not learned.
/// Free-text notes (no term pair) require an explicit memory or usage
/// instruction; negation and filler openers such as "no", "not" and
/// "actually" only count for a quoted term pair.
fn direct_statement(transcript: &str, quote: &str, term_pair: bool) -> bool {
    let text = normalized(transcript);
    let quote = normalized(quote);
    let free_text_starts = [
        "remember ",
        "when i say ",
        "i mean ",
        "i meant ",
        "use ",
        "verwende ",
        "folosește ",
        "foloseste ",
        "utilise ",
        "usa ",
        "请记住",
        "覚えて",
    ];
    let term_starts = [
        "that's not ",
        "thats not ",
        "not ",
        "when i say ",
        "i mean ",
        "i meant ",
        "remember ",
        "use ",
        "actually ",
        "correct ",
        "nicht ",
        "verwende ",
        "nu ",
        "folosește ",
        "foloseste ",
        "pas ",
        "utilise ",
        "no ",
        "usa ",
        "不是",
        "请记住",
        "覚えて",
    ];
    let starts: &[&str] = if term_pair {
        &term_starts[..]
    } else {
        &free_text_starts[..]
    };
    starts
        .iter()
        .any(|start| text.starts_with(start) && quote.starts_with(start))
        && ![
            "do not remember",
            "don't remember",
            "do not use",
            "don't use",
            "hypothetically",
            "for example",
            "he said",
            "she said",
            "quote",
            "if i ",
            "ignore previous",
            "never remember",
            "change my mind",
            "for this message",
            "for this draft",
            "this time",
            "only today",
            "scratch that",
            "never mind",
            "nevermind",
            "forget that",
            "forget it",
            "wait, no",
        ]
        .iter()
        .any(|bad| text.contains(bad))
        && !transcript.contains(['"', '“', '”', '«', '»'])
}

pub(super) fn validate(
    context: &ResolvedContext,
    transcript: &str,
    proposals: &[MemoryProposal],
) -> Result<(), &'static str> {
    if proposals.len() > 4 {
        return Err("too_many_changes");
    }
    let mut targets = HashSet::new();
    let mut terms = HashSet::new();
    let mut texts = HashSet::new();
    for p in proposals {
        if !bounded(&p.text)
            || !bounded(&p.evidence_quote)
            || !transcript.contains(&p.evidence_quote)
        {
            return Err("ungrounded_evidence");
        }
        if !direct_statement(transcript, &p.evidence_quote, p.wrong.is_some()) {
            return Err("unsupported_evidence");
        }
        if !texts.insert(normalized(&p.text)) {
            return Err("duplicate_change");
        }
        if let Some(scope) = &p.scope {
            if !bounded(scope) || !p.evidence_quote.contains(scope) || !p.text.contains(scope) {
                return Err("ungrounded_scope");
            }
        }
        match (&p.wrong, &p.corrected) {
            (Some(wrong), Some(corrected)) => {
                if !bounded(wrong)
                    || !bounded(corrected)
                    || wrong == corrected
                    || !phrase(&p.evidence_quote, wrong)
                    || !phrase(&p.evidence_quote, corrected)
                    || !p.text.contains(wrong)
                    || !p.text.contains(corrected)
                {
                    return Err("ungrounded_terms");
                }
                let key = (normalized(wrong), p.scope.as_deref().map(normalized));
                if !terms.insert(key.clone()) {
                    return Err("conflicting_changes");
                }
                if context.memory.iter().any(|r| {
                    r.wrong.as_deref().map(normalized) == Some(key.0.clone())
                        && r.scope.as_deref().map(normalized) == key.1
                        && r.corrected.as_deref() != Some(corrected)
                        && p.target_id.as_deref() != Some(&r.id)
                }) {
                    return Err("conflicting_changes");
                }
                // A reversed pair (bomb->BOM vs BOM->bomb) is the same
                // contradiction; only a replace of that record may resolve it.
                if context.memory.iter().any(|r| {
                    inverse_terms(r.wrong.as_deref(), r.corrected.as_deref(), wrong, corrected)
                        && r.scope.as_deref().map(normalized) == key.1
                        && p.target_id.as_deref() != Some(&r.id)
                }) {
                    return Err("conflicting_changes");
                }
            }
            (None, None) if p.text.trim() == p.evidence_quote.trim() => {}
            _ => return Err("ungrounded_terms"),
        }
        match p.action {
            MemoryAction::Add if p.target_id.is_none() && p.expected_revision.is_none() => {}
            MemoryAction::Replace => {
                let id = p.target_id.as_deref().ok_or("unknown_target")?;
                let record = context
                    .memory
                    .iter()
                    .find(|m| m.id == id)
                    .ok_or("unknown_target")?;
                if p.expected_revision != Some(record.revision) || !targets.insert(id) {
                    return Err("stale_target");
                }
                let same_term =
                    record.wrong.as_deref().map(normalized) == p.wrong.as_deref().map(normalized);
                let reversed_term = match (&p.wrong, &p.corrected) {
                    (Some(wrong), Some(corrected)) => inverse_terms(
                        record.wrong.as_deref(),
                        record.corrected.as_deref(),
                        wrong,
                        corrected,
                    ),
                    _ => false,
                };
                if !(same_term || reversed_term)
                    || record.scope.as_deref().map(normalized) != p.scope.as_deref().map(normalized)
                {
                    return Err("conflicting_changes");
                }
            }
            _ => return Err("invalid_target"),
        }
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn correction() -> MemoryProposal {
    MemoryProposal {
        action: MemoryAction::Add,
        text: "Use BOM when bomb refers to this term.".into(),
        evidence_quote: "not bomb, BOM".into(),
        target_id: None,
        expected_revision: None,
        wrong: Some("bomb".into()),
        corrected: Some("BOM".into()),
        scope: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_guesses_quotations_negation_and_ungrounded_scope() {
        let context = super::super::feedback::tests::context("bomb", 0, 4);
        let proposal = correction();
        assert!(validate(&context, "not bomb, BOM", &[proposal.clone()]).is_ok());
        for text in [
            "He said not bomb, BOM",
            "Do not remember not bomb, BOM",
            "Not bomb, BOM hypothetically",
            "Please polish this text",
        ] {
            assert!(validate(&context, text, &[proposal.clone()]).is_err());
        }
        let mut scoped = proposal;
        scoped.scope = Some("in accounting".into());
        assert_eq!(
            validate(&context, "not bomb, BOM", &[scoped]),
            Err("ungrounded_scope")
        );
    }

    fn free_text(text: &str) -> MemoryProposal {
        MemoryProposal {
            action: MemoryAction::Add,
            text: text.into(),
            evidence_quote: text.into(),
            target_id: None,
            expected_revision: None,
            wrong: None,
            corrected: None,
            scope: None,
        }
    }

    #[test]
    fn free_text_requires_explicit_memory_instruction_and_retractions_block() {
        let context = super::super::feedback::tests::context("bomb", 0, 4);
        let remember = "Remember project Atlas uses British spelling.";
        assert!(validate(&context, remember, &[free_text(remember)]).is_ok());
        for text in [
            "No worries, I'll send it tomorrow.",
            "Not the final version yet.",
            "Actually make it shorter.",
            "Correct the second paragraph.",
        ] {
            assert_eq!(
                validate(&context, text, &[free_text(text)]),
                Err("unsupported_evidence")
            );
        }
        let mut retracted = correction();
        retracted.evidence_quote = "Not bomb, BOM".into();
        assert_eq!(
            validate(
                &context,
                "Not bomb, BOM. Scratch that, it is bomb.",
                &[retracted]
            ),
            Err("unsupported_evidence")
        );
    }

    #[test]
    fn reversed_term_pair_is_a_contradiction_unless_it_replaces_the_record() {
        let mut context = super::super::feedback::tests::context("bomb", 0, 4);
        context.memory.push(super::super::session::MemoryItem {
            id: "1".into(),
            revision: 3,
            text: "Use BOM when bomb refers to this term.".into(),
            wrong: Some("bomb".into()),
            corrected: Some("BOM".into()),
            scope: None,
        });
        let mut reverse = correction();
        reverse.text = "Use bomb when BOM refers to this term.".into();
        reverse.evidence_quote = "not BOM, bomb".into();
        reverse.wrong = Some("BOM".into());
        reverse.corrected = Some("bomb".into());
        assert_eq!(
            validate(&context, "not BOM, bomb", &[reverse.clone()]),
            Err("conflicting_changes")
        );
        reverse.action = MemoryAction::Replace;
        reverse.target_id = Some("1".into());
        reverse.expected_revision = Some(3);
        assert!(validate(&context, "not BOM, bomb", &[reverse.clone()]).is_ok());
        // A differently scoped reverse note is not the same contradiction.
        let mut scoped = correction();
        scoped.text = "Use bomb when BOM refers to this term in accounting.".into();
        scoped.evidence_quote = "not BOM, bomb in accounting".into();
        scoped.wrong = Some("BOM".into());
        scoped.corrected = Some("bomb".into());
        scoped.scope = Some("in accounting".into());
        assert!(validate(&context, "not BOM, bomb in accounting", &[scoped]).is_ok());
    }

    #[test]
    fn bounded_batches_require_known_revision_and_literal_terms() {
        let mut context = super::super::feedback::tests::context("bomb", 0, 4);
        let mut p = correction();
        p.action = MemoryAction::Replace;
        p.target_id = Some("1".into());
        p.expected_revision = Some(1);
        assert_eq!(
            validate(&context, "not bomb, BOM", &[p.clone()]),
            Err("unknown_target")
        );
        context.memory.push(super::super::session::MemoryItem {
            id: "1".into(),
            revision: 2,
            text: "Old rule".into(),
            wrong: p.wrong.clone(),
            corrected: Some("old".into()),
            scope: None,
        });
        assert_eq!(
            validate(&context, "not bomb, BOM", &[p.clone()]),
            Err("stale_target")
        );
        p.expected_revision = Some(2);
        assert!(validate(&context, "not bomb, BOM", &[p.clone()]).is_ok());
        assert_eq!(
            validate(&context, "not bomb, BOM", &vec![p; 5]),
            Err("too_many_changes")
        );
        let mut bad = correction();
        bad.corrected = Some("Invented".into());
        assert_eq!(
            validate(&context, "not bomb, BOM", &[bad]),
            Err("ungrounded_terms")
        );
    }
}
