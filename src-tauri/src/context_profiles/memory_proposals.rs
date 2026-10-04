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

/// Initial delivery supports explicit direct statements. Quoted/reporting,
/// hypothetical and context-only corrections are deliberately not learned.
fn direct_statement(transcript: &str, quote: &str) -> bool {
    let text = normalized(transcript);
    let quote = normalized(quote);
    let starts = [
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
        if !direct_statement(transcript, &p.evidence_quote) {
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
                if record.wrong.as_deref().map(normalized) != p.wrong.as_deref().map(normalized)
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
