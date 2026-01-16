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
