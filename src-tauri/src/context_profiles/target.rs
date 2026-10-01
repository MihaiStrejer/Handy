/// Opaque local destination identity. Intentionally neither serializable nor
/// Debug: native handles must not enter endpoint payloads or ordinary logs.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct TargetIdentity {
    pub(super) window: usize,
    pub(super) focused_control: usize,
    pub(super) process_id: u32,
    pub(super) thread_id: u32,
}

/// Snapshot native focus before showing Handy UI. This does not query an
/// accessibility provider or read any field contents on the microphone path.
#[cfg(target_os = "windows")]
pub(crate) fn capture_target() -> Option<TargetIdentity> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
    };

    // These calls read desktop state; no handle ownership is transferred.
    unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return None;
        }
        let mut process_id = 0;
        let thread_id = GetWindowThreadProcessId(window, Some(&mut process_id));
        if thread_id == 0 || process_id == 0 {
            return None;
        }
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        if GetGUIThreadInfo(thread_id, &mut info).is_err()
            || GetForegroundWindow() != window
            || info.hwndFocus.is_invalid()
        {
            return None;
        }
        Some(TargetIdentity {
            window: window.0 as usize,
            focused_control: info.hwndFocus.0 as usize,
            process_id,
            thread_id,
        })
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn capture_target() -> Option<TargetIdentity> {
    None
}
