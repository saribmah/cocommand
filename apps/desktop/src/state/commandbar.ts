import { useState, useCallback } from "react";
import { submitCommand, hideWindow, type CoreResponse, type RoutedCandidate } from "../lib/ipc";

export interface CommandBarState {
  input: string;
  suggestions: RoutedCandidate[];
  selectedIndex: number;
  clarification: string | null;
  isSubmitting: boolean;
}

export function useCommandBar() {
  const [state, setState] = useState<CommandBarState>({
    input: "",
    suggestions: [],
    selectedIndex: -1,
    clarification: null,
    isSubmitting: false,
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
    });
  }, []);

  const submit = useCallback(async () => {
    const text = state.input.trim();
    if (!text) return;

    setState((s) => ({ ...s, isSubmitting: true }));

    try {
      const response: CoreResponse = await submitCommand(text);

      if (response.type === "Routed") {
        setState((s) => ({
          ...s,
          suggestions: response.candidates,
          selectedIndex: response.candidates.length > 0 ? 0 : -1,
          clarification: null,
          isSubmitting: false,
        }));
      } else {
        setState((s) => ({
          ...s,
          suggestions: [],
          selectedIndex: -1,
          clarification: response.message,
          isSubmitting: false,
        }));
      }
    } catch (err) {
      setState((s) => ({
        ...s,
        isSubmitting: false,
        clarification: `Error: ${err}`,
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

  const dismiss = useCallback(() => {
    if (state.suggestions.length > 0 || state.clarification) {
      reset();
    } else {
      hideWindow();
    }
  }, [state.suggestions.length, state.clarification, reset]);

  return {
    ...state,
    setInput,
    submit,
    navigateUp,
    navigateDown,
    dismiss,
    reset,
  };
}
