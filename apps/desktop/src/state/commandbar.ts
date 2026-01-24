import { useState, useCallback, useEffect } from "react";
import {
  submitCommand,
  confirmAction,
  getWorkspaceSnapshot,
  hideWindow,
  normalizeResponse,
  type CoreResponse,
  type CoreResult,
  type ConfirmationResult,
} from "../lib/ipc";

export interface CommandBarState {
  input: string;
  selectedIndex: number;
  isSubmitting: boolean;
  results: CoreResult[];
  pendingConfirmation: ConfirmationResult | null;
  followUpActive: boolean;
}

export function useCommandBar() {
  const [state, setState] = useState<CommandBarState>({
    input: "",
    selectedIndex: -1,
    isSubmitting: false,
    results: [],
    pendingConfirmation: null,
    followUpActive: false,
  });

  const setInput = useCallback((value: string) => {
    setState((s) => ({ ...s, input: value }));
  }, []);

  const reset = useCallback(() => {
    setState({
      input: "",
      selectedIndex: -1,
      isSubmitting: false,
      results: [],
      pendingConfirmation: null,
      followUpActive: false,
    });
  }, []);

  const syncFollowUp = useCallback(async () => {
    try {
      const workspace = await getWorkspaceSnapshot();
      setState((s) => ({
        ...s,
        followUpActive: workspace.mode === "FollowUpActive" && workspace.follow_up !== null,
      }));
    } catch {
      // If snapshot fails, default to inactive
      setState((s) => ({ ...s, followUpActive: false }));
    }
  }, []);

  // Sync follow-up state on mount (window open)
  useEffect(() => {
    syncFollowUp();
  }, [syncFollowUp]);

  const submit = useCallback(async () => {
    const text = state.input.trim();
    if (!text) return;

    setState((s) => ({ ...s, isSubmitting: true }));

    try {
      const response: CoreResponse = await submitCommand(text);
      const result = normalizeResponse(response);

      if (result.type === "confirmation") {
        setState((s) => ({
          ...s,
          input: "",
          isSubmitting: false,
          pendingConfirmation: result as ConfirmationResult,
        }));
      } else {
        setState((s) => ({
          ...s,
          input: "",
          isSubmitting: false,
          results: [result],
        }));
      }
      await syncFollowUp();
    } catch (err) {
      const errorResult: CoreResult = {
        type: "error",
        title: "Error",
        body: String(err),
      };
      setState((s) => ({
        ...s,
        isSubmitting: false,
        results: [errorResult],
      }));
    }
  }, [state.input, syncFollowUp]);

  const dismissResult = useCallback((index: number) => {
    setState((s) => ({
      ...s,
      results: s.results.filter((_, i) => i !== index),
    }));
  }, []);

  const confirmPending = useCallback(async () => {
    if (!state.pendingConfirmation) return;
    const confirmationId = state.pendingConfirmation.confirmation_id;

    try {
      const response: CoreResponse = await confirmAction(confirmationId, true);
      const result = normalizeResponse(response);
      setState((s) => ({
        ...s,
        pendingConfirmation: null,
        results: [result],
      }));
    } catch (err) {
      const errorResult: CoreResult = {
        type: "error",
        title: "Confirmation Failed",
        body: String(err),
      };
      setState((s) => ({
        ...s,
        pendingConfirmation: null,
        results: [errorResult],
      }));
    }
    await syncFollowUp();
  }, [state.pendingConfirmation, syncFollowUp]);

  const cancelPending = useCallback(async () => {
    if (!state.pendingConfirmation) return;
    const confirmationId = state.pendingConfirmation.confirmation_id;

    try {
      await confirmAction(confirmationId, false);
    } catch {
      // Ignore errors on cancel — just clear the UI
    }
    setState((s) => ({ ...s, pendingConfirmation: null }));
    await syncFollowUp();
  }, [state.pendingConfirmation, syncFollowUp]);

  const dismiss = useCallback(() => {
    if (state.pendingConfirmation) {
      cancelPending();
    } else if (state.results.length > 0) {
      setState((s) => ({ ...s, results: [] }));
    } else {
      hideWindow();
    }
  }, [state.pendingConfirmation, state.results.length, cancelPending]);

  return {
    ...state,
    setInput,
    submit,
    dismiss,
    dismissResult,
    confirmPending,
    cancelPending,
    reset,
  };
}
