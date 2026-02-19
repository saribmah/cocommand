import { describe, expect, it } from "bun:test";
import type { ExtensionInfo } from "@cocommand/api";
import { createApiClient } from "../client";
import { loadExtensionViewModules } from "../react/index";
import { createSdk } from "../sdk";

function extension(overrides: Partial<ExtensionInfo> & Pick<ExtensionInfo, "id" | "name" | "kind">): ExtensionInfo {
  return {
    id: overrides.id,
    name: overrides.name,
    kind: overrides.kind,
    status: overrides.status ?? "ready",
    tags: overrides.tags ?? [],
    tools: overrides.tools ?? [],
    view: overrides.view ?? null,
  };
}

describe("loadExtensionViewModules", () => {
  it("returns fulfilled results with module payloads", async () => {
    const sdk = createSdk({ client: createApiClient("http://localhost:8080") });
    const extensions: ExtensionInfo[] = [
      extension({
        id: "sample",
        name: "Sample",
        kind: "custom",
        view: { entry: "dist/view.js", label: "Sample View", popout: null },
      }),
    ];

    const seenUrls: string[] = [];
    const results = await loadExtensionViewModules(sdk, extensions, {
      importer: async (url) => {
        seenUrls.push(url);
        return { default: () => null };
      },
    });

    expect(seenUrls).toEqual([
      "http://localhost:8080/extension/sample/assets/dist/view.js",
    ]);
    expect(results.length).toBe(1);
    expect(results[0]?.status).toBe("fulfilled");
    if (results[0]?.status === "fulfilled") {
      expect(typeof results[0].module.default).toBe("function");
    }
  });

  it("returns mixed settled results on partial failures", async () => {
    const sdk = createSdk({ client: createApiClient("http://localhost:8080") });
    const extensions: ExtensionInfo[] = [
      extension({
        id: "good",
        name: "Good",
        kind: "custom",
        view: { entry: "view.js", label: "Good View", popout: null },
      }),
      extension({
        id: "bad",
        name: "Bad",
        kind: "custom",
        view: { entry: "view.js", label: "Bad View", popout: null },
      }),
    ];

    const results = await loadExtensionViewModules(sdk, extensions, {
      importer: async (url) => {
        if (url.includes("/bad/")) {
          throw new Error("load failed");
        }
        return { default: () => null };
      },
    });

    expect(results.length).toBe(2);
    expect(results.filter((result) => result.status === "fulfilled").length).toBe(1);
    expect(results.filter((result) => result.status === "rejected").length).toBe(1);
  });

  it("uses custom importer and filters out non-custom/non-view extensions", async () => {
    const sdk = createSdk({ client: createApiClient("http://localhost:8080") });
    const extensions: ExtensionInfo[] = [
      extension({
        id: "custom-with-view",
        name: "Custom With View",
        kind: "custom",
        view: { entry: "view.js", label: "View", popout: null },
      }),
      extension({
        id: "custom-no-view",
        name: "Custom No View",
        kind: "custom",
        view: null,
      }),
      extension({
        id: "builtin-with-view",
        name: "Builtin With View",
        kind: "built-in",
        view: { entry: "view.js", label: "View", popout: null },
      }),
    ];

    const imported: string[] = [];
    await loadExtensionViewModules(sdk, extensions, {
      importer: async (url) => {
        imported.push(url);
        return { default: () => null };
      },
    });

    expect(imported).toEqual([
      "http://localhost:8080/extension/custom-with-view/assets/view.js",
    ]);
  });
});
