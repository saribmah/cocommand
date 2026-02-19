import { type PropsWithChildren, useRef } from "react";
import { useRuntimeSdk } from "../server/runtime-sdk.context";
import { ExtensionContext } from "./extension.context";
import { resetDynamicExtensionStoreFactories } from "./extension-stores";
import { resetDynamicExtensionViews } from "./extension-views";
import { createExtensionStore, type ExtensionStore } from "./extension.store";
import "./register-builtins";

type ExtensionProviderProps = PropsWithChildren;

export function ExtensionProvider({ children }: ExtensionProviderProps) {
  const sdk = useRuntimeSdk();

  const storeRef = useRef<ExtensionStore | null>(null);
  if (storeRef.current === null) {
    resetDynamicExtensionViews();
    resetDynamicExtensionStoreFactories();
    storeRef.current = createExtensionStore(sdk);
  }

  return (
    <ExtensionContext.Provider value={storeRef.current}>
      {children}
    </ExtensionContext.Provider>
  );
}
