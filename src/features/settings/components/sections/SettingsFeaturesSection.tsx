import type { AppSettings } from "../../../../types";
import { fileManagerName, openInFileManagerLabel } from "../../../../utils/platformPaths";

type SettingsFeaturesSectionProps = {
  appSettings: AppSettings;
  hasCodexHomeOverrides: boolean;
  openConfigError: string | null;
  onOpenConfig: () => void;
  onUpdateAppSettings: (next: AppSettings) => Promise<void>;
};

export function SettingsFeaturesSection({
  appSettings,
  hasCodexHomeOverrides,
  openConfigError,
  onOpenConfig,
  onUpdateAppSettings,
}: SettingsFeaturesSectionProps) {
  return (
    <section className="settings-section">
      <div className="settings-section-title">Features</div>
      <div className="settings-section-subtitle">
        Manage stable and experimental Codex features.
      </div>
      {hasCodexHomeOverrides && (
        <div className="settings-help">
          Feature settings are stored in the default CODEX_HOME config.toml.
          <br />
          Workspace overrides are not updated.
        </div>
      )}
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Config file</div>
          <div className="settings-toggle-subtitle">
            Open the Codex config in {fileManagerName()}.
          </div>
        </div>
        <button type="button" className="ghost" onClick={onOpenConfig}>
          {openInFileManagerLabel()}
        </button>
      </div>
      {openConfigError && <div className="settings-help">{openConfigError}</div>}
      <div className="settings-subsection-title">Stable Features</div>
      <div className="settings-subsection-subtitle">
        Production-ready features enabled by default.
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Model</div>
          <div className="settings-toggle-subtitle">
            Set top-level <code>model</code> in config.toml.
          </div>
        </div>
        <input
          id="features-model-input"
          className="settings-input settings-input-inline"
          value={appSettings.codexModel ?? ""}
          placeholder="gpt-5"
          onChange={(event) =>
            void onUpdateAppSettings({
              ...appSettings,
              codexModel: event.target.value.trim() ? event.target.value : null,
            })
          }
        />
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Model Provider</div>
          <div className="settings-toggle-subtitle">
            Choose top-level <code>model_provider</code>.
          </div>
        </div>
        <input
          id="features-model-provider-input"
          className="settings-input settings-input-inline"
          value={appSettings.codexModelProvider ?? ""}
          placeholder="openai"
          onChange={(event) =>
            void onUpdateAppSettings({
              ...appSettings,
              codexModelProvider: event.target.value.trim() ? event.target.value : null,
            })
          }
        />
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Reasoning Effort</div>
          <div className="settings-toggle-subtitle">
            Set <code>model_reasoning_effort</code>.
          </div>
        </div>
        <select
          id="features-reasoning-effort-select"
          className="settings-select"
          value={appSettings.codexModelReasoningEffort}
          onChange={(event) =>
            void onUpdateAppSettings({
              ...appSettings,
              codexModelReasoningEffort: event.target
                .value as AppSettings["codexModelReasoningEffort"],
            })
          }
          aria-label="Reasoning Effort"
        >
          <option value="minimal">Minimal</option>
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
        </select>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Approval Policy</div>
          <div className="settings-toggle-subtitle">
            Set top-level <code>approval_policy</code>.
          </div>
        </div>
        <select
          id="features-approval-policy-select"
          className="settings-select"
          value={appSettings.codexApprovalPolicy}
          onChange={(event) =>
            void onUpdateAppSettings({
              ...appSettings,
              codexApprovalPolicy: event.target.value as AppSettings["codexApprovalPolicy"],
            })
          }
          aria-label="Approval Policy"
        >
          <option value="untrusted">Untrusted</option>
          <option value="on-failure">On failure</option>
          <option value="on-request">On request</option>
          <option value="never">Never</option>
        </select>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Sandbox Mode</div>
          <div className="settings-toggle-subtitle">
            Set top-level <code>sandbox_mode</code>.
          </div>
        </div>
        <select
          id="features-sandbox-mode-select"
          className="settings-select"
          value={appSettings.codexSandboxMode}
          onChange={(event) =>
            void onUpdateAppSettings({
              ...appSettings,
              codexSandboxMode: event.target.value as AppSettings["codexSandboxMode"],
            })
          }
          aria-label="Sandbox Mode"
        >
          <option value="read-only">Read only</option>
          <option value="workspace-write">Workspace write</option>
          <option value="danger-full-access">Danger full access</option>
        </select>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Web Search</div>
          <div className="settings-toggle-subtitle">
            Set top-level <code>web_search</code>.
          </div>
        </div>
        <select
          id="features-web-search-select"
          className="settings-select"
          value={appSettings.codexWebSearch}
          onChange={(event) =>
            void onUpdateAppSettings({
              ...appSettings,
              codexWebSearch: event.target.value as AppSettings["codexWebSearch"],
            })
          }
          aria-label="Web Search"
        >
          <option value="live">Live</option>
          <option value="cached">Cached</option>
        </select>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Auth Credentials Store</div>
          <div className="settings-toggle-subtitle">
            Set <code>cli_auth_credentials_store</code>.
          </div>
        </div>
        <select
          id="features-auth-credentials-store-select"
          className="settings-select"
          value={appSettings.codexCliAuthCredentialsStore}
          onChange={(event) =>
            void onUpdateAppSettings({
              ...appSettings,
              codexCliAuthCredentialsStore: event.target
                .value as AppSettings["codexCliAuthCredentialsStore"],
            })
          }
          aria-label="Auth Credentials Store"
        >
          <option value="auto">Auto</option>
          <option value="keyring">Keyring</option>
          <option value="file">File</option>
        </select>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Preferred Auth Method</div>
          <div className="settings-toggle-subtitle">
            Set top-level <code>preferred_auth_method</code>.
          </div>
        </div>
        <input
          id="features-auth-method-input"
          className="settings-input settings-input-inline"
          value={appSettings.codexPreferredAuthMethod ?? ""}
          placeholder="chatgpt"
          onChange={(event) =>
            void onUpdateAppSettings({
              ...appSettings,
              codexPreferredAuthMethod: event.target.value.trim() ? event.target.value : null,
            })
          }
        />
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Check for Updates</div>
          <div className="settings-toggle-subtitle">
            Set top-level <code>check_for_updates</code>.
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.codexCheckForUpdates ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              codexCheckForUpdates: !appSettings.codexCheckForUpdates,
            })
          }
          aria-pressed={appSettings.codexCheckForUpdates}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Collaboration modes</div>
          <div className="settings-toggle-subtitle">
            Enable collaboration mode presets (Code, Plan).
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.collaborationModesEnabled ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              collaborationModesEnabled: !appSettings.collaborationModesEnabled,
            })
          }
          aria-pressed={appSettings.collaborationModesEnabled}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Personality</div>
          <div className="settings-toggle-subtitle">
            Choose Codex communication style (writes top-level <code>personality</code> in
            config.toml).
          </div>
        </div>
        <select
          id="features-personality-select"
          className="settings-select"
          value={appSettings.personality}
          onChange={(event) =>
            void onUpdateAppSettings({
              ...appSettings,
              personality: event.target.value as AppSettings["personality"],
            })
          }
          aria-label="Personality"
        >
          <option value="friendly">Friendly</option>
          <option value="pragmatic">Pragmatic</option>
        </select>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Steer mode</div>
          <div className="settings-toggle-subtitle">
            Send messages immediately. Use Tab to queue while a run is active.
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.steerEnabled ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              steerEnabled: !appSettings.steerEnabled,
            })
          }
          aria-pressed={appSettings.steerEnabled}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Background terminal</div>
          <div className="settings-toggle-subtitle">
            Run long-running terminal commands in the background.
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.unifiedExecEnabled ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              unifiedExecEnabled: !appSettings.unifiedExecEnabled,
            })
          }
          aria-pressed={appSettings.unifiedExecEnabled}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-subsection-title">Experimental Features</div>
      <div className="settings-subsection-subtitle">
        Preview features that may change or be removed.
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Shell tool</div>
          <div className="settings-toggle-subtitle">
            Set <code>[features].shell_tool</code>.
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.codexShellToolEnabled ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              codexShellToolEnabled: !appSettings.codexShellToolEnabled,
            })
          }
          aria-pressed={appSettings.codexShellToolEnabled}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Shell snapshot</div>
          <div className="settings-toggle-subtitle">
            Set <code>[features].shell_snapshot</code>.
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.codexShellSnapshotEnabled ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              codexShellSnapshotEnabled: !appSettings.codexShellSnapshotEnabled,
            })
          }
          aria-pressed={appSettings.codexShellSnapshotEnabled}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Apply patch freeform</div>
          <div className="settings-toggle-subtitle">
            Set <code>[features].apply_patch_freeform</code>.
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.codexApplyPatchFreeformEnabled ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              codexApplyPatchFreeformEnabled: !appSettings.codexApplyPatchFreeformEnabled,
            })
          }
          aria-pressed={appSettings.codexApplyPatchFreeformEnabled}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Exec policy</div>
          <div className="settings-toggle-subtitle">
            Set <code>[features].exec_policy</code>.
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.codexExecPolicyEnabled ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              codexExecPolicyEnabled: !appSettings.codexExecPolicyEnabled,
            })
          }
          aria-pressed={appSettings.codexExecPolicyEnabled}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Smart approvals</div>
          <div className="settings-toggle-subtitle">
            Set <code>[features].smart_approvals</code>.
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.codexSmartApprovalsEnabled ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              codexSmartApprovalsEnabled: !appSettings.codexSmartApprovalsEnabled,
            })
          }
          aria-pressed={appSettings.codexSmartApprovalsEnabled}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Remote compaction</div>
          <div className="settings-toggle-subtitle">
            Set <code>[features].remote_compaction</code>.
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.codexRemoteCompactionEnabled ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              codexRemoteCompactionEnabled: !appSettings.codexRemoteCompactionEnabled,
            })
          }
          aria-pressed={appSettings.codexRemoteCompactionEnabled}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Windows sandbox (experimental)</div>
          <div className="settings-toggle-subtitle">
            Set <code>[features].experimental_windows_sandbox</code>.
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.codexExperimentalWindowsSandboxEnabled ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              codexExperimentalWindowsSandboxEnabled:
                !appSettings.codexExperimentalWindowsSandboxEnabled,
            })
          }
          aria-pressed={appSettings.codexExperimentalWindowsSandboxEnabled}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Windows sandbox (elevated)</div>
          <div className="settings-toggle-subtitle">
            Set <code>[features].elevated_windows_sandbox</code>.
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.codexElevatedWindowsSandboxEnabled ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              codexElevatedWindowsSandboxEnabled: !appSettings.codexElevatedWindowsSandboxEnabled,
            })
          }
          aria-pressed={appSettings.codexElevatedWindowsSandboxEnabled}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Multi-agent</div>
          <div className="settings-toggle-subtitle">
            Enable multi-agent collaboration tools in Codex.
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.experimentalCollabEnabled ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              experimentalCollabEnabled: !appSettings.experimentalCollabEnabled,
            })
          }
          aria-pressed={appSettings.experimentalCollabEnabled}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
      <div className="settings-toggle-row">
        <div>
          <div className="settings-toggle-title">Apps</div>
          <div className="settings-toggle-subtitle">
            Enable ChatGPT apps/connectors and the <code>/apps</code> command.
          </div>
        </div>
        <button
          type="button"
          className={`settings-toggle ${appSettings.experimentalAppsEnabled ? "on" : ""}`}
          onClick={() =>
            void onUpdateAppSettings({
              ...appSettings,
              experimentalAppsEnabled: !appSettings.experimentalAppsEnabled,
            })
          }
          aria-pressed={appSettings.experimentalAppsEnabled}
        >
          <span className="settings-toggle-knob" />
        </button>
      </div>
    </section>
  );
}
