import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Set ORA_VITEST_MAX_WORKERS only on machines that need a lower memory peak;
    // leaving it unset preserves Vitest's existing defaults for everyone else.
    maxWorkers: process.env.ORA_VITEST_MAX_WORKERS
      ? Number.parseInt(process.env.ORA_VITEST_MAX_WORKERS, 10)
      : undefined,
  },
});
