import { type PropsWithChildren, useContext, useRef } from "react";
import { ServerContext } from "../server/server.context";
import { NotesContext } from "./notes.context";
import { createNotesStore, type NotesStore } from "./notes.store";

type NotesProviderProps = PropsWithChildren;

export function NotesProvider({ children }: NotesProviderProps) {
  const serverStore = useContext(ServerContext);
  if (!serverStore) {
    throw new Error("Missing ServerContext.Provider in the tree");
  }

  const storeRef = useRef<NotesStore | null>(null);
  if (storeRef.current === null) {
    storeRef.current = createNotesStore(() => serverStore.getState().info?.addr ?? null);
  }

  return (
    <NotesContext.Provider value={storeRef.current}>
      {children}
    </NotesContext.Provider>
  );
}
