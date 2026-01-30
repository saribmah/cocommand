export interface SessionContext {
  workspace_id: string;
  session_id: string;
  started_at: number;
  ended_at: number | null;
}

export interface RecordMessageResponse {
  context: SessionContext;
  reply: string;
}
