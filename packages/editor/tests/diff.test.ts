import assert from "node:assert/strict";
import test from "node:test";
import type { JSONContent } from "@tiptap/react";
import { diffJSONContent } from "../src/diff.ts";

const paragraph = (text: string): JSONContent => ({
  type: "paragraph",
  content: [{ type: "text", text }],
});

const doc = (...content: JSONContent[]): JSONContent => ({
  type: "doc",
  content,
});

test("returns an empty document when both sides are null", () => {
  assert.deepEqual(diffJSONContent(null, null), { type: "doc", content: [] });
});

test("marks every new node as added when there is no previous document", () => {
  const next = doc(paragraph("hello"));
  assert.deepEqual(diffJSONContent(null, next), {
    type: "doc",
    content: [
      {
        type: "paragraph",
        attrs: { diffStatus: "added" },
        content: [
          { type: "text", text: "hello", attrs: { diffStatus: "added" } },
        ],
      },
    ],
  });
});

test("marks every old node as removed when there is no next document", () => {
  const previous = doc(paragraph("bye"));
  assert.deepEqual(diffJSONContent(previous, null), {
    type: "doc",
    content: [
      {
        type: "paragraph",
        attrs: { diffStatus: "removed" },
        content: [
          { type: "text", text: "bye", attrs: { diffStatus: "removed" } },
        ],
      },
    ],
  });
});

test("keeps identical top-level nodes unmarked", () => {
  const previous = doc(paragraph("same"));
  const next = doc(paragraph("same"));
  assert.deepEqual(diffJSONContent(previous, next), previous);
});

test("emits a unified view of removed then added nodes", () => {
  const previous = doc(paragraph("old"));
  const next = doc(paragraph("new"));
  assert.deepEqual(diffJSONContent(previous, next), {
    type: "doc",
    content: [
      {
        type: "paragraph",
        attrs: { diffStatus: "removed" },
        content: [
          { type: "text", text: "old", attrs: { diffStatus: "removed" } },
        ],
      },
      {
        type: "paragraph",
        attrs: { diffStatus: "added" },
        content: [
          { type: "text", text: "new", attrs: { diffStatus: "added" } },
        ],
      },
    ],
  });
});

test("diffs list children without marking the shared list wrapper", () => {
  const previous: JSONContent = {
    type: "doc",
    content: [
      {
        type: "bulletList",
        content: [
          { type: "listItem", content: [paragraph("keep")] },
          { type: "listItem", content: [paragraph("gone")] },
        ],
      },
    ],
  };
  const next: JSONContent = {
    type: "doc",
    content: [
      {
        type: "bulletList",
        content: [
          { type: "listItem", content: [paragraph("keep")] },
          { type: "listItem", content: [paragraph("added")] },
        ],
      },
    ],
  };

  assert.deepEqual(diffJSONContent(previous, next), {
    type: "doc",
    content: [
      {
        type: "bulletList",
        content: [
          { type: "listItem", content: [paragraph("keep")] },
          {
            type: "listItem",
            attrs: { diffStatus: "removed" },
            content: [
              {
                type: "paragraph",
                attrs: { diffStatus: "removed" },
                content: [
                  {
                    type: "text",
                    text: "gone",
                    attrs: { diffStatus: "removed" },
                  },
                ],
              },
            ],
          },
          {
            type: "listItem",
            attrs: { diffStatus: "added" },
            content: [
              {
                type: "paragraph",
                attrs: { diffStatus: "added" },
                content: [
                  {
                    type: "text",
                    text: "added",
                    attrs: { diffStatus: "added" },
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  });
});
