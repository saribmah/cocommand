import { createContext, useContext, useMemo } from "react";
import { createApiClient } from "../client";
import { createSdk, type ExtensionSdk, type Sdk } from "../sdk";
import type { ComposerActionsBridge } from "../configure";

const SdkContext = createContext<Sdk | null>(null);
const ExtensionSdkContext = createContext<ExtensionSdk | null>(null);

export interface SdkProviderProps {
  baseUrl: string;
  children: React.ReactNode;
}

export interface ExtensionSdkProviderProps {
  baseUrl: string;
  extensionId: string;
  composer?: ComposerActionsBridge;
  children: React.ReactNode;
}

export function SdkProvider({ baseUrl, children }: SdkProviderProps) {
  const sdk = useMemo(() => {
    return createSdk({
      client: createApiClient(baseUrl),
    });
  }, [baseUrl]);

  return <SdkContext.Provider value={sdk}>{children}</SdkContext.Provider>;
}

export function ExtensionSdkProvider({ baseUrl, extensionId, composer, children }: ExtensionSdkProviderProps) {
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

export function useSdk(): Sdk {
  const sdk = useContext(SdkContext);
  if (!sdk) {
    throw new Error(
      "@cocommand/sdk/react: useSdk() must be used inside <SdkProvider> or <ExtensionSdkProvider>.",
    );
  }
  return sdk;
}

export function useExtensionSdk(): ExtensionSdk {
  const sdk = useContext(ExtensionSdkContext);
  if (!sdk) {
    throw new Error(
      "@cocommand/sdk/react: useExtensionSdk() must be used inside <ExtensionSdkProvider>.",
    );
  }
  return sdk;
}
