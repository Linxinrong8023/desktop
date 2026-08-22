import { describe, it, expect } from "vitest";
import { conversationKeyFor } from "./conversation-key";

describe("conversationKeyFor", () => {
  it("prefers the session id, then a draft, then a task key, then a sentinel", () => {
    expect(
      conversationKeyFor({ sessionId: "s1", taskId: "t1", draftId: null }),
    ).toBe("s1");
    expect(
      conversationKeyFor({ sessionId: null, taskId: "t1", draftId: "d1" }),
    ).toBe("draft:d1");
    expect(
      conversationKeyFor({ sessionId: null, taskId: "t1", draftId: null }),
    ).toBe("task:t1");
    expect(
      conversationKeyFor({ sessionId: null, taskId: null, draftId: null }),
    ).toBe("__none__");
  });

  it("separates sibling sessions that share a task", () => {
    expect(
      conversationKeyFor({ sessionId: "s1", taskId: "t1", draftId: null }),
    ).not.toBe(
      conversationKeyFor({ sessionId: "s2", taskId: "t1", draftId: null }),
    );
  });
});
