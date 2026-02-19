import { type PropsWithChildren, useRef } from "react";
import { useRuntimeSdk } from "../server/runtime-sdk.context";
import { ApplicationContext } from "./application.context";
import {
  createApplicationStore,
  type ApplicationStore,
} from "./application.store";

type ApplicationProviderProps = PropsWithChildren;

export function ApplicationProvider({ children }: ApplicationProviderProps) {
  const sdk = useRuntimeSdk();

  const storeRef = useRef<ApplicationStore | null>(null);
  if (storeRef.current === null) {
    storeRef.current = createApplicationStore(sdk);
  }

  return (
    <ApplicationContext.Provider value={storeRef.current}>
      {children}
    </ApplicationContext.Provider>
  );
}
