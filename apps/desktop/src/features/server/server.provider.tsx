import React, { useRef } from 'react';
import {ServerContext} from "./server.context.ts";
import {createServerStore, type ServerStore} from "./server.store.ts";
import {ServerStatus} from "../../lib/ipc.ts";

type ServerProviderProps = React.PropsWithChildren & {
    status: ServerStatus
};

export const ServerProvider = ({ children, status }: ServerProviderProps) => {
    // Initialize a fresh server store per provider mount
    const storeRef = useRef<ServerStore>(null)
    if (storeRef.current === null) {
        storeRef.current = createServerStore(status);
    }

    return (
        <ServerContext.Provider value={storeRef.current}>
            {children}
        </ServerContext.Provider>
    );
};
