import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  COMPOSER_INPUT_STORAGE_KEY,
  composerInputHasContent,
  useComposerInputStore,
} from "./composer-input-store";
import { flushDebouncedPersistStorage } from "./debounced-json-storage";

beforeEach(() => {
  flushDebouncedPersistStorage();
  window.localStorage.clear();
  useComposerInputStore.getState().reset();
  flushDebouncedPersistStorage();
  window.localStorage.clear();
});

afterEach(() => {
  useComposerInputStore.getState().reset();
  flushDebouncedPersistStorage();
  window.localStorage.clear();
});

describe("composerInputHasContent", () => {
  it("treats whitespace-only text as empty unless images are parked", () => {
    expect(composerInputHasContent({ text: "  ", images: [] })).toBe(false);
    expect(
      composerInputHasContent({
        text: "",
        images: [
          {
            id: "i1",
            name: "a.png",
            size: 1,
            content: { mimeType: "image/png", data: "aa" },
          },
        ],
      }),
    ).toBe(true);
  });
});

describe("useComposerInputStore", () => {
  it("parks and clears input per conversation key", () => {
    useComposerInputStore.getState().setInput("s1", {
      text: "hello",
      images: [],
    });
    useComposerInputStore.getState().setInput("s2", {
      text: "other",
      images: [],
    });

    expect(useComposerInputStore.getState().byKey.s1).toEqual({
      text: "hello",
      images: [],
    });

    useComposerInputStore.getState().clear("s1");
    expect(useComposerInputStore.getState().byKey.s1).toBeUndefined();
    expect(useComposerInputStore.getState().byKey.s2?.text).toBe("other");
  });

  it("drops empty payloads instead of storing blanks", () => {
    useComposerInputStore.getState().setInput("s1", {
      text: "keep",
      images: [],
    });
    useComposerInputStore.getState().setInput("s1", { text: "  ", images: [] });
    expect(useComposerInputStore.getState().byKey.s1).toBeUndefined();
  });

  it("rekeys parked input onto a new conversation id", () => {
    useComposerInputStore.getState().setInput("draft:d1", {
      text: "first send",
      images: [],
    });
    useComposerInputStore.getState().rekey("draft:d1", "session-1");

    expect(useComposerInputStore.getState().byKey["draft:d1"]).toBeUndefined();
    expect(useComposerInputStore.getState().byKey["session-1"]?.text).toBe(
      "first send",
    );
  });

  it("preserves existing destination input when rekeying", () => {
    useComposerInputStore.getState().setInput("draft:d1", {
      text: "draft text",
      images: [],
    });
    useComposerInputStore.getState().setInput("session-1", {
      text: "newer session text",
      images: [],
    });

    useComposerInputStore.getState().rekey("draft:d1", "session-1");

    expect(useComposerInputStore.getState().byKey).toEqual({
      "session-1": { text: "newer session text", images: [] },
    });
  });

  it("copies caller-owned image arrays", () => {
    const images = [
      {
        id: "i1",
        name: "a.png",
        size: 1,
        content: { mimeType: "image/png", data: "aa" },
      },
    ];
    useComposerInputStore.getState().setInput("s1", { text: "", images });

    images.push({
      id: "i2",
      name: "b.png",
      size: 2,
      content: { mimeType: "image/png", data: "bb" },
    });

    expect(useComposerInputStore.getState().byKey.s1?.images).toHaveLength(1);
  });

  it("persists typed text to localStorage without image payloads", () => {
    useComposerInputStore.getState().setInput("s1", {
      text: "parked",
      images: [
        {
          id: "i1",
          name: "a.png",
          size: 1,
          content: { mimeType: "image/png", data: "aa" },
        },
      ],
    });
    flushDebouncedPersistStorage();

    const raw = window.localStorage.getItem(COMPOSER_INPUT_STORAGE_KEY);
    expect(raw).not.toBeNull();
    const parsed = JSON.parse(raw!) as {
      state: { byKey: Record<string, { text: string; images: unknown[] }> };
    };
    expect(parsed.state.byKey.s1).toEqual({
      text: "parked",
      images: [],
    });
    // In-process memory still keeps the attachment for the current session.
    expect(useComposerInputStore.getState().byKey.s1?.images).toHaveLength(1);
  });

  it("parks TipTap doc in memory but strips it from localStorage", () => {
    const doc = {
      type: "doc",
      content: [
        {
          type: "paragraph",
          content: [
            {
              type: "composerFile",
              attrs: {
                path: "src/a.ts",
                startLine: null,
                endLine: null,
                kind: "file",
              },
            },
          ],
        },
      ],
    };
    useComposerInputStore.getState().setInput("s1", {
      text: "`src/a.ts`",
      images: [],
      doc,
    });
    expect(useComposerInputStore.getState().byKey.s1?.doc).toEqual(doc);
    flushDebouncedPersistStorage();

    const raw = window.localStorage.getItem(COMPOSER_INPUT_STORAGE_KEY);
    expect(raw).not.toBeNull();
    const parsed = JSON.parse(raw!) as {
      state: { byKey: Record<string, { text: string; doc?: unknown }> };
    };
    expect(parsed.state.byKey.s1).toEqual({
      text: "`src/a.ts`",
      images: [],
    });
  });

  it("preserves a parked TipTap doc when a later text-only setInput omits it", () => {
    const doc = {
      type: "doc",
      content: [
        {
          type: "paragraph",
          content: [
            {
              type: "promptToken",
              attrs: { kind: "command", name: "test" },
            },
          ],
        },
      ],
    };
    useComposerInputStore.getState().setInput("draft:d1", {
      text: "/test ",
      images: [],
      doc,
    });
    useComposerInputStore.getState().setInput("draft:d1", {
      text: "/test ",
      images: [],
    });
    expect(useComposerInputStore.getState().byKey["draft:d1"]?.doc).toEqual(
      doc,
    );
  });

  it("rehydrates text-only parks from localStorage", async () => {
    useComposerInputStore.getState().reset();
    flushDebouncedPersistStorage();
    window.localStorage.setItem(
      COMPOSER_INPUT_STORAGE_KEY,
      JSON.stringify({
        state: {
          byKey: { s1: { text: "survive restart", images: [] } },
        },
        version: 0,
      }),
    );
    await useComposerInputStore.persist.rehydrate();
    expect(useComposerInputStore.getState().byKey.s1).toEqual({
      text: "survive restart",
      images: [],
    });
  });

  it("drops malformed persisted entries without breaking rehydration", async () => {
    window.localStorage.setItem(
      COMPOSER_INPUT_STORAGE_KEY,
      JSON.stringify({
        state: {
          byKey: {
            valid: { text: "keep", images: "corrupt" },
            number: 42,
            missingText: { images: [] },
          },
        },
        version: 0,
      }),
    );

    await expect(
      useComposerInputStore.persist.rehydrate(),
    ).resolves.toBeUndefined();
    expect(useComposerInputStore.getState().byKey).toEqual({
      valid: { text: "keep", images: [] },
    });
  });

  it("drops a malformed persisted byKey container", async () => {
    window.localStorage.setItem(
      COMPOSER_INPUT_STORAGE_KEY,
      JSON.stringify({ state: { byKey: "corrupt" }, version: 0 }),
    );

    await expect(
      useComposerInputStore.persist.rehydrate(),
    ).resolves.toBeUndefined();
    expect(useComposerInputStore.getState().byKey).toEqual({});
  });

  it("does not persist image-only parks", () => {
    useComposerInputStore.getState().setInput("s1", {
      text: "",
      images: [
        {
          id: "i1",
          name: "a.png",
          size: 1,
          content: { mimeType: "image/png", data: "aa" },
        },
      ],
    });
    expect(useComposerInputStore.getState().byKey.s1?.images).toHaveLength(1);
    flushDebouncedPersistStorage();

    const raw = window.localStorage.getItem(COMPOSER_INPUT_STORAGE_KEY);
    const parsed = JSON.parse(raw ?? '{"state":{"byKey":{}}}') as {
      state: { byKey: Record<string, unknown> };
    };
    expect(parsed.state.byKey.s1).toBeUndefined();
  });
});
