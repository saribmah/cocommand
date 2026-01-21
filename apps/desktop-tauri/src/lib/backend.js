// @ts-check

/**
 * Backend API client for cocommand server.
 *
 * Provides a thin wrapper around the HTTP API exposed by the Rust backend.
 */

const BASE_URL = "http://127.0.0.1:4840";

/**
 * @typedef {Object} WorkspaceSnapshot
 * @property {string|null} focused_app
 * @property {Array<{id: string, summary: string}>} open_apps
 * @property {string} staleness
 */

/**
 * @typedef {Object} WindowResponse
 * @property {string} status
 * @property {WorkspaceSnapshot|null} snapshot
 * @property {string|null} message
 * @property {boolean} [soft_reset]
 * @property {boolean} [archived]
 */

/**
 * @typedef {Object} AppDefinition
 * @property {string} id
 * @property {string} name
 * @property {string} version
 * @property {string} description
 * @property {Array<ToolDefinition>} tools
 */

/**
 * @typedef {Object} ToolDefinition
 * @property {string} id
 * @property {string} name
 * @property {string} description
 * @property {Object} parameters
 */

/**
 * @typedef {Object} ExecuteResponse
 * @property {string} status
 * @property {string|null} message
 */

/**
 * @typedef {Object} CommandResponse
 * @property {string} status
 * @property {Object|null} command
 * @property {string|null} app_id
 * @property {string|null} tool_id
 * @property {{status: string, message: string}|null} result
 * @property {string|null} message
 * @property {string} [phase]
 * @property {number} [turns_used]
 */

/**
 * Check backend health.
 * @returns {Promise<{status: string}>}
 */
export async function getHealth() {
  const response = await fetch(`${BASE_URL}/health`);
  return response.json();
}

/**
 * Get the current workspace snapshot with lifecycle info.
 * @returns {Promise<WindowResponse>}
 */
export async function getSnapshot() {
  const response = await fetch(`${BASE_URL}/window/snapshot`);
  return response.json();
}

/**
 * List all available applications.
 * @returns {Promise<Array<AppDefinition>>}
 */
export async function listApps() {
  const response = await fetch(`${BASE_URL}/apps`);
  return response.json();
}

/**
 * List tools for currently open applications.
 * @returns {Promise<Array<ToolDefinition>>}
 */
export async function listTools() {
  const response = await fetch(`${BASE_URL}/tools`);
  return response.json();
}

/**
 * Open an application in the workspace.
 * @param {string} appId - The application ID to open.
 * @returns {Promise<WindowResponse>}
 */
export async function openApp(appId) {
  const response = await fetch(`${BASE_URL}/window/open`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ appId }),
  });
  return response.json();
}

/**
 * Close an application in the workspace.
 * @param {string} appId - The application ID to close.
 * @returns {Promise<WindowResponse>}
 */
export async function closeApp(appId) {
  const response = await fetch(`${BASE_URL}/window/close`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ appId }),
  });
  return response.json();
}

/**
 * Focus an already-open application.
 * @param {string} appId - The application ID to focus.
 * @returns {Promise<WindowResponse>}
 */
export async function focusApp(appId) {
  const response = await fetch(`${BASE_URL}/window/focus`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ appId }),
  });
  return response.json();
}

/**
 * Execute a tool directly.
 * @param {string} toolId - The tool ID to execute.
 * @param {Object} inputs - The inputs for the tool.
 * @returns {Promise<ExecuteResponse>}
 */
export async function executeTool(toolId, inputs = {}) {
  const response = await fetch(`${BASE_URL}/execute`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ tool_id: toolId, inputs }),
  });
  return response.json();
}

/**
 * Submit a command for processing.
 * @param {string} text - The command text.
 * @returns {Promise<CommandResponse>}
 */
export async function submitCommand(text) {
  const response = await fetch(`${BASE_URL}/command`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ text }),
  });
  return response.json();
}

/**
 * Restore an archived workspace.
 * @returns {Promise<WindowResponse>}
 */
export async function restoreWorkspace() {
  const response = await fetch(`${BASE_URL}/window/restore`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({}),
  });
  return response.json();
}

// ============================================================================
// LLM Settings API
// ============================================================================

/**
 * @typedef {Object} RedactedLlmSettings
 * @property {string} provider - "openai", "anthropic", or "custom"
 * @property {string} auth_method - "api_key"
 * @property {boolean} has_api_key - Whether an API key is configured
 * @property {string|null} base_url - Custom base URL (if any)
 * @property {string} model - Model identifier
 */

/**
 * @typedef {Object} LlmSettingsResponse
 * @property {string} status
 * @property {RedactedLlmSettings|null} settings
 * @property {string|null} message
 */

/**
 * @typedef {Object} ProviderInfo
 * @property {string} id
 * @property {string} name
 * @property {boolean} supports_api_key
 * @property {string|null} default_base_url
 * @property {string[]} default_models
 */

/**
 * @typedef {Object} ProvidersResponse
 * @property {string} status
 * @property {ProviderInfo[]} providers
 */

/**
 * Get current LLM settings (redacted, no secrets).
 * @returns {Promise<LlmSettingsResponse>}
 */
export async function getLlmSettings() {
  const response = await fetch(`${BASE_URL}/llm/settings`);
  return response.json();
}

/**
 * Update LLM settings.
 * @param {Object} settings - Settings to update
 * @param {string} [settings.provider] - Provider ID
 * @param {string} [settings.auth_method] - Auth method
 * @param {string} [settings.api_key] - API key (if using api_key auth)
 * @param {string} [settings.base_url] - Custom base URL
 * @param {string} [settings.model] - Model identifier
 * @returns {Promise<LlmSettingsResponse>}
 */
export async function updateLlmSettings(settings) {
  const response = await fetch(`${BASE_URL}/llm/settings`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(settings),
  });
  return response.json();
}

/**
 * List available LLM providers.
 * @returns {Promise<ProvidersResponse>}
 */
export async function listLlmProviders() {
  const response = await fetch(`${BASE_URL}/llm/providers`);
  return response.json();
}
