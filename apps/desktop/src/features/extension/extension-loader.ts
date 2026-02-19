import type { ComponentType } from "react";
import type { Sdk } from "@cocommand/sdk";
import { loadExtensionViewModules } from "@cocommand/sdk/react";
import type { ExtensionStoreFactory } from "./extension-stores";
import { registerExtensionView } from "./extension-views";
import type { ExtensionViewProps } from "./extension-views";
import { registerExtensionStore } from "./extension-stores";
import type { ExtensionInfo } from "./extension.types";

export async function loadDynamicExtensionViews(
  sdk: Sdk,
  extensions: ExtensionInfo[],
): Promise<string[]> {
  const results = await loadExtensionViewModules(sdk, extensions);

  const loaded: string[] = [];
  for (const result of results) {
    if (result.status === "fulfilled") {
      const mod = result.module as {
        default?: unknown;
        createStore?: unknown;
      };
      const { view } = result;
      if (mod.default) {
        registerExtensionView(view.extensionId, {
          component: mod.default as ComponentType<ExtensionViewProps>,
          label: view.label,
          popout: view.popout ?? undefined,
        });
      }

      if (typeof mod.createStore === "function") {
        registerExtensionStore(
          view.extensionId,
          mod.createStore as ExtensionStoreFactory,
        );
      }
      loaded.push(view.extensionId);
    } else {
      console.warn(
        `Failed to load dynamic extension view for "${result.view.extensionId}":`,
        result.reason,
      );
    }
  }

  return loaded;
}
