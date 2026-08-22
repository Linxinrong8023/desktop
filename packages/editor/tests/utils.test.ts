import assert from "node:assert/strict";
import test from "node:test";
import { isAllowedUri, sanitizeUrl } from "../src/utils.ts";

test("allows http and https URLs", () => {
  assert.ok(isAllowedUri("https://example.com/path"));
  assert.ok(isAllowedUri("http://example.com"));
});

test("rejects javascript URLs", () => {
  assert.ok(!isAllowedUri("javascript:alert(1)"));
});

test("sanitizes allowed absolute URLs to href form", () => {
  assert.equal(
    sanitizeUrl("https://example.com/a", "https://unused.example"),
    "https://example.com/a",
  );
});

test("resolves relative URLs against the provided base", () => {
  assert.equal(
    sanitizeUrl("/docs", "https://example.com/app"),
    "https://example.com/docs",
  );
});

test("replaces disallowed URLs with a hash", () => {
  assert.equal(sanitizeUrl("javascript:alert(1)", "https://example.com"), "#");
});
