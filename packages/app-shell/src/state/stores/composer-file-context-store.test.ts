import { beforeEach, describe, expect, it } from "vitest";
import { useComposerFileContextStore } from "./composer-file-context-store";

beforeEach(() => {
  useComposerFileContextStore.setState({ pendingByConversation: {} });
});

describe("useComposerFileContextStore", () => {
  it("queues and consumes selections per conversation key", () => {
    useComposerFileContextStore.getState().addSelection("session-a", {
      path: "a.ts",
      startLine: 1,
      endLine: 1,
    });
    useComposerFileContextStore.getState().addSelection("session-b", {
      path: "b.ts",
      startLine: 2,
      endLine: 2,
    });

    const pendingA =
      useComposerFileContextStore.getState().pendingByConversation["session-a"];
    const pendingB =
      useComposerFileContextStore.getState().pendingByConversation["session-b"];
    expect(pendingA?.selections).toEqual([
      { path: "a.ts", startLine: 1, endLine: 1 },
    ]);
    expect(pendingB?.selections).toEqual([
      { path: "b.ts", startLine: 2, endLine: 2 },
    ]);
    expect(pendingA?.id).not.toBe(pendingB?.id);

    useComposerFileContextStore
      .getState()
      .consumeSelections("session-a", pendingA!.id);
    expect(
      useComposerFileContextStore.getState().pendingByConversation["session-a"],
    ).toBeUndefined();
    expect(
      useComposerFileContextStore.getState().pendingByConversation["session-b"]
        ?.selections,
    ).toEqual([{ path: "b.ts", startLine: 2, endLine: 2 }]);
  });

  it("ignores consumeSelections for a stale request id", () => {
    useComposerFileContextStore.getState().addSelection("session-a", {
      path: "a.ts",
      startLine: 1,
      endLine: 1,
    });
    const id =
      useComposerFileContextStore.getState().pendingByConversation["session-a"]!
        .id;
    useComposerFileContextStore
      .getState()
      .consumeSelections("session-a", id + 1);
    expect(
      useComposerFileContextStore.getState().pendingByConversation["session-a"],
    ).toBeDefined();
  });
});
