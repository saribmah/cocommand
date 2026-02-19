import type { ReactNode } from "react";
import type { ComposerActionsBridge } from "../configure";
import type { ResolvedExtensionViewAsset } from "../extensions";

export interface SdkProviderProps {
  baseUrl: string;
  children: ReactNode;
}

export interface ExtensionSdkProviderProps {
  baseUrl: string;
  extensionId: string;
  composer?: ComposerActionsBridge;
  children: ReactNode;
}

export interface ExtensionViewModuleLoadSuccess {
  status: "fulfilled";
  view: ResolvedExtensionViewAsset;
  module: Record<string, unknown>;
}

export interface ExtensionViewModuleLoadFailure {
  status: "rejected";
  view: ResolvedExtensionViewAsset;
  reason: unknown;
}

export type ExtensionViewModuleLoadResult =
  | ExtensionViewModuleLoadSuccess
  | ExtensionViewModuleLoadFailure;

export interface LoadExtensionViewModulesOptions {
  importer?: (url: string) => Promise<unknown>;
}
