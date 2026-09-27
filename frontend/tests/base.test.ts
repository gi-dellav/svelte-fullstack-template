import { describe, expect, it } from "bun:test";
import { normalizeBaseOverride, resolveBase } from "../src/lib/base.js";

describe("resolveBase", () => {
  it("defaults to root (Vercel serves from the domain root)", () => {
    expect(resolveBase({ baseOverride: undefined })).toBe("/");
  });

  it("honors an explicit BASE_PATH override", () => {
    expect(resolveBase({ baseOverride: "/custom/" })).toBe("/custom/");
    expect(resolveBase({ baseOverride: "/" })).toBe("/");
  });

  it("treats blank overrides as unset", () => {
    expect(resolveBase({ baseOverride: "  " })).toBe("/");
  });
});

describe("normalizeBaseOverride", () => {
  it("returns undefined for missing or blank values", () => {
    expect(normalizeBaseOverride(undefined)).toBeUndefined();
    expect(normalizeBaseOverride("")).toBeUndefined();
    expect(normalizeBaseOverride("   ")).toBeUndefined();
  });

  it("ensures leading and trailing slashes", () => {
    expect(normalizeBaseOverride("/")).toBe("/");
    expect(normalizeBaseOverride("custom")).toBe("/custom/");
    expect(normalizeBaseOverride("custom/")).toBe("/custom/");
    expect(normalizeBaseOverride("/custom")).toBe("/custom/");
    expect(normalizeBaseOverride(" /custom/ ")).toBe("/custom/");
  });
});
