import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";

// @testing-library/react's automatic afterEach(cleanup) only registers itself
// when `afterEach` already exists as a global function. This project's
// vite.config.ts does not set `test.globals: true`, so that auto-registration
// never fires and DOM from one test leaks into the next. Register cleanup
// explicitly instead, per Testing Library's own docs for Vitest without globals.
afterEach(() => {
  cleanup();
});
