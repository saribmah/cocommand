import { type PropsWithChildren, useContext, useRef } from "react";
import { ServerContext } from "../server/server.context";
import { WorkspaceContext } from "./workspace.context";
import { createWorkspaceStore, type WorkspaceStore } from "./workspace.store";

type WorkspaceProviderProps = PropsWithChildren;

export function WorkspaceProvider({ children }: WorkspaceProviderProps) {
  const serverStore = useContext(ServerContext);
  if (!serverStore) {
    throw new Error("Missing ServerContext.Provider in the tree");
  }

  const storeRef = useRef<WorkspaceStore | null>(null);
  if (storeRef.current === null) {
    storeRef.current = createWorkspaceStore(() => serverStore.getState().info);
  }

  return (
    <WorkspaceContext.Provider value={storeRef.current}>
      {children}
    </WorkspaceContext.Provider>
  );
}
