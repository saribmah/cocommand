import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  build: {
    lib: {
      entry: "src/view.tsx",
      formats: ["es"],
      fileName: "view",
    },
    outDir: "dist",
    rollupOptions: {
      external: ["react", "react/jsx-runtime", "zustand", "@cocommand/ui"],
    },
  },
});
