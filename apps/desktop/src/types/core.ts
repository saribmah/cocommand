export interface ArtifactAction {
  id: string;
  label: string;
}

export interface ArtifactResult {
  type: "artifact";
  title: string;
  body: string;
  actions: ArtifactAction[];
}

export interface PreviewResult {
  type: "preview";
  title: string;
  body: string;
}

export interface ConfirmationResult {
  type: "confirmation";
  title: string;
  body: string;
  confirmation_id: string;
}

export interface ErrorResult {
  type: "error";
  title: string;
  body: string;
}

export type CoreResult = ArtifactResult | PreviewResult | ConfirmationResult | ErrorResult;
