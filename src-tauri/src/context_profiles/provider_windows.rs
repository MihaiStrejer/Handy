use super::{
    providers::{metadata, InputMetadata},
    session::{Captured, InputContext},
    target::TargetIdentity,
};
use windows::{
    core::*,
    Win32::{
        Foundation::HWND,
        System::{Com::*, Variant::VARIANT},
        UI::{Accessibility::*, WindowsAndMessaging::*},
    },
};

pub(super) struct Source {
    pub target: TargetIdentity,
    automation: IUIAutomation,
    root: IUIAutomationElement,
}

pub(super) fn window_class(hwnd: usize) -> String {
    let mut buffer = [0u16; 256];
    let length = unsafe { GetClassNameW(HWND(hwnd as *mut _), &mut buffer) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}
pub(super) fn window_title(hwnd: usize) -> String {
    let mut buffer = [0u16; 514];
    let length = unsafe { GetWindowTextW(HWND(hwnd as *mut _), &mut buffer) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

impl Source {
    /// Caller owns COM initialization and checks the frozen native focus.
    pub(super) unsafe fn new(target: TargetIdentity) -> Result<Self> {
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
        let root = automation.ElementFromHandle(HWND(target.window as *mut _))?;
        Ok(Self {
            target,
            automation,
            root,
        })
    }
    fn elements(
        &self,
        root: &IUIAutomationElement,
        scope: TreeScope,
        kind: UIA_CONTROLTYPE_ID,
    ) -> Result<Vec<IUIAutomationElement>> {
        unsafe {
            let condition = self
                .automation
                .CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(kind.0))?;
            let array = root.FindAll(scope, &condition)?;
            let count = array.Length()?;
            if count > 64 {
                return Ok(vec![]);
            }
            (0..count).map(|i| array.GetElement(i)).collect()
        }
    }
    pub(super) fn t3(&self, input: &mut InputContext) -> Result<()> {
        unsafe {
            // Request the normal accessibility object, which initializes the
            // renderer tree on Electron. No setting, input or clipboard write.
            let mut accessible = std::ptr::null_mut();
            if AccessibleObjectFromWindow(
                HWND(self.target.focused_control as *mut _),
                OBJID_CLIENT.0 as u32,
                &IAccessible::IID,
                &mut accessible,
            )
            .is_ok()
                && !accessible.is_null()
            {
                drop(IUnknown::from_raw(accessible));
            }
            let mut candidates = vec![];
            for list in self.elements(&self.root, TreeScope_Descendants, UIA_ListControlTypeId)? {
                if list.CurrentIsOffscreen()?.as_bool() {
                    continue;
                }
                let items = self.elements(&list, TreeScope_Children, UIA_ListItemControlTypeId)?;
                if items.len() != 2 {
                    continue;
                }
                let project = items[0].CurrentName()?.to_string();
                let conversation = items[1].CurrentName()?.to_string();
                if project.is_empty() || conversation.is_empty() {
                    continue;
                }
                // Breadcrumb child commands corroborate its role. A chat list
                // mentioning a project cannot masquerade as this breadcrumb.
                let project_buttons =
                    self.elements(&items[0], TreeScope_Descendants, UIA_ButtonControlTypeId)?;
                let thread_buttons =
                    self.elements(&items[1], TreeScope_Descendants, UIA_ButtonControlTypeId)?;
                let project_ok = project_buttons.iter().any(|b| {
                    b.CurrentName()
                        .is_ok_and(|n| n.to_string() == format!("New thread in {project}"))
                });
                let thread_ok = thread_buttons.iter().any(|b| {
                    b.CurrentName().is_ok_and(|n| {
                        n.to_string() == format!("Thread actions for {conversation}")
                    })
                });
                if project_ok && thread_ok {
                    candidates.push((project, conversation, items[0].clone(), items[1].clone()));
                }
            }
            if let [(project, conversation, _, _)] = candidates.as_slice() {
                input.provider.project = metadata(project);
                input.provider.conversation = metadata(conversation);
            }
            let focused = self.automation.GetFocusedElement()?;
            if focused.CurrentProcessId()? as u32 == self.target.process_id
                && !focused.CurrentIsOffscreen()?.as_bool()
            {
                if focused.CurrentIsPassword()?.as_bool() {
                    input.provider.input = Captured::Protected;
                    input.selection = Captured::Protected;
                    input.surrounding_text = Captured::Protected;
                    return Ok(());
                }
                if focused.CurrentControlType()? == UIA_EditControlTypeId {
                    let name = focused.CurrentName()?.to_string();
                    let value = focused
                        .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId);
                    let editable = value
                        .as_ref()
                        .is_ok_and(|p| p.CurrentIsReadOnly().is_ok_and(|r| !r.as_bool()));
                    if name.chars().count() <= 512 {
                        input.provider.input = Captured::Present(InputMetadata {
                            name,
                            role: "edit".into(),
                            editable,
                            text_pattern: focused
                                .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
                                .is_ok(),
                        });
                    }
                }
            }
            // A virtual browser element is metadata only until full local
            // identity, exact field extent and readback are implemented.
            let mut branches = vec![];
            for combo in
                self.elements(&self.root, TreeScope_Descendants, UIA_ComboBoxControlTypeId)?
            {
                if combo.CurrentIsOffscreen()?.as_bool() {
                    continue;
                }
                // Current T3 composer branch control exposes this class marker;
                // unsupported UI versions return unavailable rather than guess.
                if !combo
                    .CurrentClassName()?
                    .to_string()
                    .contains("active:scale-100")
                {
                    continue;
                }
                let text = self.elements(&combo, TreeScope_Children, UIA_TextControlTypeId)?;
                if let [text] = text.as_slice() {
                    branches.push(text.CurrentName()?.to_string());
                }
            }
            if let [branch] = branches.as_slice() {
                input.provider.branch = metadata(branch);
            }
            if let [(project, conversation, project_element, conversation_element)] =
                candidates.as_slice()
            {
                if project_element.CurrentName()?.to_string() != *project
                    || conversation_element.CurrentName()?.to_string() != *conversation
                {
                    input.provider.project = Captured::Uncertain;
                    input.provider.conversation = Captured::Uncertain;
                    input.provider.branch = Captured::Uncertain;
                }
            }
            // Directory labels are not path evidence. A future T3 integration
            // can supply verified local/remote workspace without changing routing.
        }
        Ok(())
    }
    fn selected_tab(&self) -> Result<Option<String>> {
        unsafe {
            let mut tabs = vec![];
            for tab in self.elements(&self.root, TreeScope_Descendants, UIA_TabItemControlTypeId)? {
                if let Ok(pattern) = tab.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                    UIA_SelectionItemPatternId,
                ) {
                    if pattern.CurrentIsSelected()?.as_bool() {
                        tabs.push(tab.CurrentName()?.to_string());
                    }
                }
            }
            Ok(if let [tab] = tabs.as_slice() {
                Some(tab.clone())
            } else if window_class(self.target.window) == "ConsoleWindowClass" {
                Some(window_title(self.target.window))
            } else {
                None
            })
        }
    }
    pub(super) fn terminal(&self, input: &mut InputContext) -> Result<()> {
        if let Some(tab) = self.selected_tab()? {
            input.provider.terminal_tab = metadata(&tab);
            if let Some(directory) = super::terminal_process::workspace(&self.target, &tab) {
                input.workspace = Captured::Present(directory);
                input.provider.workspace_source =
                    Captured::Present("console_owner_selected_tab_process_cwd".into());
            }
            // Recheck selected tab after the helper; a same-HWND tab switch
            // must not attach a directory from the previous tab.
            if self.selected_tab()? != Some(tab) {
                input.workspace = Captured::Uncertain;
                input.provider.workspace_source = Captured::Uncertain;
            }
        }
        Ok(())
    }
}
