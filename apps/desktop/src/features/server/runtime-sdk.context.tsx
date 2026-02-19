import { createContext, useContext, useMemo, type PropsWithChildren } from "react";
import { createSdk, createSdkClient, type Sdk } from "@cocommand/sdk";
import type { ServerInfo } from "../../lib/ipc";

const RuntimeSdkContext = createContext<Sdk | null>(null);

interface RuntimeSdkProviderProps extends PropsWithChildren {
  serverInfo: ServerInfo;
}

export function RuntimeSdkProvider({ serverInfo, children }: RuntimeSdkProviderProps) {
  const sdk = useMemo(() => {
    if (!serverInfo.addr) {
      throw new Error("RuntimeSdkProvider requires a ready server address");
    }
    return createSdk({ client: createSdkClient(serverInfo.addr) });
  }, [serverInfo.addr]);

  return (
    <RuntimeSdkContext.Provider value={sdk}>
      {children}
    </RuntimeSdkContext.Provider>
  );
}

export function useRuntimeSdk(): Sdk {
  const sdk = useContext(RuntimeSdkContext);
  if (!sdk) {
    throw new Error("Missing RuntimeSdkProvider in the tree");
  }
  return sdk;
}
