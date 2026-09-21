import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "MINICLOUD_");
  return {
    plugins: [react(), tailwindcss()],
    cacheDir: env.MINICLOUD_CACHE_DIR || "node_modules/.vite",
    server: {
      host: "127.0.0.1",
      port: 5174,
      strictPort: true,
      proxy: {
        "/api": { target: env.MINICLOUD_SERVER_URL || "http://127.0.0.1:4317" },
      },
    },
  };
});
