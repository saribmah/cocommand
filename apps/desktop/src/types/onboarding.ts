export interface OnboardingStatus {
  completed: boolean;
  completed_at: number | null;
  version: string;
}

export interface UpdateOnboardingPayload {
  completed: boolean;
  version?: string;
}
