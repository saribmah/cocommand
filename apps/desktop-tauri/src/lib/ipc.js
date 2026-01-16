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
