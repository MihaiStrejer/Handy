//! Terminal TextPattern describes the screen, not the CLI editor's input field.
//! Read only bounded reference text and retain pane/range evidence locally.
use super::{
    capture::{bounded_text, SELECTION_LIMIT, SURROUNDING_LIMIT},
    session::{Captured, InputContext, InputIdentity, SelectionKind},
};
use windows::Win32::{
    System::{Com::SAFEARRAY, Ole::*, Variant::*},
    UI::Accessibility::*,
};

#[derive(Clone, PartialEq, Eq)]
pub(super) struct TerminalText {
    selection: Captured<String>,
    surrounding: Captured<String>,
    identity: InputIdentity,
    truncated: bool,
}

// Every array returned by UIA is owned by the caller, including error paths.
unsafe fn array<T: Copy>(raw: *mut SAFEARRAY, kind: VARENUM, limit: usize) -> Option<Vec<T>> {
    if raw.is_null() {
        return None;
    }
    struct Destroy(*mut SAFEARRAY);
    impl Drop for Destroy {
        fn drop(&mut self) {
            unsafe {
                let _ = SafeArrayDestroy(self.0);
            }
        }
    }
    let _owned = Destroy(raw);
    if SafeArrayGetDim(raw) != 1
        || SafeArrayGetVartype(raw).ok()? != kind
        || SafeArrayGetElemsize(raw) as usize != std::mem::size_of::<T>()
    {
        return None;
    }
    let lower = SafeArrayGetLBound(raw, 1).ok()?;
    let upper = SafeArrayGetUBound(raw, 1).ok()?;
    let count = usize::try_from(i64::from(upper) - i64::from(lower) + 1).ok()?;
    if count > limit {
        return None;
    }
    if count == 0 {
        return Some(vec![]);
    }
    let mut data = std::ptr::null_mut();
    SafeArrayAccessData(raw, &mut data).ok()?;
    struct Unlock(*mut SAFEARRAY);
    impl Drop for Unlock {
        fn drop(&mut self) {
            unsafe {
                let _ = SafeArrayUnaccessData(self.0);
            }
        }
    }
    let _lock = Unlock(raw);
    if data.is_null() {
        return None;
    }
    Some(std::slice::from_raw_parts(data.cast::<T>(), count).to_vec())
}

/// A caller may publish the result only after a matching second snapshot.
pub(super) unsafe fn read(
    pane: &IUIAutomationElement,
    tab: &str,
    console_title: &str,
    window_title: &str,
) -> Option<TerminalText> {
    if !pane.CurrentHasKeyboardFocus().ok()?.as_bool()
        || pane.CurrentIsPassword().ok()?.as_bool()
        || pane.CurrentIsOffscreen().ok()?.as_bool()
    {
        return None;
    }
    let element = array::<i32>(pane.GetRuntimeId().ok()?, VT_I4, 64)?;
    if element.is_empty() {
        return None;
    }
    let pattern = pane
        .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
        .ok()?;
    let ranges = pattern.GetSelection().ok()?;
    if ranges.Length().ok()? != 1 {
        return None;
    }
    let selected = ranges.GetElement(0).ok()?;
    let raw = selected
        .GetText((SELECTION_LIMIT * 2 + 2) as i32)
        .ok()?
        .to_string();
    let (text, selection_truncated) = bounded_text(&raw, SELECTION_LIMIT);
    let rectangles = array::<f64>(selected.GetBoundingRectangles().ok()?, VT_R8, 256)?;
    if rectangles.len() % 4 != 0 || rectangles.iter().any(|value| !value.is_finite()) {
        return None;
    }
    // A highlighted span must be visible and have range evidence. A degenerate
    // screen-cursor range may have no rectangles; it is never an edit caret.
    if !text.is_empty() && rectangles.is_empty() {
        return None;
    }
    let surrounding = selected.Clone().ok()?;
    surrounding.ExpandToEnclosingUnit(TextUnit_Line).ok()?;
    surrounding
        .MoveEndpointByUnit(TextPatternRangeEndpoint_Start, TextUnit_Line, -1)
        .ok()?;
    surrounding
        .MoveEndpointByUnit(TextPatternRangeEndpoint_End, TextUnit_Line, 1)
        .ok()?;
    let raw = surrounding
        .GetText((SURROUNDING_LIMIT * 2 + 2) as i32)
        .ok()?
        .to_string();
    let (context, surrounding_truncated) = bounded_text(&raw, SURROUNDING_LIMIT);
    Some(TerminalText {
        selection: if selection_truncated {
            Captured::Uncertain
        } else if text.is_empty() {
            Captured::Empty
        } else {
            Captured::Present(text)
        },
        surrounding: if context.is_empty() {
            Captured::Empty
        } else {
            Captured::Present(context)
        },
        identity: InputIdentity {
            element,
            tab: tab.into(),
            console_title: console_title.into(),
            window_title: window_title.into(),
            range_rectangles: rectangles.into_iter().map(f64::to_bits).collect(),
        },
        truncated: selection_truncated || surrounding_truncated,
    })
}

pub(super) fn publish(
    input: &mut InputContext,
    before: Option<TerminalText>,
    after: Option<TerminalText>,
) {
    input.selection_kind = SelectionKind::Reference;
    // Screen coordinates cannot authorize replacement or memory readback.
    input.selection_range_utf16 = None;
    input.caret_utf16 = Captured::Unavailable;
    match (before, after) {
        (Some(before), Some(after)) if before == after => {
            input.selection = before.selection;
            input.surrounding_text = before.surrounding;
            input.input_identity = Some(before.identity);
            input.truncated = before.truncated;
        }
        (None, None) => {}
        _ => {
            input.selection = Captured::Uncertain;
            input.surrounding_text = Captured::Uncertain;
            input.input_identity = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> InputContext {
        InputContext {
            provider: Default::default(),
            application: Captured::Unavailable,
            workspace: Captured::Unavailable,
            selection: Captured::Unavailable,
            selection_kind: SelectionKind::Editable,
            surrounding_text: Captured::Unavailable,
            caret_utf16: Captured::Unavailable,
            selection_range_utf16: None,
            input_identity: None,
            captured_at_ms: 0,
            truncated: false,
        }
    }
    fn text() -> TerminalText {
        TerminalText {
            selection: Captured::Present("shell \u{1f600}".into()),
            surrounding: Captured::Present("Buttons are contained to the shell \u{1f600}.".into()),
            identity: InputIdentity {
                element: vec![42, 1, 2],
                tab: "Project".into(),
                console_title: "Console".into(),
                window_title: "Project".into(),
                range_rectangles: vec![
                    10f64.to_bits(),
                    20f64.to_bits(),
                    40f64.to_bits(),
                    15f64.to_bits(),
                ],
            },
            truncated: false,
        }
    }

    #[test]
    fn stable_highlight_is_reference_with_no_edit_range_or_serialized_identity() {
        let mut captured = input();
        publish(&mut captured, Some(text()), Some(text()));
        assert!(captured.selection == Captured::Present("shell \u{1f600}".into()));
        assert!(captured.selection_kind == SelectionKind::Reference);
        assert!(captured.selection_range_utf16.is_none());
        assert!(captured.caret_utf16 == Captured::Unavailable);
        let json = serde_json::to_value(captured).unwrap();
        assert_eq!(json["selection_kind"], "reference");
        assert!(json.get("input_identity").is_none());
        assert!(json.get("selection_range_utf16").is_none());
    }

    #[test]
    fn same_text_in_another_pane_or_at_another_range_is_not_the_same_capture() {
        for change in 0..4 {
            let mut after = text();
            match change {
                0 => after.identity.element[2] += 1,
                1 => after.identity.tab = "Other tab".into(),
                2 => after.identity.range_rectangles[0] = 11f64.to_bits(),
                _ => after.surrounding = Captured::Present("Changed screen".into()),
            }
            let mut captured = input();
            publish(&mut captured, Some(text()), Some(after));
            assert!(captured.selection == Captured::Uncertain);
            assert!(captured.surrounding_text == Captured::Uncertain);
            assert!(captured.input_identity.is_none());
        }
        let mut captured = input();
        publish(&mut captured, Some(text()), None);
        assert!(captured.selection == Captured::Uncertain);
    }

    #[test]
    fn unavailable_terminal_pattern_does_not_invent_an_empty_selection() {
        let mut captured = input();
        publish(&mut captured, None, None);
        assert!(captured.selection == Captured::Unavailable);
        assert!(captured.surrounding_text == Captured::Unavailable);
        assert!(captured.input_identity.is_none());
    }
}
