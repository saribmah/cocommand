// @ts-check
import { invoke } from "@tauri-apps/api/core";

/**
 * @typedef {Object} CommandRequest
 * @property {string} input
 */

/**
 * @typedef {Object} CommandResponse
 * @property {"ok" | "empty"} status
 * @property {string} output
 */

/**
 * Execute a command via the Tauri backend.
 * @param {string} input
 * @returns {Promise<CommandResponse>}
 */
export async function executeCommand(input) {
  /** @type {CommandRequest} */
  const payload = { input };
  return invoke("execute_command", { request: payload });
}

/**
 * @typedef {Object} PlanRequest
 * @property {string} input
 */

/**
 * @typedef {Object} PlanStep
 * @property {string} id
 * @property {string} tool
 * @property {Record<string, unknown>} inputs
 * @property {"pending" | "running" | "completed" | "failed"} status
 */

/**
 * @typedef {Object} Intent
 * @property {string} id
 * @property {string} name
 * @property {number} confidence
 * @property {Record<string, unknown>} parameters
 */

/**
 * @typedef {Object} ExecutionPlan
 * @property {string} id
 * @property {Intent} intent
 * @property {PlanStep[]} steps
 * @property {string} createdAt
 */

/**
 * @typedef {Object} PlanResponse
 * @property {"ok" | "empty" | "error"} status
 * @property {ExecutionPlan=} plan
 * @property {string=} message
 */

/**
 * Plan a command via the Tauri backend.
 * @param {string} input
 * @returns {Promise<PlanResponse>}
 */
export async function planCommand(input) {
  /** @type {PlanRequest} */
  const payload = { input };
  return invoke("plan_command", { request: payload });
}

/**
 * @typedef {Object} WorkflowRunRequest
 * @property {string} id
 */

/**
 * @typedef {Object} WorkflowRunStep
 * @property {string} id
 * @property {string} command_id
 * @property {string} status
 * @property {string=} message
 */

/**
 * @typedef {Object} WorkflowRunResponse
 * @property {"ok" | "failed" | "error"} status
 * @property {string} summary
 * @property {WorkflowRunStep[]} steps
 */

/**
 * Execute a workflow via the Tauri backend.
 * @param {string} id
 * @returns {Promise<WorkflowRunResponse>}
 */
export async function runWorkflow(id) {
  /** @type {WorkflowRunRequest} */
  const payload = { id };
  return invoke("run_workflow", { request: payload });
}

/**
 * @typedef {Object} CommandDefinition
 * @property {string} id
 * @property {string} name
 * @property {string} version
 * @property {string=} description
 * @property {Record<string, unknown>=} inputs
 * @property {Array<Record<string, unknown>>} steps
 * @property {Record<string, string>=} permissions
 */

/**
 * @typedef {Object} CommandLoadError
 * @property {string} file
 * @property {string} message
 */

/**
 * @typedef {Object} CommandLoadResponse
 * @property {CommandDefinition[]} commands
 * @property {CommandLoadError[]} errors
 */

/**
 * Load commands from the Tauri backend.
 * @returns {Promise<CommandLoadResponse>}
 */
export async function listCommands() {
  return invoke("list_commands");
}

/**
 * @typedef {Object} WorkflowDefinition
 * @property {string} id
 * @property {string} name
 * @property {string} version
 * @property {string=} description
 * @property {Record<string, unknown>=} inputs
 * @property {Array<Record<string, unknown>>} steps
 * @property {Record<string, string>=} permissions
 */

/**
 * @typedef {Object} WorkflowLoadError
 * @property {string} file
 * @property {string} message
 */

/**
 * @typedef {Object} WorkflowLoadResponse
 * @property {WorkflowDefinition[]} workflows
 * @property {WorkflowLoadError[]} errors
 */

/**
 * Load workflows from the Tauri backend.
 * @returns {Promise<WorkflowLoadResponse>}
 */
export async function listWorkflows() {
  return invoke("list_workflows");
}

/**
 * Save or update a command JSON file.
 * @param {CommandDefinition} command
 * @returns {Promise<{status: string, file?: string, message?: string}>}
 */
export async function saveCommand(command) {
  return invoke("save_command", { request: { command } });
}

/**
 * Save or update a workflow JSON file.
 * @param {WorkflowDefinition} workflow
 * @returns {Promise<{status: string, file?: string, message?: string}>}
 */
export async function saveWorkflow(workflow) {
  return invoke("save_workflow", { request: { workflow } });
}

/**
 * Delete a command JSON file by id.
 * @param {string} id
 * @returns {Promise<{status: string, file?: string, message?: string}>}
 */
export async function deleteCommand(id) {
  return invoke("delete_command", { request: { id } });
}

/**
 * Delete a workflow JSON file by id.
 * @param {string} id
 * @returns {Promise<{status: string, file?: string, message?: string}>}
 */
export async function deleteWorkflow(id) {
  return invoke("delete_workflow", { request: { id } });
}

/**
 * Hide the main window.
 * @returns {Promise<void>}
 */
export async function hideWindow() {
  return invoke("hide_window");
}
