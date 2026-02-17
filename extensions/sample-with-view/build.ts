import * as esbuild from "https://deno.land/x/esbuild@v0.24.2/mod.js";

await esbuild.build({
  entryPoints: ["src/view.tsx"],
  bundle: true,
  format: "esm",
  outfile: "dist/view.js",
  external: ["react", "react/jsx-runtime", "zustand", "@cocommand/ui", "@cocommand/api"],
  jsx: "automatic",
});

esbuild.stop();
