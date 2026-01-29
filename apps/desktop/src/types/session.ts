export interface SessionMessage {
  seq: number;
  timestamp: string;
  role: string;
  text: string;
}

export interface SessionContext {
  workspace_id: string;
  session_id: string;
  started_at: number;
  ended_at: number | null;
  messages: SessionMessage[];
}
