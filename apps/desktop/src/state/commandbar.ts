import { useState, useCallback } from "react";
import { submitCommand, hideWindow, normalizeResponse, type CoreResponse, type RoutedCandidate } from "../lib/ipc";
import type { CoreResult } from "../types/core";

export interface CommandBarState {
  input: string;
  suggestions: RoutedCandidate[];
  selectedIndex: number;
  clarification: string | null;
  isSubmitting: boolean;
  results: CoreResult[];
}

export function useCommandBar() {
  const [state, setState] = useState<CommandBarState>({
    input: "",
    suggestions: [],
    selectedIndex: -1,
    clarification: null,
    isSubmitting: false,
    results: [],
  });

  const setInput = useCallback((value: string) => {
    setState((s) => ({ ...s, input: value, clarification: null }));
  }, []);

  const reset = useCallback(() => {
    setState({
      input: "",
      suggestions: [],
      selectedIndex: -1,
      clarification: null,
      isSubmitting: false,
      results: [],
    });
  }, []);

  const submit = useCallback(async () => {
    const text = state.input.trim();
    if (!text) return;

    setState((s) => ({ ...s, isSubmitting: true }));

    try {
      const response: CoreResponse = await submitCommand(text);
      const result = normalizeResponse(response);

      if (result) {
        setState((s) => ({
          ...s,
          input: "",
          suggestions: [],
          selectedIndex: -1,
          clarification: null,
          isSubmitting: false,
          results: [...s.results, result],
        }));
      } else if (response.type === "ClarificationNeeded") {
        setState((s) => ({
          ...s,
          suggestions: [],
          selectedIndex: -1,
          clarification: response.message,
          isSubmitting: false,
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
        clarification: null,
        results: [...s.results, errorResult],
      }));
    }
  }, [state.input]);

  const navigateUp = useCallback(() => {
    setState((s) => {
      if (s.suggestions.length === 0) return s;
      const next = s.selectedIndex <= 0 ? s.suggestions.length - 1 : s.selectedIndex - 1;
      return { ...s, selectedIndex: next };
    });
  }, []);

  const navigateDown = useCallback(() => {
    setState((s) => {
      if (s.suggestions.length === 0) return s;
      const next = s.selectedIndex >= s.suggestions.length - 1 ? 0 : s.selectedIndex + 1;
      return { ...s, selectedIndex: next };
    });
  }, []);

  const dismissResult = useCallback((index: number) => {
    setState((s) => ({
      ...s,
      results: s.results.filter((_, i) => i !== index),
    }));
  }, []);

  const dismiss = useCallback(() => {
    if (state.results.length > 0) {
      setState((s) => ({ ...s, results: [] }));
    } else if (state.suggestions.length > 0 || state.clarification) {
      reset();
    } else {
      hideWindow();
    }
  }, [state.results.length, state.suggestions.length, state.clarification, reset]);

  return {
    ...state,
    setInput,
    submit,
    navigateUp,
    navigateDown,
    dismiss,
    dismissResult,
    reset,
  };
}
