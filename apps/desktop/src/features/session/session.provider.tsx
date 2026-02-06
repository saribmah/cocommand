import { type PropsWithChildren, useContext, useRef } from "react";
import { ServerContext } from "../server/server.context";
import { SessionContext } from "./session.context";
import { createSessionStore, type SessionStore } from "./session.store";

type SessionProviderProps = PropsWithChildren;

export function SessionProvider({ children }: SessionProviderProps) {
  const serverStore = useContext(ServerContext);
  if (!serverStore) {
    throw new Error("Missing ServerContext.Provider in the tree");
  }

  const storeRef = useRef<SessionStore | null>(null);
  if (storeRef.current === null) {
    storeRef.current = createSessionStore(() => serverStore.getState().info);
  }

  return (
    <SessionContext.Provider value={storeRef.current}>
      {children}
    </SessionContext.Provider>
  );
}
