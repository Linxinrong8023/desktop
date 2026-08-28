import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "node:path";

export default defineConfig({
  plugins: [react()],
  // Keep progress on stdout; the default TTY reporter clears the screen and
  // can hide stderr that run-with-clean-stderr is gating on.
  clearScreen: false,
  resolve: {
    alias: {
      "@ora/editor/composer": path.resolve(
        __dirname,
        "../editor/src/composer/index.ts",
      ),
      "@ora/editor": path.resolve(__dirname, "../editor/src/index.ts"),
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
    css: false,
  },
});
