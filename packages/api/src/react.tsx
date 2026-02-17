import { createContext, useContext, useRef } from "react";
import { createApi, type CocommandApi } from "./create-api";
import { createTransport } from "./transport";
import type { ComposerActionsBridge } from "./configure";

const ApiContext = createContext<CocommandApi | null>(null);

export interface ApiProviderProps {
  baseUrl: string;
  extensionId: string;
  composer?: ComposerActionsBridge;
  children: React.ReactNode;
}

export function ApiProvider({ baseUrl, extensionId, composer, children }: ApiProviderProps) {
  const apiRef = useRef<CocommandApi | null>(null);

  if (!apiRef.current) {
    apiRef.current = createApi({
      transport: createTransport(baseUrl),
      extensionId,
      composer,
    });
  }

  return <ApiContext.Provider value={apiRef.current}>{children}</ApiContext.Provider>;
}

export function useApi(): CocommandApi {
  const api = useContext(ApiContext);
  if (!api) {
    throw new Error(
      "@cocommand/api: useApi() must be used within an <ApiProvider>. " +
      "Wrap your extension view in <ApiProvider baseUrl={...} extensionId={...}>.",
    );
  }
  return api;
}
