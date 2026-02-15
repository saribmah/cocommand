/**
 * Summary of a note (used in list views).
 * Matches backend NoteSummaryPayload.
 */
export interface NoteSummary {
  id: string;
  title: string;
  preview: string;
  path: string;
  modifiedAt: number | null;
  size: number | null;
}

/**
 * Full note with content.
 * Matches backend NotePayload.
 */
export interface Note {
  id: string;
  title: string;
  preview: string;
  content: string;
  path: string;
  modifiedAt: number | null;
  size: number | null;
}

/**
 * Response from list notes endpoint.
 * Matches backend ListNotesPayload.
 */
export interface ListNotesResponse {
  root: string;
  notes: NoteSummary[];
  count: number;
  truncated: boolean;
  errors: number;
}

/**
 * Response from delete note endpoint.
 */
export interface DeleteNoteResponse {
  status: string;
  deleted: boolean;
}

/**
 * Request payload for creating a note.
 */
export interface CreateNoteRequest {
  title?: string;
  content?: string;
  folder?: string;
}

/**
 * Request payload for updating a note.
 */
export interface UpdateNoteRequest {
  id: string;
  content: string;
}
