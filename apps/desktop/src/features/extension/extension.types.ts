import type {
  ExtensionInfo as SdkExtensionInfo,
  ExtensionToolInfo as SdkExtensionToolInfo,
  ExtensionViewInfo as SdkExtensionViewInfo,
} from "@cocommand/sdk";

export type ExtensionToolInfo = SdkExtensionToolInfo;
export type ExtensionViewInfo = SdkExtensionViewInfo;
export type ExtensionInfo = SdkExtensionInfo;

export type ExtensionInvokeFn = <T = unknown>(
  extensionId: string,
  toolId: string,
  input?: Record<string, unknown>,
  options?: { signal?: AbortSignal },
) => Promise<T>;
