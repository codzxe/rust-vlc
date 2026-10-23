import { cleanup } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";

// vitest runs with `globals: false`, so RTL's auto-cleanup (which hooks the
// global afterEach) never registers — do it explicitly.
afterEach(cleanup);
