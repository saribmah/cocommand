import { create } from "zustand";
import { invokeExtensionTool } from "../../lib/extension-client";
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

export type NotesStore = ReturnType<typeof createNotesStore>;

export const createNotesStore = (getAddr: () => string | null) => {
  return create<NotesState>()((set, get) => ({
    notes: [],
    selectedNoteId: null,
    selectedNote: null,
    isLoading: false,
    isSaving: false,
    error: null,

    fetchNotes: async () => {
      const addr = getAddr();
      if (!addr) {
        set({ notes: [], isLoading: false, error: null });
        return;
      }

      set({ isLoading: true, error: null });

      try {
        const data = await invokeExtensionTool<ListNotesResponse>(
          addr,
          "notes",
          "list-notes",
          { limit: 500 },
        );
        set({ notes: data.notes, isLoading: false, error: null });
      } catch (error) {
        set({ notes: [], isLoading: false, error: String(error) });
      }
    },

    selectNote: async (id: string) => {
      const addr = getAddr();
      if (!addr) {
        throw new Error("Server unavailable");
      }

      set({ selectedNoteId: id, isLoading: true, error: null });

      try {
        const note = await invokeExtensionTool<Note>(
          addr,
          "notes",
          "read-note",
          { id },
        );
        set({ selectedNote: note, isLoading: false, error: null });
      } catch (error) {
        set({ selectedNote: null, isLoading: false, error: String(error) });
      }
    },

    clearSelection: () => {
      set({ selectedNoteId: null, selectedNote: null });
    },

    createNote: async (request?: CreateNoteRequest) => {
      const addr = getAddr();
      if (!addr) {
        throw new Error("Server unavailable");
      }

      set({ isSaving: true, error: null });

      try {
        const note = await invokeExtensionTool<Note>(
          addr,
          "notes",
          "create-note",
          (request as Record<string, unknown>) ?? {},
        );

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
      const addr = getAddr();
      if (!addr) {
        throw new Error("Server unavailable");
      }

      set({ isSaving: true, error: null });

      try {
        const note = await invokeExtensionTool<Note>(
          addr,
          "notes",
          "update-note",
          { id, content },
        );

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
      const addr = getAddr();
      if (!addr) {
        throw new Error("Server unavailable");
      }

      try {
        const result = await invokeExtensionTool<DeleteNoteResponse>(
          addr,
          "notes",
          "delete-note",
          { id },
        );

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
