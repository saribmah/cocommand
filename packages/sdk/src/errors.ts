export type SdkErrorCode =
  | "http_error"
  | "api_error"
  | "tool_error"
  | "invalid_response"
  | "sse_error"
  | "sse_parse_error"
  | "aborted"
  | "timeout"
  | "not_implemented";

export interface SdkErrorOptions {
  code: SdkErrorCode;
  message: string;
  status?: number;
  source?: string;
  details?: unknown;
  cause?: unknown;
}

export class SdkError extends Error {
  readonly code: SdkErrorCode;
  readonly status?: number;
  readonly source?: string;
  readonly details?: unknown;

  constructor(options: SdkErrorOptions) {
    super(options.message);
    this.name = "SdkError";
    this.code = options.code;
    this.status = options.status;
    this.source = options.source;
    this.details = options.details;
    if (options.cause !== undefined) {
      (this as Error & { cause?: unknown }).cause = options.cause;
    }
  }
}

export class SdkNotImplementedError extends SdkError {
  readonly feature: string;

  constructor(feature: string, details?: unknown) {
    super({
      code: "not_implemented",
      message: `@cocommand/sdk: ${feature} is not implemented yet`,
      source: feature,
      details,
    });
    this.name = "SdkNotImplementedError";
    this.feature = feature;
  }
}

export function notImplemented(feature: string, details?: unknown): never {
  throw new SdkNotImplementedError(feature, details);
}

export function normalizeUnknownError(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

export function toSdkError(error: unknown, fallback: Omit<SdkErrorOptions, "message"> & { message?: string }): SdkError {
  if (error instanceof SdkError) return error;
  return new SdkError({
    code: fallback.code,
    message: fallback.message ?? normalizeUnknownError(error),
    status: fallback.status,
    source: fallback.source,
    details: fallback.details,
    cause: error,
  });
}
