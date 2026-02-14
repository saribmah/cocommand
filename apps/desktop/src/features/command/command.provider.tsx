import { type PropsWithChildren, useRef } from "react";
import { CommandContext } from "./command.context";
import { createCommandStore, type CommandStore } from "./command.store";

type CommandProviderProps = PropsWithChildren;

export function CommandProvider({ children }: CommandProviderProps) {
  const storeRef = useRef<CommandStore | null>(null);
  if (storeRef.current === null) {
    storeRef.current = createCommandStore();
  }

  return (
    <CommandContext.Provider value={storeRef.current}>
      {children}
    </CommandContext.Provider>
  );
}
