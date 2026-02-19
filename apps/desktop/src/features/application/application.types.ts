export interface ApplicationInfo {
  id: string;
  name: string;
  bundleId?: string | null;
  path: string;
  icon?: string | null;
}

export interface ApplicationsResponse {
  applications: ApplicationInfo[];
  count: number;
}

export interface OpenApplicationRequest {
  id: string;
}

export interface OpenApplicationResponse {
  status: string;
  opened: boolean;
  id: string;
}
