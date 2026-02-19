import { describe, expect, it } from "bun:test";
import { createDeferredAI, createDeferredLocalStorage, createDeferredUiApi } from "../deferred";
import { SdkNotImplementedError } from "../errors";

describe("deferred surfaces", () => {
  it("ai.generate throws not_implemented", async () => {
    const ai = createDeferredAI();
    await expect(ai.generate({ prompt: "test" })).rejects.toBeInstanceOf(SdkNotImplementedError);
  });

  it("localStorage throws not_implemented", async () => {
    const storage = createDeferredLocalStorage("sample-ext");
    await expect(storage.keys()).rejects.toBeInstanceOf(SdkNotImplementedError);
  });

  it("ui methods throw not_implemented", async () => {
    const ui = createDeferredUiApi();
    await expect(ui.showToast({ message: "x" })).rejects.toBeInstanceOf(SdkNotImplementedError);
    await expect(ui.windowManagement.resize(300, 200)).rejects.toBeInstanceOf(SdkNotImplementedError);
  });
});
