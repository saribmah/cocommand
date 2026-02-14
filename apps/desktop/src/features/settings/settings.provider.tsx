import { type PropsWithChildren, useRef } from "react";
import { SettingsContext } from "./settings.context";
import { createSettingsStore, type SettingsStore } from "./settings.store";

type SettingsProviderProps = PropsWithChildren;

export function SettingsProvider({ children }: SettingsProviderProps) {
  const storeRef = useRef<SettingsStore | null>(null);
  if (storeRef.current === null) {
    storeRef.current = createSettingsStore();
  }

  return (
    <SettingsContext.Provider value={storeRef.current}>
      {children}
    </SettingsContext.Provider>
  );
}
