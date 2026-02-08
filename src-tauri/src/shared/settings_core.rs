use std::path::PathBuf;

use tokio::sync::Mutex;

use crate::codex::config as codex_config;
use crate::storage::write_settings;
use crate::types::AppSettings;

fn normalize_personality(value: &str) -> Option<&'static str> {
    match value.trim() {
        "friendly" => Some("friendly"),
        "pragmatic" => Some("pragmatic"),
        _ => None,
    }
}

fn normalize_model_reasoning_effort(value: &str) -> Option<&'static str> {
    match value.trim() {
        "minimal" => Some("minimal"),
        "low" => Some("low"),
        "medium" => Some("medium"),
        "high" => Some("high"),
        _ => None,
    }
}

fn normalize_approval_policy(value: &str) -> Option<&'static str> {
    match value.trim() {
        "untrusted" => Some("untrusted"),
        "on-failure" => Some("on-failure"),
        "on-request" => Some("on-request"),
        "never" => Some("never"),
        _ => None,
    }
}

fn normalize_sandbox_mode(value: &str) -> Option<&'static str> {
    match value.trim() {
        "read-only" => Some("read-only"),
        "workspace-write" => Some("workspace-write"),
        "danger-full-access" => Some("danger-full-access"),
        _ => None,
    }
}

fn normalize_web_search(value: &str) -> Option<&'static str> {
    match value.trim() {
        "cached" => Some("cached"),
        "live" => Some("live"),
        _ => None,
    }
}

fn normalize_cli_auth_credentials_store(value: &str) -> Option<&'static str> {
    match value.trim() {
        "file" => Some("file"),
        "keyring" => Some("keyring"),
        "auto" => Some("auto"),
        _ => None,
    }
}

pub(crate) async fn get_app_settings_core(app_settings: &Mutex<AppSettings>) -> AppSettings {
    let mut settings = app_settings.lock().await.clone();
    if let Ok(Some(collab_enabled)) = codex_config::read_collab_enabled() {
        settings.experimental_collab_enabled = collab_enabled;
    }
    if let Ok(Some(collaboration_modes_enabled)) = codex_config::read_collaboration_modes_enabled()
    {
        settings.collaboration_modes_enabled = collaboration_modes_enabled;
    }
    if let Ok(Some(steer_enabled)) = codex_config::read_steer_enabled() {
        settings.steer_enabled = steer_enabled;
    }
    if let Ok(Some(unified_exec_enabled)) = codex_config::read_unified_exec_enabled() {
        settings.unified_exec_enabled = unified_exec_enabled;
    }
    if let Ok(Some(apps_enabled)) = codex_config::read_apps_enabled() {
        settings.experimental_apps_enabled = apps_enabled;
    }
    if let Ok(Some(shell_tool_enabled)) = codex_config::read_shell_tool_enabled() {
        settings.codex_shell_tool_enabled = shell_tool_enabled;
    }
    if let Ok(Some(shell_snapshot_enabled)) = codex_config::read_shell_snapshot_enabled() {
        settings.codex_shell_snapshot_enabled = shell_snapshot_enabled;
    }
    if let Ok(Some(apply_patch_freeform_enabled)) =
        codex_config::read_apply_patch_freeform_enabled()
    {
        settings.codex_apply_patch_freeform_enabled = apply_patch_freeform_enabled;
    }
    if let Ok(Some(exec_policy_enabled)) = codex_config::read_exec_policy_enabled() {
        settings.codex_exec_policy_enabled = exec_policy_enabled;
    }
    if let Ok(Some(smart_approvals_enabled)) = codex_config::read_smart_approvals_enabled() {
        settings.codex_smart_approvals_enabled = smart_approvals_enabled;
    }
    if let Ok(Some(remote_compaction_enabled)) = codex_config::read_remote_compaction_enabled() {
        settings.codex_remote_compaction_enabled = remote_compaction_enabled;
    }
    if let Ok(Some(experimental_windows_sandbox_enabled)) =
        codex_config::read_experimental_windows_sandbox_enabled()
    {
        settings.codex_experimental_windows_sandbox_enabled = experimental_windows_sandbox_enabled;
    }
    if let Ok(Some(elevated_windows_sandbox_enabled)) =
        codex_config::read_elevated_windows_sandbox_enabled()
    {
        settings.codex_elevated_windows_sandbox_enabled = elevated_windows_sandbox_enabled;
    }
    if let Ok(model) = codex_config::read_model() {
        settings.codex_model = model;
    }
    if let Ok(model_provider) = codex_config::read_model_provider() {
        settings.codex_model_provider = model_provider;
    }
    if let Ok(Some(model_reasoning_effort)) = codex_config::read_model_reasoning_effort() {
        if let Some(value) = normalize_model_reasoning_effort(&model_reasoning_effort) {
            settings.codex_model_reasoning_effort = value.to_string();
        }
    }
    if let Ok(Some(approval_policy)) = codex_config::read_approval_policy() {
        if let Some(value) = normalize_approval_policy(&approval_policy) {
            settings.codex_approval_policy = value.to_string();
        }
    }
    if let Ok(Some(sandbox_mode)) = codex_config::read_sandbox_mode() {
        if let Some(value) = normalize_sandbox_mode(&sandbox_mode) {
            settings.codex_sandbox_mode = value.to_string();
        }
    }
    if let Ok(Some(check_for_updates)) = codex_config::read_check_for_updates() {
        settings.codex_check_for_updates = check_for_updates;
    }
    if let Ok(Some(web_search)) = codex_config::read_web_search() {
        if let Some(value) = normalize_web_search(&web_search) {
            settings.codex_web_search = value.to_string();
        }
    }
    if let Ok(Some(credentials_store)) = codex_config::read_cli_auth_credentials_store() {
        if let Some(value) = normalize_cli_auth_credentials_store(&credentials_store) {
            settings.codex_cli_auth_credentials_store = value.to_string();
        }
    }
    if let Ok(preferred_auth_method) = codex_config::read_preferred_auth_method() {
        settings.codex_preferred_auth_method = preferred_auth_method;
    }
    if let Ok(personality) = codex_config::read_personality() {
        settings.personality = personality
            .as_deref()
            .and_then(normalize_personality)
            .unwrap_or("friendly")
            .to_string();
    }
    settings
}

pub(crate) async fn update_app_settings_core(
    settings: AppSettings,
    app_settings: &Mutex<AppSettings>,
    settings_path: &PathBuf,
) -> Result<AppSettings, String> {
    let _ = codex_config::write_collab_enabled(settings.experimental_collab_enabled);
    let _ = codex_config::write_collaboration_modes_enabled(settings.collaboration_modes_enabled);
    let _ = codex_config::write_steer_enabled(settings.steer_enabled);
    let _ = codex_config::write_unified_exec_enabled(settings.unified_exec_enabled);
    let _ = codex_config::write_apps_enabled(settings.experimental_apps_enabled);
    let _ = codex_config::write_shell_tool_enabled(settings.codex_shell_tool_enabled);
    let _ = codex_config::write_shell_snapshot_enabled(settings.codex_shell_snapshot_enabled);
    let _ = codex_config::write_apply_patch_freeform_enabled(
        settings.codex_apply_patch_freeform_enabled,
    );
    let _ = codex_config::write_exec_policy_enabled(settings.codex_exec_policy_enabled);
    let _ = codex_config::write_smart_approvals_enabled(settings.codex_smart_approvals_enabled);
    let _ = codex_config::write_remote_compaction_enabled(settings.codex_remote_compaction_enabled);
    let _ = codex_config::write_experimental_windows_sandbox_enabled(
        settings.codex_experimental_windows_sandbox_enabled,
    );
    let _ = codex_config::write_elevated_windows_sandbox_enabled(
        settings.codex_elevated_windows_sandbox_enabled,
    );
    let _ = codex_config::write_model(settings.codex_model.as_deref());
    let _ = codex_config::write_model_provider(settings.codex_model_provider.as_deref());
    let _ =
        codex_config::write_model_reasoning_effort(settings.codex_model_reasoning_effort.as_str());
    let _ = codex_config::write_approval_policy(settings.codex_approval_policy.as_str());
    let _ = codex_config::write_sandbox_mode(settings.codex_sandbox_mode.as_str());
    let _ = codex_config::write_check_for_updates(settings.codex_check_for_updates);
    let _ = codex_config::write_web_search(settings.codex_web_search.as_str());
    let _ = codex_config::write_cli_auth_credentials_store(
        settings.codex_cli_auth_credentials_store.as_str(),
    );
    let _ =
        codex_config::write_preferred_auth_method(settings.codex_preferred_auth_method.as_deref());
    let _ = codex_config::write_personality(settings.personality.as_str());
    write_settings(settings_path, &settings)?;
    let mut current = app_settings.lock().await;
    *current = settings.clone();
    Ok(settings)
}

pub(crate) fn get_codex_config_path_core() -> Result<String, String> {
    codex_config::config_toml_path()
        .ok_or_else(|| "Unable to resolve CODEX_HOME".to_string())
        .and_then(|path| {
            path.to_str()
                .map(|value| value.to_string())
                .ok_or_else(|| "Unable to resolve CODEX_HOME".to_string())
        })
}
