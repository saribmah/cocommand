import { createContext, useContext } from "react";
import { useStore } from "zustand";
import type { CommandState, CommandStore } from "./command.store";

export const CommandContext = createContext<CommandStore | null>(null);

export function useCommandContext<T>(selector: (state: CommandState) => T): T {
  const store = useContext(CommandContext);
  if (!store) {
    throw new Error("Missing CommandContext.Provider in the tree");
  }
  return useStore(store, selector);
}
