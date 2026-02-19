import type { ComponentType } from "react";
import type { ComposerActions } from "../command/composer-actions";

export type ExtensionViewMode = "inline" | "popout";

export interface ExtensionViewProps {
  mode: ExtensionViewMode;
  actions?: ComposerActions;
}

export interface ExtensionViewConfig {
  component: ComponentType<ExtensionViewProps>;
  label: string;
  popout?: { width: number; height: number; title: string };
}

export type ExtensionViewSource = "builtin" | "dynamic";

interface RegisteredExtensionView {
  config: ExtensionViewConfig;
  source: ExtensionViewSource;
}

const registry = new Map<string, RegisteredExtensionView>();

export function registerExtensionView(
  extensionId: string,
  config: ExtensionViewConfig,
  options?: { source?: ExtensionViewSource },
): void {
  registry.set(extensionId, {
    config,
    source: options?.source ?? "dynamic",
  });
}

export function getExtensionView(extensionId: string): ExtensionViewConfig | undefined {
  return registry.get(extensionId)?.config;
}

export function hasExtensionView(extensionId: string): boolean {
  return registry.has(extensionId);
}

export function resetDynamicExtensionViews(): void {
  for (const [extensionId, entry] of registry.entries()) {
    if (entry.source === "dynamic") {
      registry.delete(extensionId);
    }
  }
}
