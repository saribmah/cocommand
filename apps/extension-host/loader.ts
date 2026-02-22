/**
 * Extension loader: reads manifest and loads tool handlers.
 *
 * Responsible for reading the extension manifest from disk and
 * dynamically importing the entrypoint module to collect tool handlers.
 */

/** Tool handler function signature. */
export type ToolHandler = (
  args: Record<string, unknown>
) =>
  | {
      title: string;
      metadata: unknown;
      output: unknown;
    }
  | Promise<{
      title: string;
      metadata: unknown;
      output: unknown;
    }>;

/** Extension manifest structure (matches Rust-side ExtensionManifest). */
export interface ExtensionManifest {
  id: string;
  name: string;
  description: string;
  entrypoint: string;
  routing?: {
    keywords?: string[];
    examples?: string[];
    verbs?: string[];
    objects?: string[];
  };
  tools?: Array<{
    id: string;
    risk_level: string;
    input_schema?: Record<string, unknown>;
    output_schema?: Record<string, unknown>;
  }>;
}

/** A loaded extension with its manifest and tool handlers. */
export interface LoadedExtension {
  manifest: ExtensionManifest;
  handlers: Map<string, ToolHandler>;
}

/** Read and parse the extension manifest from the given directory. */
export async function readManifest(extensionDir: string): Promise<ExtensionManifest> {
  const manifestPath = `${extensionDir}/manifest.json`;
    // @ts-ignore
  const content = await Deno.readTextFile(manifestPath);
  return JSON.parse(content) as ExtensionManifest;
}

/**
 * Load an extension: read manifest and import tool handlers.
 *
 * The entrypoint module must export a `tools` object mapping
 * tool IDs to handler functions.
 */
export async function loadExtension(extensionDir: string): Promise<LoadedExtension> {
  const manifest = await readManifest(extensionDir);

  const entrypointPath = `${extensionDir}/${manifest.entrypoint}`;
  const entrypointUrl = new URL(`file://${entrypointPath}`);
  const module = await import(entrypointUrl.href);

  const handlers = new Map<string, ToolHandler>();

  // The entrypoint should export a `tools` object: { [toolId]: handler }
  if (module.tools && typeof module.tools === "object") {
    for (const [toolId, handler] of Object.entries(module.tools)) {
      if (typeof handler === "function") {
        handlers.set(toolId, handler as ToolHandler);
      }
    }
  }

  return { manifest, handlers };
}
