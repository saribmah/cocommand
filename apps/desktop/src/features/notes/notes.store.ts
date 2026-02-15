import { create } from "zustand";
import type { ServerInfo } from "../../lib/ipc";
import type {
  Note,
  NoteSummary,
  ListNotesResponse,
  CreateNoteRequest,
  DeleteNoteResponse,
} from "./notes.types";

export interface NotesState {
  /** List of all notes (summaries) */
  notes: NoteSummary[];
  /** Currently selected note ID */
  selectedNoteId: string | null;
  /** Full content of selected note */
  selectedNote: Note | null;
  /** Whether notes list is loading */
  isLoading: boolean;
  /** Whether a note is being saved */
  isSaving: boolean;
  /** Error message if any */
  error: string | null;

  /** Fetch all notes from the server */
  fetchNotes: () => Promise<void>;
  /** Select and load a note by ID */
  selectNote: (id: string) => Promise<void>;
  /** Clear the current selection */
  clearSelection: () => void;
  /** Create a new note */
  createNote: (request?: CreateNoteRequest) => Promise<Note>;
  /** Update a note's content */
  updateNote: (id: string, content: string) => Promise<Note>;
  /** Delete a note */
  deleteNote: (id: string) => Promise<boolean>;
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

export type NotesStore = ReturnType<typeof createNotesStore>;

export const createNotesStore = (getServer: () => ServerInfo | null) => {
  return create<NotesState>()((set, get) => ({
    notes: [],
    selectedNoteId: null,
    selectedNote: null,
    isLoading: false,
    isSaving: false,
    error: null,

    fetchNotes: async () => {
      const server = getServer();
      if (!server || !server.addr) {
        set({ notes: [], isLoading: false, error: null });
        return;
      }

      set({ isLoading: true, error: null });

      const url = buildServerUrl(server.addr, "/extension/notes/list");
      try {
        const response = await fetch(url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ limit: 500 }),
        });
        if (!response.ok) {
          throw new Error(`Server error (${response.status})`);
        }
        const data = (await response.json()) as ListNotesResponse;
        set({ notes: data.notes, isLoading: false, error: null });
      } catch (error) {
        set({ notes: [], isLoading: false, error: String(error) });
      }
    },

    selectNote: async (id: string) => {
      const server = getServer();
      if (!server || !server.addr) {
        throw new Error("Server unavailable");
      }

      set({ selectedNoteId: id, isLoading: true, error: null });

      const url = buildServerUrl(server.addr, "/extension/notes/get");
      try {
        const response = await fetch(url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ id }),
        });
        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `Server error (${response.status})`);
        }
        const note = (await response.json()) as Note;
        set({ selectedNote: note, isLoading: false, error: null });
      } catch (error) {
        set({ selectedNote: null, isLoading: false, error: String(error) });
      }
    },

    clearSelection: () => {
      set({ selectedNoteId: null, selectedNote: null });
    },

    createNote: async (request?: CreateNoteRequest) => {
      const server = getServer();
      if (!server || !server.addr) {
        throw new Error("Server unavailable");
      }

      set({ isSaving: true, error: null });

      const url = buildServerUrl(server.addr, "/extension/notes/create");
      try {
        const response = await fetch(url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(request ?? {}),
        });
        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `Server error (${response.status})`);
        }
        const note = (await response.json()) as Note;

        // Add to notes list and select it
        const currentNotes = get().notes;
        const newSummary: NoteSummary = {
          id: note.id,
          title: note.title,
          preview: note.preview,
          path: note.path,
          modifiedAt: note.modifiedAt,
          size: note.size,
        };
        set({
          notes: [newSummary, ...currentNotes],
          selectedNoteId: note.id,
          selectedNote: note,
          isSaving: false,
          error: null,
        });

        return note;
      } catch (error) {
        set({ isSaving: false, error: String(error) });
        throw error;
      }
    },

    updateNote: async (id: string, content: string) => {
      const server = getServer();
      if (!server || !server.addr) {
        throw new Error("Server unavailable");
      }

      set({ isSaving: true, error: null });

      const url = buildServerUrl(server.addr, "/extension/notes/update");
      try {
        const response = await fetch(url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ id, content }),
        });
        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `Server error (${response.status})`);
        }
        const note = (await response.json()) as Note;

        // Update in notes list
        const currentNotes = get().notes;
        const updatedNotes = currentNotes.map((n) =>
          n.id === id
            ? {
                ...n,
                title: note.title,
                preview: note.preview,
                modifiedAt: note.modifiedAt,
                size: note.size,
              }
            : n
        );

        // Move updated note to top (most recent)
        const noteIndex = updatedNotes.findIndex((n) => n.id === id);
        if (noteIndex > 0) {
          const [movedNote] = updatedNotes.splice(noteIndex, 1);
          updatedNotes.unshift(movedNote);
        }

        set({
          notes: updatedNotes,
          selectedNote: note,
          isSaving: false,
          error: null,
        });

        return note;
      } catch (error) {
        set({ isSaving: false, error: String(error) });
        throw error;
      }
    },

    deleteNote: async (id: string) => {
      const server = getServer();
      if (!server || !server.addr) {
        throw new Error("Server unavailable");
      }

      const url = buildServerUrl(server.addr, "/extension/notes/delete");
      try {
        const response = await fetch(url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ id }),
        });
        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `Server error (${response.status})`);
        }
        const result = (await response.json()) as DeleteNoteResponse;

        if (result.deleted) {
          // Remove from notes list
          const currentNotes = get().notes;
          const filteredNotes = currentNotes.filter((n) => n.id !== id);

          // Clear selection if this note was selected
          const selectedId = get().selectedNoteId;
          if (selectedId === id) {
            set({
              notes: filteredNotes,
              selectedNoteId: null,
              selectedNote: null,
            });
          } else {
            set({ notes: filteredNotes });
          }
        }

        return result.deleted;
      } catch (error) {
        set({ error: String(error) });
        throw error;
      }
    },
  }));
};
