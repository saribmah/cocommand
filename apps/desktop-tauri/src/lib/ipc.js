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
 * Save or update a command JSON file.
 * @param {CommandDefinition} command
 * @returns {Promise<{status: string, file?: string, message?: string}>}
 */
export async function saveCommand(command) {
  return invoke("save_command", { request: { command } });
}

/**
 * Delete a command JSON file by id.
 * @param {string} id
 * @returns {Promise<{status: string, file?: string, message?: string}>}
 */
export async function deleteCommand(id) {
  return invoke("delete_command", { request: { id } });
}
