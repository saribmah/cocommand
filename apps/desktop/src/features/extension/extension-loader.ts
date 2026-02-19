import { registerExtensionView } from "./extension-views";
import { registerExtensionStore } from "./extension-stores";
import type { ExtensionInfo } from "./extension.types";

export async function loadDynamicExtensionViews(
  extensions: ExtensionInfo[],
  serverAddr: string,
): Promise<string[]> {
  const prefix = serverAddr.startsWith("http") ? serverAddr : `http://${serverAddr}`;
  const candidates = extensions.filter((ext) => ext.view && ext.kind === "custom");

  if (candidates.length === 0) return [];

  const results = await Promise.allSettled(
    candidates.map(async (ext) => {
      const assetUrl = `${prefix}/extension/${ext.id}/assets/${ext.view!.entry}`;
      const mod = await import(/* @vite-ignore */ assetUrl);

      if (mod.default) {
        registerExtensionView(ext.id, {
          component: mod.default,
          label: ext.view!.label,
          popout: ext.view!.popout ?? undefined,
        });
      }

      if (typeof mod.createStore === "function") {
        registerExtensionStore(ext.id, mod.createStore);
      }

      return ext.id;
    }),
  );

  const loaded: string[] = [];
  for (const result of results) {
    if (result.status === "fulfilled") {
      loaded.push(result.value);
    } else {
      console.warn("Failed to load dynamic extension view:", result.reason);
    }
  }
  return loaded;
}
