use serde_json::{json, Value};
use std::sync::Arc;

use crate::backend::adapter_base::{build_adapter_command, spawn_adapter_session, CliProfile};
use crate::backend::app_server::{CliSpawnConfig, WorkspaceSession};
use crate::backend::events::EventSink;
use crate::types::WorkspaceEntry;

pub(crate) struct ClaudeProfile;

impl CliProfile for ClaudeProfile {
    fn build_turn_command(
        &self,
        config: &CliSpawnConfig,
        session_id: Option<&str>,
        prompt: &str,
        cwd: &str,
        params: &Value,
    ) -> Result<tokio::process::Command, String> {
        let effort = params.get("effort").and_then(|v| v.as_str());
        build_claude_command(config, session_id, prompt, cwd, effort)
    }

    fn parse_stream_line(&self, line: &str, thread_id: &str, turn_id: &str) -> Option<Value> {
        parse_stream_json_line(line, thread_id, turn_id)
    }

    fn extract_session_id(&self, line: &str) -> Option<String> {
        extract_session_id_from_line(line)
    }

    fn model_list(&self) -> Value {
        let standard_efforts = json!([
            { "reasoningEffort": "low", "description": "Fast, minimal thinking" },
            { "reasoningEffort": "medium", "description": "Balanced speed and depth" },
            { "reasoningEffort": "high", "description": "Deep thinking (default)" }
        ]);
        let opus_efforts = json!([
            { "reasoningEffort": "low", "description": "Fast, minimal thinking" },
            { "reasoningEffort": "medium", "description": "Balanced speed and depth" },
            { "reasoningEffort": "high", "description": "Deep thinking (default)" },
            { "reasoningEffort": "max", "description": "Maximum depth, no token limit" }
        ]);
        json!({
            "result": {
                "models": [
                    {
                        "id": "claude-sonnet-4-20250514",
                        "name": "Claude Sonnet 4",
                        "supportedReasoningEfforts": standard_efforts,
                        "defaultReasoningEffort": "high"
                    },
                    {
                        "id": "claude-opus-4-20250514",
                        "name": "Claude Opus 4",
                        "supportedReasoningEfforts": opus_efforts,
                        "defaultReasoningEffort": "high"
                    },
                    {
                        "id": "claude-haiku-4-20250514",
                        "name": "Claude Haiku 4",
                        "supportedReasoningEfforts": standard_efforts,
                        "defaultReasoningEffort": "high"
                    }
                ],
                "defaultModel": "claude-sonnet-4-20250514"
            }
        })
    }

    fn provider_name(&self) -> &str {
        "claude"
    }
}

pub(crate) fn build_claude_command(
    config: &CliSpawnConfig,
    session_id: Option<&str>,
    prompt: &str,
    cwd: &str,
    effort: Option<&str>,
) -> Result<tokio::process::Command, String> {
    let mut args = vec![
        "-p".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--verbose".to_string(),
    ];
    if let Some(sid) = session_id {
        args.push("--resume".to_string());
        args.push(sid.to_string());
    }
    args.push(prompt.to_string());

    let home_env = config.cli_home.as_ref().map(|h| ("CLAUDE_HOME", h));
    let mut command = build_adapter_command(config, args, cwd, home_env)?;

    if let Some(effort_value) = effort {
        if effort_value == "max" {
            command.env("CLAUDE_CODE_EFFORT_LEVEL", "high");
            command.env("CLAUDE_CODE_MAX_THINKING_TOKENS", "128000");
        } else {
            command.env("CLAUDE_CODE_EFFORT_LEVEL", effort_value);
        }
    }

    Ok(command)
}

pub(crate) fn parse_stream_json_line(
    line: &str,
    thread_id: &str,
    turn_id: &str,
) -> Option<Value> {
    let event: Value = serde_json::from_str(line).ok()?;
    let event_type = event.get("type")?.as_str()?;

    let msg_item_id = format!("msg_{turn_id}");

    match event_type {
        "system" => {
            let subtype = event.get("subtype").and_then(|s| s.as_str()).unwrap_or("");
            if subtype == "init" {
                Some(json!({
                    "method": "turn/started",
                    "params": {
                        "threadId": thread_id,
                        "turnId": turn_id
                    }
                }))
            } else {
                None
            }
        }
        "content_block_delta" => {
            let delta = event.get("delta")?;
            let delta_type = delta.get("type")?.as_str()?;
            match delta_type {
                "text_delta" => {
                    let text = delta.get("text")?.as_str()?;
                    Some(json!({
                        "method": "item/agentMessage/delta",
                        "params": {
                            "threadId": thread_id,
                            "turnId": turn_id,
                            "itemId": msg_item_id,
                            "delta": text
                        }
                    }))
                }
                "input_json_delta" => None,
                _ => None,
            }
        }
        "content_block_start" => {
            let block = event.get("content_block")?;
            let block_type = block.get("type")?.as_str()?;
            if block_type == "tool_use" {
                let tool_name = block.get("name").and_then(|n| n.as_str()).unwrap_or("tool");
                let tool_id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                Some(json!({
                    "method": "item/started",
                    "params": {
                        "threadId": thread_id,
                        "turnId": turn_id,
                        "item": {
                            "id": tool_id,
                            "type": "tool_use",
                            "name": tool_name
                        }
                    }
                }))
            } else {
                None
            }
        }
        "tool_result" => {
            let tool_use_id = event.get("tool_use_id").and_then(|i| i.as_str()).unwrap_or("");
            Some(json!({
                "method": "item/completed",
                "params": {
                    "threadId": thread_id,
                    "turnId": turn_id,
                    "item": {
                        "id": tool_use_id,
                        "type": "tool_use"
                    }
                }
            }))
        }
        "result" => {
            Some(json!({
                "method": "turn/completed",
                "params": {
                    "threadId": thread_id,
                    "turnId": turn_id,
                    "costUsd": event.get("cost_usd"),
                    "durationMs": event.get("duration_ms")
                }
            }))
        }
        _ => None,
    }
}

fn extract_session_id_from_line(line: &str) -> Option<String> {
    let event: Value = serde_json::from_str(line).ok()?;
    if event.get("type")?.as_str()? != "system" {
        return None;
    }
    if event.get("subtype").and_then(|s| s.as_str()) != Some("init") {
        return None;
    }
    event
        .get("session_id")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

pub(crate) async fn spawn_claude_session<E: EventSink>(
    entry: WorkspaceEntry,
    config: CliSpawnConfig,
    event_sink: E,
) -> Result<Arc<WorkspaceSession>, String> {
    spawn_adapter_session(ClaudeProfile, "Claude", entry, config, event_sink).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::adapter_base::{GenericAdapterSession, ThreadStore};
    use crate::backend::app_server::CliAdapter;
    use crate::backend::events::AppServerEvent;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    fn test_emitter() -> Arc<dyn Fn(AppServerEvent) + Send + Sync> {
        Arc::new(|_| {})
    }

    fn test_adapter() -> GenericAdapterSession<ClaudeProfile> {
        let entry = crate::types::WorkspaceEntry {
            id: "test-ws".to_string(),
            name: "Test".to_string(),
            path: "/tmp".to_string(),
            codex_bin: None,
            kind: crate::types::WorkspaceKind::Main,
            parent_id: None,
            worktree: None,
            settings: crate::types::WorkspaceSettings::default(),
        };
        let config = CliSpawnConfig {
            cli_type: "claude".to_string(),
            cli_bin: None,
            cli_args: None,
            cli_home: None,
        };
        GenericAdapterSession::new(
            ClaudeProfile,
            &entry,
            config,
            test_emitter(),
            Arc::new(Mutex::new(HashMap::new())),
        )
    }

    #[test]
    fn build_claude_command_basic() {
        let config = CliSpawnConfig {
            cli_type: "claude".to_string(),
            cli_bin: Some("claude".to_string()),
            cli_args: None,
            cli_home: None,
        };
        let result = build_claude_command(&config, None, "hello world", "/tmp", None);
        assert!(result.is_ok());
    }

    #[test]
    fn build_claude_command_with_resume() {
        let config = CliSpawnConfig {
            cli_type: "claude".to_string(),
            cli_bin: Some("claude".to_string()),
            cli_args: None,
            cli_home: None,
        };
        let result = build_claude_command(&config, Some("session-123"), "hello", "/tmp", None);
        assert!(result.is_ok());
    }

    #[test]
    fn build_claude_command_with_effort() {
        let config = CliSpawnConfig {
            cli_type: "claude".to_string(),
            cli_bin: Some("claude".to_string()),
            cli_args: None,
            cli_home: None,
        };
        let result = build_claude_command(&config, None, "hello", "/tmp", Some("low"));
        assert!(result.is_ok());
    }

    #[test]
    fn build_claude_command_with_max_effort() {
        let config = CliSpawnConfig {
            cli_type: "claude".to_string(),
            cli_bin: Some("claude".to_string()),
            cli_args: None,
            cli_home: None,
        };
        let result = build_claude_command(&config, None, "hello", "/tmp", Some("max"));
        assert!(result.is_ok());
    }

    #[test]
    fn parse_stream_json_init() {
        let line = r#"{"type":"system","subtype":"init","session_id":"s1","tools":[],"model":"claude-4"}"#;
        let event = parse_stream_json_line(line, "t1", "turn1");
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("turn/started")
        );
    }

    #[test]
    fn parse_stream_json_text_delta_has_item_id() {
        let line = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hello"}}"#;
        let event = parse_stream_json_line(line, "t1", "turn1").unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("item/agentMessage/delta")
        );
        let params = event.get("params").unwrap();
        assert_eq!(params.get("delta").and_then(|d| d.as_str()), Some("hello"));
        assert!(
            params.get("itemId").and_then(|i| i.as_str()).is_some(),
            "item/agentMessage/delta must include itemId for frontend dispatch"
        );
    }

    #[test]
    fn parse_stream_json_tool_use_start_emits_item_started() {
        let line = r#"{"type":"content_block_start","content_block":{"type":"tool_use","name":"Read","id":"tool-1"}}"#;
        let event = parse_stream_json_line(line, "t1", "turn1").unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("item/started"),
        );
        let item = event.get("params").and_then(|p| p.get("item")).unwrap();
        assert_eq!(item.get("id").and_then(|i| i.as_str()), Some("tool-1"));
        assert_eq!(item.get("name").and_then(|n| n.as_str()), Some("Read"));
    }

    #[test]
    fn parse_stream_json_tool_input_delta_is_dropped() {
        let line = r#"{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"path\":"}}"#;
        assert!(parse_stream_json_line(line, "t1", "turn1").is_none());
    }

    #[test]
    fn parse_stream_json_tool_result_emits_item_completed() {
        let line = r#"{"type":"tool_result","tool_use_id":"tool-1","content":"done"}"#;
        let event = parse_stream_json_line(line, "t1", "turn1").unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("item/completed"),
        );
        let item = event.get("params").and_then(|p| p.get("item")).unwrap();
        assert_eq!(item.get("id").and_then(|i| i.as_str()), Some("tool-1"));
    }

    const SUPPORTED_METHODS: &[&str] = &[
        "item/agentMessage/delta",
        "item/completed",
        "item/started",
        "turn/completed",
        "turn/started",
    ];

    #[test]
    fn all_emitted_methods_are_supported_by_frontend() {
        let test_lines = vec![
            r#"{"type":"system","subtype":"init","session_id":"s1","tools":[]}"#,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hi"}}"#,
            r#"{"type":"content_block_start","content_block":{"type":"tool_use","name":"Read","id":"t1"}}"#,
            r#"{"type":"tool_result","tool_use_id":"t1","content":"ok"}"#,
            r#"{"type":"result","subtype":"success","cost_usd":0.01,"duration_ms":100}}"#,
        ];
        for line in test_lines {
            if let Some(event) = parse_stream_json_line(line, "thread1", "turn1") {
                let method = event.get("method").and_then(|m| m.as_str()).unwrap();
                assert!(
                    SUPPORTED_METHODS.contains(&method),
                    "Emitted method '{method}' is not in SUPPORTED_APP_SERVER_METHODS"
                );
            }
        }
    }

    #[test]
    fn parse_stream_json_result() {
        let line = r#"{"type":"result","subtype":"success","cost_usd":0.05,"duration_ms":1200,"session_id":"s1"}"#;
        let event = parse_stream_json_line(line, "t1", "turn1");
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("turn/completed")
        );
    }

    #[test]
    fn parse_stream_json_unknown_type() {
        let line = r#"{"type":"unknown_event"}"#;
        assert!(parse_stream_json_line(line, "t1", "turn1").is_none());
    }

    #[test]
    fn extract_session_id_from_init_line() {
        let line = r#"{"type":"system","subtype":"init","session_id":"abc-123","tools":[]}"#;
        assert_eq!(
            extract_session_id_from_line(line),
            Some("abc-123".to_string())
        );
    }

    #[test]
    fn extract_session_id_from_non_init_line() {
        let line = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"hi"}}"#;
        assert_eq!(extract_session_id_from_line(line), None);
    }

    #[test]
    fn thread_store_roundtrip() {
        use crate::backend::adapter_base::ThreadMetadata;

        let temp_dir = std::env::temp_dir().join(format!(
            "claude-adapter-test-{}",
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

    #[tokio::test]
    async fn adapter_send_request_routing() {
        let adapter = test_adapter();

        let init_result = adapter.send_request("initialize", json!({})).await;
        assert!(init_result.is_ok());

        let thread_result = adapter.send_request("thread/start", json!({})).await;
        assert!(thread_result.is_ok());
        let thread_id = thread_result
            .unwrap()
            .get("result")
            .and_then(|r| r.get("threadId"))
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let list_result = adapter.send_request("thread/list", json!({})).await;
        assert!(list_result.is_ok());

        let archive_result = adapter
            .send_request("thread/archive", json!({ "threadId": thread_id }))
            .await;
        assert!(archive_result.is_ok());

        let model_result = adapter.send_request("model/list", json!({})).await;
        assert!(model_result.is_ok());
        let models = model_result
            .unwrap()
            .get("result")
            .and_then(|r| r.get("models"))
            .and_then(|m| m.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        assert!(models > 0);

        let account_result = adapter.send_request("account/read", json!({})).await;
        assert!(account_result.is_ok());

        let unknown_result = adapter.send_request("nonexistent/method", json!({})).await;
        assert!(unknown_result.is_err());
    }

    #[tokio::test]
    async fn thread_start_response_has_thread_id_and_thread_object() {
        let adapter = test_adapter();
        let result = adapter.send_request("thread/start", json!({})).await.unwrap();
        let r = result.get("result").expect("must have result");
        assert!(
            r.get("threadId").and_then(|v| v.as_str()).is_some(),
            "thread/start result must include threadId"
        );
        let thread = r.get("thread").expect("must have thread object");
        assert!(
            thread.get("id").and_then(|v| v.as_str()).is_some(),
            "thread/start result.thread must include id"
        );
    }

    #[tokio::test]
    async fn model_list_includes_reasoning_efforts() {
        let adapter = test_adapter();
        let result = adapter.send_request("model/list", json!({})).await.unwrap();
        let models = result["result"]["models"].as_array().unwrap();

        for model in models {
            assert!(model.get("supportedReasoningEfforts").is_some());
            assert!(model.get("defaultReasoningEffort").is_some());
        }

        let opus = models.iter().find(|m| m["id"] == "claude-opus-4-20250514").unwrap();
        let opus_efforts = opus["supportedReasoningEfforts"].as_array().unwrap();
        assert_eq!(opus_efforts.len(), 4);
        assert!(opus_efforts.iter().any(|e| e["reasoningEffort"] == "max"));

        let sonnet = models.iter().find(|m| m["id"] == "claude-sonnet-4-20250514").unwrap();
        let sonnet_efforts = sonnet["supportedReasoningEfforts"].as_array().unwrap();
        assert_eq!(sonnet_efforts.len(), 3);
        assert!(!sonnet_efforts.iter().any(|e| e["reasoningEffort"] == "max"));
    }
}
