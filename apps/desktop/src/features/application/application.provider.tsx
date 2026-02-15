import { type PropsWithChildren, useContext, useRef } from "react";
import { ServerContext } from "../server/server.context";
import { ApplicationContext } from "./application.context";
import {
  createApplicationStore,
  type ApplicationStore,
} from "./application.store";

type ApplicationProviderProps = PropsWithChildren;

export function ApplicationProvider({ children }: ApplicationProviderProps) {
  const serverStore = useContext(ServerContext);
  if (!serverStore) {
    throw new Error("Missing ServerContext.Provider in the tree");
  }

  const storeRef = useRef<ApplicationStore | null>(null);
  if (storeRef.current === null) {
    storeRef.current = createApplicationStore(() => serverStore.getState().info);
  }

  return (
    <ApplicationContext.Provider value={storeRef.current}>
      {children}
    </ApplicationContext.Provider>
  );
}
