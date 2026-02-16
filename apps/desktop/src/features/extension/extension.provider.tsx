import { type PropsWithChildren, useContext, useRef } from "react";
import { ServerContext } from "../server/server.context";
import { ExtensionContext } from "./extension.context";
import { createExtensionStore, type ExtensionStore } from "./extension.store";
import "./register-builtins";

type ExtensionProviderProps = PropsWithChildren;

export function ExtensionProvider({ children }: ExtensionProviderProps) {
  const serverStore = useContext(ServerContext);
  if (!serverStore) {
    throw new Error("Missing ServerContext.Provider in the tree");
  }

  const storeRef = useRef<ExtensionStore | null>(null);
  if (storeRef.current === null) {
    storeRef.current = createExtensionStore(() => serverStore.getState().info?.addr ?? null);
  }

  return (
    <ExtensionContext.Provider value={storeRef.current}>
      {children}
    </ExtensionContext.Provider>
  );
}
