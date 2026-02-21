import { SdkError } from "@cocommand/sdk";
import { type PropsWithChildren, useEffect, useRef } from "react";
import { useRuntimeSdk } from "../server/runtime-sdk.context";
import { useSessionContext } from "../session/session.context";
import { CommandContext } from "./command.context";
import { createCommandStore, type CommandStore } from "./command.store";

type CommandProviderProps = PropsWithChildren;

export function CommandProvider({ children }: CommandProviderProps) {
  const sdk = useRuntimeSdk();
  const sessionId = useSessionContext((state) => state.context?.session_id ?? null);
  const setSessionContext = useSessionContext((state) => state.setContext);

  const storeRef = useRef<CommandStore | null>(null);
  if (storeRef.current === null) {
    storeRef.current = createCommandStore();
  }

  useEffect(() => {
    let disposed = false;
    let activeController: AbortController | null = null;

    const wait = (ms: number) =>
      new Promise<void>((resolve) => {
        const timeout = window.setTimeout(() => {
          window.clearTimeout(timeout);
          resolve();
        }, ms);
      });

    const streamEvents = async () => {
      while (!disposed) {
        const controller = new AbortController();
        activeController = controller;

        try {
          for await (const event of sdk.events.stream({
            signal: controller.signal,
            sessionId: sessionId ?? undefined,
          })) {
            if (disposed) {
              return;
            }
            if (sessionId && event.sessionId !== sessionId) {
              continue;
            }
            if (event.type === "context") {
              setSessionContext(event.context);
            }
            storeRef.current?.getState().applyRuntimeEvent(event);
          }

          if (!disposed) {
            await wait(250);
          }
        } catch (error) {
          if (disposed) {
            return;
          }
          if (error instanceof SdkError && error.code === "aborted") {
            return;
          }
          console.error("Runtime events stream error", error);
          await wait(1000);
        }
      }
    };

    void streamEvents();

    return () => {
      disposed = true;
      activeController?.abort();
    };
  }, [sdk, sessionId, setSessionContext]);

  return (
    <CommandContext.Provider value={storeRef.current}>
      {children}
    </CommandContext.Provider>
  );
}
