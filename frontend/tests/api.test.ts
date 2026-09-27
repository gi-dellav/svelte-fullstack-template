import { describe, expect, it } from "bun:test";
import { apiUrl } from "../src/lib/api.js";

describe("apiUrl", () => {
  it("builds same-origin paths when VITE_API_URL is unset", () => {
    expect(apiUrl("/api/health")).toBe("/api/health");
  });
});
