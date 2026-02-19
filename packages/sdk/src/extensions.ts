import {
  listExtensions,
  openExtension,
  type Client,
  type ExtensionInfo,
  type ExtensionViewPopoutInfo,
} from "@cocommand/api";
import { resolveClientBaseUrl } from "./client";
import { unwrapApiResponse } from "./request";

export interface ResolvedExtensionViewAsset {
  extensionId: string;
  extensionName: string;
  extensionKind: string;
  entry: string;
  label: string;
  popout?: ExtensionViewPopoutInfo | null;
  assetPath: string;
  assetUrl: string;
}

export interface ExtensionViewsApi {
  resolveAssetUrl(extensionId: string, assetPath: string): string;
  fromExtensions(extensions: ExtensionInfo[]): ResolvedExtensionViewAsset[];
  listCustom(): Promise<ResolvedExtensionViewAsset[]>;
}

export interface ExtensionsApi {
  list(): Promise<ExtensionInfo[]>;
  open(id: string): Promise<unknown>;
  views: ExtensionViewsApi;
}

function normalizeAssetPath(assetPath: string): string {
  return assetPath
    .replace(/^\/+/, "")
    .split("/")
    .filter((segment) => segment.length > 0)
    .map((segment) => encodeURIComponent(segment))
    .join("/");
}

export function createExtensionsApi(client: Client): ExtensionsApi {
  const resolveAssetUrl = (extensionId: string, assetPath: string): string => {
    const baseUrl = resolveClientBaseUrl(client);
    const encodedExtensionId = encodeURIComponent(extensionId);
    const encodedAssetPath = normalizeAssetPath(assetPath);

    if (!encodedAssetPath) {
      return `${baseUrl}/extension/${encodedExtensionId}/assets`;
    }
    return `${baseUrl}/extension/${encodedExtensionId}/assets/${encodedAssetPath}`;
  };

  const fromExtensions = (extensions: ExtensionInfo[]): ResolvedExtensionViewAsset[] => {
    return extensions
      .filter((extension) => extension.kind === "custom" && extension.view != null)
      .map((extension) => {
        const view = extension.view!;
        return {
          extensionId: extension.id,
          extensionName: extension.name,
          extensionKind: extension.kind,
          entry: view.entry,
          label: view.label,
          popout: view.popout ?? null,
          assetPath: view.entry,
          assetUrl: resolveAssetUrl(extension.id, view.entry),
        };
      });
  };

  const list = async (): Promise<ExtensionInfo[]> => {
    const result = await listExtensions({ client });
    return unwrapApiResponse("extensions.list", result);
  };

  const views: ExtensionViewsApi = {
    resolveAssetUrl,
    fromExtensions,
    async listCustom() {
      const extensions = await list();
      return fromExtensions(extensions);
    },
  };

  return {
    list,
    async open(id: string) {
      const result = await openExtension({ client, body: { id } });
      return unwrapApiResponse("extensions.open", result, { allowNull: true });
    },
    views,
  };
}
