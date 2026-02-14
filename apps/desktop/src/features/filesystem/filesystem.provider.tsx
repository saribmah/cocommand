import { type PropsWithChildren, useContext, useRef } from "react";
import { ServerContext } from "../server/server.context";
import { FileSystemContext } from "./filesystem.context";
import { createFileSystemStore, type FileSystemStore } from "./filesystem.store";

type FileSystemProviderProps = PropsWithChildren;

export function FileSystemProvider({ children }: FileSystemProviderProps) {
  const serverStore = useContext(ServerContext);
  if (!serverStore) {
    throw new Error("Missing ServerContext.Provider in the tree");
  }

  const storeRef = useRef<FileSystemStore | null>(null);
  if (storeRef.current === null) {
    storeRef.current = createFileSystemStore(() => serverStore.getState().info);
  }

  return (
    <FileSystemContext.Provider value={storeRef.current}>
      {children}
    </FileSystemContext.Provider>
  );
}
