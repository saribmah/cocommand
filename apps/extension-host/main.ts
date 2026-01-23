/**
 * Deno Extension Host — main entry point.
 *
 * Reads newline-delimited JSON-RPC 2.0 requests from stdin,
 * dispatches them to the appropriate handler, and writes
 * responses to stdout.
 */

import {
  type RpcRequest,
  type InitializeParams,
  type InvokeToolParams,
  ErrorCodes,
  successResponse,
  errorResponse,
} from "./protocol.ts";
import { type LoadedExtension, type ToolHandler, loadExtension } from "./loader.ts";

/** Currently loaded extension (one per host process). */
let loadedExtension: LoadedExtension | null = null;

/** Handle the `initialize` method: load the extension. */
async function handleInitialize(id: number, params: InitializeParams): Promise<void> {
  try {
    loadedExtension = await loadExtension(params.extension_dir);
    const toolIds = Array.from(loadedExtension.handlers.keys());
    respond(successResponse(id, { tools: toolIds }));
  } catch (err) {
    respond(
      errorResponse(
        id,
        ErrorCodes.INTERNAL_ERROR,
        `failed to initialize: ${err instanceof Error ? err.message : String(err)}`
      )
    );
  }
}

/** Handle the `invoke_tool` method: execute a tool handler. */
async function handleInvokeTool(id: number, params: InvokeToolParams): Promise<void> {
  if (!loadedExtension) {
    respond(errorResponse(id, ErrorCodes.INTERNAL_ERROR, "extension not initialized"));
    return;
  }

  const handler = loadedExtension.handlers.get(params.tool_id);
  if (!handler) {
    respond(
      errorResponse(id, ErrorCodes.METHOD_NOT_FOUND, `unknown tool: ${params.tool_id}`)
    );
    return;
  }

  try {
    const output = await handler(params.args);
    respond(successResponse(id, { output }));
  } catch (err) {
    respond(
      errorResponse(
        id,
        ErrorCodes.TOOL_EXECUTION_ERROR,
        `tool execution failed: ${err instanceof Error ? err.message : String(err)}`
      )
    );
  }
}

/** Write a JSON-RPC response to stdout (newline-delimited). */
function respond(response: unknown): void {
  const encoder = new TextEncoder();
  const line = JSON.stringify(response) + "\n";
  Deno.stdout.writeSync(encoder.encode(line));
}

/** Dispatch a parsed JSON-RPC request to the appropriate handler. */
async function dispatch(request: RpcRequest): Promise<void> {
  switch (request.method) {
    case "initialize":
      await handleInitialize(request.id, request.params as InitializeParams);
      break;
    case "invoke_tool":
      await handleInvokeTool(request.id, request.params as InvokeToolParams);
      break;
    case "shutdown":
      respond(successResponse(request.id, null));
      Deno.exit(0);
      break;
    default:
      respond(
        errorResponse(request.id, ErrorCodes.METHOD_NOT_FOUND, `unknown method: ${request.method}`)
      );
  }
}

/** Main loop: read lines from stdin and dispatch requests. */
async function main(): Promise<void> {
  const decoder = new TextDecoder();
  const reader = Deno.stdin.readable.getReader();
  let buffer = "";

  while (true) {
    const { value, done } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });

    // Process complete lines
    let newlineIdx: number;
    while ((newlineIdx = buffer.indexOf("\n")) !== -1) {
      const line = buffer.slice(0, newlineIdx).trim();
      buffer = buffer.slice(newlineIdx + 1);

      if (line.length === 0) continue;

      try {
        const request = JSON.parse(line) as RpcRequest;
        if (request.jsonrpc !== "2.0" || typeof request.id !== "number") {
          respond(errorResponse(0, ErrorCodes.INVALID_REQUEST, "invalid JSON-RPC request"));
          continue;
        }
        await dispatch(request);
      } catch {
        respond(errorResponse(0, ErrorCodes.PARSE_ERROR, "failed to parse JSON"));
      }
    }
  }
}

main();
