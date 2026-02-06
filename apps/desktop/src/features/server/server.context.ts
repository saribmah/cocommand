import { createContext, useContext } from 'react';
import { useStore } from 'zustand';
import {ServerState, ServerStore} from "./server.store.ts";

export const ServerContext = createContext<ServerStore | null>(null);

export function useServerContext<T>(selector: (state: ServerState) => T): T {
    const store = useContext(ServerContext);
    if (!store) throw new Error('Missing ServerContext.Provider in the tree');
    return useStore(store, selector);
}
