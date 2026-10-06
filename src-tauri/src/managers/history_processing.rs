//! Durable evidence for each post-processing attempt. Request bodies are fetched only by detail commands.
use anyhow::{anyhow, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;
pub type WarningCallback = Arc<dyn Fn(i64) + Send + Sync>;

const ARCHIVE_LIMIT: usize = 1024 * 1024;

fn connect(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(conn)
}

/// Records that a run's details are incomplete. `warned` is shared by the run
/// and its calls, so the warning callback fires at most once per run.
fn note_incomplete(
    path: &Path,
    run_id: i64,
    entry_id: i64,
    warning: Option<&WarningCallback>,
    warned: &AtomicBool,
) {
    match connect(path) {
        Ok(conn) => {
            if let Err(error) = conn.execute(
                "UPDATE history_processing_runs SET details_incomplete=1 WHERE id=?1",
                [run_id],
            ) {
                log::error!("Details could not be saved: {error}");
            }
        }
        Err(error) => log::error!("Details could not be saved: {error}"),
    }
    if let Some(warning) = warning {
        if !warned.swap(true, Ordering::SeqCst) {
            warning(entry_id);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct ProcessingRun {
    pub id: i64,
    pub entry_id: i64,
    pub session_id: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: String,
    pub original_text: String,
    pub processed_text: Option<String>,
    pub profile_id: Option<String>,
    pub profile_name: Option<String>,
    pub profile_revision: Option<i64>,
    pub prompt_source: Option<String>,
    pub prompt_template: Option<String>,
    pub elapsed_ms: Option<i64>,
    pub stop_to_output_ms: Option<i64>,
    pub output_outcome: Option<String>,
    pub error_code: Option<String>,
    pub error_detail: Option<String>,
    pub provider_id: Option<String>,
    pub provider_name: Option<String>,
    pub requested_model: Option<String>,
    pub compatibility_cache_hit: bool,
    pub validated_operation: Option<String>,
    pub validated_effect_kind: Option<String>,
    pub details_incomplete: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct ProviderCall {
    pub id: i64,
    pub run_id: i64,
    pub ordinal: i64,
    pub purpose: String,
    pub retry_of: Option<i64>,
    pub provider_id: String,
    pub provider_name: String,
    pub endpoint_label: String,
    pub requested_model: String,
    pub reported_model: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub elapsed_ms: Option<i64>,
    pub http_status: Option<i64>,
    pub outcome: String,
    pub error_code: Option<String>,
    pub provider_request_id: Option<String>,
    pub usage_json: Option<String>,
    pub rate_json: Option<String>,
    pub cost_usd: Option<String>,
    pub archive_status: String,
    pub request_bytes: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct RunDetail {
    pub run: ProcessingRun,
    pub calls: Vec<ProviderCall>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct RequestContents {
    pub call_id: i64,
    pub archive_status: String,
    pub request_json: Option<String>,
    pub byte_count: i64,
    pub prompt_template: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct RunSummary {
    pub id: i64,
    pub status: String,
    pub provider_name: Option<String>,
    pub requested_model: Option<String>,
    pub profile_name: Option<String>,
    pub elapsed_ms: Option<i64>,
    pub error_code: Option<String>,
    pub error_detail: Option<String>,
    pub call_count: i64,
    pub known_total_tokens: Option<i64>,
    pub usage_complete: bool,
    pub total_cost_usd: Option<String>,
    pub cost_complete: bool,
    pub details_incomplete: bool,
    pub http_status: Option<i64>,
}

#[derive(Default)]
pub struct RunSnapshot {
    pub session_id: Option<String>,
    pub profile_id: Option<String>,
    pub profile_name: Option<String>,
    pub profile_revision: Option<i64>,
    pub prompt_source: Option<String>,
    pub prompt_template: Option<String>,
    pub provider_id: Option<String>,
    pub provider_name: Option<String>,
    pub requested_model: Option<String>,
}

pub struct RunGuard {
    db_path: PathBuf,
    id: i64,
    started: Instant,
    finished: bool,
    archive_enabled: bool,
    entry_id: i64,
    warning: Option<WarningCallback>,
    incomplete: Arc<AtomicBool>,
    warned: Arc<AtomicBool>,
}

impl RunGuard {
    pub fn start(
        path: &Path,
        entry_id: i64,
        original: &str,
        snapshot: RunSnapshot,
        archive_enabled: bool,
    ) -> Result<Self> {
        let conn = connect(path)?;
        let prompt_template = if archive_enabled {
            snapshot
                .prompt_template
                .filter(|template| template.len() <= ARCHIVE_LIMIT)
        } else {
            None
        };
        conn.execute(
            "INSERT INTO history_processing_runs (entry_id,session_id,started_at,status,original_text,profile_id,profile_name,profile_revision,prompt_source,prompt_template,provider_id,provider_name,requested_model) SELECT id,?2,?3,'running',?4,?5,?6,?7,?8,?9,?10,?11,?12 FROM transcription_history WHERE id=?1",
            params![entry_id, snapshot.session_id, Utc::now().to_rfc3339(), original, snapshot.profile_id, snapshot.profile_name, snapshot.profile_revision, snapshot.prompt_source, prompt_template, snapshot.provider_id, snapshot.provider_name, snapshot.requested_model],
        )?;
        if conn.changes() == 0 {
            return Err(anyhow!("History entry was deleted"));
        }
        Ok(Self {
            db_path: path.to_owned(),
            id: conn.last_insert_rowid(),
            started: Instant::now(),
            finished: false,
            archive_enabled,
            entry_id,
            warning: None,
            incomplete: Arc::new(AtomicBool::new(false)),
            warned: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn attach_warning(&mut self, warning: WarningCallback) {
        self.warning = Some(warning);
    }

    pub fn mark_incomplete(&self) {
        self.incomplete.store(true, Ordering::SeqCst);
        note_incomplete(
            &self.db_path,
            self.id,
            self.entry_id,
            self.warning.as_ref(),
            &self.warned,
        );
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn latest_call_id(&self) -> Result<Option<i64>> {
        let conn = connect(&self.db_path)?;
        Ok(conn.query_row("SELECT id FROM history_provider_calls WHERE run_id=?1 ORDER BY ordinal DESC LIMIT 1", [self.id], |row| row.get(0)).optional()?)
    }

    pub fn set_profile(
        &self,
        profile_id: &str,
        profile_name: &str,
        revision: u64,
        prompt_source: &str,
        prompt_template: &str,
    ) -> Result<()> {
        let conn = connect(&self.db_path)?;
        conn.execute("UPDATE history_processing_runs SET profile_id=?2,profile_name=?3,profile_revision=?4,prompt_source=?5,prompt_template=?6 WHERE id=?1 AND status='running'", params![self.id,profile_id,profile_name,revision as i64,prompt_source,(self.archive_enabled && prompt_template.len() <= ARCHIVE_LIMIT).then_some(prompt_template)])?;
        Ok(())
    }

    pub fn mark_compatibility_cache_hit(&self) -> Result<()> {
        let conn = connect(&self.db_path)?;
        conn.execute("UPDATE history_processing_runs SET compatibility_cache_hit=1 WHERE id=?1 AND status='running'", [self.id])?;
        Ok(())
    }

    pub fn set_validated_action(&self, operation: &str, effect_kind: &str) -> Result<()> {
        let conn = connect(&self.db_path)?;
        conn.execute("UPDATE history_processing_runs SET validated_operation=?2,validated_effect_kind=?3 WHERE id=?1 AND status='running'", params![self.id,operation,effect_kind])?;
        Ok(())
    }

    pub fn set_output(&self, outcome: &str) -> Result<()> {
        let conn = connect(&self.db_path)?;
        conn.execute(
            "UPDATE history_processing_runs SET output_outcome=?2 WHERE id=?1",
            params![self.id, outcome],
        )?;
        Ok(())
    }

    pub fn set_processed_candidate(&self, text: &str) -> Result<()> {
        let conn = connect(&self.db_path)?;
        conn.execute(
            "UPDATE history_processing_runs SET processed_text=?2 WHERE id=?1 AND status='running'",
            params![self.id, text],
        )?;
        Ok(())
    }

    pub fn mark_last_call_timeout(&self) -> Result<()> {
        let conn = connect(&self.db_path)?;
        conn.execute("UPDATE history_provider_calls SET outcome='failed',error_code='timeout' WHERE id=(SELECT id FROM history_provider_calls WHERE run_id=?1 ORDER BY ordinal DESC LIMIT 1) AND outcome='cancelled'", [self.id])?;
        Ok(())
    }

    pub fn start_call(
        &self,
        purpose: &str,
        retry_of: Option<i64>,
        provider_id: &str,
        provider_name: &str,
        endpoint_label: &str,
        model: &str,
        request_json: &str,
    ) -> Result<CallGuard> {
        let mut conn = connect(&self.db_path)?;
        let tx = conn.transaction()?;
        let ordinal: i64 = tx.query_row(
            "SELECT COALESCE(MAX(ordinal),0)+1 FROM history_provider_calls WHERE run_id=?1",
            [self.id],
            |row| row.get(0),
        )?;
        let endpoint = safe_endpoint(endpoint_label);
        tx.execute("INSERT INTO history_provider_calls (run_id,ordinal,purpose,retry_of,provider_id,provider_name,endpoint_label,requested_model,started_at,outcome) SELECT id,?2,?3,?4,?5,?6,?7,?8,?9,'dispatched' FROM history_processing_runs WHERE id=?1", params![self.id, ordinal, purpose, retry_of, provider_id, provider_name, endpoint, model, Utc::now().to_rfc3339()])?;
        if tx.changes() == 0 {
            return Err(anyhow!("Processing run was deleted"));
        }
        let id = tx.last_insert_rowid();
        let size = request_json.len();
        let status = if !self.archive_enabled {
            "disabled"
        } else if size > ARCHIVE_LIMIT {
            "omitted_too_large"
        } else {
            "retained"
        };
        let body = (status == "retained").then_some(request_json);
        tx.execute("INSERT INTO history_request_contents (call_id,archive_status,request_json,byte_count) VALUES (?1,?2,?3,?4)", params![id, status, body, size as i64])?;
        tx.commit()?;
        Ok(CallGuard {
            db_path: self.db_path.clone(),
            id,
            started: Instant::now(),
            finished: false,
            provider_id: provider_id.to_owned(),
            base_url: endpoint_label
                .trim_end_matches("/chat/completions")
                .to_owned(),
            requested_model: model.to_owned(),
            run_id: self.id,
            entry_id: self.entry_id,
            warning: self.warning.clone(),
            incomplete: Arc::clone(&self.incomplete),
            warned: Arc::clone(&self.warned),
        })
    }

    pub fn finish(
        &mut self,
        status: &str,
        processed_text: Option<&str>,
        error_code: Option<&str>,
        error_detail: Option<&str>,
        output_outcome: Option<&str>,
        stop_to_output_ms: Option<i64>,
    ) -> Result<()> {
        let result = (|| {
            let conn = connect(&self.db_path)?;
            let changed = conn.execute("UPDATE history_processing_runs SET status=?2,processed_text=COALESCE(?3,processed_text),error_code=?4,error_detail=?5,output_outcome=?6,stop_to_output_ms=?7,ended_at=?8,elapsed_ms=?9,details_incomplete=(details_incomplete OR ?10) WHERE id=?1 AND status='running'", params![self.id,status,processed_text,error_code,error_detail,output_outcome,stop_to_output_ms,Utc::now().to_rfc3339(),self.started.elapsed().as_millis() as i64,self.incomplete.load(Ordering::SeqCst)])?;
            if changed == 0 {
                return Err(anyhow!("Processing run was deleted or finalized"));
            }
            Ok(())
        })();
        self.finished = true;
        if result.is_err() {
            self.mark_incomplete();
        }
        result
    }
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        if !self.finished {
            if let Ok(conn) = connect(&self.db_path) {
                if let Err(error) = conn.execute("UPDATE history_processing_runs SET status='cancelled',ended_at=?2,elapsed_ms=?3,details_incomplete=(details_incomplete OR ?4) WHERE id=?1 AND status='running'", params![self.id,Utc::now().to_rfc3339(),self.started.elapsed().as_millis() as i64,self.incomplete.load(Ordering::SeqCst)]) { log::error!("Details could not be saved: {error}"); self.mark_incomplete(); }
            } else {
                self.mark_incomplete();
            }
        }
    }
}

pub struct CallGuard {
    db_path: PathBuf,
    id: i64,
    started: Instant,
    finished: bool,
    provider_id: String,
    base_url: String,
    requested_model: String,
    run_id: i64,
    entry_id: i64,
    warning: Option<WarningCallback>,
    incomplete: Arc<AtomicBool>,
    warned: Arc<AtomicBool>,
}

impl CallGuard {
    pub fn id(&self) -> i64 {
        self.id
    }
    pub fn finish(
        &mut self,
        outcome: &str,
        http_status: Option<u16>,
        reported_model: Option<&str>,
        request_id: Option<&str>,
        usage_json: Option<&str>,
        error_code: Option<&str>,
    ) -> Result<()> {
        let (rate_json, cost_usd) =
            if reported_model.is_some_and(|reported| reported != self.requested_model) {
                (None, None)
            } else {
                super::history_pricing::estimate(
                    &self.provider_id,
                    &self.base_url,
                    &self.requested_model,
                    usage_json,
                )
            };
        let result = (|| {
            let conn = connect(&self.db_path)?;
            let changed = conn.execute("UPDATE history_provider_calls SET outcome=?2,http_status=?3,reported_model=?4,provider_request_id=?5,usage_json=?6,error_code=?7,ended_at=?8,elapsed_ms=?9,rate_json=?10,cost_usd=?11 WHERE id=?1 AND outcome='dispatched'", params![self.id,outcome,http_status,reported_model,request_id,usage_json,error_code,Utc::now().to_rfc3339(),self.started.elapsed().as_millis() as i64,rate_json,cost_usd])?;
            if changed == 0 {
                return Err(anyhow!("Provider call was deleted or finalized"));
            }
            Ok(())
        })();
        self.finished = true;
        if result.is_err() {
            self.incomplete.store(true, Ordering::SeqCst);
            note_incomplete(
                &self.db_path,
                self.run_id,
                self.entry_id,
                self.warning.as_ref(),
                &self.warned,
            );
        }
        result
    }
}

impl Drop for CallGuard {
    fn drop(&mut self) {
        if !self.finished {
            if let Ok(conn) = connect(&self.db_path) {
                if let Err(error) = conn.execute("UPDATE history_provider_calls SET outcome='cancelled',ended_at=?2,elapsed_ms=?3 WHERE id=?1 AND outcome='dispatched'", params![self.id,Utc::now().to_rfc3339(),self.started.elapsed().as_millis() as i64]) { log::error!("Details could not be saved: {error}"); self.incomplete.store(true, Ordering::SeqCst); note_incomplete(&self.db_path, self.run_id, self.entry_id, self.warning.as_ref(), &self.warned); }
            } else {
                self.incomplete.store(true, Ordering::SeqCst);
                note_incomplete(
                    &self.db_path,
                    self.run_id,
                    self.entry_id,
                    self.warning.as_ref(),
                    &self.warned,
                );
            }
        }
    }
}

pub fn safe_endpoint(raw: &str) -> String {
    let Ok(mut url) = reqwest::Url::parse(raw) else {
        return "<invalid endpoint>".into();
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_path("");
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

pub fn reconcile_interrupted(path: &Path) -> Result<()> {
    let conn = connect(path)?;
    conn.execute("UPDATE history_processing_runs SET status='interrupted',ended_at=?1 WHERE status='running'", [Utc::now().to_rfc3339()])?;
    conn.execute("UPDATE history_provider_calls SET outcome='interrupted',ended_at=?1 WHERE outcome='dispatched'", [Utc::now().to_rfc3339()])?;
    Ok(())
}

pub fn list_runs(path: &Path, entry_id: i64) -> Result<Vec<ProcessingRun>> {
    let conn = connect(path)?;
    let mut stmt = conn.prepare("SELECT id,entry_id,session_id,started_at,ended_at,status,original_text,processed_text,profile_id,profile_name,profile_revision,prompt_source,prompt_template,elapsed_ms,stop_to_output_ms,output_outcome,error_code,error_detail,provider_id,provider_name,requested_model,compatibility_cache_hit,validated_operation,validated_effect_kind,details_incomplete FROM history_processing_runs WHERE entry_id=?1 ORDER BY id DESC")?;
    let runs = stmt
        .query_map([entry_id], map_run)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(runs)
}

fn map_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProcessingRun> {
    Ok(ProcessingRun {
        id: row.get(0)?,
        entry_id: row.get(1)?,
        session_id: row.get(2)?,
        started_at: row.get(3)?,
        ended_at: row.get(4)?,
        status: row.get(5)?,
        original_text: row.get(6)?,
        processed_text: row.get(7)?,
        profile_id: row.get(8)?,
        profile_name: row.get(9)?,
        profile_revision: row.get(10)?,
        prompt_source: row.get(11)?,
        prompt_template: row.get(12)?,
        elapsed_ms: row.get(13)?,
        stop_to_output_ms: row.get(14)?,
        output_outcome: row.get(15)?,
        error_code: row.get(16)?,
        error_detail: row.get(17)?,
        provider_id: row.get(18)?,
        provider_name: row.get(19)?,
        requested_model: row.get(20)?,
        compatibility_cache_hit: row.get(21)?,
        validated_operation: row.get(22)?,
        validated_effect_kind: row.get(23)?,
        details_incomplete: row.get(24)?,
    })
}

pub fn get_run(path: &Path, id: i64) -> Result<Option<RunDetail>> {
    let conn = connect(path)?;
    let run = conn.query_row("SELECT id,entry_id,session_id,started_at,ended_at,status,original_text,processed_text,profile_id,profile_name,profile_revision,prompt_source,prompt_template,elapsed_ms,stop_to_output_ms,output_outcome,error_code,error_detail,provider_id,provider_name,requested_model,compatibility_cache_hit,validated_operation,validated_effect_kind,details_incomplete FROM history_processing_runs WHERE id=?1", [id], map_run).optional()?;
    let Some(run) = run else { return Ok(None) };
    let mut stmt = conn.prepare("SELECT c.id,c.run_id,c.ordinal,c.purpose,c.retry_of,c.provider_id,c.provider_name,c.endpoint_label,c.requested_model,c.reported_model,c.started_at,c.ended_at,c.elapsed_ms,c.http_status,c.outcome,c.error_code,c.provider_request_id,c.usage_json,c.rate_json,c.cost_usd,COALESCE(r.archive_status,'not_recorded'),COALESCE(r.byte_count,0) FROM history_provider_calls c LEFT JOIN history_request_contents r ON r.call_id=c.id WHERE c.run_id=?1 ORDER BY c.ordinal")?;
    let calls = stmt
        .query_map([id], |row| {
            Ok(ProviderCall {
                id: row.get(0)?,
                run_id: row.get(1)?,
                ordinal: row.get(2)?,
                purpose: row.get(3)?,
                retry_of: row.get(4)?,
                provider_id: row.get(5)?,
                provider_name: row.get(6)?,
                endpoint_label: row.get(7)?,
                requested_model: row.get(8)?,
                reported_model: row.get(9)?,
                started_at: row.get(10)?,
                ended_at: row.get(11)?,
                elapsed_ms: row.get(12)?,
                http_status: row.get(13)?,
                outcome: row.get(14)?,
                error_code: row.get(15)?,
                provider_request_id: row.get(16)?,
                usage_json: row.get(17)?,
                rate_json: row.get(18)?,
                cost_usd: row.get(19)?,
                archive_status: row.get(20)?,
                request_bytes: row.get(21)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Some(RunDetail { run, calls }))
}

pub fn get_request(path: &Path, call_id: i64) -> Result<Option<RequestContents>> {
    let conn = connect(path)?;
    Ok(conn.query_row("SELECT call_id,archive_status,request_json,byte_count,prompt_template FROM history_request_contents WHERE call_id=?1", [call_id], |row| Ok(RequestContents { call_id:row.get(0)?,archive_status:row.get(1)?,request_json:row.get(2)?,byte_count:row.get(3)?,prompt_template:row.get(4)? })).optional()?)
}

pub fn clear_requests(path: &Path) -> Result<usize> {
    let mut conn = connect(path)?;
    let tx = conn.transaction()?;
    let count = tx.execute("UPDATE history_request_contents SET request_json=NULL,prompt_template=NULL,archive_status='cleared' WHERE request_json IS NOT NULL OR prompt_template IS NOT NULL", [])?;
    tx.execute(
        "UPDATE history_processing_runs SET prompt_template=NULL WHERE prompt_template IS NOT NULL",
        [],
    )?;
    tx.execute("UPDATE transcription_history SET post_process_prompt=NULL WHERE post_process_prompt IS NOT NULL", [])?;
    tx.commit()?;
    Ok(count)
}

pub fn set_output(
    path: &Path,
    run_id: i64,
    outcome: &str,
    stop_to_output_ms: Option<i64>,
) -> Result<()> {
    let conn = connect(path)?;
    conn.execute(
        "UPDATE history_processing_runs SET output_outcome=?2,stop_to_output_ms=?3 WHERE id=?1",
        params![run_id, outcome, stop_to_output_ms],
    )?;
    Ok(())
}

pub fn summary(path: &Path, entry_id: i64) -> Result<Option<RunSummary>> {
    let conn = connect(path)?;
    let mut summary = conn.query_row("SELECT r.id,r.status,COALESCE((SELECT c.provider_name FROM history_provider_calls c WHERE c.run_id=r.id ORDER BY c.ordinal DESC LIMIT 1),r.provider_name),COALESCE((SELECT c.requested_model FROM history_provider_calls c WHERE c.run_id=r.id ORDER BY c.ordinal DESC LIMIT 1),r.requested_model),r.profile_name,r.elapsed_ms,CASE WHEN r.error_code='processing_failed' THEN COALESCE((SELECT c.error_code FROM history_provider_calls c WHERE c.run_id=r.id AND c.error_code IS NOT NULL ORDER BY c.ordinal DESC LIMIT 1),r.error_code) ELSE r.error_code END,r.error_detail,(SELECT COUNT(*) FROM history_provider_calls c WHERE c.run_id=r.id),r.details_incomplete,(SELECT c.http_status FROM history_provider_calls c WHERE c.run_id=r.id AND c.http_status IS NOT NULL ORDER BY c.ordinal DESC LIMIT 1) FROM history_processing_runs r WHERE r.entry_id=?1 ORDER BY r.id DESC LIMIT 1", [entry_id], |row| Ok(RunSummary { id:row.get(0)?,status:row.get(1)?,provider_name:row.get(2)?,requested_model:row.get(3)?,profile_name:row.get(4)?,elapsed_ms:row.get(5)?,error_code:row.get(6)?,error_detail:row.get(7)?,call_count:row.get(8)?,details_incomplete:row.get(9)?,http_status:row.get(10)?,known_total_tokens:None,usage_complete:false,total_cost_usd:None,cost_complete:false })).optional()?;
    if let Some(ref mut result) = summary {
        let mut stmt =
            conn.prepare("SELECT usage_json,cost_usd FROM history_provider_calls WHERE run_id=?1")?;
        let rows = stmt.query_map([result.id], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
            ))
        })?;
        let mut token_total = 0_i64;
        let mut cost_total = 0_u128;
        let mut token_count = 0_i64;
        let mut cost_count = 0_i64;
        for row in rows {
            let (usage, cost) = row?;
            if let Some(tokens) = usage
                .as_deref()
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
                .and_then(|value| {
                    value
                        .get("total_tokens")
                        .and_then(serde_json::Value::as_i64)
                })
            {
                token_total = token_total.saturating_add(tokens);
                token_count += 1;
            }
            if let Some(nanodollars) = cost.as_deref().and_then(parse_usd_nanodollars) {
                cost_total = cost_total.saturating_add(nanodollars);
                cost_count += 1;
            }
        }
        result.known_total_tokens = (token_count > 0).then_some(token_total);
        result.usage_complete =
            !result.details_incomplete && result.call_count > 0 && token_count == result.call_count;
        result.total_cost_usd = (cost_count > 0).then(|| {
            format!(
                "{}.{:09}",
                cost_total / 1_000_000_000,
                cost_total % 1_000_000_000
            )
        });
        result.cost_complete =
            !result.details_incomplete && result.call_count > 0 && cost_count == result.call_count;
    }
    Ok(summary)
}

fn parse_usd_nanodollars(cost: &str) -> Option<u128> {
    let (whole, fraction) = cost.split_once('.')?;
    if fraction.len() != 9 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    whole
        .parse::<u128>()
        .ok()?
        .checked_mul(1_000_000_000)?
        .checked_add(fraction.parse::<u128>().ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite_migration::Migrations;

    fn database() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        let mut conn = connect(&path).unwrap();
        let migrations = Migrations::new(super::super::history::MIGRATIONS.to_vec());
        migrations.to_version(&mut conn, 4).unwrap();
        conn.execute("INSERT INTO transcription_history (id,file_name,timestamp,title,transcription_text,post_process_requested) VALUES (1,'one.wav',123,'Old','original',1)", []).unwrap();
        migrations.to_latest(&mut conn).unwrap();
        (dir, path)
    }

    #[test]
    fn migration_preserves_v4_entry_and_cascades_all_evidence() {
        let (_dir, path) = database();
        let mut run = RunGuard::start(&path, 1, "original", RunSnapshot::default(), true).unwrap();
        let mut call = run
            .start_call(
                "rewrite",
                None,
                "openai",
                "OpenAI",
                "https://user:password@api.openai.com/v1?key=SECRET",
                "model",
                "{\"messages\":[]}",
            )
            .unwrap();
        call.finish(
            "succeeded",
            Some(200),
            None,
            None,
            Some("{\"input_tokens\":2}"),
            None,
        )
        .unwrap();
        run.finish(
            "succeeded",
            Some("processed"),
            None,
            None,
            Some("dispatched"),
            None,
        )
        .unwrap();
        assert_eq!(
            get_request(&path, call.id())
                .unwrap()
                .unwrap()
                .request_json
                .as_deref(),
            Some("{\"messages\":[]}")
        );
        assert!(!get_run(&path, run.id()).unwrap().unwrap().calls[0]
            .endpoint_label
            .contains("SECRET"));
        let conn = connect(&path).unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT transcription_text FROM transcription_history WHERE id=1",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "original"
        );
        conn.execute("DELETE FROM transcription_history WHERE id=1", [])
            .unwrap();
        assert!(get_run(&path, run.id()).unwrap().is_none());
        assert!(get_request(&path, call.id()).unwrap().is_none());
    }

    #[test]
    fn cancellation_and_reconciliation_do_not_recreate_deleted_rows() {
        let (_dir, path) = database();
        let run = RunGuard::start(&path, 1, "original", RunSnapshot::default(), true).unwrap();
        let old_id = run.id();
        let call = run
            .start_call(
                "rewrite",
                None,
                "custom",
                "Custom",
                "http://localhost/v1",
                "model",
                "{}",
            )
            .unwrap();
        let call_id = call.id();
        drop(call);
        drop(run);
        assert_eq!(
            get_run(&path, old_id).unwrap().unwrap().calls[0].outcome,
            "cancelled"
        );
        assert_eq!(
            get_run(&path, old_id).unwrap().unwrap().run.status,
            "cancelled"
        );
        let conn = connect(&path).unwrap();
        conn.execute("DELETE FROM transcription_history WHERE id=1", [])
            .unwrap();
        assert!(get_request(&path, call_id).unwrap().is_none());
        assert!(RunGuard::start(&path, 1, "original", RunSnapshot::default(), true).is_err());
        reconcile_interrupted(&path).unwrap();
    }

    #[test]
    fn restart_marks_unfinished_work_interrupted_and_late_guards_cannot_hit_new_ids() {
        let (_dir, path) = database();
        let run = RunGuard::start(&path, 1, "original", RunSnapshot::default(), true).unwrap();
        let old_id = run.id();
        let call = run
            .start_call(
                "rewrite",
                None,
                "custom",
                "Custom",
                "http://localhost/v1",
                "model",
                "{}",
            )
            .unwrap();
        reconcile_interrupted(&path).unwrap();
        assert_eq!(
            get_run(&path, old_id).unwrap().unwrap().run.status,
            "interrupted"
        );
        assert_eq!(
            get_run(&path, old_id).unwrap().unwrap().calls[0].outcome,
            "interrupted"
        );
        let conn = connect(&path).unwrap();
        conn.execute("DELETE FROM transcription_history WHERE id=1", [])
            .unwrap();
        conn.execute("INSERT INTO transcription_history (file_name,timestamp,title,transcription_text) VALUES ('two.wav',124,'New','new')", []).unwrap();
        let new_entry = conn.last_insert_rowid();
        let new_run =
            RunGuard::start(&path, new_entry, "new", RunSnapshot::default(), true).unwrap();
        assert!(new_run.id() > old_id);
        drop(call);
        drop(run);
        assert_eq!(
            get_run(&path, new_run.id()).unwrap().unwrap().run.status,
            "running"
        );
    }

    #[test]
    fn archive_controls_keep_metrics_but_remove_all_templates() {
        let (_dir, path) = database();
        let snapshot = RunSnapshot {
            prompt_template: Some("secret template".into()),
            ..Default::default()
        };
        let mut run = RunGuard::start(&path, 1, "original", snapshot, true).unwrap();
        let mut call = run
            .start_call(
                "rewrite",
                None,
                "custom",
                "Custom",
                "http://localhost/v1",
                "model",
                "{\"secret\":true}",
            )
            .unwrap();
        call.finish(
            "succeeded",
            Some(200),
            None,
            None,
            Some("{\"input_tokens\":42}"),
            None,
        )
        .unwrap();
        run.finish("succeeded", Some("processed"), None, None, None, None)
            .unwrap();
        assert_eq!(clear_requests(&path).unwrap(), 1);
        assert_eq!(
            get_request(&path, call.id())
                .unwrap()
                .unwrap()
                .archive_status,
            "cleared"
        );
        let detail = get_run(&path, run.id()).unwrap().unwrap();
        assert!(detail.run.prompt_template.is_none());
        assert!(detail.calls[0]
            .usage_json
            .as_deref()
            .unwrap()
            .contains("42"));
        let disabled = RunGuard::start(
            &path,
            1,
            "original",
            RunSnapshot {
                prompt_template: Some("new secret".into()),
                ..Default::default()
            },
            false,
        )
        .unwrap();
        let disabled_call = disabled
            .start_call(
                "rewrite",
                None,
                "custom",
                "Custom",
                "http://localhost/v1",
                "model",
                "{\"new\":true}",
            )
            .unwrap();
        assert_eq!(
            get_request(&path, disabled_call.id())
                .unwrap()
                .unwrap()
                .archive_status,
            "disabled"
        );
        assert!(get_run(&path, disabled.id())
            .unwrap()
            .unwrap()
            .run
            .prompt_template
            .is_none());
    }

    #[test]
    fn oversize_archive_is_marked_without_truncating_or_storing_text() {
        let (_dir, path) = database();
        let run = RunGuard::start(&path, 1, "original", RunSnapshot::default(), true).unwrap();
        let body = "x".repeat(ARCHIVE_LIMIT + 1);
        let call = run
            .start_call(
                "rewrite",
                None,
                "custom",
                "Custom",
                "http://localhost/v1",
                "model",
                &body,
            )
            .unwrap();
        let request = get_request(&path, call.id()).unwrap().unwrap();
        assert_eq!(request.archive_status, "omitted_too_large");
        assert!(request.request_json.is_none());
        assert_eq!(request.byte_count, body.len() as i64);
    }

    #[test]
    fn transient_metadata_failure_stays_incomplete_after_database_recovers() {
        let (_dir, path) = database();
        let mut run = RunGuard::start(&path, 1, "original", RunSnapshot::default(), true).unwrap();
        let hidden = path.with_extension("hidden");
        std::fs::rename(&path, &hidden).unwrap();
        run.mark_incomplete();
        std::fs::remove_file(&path).unwrap();
        std::fs::rename(&hidden, &path).unwrap();
        run.finish("succeeded", Some("processed"), None, None, None, None)
            .unwrap();
        assert!(
            get_run(&path, run.id())
                .unwrap()
                .unwrap()
                .run
                .details_incomplete
        );
        assert!(summary(&path, 1).unwrap().unwrap().details_incomplete);
        assert!(!summary(&path, 1).unwrap().unwrap().cost_complete);
    }
}
