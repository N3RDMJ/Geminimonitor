use serde_json::{json, Value};
use std::sync::Arc;

use crate::backend::adapter_base::{build_adapter_command, spawn_adapter_session, CliProfile};
use crate::backend::app_server::{CliSpawnConfig, WorkspaceSession};
use crate::backend::events::EventSink;
use crate::types::WorkspaceEntry;

pub(crate) struct CursorProfile;

impl CliProfile for CursorProfile {
    fn build_turn_command(
        &self,
        config: &CliSpawnConfig,
        session_id: Option<&str>,
        prompt: &str,
        cwd: &str,
        _params: &Value,
    ) -> Result<tokio::process::Command, String> {
        build_cursor_command(config, session_id, prompt, cwd)
    }

    fn parse_stream_line(&self, line: &str, thread_id: &str, turn_id: &str) -> Option<Value> {
        parse_cursor_stream_line(line, thread_id, turn_id)
    }

    fn extract_session_id(&self, line: &str) -> Option<String> {
        extract_cursor_session_id(line)
    }

    fn model_list(&self) -> Value {
        json!({
            "result": {
                "models": [],
                "defaultModel": null
            }
        })
    }

    fn provider_name(&self) -> &str {
        "cursor"
    }
}

pub(crate) fn build_cursor_command(
    config: &CliSpawnConfig,
    session_id: Option<&str>,
    prompt: &str,
    cwd: &str,
) -> Result<tokio::process::Command, String> {
    let mut args = vec![
        "-p".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
    ];
    if let Some(sid) = session_id {
        args.push("--resume".to_string());
        args.push(sid.to_string());
    }
    args.push(prompt.to_string());

    build_adapter_command(config, args, cwd, None)
}

pub(crate) fn parse_cursor_stream_line(
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
        "assistant" => {
            let text = event
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|item| item.get("text"))
                .and_then(|t| t.as_str())?;
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
        "tool_call" => {
            let subtype = event.get("subtype").and_then(|s| s.as_str()).unwrap_or("");
            let call_id = event
                .get("call_id")
                .and_then(|c| c.as_str())
                .unwrap_or("");

            match subtype {
                "started" => {
                    let tool_name = extract_tool_name_from_cursor_event(&event);
                    Some(json!({
                        "method": "item/started",
                        "params": {
                            "threadId": thread_id,
                            "turnId": turn_id,
                            "item": {
                                "id": call_id,
                                "type": "tool_use",
                                "name": tool_name
                            }
                        }
                    }))
                }
                "completed" => Some(json!({
                    "method": "item/completed",
                    "params": {
                        "threadId": thread_id,
                        "turnId": turn_id,
                        "item": {
                            "id": call_id,
                            "type": "tool_use"
                        }
                    }
                })),
                _ => None,
            }
        }
        "result" => Some(json!({
            "method": "turn/completed",
            "params": {
                "threadId": thread_id,
                "turnId": turn_id,
                "durationMs": event.get("duration_ms")
            }
        })),
        _ => None,
    }
}

fn extract_tool_name_from_cursor_event(event: &Value) -> &str {
    if let Some(obj) = event.as_object() {
        for key in obj.keys() {
            if key.ends_with("ToolCall") {
                return key.strip_suffix("ToolCall").unwrap_or(key);
            }
        }
    }
    event
        .get("tool_name")
        .and_then(|n| n.as_str())
        .unwrap_or("tool")
}

fn extract_cursor_session_id(line: &str) -> Option<String> {
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

pub(crate) async fn spawn_cursor_session<E: EventSink>(
    entry: WorkspaceEntry,
    config: CliSpawnConfig,
    event_sink: E,
) -> Result<Arc<WorkspaceSession>, String> {
    spawn_adapter_session(CursorProfile, "Cursor", entry, config, event_sink).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_cursor_command_basic() {
        let config = CliSpawnConfig {
            cli_type: "cursor".to_string(),
            cli_bin: Some("cursor".to_string()),
            cli_args: None,
            cli_home: None,
        };
        let result = build_cursor_command(&config, None, "hello", "/tmp");
        assert!(result.is_ok());
    }

    #[test]
    fn build_cursor_command_with_resume() {
        let config = CliSpawnConfig {
            cli_type: "cursor".to_string(),
            cli_bin: Some("cursor".to_string()),
            cli_args: None,
            cli_home: None,
        };
        let result = build_cursor_command(&config, Some("sess-1"), "hello", "/tmp");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_system_init() {
        let line = r#"{"type":"system","subtype":"init","session_id":"cs-1"}"#;
        let event = parse_cursor_stream_line(line, "t1", "turn1").unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("turn/started")
        );
    }

    #[test]
    fn parse_system_non_init_is_dropped() {
        let line = r#"{"type":"system","subtype":"config","data":{}}"#;
        assert!(parse_cursor_stream_line(line, "t1", "turn1").is_none());
    }

    #[test]
    fn parse_assistant_message() {
        let line = r#"{"type":"assistant","message":{"content":[{"text":"Hello world"}]}}"#;
        let event = parse_cursor_stream_line(line, "t1", "turn1").unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("item/agentMessage/delta")
        );
        let params = event.get("params").unwrap();
        assert_eq!(
            params.get("delta").and_then(|d| d.as_str()),
            Some("Hello world")
        );
        assert!(params.get("itemId").is_some());
    }

    #[test]
    fn parse_assistant_message_empty_content_is_dropped() {
        let line = r#"{"type":"assistant","message":{"content":[]}}"#;
        assert!(parse_cursor_stream_line(line, "t1", "turn1").is_none());
    }

    #[test]
    fn parse_tool_call_started() {
        let line = r#"{"type":"tool_call","subtype":"started","call_id":"c1","ReadToolCall":{"path":"test.rs"}}"#;
        let event = parse_cursor_stream_line(line, "t1", "turn1").unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("item/started")
        );
        let item = event.get("params").and_then(|p| p.get("item")).unwrap();
        assert_eq!(item.get("id").and_then(|i| i.as_str()), Some("c1"));
        assert_eq!(item.get("name").and_then(|n| n.as_str()), Some("Read"));
    }

    #[test]
    fn parse_tool_call_completed() {
        let line = r#"{"type":"tool_call","subtype":"completed","call_id":"c1"}"#;
        let event = parse_cursor_stream_line(line, "t1", "turn1").unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("item/completed")
        );
    }

    #[test]
    fn parse_result_event() {
        let line = r#"{"type":"result","duration_ms":1500}"#;
        let event = parse_cursor_stream_line(line, "t1", "turn1").unwrap();
        assert_eq!(
            event.get("method").and_then(|v| v.as_str()),
            Some("turn/completed")
        );
        assert_eq!(
            event
                .get("params")
                .and_then(|p| p.get("durationMs"))
                .and_then(|d| d.as_u64()),
            Some(1500)
        );
    }

    #[test]
    fn parse_unknown_event() {
        let line = r#"{"type":"internal_debug","data":{}}"#;
        assert!(parse_cursor_stream_line(line, "t1", "turn1").is_none());
    }

    #[test]
    fn extract_session_id_from_init() {
        let line = r#"{"type":"system","subtype":"init","session_id":"cs-42"}"#;
        assert_eq!(
            extract_cursor_session_id(line),
            Some("cs-42".to_string())
        );
    }

    #[test]
    fn extract_session_id_from_non_init() {
        let line = r#"{"type":"assistant","message":{"content":[{"text":"hi"}]}}"#;
        assert_eq!(extract_cursor_session_id(line), None);
    }

    #[test]
    fn extract_tool_name_from_tool_call_key() {
        let event: Value =
            serde_json::from_str(r#"{"type":"tool_call","subtype":"started","call_id":"c1","EditToolCall":{"path":"x"}}"#)
                .unwrap();
        assert_eq!(extract_tool_name_from_cursor_event(&event), "Edit");
    }

    #[test]
    fn extract_tool_name_fallback() {
        let event: Value =
            serde_json::from_str(r#"{"type":"tool_call","subtype":"started","call_id":"c1","tool_name":"Bash"}"#)
                .unwrap();
        assert_eq!(extract_tool_name_from_cursor_event(&event), "Bash");
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
            r#"{"type":"system","subtype":"init","session_id":"s1"}"#,
            r#"{"type":"assistant","message":{"content":[{"text":"hi"}]}}"#,
            r#"{"type":"tool_call","subtype":"started","call_id":"c1","ReadToolCall":{}}"#,
            r#"{"type":"tool_call","subtype":"completed","call_id":"c1"}"#,
            r#"{"type":"result","duration_ms":100}"#,
        ];
        for line in test_lines {
            if let Some(event) = parse_cursor_stream_line(line, "thread1", "turn1") {
                let method = event.get("method").and_then(|m| m.as_str()).unwrap();
                assert!(
                    SUPPORTED_METHODS.contains(&method),
                    "Emitted method '{method}' is not in SUPPORTED_APP_SERVER_METHODS"
                );
            }
        }
    }
}
