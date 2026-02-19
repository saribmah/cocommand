import { type PropsWithChildren, useRef } from "react";
import { useRuntimeSdk } from "../server/runtime-sdk.context";
import { SessionContext } from "./session.context";
import { createSessionStore, type SessionStore } from "./session.store";

type SessionProviderProps = PropsWithChildren;

export function SessionProvider({ children }: SessionProviderProps) {
  const sdk = useRuntimeSdk();

  const storeRef = useRef<SessionStore | null>(null);
  if (storeRef.current === null) {
    storeRef.current = createSessionStore(sdk);
  }

  return (
    <SessionContext.Provider value={storeRef.current}>
      {children}
    </SessionContext.Provider>
  );
}
