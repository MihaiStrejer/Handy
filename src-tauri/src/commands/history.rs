use crate::actions::process_transcription_output_observed;
use crate::managers::{
    history::{HistoryManager, PaginatedHistory},
    history_processing::{ProcessingRun, RequestContents, RunDetail, RunSnapshot},
    transcription::TranscriptionManager,
};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub async fn get_history_entries(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    cursor: Option<i64>,
    limit: Option<usize>,
) -> Result<PaginatedHistory, String> {
    history_manager
        .get_history_entries(cursor, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_history_processing_runs(
    history_manager: State<'_, Arc<HistoryManager>>,
    entry_id: i64,
) -> Result<Vec<ProcessingRun>, String> {
    history_manager
        .get_processing_runs(entry_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_history_processing_run(
    history_manager: State<'_, Arc<HistoryManager>>,
    run_id: i64,
) -> Result<Option<RunDetail>, String> {
    history_manager
        .get_processing_run(run_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_history_request_contents(
    history_manager: State<'_, Arc<HistoryManager>>,
    call_id: i64,
) -> Result<Option<RequestContents>, String> {
    history_manager
        .get_request_contents(call_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn clear_history_request_contents(
    history_manager: State<'_, Arc<HistoryManager>>,
) -> Result<usize, String> {
    history_manager
        .clear_request_contents()
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn set_history_request_contents_enabled(
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    let mut settings = crate::settings::get_settings(&app);
    settings.save_history_request_contents = enabled;
    crate::settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_history_entry_saved(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    history_manager
        .toggle_saved_status(id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_audio_file_path(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    file_name: String,
) -> Result<String, String> {
    let path = history_manager.get_audio_file_path(&file_name);
    path.to_str()
        .ok_or_else(|| "Invalid file path".to_string())
        .map(|s| s.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_history_entry(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    history_manager
        .delete_entry(id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn retry_history_entry_transcription(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
    id: i64,
) -> Result<(), String> {
    let entry = history_manager
        .get_entry_by_id(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("History entry {} not found", id))?;
    if entry.post_process_requested
        && history_manager
            .get_processing_runs(id)
            .map_err(|e| e.to_string())?
            .iter()
            .any(|run| run.profile_id.is_some() || run.prompt_source.as_deref() == Some("profile"))
    {
        return Err("History retry cannot replay a profile's captured context".into());
    }

    let audio_path = history_manager.get_audio_file_path(&entry.file_name);
    let samples = crate::audio_toolkit::read_wav_samples(&audio_path)
        .map_err(|e| format!("Failed to load audio: {}", e))?;

    if samples.is_empty() {
        return Err("Recording has no audio samples".to_string());
    }

    transcription_manager.initiate_model_load();

    let tm = Arc::clone(&transcription_manager);
    let transcription = tauri::async_runtime::spawn_blocking(move || tm.transcribe(samples))
        .await
        .map_err(|e| format!("Transcription task panicked: {}", e))?
        .map_err(|e| e.to_string())?;

    if transcription.is_empty() {
        return Err("Recording contains no speech".to_string());
    }

    let settings = crate::settings::get_settings(&app);
    let mut run = if entry.post_process_requested {
        let snapshot = RunSnapshot {
            provider_id: settings
                .active_post_process_provider()
                .map(|provider| provider.id.clone()),
            provider_name: settings
                .active_post_process_provider()
                .map(|provider| provider.label.clone()),
            requested_model: settings
                .active_post_process_provider()
                .and_then(|provider| settings.post_process_models.get(&provider.id))
                .cloned(),
            prompt_template: settings
                .post_process_selected_prompt_id
                .as_ref()
                .and_then(|prompt_id| {
                    settings
                        .post_process_prompts
                        .iter()
                        .find(|prompt| &prompt.id == prompt_id)
                })
                .map(|prompt| prompt.prompt.clone()),
            prompt_source: Some("retry_current_settings".into()),
            ..RunSnapshot::default()
        };
        match history_manager.start_processing_run(
            id,
            &transcription,
            snapshot,
            settings.save_history_request_contents,
        ) {
            Ok(run) => Some(run),
            Err(error) => {
                log::error!("Could not save retry processing details: {error}");
                None
            }
        }
    } else {
        None
    };
    if run.is_some() {
        if let Err(error) = history_manager.emit_entry_update(id) {
            log::error!("Could not notify history about retry progress: {error}");
        }
    }
    let processed = process_transcription_output_observed(
        &app,
        &transcription,
        entry.post_process_requested,
        run.as_ref(),
    )
    .await;
    if let Some(ref mut run) = run {
        let status = if processed.processing_error_code.is_some() {
            "failed"
        } else {
            "succeeded"
        };
        if let Err(error) = run.finish(
            status,
            processed.post_processed_text.as_deref(),
            processed.processing_error_code,
            None,
            Some("history_only"),
            None,
        ) {
            log::error!("Could not save retry processing outcome: {error}");
        }
    }
    history_manager
        .update_transcription(id, transcription, processed.post_processed_text, None)
        .map_err(|e| e.to_string())?;
    if let Err(error) = history_manager.emit_entry_update(id) {
        log::error!("Could not notify history about retry completion: {error}");
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_history_limit(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    limit: usize,
) -> Result<(), String> {
    let mut settings = crate::settings::get_settings(&app);
    settings.history_limit = limit;
    crate::settings::write_settings(&app, settings);

    history_manager
        .cleanup_old_entries()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_recording_retention_period(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    period: String,
) -> Result<(), String> {
    use crate::settings::RecordingRetentionPeriod;

    let retention_period = match period.as_str() {
        "never" => RecordingRetentionPeriod::Never,
        "preserve_limit" => RecordingRetentionPeriod::PreserveLimit,
        "days3" => RecordingRetentionPeriod::Days3,
        "weeks2" => RecordingRetentionPeriod::Weeks2,
        "months3" => RecordingRetentionPeriod::Months3,
        _ => return Err(format!("Invalid retention period: {}", period)),
    };

    let mut settings = crate::settings::get_settings(&app);
    settings.recording_retention_period = retention_period;
    crate::settings::write_settings(&app, settings);

    history_manager
        .cleanup_old_entries()
        .map_err(|e| e.to_string())?;

    Ok(())
}
