import type { ExtensionInfo } from "../types";
import type { Sdk } from "../sdk";
import type {
  ExtensionViewModuleLoadResult,
  LoadExtensionViewModulesOptions,
} from "./types";

async function defaultModuleImporter(url: string): Promise<unknown> {
  return import(/* @vite-ignore */ url);
}

export async function loadExtensionViewModules(
  sdk: Sdk,
  extensions: ExtensionInfo[],
  options?: LoadExtensionViewModulesOptions,
): Promise<ExtensionViewModuleLoadResult[]> {
  const importer = options?.importer ?? defaultModuleImporter;
  const views = sdk.extensions.views.fromExtensions(extensions);

  return Promise.all(
    views.map(async (view): Promise<ExtensionViewModuleLoadResult> => {
      try {
        const loaded = await importer(view.assetUrl);
        if (!loaded || typeof loaded !== "object") {
          return {
            status: "rejected",
            view,
            reason: new Error(
              `Extension view module did not return an object for ${view.extensionId}`,
            ),
          };
        }
        return {
          status: "fulfilled",
          view,
          module: loaded as Record<string, unknown>,
        };
      } catch (error) {
        return {
          status: "rejected",
          view,
          reason: error,
        };
      }
    }),
  );
}
