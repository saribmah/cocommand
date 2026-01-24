/**
 * JSON-RPC 2.0 protocol types for the extension host.
 *
 * Communication between the Rust core and Deno host uses
 * newline-delimited JSON-RPC 2.0 messages over stdio.
 */

/** JSON-RPC 2.0 request. */
export interface RpcRequest {
  jsonrpc: "2.0";
  id: number;
  method: string;
  params?: unknown;
}

/** JSON-RPC 2.0 successful response. */
export interface RpcSuccessResponse {
  jsonrpc: "2.0";
  id: number;
  result: unknown;
}

/** JSON-RPC 2.0 error object. */
export interface RpcError {
  code: number;
  message: string;
  data?: unknown;
}

/** JSON-RPC 2.0 error response. */
export interface RpcErrorResponse {
  jsonrpc: "2.0";
  id: number;
  error: RpcError;
}

/** Union of success and error responses. */
export type RpcResponse = RpcSuccessResponse | RpcErrorResponse;

/** Parameters for the `initialize` method. */
export interface InitializeParams {
  extension_dir: string;
  extension_id: string;
}

/** Result of the `initialize` method. */
export interface InitializeResult {
  tools: string[];
}

/** Parameters for the `invoke_tool` method. */
export interface InvokeToolParams {
  tool_id: string;
  args: Record<string, unknown>;
}

/** Result of the `invoke_tool` method. */
export interface InvokeToolResult {
  output: unknown;
}

/** Standard JSON-RPC error codes. */
export const ErrorCodes = {
  PARSE_ERROR: -32700,
  INVALID_REQUEST: -32600,
  METHOD_NOT_FOUND: -32601,
  INVALID_PARAMS: -32602,
  INTERNAL_ERROR: -32603,
  TOOL_EXECUTION_ERROR: -32000,
} as const;

/** Create a success response. */
export function successResponse(id: number, result: unknown): RpcSuccessResponse {
  return { jsonrpc: "2.0", id, result };
}

/** Create an error response. */
export function errorResponse(id: number, code: number, message: string): RpcErrorResponse {
  return { jsonrpc: "2.0", id, error: { code, message } };
}
