//! Application extractors share one bounded snapshot contract and registry.
use super::session::Captured;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub(super) enum ProviderId {
    T3Code,
    WindowsTerminal,
    #[default]
    Default,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct InputMetadata {
    pub name: String,
    pub role: String,
    pub editable: bool,
    pub text_pattern: bool,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ProviderContext {
    pub id: ProviderId,
    pub window_title: Captured<String>,
    pub project: Captured<String>,
    pub conversation: Captured<String>,
    pub branch: Captured<String>,
    pub terminal_tab: Captured<String>,
    pub workspace_source: Captured<String>,
    pub input: Captured<InputMetadata>,
}

impl Default for ProviderContext {
    fn default() -> Self {
        Self {
            id: ProviderId::Default,
            window_title: Captured::Unavailable,
            project: Captured::Unavailable,
            conversation: Captured::Unavailable,
            branch: Captured::Unavailable,
            terminal_tab: Captured::Unavailable,
            workspace_source: Captured::Unavailable,
            input: Captured::Unavailable,
        }
    }
}

pub(super) trait ContextProvider: Sync {
    fn id(&self) -> ProviderId;
    fn matches(&self, application: &str, window_class: &str) -> bool;
    #[cfg(target_os = "windows")]
    fn extract(
        &self,
        source: &super::provider_windows::Source,
        input: &mut super::session::InputContext,
    ) -> windows::core::Result<()>;
}

struct T3Code;
struct Terminal;
struct DefaultProvider;

impl ContextProvider for T3Code {
    fn id(&self) -> ProviderId {
        ProviderId::T3Code
    }
    fn matches(&self, application: &str, class: &str) -> bool {
        ["T3 Code.exe", "T3 Code (Nightly).exe", "t3code.exe"]
            .iter()
            .any(|name| application.eq_ignore_ascii_case(name))
            && class == "Chrome_WidgetWin_1"
    }
    #[cfg(target_os = "windows")]
    fn extract(
        &self,
        source: &super::provider_windows::Source,
        input: &mut super::session::InputContext,
    ) -> windows::core::Result<()> {
        source.t3(input)
    }
}
impl ContextProvider for Terminal {
    fn id(&self) -> ProviderId {
        ProviderId::WindowsTerminal
    }
    fn matches(&self, application: &str, class: &str) -> bool {
        (application.eq_ignore_ascii_case("WindowsTerminal.exe")
            && class == "CASCADIA_HOSTING_WINDOW_CLASS")
            || (class == "ConsoleWindowClass"
                && [
                    "powershell.exe",
                    "pwsh.exe",
                    "conhost.exe",
                    "OpenConsole.exe",
                ]
                .iter()
                .any(|name| application.eq_ignore_ascii_case(name)))
    }
    #[cfg(target_os = "windows")]
    fn extract(
        &self,
        source: &super::provider_windows::Source,
        input: &mut super::session::InputContext,
    ) -> windows::core::Result<()> {
        source.terminal(input)
    }
}
impl ContextProvider for DefaultProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Default
    }
    fn matches(&self, _: &str, _: &str) -> bool {
        true
    }
    #[cfg(target_os = "windows")]
    fn extract(
        &self,
        _: &super::provider_windows::Source,
        _: &mut super::session::InputContext,
    ) -> windows::core::Result<()> {
        Ok(())
    }
}

// Registration is the sole dispatch seam; routing and request assembly know no
// application-specific extraction logic. Default must remain the last entry.
static REGISTRY: [&dyn ContextProvider; 3] = [&T3Code, &Terminal, &DefaultProvider];
pub(super) fn select(application: &str, class: &str) -> &'static dyn ContextProvider {
    REGISTRY
        .iter()
        .copied()
        .find(|provider| provider.matches(application, class))
        .unwrap_or(&DefaultProvider)
}

pub(super) fn metadata(value: &str) -> Captured<String> {
    if value.is_empty() {
        Captured::Empty
    } else if value.chars().count() <= 512 {
        Captured::Present(value.into())
    } else {
        Captured::Unavailable
    }
}

/// Windows drive/UNC paths and absolute Unix paths remain separate namespaces.
pub(super) fn directory_key(value: &str) -> Option<String> {
    if value.trim() != value || value.contains(['\0', '\n', '\r']) {
        return None;
    }
    let bytes = value.as_bytes();
    let windows = (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\'))
        || value.starts_with("\\\\")
        || value.starts_with("//");
    let mut path = if windows {
        value.replace('\\', "/")
    } else {
        value.into()
    };
    if path.starts_with("//?/UNC/") {
        path = format!("//{}", &path[8..]);
    } else if path.starts_with("//?/") {
        path = path[4..].into();
    }
    if windows
        && !path.starts_with("//")
        && !(path.len() >= 3
            && path.as_bytes()[0].is_ascii_alphabetic()
            && path.as_bytes()[1] == b':'
            && path.as_bytes()[2] == b'/')
    {
        return None;
    }
    if let Some(unc) = path.strip_prefix("//") {
        let mut components = unc.split('/');
        if components.next().is_none_or(str::is_empty)
            || components.next().is_none_or(str::is_empty)
        {
            return None;
        }
    }
    if !windows && !path.starts_with('/') {
        return None;
    }
    if path.split('/').any(|part| part == "." || part == "..") {
        return None;
    }
    while path.ends_with('/') && path.len() > 1 && !(path.len() == 3 && path.ends_with(":/")) {
        path.pop();
    }
    if windows {
        path.make_ascii_lowercase();
    }
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_requires_supported_host_identity_and_keeps_default_last() {
        assert_eq!(
            select("T3 Code (Nightly).exe", "Chrome_WidgetWin_1").id(),
            ProviderId::T3Code
        );
        assert_eq!(
            select("WindowsTerminal.exe", "CASCADIA_HOSTING_WINDOW_CLASS").id(),
            ProviderId::WindowsTerminal
        );
        assert_eq!(
            select("powershell.exe", "ConsoleWindowClass").id(),
            ProviderId::WindowsTerminal
        );
        assert_eq!(
            select("chrome.exe", "Chrome_WidgetWin_1").id(),
            ProviderId::Default
        );
        assert_eq!(select("T3 Code.exe", "other").id(), ProviderId::Default);
    }
    #[test]
    fn paths_are_exact_absolute_and_namespace_aware() {
        assert_eq!(
            directory_key("D:\\Work\\Project\\"),
            directory_key("d:/Work/Project")
        );
        assert_eq!(directory_key("\\\\?\\D:\\Work"), directory_key("d:/Work"));
        assert_ne!(directory_key("/home/A"), directory_key("/home/a"));
        assert_ne!(directory_key("D:/Work"), directory_key("D:/Workspace"));
        assert!(directory_key("D:relative").is_none());
        assert!(directory_key("A😀").is_none());
        assert!(directory_key("\\\\server").is_none());
        assert!(directory_key("\\\\?\\relative").is_none());
        assert_ne!(directory_key("/work/a\\b"), directory_key("/work/a/b"));
        assert!(directory_key("D:/Work/../Other").is_none());
    }
}
