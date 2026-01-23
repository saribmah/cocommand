import { useState, useCallback } from "react";
import {
  submitCommand,
  confirmAction,
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
}

export function useCommandBar() {
  const [state, setState] = useState<CommandBarState>({
    input: "",
    selectedIndex: -1,
    isSubmitting: false,
    results: [],
    pendingConfirmation: null,
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
    });
  }, []);

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
          results: [...s.results, result],
        }));
      }
    } catch (err) {
      const errorResult: CoreResult = {
        type: "error",
        title: "Error",
        body: String(err),
      };
      setState((s) => ({
        ...s,
        isSubmitting: false,
        results: [...s.results, errorResult],
      }));
    }
  }, [state.input]);

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
        results: [...s.results, result],
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
        results: [...s.results, errorResult],
      }));
    }
  }, [state.pendingConfirmation]);

  const cancelPending = useCallback(async () => {
    if (!state.pendingConfirmation) return;
    const confirmationId = state.pendingConfirmation.confirmation_id;

    try {
      await confirmAction(confirmationId, false);
    } catch {
      // Ignore errors on cancel — just clear the UI
    }
    setState((s) => ({ ...s, pendingConfirmation: null }));
  }, [state.pendingConfirmation]);

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
