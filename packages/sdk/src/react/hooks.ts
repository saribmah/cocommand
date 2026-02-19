import { useContext } from "react";
import type { ExtensionSdk, Sdk } from "../sdk";
import { ExtensionSdkContext, SdkContext } from "./context";

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
