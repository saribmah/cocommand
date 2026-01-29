import { useState, useCallback } from "react";
import { hideWindow, type CoreResult } from "../lib/ipc";
import { useSessionStore } from "./session";

export interface CommandBarState {
  input: string;
  selectedIndex: number;
  isSubmitting: boolean;
  results: CoreResult[];
  pendingConfirmation: null;
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

  const sendMessage = useSessionStore((store) => store.sendMessage);

  const submit = useCallback(async () => {
    const text = state.input.trim();
    if (!text) return;

    setState((s) => ({ ...s, isSubmitting: true }));

    try {
      await sendMessage(text);
      setState((s) => ({
        ...s,
        input: "",
        isSubmitting: false,
        results: [],
        pendingConfirmation: null,
        followUpActive: false,
      }));
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
  }, [state.input, sendMessage]);

  const dismissResult = useCallback((index: number) => {
    setState((s) => ({
      ...s,
      results: s.results.filter((_, i) => i !== index),
    }));
  }, []);

  const confirmPending = useCallback(async () => {}, []);
  const cancelPending = useCallback(async () => {}, []);

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
