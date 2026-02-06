import { create } from "zustand";
import {getServerStatus, type ServerInfo, ServerStatus} from "../../lib/ipc";

export interface ServerState {
    info: ServerInfo | null;
    status: "starting" | "ready" | "error";
    statusError: string | null;
    workspaceDir: string | null;
    setInfo: (info: ServerInfo) => void;
    clear: () => void;
    fetchStatus: () => Promise<void>;
    getInfo: () => ServerInfo | null;
}

export type ServerStore = ReturnType<typeof createServerStore>

export const createServerStore = (status: ServerStatus) => {
    return create<ServerState>()(
        (set, get) => ({
            info: null,
            status: status.status,
            statusError: status.error ?? null,
            workspaceDir: status.workspace_dir,
            setInfo: (info) => set({ info, workspaceDir: info.workspace_dir }),
            clear: () => set({ info: null }),
            fetchStatus: async () => {
                try {
                    const status = await getServerStatus();
                    const info =
                        typeof status.addr === "string" && status.addr.length > 0
                            ? { addr: status.addr, workspace_dir: status.workspace_dir }
                            : null;
                    set({
                        info,
                        status: status.status,
                        statusError: status.error ?? null,
                        workspaceDir: status.workspace_dir,
                    });
                } catch (error) {
                    set({
                        info: null,
                        workspaceDir: null,
                        status: "error",
                        statusError: String(error),
                    });
                }
            },
            getInfo: () => get().info,
        })
    )
};
