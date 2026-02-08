export interface SessionContext {
  workspace_id: string;
  session_id: string;
  started_at: number;
  ended_at: number | null;
}

export interface StreamEvent<T = unknown> {
  event: string;
  data: T;
}
