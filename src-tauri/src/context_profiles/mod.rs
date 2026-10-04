//! Session ownership for context-aware post-processing.
//!
//! Native destination handles stay in this module; only a frozen snapshot may
//! become request data. Platform enrichment and routing attach to the session.

mod capture;
pub(crate) mod consolidation;
pub(crate) mod feedback;
pub(crate) mod icons;
mod memory_proposals;
mod migration;
#[cfg(target_os = "windows")]
mod provider_windows;
mod providers;
pub(crate) mod request;
mod routing;
mod session;
pub(crate) mod storage;
mod target;
mod template;
#[cfg(target_os = "windows")]
mod terminal_input;
#[cfg(target_os = "windows")]
mod terminal_process;

pub(crate) use capture::CaptureService;
pub(crate) use session::{SessionDisplay, SessionId, SessionStore};
pub(crate) use target::capture_target;

/// Run the disposable read-only console observer before application startup.
pub fn run_context_helper(window: usize, tab: &str) -> i32 {
    #[cfg(target_os = "windows")]
    {
        terminal_process::helper(window, tab)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, tab);
        2
    }
}

pub(crate) fn inspect(
    app: &tauri::AppHandle,
    target: Option<target::TargetIdentity>,
) -> Result<serde_json::Value, String> {
    let input = CaptureService::default().request(target).wait();
    let snapshot = storage::snapshot(app)?;
    let resolved = routing::resolve(&snapshot, input).map_err(str::to_owned)?;
    Ok(
        serde_json::json!({"profile_id":resolved.profile_id,"profile_name":resolved.profile_name,"match_basis":resolved.match_basis,"input_context":resolved.input}),
    )
}

pub(crate) fn start_capture(
    app: tauri::AppHandle,
    id: SessionId,
    audio: std::sync::Arc<crate::managers::audio::AudioRecordingManager>,
) {
    use tauri::Manager;
    let snapshot = match storage::snapshot(&app) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            log::error!("{error}");
            return;
        }
    };
    let target = match app.state::<SessionStore>().target(id) {
        Ok(target) => target,
        Err(error) => {
            log::error!("{error}");
            return;
        }
    };
    let ticket = app.state::<CaptureService>().request(target);
    tauri::async_runtime::spawn_blocking(move || {
        let captured = ticket.wait();
        let generation = audio.cancel_generation();
        let resolved = routing::resolve(&snapshot, captured.clone());
        match app
            .state::<SessionStore>()
            .attach_capture(id, generation, captured)
        {
            Ok(true) => match resolved {
                Ok(context) => {
                    match app.state::<SessionStore>().resolve(
                        id,
                        audio.cancel_generation(),
                        context,
                    ) {
                        Ok(true) => {
                            let handle = app.clone();
                            if let Err(error) = app.run_on_main_thread(move || {
                                use tauri_specta::Event;
                                if handle
                                    .state::<SessionStore>()
                                    .current(audio.cancel_generation())
                                    .ok()
                                    .flatten()
                                    == Some(id)
                                {
                                    if let Ok(Some(display)) =
                                        handle.state::<SessionStore>().display()
                                    {
                                        if let Err(error) = display.emit(&handle) {
                                            log::warn!(
                                                "Could not update context indicator: {error}"
                                            );
                                        }
                                    }
                                }
                            }) {
                                log::warn!("Could not schedule context indicator: {error}");
                            }
                        }
                        Ok(false) => {}
                        Err(error) => log::error!("{error}"),
                    }
                }
                Err(error) => log::warn!("{error}"),
            },
            Ok(false) => {}
            Err(error) => log::error!("{error}"),
        }
    });
}
