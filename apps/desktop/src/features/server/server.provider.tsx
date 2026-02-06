import React, { useRef } from "react";
import { ServerContext } from "./server.context.ts";
import { createServerStore, type ServerStore } from "./server.store.ts";
import { ServerInfo } from "../../lib/ipc.ts";

type ServerProviderProps = React.PropsWithChildren & {
  serverInfo: ServerInfo;
};

export const ServerProvider = ({ children, serverInfo }: ServerProviderProps) => {
  // Initialize a fresh server store per provider mount
  const storeRef = useRef<ServerStore>(null);
  if (storeRef.current === null) {
    storeRef.current = createServerStore(serverInfo);
  }

  return <ServerContext.Provider value={storeRef.current}>{children}</ServerContext.Provider>;
};
