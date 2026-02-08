use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::{mpsc, Mutex};

use crate::backend::app_server::{
    build_codex_command_with_bin, check_cli_installation, CliAdapter, CliSpawnConfig,
    WorkspaceSession,
};
use crate::backend::events::{AppServerEvent, EventSink};
use crate::shared::process_core::kill_child_process_tree;
use crate::types::WorkspaceEntry;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct ThreadMetadata {
    #[serde(alias = "claude_session_id")]
    pub(crate) cli_session_id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) created_at: u64,
    pub(crate) updated_at: u64,
    pub(crate) archived: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub(crate) struct ThreadStore {
    pub(crate) threads: HashMap<String, ThreadMetadata>,
}

impl ThreadStore {
    pub(crate) fn load(path: &PathBuf) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub(crate) fn save(&self, path: &PathBuf) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create thread store directory: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| format!("Failed to write thread store: {e}"))
    }
}

pub(crate) fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(crate) fn thread_store_path(workspace_id: &str) -> PathBuf {
    let data_dir = dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("agent-monitor")
        .join("adapter-threads");
    data_dir.join(format!("{workspace_id}.json"))
}

pub(crate) trait CliProfile: Send + Sync + 'static {
    fn build_turn_command(
        &self,
        config: &CliSpawnConfig,
        session_id: Option<&str>,
        prompt: &str,
        cwd: &str,
        params: &Value,
    ) -> Result<tokio::process::Command, String>;

    fn parse_stream_line(&self, line: &str, thread_id: &str, turn_id: &str) -> Option<Value>;

    fn extract_session_id(&self, line: &str) -> Option<String>;

    fn model_list(&self) -> Value;

    fn provider_name(&self) -> &str;
}

pub(crate) struct GenericAdapterSession<P: CliProfile> {
    profile: Arc<P>,
    workspace_id: String,
    cwd: String,
    config: CliSpawnConfig,
    thread_store_path: PathBuf,
    thread_store: Arc<Mutex<ThreadStore>>,
    active_child: Arc<Mutex<Option<Child>>>,
    event_emitter: Arc<dyn Fn(AppServerEvent) + Send + Sync>,
    background_callbacks: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<Value>>>>,
}

impl<P: CliProfile> GenericAdapterSession<P> {
    pub(crate) fn new(
        profile: P,
        entry: &WorkspaceEntry,
        config: CliSpawnConfig,
        event_emitter: Arc<dyn Fn(AppServerEvent) + Send + Sync>,
        background_callbacks: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<Value>>>>,
    ) -> Self {
        let store_path = thread_store_path(&entry.id);
        let store = ThreadStore::load(&store_path);
        Self {
            profile: Arc::new(profile),
            workspace_id: entry.id.clone(),
            cwd: entry.path.clone(),
            config,
            thread_store_path: store_path,
            thread_store: Arc::new(Mutex::new(store)),
            active_child: Arc::new(Mutex::new(None)),
            event_emitter,
            background_callbacks,
        }
    }

    async fn handle_thread_start(&self) -> Result<Value, String> {
        let thread_id = uuid::Uuid::new_v4().to_string();
        let now = now_epoch();
        let meta = ThreadMetadata {
            cli_session_id: None,
            name: None,
            created_at: now,
            updated_at: now,
            archived: false,
        };
        {
            let mut store = self.thread_store.lock().await;
            store.threads.insert(thread_id.clone(), meta);
            store.save(&self.thread_store_path)?;
        }
        Ok(json!({
            "result": {
                "threadId": thread_id,
                "thread": { "id": thread_id }
            }
        }))
    }

    async fn handle_thread_resume(&self, params: &Value) -> Result<Value, String> {
        let thread_id = params
            .get("threadId")
            .and_then(|v| v.as_str())
            .ok_or("missing threadId")?;
        let store = self.thread_store.lock().await;
        if !store.threads.contains_key(thread_id) {
            return Err("thread not found".to_string());
        }
        Ok(json!({
            "result": {
                "threadId": thread_id,
                "thread": { "id": thread_id }
            }
        }))
    }

    async fn handle_thread_list(&self) -> Result<Value, String> {
        let store = self.thread_store.lock().await;
        let threads: Vec<Value> = store
            .threads
            .iter()
            .filter(|(_, meta)| !meta.archived)
            .map(|(id, meta)| {
                json!({
                    "id": id,
                    "name": meta.name,
                    "createdAt": meta.created_at,
                    "updatedAt": meta.updated_at,
                    "archived": meta.archived,
                })
            })
            .collect();
        Ok(json!({
            "result": {
                "threads": threads,
                "hasMore": false
            }
        }))
    }

    async fn handle_thread_archive(&self, params: &Value) -> Result<Value, String> {
        let thread_id = params
            .get("threadId")
            .and_then(|v| v.as_str())
            .ok_or("missing threadId")?;
        let mut store = self.thread_store.lock().await;
        if let Some(meta) = store.threads.get_mut(thread_id) {
            meta.archived = true;
            meta.updated_at = now_epoch();
        }
        store.save(&self.thread_store_path)?;
        Ok(json!({ "result": {} }))
    }

    async fn handle_thread_name_set(&self, params: &Value) -> Result<Value, String> {
        let thread_id = params
            .get("threadId")
            .and_then(|v| v.as_str())
            .ok_or("missing threadId")?;
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let mut store = self.thread_store.lock().await;
        if let Some(meta) = store.threads.get_mut(thread_id) {
            meta.name = Some(name.to_string());
            meta.updated_at = now_epoch();
        }
        store.save(&self.thread_store_path)?;
        Ok(json!({ "result": {} }))
    }

    async fn handle_thread_fork(&self, params: &Value) -> Result<Value, String> {
        let source_id = params
            .get("threadId")
            .and_then(|v| v.as_str())
            .ok_or("missing threadId")?;
        let mut store = self.thread_store.lock().await;
        let source = store
            .threads
            .get(source_id)
            .cloned()
            .ok_or("thread not found")?;
        let new_id = uuid::Uuid::new_v4().to_string();
        let now = now_epoch();
        let meta = ThreadMetadata {
            cli_session_id: None,
            name: source.name.map(|n| format!("{n} (fork)")),
            created_at: now,
            updated_at: now,
            archived: false,
        };
        store.threads.insert(new_id.clone(), meta);
        store.save(&self.thread_store_path)?;
        Ok(json!({
            "result": {
                "threadId": new_id,
                "thread": { "id": new_id }
            }
        }))
    }

    async fn handle_turn_start(&self, params: &Value) -> Result<Value, String> {
        let thread_id = params
            .get("threadId")
            .and_then(|v| v.as_str())
            .ok_or("missing threadId")?
            .to_string();
        let prompt = params
            .get("input")
            .and_then(|v| v.as_str())
            .ok_or("missing input")?
            .to_string();
        let turn_id = uuid::Uuid::new_v4().to_string();

        let session_id = {
            let store = self.thread_store.lock().await;
            store
                .threads
                .get(&thread_id)
                .and_then(|meta| meta.cli_session_id.clone())
        };

        {
            let mut guard = self.active_child.lock().await;
            if let Some(mut prev) = guard.take() {
                kill_child_process_tree(&mut prev).await;
            }
        }

        let mut command = self.profile.build_turn_command(
            &self.config,
            session_id.as_deref(),
            &prompt,
            &self.cwd,
            params,
        )?;
        let mut child = command
            .spawn()
            .map_err(|e| format!("Failed to spawn CLI: {e}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Failed to capture CLI stdout")?;
        let stderr = child.stderr.take();

        {
            let mut guard = self.active_child.lock().await;
            *guard = Some(child);
        }

        let profile = self.profile.clone();
        let emitter = self.event_emitter.clone();
        let ws_id = self.workspace_id.clone();
        let store = self.thread_store.clone();
        let store_path = self.thread_store_path.clone();
        let active_child = self.active_child.clone();
        let bg_callbacks = self.background_callbacks.clone();
        let thread_id_bg = thread_id.clone();
        let turn_id_bg = turn_id.clone();

        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            let mut got_result = false;

            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(sid) = profile.extract_session_id(&line) {
                    let mut s = store.lock().await;
                    if let Some(meta) = s.threads.get_mut(&thread_id_bg) {
                        meta.cli_session_id = Some(sid);
                        meta.updated_at = now_epoch();
                        if let Err(e) = s.save(&store_path) {
                            eprintln!("adapter: failed to persist session id: {e}");
                        }
                    }
                }

                if let Some(event) =
                    profile.parse_stream_line(&line, &thread_id_bg, &turn_id_bg)
                {
                    if event.get("method").and_then(|m| m.as_str()) == Some("turn/completed") {
                        got_result = true;
                    }
                    let mut sent_to_background = false;
                    {
                        let callbacks = bg_callbacks.lock().await;
                        if let Some(tx) = callbacks.get(&thread_id_bg) {
                            let _ = tx.send(event.clone());
                            sent_to_background = true;
                        }
                    }
                    if !sent_to_background {
                        (emitter)(AppServerEvent {
                            workspace_id: ws_id.clone(),
                            message: event,
                        });
                    }
                }
            }

            if !got_result {
                let fallback_event = json!({
                    "method": "turn/completed",
                    "params": {
                        "threadId": thread_id_bg,
                        "turnId": turn_id_bg
                    }
                });
                let mut sent_to_background = false;
                {
                    let callbacks = bg_callbacks.lock().await;
                    if let Some(tx) = callbacks.get(&thread_id_bg) {
                        let _ = tx.send(fallback_event.clone());
                        sent_to_background = true;
                    }
                }
                if !sent_to_background {
                    (emitter)(AppServerEvent {
                        workspace_id: ws_id,
                        message: fallback_event,
                    });
                }
            }

            let mut guard = active_child.lock().await;
            if let Some(mut child) = guard.take() {
                let _ = child.wait().await;
            }
        });

        if let Some(stderr) = stderr {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(_)) = lines.next_line().await {}
            });
        }

        Ok(json!({
            "result": {
                "turn": { "id": turn_id },
                "threadId": thread_id
            }
        }))
    }
}

#[async_trait::async_trait]
impl<P: CliProfile> CliAdapter for GenericAdapterSession<P> {
    async fn send_request(&self, method: &str, params: Value) -> Result<Value, String> {
        let provider = self.profile.provider_name();
        match method {
            "initialize" => Ok(json!({
                "result": {
                    "serverInfo": {
                        "name": format!("{provider}-adapter"),
                        "version": "0.1.0"
                    },
                    "capabilities": {}
                }
            })),
            "thread/start" => self.handle_thread_start().await,
            "thread/resume" => self.handle_thread_resume(&params).await,
            "thread/fork" => self.handle_thread_fork(&params).await,
            "thread/list" => self.handle_thread_list().await,
            "thread/archive" => self.handle_thread_archive(&params).await,
            "thread/compact/start" => Ok(json!({ "result": {} })),
            "thread/name/set" => self.handle_thread_name_set(&params).await,
            "turn/start" => self.handle_turn_start(&params).await,
            "turn/interrupt" => {
                let mut child_guard = self.active_child.lock().await;
                if let Some(mut child) = child_guard.take() {
                    kill_child_process_tree(&mut child).await;
                }
                Ok(json!({ "result": {} }))
            }
            "model/list" => Ok(self.profile.model_list()),
            "account/read" => Ok(json!({ "result": { "provider": provider } })),
            "account/rateLimits/read" => Ok(json!({ "result": Value::Null })),
            "collaborationMode/list" => Ok(json!({ "result": { "modes": [] } })),
            "skills/list" => Ok(json!({ "result": { "skills": [] } })),
            "app/list" => Ok(json!({ "result": { "apps": [] } })),
            "mcpServerStatus/list" => Ok(json!({ "result": { "servers": [] } })),
            _ => Err(format!("unsupported method: {method}")),
        }
    }

    async fn send_notification(&self, _method: &str, _params: Option<Value>) -> Result<(), String> {
        Ok(())
    }

    async fn send_response(&self, _id: Value, _result: Value) -> Result<(), String> {
        Ok(())
    }

    async fn kill(&self) {
        let mut child_guard = self.active_child.lock().await;
        if let Some(mut child) = child_guard.take() {
            kill_child_process_tree(&mut child).await;
        }
    }
}

pub(crate) async fn spawn_adapter_session<P: CliProfile, E: EventSink>(
    profile: P,
    cli_name: &str,
    entry: WorkspaceEntry,
    config: CliSpawnConfig,
    event_sink: E,
) -> Result<Arc<WorkspaceSession>, String> {
    let _ = check_cli_installation(config.cli_bin.clone(), cli_name).await?;

    let event_sink_clone = event_sink.clone();
    let emitter: Arc<dyn Fn(AppServerEvent) + Send + Sync> = Arc::new(move |event| {
        event_sink_clone.emit_app_server_event(event);
    });

    let shared_callbacks = Arc::new(Mutex::new(HashMap::new()));
    let adapter =
        GenericAdapterSession::new(profile, &entry, config, emitter, shared_callbacks.clone());
    let session = Arc::new(WorkspaceSession::new_with_adapter(
        entry.clone(),
        Box::new(adapter),
        shared_callbacks,
    ));

    event_sink.emit_app_server_event(AppServerEvent {
        workspace_id: entry.id.clone(),
        message: json!({
            "method": "codex/connected",
            "params": { "workspaceId": entry.id }
        }),
    });

    Ok(session)
}

// Shared command builder helper used by profiles
pub(crate) fn build_adapter_command(
    config: &CliSpawnConfig,
    args: Vec<String>,
    cwd: &str,
    home_env_var: Option<(&str, &PathBuf)>,
) -> Result<tokio::process::Command, String> {
    let mut command = build_codex_command_with_bin(
        config.cli_bin.clone(),
        config.cli_args.as_deref(),
        args,
    )?;
    command.current_dir(cwd);
    if let Some((var_name, home_path)) = home_env_var {
        command.env(var_name, home_path);
    }
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_store_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!(
            "adapter-base-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let path = temp_dir.join("threads.json");

        let mut store = ThreadStore::default();
        store.threads.insert(
            "t1".to_string(),
            ThreadMetadata {
                cli_session_id: Some("s1".to_string()),
                name: Some("Test Thread".to_string()),
                created_at: 1000,
                updated_at: 2000,
                archived: false,
            },
        );
        store.save(&path).unwrap();

        let loaded = ThreadStore::load(&path);
        assert!(loaded.threads.contains_key("t1"));
        let meta = &loaded.threads["t1"];
        assert_eq!(meta.cli_session_id.as_deref(), Some("s1"));
        assert_eq!(meta.name.as_deref(), Some("Test Thread"));
        assert!(!meta.archived);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn thread_store_load_missing_file_returns_default() {
        let path = PathBuf::from("/tmp/nonexistent-adapter-test.json");
        let store = ThreadStore::load(&path);
        assert!(store.threads.is_empty());
    }

    #[test]
    fn now_epoch_returns_nonzero() {
        assert!(now_epoch() > 0);
    }

    #[test]
    fn thread_store_deserializes_legacy_claude_session_id_field() {
        let legacy_json = r#"{
            "threads": {
                "t1": {
                    "claude_session_id": "old-session",
                    "name": "Legacy Thread",
                    "created_at": 1000,
                    "updated_at": 2000,
                    "archived": false
                }
            }
        }"#;
        let store: ThreadStore = serde_json::from_str(legacy_json).unwrap();
        let meta = &store.threads["t1"];
        assert_eq!(
            meta.cli_session_id.as_deref(),
            Some("old-session"),
            "legacy claude_session_id must deserialize into cli_session_id via serde alias"
        );
    }
}
