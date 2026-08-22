import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  test: {
    environment: "happy-dom",
    setupFiles: ["src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
      include: [
        "src/features/pipeline/lib/**/*.{ts,tsx}",
        "src/features/pipeline/hooks/**/*.{ts,tsx}",
        "src/features/pipeline/api/**/*.{ts,tsx}",
        "src/features/config/lib/**/*.{ts,tsx}",
        "src/features/overlay/lib/**/*.{ts,tsx}",
        "src/features/meeting/**/lib/**/*.{ts,tsx}",
      ],
      exclude: [
        "src/**/*.test.{ts,tsx}",
        "src/test/**",
        "src/**/*.d.ts",
      ],
      thresholds: {
        lines: 40,
        functions: 35,
        branches: 30,
        statements: 40,
      },
    },
  },
});
