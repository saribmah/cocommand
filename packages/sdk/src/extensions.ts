import { listExtensions, openExtension, type Client, type ExtensionInfo } from "@cocommand/api";
import { unwrapApiResponse } from "./request";

export interface ExtensionsApi {
  list(): Promise<ExtensionInfo[]>;
  open(id: string): Promise<unknown>;
}

export function createExtensionsApi(client: Client): ExtensionsApi {
  return {
    async list() {
      const result = await listExtensions({ client });
      return unwrapApiResponse("extensions.list", result);
    },
    async open(id: string) {
      const result = await openExtension({ client, body: { id } });
      return unwrapApiResponse("extensions.open", result, { allowNull: true });
    },
  };
}
