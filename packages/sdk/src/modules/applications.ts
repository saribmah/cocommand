import type { Transport } from "../transport";
import type { Application } from "../types";

export interface ApplicationsApi {
  getApplications(): Promise<Application[]>;
  openApplication(id: string): Promise<void>;
}

export function createApplications(t: Transport): ApplicationsApi {
  return {
    async getApplications() {
      return t.apiGet<Application[]>("/workspace/extension/system/applications");
    },
    async openApplication(id) {
      await t.apiPost("/workspace/extensions/open", { id });
    },
  };
}
