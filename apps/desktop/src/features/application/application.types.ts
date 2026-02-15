export interface ApplicationInfo {
  id: string;
  name: string;
  bundleId?: string;
  path: string;
  icon?: string;
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
