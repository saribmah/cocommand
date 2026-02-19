import type { ComposerActionsBridge } from "./configure";
import type { FilePartInput, ExtensionPartInput } from "./types";
import { SdkError } from "./errors";

export interface ComposerApi {
  addFile(part: FilePartInput): void;
  addExtension(part: ExtensionPartInput): void;
  removePart(match: { type: "extension" | "file"; name: string }): void;
  setActiveTab(tab: string): void;
  focusInput(): void;
}

export function createComposerApi(bridge?: ComposerActionsBridge): ComposerApi {
  function getBridge(): ComposerActionsBridge {
    if (!bridge) {
      throw new SdkError({
        code: "invalid_response",
        message: "Composer bridge is only available inside extension view context",
        source: "composer",
      });
    }
    return bridge;
  }

  return {
    addFile(part) {
      getBridge().addPart(part);
    },
    addExtension(part) {
      getBridge().addPart(part);
    },
    removePart(match) {
      getBridge().removePart(match);
    },
    setActiveTab(tab) {
      getBridge().setActiveTab(tab);
    },
    focusInput() {
      getBridge().focusInput();
    },
  };
}
