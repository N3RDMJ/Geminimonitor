use serde_json::{json, Value};
use std::sync::Arc;

use crate::backend::adapter_base::{build_adapter_command, spawn_adapter_session, CliProfile};
use crate::backend::app_server::{CliSpawnConfig, WorkspaceSession};
use crate::backend::events::EventSink;
use crate::types::WorkspaceEntry;

pub(crate) struct GeminiProfile;

impl CliProfile for GeminiProfile {
    fn build_turn_command(
        &self,
        config: &CliSpawnConfig,
        session_id: Option<&str>,
        prompt: &str,
        cwd: &str,
        _params: &Value,
    ) -> Result<tokio::process::Command, String> {
        build_gemini_command(config, session_id, prompt, cwd)
    }

    fn parse_stream_line(&self, line: &str, thread_id: &str, turn_id: &str) -> Option<Value> {
        parse_gemini_stream_line(line, thread_id, turn_id)
    }

    fn extract_session_id(&self, line: &str) -> Option<String> {
        extract_gemini_session_id(line)
    }

    fn model_list(&self) -> Value {
        json!({
            "result": {
                "models": [
                    { "id": "gemini-2.5-flash", "name": "Gemini 2.5 Flash" },
                    { "id": "gemini-2.5-pro", "name": "Gemini 2.5 Pro" }
                ],
                "defaultModel": "gemini-2.5-flash"
            }
        })
    }

    fn provider_name(&self) -> &str {
        "gemini"
    }
}

pub(crate) fn build_gemini_command(
    config: &CliSpawnConfig,
    session_id: Option<&str>,
    prompt: &str,
    cwd: &str,
) -> Result<tokio::process::Command, String> {
    let mut args = vec![
        "--output-format".to_string(),
        "stream-json".to_string(),
        "-p".to_string(),
    ];
    if let Some(sid) = session_id {
        args.push("--resume".to_string());
        args.push(sid.to_string());
    }
    args.push(prompt.to_string());

    let home_env = config.cli_home.as_ref().map(|h| ("GEMINI_HOME", h));
    build_adapter_command(config, args, cwd, home_env)
}

pub(crate) fn parse_gemini_stream_line(
    line: &str,
    thread_id: &str,
    turn_id: &str,
) -> Option<Value> {
    let event: Value = serde_json::from_str(line).ok()?;
    let event_type = event.get("type")?.as_str()?;

    let msg_item_id = format!("msg_{turn_id}");

    match event_type {
        "init" => Some(json!({
            "method": "turn/started",
            "params": {
                "threadId": thread_id,
                "turnId": turn_id
            }
        })),
        "message" => {
            let role = event.get("role").and_then(|r| r.as_str()).unwrap_or("");
            if role != "assistant" {
                return None;
            }
            let content = event.get("content").and_then(|c| c.as_str())?;
            Some(json!({
                "method": "item/agentMessage/delta",
                "params": {
                    "threadId": thread_id,
                    "turnId": turn_id,
                    "itemId": msg_item_id,
                    "delta": content
                }
            }))
        }
        "tool_use" => {
            let tool_name = event
                .get("tool_name")
                .and_then(|n| n.as_str())
                .unwrap_or("tool");
            let tool_id = event
                .get("tool_id")
                .and_then(|i| i.as_str())
                .unwrap_or("");
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
        }
        "tool_result" => {
            let tool_id = event
                .get("tool_id")
                .and_then(|i| i.as_str())
                .unwrap_or("");
            Some(json!({
                "method": "item/completed",
                "params": {
                    "threadId": thread_id,
                    "turnId": turn_id,
                    "item": {
                        "id": tool_id,
                        "type": "tool_use"
                    }
                }
            }))
        }
        "result" => Some(json!({
            "method": "turn/completed",
            "params": {
                "threadId": thread_id,
                "turnId": turn_id,
                "durationMs": event.get("stats").and_then(|s| s.get("duration_ms"))
            }
        })),
        _ => None,
    }
}

fn extract_gemini_session_id(line: &str) -> Option<String> {
    let event: Value = serde_json::from_str(line).ok()?;
    if event.get("type")?.as_str()? != "init" {
        return None;
    }
    event
        .get("session_id")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

pub(crate) async fn spawn_gemini_session<E: EventSink>(
    entry: WorkspaceEntry,
    config: CliSpawnConfig,
    event_sink: E,
) -> Result<Arc<WorkspaceSession>, String> {
    spawn_adapter_session(GeminiProfile, "Gemini", entry, config, event_sink).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_gemini_command_basic() {
        let config = CliSpawnConfig {
            cli_type: "gemini".to_string(),
            cli_bin: Some("gemini".to_string()),
            cli_args: None,
            cli_home: None,
        };
        let result = build_gemini_command(&config, None, "hello", "/tmp");
        assert!(result.is_ok());
    }

    #[test]
    fn build_gemini_command_with_resume() {
        let config = CliSpawnConfig {
            cli_type: "gemini".to_string(),
            cli_bin: Some("gemini".to_string()),
            cli_args: None,
            cli_home: None,
        };
        let result = build_gemini_command(&config, Some("sess-1"), "hello", "/tmp");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_init_event() {
        let line = r#"{"type":"init","session_id":"gs-1","model":"gemini-2.5-flash"}"#;
        let event = parse_gemini_stream_line(line, "t1", "turn1").unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("turn/started")
        );
    }

    #[test]
    fn parse_assistant_message() {
        let line = r#"{"type":"message","role":"assistant","content":"Hello!","delta":true}"#;
        let event = parse_gemini_stream_line(line, "t1", "turn1").unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("item/agentMessage/delta")
        );
        let params = event.get("params").unwrap();
        assert_eq!(
            params.get("delta").and_then(|d| d.as_str()),
            Some("Hello!")
        );
        assert!(params.get("itemId").is_some());
    }

    #[test]
    fn parse_user_message_is_dropped() {
        let line = r#"{"type":"message","role":"user","content":"hi","delta":true}"#;
        assert!(parse_gemini_stream_line(line, "t1", "turn1").is_none());
    }

    #[test]
    fn parse_tool_use_event() {
        let line = r#"{"type":"tool_use","tool_name":"ReadFile","tool_id":"tu-1"}"#;
        let event = parse_gemini_stream_line(line, "t1", "turn1").unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("item/started")
        );
        let item = event.get("params").and_then(|p| p.get("item")).unwrap();
        assert_eq!(item.get("name").and_then(|n| n.as_str()), Some("ReadFile"));
        assert_eq!(item.get("id").and_then(|i| i.as_str()), Some("tu-1"));
    }

    #[test]
    fn parse_tool_result_event() {
        let line = r#"{"type":"tool_result","tool_id":"tu-1","status":"success"}"#;
        let event = parse_gemini_stream_line(line, "t1", "turn1").unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("item/completed")
        );
    }

    #[test]
    fn parse_result_event() {
        let line = r#"{"type":"result","status":"success","stats":{"duration_ms":500}}"#;
        let event = parse_gemini_stream_line(line, "t1", "turn1").unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("turn/completed")
        );
    }

    #[test]
    fn parse_unknown_event() {
        let line = r#"{"type":"debug","msg":"internal"}"#;
        assert!(parse_gemini_stream_line(line, "t1", "turn1").is_none());
    }

    #[test]
    fn extract_session_id_from_init() {
        let line = r#"{"type":"init","session_id":"gs-42","model":"gemini-2.5-pro"}"#;
        assert_eq!(
            extract_gemini_session_id(line),
            Some("gs-42".to_string())
        );
    }

    #[test]
    fn extract_session_id_from_non_init() {
        let line = r#"{"type":"message","role":"assistant","content":"hi"}"#;
        assert_eq!(extract_gemini_session_id(line), None);
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
            r#"{"type":"init","session_id":"s1","model":"gemini-2.5-flash"}"#,
            r#"{"type":"message","role":"assistant","content":"hi","delta":true}"#,
            r#"{"type":"tool_use","tool_name":"Read","tool_id":"t1"}"#,
            r#"{"type":"tool_result","tool_id":"t1","status":"success"}"#,
            r#"{"type":"result","status":"success","stats":{"duration_ms":100}}"#,
        ];
        for line in test_lines {
            if let Some(event) = parse_gemini_stream_line(line, "thread1", "turn1") {
                let method = event.get("method").and_then(|m| m.as_str()).unwrap();
                assert!(
                    SUPPORTED_METHODS.contains(&method),
                    "Emitted method '{method}' is not in SUPPORTED_APP_SERVER_METHODS"
                );
            }
        }
    }
}
