import { describe, expect, it } from "bun:test";
import {
  joinBase,
  parseHash,
  parseQuery,
  parseRoute,
  pathWithoutBase,
  stripBase,
  withBase,
} from "../src/lib/router.js";

describe("stripBase", () => {
  it("strips the base prefix", () => {
    expect(stripBase("/sub/post/x", "/sub")).toBe("/post/x");
  });

  it("maps the bare base to root", () => {
    expect(stripBase("/sub", "/sub")).toBe("/");
  });

  it("leaves non-matching paths untouched", () => {
    expect(stripBase("/other/post", "/sub")).toBe("/other/post");
    expect(stripBase("/sub-extra", "/sub")).toBe("/sub-extra");
  });

  it("treats empty base as identity", () => {
    expect(stripBase("/post/x", "")).toBe("/post/x");
  });
});

describe("pathWithoutBase", () => {
  it("accepts an injected base without touching import.meta.env", () => {
    expect(pathWithoutBase("/sub/post/x", "/sub")).toBe("/post/x");
    expect(pathWithoutBase("/sub", "/sub")).toBe("/");
  });

  it("leaves paths outside the base untouched", () => {
    expect(pathWithoutBase("/post/x", "/sub")).toBe("/post/x");
  });
});

describe("joinBase / withBase", () => {
  it("leaves root base unprefixed", () => {
    expect(joinBase("/", "/")).toBe("/");
    expect(joinBase("/post/x", "/")).toBe("/post/x");
  });

  it("prefixes a sub-path base", () => {
    expect(joinBase("/", "/sub")).toBe("/sub/");
    expect(joinBase("/post/x", "/sub")).toBe("/sub/post/x");
    expect(joinBase("/post/x", "/sub/")).toBe("/sub/post/x");
  });

  it("normalizes relative paths with a leading slash", () => {
    expect(joinBase("post/x", "/")).toBe("/post/x");
    expect(joinBase("post/x", "/sub")).toBe("/sub/post/x");
  });

  it("withBase accepts an injected base", () => {
    expect(withBase("/post/x", "/")).toBe("/post/x");
    expect(withBase("/post/x", "/sub")).toBe("/sub/post/x");
  });
});

describe("parseRoute", () => {
  it("matches home", () => {
    expect(parseRoute("/", "/")).toEqual({ name: "home" });
  });

  it("matches posts with trailing slash and encoded slugs", () => {
    expect(parseRoute("/post/hello", "/")).toEqual({ name: "post", slug: "hello" });
    expect(parseRoute("/post/hello/", "/")).toEqual({ name: "post", slug: "hello" });
    expect(parseRoute("/post/hello%20world", "/")).toEqual({ name: "post", slug: "hello world" });
  });

  it("falls back to not-found with the stripped path", () => {
    expect(parseRoute("/unknown", "/")).toEqual({ name: "not-found", path: "/unknown" });
    expect(parseRoute("/post/a/b", "/")).toEqual({ name: "not-found", path: "/post/a/b" });
  });

  it("strips the configured base before matching", () => {
    expect(parseRoute("/sub/", "/sub")).toEqual({ name: "home" });
    expect(parseRoute("/sub/post/hello", "/sub")).toEqual({ name: "post", slug: "hello" });
    expect(parseRoute("/sub/unknown", "/sub")).toEqual({ name: "not-found", path: "/unknown" });
  });

  it("parses query strings and keeps the base-aware split consistent", () => {
    expect(parseRoute("/search?q=a%20b#top", "/")).toEqual({
      name: "not-found",
      path: "/search?q=a%20b#top",
    });
    expect(pathWithoutBase("/sub/search?q=x", "/sub")).toBe("/search?q=x");
    expect(withBase(`/search?q=${encodeURIComponent("a b")}`, "/sub")).toBe("/sub/search?q=a%20b");
  });
});

describe("parseQuery / parseHash", () => {
  it("parses query strings with or without a leading ?", () => {
    expect(parseQuery("?q=a%20b").get("q")).toBe("a b");
    expect(parseQuery("q=x").get("q")).toBe("x");
    expect(parseQuery("").get("q")).toBeNull();
  });

  it("normalizes hashes to a leading # or empty", () => {
    expect(parseHash("#top")).toBe("#top");
    expect(parseHash("top")).toBe("#top");
    expect(parseHash("")).toBe("");
  });
});
