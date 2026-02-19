import type { Client } from "../client";
import { listApplications, openApplication } from "@cocommand/api";
import type { Application } from "../types";

export interface ApplicationsApi {
  getApplications(): Promise<Application[]>;
  openApplication(id: string): Promise<void>;
}

export function createApplications(client: Client): ApplicationsApi {
  return {
    async getApplications() {
      const { data, error } = await listApplications({ client });
      if (error) {
        throw new Error("Failed to list applications");
      }
      return data?.applications ?? [];
    },
    async openApplication(id) {
      const { error } = await openApplication({
        client,
        body: { id },
      });
      if (error) {
        throw new Error("Failed to open application");
      }
    },
  };
}
