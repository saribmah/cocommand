export interface PermissionStatus {
  id: string;
  label: string;
  granted: boolean;
  required: boolean;
}

export interface PermissionsResponse {
  platform: string;
  permissions: PermissionStatus[];
}
