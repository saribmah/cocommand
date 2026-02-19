import { afterEach, describe, expect, it } from "bun:test";
import type { ExtensionInfo } from "@cocommand/api";
import { createApiClient } from "../client";
import { createExtensionsApi } from "../extensions";

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
});

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

describe("extensions.views", () => {
  it("normalizes trailing slash and strips leading slash from asset path", () => {
    const api = createExtensionsApi(createApiClient("http://localhost:8080/"));
    const url = api.views.resolveAssetUrl("sample-extension", "/dist/view.js");

    expect(url).toBe("http://localhost:8080/extension/sample-extension/assets/dist/view.js");
  });

  it("encodes extension id and asset path segments", () => {
    const api = createExtensionsApi(createApiClient("http://localhost:8080"));
    const url = api.views.resolveAssetUrl("my ext/1", "dist/view file.js");

    expect(url).toBe(
      "http://localhost:8080/extension/my%20ext%2F1/assets/dist/view%20file.js",
    );
  });

  it("fromExtensions includes only custom extensions with a view", () => {
    const api = createExtensionsApi(createApiClient("http://localhost:8080"));
    const result = api.views.fromExtensions([
      extension({
        id: "custom-with-view",
        name: "Custom With View",
        kind: "custom",
        view: { entry: "dist/view.js", label: "View", popout: null },
      }),
      extension({
        id: "builtin-with-view",
        name: "Builtin View",
        kind: "built-in",
        view: { entry: "dist/view.js", label: "View", popout: null },
      }),
      extension({
        id: "custom-no-view",
        name: "Custom No View",
        kind: "custom",
        view: null,
      }),
    ]);

    expect(result.length).toBe(1);
    expect(result[0]?.extensionId).toBe("custom-with-view");
    expect(result[0]?.assetUrl).toBe(
      "http://localhost:8080/extension/custom-with-view/assets/dist/view.js",
    );
  });

  it("listCustom resolves descriptors from list()", async () => {
    const payload: ExtensionInfo[] = [
      extension({
        id: "sample",
        name: "Sample",
        kind: "custom",
        view: { entry: "view.js", label: "Sample View", popout: null },
      }),
      extension({
        id: "notes",
        name: "Notes",
        kind: "built-in",
        view: null,
      }),
    ];

    globalThis.fetch = ((input: RequestInfo | URL) => {
      const url =
        typeof input === "string"
          ? input
          : input instanceof Request
            ? input.url
            : String(input);

      if (url.endsWith("/workspace/extensions")) {
        return Promise.resolve(
          new Response(JSON.stringify(payload), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );
      }

      return Promise.resolve(new Response("Not Found", { status: 404 }));
    }) as typeof fetch;

    const api = createExtensionsApi(createApiClient("http://localhost:8080"));
    const result = await api.views.listCustom();

    expect(result.length).toBe(1);
    expect(result[0]?.extensionId).toBe("sample");
    expect(result[0]?.assetUrl).toBe("http://localhost:8080/extension/sample/assets/view.js");
  });
});
