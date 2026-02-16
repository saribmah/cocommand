import type { ComponentType } from "react";
import type { ExtensionInvokeFn } from "./extension.types";
import type { ComposerActions } from "../command/composer-actions";

export type ExtensionViewMode = "inline" | "popout";

export interface ExtensionViewProps {
  mode: ExtensionViewMode;
  invoke?: ExtensionInvokeFn;
  extensionId?: string;
  actions?: ComposerActions;
}

export interface ExtensionViewConfig {
  component: ComponentType<ExtensionViewProps>;
  label: string;
  popout?: { width: number; height: number; title: string };
}

const registry = new Map<string, ExtensionViewConfig>();

export function registerExtensionView(extensionId: string, config: ExtensionViewConfig): void {
  registry.set(extensionId, config);
}

export function getExtensionView(extensionId: string): ExtensionViewConfig | undefined {
  return registry.get(extensionId);
}

export function hasExtensionView(extensionId: string): boolean {
  return registry.has(extensionId);
}
