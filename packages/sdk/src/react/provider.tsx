import { useMemo } from "react";
import { createApiClient } from "../client";
import { createSdk } from "../sdk";
import { ExtensionSdkContext, SdkContext } from "./context";
import type { ExtensionSdkProviderProps, SdkProviderProps } from "./types";

export function SdkProvider({ baseUrl, children }: SdkProviderProps) {
  const sdk = useMemo(() => {
    return createSdk({
      client: createApiClient(baseUrl),
    });
  }, [baseUrl]);

  return <SdkContext.Provider value={sdk}>{children}</SdkContext.Provider>;
}

export function ExtensionSdkProvider({
  baseUrl,
  extensionId,
  composer,
  children,
}: ExtensionSdkProviderProps) {
  const sdk = useMemo(() => {
    return createSdk({
      client: createApiClient(baseUrl),
    });
  }, [baseUrl]);

  const extensionSdk = useMemo(() => {
    return sdk.extension(extensionId, { composer });
  }, [sdk, extensionId, composer]);

  return (
    <SdkContext.Provider value={sdk}>
      <ExtensionSdkContext.Provider value={extensionSdk}>
        {children}
      </ExtensionSdkContext.Provider>
    </SdkContext.Provider>
  );
}
