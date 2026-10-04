use super::session::{Captured, InputContext};
use super::target::TargetIdentity;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const LOOKUP_BUDGET: Duration = Duration::from_millis(250);
const SELECTION_LIMIT: usize = 2048;
const SURROUNDING_LIMIT: usize = 4096;

fn unavailable() -> InputContext {
    InputContext {
        provider: crate::context_profiles::providers::ProviderContext::default(),
        application: Captured::Unavailable,
        workspace: Captured::Unavailable,
        selection: Captured::Unavailable,
        surrounding_text: Captured::Unavailable,
        caret_utf16: Captured::Unavailable,
        selection_range_utf16: None,
        captured_at_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        truncated: false,
    }
}

pub(crate) struct CaptureTicket {
    receiver: mpsc::Receiver<InputContext>,
    deadline: Instant,
    base: Arc<Mutex<Option<InputContext>>>,
}

impl CaptureTicket {
    /// Called on a background task, never on the microphone startup thread.
    pub(super) fn wait(self) -> InputContext {
        match self
            .receiver
            .recv_timeout(self.deadline.saturating_duration_since(Instant::now()))
        {
            Ok(context) if Instant::now() <= self.deadline => context,
            Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {
                let mut context = self
                    .base
                    .lock()
                    .ok()
                    .and_then(|mut base| base.take())
                    .unwrap_or_else(unavailable);
                context.workspace = Captured::TimedOut;
                context.selection = Captured::TimedOut;
                context.surrounding_text = Captured::TimedOut;
                context.caret_utf16 = Captured::TimedOut;
                context
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => unavailable(),
        }
    }
}

/// At most one provider thread may be outstanding. A stuck COM provider cannot
/// create a growing number of worker threads across repeated recordings.
#[derive(Default)]
pub(crate) struct CaptureService {
    busy: Arc<AtomicBool>,
}

impl CaptureService {
    pub(super) fn request(&self, target: Option<TargetIdentity>) -> CaptureTicket {
        self.request_with_base(move |base| platform_read(target, base), LOOKUP_BUDGET)
    }

    #[cfg(test)]
    fn request_with<F>(&self, reader: F, budget: Duration) -> CaptureTicket
    where
        F: FnOnce() -> InputContext + Send + 'static,
    {
        self.request_with_base(move |_| reader(), budget)
    }

    fn request_with_base<F>(&self, reader: F, budget: Duration) -> CaptureTicket
    where
        F: FnOnce(Arc<Mutex<Option<InputContext>>>) -> InputContext + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel();
        let base = Arc::new(Mutex::new(None));
        let ticket = CaptureTicket {
            receiver,
            deadline: Instant::now() + budget,
            base: Arc::clone(&base),
        };
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            let _ = sender.send(unavailable());
            return ticket;
        }
        let busy = Arc::clone(&self.busy);
        let worker = std::thread::Builder::new()
            .name("handy-context-capture".into())
            .spawn(move || {
                struct BusyGuard(Arc<AtomicBool>);
                impl Drop for BusyGuard {
                    fn drop(&mut self) {
                        self.0.store(false, Ordering::Release);
                    }
                }
                let _guard = BusyGuard(busy);
                // A timed-out ticket drops the receiver; its late result is discarded.
                let _ = sender.send(reader(base));
            });
        if worker.is_err() {
            self.busy.store(false, Ordering::Release);
            log::warn!("Could not start context capture worker");
        }
        ticket
    }
}

fn bounded_text(value: &str, limit: usize) -> (String, bool) {
    let mut chars = value.chars();
    let text: String = chars.by_ref().take(limit).collect();
    (text, chars.next().is_some())
}

#[cfg(not(target_os = "windows"))]
fn platform_read(
    _target: Option<TargetIdentity>,
    _base: Arc<Mutex<Option<InputContext>>>,
) -> InputContext {
    unavailable()
}

#[cfg(target_os = "windows")]
fn platform_read(
    target: Option<TargetIdentity>,
    base: Arc<Mutex<Option<InputContext>>>,
) -> InputContext {
    windows_reader::read(target, base)
}

#[cfg(target_os = "windows")]
mod windows_reader {
    use super::*;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, HWND};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_MULTITHREADED,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationTextPattern, TextPatternRangeEndpoint_End,
        TextPatternRangeEndpoint_Start, UIA_TextPatternId,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetWindowThreadProcessId, IsWindow,
    };

    pub(super) fn read(
        target: Option<TargetIdentity>,
        base: Arc<Mutex<Option<InputContext>>>,
    ) -> InputContext {
        let mut context = unavailable();
        let Some(target) = target else {
            return context;
        };
        context.application = application(target.process_id)
            .map(Captured::Present)
            .unwrap_or(Captured::Unavailable);
        let class = super::super::provider_windows::window_class(target.window);
        let application = match &context.application {
            Captured::Present(name) => name.as_str(),
            _ => "",
        };
        let provider = super::super::providers::select(application, &class);
        context.provider.id = provider.id();
        context.provider.window_title = super::super::providers::metadata(
            &super::super::provider_windows::window_title(target.window),
        );
        if let Ok(mut base) = base.lock() {
            *base = Some(context.clone());
        }
        // Do not discover another input after a delayed worker starts.
        if super::super::target::capture_target().as_ref() != Some(&target) {
            context.selection = Captured::Uncertain;
            return context;
        }
        if provider.id() != super::super::providers::ProviderId::Default {
            unsafe {
                if CoInitializeEx(None, COINIT_MULTITHREADED).is_ok() {
                    if let Ok(source) = super::super::provider_windows::Source::new(target.clone())
                    {
                        if provider.extract(&source, &mut context).is_err() {
                            // A failed stability recheck invalidates partial
                            // enrichment, including a previously obtained CWD.
                            context.workspace = Captured::Uncertain;
                            context.provider.workspace_source = Captured::Uncertain;
                            context.provider.project = Captured::Uncertain;
                            context.provider.conversation = Captured::Uncertain;
                            context.provider.branch = Captured::Uncertain;
                            context.provider.terminal_tab = Captured::Uncertain;
                            context.provider.input = Captured::Uncertain;
                        }
                    }
                    CoUninitialize();
                }
            }
            if super::super::target::capture_target().as_ref() != Some(&target) {
                context.workspace = Captured::Uncertain;
                context.provider.project = Captured::Uncertain;
                context.provider.conversation = Captured::Uncertain;
                context.provider.branch = Captured::Uncertain;
                context.provider.input = Captured::Uncertain;
            }
            return context;
        }
        // A native Edit HWND identifies one input. Browser/editor render HWNDs
        // contain many inputs and need a separate element-identity integration.
        let control = HWND(target.focused_control as *mut _);
        let mut class = [0u16; 128];
        unsafe {
            let length = GetClassNameW(control, &mut class);
            if length <= 0
                || !String::from_utf16_lossy(&class[..length as usize]).eq_ignore_ascii_case("Edit")
            {
                return context;
            }
        }
        if let Ok(mut captured) = read_control(&target, context.clone()) {
            if super::super::target::capture_target().as_ref() != Some(&target) {
                captured.selection = Captured::Uncertain;
                captured.surrounding_text = Captured::Uncertain;
                captured.caret_utf16 = Captured::Uncertain;
            }
            captured
        } else {
            context
        }
    }

    fn application(pid: u32) -> Option<String> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buffer = [0u16; 32768];
            let mut length = buffer.len() as u32;
            let result = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut length,
            );
            let _ = CloseHandle(handle);
            result.ok()?;
            let path = String::from_utf16_lossy(&buffer[..length as usize]);
            path.rsplit(['\\', '/']).next().map(str::to_owned)
        }
    }

    fn same_control(target: &TargetIdentity) -> bool {
        unsafe {
            let control = HWND(target.focused_control as *mut _);
            let mut pid = 0;
            IsWindow(Some(control)).as_bool()
                && GetWindowThreadProcessId(control, Some(&mut pid)) == target.thread_id
                && pid == target.process_id
        }
    }

    fn read_control(
        target: &TargetIdentity,
        mut context: InputContext,
    ) -> windows::core::Result<InputContext> {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongW, ES_PASSWORD, GWL_STYLE};
            // Native Edit marks protected fields independently of UIA. Refuse
            // those before contacting a provider, even when UIA is unavailable.
            if GetWindowLongW(HWND(target.focused_control as *mut _), GWL_STYLE) & ES_PASSWORD != 0
            {
                context.selection = Captured::Protected;
                context.surrounding_text = Captured::Protected;
                context.caret_utf16 = Captured::Protected;
                return Ok(context);
            }
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            struct ComGuard;
            impl Drop for ComGuard {
                fn drop(&mut self) {
                    unsafe {
                        CoUninitialize();
                    }
                }
            }
            let _com = ComGuard;
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
            let control = HWND(target.focused_control as *mut _);
            let element = automation.ElementFromHandle(control)?;
            if element.CurrentProcessId()? as u32 != target.process_id
                || element.CurrentNativeWindowHandle()? != control
                || !same_control(target)
            {
                context.selection = Captured::Uncertain;
                return Ok(context);
            }
            // Check before requesting a text pattern or reading any text.
            if element.CurrentIsPassword()?.as_bool() {
                context.selection = Captured::Protected;
                context.surrounding_text = Captured::Protected;
                context.caret_utf16 = Captured::Protected;
                return Ok(context);
            }
            let pattern: IUIAutomationTextPattern =
                match element.GetCurrentPatternAs(UIA_TextPatternId) {
                    Ok(pattern) => pattern,
                    Err(_) => return Ok(read_native_edit(target, context)),
                };
            let document = pattern.DocumentRange()?;
            // UIA lengths use UTF-16; the larger read allows retaining a full
            // surrogate pair before the Unicode-scalar bound is applied.
            let raw = document
                .GetText((SURROUNDING_LIMIT * 2 + 2) as i32)?
                .to_string();
            let (surrounding, truncated) = bounded_text(&raw, SURROUNDING_LIMIT);
            context.truncated = truncated;
            context.surrounding_text = if surrounding.is_empty() {
                Captured::Empty
            } else {
                Captured::Present(surrounding)
            };
            let ranges = pattern.GetSelection()?;
            if ranges.Length()? != 1 {
                context.selection = Captured::Uncertain;
                return Ok(context);
            }
            let selected = ranges.GetElement(0)?;
            let raw = selected
                .GetText((SELECTION_LIMIT * 2 + 2) as i32)?
                .to_string();
            let (text, truncated) = bounded_text(&raw, SELECTION_LIMIT);
            context.truncated |= truncated;
            context.selection = if truncated {
                Captured::Uncertain
            } else if text.is_empty() {
                Captured::Empty
            } else {
                Captured::Present(text)
            };
            if !context.truncated {
                let prefix = document.Clone()?;
                prefix.MoveEndpointByRange(
                    TextPatternRangeEndpoint_End,
                    &selected,
                    TextPatternRangeEndpoint_Start,
                )?;
                let raw_prefix = prefix
                    .GetText((SURROUNDING_LIMIT * 2 + 2) as i32)?
                    .to_string();
                if raw_prefix.chars().count() <= SURROUNDING_LIMIT {
                    let start = raw_prefix.encode_utf16().count() as u32;
                    match &context.selection {
                        Captured::Empty => {
                            context.caret_utf16 = Captured::Present(start);
                            context.selection_range_utf16 = Some((start, start));
                        }
                        Captured::Present(text) => {
                            context.selection_range_utf16 =
                                Some((start, start + text.encode_utf16().count() as u32));
                        }
                        _ => {}
                    }
                }
            }
            if !same_control(target) {
                // Discard partial content if the input identity/focus changed.
                context.selection = Captured::Uncertain;
                context.surrounding_text = Captured::Uncertain;
                context.caret_utf16 = Captured::Uncertain;
            }
            Ok(context)
        }
    }

    // Some native Edit providers expose password/identity properties through
    // UIA but no TextPattern. Read their documented Edit messages through the
    // same captured HWND, with per-call deadlines and no clipboard access.
    fn read_native_edit(target: &TargetIdentity, mut context: InputContext) -> InputContext {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongW, SendMessageTimeoutW, ES_PASSWORD, GWL_STYLE, SMTO_ABORTIFHUNG,
            SMTO_BLOCK, WM_GETTEXT, WM_GETTEXTLENGTH,
        };
        const EM_GETSEL: u32 = 0x00B0;
        unsafe fn message(hwnd: HWND, id: u32, w: usize, l: isize) -> Option<usize> {
            let mut result = 0;
            (SendMessageTimeoutW(
                hwnd,
                id,
                WPARAM(w),
                LPARAM(l),
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                25,
                Some(&mut result),
            )
            .0 != 0)
                .then_some(result)
        }
        unsafe fn read(hwnd: HWND) -> Option<(Vec<u16>, u32, u32, usize)> {
            let mut start = 0u32;
            let mut end = 0u32;
            message(
                hwnd,
                EM_GETSEL,
                &mut start as *mut u32 as usize,
                &mut end as *mut u32 as isize,
            )?;
            let length = message(hwnd, WM_GETTEXTLENGTH, 0, 0)?;
            let mut buffer = vec![0u16; SURROUNDING_LIMIT * 2 + 3];
            let copied = message(hwnd, WM_GETTEXT, buffer.len(), buffer.as_mut_ptr() as isize)?;
            if copied >= buffer.len() {
                return None;
            }
            buffer.truncate(copied);
            Some((buffer, start, end, length))
        }
        unsafe {
            let hwnd = HWND(target.focused_control as *mut _);
            if !same_control(target) || GetWindowLongW(hwnd, GWL_STYLE) & ES_PASSWORD != 0 {
                return context;
            }
            let Some(first) = read(hwnd) else {
                return context;
            };
            let Some(second) = read(hwnd) else {
                return context;
            };
            if first != second
                || !same_control(target)
                || GetWindowLongW(hwnd, GWL_STYLE) & ES_PASSWORD != 0
            {
                context.selection = Captured::Uncertain;
                return context;
            }
            let (raw, start, end, total) = first;
            let Ok(text) = String::from_utf16(&raw) else {
                return context;
            };
            let (surrounding, truncated) = bounded_text(&text, SURROUNDING_LIMIT);
            let surrounding_units = surrounding.encode_utf16().count();
            context.truncated = truncated || total > raw.len();
            context.surrounding_text = if surrounding.is_empty() {
                Captured::Empty
            } else {
                Captured::Present(surrounding)
            };
            if start > end || end as usize > raw.len() {
                context.selection = Captured::Uncertain;
                return context;
            }
            if !context.truncated {
                context.selection_range_utf16 = Some((start, end));
            }
            if start == end {
                context.selection = Captured::Empty;
                if end as usize <= surrounding_units {
                    context.caret_utf16 = Captured::Present(end);
                }
            } else if let Ok(selected) = String::from_utf16(&raw[start as usize..end as usize]) {
                if selected.chars().count() <= SELECTION_LIMIT {
                    context.selection = Captured::Present(selected);
                } else {
                    context.selection = Captured::Uncertain;
                    context.truncated = true;
                }
            } else {
                context.selection = Captured::Uncertain;
            }
            context
        }
    }

    #[cfg(test)]
    mod native_tests {
        use super::*;
        use windows::core::w;
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::System::Threading::{GetCurrentProcessId, GetCurrentThreadId};
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, DispatchMessageW, PeekMessageW, SendMessageW,
            TranslateMessage, ES_MULTILINE, ES_PASSWORD, MSG, PM_REMOVE, WINDOW_EX_STYLE,
            WINDOW_STYLE, WS_CHILD, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
        };

        // WinUser.h: the native Edit selection message. Fixture setup only;
        // production selection capture uses UI Automation TextPattern.
        const EM_SETSEL: u32 = 0x00B1;

        struct Fixture {
            target: TargetIdentity,
            stop: mpsc::Sender<()>,
            thread: Option<std::thread::JoinHandle<()>>,
        }

        impl Fixture {
            fn new(password: bool, start: usize, end: isize) -> Self {
                let (send, receive) = mpsc::channel();
                let (stop, stopped) = mpsc::channel();
                let thread = std::thread::spawn(move || unsafe {
                    let parent = CreateWindowExW(
                        WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                        w!("STATIC"),
                        w!("Handy capture test"),
                        WS_POPUP | WS_VISIBLE,
                        -10000,
                        -10000,
                        400,
                        160,
                        None,
                        None,
                        None,
                        None,
                    )
                    .unwrap();
                    let style = if password { ES_PASSWORD } else { ES_MULTILINE };
                    let edit = CreateWindowExW(
                        WINDOW_EX_STYLE::default(),
                        w!("EDIT"),
                        w!("alpha\r\nCodex \u{1f600}\r\nomega"),
                        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(style as u32),
                        0,
                        0,
                        380,
                        140,
                        Some(parent),
                        None,
                        None,
                        None,
                    )
                    .unwrap();
                    SendMessageW(edit, EM_SETSEL, Some(WPARAM(start)), Some(LPARAM(end)));
                    send.send(TargetIdentity {
                        window: parent.0 as usize,
                        focused_control: edit.0 as usize,
                        process_id: GetCurrentProcessId(),
                        thread_id: GetCurrentThreadId(),
                    })
                    .unwrap();
                    // The fixture owns its message pump and never changes the
                    // user's foreground window or reads another app's text.
                    while matches!(stopped.try_recv(), Err(mpsc::TryRecvError::Empty)) {
                        let mut msg = MSG::default();
                        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                            let _ = TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    DestroyWindow(parent).unwrap();
                });
                Self {
                    target: receive.recv_timeout(Duration::from_secs(5)).unwrap(),
                    stop,
                    thread: Some(thread),
                }
            }
        }

        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = self.stop.send(());
                if let Some(thread) = self.thread.take() {
                    thread.join().unwrap();
                }
            }
        }

        #[test]
        fn windows_uia_reads_native_selection_and_utf16_caret() {
            let fixture = Fixture::new(false, 7, 15);
            let context = read_control(&fixture.target, unavailable()).unwrap();
            assert!(context.selection == Captured::Present("Codex \u{1f600}".into()));
            assert!(
                matches!(context.surrounding_text,Captured::Present(ref text) if text.contains("alpha") && text.contains("omega"))
            );
            let caret = Fixture::new(false, 15, 15);
            let context = read_control(&caret.target, unavailable()).unwrap();
            assert!(context.selection == Captured::Empty);
            assert!(context.caret_utf16 == Captured::Present(15));
            let multiline = Fixture::new(false, 0, 17);
            let context = read_control(&multiline.target, unavailable()).unwrap();
            assert!(context.selection == Captured::Present("alpha\r\nCodex \u{1f600}\r\n".into()));
            // A worker that starts after focus moved must not inspect a new
            // focused element. This fixture never owned foreground focus.
            let stale = read(Some(multiline.target.clone()), Arc::new(Mutex::new(None)));
            assert!(stale.selection == Captured::Uncertain);
            assert!(stale.surrounding_text == Captured::Unavailable);
        }

        #[test]
        fn native_corrected_selection_is_verified_before_memory_reaches_next_request() {
            use crate::context_profiles::{
                feedback, request, routing,
                storage::{MemoryState, ProfileSnapshot},
                SessionStore,
            };
            use std::collections::HashMap;
            use windows::Win32::UI::WindowsAndMessaging::WM_SETTEXT;
            let fixture = Fixture::new(false, 0, 0);
            let hwnd = HWND(fixture.target.focused_control as *mut _);
            unsafe {
                SendMessageW(
                    hwnd,
                    WM_SETTEXT,
                    None,
                    Some(LPARAM(w!("Review the bomb today").as_ptr() as isize)),
                );
                SendMessageW(hwnd, EM_SETSEL, Some(WPARAM(11)), Some(LPARAM(15)));
            }
            let before = read_native_edit(&fixture.target, unavailable());
            assert!(before.selection == Captured::Present("bomb".into()));
            assert_eq!(before.selection_range_utf16, Some((11, 15)));
            let mut snapshot = ProfileSnapshot {
                catalog: Arc::new({
                    let legacy: serde_json::Value = serde_json::from_str(include_str!(
                        "../../../tests/fixtures/profile-memory-legacy-catalog.json"
                    ))
                    .unwrap();
                    crate::context_profiles::migration::convert(legacy["profiles"].clone()).unwrap()
                }),
                memory: HashMap::new(),
                memory_epochs: HashMap::new(),
            };
            let context = routing::resolve(&snapshot, before.clone()).unwrap();
            let prediction = feedback::tests::correction();
            let expected =
                feedback::ExpectedChange::new(&before, &prediction, &prediction.text).unwrap();
            let sessions = SessionStore::default();
            let id = sessions
                .begin(0, Some(fixture.target.clone()), true)
                .unwrap();
            sessions.resolve(id, 0, context).unwrap();
            sessions.begin_request(id, 0).unwrap();
            sessions
                .accept_prediction(id, 0, prediction, "not bomb, BOM")
                .unwrap();
            let mut memory = MemoryState::default();
            assert!(!expected.matches(&read_native_edit(&fixture.target, unavailable())));
            assert!(memory.items.is_empty());
            // Perform a real edit in our offscreen native control. No user
            // clipboard, focus, microphone, or external endpoint is touched.
            unsafe {
                SendMessageW(
                    hwnd,
                    0x00C2, /* EM_REPLACESEL */
                    Some(WPARAM(1)),
                    Some(LPARAM(w!("BOM").as_ptr() as isize)),
                );
            }
            let after = read_native_edit(&fixture.target, unavailable());
            assert!(expected.matches(&after));
            assert!(sessions
                .commit_feedback(id, 0, |c, t, s| Ok(memory.admit_batch(
                    &c.profile_id,
                    c.memory_epoch,
                    t,
                    s
                )))
                .unwrap());
            assert!(!sessions
                .commit_feedback(id, 0, |_, _, _| panic!("duplicate"))
                .unwrap());
            snapshot.memory = memory.items.clone();
            let next = routing::resolve(&snapshot, after).unwrap();
            let (_, user) = request::assemble(&next, "Check the bomb").unwrap();
            let data: serde_json::Value = serde_json::from_str(&user).unwrap();
            assert_eq!(
                data["short_term_memory"][0]["text"],
                "Use BOM when bomb refers to this term."
            );
            assert!(data["long_term_memory"].as_str().unwrap().contains("Codex"));
            assert!(data["input_context"].get("selection_range_utf16").is_none());
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                request::tests::verify_memory_round_trip(&next).await;
                crate::context_profiles::consolidation::tests::verify_controlled_promotion_and_restart(&snapshot.catalog, &memory).await;
            });
            memory.remove("general", None);
            snapshot.memory = memory.items;
            snapshot.memory_epochs = memory.epochs;
            let next = routing::resolve(&snapshot, before).unwrap();
            let (_, user) = request::assemble(&next, "next request").unwrap();
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&user).unwrap()["short_term_memory"],
                serde_json::json!([])
            );
        }

        #[test]
        fn windows_uia_never_reads_password_content() {
            let fixture = Fixture::new(true, 0, 4);
            let context = read_control(&fixture.target, unavailable()).unwrap();
            assert!(context.selection == Captured::Protected);
            assert!(context.surrounding_text == Captured::Protected);
            assert!(context.caret_utf16 == Captured::Protected);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_retains_distinct_empty_protected_and_unavailable_states() {
        let service = CaptureService::default();
        let result = service
            .request_with(
                || {
                    let mut c = unavailable();
                    c.selection = Captured::Empty;
                    c.surrounding_text = Captured::Protected;
                    c
                },
                Duration::from_secs(1),
            )
            .wait();
        assert!(result.selection == Captured::Empty);
        assert!(result.surrounding_text == Captured::Protected);
        assert!(result.workspace == Captured::Unavailable);
    }

    #[test]
    fn hung_reader_times_out_and_does_not_start_another_worker() {
        let service = CaptureService::default();
        let (release, wait) = mpsc::channel();
        let ticket = service.request_with(
            move || {
                wait.recv().unwrap();
                unavailable()
            },
            Duration::from_millis(10),
        );
        assert!(ticket.wait().selection == Captured::TimedOut);
        let ran = Arc::new(AtomicBool::new(false));
        let ran_copy = Arc::clone(&ran);
        let second = service
            .request_with(
                move || {
                    ran_copy.store(true, Ordering::Release);
                    unavailable()
                },
                Duration::from_secs(1),
            )
            .wait();
        assert!(second.selection == Captured::Unavailable);
        assert!(!ran.load(Ordering::Acquire));
        release.send(()).unwrap();
    }

    #[test]
    fn bounds_preserve_unicode_and_multiline_content() {
        assert_eq!(
            bounded_text("a\u{1f600}\nbc", 4),
            ("a\u{1f600}\nb".into(), true)
        );
        assert_eq!(bounded_text("\u{1f600}", 1), ("\u{1f600}".into(), false));
    }

    #[test]
    fn timeout_keeps_known_provider_identity_without_publishing_late_metadata() {
        let service = CaptureService::default();
        let (release, wait) = mpsc::channel();
        let ticket = service.request_with_base(
            move |base| {
                let mut context = unavailable();
                context.provider.id = super::super::providers::ProviderId::T3Code;
                *base.lock().unwrap() = Some(context);
                wait.recv().unwrap();
                unavailable()
            },
            Duration::from_millis(30),
        );
        let result = ticket.wait();
        assert_eq!(
            result.provider.id,
            super::super::providers::ProviderId::T3Code
        );
        assert!(result.workspace == Captured::TimedOut);
        release.send(()).unwrap();
    }
}
