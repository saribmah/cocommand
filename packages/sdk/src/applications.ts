import { listApplications, openApplication, type ApplicationInfo, type Client, type OpenApplicationResponse2 } from "@cocommand/api";
import { unwrapApiResponse } from "./request";

export interface ApplicationsApi {
  list(): Promise<ApplicationInfo[]>;
  open(id: string): Promise<OpenApplicationResponse2>;
}

export function createApplicationsApi(client: Client): ApplicationsApi {
  return {
    async list() {
      const result = await listApplications({ client });
      const data = unwrapApiResponse("applications.list", result);
      return data.applications;
    },
    async open(id: string) {
      const result = await openApplication({
        client,
        body: { id },
      });
      return unwrapApiResponse("applications.open", result);
    },
  };
}
