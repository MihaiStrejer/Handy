//! Console inspection runs in a disposable child: AttachConsole never changes
//! Handy's own console or keyboard/clipboard state.
use super::{providers::directory_key, target::TargetIdentity};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::*,
    System::{
        Console::*,
        Diagnostics::{Debug::ReadProcessMemory, ToolHelp::*},
        Threading::*,
    },
    UI::WindowsAndMessaging::*,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Workspace {
    directory: String,
    console: usize,
    process: u32,
}

fn sole_workspace(mut matching: Vec<Option<Workspace>>) -> Option<Workspace> {
    if matching.len() == 1 {
        matching.pop().flatten()
    } else {
        None
    }
}

/// Shells whose process current directory does not follow the prompt location:
/// PowerShell does not sync `Set-Location`, WSL and SSH hold a foreign
/// directory, and MSYS/Cygwin shells keep their own cwd.
fn directory_untrusted(image: &str) -> bool {
    [
        "powershell.exe",
        "pwsh.exe",
        "wsl.exe",
        "wslhost.exe",
        "ssh.exe",
        "bash.exe",
        "sh.exe",
        "zsh.exe",
        "fish.exe",
    ]
    .iter()
    .any(|name| image.eq_ignore_ascii_case(name))
}

fn leaf(clients: &[u32], parents: &HashMap<u32, u32>, observer: u32) -> Option<u32> {
    let set: HashSet<_> = clients
        .iter()
        .copied()
        .filter(|pid| *pid != observer)
        .collect();
    let mut leaves = set
        .iter()
        .filter(|pid| !set.iter().any(|child| parents.get(child) == Some(pid)));
    let result = leaves.next().copied()?;
    if leaves.next().is_some() {
        None
    } else {
        Some(result)
    }
}

pub(super) fn workspace(target: &TargetIdentity, tab: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    let exe = std::env::current_exe().ok()?;
    // Unit executables cannot run the application helper. Pure decision and
    // memory-reader tests remain available; native app proof uses handy.exe.
    if exe.file_stem()?.to_string_lossy() != "handy" {
        return None;
    }
    let mut child = std::process::Command::new(exe)
        // `=` form: clap accepts a title that starts with `-` (e.g. `-bash`).
        .args([
            "--context-helper".to_string(),
            target.window.to_string(),
            format!("--context-tab={tab}"),
        ])
        .creation_flags(0x08000000)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_millis(180);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(2)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let output = child.wait_with_output().ok()?;
    if output.stdout.len() > 8192 {
        return None;
    }
    let value: Option<Workspace> = serde_json::from_slice(&output.stdout).ok()?;
    let value = value?;
    directory_key(&value.directory)?;
    Some(value.directory)
}

pub(super) fn helper(window: usize, tab: &str) -> i32 {
    use std::{io::Write, os::windows::io::FromRawHandle};
    // Attach/FreeConsole replaces standard handles. Duplicate the caller's IPC
    // pipe first and write through that owned handle after detaching.
    let output = unsafe {
        let Ok(original) = GetStdHandle(STD_OUTPUT_HANDLE) else {
            return 1;
        };
        let process = GetCurrentProcess();
        let mut duplicate = HANDLE::default();
        if DuplicateHandle(
            process,
            original,
            process,
            &mut duplicate,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )
        .is_err()
        {
            return 1;
        }
        std::fs::File::from_raw_handle(duplicate.0)
    };
    let result = inspect(window, tab);
    // Console has been detached before this write; stdout remains the parent's
    // IPC pipe, never the observed terminal output buffer.
    match serde_json::to_string(&result) {
        Ok(value) => {
            let mut output = output;
            if writeln!(output, "{value}").is_ok() {
                0
            } else {
                1
            }
        }
        Err(_) => 1,
    }
}

fn inspect(window: usize, tab: &str) -> Option<Workspace> {
    if tab.chars().count() > 512 || window == 0 {
        return None;
    }
    let class = super::provider_windows::window_class(window);
    if class != "CASCADIA_HOSTING_WINDOW_CLASS" && class != "ConsoleWindowClass" {
        return None;
    }
    unsafe {
        let mut host = 0;
        if GetWindowThreadProcessId(HWND(window as *mut _), Some(&mut host)) == 0 {
            return None;
        }
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        struct Close(HANDLE);
        impl Drop for Close {
            fn drop(&mut self) {
                unsafe {
                    let _ = CloseHandle(self.0);
                }
            }
        }
        let _close = Close(snapshot);
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut parents = HashMap::new();
        let mut untrusted = HashSet::new();
        let mut seeds = vec![];
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|x| *x == 0)
                    .unwrap_or(entry.szExeFile.len());
                let image = String::from_utf16_lossy(&entry.szExeFile[..end]);
                if directory_untrusted(&image) {
                    untrusted.insert(entry.th32ProcessID);
                }
                if entry.th32ProcessID == host
                    || entry.th32ParentProcessID == host
                        && !["OpenConsole.exe", "conhost.exe"]
                            .iter()
                            .any(|n| image.eq_ignore_ascii_case(n))
                {
                    seeds.push(entry.th32ProcessID);
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        if seeds.len() > 64 {
            return None;
        }
        // Only this disposable process detaches its inherited console.
        let _ = FreeConsole();
        let observer = GetCurrentProcessId();
        let mut seen = HashSet::new();
        let mut matches = vec![];
        for pid in seeds.into_iter().take(64) {
            if AttachConsole(pid).is_err() {
                continue;
            }
            struct Detach;
            impl Drop for Detach {
                fn drop(&mut self) {
                    unsafe {
                        let _ = FreeConsole();
                    }
                }
            }
            let detach = Detach;
            let console = GetConsoleWindow();
            let owner = GetAncestor(console, GA_ROOTOWNER);
            let mut title = [0u16; 514];
            let title_length = GetConsoleTitleW(&mut title) as usize;
            let console_title = String::from_utf16_lossy(&title[..title_length.min(title.len())]);
            if owner.0 as usize == window && seen.insert(console.0 as usize) {
                // The caller supplies the unique visible TermControl's actual
                // connected-console title, not the editable tab caption. Count
                // every matching owned console, including unreadable clients,
                // so duplicate titles cannot select an arbitrary inactive tab.
                if console_title != tab {
                    drop(detach);
                    continue;
                }
                let mut clients = [0u32; 128];
                let count = GetConsoleProcessList(&mut clients) as usize;
                if count > 0 && count <= clients.len() {
                    let process = leaf(&clients[..count], &parents, observer);
                    if let Some(process) = process.filter(|pid| !untrusted.contains(pid)) {
                        let directory = cwd(process);
                        let mut after = [0u16; 514];
                        let count = GetConsoleTitleW(&mut after) as usize;
                        let stable =
                            String::from_utf16_lossy(&after[..count.min(after.len())]) == tab;
                        matches.push(directory.filter(|_| stable).map(|directory| Workspace {
                            directory,
                            process,
                            console: console.0 as usize,
                        }));
                    } else {
                        matches.push(None);
                    }
                } else {
                    matches.push(None);
                }
            }
            drop(detach);
        }
        sole_workspace(matches)
    }
}

#[repr(C)]
#[derive(Default)]
struct BasicInformation {
    reserved: usize,
    peb: usize,
    reserved2: [usize; 2],
    pid: usize,
    parent: usize,
}
#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQueryInformationProcess(
        handle: HANDLE,
        class: u32,
        info: *mut BasicInformation,
        length: u32,
        returned: *mut u32,
    ) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn IsWow64Process2(handle: HANDLE, process_machine: *mut u16, native_machine: *mut u16) -> i32;
}

unsafe fn read<const N: usize>(handle: HANDLE, address: usize) -> Option<[u8; N]> {
    if address == 0 {
        return None;
    }
    let mut bytes = [0u8; N];
    let mut received = 0;
    ReadProcessMemory(
        handle,
        address as *const _,
        bytes.as_mut_ptr() as *mut _,
        N,
        Some(&mut received),
    )
    .ok()?;
    (received == N).then_some(bytes)
}

unsafe fn cwd(pid: u32) -> Option<String> {
    if std::mem::size_of::<usize>() != 8 {
        return None;
    }
    let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;
    struct Close(HANDLE);
    impl Drop for Close {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let _close = Close(handle);
    let (mut machine, mut native) = (0, 0);
    if IsWow64Process2(handle, &mut machine, &mut native) == 0 || machine != 0 || native != 0x8664 {
        return None;
    }
    let mut basic = BasicInformation::default();
    let mut returned = 0;
    if NtQueryInformationProcess(
        handle,
        0,
        &mut basic,
        std::mem::size_of::<BasicInformation>() as u32,
        &mut returned,
    ) < 0
        || basic.pid != pid as usize
    {
        return None;
    }
    let parameters = usize::from_le_bytes(read::<8>(handle, basic.peb.checked_add(0x20)?)?);
    if u32::from_le_bytes(read::<4>(handle, parameters.checked_add(8)?)?) & 1 == 0 {
        return None;
    }
    let address = parameters.checked_add(0x38)?;
    let descriptor = read::<16>(handle, address)?;
    let length = u16::from_le_bytes([descriptor[0], descriptor[1]]) as usize;
    let maximum = u16::from_le_bytes([descriptor[2], descriptor[3]]) as usize;
    let pointer = usize::from_le_bytes(descriptor[8..16].try_into().ok()?);
    if length == 0 || length % 2 != 0 || length > maximum || length > 2048 || pointer == 0 {
        return None;
    }
    let mut bytes = vec![0u8; length];
    let mut received = 0;
    ReadProcessMemory(
        handle,
        pointer as *const _,
        bytes.as_mut_ptr() as *mut _,
        length,
        Some(&mut received),
    )
    .ok()?;
    if received != length || descriptor != read::<16>(handle, address)? {
        return None;
    }
    let mut again = vec![0u8; length];
    ReadProcessMemory(
        handle,
        pointer as *const _,
        again.as_mut_ptr() as *mut _,
        length,
        Some(&mut received),
    )
    .ok()?;
    if bytes != again || received != length {
        return None;
    }
    let text = String::from_utf16(
        &bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    )
    .ok()?;
    directory_key(&text)?;
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_unavailable_matching_console_still_makes_workspace_selection_ambiguous() {
        let candidate = || {
            Some(Workspace {
                directory: "D:\\Work".into(),
                console: 1,
                process: 10,
            })
        };
        assert!(sole_workspace(vec![candidate()]).is_some());
        assert!(sole_workspace(vec![candidate(), None]).is_none());
        assert!(sole_workspace(vec![candidate(), candidate()]).is_none());
        assert!(sole_workspace(vec![None]).is_none());
        assert!(sole_workspace(vec![]).is_none());
    }
    #[test]
    fn foreground_console_chain_has_one_leaf_and_background_jobs_are_ambiguous() {
        let parents = HashMap::from([(10, 1), (20, 10), (30, 20), (40, 10)]);
        assert_eq!(leaf(&[10, 20, 30, 99], &parents, 99), Some(30));
        assert_eq!(leaf(&[10, 20, 30, 40, 99], &parents, 99), None);
        assert_eq!(leaf(&[99], &parents, 99), None);
    }
    #[test]
    fn shells_with_unsynced_directories_are_untrusted() {
        for image in [
            "powershell.exe",
            "PWSH.EXE",
            "wsl.exe",
            "wslhost.exe",
            "ssh.exe",
            "bash.exe",
            "sh.exe",
            "Zsh.exe",
            "fish.exe",
        ] {
            assert!(directory_untrusted(image), "{image}");
        }
        for image in ["cmd.exe", "node.exe", "claude.exe", "codex.exe", "bash", ""] {
            assert!(!directory_untrusted(image), "{image}");
        }
    }
    #[test]
    fn reads_only_current_process_directory_without_console_attachment() {
        let directory = unsafe { cwd(GetCurrentProcessId()) };
        if cfg!(target_arch = "x86_64") {
            assert_eq!(
                directory.and_then(|d| directory_key(&d)),
                std::env::current_dir()
                    .ok()
                    .and_then(|d| directory_key(&d.to_string_lossy()))
            );
        } else {
            assert!(directory.is_none());
        }
    }
}
